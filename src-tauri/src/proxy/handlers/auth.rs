//! Auth Handler
//!
//! Socket proxy handler for the `auth.*` JSON-RPC namespace.
//! Delegates to the AuthBroker for all operations.

use serde_json::Value;
use tracing::error;

use super::super::protocol::JsonRpcResponse;
use crate::auth::{get_or_init_broker, TokenRecord};

/// Get the shared auth broker instance.
async fn get_broker() -> Result<std::sync::Arc<crate::auth::AuthBroker>, JsonRpcResponse> {
    get_or_init_broker()
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

/// Handle auth-related methods.
pub async fn handle(action: &str, params: &Value, id: Value) -> JsonRpcResponse {
    match action {
        "status" => handle_status(params, id).await,
        "providers" => handle_providers(params, id).await,
        "accounts" | "list" => handle_accounts(params, id).await,
        "token" | "get" => handle_token(params, id).await,
        "refresh" => handle_refresh(params, id).await,
        "revoke" => handle_revoke(params, id).await,
        "store" | "import" => handle_store(params, id).await,
        "gogPassphrase" => handle_gog_passphrase(params, id).await,
        _ => JsonRpcResponse::method_not_found(id, &format!("auth.{}", action)),
    }
}

// ── Handlers ────────────────────────────────────────────────────────────────

/// `auth.status` — subsystem health check.
async fn handle_status(_params: &Value, id: Value) -> JsonRpcResponse {
    match get_broker().await {
        Ok(broker) => {
            let status = broker.status().await;
            JsonRpcResponse::success(id, serde_json::to_value(status).unwrap_or_default())
        }
        Err(mut resp) => {
            resp.id = id;
            resp
        }
    }
}

/// `auth.providers` — list supported providers.
async fn handle_providers(_params: &Value, id: Value) -> JsonRpcResponse {
    match get_broker().await {
        Ok(broker) => {
            let providers = broker.list_providers();
            JsonRpcResponse::success(id, serde_json::json!({ "providers": providers }))
        }
        Err(mut resp) => {
            resp.id = id;
            resp
        }
    }
}

/// `auth.accounts` / `auth.list` — list authorised accounts.
async fn handle_accounts(params: &Value, id: Value) -> JsonRpcResponse {
    let broker = match get_broker().await {
        Ok(b) => b,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };

    let provider_filter = params.get("provider").and_then(|v| v.as_str());
    let accounts = broker.list_accounts(provider_filter).await;

    JsonRpcResponse::success(
        id,
        serde_json::json!({
            "accounts": accounts,
            "count": accounts.len(),
        }),
    )
}

/// `auth.token` / `auth.get` — retrieve a valid access token (auto-refresh).
async fn handle_token(params: &Value, id: Value) -> JsonRpcResponse {
    let broker = match get_broker().await {
        Ok(b) => b,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };

    let provider = match params.get("provider").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return JsonRpcResponse::invalid_params(id, "Missing 'provider' parameter"),
    };

    let account = match params.get("account").and_then(|v| v.as_str()) {
        Some(a) => a,
        None => return JsonRpcResponse::invalid_params(id, "Missing 'account' parameter"),
    };

    let scopes: Option<Vec<String>> = params
        .get("scopes")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());

    match broker
        .get_token(provider, account, scopes.as_deref())
        .await
    {
        Ok(token_info) => JsonRpcResponse::success(id, token_info),
        Err((code, msg)) => JsonRpcResponse::error(id, code, msg, None),
    }
}

/// `auth.refresh` — force-refresh a token.
async fn handle_refresh(params: &Value, id: Value) -> JsonRpcResponse {
    let broker = match get_broker().await {
        Ok(b) => b,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };

    let provider = match params.get("provider").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return JsonRpcResponse::invalid_params(id, "Missing 'provider' parameter"),
    };

    let account = match params.get("account").and_then(|v| v.as_str()) {
        Some(a) => a,
        None => return JsonRpcResponse::invalid_params(id, "Missing 'account' parameter"),
    };

    match broker.force_refresh(provider, account).await {
        Ok(token_info) => JsonRpcResponse::success(id, token_info),
        Err((code, msg)) => JsonRpcResponse::error(id, code, msg, None),
    }
}

/// `auth.revoke` — revoke and remove an account's tokens.
async fn handle_revoke(params: &Value, id: Value) -> JsonRpcResponse {
    let broker = match get_broker().await {
        Ok(b) => b,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };

    let provider = match params.get("provider").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return JsonRpcResponse::invalid_params(id, "Missing 'provider' parameter"),
    };

    let account = match params.get("account").and_then(|v| v.as_str()) {
        Some(a) => a,
        None => return JsonRpcResponse::invalid_params(id, "Missing 'account' parameter"),
    };

    match broker.revoke_token(provider, account).await {
        Ok(()) => JsonRpcResponse::success(id, serde_json::json!({ "success": true })),
        Err((code, msg)) => JsonRpcResponse::error(id, code, msg, None),
    }
}

/// `auth.store` / `auth.import` — store or import a token directly.
async fn handle_store(params: &Value, id: Value) -> JsonRpcResponse {
    let broker = match get_broker().await {
        Ok(b) => b,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };

    // Parse the token record from params
    let record: TokenRecord = match serde_json::from_value(params.clone()) {
        Ok(r) => r,
        Err(e) => {
            return JsonRpcResponse::invalid_params(
                id,
                format!("Invalid token record: {}. Required fields: provider, account, access_token, refresh_token, token_type, expiry, scopes", e),
            )
        }
    };

    if record.provider.is_empty() || record.account.is_empty() {
        return JsonRpcResponse::invalid_params(
            id,
            "Both 'provider' and 'account' are required",
        );
    }

    match broker.store_token(record).await {
        Ok(()) => JsonRpcResponse::success(id, serde_json::json!({ "success": true })),
        Err((code, msg)) => JsonRpcResponse::error(id, code, msg, None),
    }
}

/// `auth.gogPassphrase` — retrieve the gog file-keyring passphrase.
async fn handle_gog_passphrase(_params: &Value, id: Value) -> JsonRpcResponse {
    let broker = match get_broker().await {
        Ok(b) => b,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };

    match broker.get_gog_passphrase().await {
        Ok(passphrase) => {
            JsonRpcResponse::success(id, serde_json::json!({ "passphrase": passphrase }))
        }
        Err((code, msg)) => JsonRpcResponse::error(id, code, msg, None),
    }
}
