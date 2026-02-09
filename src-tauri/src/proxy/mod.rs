//! Socket Proxy for Tairseach
//!
//! Provides a Unix socket server that allows external applications (OpenClaw agents)
//! to execute privileged operations through Tairseach's granted permissions.
//!
//! Protocol: JSON-RPC 2.0 over Unix socket at `~/.tairseach/tairseach.sock`

pub mod handlers;
pub mod protocol;
pub mod server;

use std::sync::Arc;
use tokio::sync::RwLock;

pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
pub use server::ProxyServer;

/// Proxy server state shared across connections
pub struct ProxyState {
    /// Connection count for metrics
    pub connection_count: RwLock<u64>,
    /// Active connections
    pub active_connections: RwLock<u32>,
}

impl ProxyState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            connection_count: RwLock::new(0),
            active_connections: RwLock::new(0),
        })
    }
}

impl Default for ProxyState {
    fn default() -> Self {
        Self {
            connection_count: RwLock::new(0),
            active_connections: RwLock::new(0),
        }
    }
}

/// Error types for the proxy
#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Method not found: {0}")]
    MethodNotFound(String),
    
    #[error("Invalid params: {0}")]
    InvalidParams(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

impl ProxyError {
    /// Convert to JSON-RPC error code
    pub fn code(&self) -> i32 {
        match self {
            ProxyError::Io(_) => -32000,
            ProxyError::Json(_) => -32700,  // Parse error
            ProxyError::PermissionDenied(_) => -32001,
            ProxyError::MethodNotFound(_) => -32601,
            ProxyError::InvalidParams(_) => -32602,
            ProxyError::Internal(_) => -32603,
        }
    }
}
