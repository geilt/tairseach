//! Unix Socket Server
//!
//! Listens for incoming connections on `~/.tairseach/tairseach.sock` and
//! dispatches JSON-RPC 2.0 requests to the appropriate handlers.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use super::handlers::HandlerRegistry;
use super::protocol::{parse_request, JsonRpcResponse};
use super::ProxyState;

/// Default socket path
pub fn default_socket_path() -> PathBuf {
    let home = dirs::home_dir().expect("Could not determine home directory");
    home.join(".tairseach").join("tairseach.sock")
}

/// Proxy server that listens on a Unix socket
pub struct ProxyServer {
    /// Path to the Unix socket
    socket_path: PathBuf,
    
    /// Handler registry for dispatching requests
    handlers: Arc<HandlerRegistry>,
    
    /// Shared state
    state: Arc<ProxyState>,
    
    /// Shutdown signal sender
    shutdown_tx: broadcast::Sender<()>,
}

#[allow(dead_code)]
impl ProxyServer {
    /// Create a new proxy server
    pub fn new(socket_path: Option<PathBuf>) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        
        Self {
            socket_path: socket_path.unwrap_or_else(default_socket_path),
            handlers: Arc::new(HandlerRegistry::new()),
            state: ProxyState::new(),
            shutdown_tx,
        }
    }

    /// Create a new proxy server with a custom handler registry
    pub fn with_handlers(socket_path: Option<PathBuf>, handlers: Arc<HandlerRegistry>) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        
        Self {
            socket_path: socket_path.unwrap_or_else(default_socket_path),
            handlers,
            state: ProxyState::new(),
            shutdown_tx,
        }
    }
    
    /// Get the socket path
    pub fn socket_path(&self) -> &PathBuf {
        &self.socket_path
    }
    
    /// Get a shutdown receiver
    pub fn shutdown_receiver(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }
    
    /// Signal shutdown
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
    
    /// Start the server
    pub async fn start(&self) -> Result<(), std::io::Error> {
        // Ensure parent directory exists
        if let Some(parent) = self.socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Remove existing socket file
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)?;
        }
        
        // Bind the socket
        let listener = UnixListener::bind(&self.socket_path)?;
        
        // Set socket permissions (owner-only: 0600)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&self.socket_path, std::fs::Permissions::from_mode(0o600))?;
        }
        
        info!("Proxy server listening on {:?}", self.socket_path);
        
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        
        loop {
            tokio::select! {
                // Accept new connections
                result = listener.accept() => {
                    match result {
                        Ok((stream, _addr)) => {
                            let handlers = Arc::clone(&self.handlers);
                            let state = Arc::clone(&self.state);
                            
                            // Increment connection count
                            {
                                let mut count = state.connection_count.write().await;
                                *count += 1;
                            }
                            {
                                let mut active = state.active_connections.write().await;
                                *active += 1;
                            }
                            
                            let state_clone = Arc::clone(&state);
                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(stream, handlers).await {
                                    error!("Connection error: {}", e);
                                }
                                
                                // Decrement active connections
                                let mut active = state_clone.active_connections.write().await;
                                *active = active.saturating_sub(1);
                            });
                        }
                        Err(e) => {
                            error!("Accept error: {}", e);
                        }
                    }
                }
                
                // Handle shutdown signal
                _ = shutdown_rx.recv() => {
                    info!("Proxy server shutting down");
                    break;
                }
            }
        }
        
        // Cleanup socket file
        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
        
        Ok(())
    }
}

/// Handle a single connection
async fn handle_connection(
    stream: UnixStream,
    handlers: Arc<HandlerRegistry>,
) -> Result<(), std::io::Error> {
    let peer_cred = stream.peer_cred().ok();
    if let Some(cred) = &peer_cred {
        debug!("Connection from PID: {:?}, UID: {:?}", cred.pid(), cred.uid());
        
        // SECURITY: Verify peer UID matches our UID (owner-only socket enforcement)
        #[cfg(unix)]
        {
            let my_uid = unsafe { libc::getuid() };
            let peer_uid = cred.uid();
            if peer_uid != my_uid {
                warn!(
                    "Rejecting connection from UID {} (expected {})",
                    peer_uid,
                    my_uid
                );
                return Ok(()); // Close connection without processing
            }
        }
    } else {
        warn!("Could not retrieve peer credentials, rejecting connection");
        return Ok(());
    }
    
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    
    loop {
        line.clear();
        
        // Read a line (each request is newline-delimited)
        match reader.read_line(&mut line).await {
            Ok(0) => {
                // EOF - client disconnected
                debug!("Client disconnected");
                break;
            }
            Ok(_) => {
                let response = process_request(&line, &handlers).await;
                
                // Serialize and send response
                let response_json = match serde_json::to_string(&response) {
                    Ok(json) => json,
                    Err(e) => {
                        error!("Failed to serialize response: {}", e);
                        continue;
                    }
                };
                
                // Write response with newline
                if let Err(e) = writer.write_all(response_json.as_bytes()).await {
                    warn!("Failed to write response: {}", e);
                    break;
                }
                if let Err(e) = writer.write_all(b"\n").await {
                    warn!("Failed to write newline: {}", e);
                    break;
                }
                if let Err(e) = writer.flush().await {
                    warn!("Failed to flush: {}", e);
                    break;
                }
            }
            Err(e) => {
                error!("Read error: {}", e);
                break;
            }
        }
    }
    
    Ok(())
}

/// Process a single request line
async fn process_request(
    line: &str,
    handlers: &HandlerRegistry,
) -> serde_json::Value {
    // Parse the request
    let requests = match parse_request(line) {
        Ok(reqs) => reqs,
        Err(error_response) => {
            return serde_json::to_value(error_response).unwrap_or_default();
        }
    };
    
    // Handle batch vs single
    if requests.len() == 1 {
        let request = &requests[0];
        
        // Validate request
        if let Err(e) = request.validate() {
            let id = request.id.clone().unwrap_or(serde_json::Value::Null);
            return serde_json::to_value(JsonRpcResponse::invalid_request(id, e))
                .unwrap_or_default();
        }
        
        // Dispatch to handler
        let response = handlers.handle(request).await;
        
        // Skip response for notifications
        if request.is_notification() {
            return serde_json::Value::Null;
        }
        
        serde_json::to_value(response).unwrap_or_default()
    } else {
        // Batch request - process all and return array
        let mut responses = Vec::new();
        
        for request in &requests {
            if let Err(e) = request.validate() {
                let id = request.id.clone().unwrap_or(serde_json::Value::Null);
                responses.push(JsonRpcResponse::invalid_request(id, e));
                continue;
            }
            
            let response = handlers.handle(request).await;
            
            // Only include response if not a notification
            if !request.is_notification() {
                responses.push(response);
            }
        }
        
        if responses.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::to_value(responses).unwrap_or_default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_socket_path() {
        let path = default_socket_path();
        assert!(path.ends_with("tairseach.sock"));
        assert!(path.to_string_lossy().contains(".tairseach"));
    }
}
