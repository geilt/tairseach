//! Google Contacts Handler
//!
//! Socket handlers for Google Contacts (People) API methods.
//! Retrieves OAuth tokens from auth broker and uses Google API client.
//!
//! Namespace: `gcontacts.*` (distinct from macOS native `contacts.*`)

use serde_json::Value;
use tracing::{error, info};

use super::common::*;
use super::super::protocol::JsonRpcResponse;

/// Handle Google Contacts-related methods
pub async fn handle(
    action: &str,
    params: &Value,
    id: Value,
) -> JsonRpcResponse {
    let auth_broker = match get_auth_broker().await {
        Ok(broker) => broker,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };
    
    // Retrieve OAuth token from auth broker
    let (provider, account) = match extract_oauth_credentials(params, "google") {
        Ok(creds) => creds,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };

    let token_data = match auth_broker
        .get_token(
            &provider,
            &account,
            Some(&["https://www.googleapis.com/auth/contacts".to_string()]),
        )
        .await
    {
        Ok(data) => data,
        Err((code, msg)) => {
            error!("Failed to get OAuth token for Google Contacts: {}", msg);
            return error(id, code, msg);
        }
    };

    let _access_token = match extract_access_token(&token_data, &id) {
        Ok(token) => token,
        Err(response) => return response,
    };

    // Dispatch to specific handler
    match action {
        "list" => handle_list(id).await,
        "search" => handle_search(params, id).await,
        "get" => handle_get(params, id).await,
        "create" => handle_create(params, id).await,
        "update" => handle_update(params, id).await,
        "delete" => handle_delete(params, id).await,
        _ => method_not_found(id, &format!("gcontacts.{}", action)),
    }
}

// ── Placeholder Handlers ────────────────────────────────────────────────────
// TODO: Implement Google Contacts API client (src-tauri/src/google/contacts_api.rs)
// and replace these stubs with actual API calls.

async fn handle_list(id: Value) -> JsonRpcResponse {
    info!("gcontacts.list called (not yet implemented)");
    error(
        id,
        -32000,
        "Google Contacts API not yet implemented. Use native 'contacts.*' for macOS Contacts.app",
    )
}

async fn handle_search(_params: &Value, id: Value) -> JsonRpcResponse {
    info!("gcontacts.search called (not yet implemented)");
    error(
        id,
        -32000,
        "Google Contacts API not yet implemented. Use native 'contacts.*' for macOS Contacts.app",
    )
}

async fn handle_get(_params: &Value, id: Value) -> JsonRpcResponse {
    info!("gcontacts.get called (not yet implemented)");
    error(
        id,
        -32000,
        "Google Contacts API not yet implemented. Use native 'contacts.*' for macOS Contacts.app",
    )
}

async fn handle_create(_params: &Value, id: Value) -> JsonRpcResponse {
    info!("gcontacts.create called (not yet implemented)");
    error(
        id,
        -32000,
        "Google Contacts API not yet implemented. Use native 'contacts.*' for macOS Contacts.app",
    )
}

async fn handle_update(_params: &Value, id: Value) -> JsonRpcResponse {
    info!("gcontacts.update called (not yet implemented)");
    error(
        id,
        -32000,
        "Google Contacts API not yet implemented. Use native 'contacts.*' for macOS Contacts.app",
    )
}

async fn handle_delete(_params: &Value, id: Value) -> JsonRpcResponse {
    info!("gcontacts.delete called (not yet implemented)");
    error(
        id,
        -32000,
        "Google Contacts API not yet implemented. Use native 'contacts.*' for macOS Contacts.app",
    )
}
