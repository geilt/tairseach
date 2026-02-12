//! 1Password Handler
//!
//! Socket handlers for 1Password Service Account API methods.
//! Retrieves service account token from auth broker.

use serde_json::Value;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::{debug, error, info};

use super::super::protocol::JsonRpcResponse;
use crate::auth::AuthBroker;

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

/// 1Password Service Account API client
struct OnePasswordApi {
    token: String,
    client: reqwest::Client,
}

impl OnePasswordApi {
    const API_BASE_URL: &'static str = "https://api.1password.com";

    fn new(token: String) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        Ok(Self {
            token,
            client,
        })
    }

    async fn get(&self, path: &str) -> Result<Value, String> {
        let url = format!("{}{}", Self::API_BASE_URL, path);
        
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());
            error!("1Password API error {}: {}", status, body);
            return Err(format!("HTTP {} error: {}", status, body));
        }

        // Handle empty responses (e.g. heartbeat returns 200 with no body)
        let text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {}", e))?;
        
        if text.is_empty() {
            return Ok(serde_json::json!({"status": "ok"}));
        }
        
        serde_json::from_str(&text)
            .map_err(|e| format!("Failed to parse JSON response: {} (body: {})", e, &text[..text.len().min(200)]))
    }

    async fn post(&self, path: &str, body: Value) -> Result<Value, String> {
        let url = format!("{}{}", Self::API_BASE_URL, path);
        
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());
            error!("1Password API error {}: {}", status, body);
            return Err(format!("HTTP {} error: {}", status, body));
        }

        response
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON response: {}", e))
    }

    /// Check API status
    async fn status(&self) -> Result<Value, String> {
        self.get("/v1/heartbeat").await
    }

    /// List vaults
    async fn list_vaults(&self) -> Result<Value, String> {
        self.get("/v1/vaults").await
    }

    /// List items in a vault
    async fn list_items(&self, vault_id: &str) -> Result<Value, String> {
        self.get(&format!("/v1/vaults/{}/items", vault_id)).await
    }

    /// Get a specific item
    async fn get_item(&self, vault_id: &str, item_id: &str) -> Result<Value, String> {
        self.get(&format!("/v1/vaults/{}/items/{}", vault_id, item_id))
            .await
    }

    /// Create an item
    async fn create_item(&self, vault_id: &str, item: Value) -> Result<Value, String> {
        self.post(&format!("/v1/vaults/{}/items", vault_id), item)
            .await
    }
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

    // Retrieve 1Password SA token — try explicit account, then "default", then first available
    let explicit_account = params
        .get("account")
        .and_then(|v| v.as_str());

    let access_token = {
        let mut attempts: Vec<String> = Vec::new();
        if let Some(acct) = explicit_account {
            attempts.push(acct.to_string());
        }
        attempts.push("default".to_string());
        
        let mut found = None;
        for acct in &attempts {
            if let Ok(fields) = auth_broker.get_credential("onepassword", Some(acct)).await {
                if let Some(token) = fields.get("service_account_token") {
                    found = Some(token.clone());
                    break;
                }
            }
        }
        
        // Fallback: list all onepassword credentials and use first match
        if found.is_none() {
            let all_creds = auth_broker.list_credentials().await;
            for cred in &all_creds {
                if cred.cred_type == "onepassword" {
                    if let Ok(fields) = auth_broker.get_credential("onepassword", Some(&cred.account)).await {
                        if let Some(token) = fields.get("service_account_token") {
                            found = Some(token.clone());
                            break;
                        }
                    }
                }
            }
        }
        
        match found {
            Some(token) => token,
            None => {
                return JsonRpcResponse::error(
                    id,
                    -32013,
                    "No 1Password credentials found. Store a token via Auth > 1Password in the Tairseach UI.".to_string(),
                    None,
                );
            }
        }
    };

    // Create API client
    let api = match OnePasswordApi::new(access_token) {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create 1Password API client: {}", e);
            return JsonRpcResponse::error(id, -32000, e, None);
        }
    };

    // Dispatch to specific handler
    match action {
        "status" => handle_status(id, api).await,
        "vaults.list" | "vaults_list" => handle_list_vaults(id, api).await,
        "items.list" | "items_list" => handle_list_items(params, id, api).await,
        "items.get" | "items_get" => handle_get_item(params, id, api).await,
        "items.create" | "items_create" => handle_create_item(params, id, api).await,
        _ => JsonRpcResponse::method_not_found(id, &format!("op.{}", action)),
    }
}

async fn handle_status(id: Value, api: OnePasswordApi) -> JsonRpcResponse {
    info!("Handling op.status");

    match api.status().await {
        Ok(status) => JsonRpcResponse::success(id, status),
        Err(e) => {
            error!("Failed to get 1Password status: {}", e);
            JsonRpcResponse::error(id, -32000, e, None)
        }
    }
}

async fn handle_list_vaults(id: Value, api: OnePasswordApi) -> JsonRpcResponse {
    info!("Handling op.vaults.list");

    match api.list_vaults().await {
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

async fn handle_list_items(params: &Value, id: Value, api: OnePasswordApi) -> JsonRpcResponse {
    info!("Handling op.items.list");

    let vault_id = match get_vault_id_with_default(params).await {
        Ok(v) => v,
        Err(e) => return JsonRpcResponse::invalid_params(id, &e),
    };

    match api.list_items(&vault_id).await {
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

async fn handle_get_item(params: &Value, id: Value, api: OnePasswordApi) -> JsonRpcResponse {
    info!("Handling op.items.get");

    let vault_id = match get_vault_id_with_default(params).await {
        Ok(v) => v,
        Err(e) => return JsonRpcResponse::invalid_params(id, &e),
    };

    let item_id = match params.get("item_id").or_else(|| params.get("itemId")).and_then(|v| v.as_str()) {
        Some(v) => v,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing required parameter: item_id");
        }
    };

    match api.get_item(&vault_id, item_id).await {
        Ok(item) => JsonRpcResponse::success(id, item),
        Err(e) => {
            error!("Failed to get item: {}", e);
            JsonRpcResponse::error(id, -32000, e, None)
        }
    }
}

async fn handle_create_item(params: &Value, id: Value, api: OnePasswordApi) -> JsonRpcResponse {
    info!("Handling op.items.create");

    let vault_id = match get_vault_id_with_default(params).await {
        Ok(v) => v,
        Err(e) => return JsonRpcResponse::invalid_params(id, &e),
    };

    let item = match params.get("item") {
        Some(v) => v.clone(),
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing required parameter: item");
        }
    };

    match api.create_item(&vault_id, item).await {
        Ok(created) => {
            info!("Item created successfully");
            JsonRpcResponse::success(id, created)
        }
        Err(e) => {
            error!("Failed to create item: {}", e);
            JsonRpcResponse::error(id, -32000, e, None)
        }
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
    // First check if vault_id is provided in params
    if let Some(vault_id) = params
        .get("vault_id")
        .or_else(|| params.get("vaultId"))
        .and_then(|v| v.as_str())
    {
        return Ok(vault_id.to_string());
    }
    
    // Fall back to default vault
    match crate::config::get_onepassword_config().await {
        Ok(Some(config)) => match config.default_vault_id {
            Some(vault_id) => Ok(vault_id),
            None => Err("No vault_id provided and no default vault configured".to_string()),
        },
        Ok(None) => Err("No vault_id provided and no default vault configured".to_string()),
        Err(e) => Err(format!("Failed to get default vault config: {}", e)),
    }
}
