//! 1Password Handler
//!
//! Socket handlers for 1Password Service Account API methods.
//! Uses the 1Password REST API directly via reqwest (no FFI/SDK dependency).
//!
//! API Reference: https://developer.1password.com/docs/service-accounts/

use serde_json::Value;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::{debug, error, info};

use super::super::protocol::JsonRpcResponse;
use crate::auth::AuthBroker;

/// 1Password Connect/SA API base URL (extracted from token's `aud` claim)
const DEFAULT_API_BASE: &str = "https://events.1password.com";

/// Global auth broker instance.
static AUTH_BROKER: OnceCell<Arc<AuthBroker>> = OnceCell::const_new();

/// Get or initialise the auth broker.
async fn get_broker() -> Result<&'static Arc<AuthBroker>, JsonRpcResponse> {
    AUTH_BROKER
        .get_or_try_init(|| async {
            match AuthBroker::new().await {
                Ok(broker) => {
                    broker.spawn_refresh_daemon();
                    Ok(broker)
                }
                Err(e) => Err(e),
            }
        })
        .await
        .map_err(|e| {
            error!("Failed to initialise auth broker: {}", e);
            JsonRpcResponse::error(
                Value::Null,
                crate::auth::error_codes::MASTER_KEY_NOT_INITIALIZED,
                format!("Auth broker init failed: {}", e),
                None,
            )
        })
}

/// Extract the API base URL from a 1Password Service Account JWT token
fn extract_api_base(token: &str) -> String {
    // SA tokens are JWTs prefixed with "ops_". The `aud` claim contains the API URL.
    let jwt_part = if token.starts_with("ops_") {
        &token[4..]
    } else {
        token
    };

    // Split JWT and decode payload (second part)
    if let Some(payload_b64) = jwt_part.split('.').nth(1) {
        // JWT uses base64url encoding (no padding)
        use base64::Engine;
        let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        if let Ok(payload_bytes) = engine.decode(payload_b64) {
            if let Ok(payload) = serde_json::from_slice::<Value>(&payload_bytes) {
                if let Some(aud) = payload.get("aud").and_then(|v| v.as_array()) {
                    // aud is an array of URLs; use the first HTTPS one
                    for url in aud {
                        if let Some(url_str) = url.as_str() {
                            if url_str.starts_with("https://") {
                                return url_str.trim_end_matches('/').to_string();
                            }
                        }
                    }
                } else if let Some(aud) = payload.get("aud").and_then(|v| v.as_str()) {
                    if aud.starts_with("https://") {
                        return aud.trim_end_matches('/').to_string();
                    }
                }
            }
        }
    }

    DEFAULT_API_BASE.to_string()
}

/// Make an authenticated request to the 1Password API
async fn op_request(
    method: reqwest::Method,
    token: &str,
    path: &str,
) -> Result<Value, String> {
    let base = extract_api_base(token);
    let url = format!("{}{}", base, path);

    let client = reqwest::Client::new();
    let resp = client
        .request(method, &url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    if !status.is_success() {
        return Err(format!("1Password API error ({}): {}", status.as_u16(), body));
    }

    serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse JSON response: {} (body: {})", e, &body[..body.len().min(200)]))
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

    let auth_broker = match get_broker().await {
        Ok(broker) => broker,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };

    // Retrieve 1Password SA token
    let access_token = match get_sa_token(auth_broker, params).await {
        Ok(token) => token,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };

    // Dispatch to specific handler
    match action {
        "status" => handle_status(id, &access_token).await,
        "vaults.list" | "vaults_list" => handle_list_vaults(id, &access_token).await,
        "items.list" | "items_list" => handle_list_items(params, id, &access_token).await,
        "items.get" | "items_get" => handle_get_item(params, id, &access_token).await,
        "items.create" | "items_create" => handle_create_item(params, id, &access_token).await,
        _ => JsonRpcResponse::method_not_found(id, &format!("op.{}", action)),
    }
}

/// Retrieve the SA token from the credential store
async fn get_sa_token(
    auth_broker: &Arc<AuthBroker>,
    params: &Value,
) -> Result<String, JsonRpcResponse> {
    let explicit_account = params.get("account").and_then(|v| v.as_str());

    let mut attempts: Vec<String> = Vec::new();
    if let Some(acct) = explicit_account {
        attempts.push(acct.to_string());
    }
    attempts.push("default".to_string());

    for acct in &attempts {
        if let Ok(fields) = auth_broker.get_credential("onepassword", Some(acct)).await {
            if let Some(token) = fields.get("service_account_token") {
                if !token.is_empty() {
                    return Ok(token.clone());
                }
            }
        }
    }

    // Fallback: list all onepassword credentials and use first match
    let all_creds = auth_broker.list_credentials().await;
    for cred in &all_creds {
        if cred.cred_type == "onepassword" {
            if let Ok(fields) = auth_broker
                .get_credential("onepassword", Some(&cred.account))
                .await
            {
                if let Some(token) = fields.get("service_account_token") {
                    if !token.is_empty() {
                        return Ok(token.clone());
                    }
                }
            }
        }
    }

    Err(JsonRpcResponse::error(
        Value::Null,
        -32013,
        "No 1Password credentials found. Store a token via Auth > 1Password in the Tairseach UI."
            .to_string(),
        None,
    ))
}

async fn handle_status(id: Value, token: &str) -> JsonRpcResponse {
    info!("Handling op.status");

    // Verify token by listing vaults (lightweight API call)
    match op_request(reqwest::Method::GET, token, "/v1/vaults").await {
        Ok(_) => JsonRpcResponse::success(
            id,
            serde_json::json!({"status": "ok", "authenticated": true}),
        ),
        Err(e) => {
            // Token exists but may be invalid
            JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "status": "error",
                    "authenticated": false,
                    "error": e,
                }),
            )
        }
    }
}

async fn handle_list_vaults(id: Value, token: &str) -> JsonRpcResponse {
    info!("Handling op.vaults.list");

    match op_request(reqwest::Method::GET, token, "/v1/vaults").await {
        Ok(vaults) => {
            debug!("Retrieved vaults");
            JsonRpcResponse::success(id, vaults)
        }
        Err(e) => {
            error!("Failed to list vaults: {}", e);
            JsonRpcResponse::error(id, -32000, e, None)
        }
    }
}

async fn handle_list_items(params: &Value, id: Value, token: &str) -> JsonRpcResponse {
    info!("Handling op.items.list");

    let vault_id = match get_vault_id_with_default(params).await {
        Ok(v) => v,
        Err(e) => return JsonRpcResponse::invalid_params(id, &e),
    };

    match op_request(
        reqwest::Method::GET,
        token,
        &format!("/v1/vaults/{}/items", vault_id),
    )
    .await
    {
        Ok(items) => {
            debug!("Retrieved items for vault {}", vault_id);
            JsonRpcResponse::success(id, items)
        }
        Err(e) => {
            error!("Failed to list items: {}", e);
            JsonRpcResponse::error(id, -32000, e, None)
        }
    }
}

async fn handle_get_item(params: &Value, id: Value, token: &str) -> JsonRpcResponse {
    info!("Handling op.items.get");

    let vault_id = match get_vault_id_with_default(params).await {
        Ok(v) => v,
        Err(e) => return JsonRpcResponse::invalid_params(id, &e),
    };

    let item_id = match params
        .get("item_id")
        .or_else(|| params.get("itemId"))
        .and_then(|v| v.as_str())
    {
        Some(v) => v,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing required parameter: item_id");
        }
    };

    match op_request(
        reqwest::Method::GET,
        token,
        &format!("/v1/vaults/{}/items/{}", vault_id, item_id),
    )
    .await
    {
        Ok(item) => JsonRpcResponse::success(id, item),
        Err(e) => {
            error!("Failed to get item: {}", e);
            JsonRpcResponse::error(id, -32000, e, None)
        }
    }
}

async fn handle_create_item(params: &Value, id: Value, token: &str) -> JsonRpcResponse {
    info!("Handling op.items.create");

    let vault_id = match get_vault_id_with_default(params).await {
        Ok(v) => v,
        Err(e) => return JsonRpcResponse::invalid_params(id, &e),
    };

    let item = match params.get("item") {
        Some(v) => v,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing required parameter: item");
        }
    };

    let base = extract_api_base(token);
    let url = format!("{}/v1/vaults/{}/items", base, vault_id);

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(item)
        .send()
        .await;

    match resp {
        Ok(r) => {
            let status = r.status();
            let body = r.text().await.unwrap_or_default();
            if status.is_success() {
                match serde_json::from_str::<Value>(&body) {
                    Ok(v) => JsonRpcResponse::success(id, v),
                    Err(_) => JsonRpcResponse::success(id, serde_json::json!({"created": true})),
                }
            } else {
                JsonRpcResponse::error(id, -32000, format!("API error ({}): {}", status, body), None)
            }
        }
        Err(e) => JsonRpcResponse::error(id, -32000, format!("Request failed: {}", e), None),
    }
}

async fn handle_get_default_vault(id: Value) -> JsonRpcResponse {
    info!("Handling op.config.defaultVault");

    match crate::config::get_onepassword_config().await {
        Ok(Some(config)) => JsonRpcResponse::success(
            id,
            serde_json::json!({
                "default_vault_id": config.default_vault_id,
            }),
        ),
        Ok(None) => JsonRpcResponse::success(
            id,
            serde_json::json!({
                "default_vault_id": null,
            }),
        ),
        Err(e) => JsonRpcResponse::error(id, -32000, e, None),
    }
}

async fn handle_set_default_vault(params: &Value, id: Value) -> JsonRpcResponse {
    info!("Handling op.vaults.setDefault");

    let vault_id = params
        .get("vault_id")
        .or_else(|| params.get("vaultId"))
        .and_then(|v| v.as_str())
        .map(String::from);

    match crate::config::save_onepassword_config(vault_id.clone()).await {
        Ok(()) => {
            info!("Set default vault to: {:?}", vault_id);
            JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "default_vault_id": vault_id,
                    "success": true,
                }),
            )
        }
        Err(e) => JsonRpcResponse::error(id, -32000, e, None),
    }
}

/// Helper to get vault ID from params or fall back to default vault
async fn get_vault_id_with_default(params: &Value) -> Result<String, String> {
    if let Some(vault_id) = params
        .get("vault_id")
        .or_else(|| params.get("vaultId"))
        .and_then(|v| v.as_str())
    {
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
