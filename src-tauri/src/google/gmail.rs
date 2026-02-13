//! Gmail API v1 Client
//!
//! Provides methods for interacting with Gmail API:
//! - List/search messages
//! - Get message details
//! - Send emails
//! - Manage labels
//!
//! All methods use the authenticated GoogleClient with Tier 1 proxy mode.

use super::client::GoogleClient;
use serde_json::{json, Value};
use tracing::{debug, info};

const GMAIL_API_BASE: &str = "https://gmail.googleapis.com/gmail/v1";

pub struct GmailApi {
    client: GoogleClient,
}

super::google_api_wrapper!(GmailApi);

impl GmailApi {
    /// List messages matching a query
    ///
    /// # Arguments
    /// * `query` - Gmail search query (same syntax as web UI)
    /// * `max_results` - Maximum number of messages to return (default: 100)
    /// * `label_ids` - Filter by label IDs (e.g., ["INBOX", "UNREAD"])
    ///
    /// # Returns
    /// Array of message objects with id and threadId
    pub async fn list_messages(
        &self,
        query: Option<&str>,
        max_results: Option<usize>,
        label_ids: Option<Vec<String>>,
    ) -> Result<Vec<Value>, String> {
        info!("Listing Gmail messages");

        let mut query_params = vec![];
        
        if let Some(q) = query {
            query_params.push(("q", q.to_string()));
        }
        
        if let Some(labels) = label_ids {
            for label in labels {
                query_params.push(("labelIds", label));
            }
        }

        let url = format!("{}/users/me/messages", GMAIL_API_BASE);
        let messages = self.client.get_paginated(&url, &query_params, max_results).await?;

        debug!("Retrieved {} messages", messages.len());
        Ok(messages)
    }

    /// Get a message by ID
    ///
    /// # Arguments
    /// * `id` - Message ID
    /// * `format` - Format to return: "full" (default), "metadata", "minimal", "raw"
    ///
    /// # Returns
    /// Full message object including headers, body, attachments
    pub async fn get_message(
        &self,
        id: &str,
        format: Option<&str>,
    ) -> Result<Value, String> {
        info!("Fetching Gmail message: {}", id);

        let mut query_params = vec![];
        if let Some(fmt) = format {
            query_params.push(("format", fmt.to_string()));
        }

        let url = format!("{}/users/me/messages/{}", GMAIL_API_BASE, id);
        let message = self.client.get(&url, &query_params).await?;

        Ok(message)
    }

    /// Send an email
    ///
    /// # Arguments
    /// * `to` - Recipient email addresses
    /// * `subject` - Email subject
    /// * `body` - Email body (plain text)
    /// * `cc` - CC recipients (optional)
    /// * `bcc` - BCC recipients (optional)
    ///
    /// # Returns
    /// Sent message object with id and threadId
    pub async fn send_message(
        &self,
        to: Vec<String>,
        subject: &str,
        body: &str,
        cc: Option<Vec<String>>,
        bcc: Option<Vec<String>>,
    ) -> Result<Value, String> {
        info!("Sending Gmail message to: {:?}", to);

        // Build RFC 2822 message
        let mut message_parts = vec![
            format!("To: {}", to.join(", ")),
            format!("Subject: {}", subject),
        ];

        if let Some(cc_addrs) = cc {
            if !cc_addrs.is_empty() {
                message_parts.push(format!("Cc: {}", cc_addrs.join(", ")));
            }
        }

        if let Some(bcc_addrs) = bcc {
            if !bcc_addrs.is_empty() {
                message_parts.push(format!("Bcc: {}", bcc_addrs.join(", ")));
            }
        }

        message_parts.push("Content-Type: text/plain; charset=UTF-8".to_string());
        message_parts.push(String::new()); // Empty line separates headers from body
        message_parts.push(body.to_string());

        let raw_message = message_parts.join("\r\n");

        // Base64url encode (no padding)
        let encoded = base64_url_encode(raw_message.as_bytes());

        let request_body = json!({
            "raw": encoded
        });

        let url = format!("{}/users/me/messages/send", GMAIL_API_BASE);
        let response = self.client.post(&url, &request_body).await?;

        info!("Message sent successfully");
        Ok(response)
    }

    /// List all labels
    ///
    /// # Returns
    /// Array of label objects with id, name, type
    pub async fn list_labels(&self) -> Result<Vec<Value>, String> {
        info!("Listing Gmail labels");

        let url = format!("{}/users/me/labels", GMAIL_API_BASE);
        let response = self.client.get(&url, &[]).await?;

        let labels = response
            .get("labels")
            .and_then(|v| v.as_array())
            .map(|arr| arr.clone())
            .unwrap_or_default();

        debug!("Retrieved {} labels", labels.len());
        Ok(labels)
    }

    /// Get details of a specific label
    #[allow(dead_code)]
    pub async fn get_label(&self, label_id: &str) -> Result<Value, String> {
        info!("Fetching label: {}", label_id);

        let url = format!("{}/users/me/labels/{}", GMAIL_API_BASE, label_id);
        self.client.get(&url, &[]).await
    }

    /// Modify message labels (add/remove labels from a message)
    pub async fn modify_message(
        &self,
        message_id: &str,
        add_label_ids: Option<Vec<String>>,
        remove_label_ids: Option<Vec<String>>,
    ) -> Result<Value, String> {
        info!("Modifying labels for message: {}", message_id);

        let mut body = json!({});
        
        if let Some(add) = add_label_ids {
            body["addLabelIds"] = json!(add);
        }
        
        if let Some(remove) = remove_label_ids {
            body["removeLabelIds"] = json!(remove);
        }

        let url = format!("{}/users/me/messages/{}/modify", GMAIL_API_BASE, message_id);
        self.client.post(&url, &body).await
    }

    /// Trash a message
    pub async fn trash_message(&self, message_id: &str) -> Result<Value, String> {
        info!("Trashing message: {}", message_id);

        let url = format!("{}/users/me/messages/{}/trash", GMAIL_API_BASE, message_id);
        self.client.post(&url, &json!({})).await
    }

    /// Permanently delete a message
    pub async fn delete_message(&self, message_id: &str) -> Result<(), String> {
        info!("Deleting message: {}", message_id);

        let url = format!("{}/users/me/messages/{}", GMAIL_API_BASE, message_id);
        self.client.delete(&url).await?;
        Ok(())
    }
}

/// Base64url encode (no padding) per RFC 4648 §5
fn base64_url_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    URL_SAFE_NO_PAD.encode(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_url_encode() {
        let input = b"Hello, World!";
        let encoded = base64_url_encode(input);
        assert!(!encoded.contains('='));  // No padding
        assert!(!encoded.contains('+'));  // URL-safe
        assert!(!encoded.contains('/'));  // URL-safe
    }
}
