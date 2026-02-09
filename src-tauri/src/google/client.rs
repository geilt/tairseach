//! Google API Authenticated HTTP Client
//!
//! Provides authenticated HTTP client that injects OAuth tokens from auth store.
//! Handles pagination, rate limiting, and error responses according to Google API
//! REST conventions.

use reqwest::{Client, RequestBuilder, StatusCode};
use serde_json::Value;
use std::time::Duration;
use tracing::{debug, error, warn};

/// Google API HTTP client with OAuth token injection
pub struct GoogleClient {
    client: Client,
    access_token: String,
}

impl GoogleClient {
    /// Create a new Google API client with an OAuth access token
    pub fn new(access_token: String) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        Ok(Self {
            client,
            access_token,
        })
    }

    /// Make an authenticated GET request
    pub async fn get(&self, url: &str, query: &[(&str, String)]) -> Result<Value, String> {
        let builder = self
            .client
            .get(url)
            .query(query)
            .bearer_auth(&self.access_token);

        self.execute_request(builder).await
    }

    /// Make an authenticated POST request with JSON body
    pub async fn post(&self, url: &str, body: &Value) -> Result<Value, String> {
        let builder = self
            .client
            .post(url)
            .bearer_auth(&self.access_token)
            .json(body);

        self.execute_request(builder).await
    }

    /// Make an authenticated PUT request with JSON body
    pub async fn put(&self, url: &str, body: &Value) -> Result<Value, String> {
        let builder = self
            .client
            .put(url)
            .bearer_auth(&self.access_token)
            .json(body);

        self.execute_request(builder).await
    }

    /// Make an authenticated PATCH request with JSON body
    #[allow(dead_code)]
    pub async fn patch(&self, url: &str, body: &Value) -> Result<Value, String> {
        let builder = self
            .client
            .patch(url)
            .bearer_auth(&self.access_token)
            .json(body);

        self.execute_request(builder).await
    }

    /// Make an authenticated DELETE request
    pub async fn delete(&self, url: &str) -> Result<Value, String> {
        let builder = self
            .client
            .delete(url)
            .bearer_auth(&self.access_token);

        self.execute_request(builder).await
    }

    /// Execute a request and handle Google API response patterns
    async fn execute_request(&self, builder: RequestBuilder) -> Result<Value, String> {
        debug!("Executing Google API request");

        let response = builder
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        let status = response.status();
        debug!("Response status: {}", status);

        // Handle rate limiting
        if status == StatusCode::TOO_MANY_REQUESTS {
            warn!("Rate limited by Google API");
            return Err("Rate limited. Please try again later.".to_string());
        }

        // Read response body
        let body = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        // Empty successful responses (e.g., DELETE)
        if status.is_success() && body.is_empty() {
            return Ok(Value::Object(serde_json::Map::new()));
        }

        // Parse JSON body
        let parsed: Value = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse JSON response: {} (body: {})", e, body))?;

        // Handle error responses
        if !status.is_success() {
            let error_msg = self.extract_error_message(&parsed, status);
            error!("Google API error: {}", error_msg);
            return Err(error_msg);
        }

        Ok(parsed)
    }

    /// Extract error message from Google API error response
    fn extract_error_message(&self, response: &Value, status: StatusCode) -> String {
        // Google APIs return errors in this format:
        // {
        //   "error": {
        //     "code": 400,
        //     "message": "Invalid request",
        //     "errors": [...]
        //   }
        // }

        if let Some(error_obj) = response.get("error") {
            if let Some(message) = error_obj.get("message").and_then(|v| v.as_str()) {
                let code = error_obj
                    .get("code")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(status.as_u16() as i64);

                return format!("Google API error {}: {}", code, message);
            }
        }

        // Fallback to status code
        format!("HTTP {} error", status)
    }

    /// Handle paginated requests with nextPageToken
    pub async fn get_paginated(
        &self,
        url: &str,
        base_query: &[(&str, String)],
        max_results: Option<usize>,
    ) -> Result<Vec<Value>, String> {
        let mut all_items = Vec::new();
        let mut page_token: Option<String> = None;
        let remaining = max_results.unwrap_or(usize::MAX);

        loop {
            let mut query = base_query.to_vec();
            if let Some(ref token) = page_token {
                query.push(("pageToken", token.clone()));
            }
            if let Some(max) = max_results {
                query.push(("maxResults", max.to_string()));
            }

            let response = self.get(url, &query).await?;

            // Extract items (array can be under different field names: items, messages, events, etc.)
            if let Some(items) = response
                .get("items")
                .or_else(|| response.get("messages"))
                .or_else(|| response.get("events"))
                .and_then(|v| v.as_array())
            {
                all_items.extend(items.clone());
                
                if all_items.len() >= remaining {
                    all_items.truncate(remaining);
                    break;
                }
            }

            // Check for next page token
            if let Some(next_token) = response.get("nextPageToken").and_then(|v| v.as_str()) {
                page_token = Some(next_token.to_string());
            } else {
                // No more pages
                break;
            }
        }

        Ok(all_items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_error_message() {
        let client = GoogleClient {
            client: Client::new(),
            access_token: "test".to_string(),
        };

        let error_response = serde_json::json!({
            "error": {
                "code": 400,
                "message": "Invalid request format"
            }
        });

        let msg = client.extract_error_message(&error_response, StatusCode::BAD_REQUEST);
        assert!(msg.contains("400"));
        assert!(msg.contains("Invalid request format"));
    }
}
