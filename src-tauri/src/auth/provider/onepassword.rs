//! 1Password Provider
//!
//! Manages 1Password Service Account tokens.
//! Validates via REST API (no FFI dependency).

use serde::{Deserialize, Serialize};
use tracing::info;

/// 1Password token record (simpler than OAuth)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnePasswordToken {
    pub service_account_token: String,
}

/// Provider name constant
pub const PROVIDER_NAME: &str = "onepassword";

/// Validate a service account token by calling the vaults endpoint
pub async fn validate_token(token: &str) -> Result<(), String> {
    info!("Validating 1Password service account token via REST API");

    let base = extract_api_base(token);
    let url = format!("{}/v1/vaults", base);

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        Err(format!("Token validation failed ({}): {}", status, &body[..body.len().min(200)]))
    }
}

/// Extract the API base URL from a 1Password Service Account JWT token
fn extract_api_base(token: &str) -> String {
    let jwt_part = if token.starts_with("ops_") {
        &token[4..]
    } else {
        token
    };

    if let Some(payload_b64) = jwt_part.split('.').nth(1) {
        use base64::Engine;
        let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        if let Ok(payload_bytes) = engine.decode(payload_b64) {
            if let Ok(payload) = serde_json::from_slice::<serde_json::Value>(&payload_bytes) {
                if let Some(aud) = payload.get("aud").and_then(|v| v.as_array()) {
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

    "https://events.1password.com".to_string()
}

/// Get Service Account API base URL
pub fn api_base_url() -> &'static str {
    "https://api.1password.com"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_name() {
        assert_eq!(PROVIDER_NAME, "onepassword");
    }

    #[test]
    fn test_api_base_url() {
        let url = api_base_url();
        assert_eq!(url, "https://api.1password.com");
    }
}
