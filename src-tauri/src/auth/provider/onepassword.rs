//! 1Password Provider
//!
//! Manages 1Password Service Account tokens.
//! Unlike OAuth providers, 1Password uses a fixed service account token.

use serde::{Deserialize, Serialize};
use tracing::info;

/// 1Password token record (simpler than OAuth)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnePasswordToken {
    pub service_account_token: String,
}

/// Provider name constant
pub const PROVIDER_NAME: &str = "onepassword";

/// Validate a service account token by attempting a status check
pub async fn validate_token(token: &str) -> Result<(), String> {
    info!("Validating 1Password service account token");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    const API_BASE_URL: &str = "https://api.1password.com";
    let url = format!("{}/v1/heartbeat", API_BASE_URL);
    
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error response".to_string());
        return Err(format!("Validation failed (HTTP {}): {}", status, body));
    }

    Ok(())
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
