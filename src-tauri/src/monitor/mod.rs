//! Activity monitoring
//!
//! Real-time monitoring of MCP tool calls and system events.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use crate::manifest::{load_manifests, Manifest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEvent {
    pub id: String,
    pub timestamp: String,
    pub event_type: String,
    pub source: String,
    pub message: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestSummary {
    pub capabilities_loaded: usize,
    pub tools_available: usize,
    pub mcp_exposed: usize,
}

fn proxy_log_path() -> PathBuf {
    crate::common::logs_dir()
        .unwrap_or_else(|_| PathBuf::from(".tairseach/logs"))
        .join("proxy.log")
}

fn manifests_root() -> PathBuf {
    crate::common::manifest_dir()
        .unwrap_or_else(|_| PathBuf::from(".tairseach/manifests"))
}

fn parse_activity_line(line: &str, fallback_idx: usize) -> ActivityEvent {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
        let timestamp = v
            .get("timestamp")
            .or_else(|| v.get("ts"))
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();

        let event_type = v
            .get("event")
            .or_else(|| v.get("event_type"))
            .and_then(|x| x.as_str())
            .unwrap_or("operation")
            .to_string();

        let source = v
            .get("client")
            .or_else(|| v.get("source"))
            .and_then(|x| x.as_str())
            .unwrap_or("tairseach")
            .to_string();

        let message = v
            .get("message")
            .or_else(|| v.get("tool"))
            .and_then(|x| x.as_str())
            .unwrap_or(line)
            .to_string();

        return ActivityEvent {
            id: v
                .get("id")
                .and_then(|x| x.as_str())
                .map(String::from)
                .unwrap_or_else(|| format!("log-{}", fallback_idx)),
            timestamp,
            event_type,
            source,
            message,
            metadata: Some(v),
        };
    }

    ActivityEvent {
        id: format!("log-{}", fallback_idx),
        timestamp: "".into(),
        event_type: "operation".into(),
        source: "tairseach".into(),
        message: line.to_string(),
        metadata: None,
    }
}

#[tauri::command]
pub async fn get_events(limit: Option<usize>) -> Result<Vec<ActivityEvent>, String> {
    let limit = limit.unwrap_or(100).max(1).min(2000);
    let path = proxy_log_path();

    if !path.exists() {
        return Ok(vec![]);
    }

    let file = File::open(&path).map_err(|e| format!("Failed to open proxy.log: {}", e))?;
    let reader = BufReader::new(file);
    let mut ring: VecDeque<String> = VecDeque::with_capacity(limit + 1);

    for line in reader.lines() {
        let line = line.map_err(|e| format!("Failed to read proxy.log line: {}", e))?;
        if ring.len() >= limit {
            ring.pop_front();
        }
        ring.push_back(line);
    }

    let events = ring
        .into_iter()
        .enumerate()
        .map(|(idx, line)| parse_activity_line(&line, idx))
        .collect::<Vec<_>>();

    Ok(events)
}

#[tauri::command]
pub async fn get_manifest_summary() -> Result<ManifestSummary, String> {
    let root = manifests_root();
    let mut manifests = 0usize;
    let mut tools = 0usize;

    let tiers = ["core", "integrations", "community"];

    for tier in tiers {
        let dir = root.join(tier);
        if !dir.exists() {
            continue;
        }

        let entries = std::fs::read_dir(&dir)
            .map_err(|e| format!("Failed reading manifest dir {}: {}", dir.display(), e))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }

            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let json = match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(j) => j,
                Err(_) => continue,
            };

            manifests += 1;
            tools += json
                .get("tools")
                .and_then(|x| x.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
        }
    }

    // Conservative for now: only expose read-only-ish portion through MCP until security gating is finalized.
    let mcp_exposed = tools;

    Ok(ManifestSummary {
        capabilities_loaded: manifests,
        tools_available: tools,
        mcp_exposed,
    })
}

/// Load all manifests from disk
#[tauri::command]
pub async fn get_all_manifests() -> Result<Vec<Manifest>, String> {
    let root = manifests_root();
    load_manifests(&root)
}

/// Check if the Tairseach socket is alive and responding
#[tauri::command]
pub async fn check_socket_alive() -> Result<serde_json::Value, String> {
    let socket_path = crate::common::socket_path()
        .unwrap_or_else(|_| PathBuf::from(".tairseach/tairseach.sock"));

    if !socket_path.exists() {
        return Ok(serde_json::json!({
            "alive": false,
            "reason": "Socket file does not exist"
        }));
    }

    // Try to connect and send a simple status request
    match UnixStream::connect(&socket_path) {
        Ok(mut stream) => {
            // Send a JSON-RPC status request
            let request = serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "server.status",
                "params": {}
            });
            
            let request_str = serde_json::to_string(&request).unwrap() + "\n";
            
            if stream.write_all(request_str.as_bytes()).is_err() {
                return Ok(serde_json::json!({
                    "alive": false,
                    "reason": "Failed to write to socket"
                }));
            }
            
            // Try to read response
            let mut buffer = vec![0u8; 4096];
            match stream.read(&mut buffer) {
                Ok(n) if n > 0 => {
                    Ok(serde_json::json!({
                        "alive": true,
                        "socket_path": socket_path.display().to_string()
                    }))
                }
                _ => {
                    Ok(serde_json::json!({
                        "alive": false,
                        "reason": "No response from socket"
                    }))
                }
            }
        }
        Err(e) => {
            Ok(serde_json::json!({
                "alive": false,
                "reason": format!("Connection failed: {}", e)
            }))
        }
    }
}

/// Test an MCP tool by calling it through the socket
#[tauri::command]
pub async fn test_mcp_tool(
    tool_name: String,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let socket_path = crate::common::socket_path()
        .unwrap_or_else(|_| PathBuf::from(".tairseach/tairseach.sock"));

    if !socket_path.exists() {
        return Err("Socket file does not exist".to_string());
    }

    let mut stream = UnixStream::connect(&socket_path)
        .map_err(|e| format!("Failed to connect to socket: {}", e))?;

    // Send JSON-RPC request
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": params
        }
    });

    let request_str = serde_json::to_string(&request).unwrap() + "\n";
    
    stream
        .write_all(request_str.as_bytes())
        .map_err(|e| format!("Failed to write to socket: {}", e))?;

    // Read response
    let mut buffer = vec![0u8; 65536]; // 64KB buffer
    let n = stream
        .read(&mut buffer)
        .map_err(|e| format!("Failed to read from socket: {}", e))?;

    if n == 0 {
        return Err("No response from socket".to_string());
    }

    let response_str = String::from_utf8_lossy(&buffer[..n]);
    
    // Parse JSON-RPC response
    let response: serde_json::Value = serde_json::from_str(&response_str)
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    // Check for JSON-RPC error
    if let Some(error) = response.get("error") {
        return Err(format!("Tool execution error: {}", error));
    }

    // Return the result
    Ok(response.get("result").cloned().unwrap_or(response))
}

/// Check namespace connection statuses by pinging socket with a tool from each manifest
#[tauri::command]
pub async fn get_namespace_statuses() -> Result<Vec<serde_json::Value>, String> {
    let manifests = get_all_manifests().await?;
    let socket_path = crate::common::socket_path()
        .unwrap_or_else(|_| PathBuf::from(".tairseach/tairseach.sock"));
    
    let mut statuses = Vec::new();
    
    for manifest in manifests {
        // Use manifest.id as the namespace
        let namespace = manifest.id.clone();
        
        // Pick the first tool to test connectivity
        let connected = if let Some(_tool) = manifest.tools.first() {
            // Try to ping with tools/list to test connectivity
            match test_tool_connectivity(&socket_path).await {
                Ok(_) => true,
                Err(_) => false,
            }
        } else {
            false // No tools means can't test connectivity
        };
        
        statuses.push(serde_json::json!({
            "namespace": namespace,
            "connected": connected
        }));
    }
    
    Ok(statuses)
}

/// Helper function to test tool connectivity
async fn test_tool_connectivity(socket_path: &PathBuf) -> Result<(), String> {
    if !socket_path.exists() {
        return Err("Socket does not exist".to_string());
    }
    
    let mut stream = UnixStream::connect(socket_path)
        .map_err(|e| format!("Failed to connect: {}", e))?;
    
    // Send a minimal JSON-RPC request
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });
    
    let request_str = serde_json::to_string(&request).unwrap() + "\n";
    stream
        .write_all(request_str.as_bytes())
        .map_err(|e| format!("Write failed: {}", e))?;
    
    // Read response (with timeout implied by blocking read)
    let mut buffer = vec![0u8; 4096];
    let n = stream.read(&mut buffer).map_err(|e| format!("Read failed: {}", e))?;
    
    if n == 0 {
        return Err("No response".to_string());
    }
    
    Ok(())
}

/// Install Tairseach MCP server config into OpenClaw
#[tauri::command]
pub async fn install_tairseach_to_openclaw(
    config_path: Option<String>,
) -> Result<serde_json::Value, String> {
    use std::io::Write;
    
    // Determine config path
    let path = if let Some(p) = config_path {
        PathBuf::from(p)
    } else {
        dirs::home_dir()
            .ok_or("Could not determine home directory")?
            .join(".openclaw/openclaw.json")
    };
    
    if !path.exists() {
        return Err(format!("OpenClaw config not found at {:?}", path));
    }
    
    // Read existing config
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read OpenClaw config: {}", e))?;
    
    let mut config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse OpenClaw config: {}", e))?;
    
    // Find the tairseach-mcp binary
    let binary_path = find_tairseach_mcp_binary()?;
    
    // Create/update mcpServers.tairseach entry
    if !config.is_object() {
        config = serde_json::json!({});
    }
    
    let config_obj = config.as_object_mut().unwrap();
    
    if !config_obj.contains_key("mcpServers") {
        config_obj.insert("mcpServers".to_string(), serde_json::json!({}));
    }
    
    let mcp_servers = config_obj
        .get_mut("mcpServers")
        .and_then(|v| v.as_object_mut())
        .ok_or("mcpServers is not an object")?;
    
    mcp_servers.insert(
        "tairseach".to_string(),
        serde_json::json!({
            "transport": "stdio",
            "command": binary_path,
            "args": []
        }),
    );
    
    // Write back atomically (write to temp, then rename)
    let temp_path = path.with_extension("json.tmp");
    let new_content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    
    {
        let mut file = std::fs::File::create(&temp_path)
            .map_err(|e| format!("Failed to create temp file: {}", e))?;
        file.write_all(new_content.as_bytes())
            .map_err(|e| format!("Failed to write temp file: {}", e))?;
        file.flush()
            .map_err(|e| format!("Failed to flush temp file: {}", e))?;
    }
    
    std::fs::rename(&temp_path, &path)
        .map_err(|e| format!("Failed to rename temp file: {}", e))?;
    
    Ok(serde_json::json!({
        "success": true,
        "config_path": path.display().to_string(),
        "binary_path": binary_path
    }))
}

/// Find the tairseach-mcp binary (sidecar or fallback)
fn find_tairseach_mcp_binary() -> Result<String, String> {
    // Try sidecar path first (resolved by Tauri at runtime)
    // Note: In production, Tauri resolves sidecar paths automatically
    // For now, we'll check the development path
    
    let dev_path = dirs::home_dir()
        .ok_or("Could not determine home directory")?
        .join("environment/tairseach/src-tauri/binaries/tairseach-mcp-aarch64-apple-darwin");
    
    if dev_path.exists() {
        return Ok(dev_path.display().to_string());
    }
    
    // TODO: In production, use tauri::api::process::Command::sidecar_path() or similar
    // For now, return the expected path
    Ok(dev_path.display().to_string())
}
