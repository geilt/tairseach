//! MCP Server Implementation
//!
//! Provides both TCP and STDIO modes for MCP communication.
//! - TCP mode: Listens on a port for direct connections
//! - STDIO mode: Reads from stdin, writes to stdout (for MCP integration)

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use super::handlers::{get_tool_definitions, handle_tool_call};
use super::protocol::{
    methods, InitializeParams, InitializeResult, McpError, McpRequest, McpResponse, RequestId,
    ServerCapabilities, ServerInfo, ToolCallParams, ToolsCapability, ToolsListResult,
    MCP_VERSION,
};

/// Server status information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpStatus {
    /// Whether the server is currently running
    pub running: bool,

    /// Port the server is listening on (if TCP mode)
    pub port: Option<u16>,

    /// Mode the server is running in
    pub mode: Option<String>,

    /// Number of connected clients
    pub connected_clients: usize,
}

impl Default for McpStatus {
    fn default() -> Self {
        Self {
            running: false,
            port: None,
            mode: None,
            connected_clients: 0,
        }
    }
}

/// Shared state for the MCP server
struct McpServerState {
    /// Port the server is configured to use
    port: u16,

    /// Running state
    running: AtomicBool,

    /// Connected client count
    client_count: AtomicUsize,

    /// Shutdown signal sender
    shutdown_tx: broadcast::Sender<()>,
}

/// MCP Server instance (cheaply cloneable)
#[derive(Clone)]
pub struct McpServer {
    state: Arc<McpServerState>,
}

impl McpServer {
    /// Default port for the MCP server
    pub const DEFAULT_PORT: u16 = 18799;

    /// Create a new MCP server instance
    pub fn new(port: u16) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            state: Arc::new(McpServerState {
                port,
                running: AtomicBool::new(false),
                client_count: AtomicUsize::new(0),
                shutdown_tx,
            }),
        }
    }

    /// Get the port this server is configured for
    pub fn port(&self) -> u16 {
        self.state.port
    }

    /// Check if the server is running
    pub fn is_running(&self) -> bool {
        self.state.running.load(Ordering::SeqCst)
    }

    /// Get current server status
    pub fn status(&self) -> McpStatus {
        let running = self.state.running.load(Ordering::SeqCst);
        McpStatus {
            running,
            port: if running { Some(self.state.port) } else { None },
            mode: if running { Some("tcp".to_string()) } else { None },
            connected_clients: self.state.client_count.load(Ordering::SeqCst),
        }
    }

    /// Start the TCP server
    pub async fn start(&self) -> Result<(), McpServerError> {
        if self.state.running.load(Ordering::SeqCst) {
            return Err(McpServerError::AlreadyRunning);
        }

        let addr = format!("127.0.0.1:{}", self.state.port);
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| McpServerError::BindFailed(e.to_string()))?;

        info!(address = %addr, "MCP server listening");
        self.state.running.store(true, Ordering::SeqCst);

        // Clone the state for the spawned task
        let state = self.state.clone();
        let mut shutdown_rx = self.state.shutdown_tx.subscribe();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    accept_result = listener.accept() => {
                        match accept_result {
                            Ok((stream, addr)) => {
                                info!(client = %addr, "New MCP client connected");
                                state.client_count.fetch_add(1, Ordering::SeqCst);
                                let state_clone = state.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = handle_tcp_connection(stream).await {
                                        error!(error = %e, "Error handling connection");
                                    }
                                    state_clone.client_count.fetch_sub(1, Ordering::SeqCst);
                                });
                            }
                            Err(e) => {
                                error!(error = %e, "Error accepting connection");
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("MCP server shutting down");
                        break;
                    }
                }
            }
            state.running.store(false, Ordering::SeqCst);
        });

        Ok(())
    }

    /// Stop the TCP server
    pub fn stop(&self) -> Result<(), McpServerError> {
        if !self.state.running.load(Ordering::SeqCst) {
            return Err(McpServerError::NotRunning);
        }

        let _ = self.state.shutdown_tx.send(());
        Ok(())
    }
}

/// Handle a single TCP connection
async fn handle_tcp_connection(stream: TcpStream) -> Result<(), McpServerError> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    let mut initialized = false;

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                debug!("Client disconnected");
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                debug!(request = %trimmed, "Received request");

                let response = process_request(trimmed, &mut initialized).await;
                let response_json = serde_json::to_string(&response)
                    .map_err(|e| McpServerError::SerializationFailed(e.to_string()))?;

                debug!(response = %response_json, "Sending response");

                writer
                    .write_all(response_json.as_bytes())
                    .await
                    .map_err(|e| McpServerError::WriteFailed(e.to_string()))?;
                writer
                    .write_all(b"\n")
                    .await
                    .map_err(|e| McpServerError::WriteFailed(e.to_string()))?;
                writer
                    .flush()
                    .await
                    .map_err(|e| McpServerError::WriteFailed(e.to_string()))?;
            }
            Err(e) => {
                error!(error = %e, "Error reading from client");
                break;
            }
        }
    }

    Ok(())
}

/// Run the server in STDIO mode (for MCP integration)
pub async fn run_stdio_mode() -> Result<(), McpServerError> {
    info!("Starting MCP server in STDIO mode");

    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();
    let mut initialized = false;

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                debug!("EOF received, shutting down");
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                debug!(request = %trimmed, "Received STDIO request");

                let response = process_request(trimmed, &mut initialized).await;
                let response_json = serde_json::to_string(&response)
                    .map_err(|e| McpServerError::SerializationFailed(e.to_string()))?;

                debug!(response = %response_json, "Sending STDIO response");

                stdout
                    .write_all(response_json.as_bytes())
                    .await
                    .map_err(|e| McpServerError::WriteFailed(e.to_string()))?;
                stdout
                    .write_all(b"\n")
                    .await
                    .map_err(|e| McpServerError::WriteFailed(e.to_string()))?;
                stdout
                    .flush()
                    .await
                    .map_err(|e| McpServerError::WriteFailed(e.to_string()))?;
            }
            Err(e) => {
                error!(error = %e, "Error reading from stdin");
                break;
            }
        }
    }

    Ok(())
}

/// Process a single JSON-RPC request
async fn process_request(input: &str, initialized: &mut bool) -> McpResponse {
    // Parse the request
    let request: McpRequest = match serde_json::from_str(input) {
        Ok(req) => req,
        Err(e) => {
            return McpResponse::error(
                None,
                McpError::parse_error(format!("Invalid JSON: {}", e)),
            );
        }
    };

    // Validate JSON-RPC version
    if request.jsonrpc != "2.0" {
        return McpResponse::error(
            request.id,
            McpError::invalid_request("Invalid JSON-RPC version, expected 2.0"),
        );
    }

    // Handle notifications (no response needed for some)
    if request.id.is_none() && request.method == methods::INITIALIZED {
        debug!("Received initialized notification");
        *initialized = true;
        // Notifications don't get responses, but we need to return something
        // In practice, this shouldn't be sent back
        return McpResponse::success(None, json!(null));
    }

    // Dispatch based on method
    match request.method.as_str() {
        methods::INITIALIZE => handle_initialize(request.id, request.params).await,
        methods::TOOLS_LIST => handle_tools_list(request.id, *initialized).await,
        methods::TOOLS_CALL => handle_tools_call(request.id, request.params, *initialized).await,
        methods::PING => McpResponse::success(request.id, json!({})),
        methods::SHUTDOWN => {
            info!("Shutdown requested");
            McpResponse::success(request.id, json!({}))
        }
        _ => {
            warn!(method = %request.method, "Unknown method");
            McpResponse::error(request.id, McpError::method_not_found(&request.method))
        }
    }
}

/// Handle the initialize request
async fn handle_initialize(id: Option<RequestId>, params: Option<Value>) -> McpResponse {
    // Parse initialize params (optional validation)
    if let Some(params) = params {
        match serde_json::from_value::<InitializeParams>(params) {
            Ok(init_params) => {
                info!(
                    client_name = %init_params.client_info.name,
                    client_version = %init_params.client_info.version,
                    protocol_version = %init_params.protocol_version,
                    "Client initializing"
                );
            }
            Err(e) => {
                warn!(error = %e, "Failed to parse initialize params, continuing anyway");
            }
        }
    }

    let result = InitializeResult {
        protocol_version: MCP_VERSION.to_string(),
        capabilities: ServerCapabilities {
            tools: Some(ToolsCapability {
                list_changed: Some(false),
            }),
            resources: None,
            prompts: None,
            logging: None,
        },
        server_info: ServerInfo {
            name: "tairseach".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    };

    McpResponse::success(id, serde_json::to_value(result).unwrap())
}

/// Handle the tools/list request
async fn handle_tools_list(id: Option<RequestId>, initialized: bool) -> McpResponse {
    if !initialized {
        // Some implementations are lenient about this, but we'll allow it with a warning
        warn!("tools/list called before initialized notification");
    }

    let result = ToolsListResult {
        tools: get_tool_definitions(),
    };

    McpResponse::success(id, serde_json::to_value(result).unwrap())
}

/// Handle the tools/call request
async fn handle_tools_call(
    id: Option<RequestId>,
    params: Option<Value>,
    initialized: bool,
) -> McpResponse {
    if !initialized {
        warn!("tools/call called before initialized notification");
    }

    let params = match params {
        Some(p) => p,
        None => {
            return McpResponse::error(id, McpError::invalid_params("Missing params"));
        }
    };

    let call_params: ToolCallParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(e) => {
            return McpResponse::error(
                id,
                McpError::invalid_params(format!("Invalid params: {}", e)),
            );
        }
    };

    debug!(tool = %call_params.name, "Calling tool");

    match handle_tool_call(&call_params.name, call_params.arguments).await {
        Ok(result) => McpResponse::success(id, serde_json::to_value(result).unwrap()),
        Err(e) => McpResponse::error(id, e),
    }
}

/// MCP Server errors
#[derive(Debug, thiserror::Error)]
pub enum McpServerError {
    #[error("Server is already running")]
    AlreadyRunning,

    #[error("Server is not running")]
    NotRunning,

    #[error("Failed to bind to address: {0}")]
    BindFailed(String),

    #[error("Failed to serialize response: {0}")]
    SerializationFailed(String),

    #[error("Failed to write response: {0}")]
    WriteFailed(String),

    #[error("Failed to read request: {0}")]
    ReadFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_initialize() {
        let request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#;
        let mut initialized = false;
        let response = process_request(request, &mut initialized).await;
        
        assert!(response.error.is_none());
        assert!(response.result.is_some());
        
        let result = response.result.unwrap();
        assert_eq!(result["protocolVersion"], MCP_VERSION);
        assert!(result["capabilities"]["tools"].is_object());
    }

    #[tokio::test]
    async fn test_process_tools_list() {
        let request = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;
        let mut initialized = true;
        let response = process_request(request, &mut initialized).await;
        
        assert!(response.error.is_none());
        assert!(response.result.is_some());
        
        let result = response.result.unwrap();
        assert!(result["tools"].is_array());
        assert!(!result["tools"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_process_tools_call() {
        let request = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"tairseach.permissions.check","arguments":{"permission":"contacts"}}}"#;
        let mut initialized = true;
        let response = process_request(request, &mut initialized).await;
        
        assert!(response.error.is_none());
        assert!(response.result.is_some());
    }

    #[tokio::test]
    async fn test_process_unknown_method() {
        let request = r#"{"jsonrpc":"2.0","id":4,"method":"unknown/method"}"#;
        let mut initialized = true;
        let response = process_request(request, &mut initialized).await;
        
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, McpError::METHOD_NOT_FOUND);
    }

    #[tokio::test]
    async fn test_process_invalid_json() {
        let request = r#"not valid json"#;
        let mut initialized = false;
        let response = process_request(request, &mut initialized).await;
        
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, McpError::PARSE_ERROR);
    }

    #[test]
    fn test_server_status_default() {
        let server = McpServer::new(18799);
        let status = server.status();
        
        assert!(!status.running);
        assert!(status.port.is_none());
        assert_eq!(status.connected_clients, 0);
    }

    #[test]
    fn test_server_is_cloneable() {
        let server = McpServer::new(18799);
        let server2 = server.clone();
        
        assert_eq!(server.port(), server2.port());
    }
}
