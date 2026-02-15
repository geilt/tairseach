//! Auth Handler
//!
//! Socket proxy handler for the `auth.*` JSON-RPC namespace.
//! Delegates to the AuthBroker for all operations.

use serde_json::Value;
use tracing::error;

use super::common::*;
use super::super::protocol::JsonRpcResponse;
use crate::auth::{get_or_init_broker, TokenRecord};

/// Get the shared auth broker instance.
async fn get_broker() -> Result<std::sync::Arc<crate::auth::AuthBroker>, JsonRpcResponse> {
    get_or_init_broker()
        .await
        .map_err(|e| {
            error!("Failed to initialise auth broker: {}", e);
            error(
                Value::Null,
                crate::auth::error_codes::MASTER_KEY_NOT_INITIALIZED,
                format!("Auth broker init failed: {}", e),
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
        // Credential type registry
        "credential_types" | "credentialTypes" => handle_credential_types(params, id).await,
        "credential_types.custom.create" => handle_create_custom_type(params, id).await,
        // Credential CRUD
        "credentials.store" => handle_store_credential(params, id).await,
        "credentials.get" => handle_get_credential(params, id).await,
        "credentials.list" => handle_list_credentials(params, id).await,
        "credentials.delete" => handle_delete_credential(params, id).await,
        "credentials.rename" => handle_rename_credential(params, id).await,
        _ => method_not_found(id, &format!("auth.{}", action)),
    }
}

// ── Handlers ────────────────────────────────────────────────────────────────

/// `auth.status` — subsystem health check.
async fn handle_status(_params: &Value, id: Value) -> JsonRpcResponse {
    match get_broker().await {
        Ok(broker) => {
            let status = broker.status().await;
            ok(id, serde_json::to_value(status).unwrap_or_default())
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
            ok(id, serde_json::json!({ "providers": providers }))
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

    let provider_filter = optional_string(params, "provider");
    let accounts = broker.list_accounts(provider_filter).await;

    ok(
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

    let provider = match require_string(params, "provider", &id) {
        Ok(p) => p,
        Err(response) => return response,
    };

    let account = match require_string(params, "account", &id) {
        Ok(a) => a,
        Err(response) => return response,
    };

    let scopes = optional_string_array(params, "scopes");

    match broker
        .get_token(provider, account, scopes.as_deref())
        .await
    {
        Ok(token_info) => ok(id, token_info),
        Err((code, msg)) => error(id, code, msg),
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

    let provider = match require_string(params, "provider", &id) {
        Ok(p) => p,
        Err(response) => return response,
    };

    let account = match require_string(params, "account", &id) {
        Ok(a) => a,
        Err(response) => return response,
    };

    match broker.force_refresh(provider, account).await {
        Ok(token_info) => ok(id, token_info),
        Err((code, msg)) => error(id, code, msg),
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

    let provider = match require_string(params, "provider", &id) {
        Ok(p) => p,
        Err(response) => return response,
    };

    let account = match require_string(params, "account", &id) {
        Ok(a) => a,
        Err(response) => return response,
    };

    match broker.revoke_token(provider, account).await {
        Ok(()) => ok(id, serde_json::json!({ "success": true })),
        Err((code, msg)) => error(id, code, msg),
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
            return invalid_params(
                id,
                format!("Invalid token record: {}. Required fields: provider, account, access_token, refresh_token, token_type, expiry, scopes", e),
            )
        }
    };

    // Validation: known providers
    const KNOWN_PROVIDERS: &[&str] = &["google", "oura", "onepassword", "jira", "slack", "datadog"];
    if !KNOWN_PROVIDERS.contains(&record.provider.as_str()) {
        return invalid_params(
            id,
            format!("Unsupported provider '{}'. Supported: {:?}", record.provider, KNOWN_PROVIDERS),
        );
    }

    // Validation: provider "_internal" is reserved
    if record.provider == "_internal" {
        return invalid_params(
            id,
            "Provider '_internal' is reserved for system use and cannot be imported",
        );
    }

    // Validation: account must not be empty and must be reasonable length
    if record.account.is_empty() {
        return invalid_params(id, "Field 'account' must not be empty");
    }

    if record.account.len() > 256 {
        return invalid_params(id, "Field 'account' exceeds maximum length of 256 characters");
    }

    // Validation: access_token must not be empty
    if record.access_token.is_empty() {
        return invalid_params(id, "Field 'access_token' must not be empty");
    }

    match broker.store_token(record).await {
        Ok(()) => ok(id, serde_json::json!({ "success": true })),
        Err((code, msg)) => error(id, code, msg),
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
        Ok(passphrase) => ok(id, serde_json::json!({ "passphrase": passphrase })),
        Err((code, msg)) => error(id, code, msg),
    }
}

// ── Credential Type Registry Handlers ───────────────────────────────────────

/// `auth.credential_types` — list all known credential schemas
async fn handle_credential_types(_params: &Value, id: Value) -> JsonRpcResponse {
    let broker = match get_broker().await {
        Ok(b) => b,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };

    let types = broker.list_credential_types().await;
    ok(id, serde_json::json!({ "types": types }))
}

/// `auth.credential_types.custom.create` — register a custom credential type
async fn handle_create_custom_type(params: &Value, id: Value) -> JsonRpcResponse {
    let broker = match get_broker().await {
        Ok(b) => b,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };

    let schema: crate::auth::credential_types::CredentialTypeSchema = match serde_json::from_value(params.clone()) {
        Ok(s) => s,
        Err(e) => {
            return invalid_params(
                id,
                format!("Invalid credential type schema: {}. Required fields: provider_type, display_name, description, fields, supports_multiple", e),
            )
        }
    };

    match broker.register_custom_credential_type(schema).await {
        Ok(()) => ok(id, serde_json::json!({ "success": true })),
        Err(e) => generic_error(id, e),
    }
}

// ── Credential CRUD Handlers ────────────────────────────────────────────────

/// `auth.credentials.store` — store a credential
async fn handle_store_credential(params: &Value, id: Value) -> JsonRpcResponse {
    let broker = match get_broker().await {
        Ok(b) => b,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };

    let provider = match require_string(params, "provider", &id) {
        Ok(p) => p,
        Err(response) => return response,
    };

    let cred_type = match require_string(params, "type", &id) {
        Ok(t) => t,
        Err(response) => return response,
    };

    let label = optional_string(params, "label");
    let account = label.unwrap_or("default");

    let fields = match params.get("fields").and_then(|v| v.as_object()) {
        Some(obj) => {
            let mut map = std::collections::HashMap::new();
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    map.insert(k.clone(), s.to_string());
                }
            }
            map
        }
        None => return invalid_params(id, "Missing or invalid 'fields' parameter"),
    };

    match broker
        .store_credential(provider, account, cred_type, fields, label)
        .await
    {
        Ok(()) => ok(id, serde_json::json!({ "success": true })),
        Err(e) => generic_error(id, e),
    }
}

/// `auth.credentials.get` — retrieve a credential (uses resolution chain)
async fn handle_get_credential(params: &Value, id: Value) -> JsonRpcResponse {
    let broker = match get_broker().await {
        Ok(b) => b,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };

    let provider = match require_string(params, "provider", &id) {
        Ok(p) => p,
        Err(response) => return response,
    };

    let label = optional_string(params, "label");

    match broker.get_credential(provider, label).await {
        Ok(fields) => ok(id, serde_json::json!({ "fields": fields })),
        Err(e) => generic_error(id, e),
    }
}

/// `auth.credentials.list` — list all credentials (metadata only, no secrets)
async fn handle_list_credentials(_params: &Value, id: Value) -> JsonRpcResponse {
    let broker = match get_broker().await {
        Ok(b) => b,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };

    let credentials = broker.list_credentials().await;
    ok(
        id,
        serde_json::json!({
            "credentials": credentials,
            "count": credentials.len(),
        }),
    )
}

/// `auth.credentials.delete` — delete a credential
async fn handle_delete_credential(params: &Value, id: Value) -> JsonRpcResponse {
    let broker = match get_broker().await {
        Ok(b) => b,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };

    let provider = match require_string(params, "provider", &id) {
        Ok(p) => p,
        Err(response) => return response,
    };

    let label = optional_string(params, "label");
    let account = label.unwrap_or("default");

    match broker.delete_credential(provider, account).await {
        Ok(()) => ok(id, serde_json::json!({ "success": true })),
        Err(e) => generic_error(id, e),
    }
}

/// `auth.credentials.rename` — rename a credential
async fn handle_rename_credential(params: &Value, id: Value) -> JsonRpcResponse {
    let broker = match get_broker().await {
        Ok(b) => b,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };

    let cred_type = match require_string(params, "credType", &id) {
        Ok(t) => t,
        Err(response) => return response,
    };

    let old_label = match require_string(params, "oldLabel", &id) {
        Ok(l) => l,
        Err(response) => return response,
    };

    let new_label = match require_string(params, "newLabel", &id) {
        Ok(l) => l,
        Err(response) => return response,
    };

    // Get the existing credential
    let fields = match broker.get_credential(&cred_type, Some(&old_label)).await {
        Ok(f) => f,
        Err(e) => return generic_error(id, e),
    };

    // Store with new label
    match broker
        .store_credential(&cred_type, &new_label, &cred_type, fields, Some(&new_label))
        .await
    {
        Ok(()) => {
            // Delete old label
            match broker.delete_credential(&cred_type, &old_label).await {
                Ok(()) => ok(
                    id,
                    serde_json::json!({ "success": true, "label": new_label }),
                ),
                Err(e) => generic_error(id, e),
            }
        }
        Err(e) => generic_error(id, e),
    }
}
