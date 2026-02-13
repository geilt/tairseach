//! 1Password Handler
//!
//! Socket handlers for 1Password operations via Go helper binary.

use serde_json::Value;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, error, info};

use super::common::*;
use super::super::protocol::JsonRpcResponse;

/// Retrieve the SA token from the auth broker
async fn get_sa_token(params: &Value) -> Result<String, JsonRpcResponse> {
    let auth_broker = get_auth_broker().await?;

    let explicit_account = optional_string(params, "account");

    let mut attempts: Vec<String> = Vec::new();
    if let Some(acct) = explicit_account {
        attempts.push(acct.to_string());
    }
    attempts.push("default".to_string());

    for acct in &attempts {
        if let Ok(fields) = auth_broker.get_credential("onepassword", Some(acct)).await {
            if let Some(token) = fields.get("service_account_token") {
                return Ok(token.clone());
            }
        }
    }

    // Fallback: first available onepassword credential
    let all_creds = auth_broker.list_credentials().await;
    for cred in &all_creds {
        if cred.cred_type == "onepassword" {
            if let Ok(fields) = auth_broker
                .get_credential("onepassword", Some(&cred.account))
                .await
            {
                if let Some(token) = fields.get("service_account_token") {
                    return Ok(token.clone());
                }
            }
        }
    }

    Err(error(
        Value::Null,
        -32013,
        "No 1Password credentials found. Store a token via Auth > 1Password in the Tairseach UI.",
    ))
}

/// Locate the op-helper binary
fn get_op_helper_path() -> Result<PathBuf, String> {
    // Try relative to executable first (bundled app: Contents/MacOS/op-helper)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let bundled_path = exe_dir.join("op-helper");
            if bundled_path.exists() {
                return Ok(bundled_path);
            }
        }
    }

    // Development fallback: src-tauri/bin/op-helper
    let dev_path = PathBuf::from("src-tauri/bin/op-helper");
    if dev_path.exists() {
        return Ok(dev_path);
    }

    Err("op-helper binary not found".to_string())
}

/// Call the Go helper binary
async fn call_op_helper(method: &str, token: &str, params: Value) -> Result<Value, String> {
    let helper_path = get_op_helper_path()?;

    let request = serde_json::json!({
        "method": method,
        "token": token,
        "params": params
    });

    let request_line = serde_json::to_string(&request).map_err(|e| format!("JSON serialize error: {}", e))?;

    debug!("Spawning op-helper: {:?}", helper_path);
    debug!("Request: {}", request_line);

    let mut child = Command::new(&helper_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn op-helper: {}", e))?;

    // Write request to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(request_line.as_bytes())
            .await
            .map_err(|e| format!("Failed to write to stdin: {}", e))?;
        stdin
            .write_all(b"\n")
            .await
            .map_err(|e| format!("Failed to write newline: {}", e))?;
    }

    // Read response with timeout
    let output = timeout(Duration::from_secs(10), child.wait_with_output())
        .await
        .map_err(|_| "op-helper timeout (10s)".to_string())?
        .map_err(|e| format!("Failed to wait for op-helper: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "op-helper exited with code {}: {}",
            output.status.code().unwrap_or(-1),
            stderr
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    debug!("Response: {}", stdout);

    let response: serde_json::Value = serde_json::from_str(stdout.trim())
        .map_err(|e| format!("Failed to parse response JSON: {}", e))?;

    if let Some(ok) = response.get("ok").and_then(|v| v.as_bool()) {
        if ok {
            if let Some(result) = response.get("result") {
                return Ok(result.clone());
            } else {
                return Ok(Value::Null);
            }
        } else if let Some(error) = response.get("error").and_then(|v| v.as_str()) {
            return Err(error.to_string());
        }
    }

    Err("Invalid response format from op-helper".to_string())
}

/// Handle 1Password-related methods
pub async fn handle(action: &str, params: &Value, id: Value) -> JsonRpcResponse {
    // Config-related methods don't need auth
    match action {
        "config.defaultVault" => return handle_get_default_vault(id).await,
        "config.setDefaultVault" | "vaults.setDefault" => {
            return handle_set_default_vault(params, id).await
        }
        _ => {}
    }

    let token = match get_sa_token(params).await {
        Ok(t) => t,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };

    match action {
        "status" => ok(id, serde_json::json!({"status": "ok", "backend": "go-helper"})),
        "vaults.list" | "vaults_list" => handle_vaults_list(&token, params, id).await,
        "items.list" | "items_list" => handle_items_list(&token, params, id).await,
        "items.get" | "items_get" => handle_items_get(&token, params, id).await,
        "secrets.resolve" => handle_secrets_resolve(&token, params, id).await,
        _ => method_not_found(id, &format!("op.{}", action)),
    }
}

async fn handle_vaults_list(token: &str, params: &Value, id: Value) -> JsonRpcResponse {
    info!("Handling op.vaults.list via Go helper");

    match call_op_helper("vaults.list", token, params.clone()).await {
        Ok(result) => {
            debug!("Retrieved vaults via Go helper");
            ok(id, result)
        }
        Err(e) => {
            error!("Failed to list vaults: {}", e);
            generic_error(id, e)
        }
    }
}

async fn handle_items_list(token: &str, params: &Value, id: Value) -> JsonRpcResponse {
    info!("Handling op.items.list via Go helper");

    let vault_id = match get_vault_id_with_default(params).await {
        Ok(v) => v,
        Err(e) => return invalid_params(id, &e),
    };

    let helper_params = serde_json::json!({
        "vault_id": vault_id
    });

    match call_op_helper("items.list", token, helper_params).await {
        Ok(result) => {
            debug!("Retrieved items for vault {}", vault_id);
            ok(id, result)
        }
        Err(e) => {
            error!("Failed to list items: {}", e);
            generic_error(id, e)
        }
    }
}

async fn handle_items_get(token: &str, params: &Value, id: Value) -> JsonRpcResponse {
    info!("Handling op.items.get via Go helper");

    let vault_id = match get_vault_id_with_default(params).await {
        Ok(v) => v,
        Err(e) => return invalid_params(id, &e),
    };

    let item_id = match require_string_or(params, "item_id", "itemId", &id) {
        Ok(v) => v,
        Err(response) => return response,
    };

    let helper_params = serde_json::json!({
        "vault_id": vault_id,
        "item_id": item_id
    });

    match call_op_helper("items.get", token, helper_params).await {
        Ok(result) => ok(id, result),
        Err(e) => {
            error!("Failed to get item: {}", e);
            generic_error(id, e)
        }
    }
}

async fn handle_secrets_resolve(token: &str, params: &Value, id: Value) -> JsonRpcResponse {
    info!("Handling op.secrets.resolve via Go helper");

    let reference = match require_string(params, "reference", &id) {
        Ok(r) => r,
        Err(response) => return response,
    };

    let helper_params = serde_json::json!({
        "reference": reference
    });

    match call_op_helper("secrets.resolve", token, helper_params).await {
        Ok(result) => ok(id, result),
        Err(e) => {
            error!("Failed to resolve secret: {}", e);
            generic_error(id, e)
        }
    }
}

async fn handle_get_default_vault(id: Value) -> JsonRpcResponse {
    info!("Handling op.config.defaultVault");
    match crate::config::get_onepassword_config().await {
        Ok(Some(config)) => ok(
            id,
            serde_json::json!({
                "default_vault_id": config.default_vault_id,
            }),
        ),
        Ok(None) => ok(
            id,
            serde_json::json!({
                "default_vault_id": null,
            }),
        ),
        Err(e) => generic_error(id, e),
    }
}

async fn handle_set_default_vault(params: &Value, id: Value) -> JsonRpcResponse {
    info!("Handling op.vaults.setDefault");
    let vault_id = optional_string_or(params, "vault_id", "vaultId").map(String::from);

    match crate::config::save_onepassword_config(vault_id.clone()).await {
        Ok(()) => {
            info!("Set default vault to: {:?}", vault_id);
            ok(
                id,
                serde_json::json!({
                    "default_vault_id": vault_id,
                    "success": true,
                }),
            )
        }
        Err(e) => generic_error(id, e),
    }
}

/// Helper to get vault ID from params or fall back to default vault
async fn get_vault_id_with_default(params: &Value) -> Result<String, String> {
    if let Some(vault_id) = optional_string_or(params, "vault_id", "vaultId") {
        return Ok(vault_id.to_string());
    }

    match crate::config::get_onepassword_config().await {
        Ok(Some(config)) => match config.default_vault_id {
            Some(vault_id) => Ok(vault_id),
            None => Err("No vault_id provided and no default vault configured".to_string()),
        },
        Ok(None) => Err("No vault_id provided and no default vault configured".to_string()),
        Err(e) => Err(format!("Failed to get default vault config: {}", e)),
    }
}
