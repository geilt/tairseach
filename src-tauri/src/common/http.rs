//! HTTP Client Utilities
//!
//! Shared HTTP client creation with consistent configuration.

use std::time::Duration;

/// Create a reqwest HTTP client with standard configuration
///
/// - 30 second timeout
/// - Reusable across requests
pub fn create_http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))
}

/// Create a reqwest HTTP client with custom timeout
pub fn create_http_client_with_timeout(timeout_secs: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))
}
