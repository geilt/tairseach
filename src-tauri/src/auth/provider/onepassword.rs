//! 1Password Provider
//!
//! Manages 1Password Connect service account tokens.
//! Unlike OAuth providers, 1Password uses a fixed service account token.

use serde::{Deserialize, Serialize};
use tracing::info;

/// 1Password token record (simpler than OAuth)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnePasswordToken {
    pub service_account_token: String,
    pub connect_host: String,
}

/// 1Password provider (token-based, not OAuth)
pub struct OnePasswordProvider;

impl OnePasswordProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn name(&self) -> &str {
        "onepassword"
    }

    /// Validate a service account token by attempting a status check
    pub async fn validate_token(&self, token: &str, connect_host: &str) -> Result<(), String> {
        info!("Validating 1Password service account token");

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let url = format!("{}/v1/heartbeat", connect_host);
        
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

    /// Get default Connect host (localhost or cloud)
    pub fn default_connect_host() -> String {
        // Default to localhost for self-hosted Connect
        "http://localhost:8080".to_string()
    }
}

impl Default for OnePasswordProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_name() {
        let provider = OnePasswordProvider::new();
        assert_eq!(provider.name(), "onepassword");
    }

    #[test]
    fn test_default_host() {
        let host = OnePasswordProvider::default_connect_host();
        assert!(!host.is_empty());
    }
}
