//! MCP Protocol Types
//!
//! JSON-RPC 2.0 message types for the Model Context Protocol.
//! Reference: https://modelcontextprotocol.io/spec

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC version constant
pub const JSONRPC_VERSION: &str = "2.0";

/// MCP protocol version
pub const MCP_VERSION: &str = "2024-11-05";

// =============================================================================
// Core JSON-RPC Types
// =============================================================================

/// Incoming JSON-RPC request from client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    /// JSON-RPC version, always "2.0"
    pub jsonrpc: String,

    /// Request ID (None for notifications)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<RequestId>,

    /// Method name to invoke
    pub method: String,

    /// Method parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// Outgoing JSON-RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    /// JSON-RPC version, always "2.0"
    pub jsonrpc: String,

    /// Request ID this response corresponds to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<RequestId>,

    /// Success result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,

    /// Error result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

/// Request ID can be either a number or string
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum RequestId {
    Number(u64),
    String(String),
}

impl From<u64> for RequestId {
    fn from(n: u64) -> Self {
        RequestId::Number(n)
    }
}

impl From<String> for RequestId {
    fn from(s: String) -> Self {
        RequestId::String(s)
    }
}

impl From<&str> for RequestId {
    fn from(s: &str) -> Self {
        RequestId::String(s.to_string())
    }
}

/// JSON-RPC error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    /// Error code (negative for JSON-RPC errors, positive for app errors)
    pub code: i32,

    /// Human-readable error message
    pub message: String,

    /// Optional additional error data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

// Standard JSON-RPC error codes
impl McpError {
    // JSON-RPC 2.0 error codes
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;

    /// Parse error - invalid JSON
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self {
            code: Self::PARSE_ERROR,
            message: message.into(),
            data: None,
        }
    }

    /// Invalid Request - not a valid JSON-RPC request
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: Self::INVALID_REQUEST,
            message: message.into(),
            data: None,
        }
    }

    /// Method not found
    pub fn method_not_found(method: impl Into<String>) -> Self {
        Self {
            code: Self::METHOD_NOT_FOUND,
            message: format!("Method not found: {}", method.into()),
            data: None,
        }
    }

    /// Invalid params
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: Self::INVALID_PARAMS,
            message: message.into(),
            data: None,
        }
    }

    /// Internal error
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self {
            code: Self::INTERNAL_ERROR,
            message: message.into(),
            data: None,
        }
    }

    /// Permission denied (application-specific)
    pub fn permission_denied(permission: impl Into<String>) -> Self {
        Self {
            code: 1001,
            message: format!("Permission denied: {}", permission.into()),
            data: None,
        }
    }
}

impl McpResponse {
    /// Create a success response
    pub fn success(id: Option<RequestId>, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response
    pub fn error(id: Option<RequestId>, error: McpError) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

// =============================================================================
// MCP Protocol Types
// =============================================================================

/// Server capabilities declared during initialization
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerCapabilities {
    /// Tools capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,

    /// Resources capability (not implemented)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Value>,

    /// Prompts capability (not implemented)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<Value>,

    /// Logging capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolsCapability {
    /// Whether the tool list can change
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Server information returned during initialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    /// Server name
    pub name: String,

    /// Server version
    pub version: String,
}

/// Client information provided during initialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    /// Client name
    pub name: String,

    /// Client version
    pub version: String,
}

/// Initialize request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    /// Protocol version client supports
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,

    /// Client capabilities (currently unused)
    pub capabilities: Value,

    /// Client information
    #[serde(rename = "clientInfo")]
    pub client_info: ClientInfo,
}

/// Initialize response result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    /// Protocol version server supports
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,

    /// Server capabilities
    pub capabilities: ServerCapabilities,

    /// Server information
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
}

/// Tool definition for tools/list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name (e.g., "tairseach.permissions.check")
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// JSON Schema for the tool's input
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Tools list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsListResult {
    /// Available tools
    pub tools: Vec<ToolDefinition>,
}

/// Tool call request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallParams {
    /// Tool name to call
    pub name: String,

    /// Tool arguments
    #[serde(default)]
    pub arguments: Value,
}

/// Tool call response content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallContent {
    /// Content type
    #[serde(rename = "type")]
    pub content_type: String,

    /// Text content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Tool call response result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    /// Response content
    pub content: Vec<ToolCallContent>,

    /// Whether the call resulted in an error
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl ToolCallResult {
    /// Create a successful text result
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: vec![ToolCallContent {
                content_type: "text".to_string(),
                text: Some(content.into()),
            }],
            is_error: None,
        }
    }

    /// Create a JSON result
    pub fn json(value: &Value) -> Self {
        Self::text(serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string()))
    }

    /// Create an error result
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![ToolCallContent {
                content_type: "text".to_string(),
                text: Some(message.into()),
            }],
            is_error: Some(true),
        }
    }
}

// =============================================================================
// MCP Method Constants
// =============================================================================

pub mod methods {
    /// Initialize the connection
    pub const INITIALIZE: &str = "initialize";

    /// Notification that initialization is complete
    pub const INITIALIZED: &str = "notifications/initialized";

    /// List available tools
    pub const TOOLS_LIST: &str = "tools/list";

    /// Call a tool
    pub const TOOLS_CALL: &str = "tools/call";

    /// Ping (keepalive)
    pub const PING: &str = "ping";

    /// Shutdown
    pub const SHUTDOWN: &str = "shutdown";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_parsing() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let request: McpRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, Some(RequestId::Number(1)));
        assert_eq!(request.method, "initialize");
    }

    #[test]
    fn test_response_serialization() {
        let response =
            McpResponse::success(Some(RequestId::Number(1)), serde_json::json!({"status": "ok"}));
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"result\""));
    }

    #[test]
    fn test_error_codes() {
        let err = McpError::method_not_found("foo.bar");
        assert_eq!(err.code, McpError::METHOD_NOT_FOUND);
        assert!(err.message.contains("foo.bar"));
    }
}
