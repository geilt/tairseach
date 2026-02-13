# MCP Bridge

> **Location:** `crates/tairseach-mcp/`  
> **Lines:** ~1,467  
> **Purpose:** Standalone MCP server that proxies to Tairseach Unix socket

---

## Overview

The MCP bridge is a standalone binary that implements the Model Context Protocol (MCP) server specification. It translates MCP tool calls into JSON-RPC requests to the Tairseach Unix socket, enabling AI agents (Claude Desktop, custom MCP clients) to access macOS capabilities.

**Architecture:**
```
AI Agent (Claude Desktop)
    ↓ stdio (MCP protocol)
tairseach-mcp binary
    ↓ Unix socket (JSON-RPC)
Tairseach Proxy Server
    ↓
Capability Handlers
```

---

## File Structure

```
crates/tairseach-mcp/
├── src/
│   ├── main.rs         # MCP server entry point
│   ├── protocol.rs     # MCP protocol types
│   ├── tools.rs        # Tool discovery + invocation
│   └── initialize.rs   # Server initialization
├── Cargo.toml
└── README.md
```

---

## Key Types

### MCP Protocol

```rust
#[derive(Serialize, Deserialize)]
pub struct MCPRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Serialize, Deserialize)]
pub struct MCPResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<MCPError>,
}

#[derive(Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub inputSchema: Value,
}
```

---

## Tool Discovery

The MCP server discovers tools by querying the Tairseach manifest registry:

```rust
// tools.rs
pub async fn list_tools(socket_path: &Path) -> Result<Vec<Tool>, String> {
    // Connect to Tairseach socket
    let mut stream = UnixStream::connect(socket_path).await?;
    
    // Send JSON-RPC request to list manifests
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "server.manifests",
        "params": {}
    });
    
    stream.write_all(serde_json::to_string(&request)?.as_bytes()).await?;
    
    // Parse response and extract tools
    let response: Value = serde_json::from_slice(&buf)?;
    let manifests = response["result"]["manifests"].as_array()?;
    
    let mut tools = Vec::new();
    for manifest in manifests {
        for tool in manifest["tools"].as_array()? {
            tools.push(Tool {
                name: format!("{}.{}", manifest["id"], tool["name"]),
                description: tool["description"].as_str()?.to_string(),
                inputSchema: tool["inputSchema"].clone(),
            });
        }
    }
    
    Ok(tools)
}
```

---

## Tool Invocation

```rust
pub async fn call_tool(
    socket_path: &Path,
    tool_name: &str,
    params: Value,
) -> Result<Value, String> {
    let mut stream = UnixStream::connect(socket_path).await?;
    
    // Format: "namespace.method"
    let request = json!({
        "jsonrpc": "2.0",
        "id": rand::random::<u32>(),
        "method": tool_name,
        "params": params,
    });
    
    stream.write_all(serde_json::to_string(&request)?.as_bytes()).await?;
    stream.write_all(b"\n").await?;  // Delimiter
    
    // Read response
    let response: Value = serde_json::from_slice(&buf)?;
    
    if let Some(error) = response.get("error") {
        return Err(error["message"].as_str()?.to_string());
    }
    
    Ok(response["result"].clone())
}
```

---

## MCP Server Implementation

```rust
// main.rs
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let socket_path = PathBuf::from("/Users/user/.tairseach/socket");
    
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    
    let mut reader = BufReader::new(stdin);
    let mut writer = BufWriter::new(stdout);
    
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        
        let request: MCPRequest = serde_json::from_str(&line)?;
        
        let response = match request.method.as_str() {
            "initialize" => handle_initialize(request.id).await,
            "tools/list" => handle_tools_list(request.id, &socket_path).await,
            "tools/call" => handle_tools_call(request.id, request.params, &socket_path).await,
            _ => MCPResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(MCPError {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                }),
            },
        };
        
        writer.write_all(serde_json::to_string(&response)?.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }
}
```

---

## Usage

### Running the MCP Server

```bash
cd crates/tairseach-mcp
cargo run -- --socket ~/.tairseach/socket
```

### Claude Desktop Configuration

Add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "tairseach": {
      "command": "/path/to/tairseach-mcp",
      "args": ["--socket", "/Users/user/.tairseach/socket"]
    }
  }
}
```

---

## Testing

```bash
# Start Tairseach UI (starts proxy server)
npm run tauri dev

# In another terminal, start MCP server
cd crates/tairseach-mcp
cargo run -- --socket ~/.tairseach/socket

# Test tool discovery
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | cargo run -- --socket ~/.tairseach/socket

# Test tool invocation
echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"gmail.list_messages","arguments":{"query":"is:unread"}}}' | cargo run -- --socket ~/.tairseach/socket
```

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `tairseach-protocol` | Shared JSON-RPC types |
| `tokio` | Async runtime + Unix socket client |
| `serde_json` | JSON serialization |

---

*For manifest structure, see [manifests.md](manifests.md)*  
*For handler implementation, see [handlers.md](handlers.md)*
