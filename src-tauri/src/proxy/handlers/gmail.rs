//! Gmail Handler
//!
//! Socket handlers for Gmail API methods.
//! Retrieves OAuth tokens from auth broker and uses Google API client.

use serde_json::Value;
use tracing::{debug, error, info};

use super::common::*;
use super::super::protocol::JsonRpcResponse;
use crate::google::GmailApi;

/// Handle Gmail-related methods
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
            return error(id, code, msg);
        }
    };

    let access_token = match extract_access_token(&token_data, &id) {
        Ok(token) => token,
        Err(response) => return response,
    };

    // Create Gmail API client
    let gmail = match GmailApi::new(access_token) {
        Ok(api) => api,
        Err(e) => {
            error!("Failed to create Gmail API client: {}", e);
            return generic_error(id, e);
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
        _ => method_not_found(id, &format!("gmail.{}", action)),
    }
}

async fn handle_list_messages(
    params: &Value,
    id: Value,
    gmail: GmailApi,
) -> JsonRpcResponse {
    info!("Handling gmail.list_messages");

    let query = optional_string(params, "query");
    let max_results = optional_u64_or(params, "maxResults", "max_results").map(|n| n as usize);
    let label_ids = optional_string_array_or(params, "labelIds", "label_ids");

    match gmail.list_messages(query, max_results, label_ids).await {
        Ok(messages) => {
            debug!("Retrieved {} messages", messages.len());
            ok(
                id,
                serde_json::json!({
                    "messages": messages,
                    "count": messages.len(),
                }),
            )
        }
        Err(e) => {
            error!("Failed to list messages: {}", e);
            generic_error(id, e)
        }
    }
}

async fn handle_get_message(
    params: &Value,
    id: Value,
    gmail: GmailApi,
) -> JsonRpcResponse {
    info!("Handling gmail.get_message");

    let message_id = match require_string_or(params, "id", "messageId", &id) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let format = optional_string(params, "format");

    match gmail.get_message(message_id, format).await {
        Ok(message) => ok(id, message),
        Err(e) => {
            error!("Failed to get message: {}", e);
            generic_error(id, e)
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
                return invalid_params(id, "Missing required parameter: to");
            }
        }
    };

    let subject = match require_string(params, "subject", &id) {
        Ok(s) => s,
        Err(response) => return response,
    };

    let body = match require_string(params, "body", &id) {
        Ok(b) => b,
        Err(response) => return response,
    };

    let cc = optional_string_array(params, "cc");
    let bcc = optional_string_array(params, "bcc");

    match gmail.send_message(to, subject, body, cc, bcc).await {
        Ok(response) => {
            info!("Message sent successfully");
            ok(id, response)
        }
        Err(e) => {
            error!("Failed to send message: {}", e);
            generic_error(id, e)
        }
    }
}

async fn handle_list_labels(id: Value, gmail: GmailApi) -> JsonRpcResponse {
    info!("Handling gmail.list_labels");

    match gmail.list_labels().await {
        Ok(labels) => ok(
            id,
            serde_json::json!({
                "labels": labels,
                "count": labels.len(),
            }),
        ),
        Err(e) => {
            error!("Failed to list labels: {}", e);
            generic_error(id, e)
        }
    }
}

async fn handle_modify_message(
    params: &Value,
    id: Value,
    gmail: GmailApi,
) -> JsonRpcResponse {
    info!("Handling gmail.modify_message");

    let message_id = match require_string_or(params, "id", "messageId", &id) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let add_label_ids = optional_string_array_or(params, "addLabelIds", "add_label_ids");
    let remove_label_ids = optional_string_array_or(params, "removeLabelIds", "remove_label_ids");

    match gmail.modify_message(message_id, add_label_ids, remove_label_ids).await {
        Ok(message) => ok(id, message),
        Err(e) => {
            error!("Failed to modify message: {}", e);
            generic_error(id, e)
        }
    }
}

async fn handle_trash_message(
    params: &Value,
    id: Value,
    gmail: GmailApi,
) -> JsonRpcResponse {
    info!("Handling gmail.trash_message");

    let message_id = match require_string_or(params, "id", "messageId", &id) {
        Ok(id) => id,
        Err(response) => return response,
    };

    match gmail.trash_message(message_id).await {
        Ok(response) => ok(id, response),
        Err(e) => {
            error!("Failed to trash message: {}", e);
            generic_error(id, e)
        }
    }
}

async fn handle_delete_message(
    params: &Value,
    id: Value,
    gmail: GmailApi,
) -> JsonRpcResponse {
    info!("Handling gmail.delete_message");

    let message_id = match require_string_or(params, "id", "messageId", &id) {
        Ok(id) => id,
        Err(response) => return response,
    };

    match gmail.delete_message(message_id).await {
        Ok(_) => ok(id, serde_json::json!({ "deleted": true })),
        Err(e) => {
            error!("Failed to delete message: {}", e);
            generic_error(id, e)
        }
    }
}
