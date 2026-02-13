//! Shared utilities for Google API modules
//!
//! Reduces duplication across Calendar, Gmail, and future Google API clients.

use serde_json::Value;

/// Extract an array field from a JSON response, returning an empty vec if missing.
///
/// Google APIs return lists under varying field names ("items", "messages", "labels", "events").
/// This helper standardizes extraction.
pub fn extract_array(response: &Value, field: &str) -> Vec<Value> {
    response
        .get(field)
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}
