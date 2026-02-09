//! Gmail Handler
//!
//! Socket handlers for Gmail API methods.
//! Retrieves OAuth tokens from auth broker and uses Google API client.

use serde_json::Value;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::{debug, error, info};

use super::super::protocol::JsonRpcResponse;
use crate::auth::AuthBroker;
use crate::google::GmailApi;

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

/// Handle Gmail-related methods
pub async fn handle(
    action: &str,
    params: &Value,
    id: Value,
) -> JsonRpcResponse {
    let auth_broker = match get_broker().await {
        Ok(broker) => broker,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };
    // Retrieve OAuth token from auth broker
    let (provider, account) = match extract_credentials(params) {
        Ok(creds) => creds,
        Err(response) => return response,
    };

    let token_data = match auth_broker
        .get_token(
            &provider,
            &account,
            Some(&[
                "https://www.googleapis.com/auth/gmail.modify".to_string(),
                "https://www.googleapis.com/auth/gmail.settings.basic".to_string(),
            ]),
        )
        .await
    {
        Ok(data) => data,
        Err((code, msg)) => {
            error!("Failed to get OAuth token: {}", msg);
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

    // Create Gmail API client
    let gmail = match GmailApi::new(access_token) {
        Ok(api) => api,
        Err(e) => {
            error!("Failed to create Gmail API client: {}", e);
            return JsonRpcResponse::error(id, -32000, e, None);
        }
    };

    // Dispatch to specific handler
    match action {
        "list_messages" | "listMessages" => handle_list_messages(params, id, gmail).await,
        "get_message" | "getMessage" => handle_get_message(params, id, gmail).await,
        "send" | "sendMessage" => handle_send_message(params, id, gmail).await,
        "list_labels" | "listLabels" => handle_list_labels(id, gmail).await,
        "modify_message" | "modifyMessage" => handle_modify_message(params, id, gmail).await,
        "trash_message" | "trashMessage" => handle_trash_message(params, id, gmail).await,
        "delete_message" | "deleteMessage" => handle_delete_message(params, id, gmail).await,
        _ => JsonRpcResponse::method_not_found(id, &format!("gmail.{}", action)),
    }
}

/// Extract credential provider and account from params
fn extract_credentials(params: &Value) -> Result<(String, String), JsonRpcResponse> {
    let provider = params
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("google")
        .to_string();

    let account = params
        .get("account")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            JsonRpcResponse::invalid_params(
                Value::Null,
                "Missing required parameter: account (Google email address)",
            )
        })?
        .to_string();

    Ok((provider, account))
}

async fn handle_list_messages(
    params: &Value,
    id: Value,
    gmail: GmailApi,
) -> JsonRpcResponse {
    info!("Handling gmail.list_messages");

    let query = params.get("query").and_then(|v| v.as_str());
    let max_results = params
        .get("maxResults")
        .or_else(|| params.get("max_results"))
        .and_then(|v| v.as_u64())
        .map(|n| n as usize);

    let label_ids = params
        .get("labelIds")
        .or_else(|| params.get("label_ids"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });

    match gmail.list_messages(query, max_results, label_ids).await {
        Ok(messages) => {
            debug!("Retrieved {} messages", messages.len());
            JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "messages": messages,
                    "count": messages.len(),
                }),
            )
        }
        Err(e) => {
            error!("Failed to list messages: {}", e);
            JsonRpcResponse::error(id, -32000, e, None)
        }
    }
}

async fn handle_get_message(
    params: &Value,
    id: Value,
    gmail: GmailApi,
) -> JsonRpcResponse {
    info!("Handling gmail.get_message");

    let message_id = match params
        .get("id")
        .or_else(|| params.get("messageId"))
        .and_then(|v| v.as_str())
    {
        Some(id) => id,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing required parameter: id");
        }
    };

    let format = params.get("format").and_then(|v| v.as_str());

    match gmail.get_message(message_id, format).await {
        Ok(message) => JsonRpcResponse::success(id, message),
        Err(e) => {
            error!("Failed to get message: {}", e);
            JsonRpcResponse::error(id, -32000, e, None)
        }
    }
}

async fn handle_send_message(
    params: &Value,
    id: Value,
    gmail: GmailApi,
) -> JsonRpcResponse {
    info!("Handling gmail.send");

    let to = match params.get("to").and_then(|v| v.as_array()) {
        Some(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect::<Vec<_>>(),
        None => {
            // Also try as a single string
            if let Some(to_str) = params.get("to").and_then(|v| v.as_str()) {
                vec![to_str.to_string()]
            } else {
                return JsonRpcResponse::invalid_params(id, "Missing required parameter: to");
            }
        }
    };

    let subject = match params.get("subject").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing required parameter: subject");
        }
    };

    let body = match params.get("body").and_then(|v| v.as_str()) {
        Some(b) => b,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing required parameter: body");
        }
    };

    let cc = params.get("cc").and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    });

    let bcc = params.get("bcc").and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    });

    match gmail.send_message(to, subject, body, cc, bcc).await {
        Ok(response) => {
            info!("Message sent successfully");
            JsonRpcResponse::success(id, response)
        }
        Err(e) => {
            error!("Failed to send message: {}", e);
            JsonRpcResponse::error(id, -32000, e, None)
        }
    }
}

async fn handle_list_labels(id: Value, gmail: GmailApi) -> JsonRpcResponse {
    info!("Handling gmail.list_labels");

    match gmail.list_labels().await {
        Ok(labels) => JsonRpcResponse::success(
            id,
            serde_json::json!({
                "labels": labels,
                "count": labels.len(),
            }),
        ),
        Err(e) => {
            error!("Failed to list labels: {}", e);
            JsonRpcResponse::error(id, -32000, e, None)
        }
    }
}

async fn handle_modify_message(
    params: &Value,
    id: Value,
    gmail: GmailApi,
) -> JsonRpcResponse {
    info!("Handling gmail.modify_message");

    let message_id = match params
        .get("id")
        .or_else(|| params.get("messageId"))
        .and_then(|v| v.as_str())
    {
        Some(id) => id,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing required parameter: id");
        }
    };

    let add_label_ids = params
        .get("addLabelIds")
        .or_else(|| params.get("add_label_ids"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });

    let remove_label_ids = params
        .get("removeLabelIds")
        .or_else(|| params.get("remove_label_ids"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });

    match gmail.modify_message(message_id, add_label_ids, remove_label_ids).await {
        Ok(message) => JsonRpcResponse::success(id, message),
        Err(e) => {
            error!("Failed to modify message: {}", e);
            JsonRpcResponse::error(id, -32000, e, None)
        }
    }
}

async fn handle_trash_message(
    params: &Value,
    id: Value,
    gmail: GmailApi,
) -> JsonRpcResponse {
    info!("Handling gmail.trash_message");

    let message_id = match params
        .get("id")
        .or_else(|| params.get("messageId"))
        .and_then(|v| v.as_str())
    {
        Some(id) => id,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing required parameter: id");
        }
    };

    match gmail.trash_message(message_id).await {
        Ok(response) => JsonRpcResponse::success(id, response),
        Err(e) => {
            error!("Failed to trash message: {}", e);
            JsonRpcResponse::error(id, -32000, e, None)
        }
    }
}

async fn handle_delete_message(
    params: &Value,
    id: Value,
    gmail: GmailApi,
) -> JsonRpcResponse {
    info!("Handling gmail.delete_message");

    let message_id = match params
        .get("id")
        .or_else(|| params.get("messageId"))
        .and_then(|v| v.as_str())
    {
        Some(id) => id,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing required parameter: id");
        }
    };

    match gmail.delete_message(message_id).await {
        Ok(_) => JsonRpcResponse::success(id, serde_json::json!({ "deleted": true })),
        Err(e) => {
            error!("Failed to delete message: {}", e);
            JsonRpcResponse::error(id, -32000, e, None)
        }
    }
}
