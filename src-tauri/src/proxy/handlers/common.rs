//! Common Handler Utilities
//!
//! Shared parameter extraction, response construction, and auth helpers.

use serde_json::Value;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::error;

use super::super::protocol::JsonRpcResponse;
use crate::auth::AuthBroker;

// ────────────────────────────────────────────────────────────────────────────
// Parameter Extraction Helpers
// ────────────────────────────────────────────────────────────────────────────

/// Extract a required string parameter
pub fn require_string<'a>(
    params: &'a Value,
    key: &str,
    id: &Value,
) -> Result<&'a str, JsonRpcResponse> {
    params
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| JsonRpcResponse::invalid_params(id.clone(), format!("Missing '{}' parameter", key)))
}

/// Extract a required string parameter with alias fallback
pub fn require_string_or<'a>(
    params: &'a Value,
    primary: &str,
    fallback: &str,
    id: &Value,
) -> Result<&'a str, JsonRpcResponse> {
    params
        .get(primary)
        .or_else(|| params.get(fallback))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            JsonRpcResponse::invalid_params(
                id.clone(),
                format!("Missing required parameter: {} (or {})", primary, fallback),
            )
        })
}

/// Extract an optional string parameter
pub fn optional_string<'a>(params: &'a Value, key: &str) -> Option<&'a str> {
    params.get(key).and_then(|v| v.as_str())
}

/// Extract an optional string parameter with alias fallback
pub fn optional_string_or<'a>(params: &'a Value, primary: &str, fallback: &str) -> Option<&'a str> {
    params
        .get(primary)
        .or_else(|| params.get(fallback))
        .and_then(|v| v.as_str())
}

/// Extract a string parameter with a default value
pub fn string_with_default<'a>(params: &'a Value, key: &str, default: &'a str) -> &'a str {
    params.get(key).and_then(|v| v.as_str()).unwrap_or(default)
}

/// Extract an optional u64 parameter
pub fn optional_u64(params: &Value, key: &str) -> Option<u64> {
    params.get(key).and_then(|v| v.as_u64())
}

/// Extract an optional u64 parameter with alias fallback
pub fn optional_u64_or(params: &Value, primary: &str, fallback: &str) -> Option<u64> {
    params
        .get(primary)
        .or_else(|| params.get(fallback))
        .and_then(|v| v.as_u64())
}

/// Extract a u64 parameter with a default value
pub fn u64_with_default(params: &Value, key: &str, default: u64) -> u64 {
    params.get(key).and_then(|v| v.as_u64()).unwrap_or(default)
}

/// Extract a u64 parameter with alias fallback and default
pub fn u64_or_with_default(params: &Value, primary: &str, fallback: &str, default: u64) -> u64 {
    params
        .get(primary)
        .or_else(|| params.get(fallback))
        .and_then(|v| v.as_u64())
        .unwrap_or(default)
}

/// Extract a required f64 parameter
pub fn require_f64(params: &Value, key: &str, id: &Value) -> Result<f64, JsonRpcResponse> {
    params
        .get(key)
        .and_then(|v| v.as_f64())
        .ok_or_else(|| invalid_params(id.clone(), format!("Missing '{}' parameter", key)))
}

/// Extract an optional f64 parameter
pub fn optional_f64(params: &Value, key: &str) -> Option<f64> {
    params.get(key).and_then(|v| v.as_f64())
}

/// Extract an optional bool parameter
pub fn optional_bool(params: &Value, key: &str) -> Option<bool> {
    params.get(key).and_then(|v| v.as_bool())
}

/// Extract a bool parameter with a default value
pub fn bool_with_default(params: &Value, key: &str, default: bool) -> bool {
    params.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
}

/// Extract an optional array of strings
pub fn optional_string_array(params: &Value, key: &str) -> Option<Vec<String>> {
    params.get(key).and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    })
}

/// Extract an optional array of strings with alias fallback
pub fn optional_string_array_or(params: &Value, primary: &str, fallback: &str) -> Option<Vec<String>> {
    params
        .get(primary)
        .or_else(|| params.get(fallback))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
}

// ────────────────────────────────────────────────────────────────────────────
// Auth Broker Helpers
// ────────────────────────────────────────────────────────────────────────────

/// Global auth broker instance (singleton pattern for handlers)
static AUTH_BROKER: OnceCell<Arc<AuthBroker>> = OnceCell::const_new();

/// Get or initialize the shared auth broker instance
pub async fn get_auth_broker() -> Result<&'static Arc<AuthBroker>, JsonRpcResponse> {
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

/// Extract provider and account from params for OAuth-based handlers
pub fn extract_oauth_credentials(
    params: &Value,
    default_provider: &str,
) -> Result<(String, String), JsonRpcResponse> {
    let provider = params
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or(default_provider)
        .to_string();

    let account = params
        .get("account")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            JsonRpcResponse::invalid_params(
                Value::Null,
                format!("Missing required parameter: account ({} email address)", default_provider),
            )
        })?
        .to_string();

    Ok((provider, account))
}

/// Extract OAuth access token from auth broker response
pub fn extract_access_token(token_data: &Value, id: &Value) -> Result<String, JsonRpcResponse> {
    token_data
        .get("access_token")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| {
            JsonRpcResponse::error(
                id.clone(),
                -32000,
                "Invalid token response: missing access_token".to_string(),
                None,
            )
        })
}

// ────────────────────────────────────────────────────────────────────────────
// Response Helpers
// ────────────────────────────────────────────────────────────────────────────

/// Success response with data
#[inline]
pub fn ok(id: Value, data: Value) -> JsonRpcResponse {
    JsonRpcResponse::success(id, data)
}

/// Error response with custom code and message
#[inline]
pub fn error(id: Value, code: i32, message: impl Into<String>) -> JsonRpcResponse {
    JsonRpcResponse::error(id, code, message.into(), None)
}

/// Generic error (-32000) with message
#[inline]
pub fn generic_error(id: Value, message: impl Into<String>) -> JsonRpcResponse {
    JsonRpcResponse::error(id, -32000, message.into(), None)
}

/// Invalid params error
#[inline]
pub fn invalid_params(id: Value, message: impl Into<String>) -> JsonRpcResponse {
    JsonRpcResponse::invalid_params(id, message.into())
}

/// Method not found error
#[inline]
pub fn method_not_found(id: Value, method: &str) -> JsonRpcResponse {
    JsonRpcResponse::method_not_found(id, method)
}

/// Success response with a simple "success: true" payload
#[inline]
pub fn simple_success(id: Value) -> JsonRpcResponse {
    JsonRpcResponse::success(id, serde_json::json!({ "success": true }))
}

/// Success response with a count field
#[inline]
pub fn success_with_count(id: Value, data: Value, count: usize) -> JsonRpcResponse {
    JsonRpcResponse::success(
        id,
        serde_json::json!({
            "data": data,
            "count": count,
        }),
    )
}

// ────────────────────────────────────────────────────────────────────────────
// Testing Utilities
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_require_string() {
        let params = json!({"name": "test"});
        let id = json!(1);
        
        assert_eq!(require_string(&params, "name", &id).unwrap(), "test");
        assert!(require_string(&params, "missing", &id).is_err());
    }

    #[test]
    fn test_optional_string() {
        let params = json!({"name": "test"});
        
        assert_eq!(optional_string(&params, "name"), Some("test"));
        assert_eq!(optional_string(&params, "missing"), None);
    }

    #[test]
    fn test_string_with_default() {
        let params = json!({"name": "test"});
        
        assert_eq!(string_with_default(&params, "name", "default"), "test");
        assert_eq!(string_with_default(&params, "missing", "default"), "default");
    }

    #[test]
    fn test_u64_with_default() {
        let params = json!({"count": 42});
        
        assert_eq!(u64_with_default(&params, "count", 100), 42);
        assert_eq!(u64_with_default(&params, "missing", 100), 100);
    }

    #[test]
    fn test_bool_with_default() {
        let params = json!({"enabled": true});
        
        assert_eq!(bool_with_default(&params, "enabled", false), true);
        assert_eq!(bool_with_default(&params, "missing", false), false);
    }

    #[test]
    fn test_optional_string_array() {
        let params = json!({"tags": ["a", "b", "c"]});
        
        let tags = optional_string_array(&params, "tags").unwrap();
        assert_eq!(tags, vec!["a", "b", "c"]);
        
        assert!(optional_string_array(&params, "missing").is_none());
    }
}
