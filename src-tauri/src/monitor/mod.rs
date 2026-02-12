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
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/"))
        .join(".tairseach/logs/proxy.log")
}

fn manifests_root() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/"))
        .join(".tairseach/manifests")
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
    let socket_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/"))
        .join(".tairseach/tairseach.sock");

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
    let socket_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/"))
        .join(".tairseach/tairseach.sock");

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
