//! JSON-RPC 2.0 Protocol Implementation
//!
//! Handles parsing and serialization of JSON-RPC 2.0 messages over the socket.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcRequest {
    /// Protocol version (must be "2.0")
    pub jsonrpc: String,
    
    /// Request ID for correlating responses
    #[serde(default)]
    pub id: Option<Value>,
    
    /// Method name (e.g., "contacts.list", "permissions.check")
    pub method: String,
    
    /// Method parameters
    #[serde(default)]
    pub params: Value,
}

impl JsonRpcRequest {
    /// Check if this is a notification (no id = no response expected)
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }
    
    /// Validate the request structure
    pub fn validate(&self) -> Result<(), String> {
        if self.jsonrpc != "2.0" {
            return Err("Invalid JSON-RPC version, expected '2.0'".to_string());
        }
        if self.method.is_empty() {
            return Err("Method cannot be empty".to_string());
        }
        Ok(())
    }
    
    /// Parse method into namespace and action
    /// e.g., "contacts.list" -> ("contacts", "list")
    pub fn parse_method(&self) -> (&str, &str) {
        if let Some((namespace, action)) = self.method.split_once('.') {
            (namespace, action)
        } else {
            (self.method.as_str(), "")
        }
    }
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcResponse {
    /// Protocol version
    pub jsonrpc: String,
    
    /// Request ID (copied from request)
    pub id: Value,
    
    /// Result (on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    
    /// Error (on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[allow(dead_code)]
impl JsonRpcResponse {
    /// Create a success response
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }
    
    /// Create an error response
    pub fn error(id: Value, code: i32, message: impl Into<String>, data: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data,
            }),
        }
    }
    
    /// Create a parse error response (for malformed JSON)
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::error(Value::Null, -32700, message, None)
    }
    
    /// Create an invalid request error
    pub fn invalid_request(id: Value, message: impl Into<String>) -> Self {
        Self::error(id, -32600, message, None)
    }
    
    /// Create a method not found error
    pub fn method_not_found(id: Value, method: &str) -> Self {
        Self::error(
            id,
            -32601,
            format!("Method not found: {}", method),
            None,
        )
    }
    
    /// Create an invalid params error
    pub fn invalid_params(id: Value, message: impl Into<String>) -> Self {
        Self::error(id, -32602, message, None)
    }
    
    /// Create an internal error response
    pub fn internal_error(id: Value, message: impl Into<String>) -> Self {
        Self::error(id, -32603, message, None)
    }
    
    /// Create a permission denied error with remediation guidance
    pub fn permission_denied(id: Value, permission: &str, status: &str) -> Self {
        let remediation = match status {
            "not_determined" => {
                format!(
                    "Permission can be requested. Call permissions.request with permission='{}'",
                    permission
                )
            }
            "denied" => {
                let pane = match permission {
                    "contacts" => "Contacts",
                    "calendar" => "Calendars",
                    "reminders" => "Reminders",
                    "location" => "Location Services",
                    "photos" => "Photos",
                    "camera" => "Camera",
                    "microphone" => "Microphone",
                    "screen_recording" => "Screen Recording",
                    "accessibility" => "Accessibility",
                    "full_disk_access" => "Full Disk Access",
                    "automation" => "Automation",
                    _ => "Privacy & Security",
                };
                format!(
                    "User must grant permission manually in System Settings > Privacy & Security > {}",
                    pane
                )
            }
            "restricted" => {
                "Permission is restricted by system policy and cannot be granted".to_string()
            }
            _ => "Permission status unknown. Check System Settings > Privacy & Security".to_string(),
        };

        Self::error(
            id,
            -32001,
            "Permission not granted",
            Some(serde_json::json!({
                "permission": permission,
                "status": status,
                "remediation": remediation
            })),
        )
    }
}

/// JSON-RPC 2.0 Error Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code
    pub code: i32,
    
    /// Error message
    pub message: String,
    
    /// Additional error data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[allow(dead_code)]
impl JsonRpcError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }
    
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }
}

/// Batch request support - parse either single request or array of requests
pub fn parse_request(input: &str) -> Result<Vec<JsonRpcRequest>, JsonRpcResponse> {
    let trimmed = input.trim();
    
    if trimmed.is_empty() {
        return Err(JsonRpcResponse::parse_error("Empty request"));
    }
    
    // Try parsing as array first
    if trimmed.starts_with('[') {
        match serde_json::from_str::<Vec<JsonRpcRequest>>(trimmed) {
            Ok(requests) => {
                if requests.is_empty() {
                    Err(JsonRpcResponse::invalid_request(
                        Value::Null,
                        "Empty batch request",
                    ))
                } else {
                    Ok(requests)
                }
            }
            Err(e) => Err(JsonRpcResponse::parse_error(e.to_string())),
        }
    } else {
        // Parse as single request
        match serde_json::from_str::<JsonRpcRequest>(trimmed) {
            Ok(request) => Ok(vec![request]),
            Err(e) => Err(JsonRpcResponse::parse_error(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_single_request() {
        let input = r#"{"jsonrpc":"2.0","id":1,"method":"contacts.list","params":{}}"#;
        let requests = parse_request(input).unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "contacts.list");
    }
    
    #[test]
    fn test_parse_method() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::Number(1.into())),
            method: "contacts.list".to_string(),
            params: Value::Null,
        };
        assert_eq!(req.parse_method(), ("contacts", "list"));
    }
    
    #[test]
    fn test_success_response() {
        let resp = JsonRpcResponse::success(
            Value::Number(1.into()),
            serde_json::json!({"count": 42}),
        );
        assert!(resp.error.is_none());
        assert!(resp.result.is_some());
    }
}
