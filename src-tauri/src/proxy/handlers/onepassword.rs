//! 1Password Handler
//!
//! Socket handlers for 1Password Connect API methods.
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

/// 1Password Connect API client
struct OnePasswordApi {
    token: String,
    connect_host: String,
    client: reqwest::Client,
}

impl OnePasswordApi {
    fn new(token: String, connect_host: String) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        Ok(Self {
            token,
            connect_host,
            client,
        })
    }

    async fn get(&self, path: &str) -> Result<Value, String> {
        let url = format!("{}{}", self.connect_host, path);
        
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

        response
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON response: {}", e))
    }

    async fn post(&self, path: &str, body: Value) -> Result<Value, String> {
        let url = format!("{}{}", self.connect_host, path);
        
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
    let auth_broker = match get_broker().await {
        Ok(broker) => broker,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };

    // Retrieve 1Password token from auth broker
    let account = match params.get("account").and_then(|v| v.as_str()) {
        Some(acc) => acc,
        None => {
            return JsonRpcResponse::invalid_params(
                id,
                "Missing required parameter: account (e.g., 'default' or service account name)",
            );
        }
    };

    let token_data = match auth_broker
        .get_token("onepassword", account, None)
        .await
    {
        Ok(data) => data,
        Err((code, msg)) => {
            error!("Failed to get 1Password token: {}", msg);
            return JsonRpcResponse::error(id, code, msg, None);
        }
    };

    let access_token = match token_data.get("access_token").and_then(|v| v.as_str()) {
        Some(token) => token.to_string(),
        None => {
            return JsonRpcResponse::error(
                id,
                -32000,
                "Invalid token response: missing access_token".to_string(),
                None,
            );
        }
    };

    // Get connect_host from params or use default
    let connect_host = params
        .get("connect_host")
        .and_then(|v| v.as_str())
        .unwrap_or("http://localhost:8080")
        .to_string();

    // Create API client
    let api = match OnePasswordApi::new(access_token, connect_host) {
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

    let vault_id = match params.get("vault_id").or_else(|| params.get("vaultId")).and_then(|v| v.as_str()) {
        Some(v) => v,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing required parameter: vault_id");
        }
    };

    match api.list_items(vault_id).await {
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

    let vault_id = match params.get("vault_id").or_else(|| params.get("vaultId")).and_then(|v| v.as_str()) {
        Some(v) => v,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing required parameter: vault_id");
        }
    };

    let item_id = match params.get("item_id").or_else(|| params.get("itemId")).and_then(|v| v.as_str()) {
        Some(v) => v,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing required parameter: item_id");
        }
    };

    match api.get_item(vault_id, item_id).await {
        Ok(item) => JsonRpcResponse::success(id, item),
        Err(e) => {
            error!("Failed to get item: {}", e);
            JsonRpcResponse::error(id, -32000, e, None)
        }
    }
}

async fn handle_create_item(params: &Value, id: Value, api: OnePasswordApi) -> JsonRpcResponse {
    info!("Handling op.items.create");

    let vault_id = match params.get("vault_id").or_else(|| params.get("vaultId")).and_then(|v| v.as_str()) {
        Some(v) => v,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing required parameter: vault_id");
        }
    };

    let item = match params.get("item") {
        Some(v) => v.clone(),
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing required parameter: item");
        }
    };

    match api.create_item(vault_id, item).await {
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
