# MCP Bridge Architecture

**Location:** `crates/tairseach-mcp/`

The **MCP Bridge** (`tairseach-mcp`) is a standalone binary that implements the [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) server specification. It translates MCP stdio-based requests into JSON-RPC calls over the Tairseach Unix domain socket.

This architecture enables any MCP-compatible client (Claude Desktop, MCP Inspector, etc.) to access Tairseach capabilities without requiring direct socket integration.

---

## Architecture Overview

```
MCP Client (Claude Desktop)
      ↓ stdio (JSON-RPC)
tairseach-mcp binary
      ↓ Unix socket (JSON-RPC)
Tairseach socket server
      ↓
HandlerRegistry / CapabilityRouter
```

**Key points:**
- **Protocol bridge:** MCP (stdio) ↔ Tairseach (socket)
- **Tool discovery:** Reads manifests from `~/.tairseach/manifests/`
- **Automatic mapping:** MCP tool names → socket method names
- **Stateless:** No persistent state; tools are registered via manifests

---

## Components

### 1. Main Event Loop

**File:** `main.rs`

The main loop reads **line-delimited JSON-RPC** from stdin and writes responses to stdout.

```rust
loop {
    let n = reader.read_line(&mut line).await?;
    if n == 0 {
        // stdin closed — sleep indefinitely to keep process alive
        // (MCP Inspector may close stdin prematurely)
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }

    let request: JsonRpcRequest = serde_json::from_str(&line)?;
    
    let response = match request.method.as_str() {
        "initialize" => handle_initialize(...),
        "tools/list" => registry.list_response(),
        "tools/call" => registry.call_tool(...).await,
        "resources/list" => empty_list(),
        "prompts/list" => empty_list(),
        _ => method_not_found(),
    };

    stdout.write_all(serde_json::to_string(&response)?.as_bytes()).await?;
    stdout.write_all(b"\n").await?;
    stdout.flush().await?;
}
```

**Stdin EOF handling:**
- When stdin reaches EOF (client closes the pipe), the bridge **does not exit**
- Instead, it sleeps indefinitely, allowing the process to remain alive
- This is required for MCP Inspector, which may close stdin but still expect the server to be reachable

---

### 2. Protocol Implementation

**File:** `protocol.rs`

Defines MCP protocol structs per the [MCP specification](https://modelcontextprotocol.io/specification).

#### Initialize

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-03-26",
    "capabilities": {},
    "clientInfo": { "name": "claude-desktop", "version": "1.0.0" }
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2025-03-26",
    "capabilities": {
      "tools": { "listChanged": false }
    },
    "serverInfo": { "name": "tairseach", "version": "0.1.0" },
    "instructions": "Tairseach provides macOS system capabilities..."
  }
}
```

**File:** `initialize.rs`

```rust
pub fn handle_initialize(req: InitializeRequest) -> InitializeResponse {
    InitializeResponse {
        protocol_version: MCP_PROTOCOL_VERSION.to_string(),
        capabilities: ServerCapabilities {
            tools: ToolsCapabilities { list_changed: false },
            resources: None,
        },
        server_info: ServerInfo {
            name: "tairseach".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        instructions: "Tairseach provides macOS system capabilities...".to_string(),
    }
}
```

---

#### tools/list

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list",
  "params": {}
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "tools": [
      {
        "name": "tairseach_contacts_list",
        "description": "List contacts from macOS Contacts app",
        "inputSchema": {
          "type": "object",
          "properties": {
            "limit": { "type": "number" },
            "offset": { "type": "number" }
          }
        }
      }
    ]
  }
}
```

---

#### tools/call

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "tairseach_contacts_list",
    "arguments": { "limit": 10 }
  }
}
```

**Response (success):**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"contacts\":[...],\"count\":10}"
      }
    ],
    "isError": false
  }
}
```

**Response (error):**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"error\":{\"code\":-32011,\"message\":\"Permission denied\"}}"
      }
    ],
    "isError": true
  }
}
```

---

### 3. Tool Registry

**File:** `tools.rs`

The **ToolRegistry** loads tools from manifest files and maintains:
- **MCP tool list** — tools exposed to MCP clients
- **Allowlist** — mapping from MCP tool names → socket method names

#### Manifest Loading

**Manifest directory:** `~/.tairseach/manifests/`

The registry recursively scans for `*.json` files and parses them as tool manifests.

**Manifest structure:**
```json
{
  "id": "contacts-macos",
  "tools": [
    {
      "name": "contacts_list",
      "description": "List contacts from macOS Contacts app",
      "inputSchema": { "type": "object", "properties": {...} },
      "mcpExpose": true,
      "annotations": {
        "readOnlyHint": true,
        "idempotentHint": true
      }
    }
  ],
  "implementation": {
    "methods": {
      "contacts_list": "contacts.list"
    }
  }
}
```

**Tool filtering:**
- Tools with `"mcpExpose": false` are **excluded** from MCP exposure
- Default: `mcpExpose = true` (expose unless explicitly disabled)

**Name transformation:**
- Manifest tool name: `contacts_list`
- MCP tool name: `tairseach_contacts_list` (prefixed)
- Socket method name: `contacts.list` (from `implementation.methods`)

---

#### Tool Registration

```rust
pub fn load() -> anyhow::Result<Self> {
    let base = manifest_base_dir()?;  // ~/.tairseach/manifests/
    let mut files = collect_json_files(&base)?;
    
    let mut tools = Vec::new();
    let mut allowlist = HashMap::new();
    
    for file in files {
        let manifest: Manifest = serde_json::from_str(&fs::read_to_string(&file)?)?;
        
        for tool in manifest.tools {
            if tool.mcp_expose == Some(false) {
                continue;  // Skip tools not exposed to MCP
            }
            
            let Some(method_name) = manifest.implementation.methods.get(&tool.name) else {
                continue;  // Skip tools without method mapping
            };
            
            let mcp_name = format!("tairseach_{}", tool.name);
            
            allowlist.insert(mcp_name.clone(), ToolIndexEntry {
                tool_name: tool.name.clone(),
                method_name: method_name.clone(),
            });
            
            tools.push(McpTool {
                name: mcp_name,
                description: tool.description,
                input_schema: tool.input_schema,
                annotations: tool.annotations.map(|a| ToolAnnotations { ... }),
            });
        }
    }
    
    tools.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Self { tools, allowlist })
}
```

**Allowlist example:**
```rust
HashMap {
    "tairseach_contacts_list" => ToolIndexEntry {
        tool_name: "contacts_list",
        method_name: "contacts.list",
    },
    "tairseach_server_status" => ToolIndexEntry {
        tool_name: "server_status",
        method_name: "server.status",
    },
}
```

---

#### Tool Calling

```rust
pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<ToolsCallResponse, ToolCallError> {
    // 1. Look up method name in allowlist
    let Some(entry) = self.allowlist.get(name) else {
        return Err(ToolCallError::UnknownTool(name.to_string()));
    };
    
    // 2. Connect to socket
    let mut client = SocketClient::connect().await?;
    
    // 3. Build JSON-RPC request
    let mut req = SocketRequest::new(entry.method_name.clone(), arguments);
    req.id = Some(json!(1));
    
    // 4. Call socket
    let resp = client.call(req).await?;
    
    // 5. Translate response to MCP format
    if let Some(err) = resp.error {
        return Ok(ToolsCallResponse {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: serde_json::to_string(&json!({"error": err}))?,
            }],
            is_error: true,
        });
    }
    
    Ok(ToolsCallResponse {
        content: vec![ToolContent {
            content_type: "text".to_string(),
            text: serde_json::to_string(&resp.result.unwrap_or(Value::Null))?,
        }],
        is_error: false,
    })
}
```

**Error handling:**
- Socket connection failure → `ToolCallError::Upstream`
- Unknown tool → `ToolCallError::UnknownTool` (MCP error -32601)
- Socket error response → MCP success with `isError: true`

---

### 4. Socket Client

**Dependency:** `tairseach-protocol` crate (shared with core server)

**Socket path:** `~/.tairseach/tairseach.sock` (or `/tmp/tairseach-{UID}.sock`)

**Connection:**
```rust
let mut client = SocketClient::connect().await?;
let response = client.call(request).await?;
```

**Protocol:** JSON-RPC 2.0 over Unix domain socket (newline-delimited)

---

## MCP Protocol Compliance

### Supported Methods

| Method | Status | Notes |
|--------|--------|-------|
| `initialize` | ✅ Full | Returns protocol version, capabilities, server info |
| `notifications/initialized` | ✅ Full | No-op (client acknowledgment) |
| `tools/list` | ✅ Full | Returns all MCP-exposed tools from manifests |
| `tools/call` | ✅ Full | Proxies to socket, translates responses |
| `resources/list` | ✅ Stub | Returns empty list (not yet implemented) |
| `resources/templates/list` | ✅ Stub | Returns empty list (not yet implemented) |
| `resources/read` | ❌ Error | Returns -32601 (not implemented) |
| `prompts/list` | ✅ Stub | Returns empty list (not yet implemented) |

### Capabilities

**Current capabilities:**
```json
{
  "tools": {
    "listChanged": false
  }
}
```

**Future capabilities:**
- `tools.listChanged: true` — when dynamic tool registration is implemented
- `resources` — expose files, documents, etc.
- `prompts` — predefined prompt templates

---

## Tool Annotations

MCP tools support **annotations** (hints for LLM behavior):

```json
{
  "name": "tairseach_contacts_list",
  "annotations": {
    "readOnlyHint": true,
    "idempotentHint": true,
    "destructiveHint": false,
    "openWorldHint": false
  }
}
```

**Annotation meanings:**
- `readOnlyHint`: Tool does not modify state (safe to call repeatedly)
- `idempotentHint`: Calling multiple times has same effect as once
- `destructiveHint`: Tool performs irreversible actions (deletions, etc.)
- `openWorldHint`: Tool interacts with external systems (internet, APIs)

**Manifest source:**
```json
{
  "tools": [
    {
      "name": "contacts_delete",
      "annotations": {
        "destructiveHint": true
      }
    }
  ]
}
```

---

## Example Manifests

### macOS Contacts

**File:** `~/.tairseach/manifests/contacts-macos.json`

```json
{
  "id": "contacts-macos",
  "version": "1.0.0",
  "name": "macOS Contacts",
  "description": "Access macOS Contacts app",
  "requires": {
    "permissions": [{ "name": "contacts" }]
  },
  "implementation": {
    "type": "internal",
    "module": "contacts",
    "methods": {
      "contacts_list": "contacts.list",
      "contacts_get": "contacts.get",
      "contacts_search": "contacts.search"
    }
  },
  "tools": [
    {
      "name": "contacts_list",
      "description": "List all contacts with optional pagination",
      "inputSchema": {
        "type": "object",
        "properties": {
          "limit": {
            "type": "number",
            "description": "Maximum number of contacts to return"
          },
          "offset": {
            "type": "number",
            "description": "Number of contacts to skip"
          }
        }
      },
      "annotations": {
        "readOnlyHint": true,
        "idempotentHint": true
      }
    }
  ]
}
```

**Result:**
- MCP tool name: `tairseach_contacts_list`
- Socket method: `contacts.list`
- Exposed in `tools/list` response

---

### Server Status

**File:** `~/.tairseach/manifests/server-status.json`

```json
{
  "id": "server-status",
  "implementation": {
    "methods": {
      "server_status": "server.status"
    }
  },
  "tools": [
    {
      "name": "server_status",
      "description": "Check Tairseach server status",
      "inputSchema": {
        "type": "object",
        "properties": {}
      },
      "annotations": {
        "readOnlyHint": true,
        "idempotentHint": true
      }
    }
  ]
}
```

---

## Usage

### MCP Client Configuration

**Claude Desktop config** (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "tairseach": {
      "command": "/Users/you/.tairseach/bin/tairseach-mcp",
      "args": ["--transport", "stdio"]
    }
  }
}
```

**MCP Inspector:**
```bash
npx @modelcontextprotocol/inspector tairseach-mcp --transport stdio
```

---

### Testing

**Manual test:**
```bash
$ echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' | tairseach-mcp
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-03-26","capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"tairseach","version":"0.1.0"},"instructions":"..."}}

$ echo '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | tairseach-mcp
{"jsonrpc":"2.0","id":2,"result":{"tools":[...]}}

$ echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"tairseach_server_status","arguments":{}}}' | tairseach-mcp
{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"{\"status\":\"running\",\"version\":\"0.1.0\"}"}],"isError":false}}
```

---

## Error Handling

### MCP-Level Errors

| Error Code | Meaning | Example |
|------------|---------|---------|
| -32700 | Parse error | Invalid JSON |
| -32600 | Invalid request | Missing `jsonrpc` field |
| -32601 | Method not found | Unknown MCP method |
| -32602 | Invalid params | Wrong param structure |

### Socket-Level Errors

When the socket returns an error, the MCP bridge wraps it in a **success response** with `isError: true`:

**Socket response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32011,
    "message": "Permission denied: contacts"
  }
}
```

**MCP response:**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"error\":{\"code\":-32011,\"message\":\"Permission denied: contacts\"}}"
      }
    ],
    "isError": true
  }
}
```

**Rationale:** MCP clients (like Claude Desktop) expect `isError` to distinguish between tool failures and protocol errors. A JSON-RPC error would indicate a bridge/protocol problem, not a tool-specific failure.

---

## Security Model

### Socket Authorization

The socket server enforces **socket-level authorization**:
- Only trusted clients (Tairseach process, MCP bridge, authorized scripts) can connect
- Socket path is restricted to user's home directory (or `/tmp` with UID suffix)

### No Additional Auth

The MCP bridge does **not** implement additional authentication:
- **Rationale:** MCP is designed for single-user environments (local stdio)
- Security relies on OS-level process isolation
- Only the user running Tairseach can access the socket

### Credential Isolation

Credentials (OAuth tokens, API keys) are **never exposed** to MCP clients:
- Credentials loaded by `CapabilityRouter` from `AuthBroker`
- Injected into HTTP headers or script environments
- Tool responses contain only result data, not credentials

---

## Future Enhancements

### Planned Features

1. **Dynamic tool registration**
   - Set `tools.listChanged: true`
   - Emit `notifications/tools/list_changed` when manifests are added/removed
   - Support hot-reloading of manifests

2. **Resources**
   - Expose files, documents, logs as MCP resources
   - Example: `file://~/.tairseach/logs/server.log`
   - Enable Claude to read Tairseach state

3. **Prompts**
   - Predefined prompt templates
   - Example: "Search contacts for {name}"
   - Enable consistent LLM interaction patterns

4. **Streaming**
   - Long-running tool calls with progress updates
   - MCP progress notifications

5. **Sampling**
   - Allow tools to request LLM completions (agent-in-the-loop)
   - Example: "Confirm deletion of contact {name}?"

### Performance Optimizations

- **Connection pooling:** Reuse socket connections instead of connect-per-call
- **Manifest caching:** Load manifests once, watch for changes
- **Async batching:** Bundle multiple tool calls into a single socket connection

---

## Build & Deployment

**Build:**
```bash
cd crates/tairseach-mcp
cargo build --release
```

**Binary location:** `target/release/tairseach-mcp`

**Install:**
```bash
mkdir -p ~/.tairseach/bin
cp target/release/tairseach-mcp ~/.tairseach/bin/
```

**Dependencies:**
- `tokio` — async runtime
- `serde_json` — JSON serialization
- `clap` — CLI parsing
- `tairseach-protocol` — socket client

---

## Debugging

**Enable trace logging:**
```bash
RUST_LOG=tairseach_mcp=debug tairseach-mcp --transport stdio
```

**Common issues:**

1. **"socket connect failed"**
   - Ensure Tairseach server is running (`tairseach server start`)
   - Check socket path: `ls -la ~/.tairseach/tairseach.sock`

2. **"unknown tool"**
   - Tool not in manifest or `mcpExpose: false`
   - Check manifest files: `ls ~/.tairseach/manifests/`
   - Validate manifest JSON

3. **"Permission denied"**
   - macOS TCC permission not granted
   - Check: `tairseach-mcp → tools/call → permissions.check`
   - Grant permission in System Settings

---

**See also:**
- [Router Architecture](./router.md) — manifest-based routing
- [Manifest System](./manifest-system.md) — manifest file format
- [Handlers](./handlers.md) — socket method implementations
- [MCP Specification](https://modelcontextprotocol.io/specification)
