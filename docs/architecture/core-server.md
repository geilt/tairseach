# Core Server Architecture

**Component:** Proxy Socket Server  
**Location:** `src-tauri/src/proxy/`  
**Protocol:** JSON-RPC 2.0 over Unix Domain Socket  
**Socket Path:** `~/.tairseach/tairseach.sock`

---

## Purpose

The core server is a Unix domain socket server that accepts JSON-RPC 2.0 requests and dispatches them to appropriate handlers. It serves as the primary integration point for external applications (AI agents, CLI tools, scripts) to access Tairseach capabilities.

## Architecture Overview

```
┌──────────────────────────────────────────────────────────────┐
│                    External Client                            │
│              (OpenClaw, scripts, MCP bridge)                  │
└────────────────────────┬─────────────────────────────────────┘
                         │
                         │ Unix Socket Connection
                         │ ~/.tairseach/tairseach.sock
                         │
┌────────────────────────▼─────────────────────────────────────┐
│                    ProxyServer                                │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  start() - Main Event Loop                             │  │
│  │  • UnixListener::bind()                                │  │
│  │  • Set socket permissions (0600)                       │  │
│  │  • Loop: accept connections + handle shutdown          │  │
│  └────────────────────┬───────────────────────────────────┘  │
│                       │                                       │
│                       │ spawn task per connection             │
│                       │                                       │
│  ┌────────────────────▼───────────────────────────────────┐  │
│  │  handle_connection()                                    │  │
│  │  • Verify peer UID == owner UID (security)             │  │
│  │  • Read newline-delimited JSON-RPC requests            │  │
│  │  • Call process_request()                              │  │
│  │  • Write newline-delimited JSON-RPC responses          │  │
│  └────────────────────┬───────────────────────────────────┘  │
│                       │                                       │
│  ┌────────────────────▼───────────────────────────────────┐  │
│  │  process_request()                                      │  │
│  │  • parse_request() → Vec<JsonRpcRequest>               │  │
│  │  • Validate each request                               │  │
│  │  • HandlerRegistry::handle() for each                  │  │
│  │  • Return single or batch response                     │  │
│  └────────────────────┬───────────────────────────────────┘  │
└───────────────────────┼───────────────────────────────────────┘
                        │
          ┌─────────────▼──────────────┐
          │    HandlerRegistry         │
          │  (see handlers.md)         │
          └────────────────────────────┘
```

## Key Files

### `server.rs` — Socket Server Implementation

**Lines:** ~250  
**Responsibilities:**
- Socket lifecycle (bind, listen, accept, close)
- Connection security (UID verification)
- Per-connection task spawning
- Shutdown signal handling

**Key Types:**

```rust
pub struct ProxyServer {
    socket_path: PathBuf,              // ~/.tairseach/tairseach.sock
    handlers: Arc<HandlerRegistry>,    // Request dispatcher
    state: Arc<ProxyState>,            // Connection metrics
    shutdown_tx: broadcast::Sender<()>, // Shutdown signal
}
```

**Key Functions:**

```rust
pub async fn start(&self) -> Result<(), std::io::Error>
```
Main server loop. Binds socket, sets permissions, accepts connections, spawns `handle_connection` tasks.

**Security:** UID Check
```rust
let peer_uid = stream.peer_cred()?.uid();
let my_uid = unsafe { libc::getuid() };
if peer_uid != my_uid {
    warn!("Rejecting connection from UID {}", peer_uid);
    return Ok(()); // Drop connection
}
```

**Why this matters:** Socket file permissions (0600) prevent other users from connecting, but this is defense-in-depth — explicitly verify the peer process runs as the same user.

### `protocol.rs` — JSON-RPC 2.0 Implementation

**Lines:** ~180  
**Responsibilities:**
- JSON-RPC request/response parsing
- Batch request support
- Standard error code handling
- Request validation

**Key Types:**

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,         // Must be "2.0"
    pub id: Option<Value>,       // Correlation ID (None = notification)
    pub method: String,          // e.g., "contacts.list"
    pub params: Value,           // Arbitrary JSON params
}
```

```rust
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,         // "2.0"
    pub id: Value,               // Copied from request
    pub result: Option<Value>,   // Success payload
    pub error: Option<JsonRpcError>, // Error details
}
```

**Standard Error Codes:**

| Code | Meaning | When Used |
|------|---------|-----------|
| -32700 | Parse error | Invalid JSON |
| -32600 | Invalid Request | Missing required fields, wrong version |
| -32601 | Method not found | Unknown method name |
| -32602 | Invalid params | Params don't match expected schema |
| -32603 | Internal error | Handler crashed or unexpected error |
| -32001 | Permission denied | Custom: macOS permission not granted |

**Batch Request Support:**

The protocol supports sending multiple requests in a single message:

```json
[
  {"jsonrpc":"2.0","id":1,"method":"contacts.list","params":{}},
  {"jsonrpc":"2.0","id":2,"method":"calendar.events","params":{"start":"2026-02-13"}}
]
```

Response is an array:

```json
[
  {"jsonrpc":"2.0","id":1,"result":[...]},
  {"jsonrpc":"2.0","id":2,"result":[...]}
]
```

**Notifications:**

Requests without an `id` are notifications (no response expected):

```json
{"jsonrpc":"2.0","method":"log.info","params":{"message":"Hello"}}
```

Server processes the request but does not send a response.

### `mod.rs` — Module Exports & State

**Lines:** ~50  
**Responsibilities:**
- Re-export key types
- Define shared server state
- Define `ProxyError` enum

**Shared State:**

```rust
pub struct ProxyState {
    pub connection_count: RwLock<u64>,    // Total connections since start
    pub active_connections: RwLock<u32>,  // Current open connections
}
```

Used for metrics and monitoring (exposed via `server.status` method).

## Request Lifecycle

### 1. Connection Establishment

```
Client ──socket──> ProxyServer::start()
                   ↓
          UnixListener::accept()
                   ↓
          spawn handle_connection(stream)
```

### 2. Security Check

```rust
let peer_cred = stream.peer_cred().ok();
let peer_uid = cred.uid();
let my_uid = unsafe { libc::getuid() };

if peer_uid != my_uid {
    // Reject and close connection
    return Ok(());
}
```

**Why UID check?**
- Socket file has mode 0600 (owner-only)
- But if file permissions change, this is the backstop
- Prevents privilege escalation via socket hijacking

### 3. Request Reading

```rust
let (reader, mut writer) = stream.into_split();
let mut reader = BufReader::new(reader);
let mut line = String::new();

loop {
    line.clear();
    match reader.read_line(&mut line).await {
        Ok(0) => break,  // EOF - client disconnected
        Ok(_) => {
            // Process line
        }
        Err(e) => {
            error!("Read error: {}", e);
            break;
        }
    }
}
```

**Protocol:** Newline-delimited JSON-RPC

Each request is a single line terminated with `\n`:

```
{"jsonrpc":"2.0","id":1,"method":"contacts.list","params":{}}\n
{"jsonrpc":"2.0","id":2,"method":"calendar.events","params":{...}}\n
```

**Why newline-delimited?**
- Simple framing — no need for length prefixes
- Easy to implement in any language
- Compatible with line-buffered tools (socat, netcat)
- Human-readable for debugging

### 4. Request Parsing

```rust
fn parse_request(input: &str) -> Result<Vec<JsonRpcRequest>, JsonRpcResponse>
```

**Steps:**
1. Trim whitespace
2. Check if batch (starts with `[`) or single request
3. Parse JSON with serde_json
4. Return `Vec<JsonRpcRequest>` (single request → vec of 1)

**Error Handling:**
- Invalid JSON → `-32700` Parse error
- Empty batch → `-32600` Invalid request

### 5. Request Validation

```rust
impl JsonRpcRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.jsonrpc != "2.0" {
            return Err("Invalid JSON-RPC version");
        }
        if self.method.is_empty() {
            return Err("Method cannot be empty");
        }
        Ok(())
    }
}
```

**Validation checks:**
- `jsonrpc` field must be exactly `"2.0"`
- `method` field cannot be empty string
- `id` can be null, string, number, or absent (for notifications)

### 6. Handler Dispatch

```rust
let response = handlers.handle(&request).await;
```

See [handlers.md](handlers.md) for detailed dispatch logic.

**Key:** `HandlerRegistry::handle()` is async — handlers can perform I/O, call APIs, etc.

### 7. Response Serialization

```rust
let response_json = serde_json::to_string(&response)?;
writer.write_all(response_json.as_bytes()).await?;
writer.write_all(b"\n").await?;
writer.flush().await?;
```

**Response format:** Same as request — newline-delimited JSON

**Notification handling:** If request has no `id`, skip response (return early).

### 8. Batch Response Assembly

For batch requests:

```rust
let mut responses = Vec::new();
for request in &requests {
    if request.is_notification() {
        continue; // Skip response
    }
    let response = handlers.handle(request).await;
    responses.push(response);
}

if responses.is_empty() {
    serde_json::Value::Null  // All were notifications
} else {
    serde_json::to_value(responses)?
}
```

## Server Lifecycle

### Startup (in `lib.rs`)

```rust
pub fn run() {
    tauri::Builder::default()
        .setup(|_app| {
            tauri::async_runtime::spawn(async {
                tokio::time::sleep(Duration::from_millis(500)).await;
                if let Err(e) = start_proxy_server_internal().await {
                    tracing::error!("Failed to start proxy server: {}", e);
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Sequence:**
1. Tauri app starts
2. Wait 500ms for app initialization
3. Spawn `start_proxy_server_internal()`
4. Initialize manifest registry
5. Start manifest hot-reload watcher
6. Initialize auth broker
7. Create capability router
8. Create handler registry
9. Create proxy server
10. Call `server.start()` (blocks until shutdown)

### Initialization (`start_proxy_server_internal`)

```rust
async fn start_proxy_server_internal() -> Result<(), String> {
    // 1. Load manifests
    let registry = Arc::new(ManifestRegistry::new());
    registry.load_from_disk().await?;
    
    // 2. Start manifest watcher (hot-reload)
    tokio::spawn(async move {
        registry_clone.start_watcher().await?;
    });
    
    // 3. Initialize auth broker
    let auth_broker = AuthBroker::new().await?;
    auth_broker.spawn_refresh_daemon();
    
    // 4. Create capability router
    let router = Arc::new(CapabilityRouter::new(registry, auth_broker));
    
    // 5. Create handler registry
    let handlers = Arc::new(HandlerRegistry::with_router(router));
    
    // 6. Create and start server
    let server = Arc::new(ProxyServer::with_handlers(None, handlers));
    server.start().await?;
    
    Ok(())
}
```

### Shutdown

**Method 1: Shutdown signal**

```rust
let mut shutdown_rx = self.shutdown_tx.subscribe();
tokio::select! {
    _ = shutdown_rx.recv() => {
        info!("Proxy server shutting down");
        break;
    }
}
```

**Method 2: Tauri command**

```rust
#[tauri::command]
async fn stop_proxy_server() -> Result<serde_json::Value, String> {
    let state = PROXY_STATE.read().await;
    if let Some(server) = &state.server {
        server.shutdown();  // Sends shutdown signal
    }
    Ok(json!({"stopped": true}))
}
```

**Cleanup:**

```rust
// Remove socket file
if self.socket_path.exists() {
    let _ = std::fs::remove_file(&self.socket_path);
}
```

## Configuration

### Socket Path

**Default:** `~/.tairseach/tairseach.sock`

**Determined by:**

```rust
pub fn default_socket_path() -> PathBuf {
    crate::common::socket_path()
        .expect("Failed to determine socket path")
}

// In common/paths.rs:
pub fn socket_path() -> Result<PathBuf, String> {
    Ok(tairseach_dir()?.join("tairseach.sock"))
}

pub fn tairseach_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir()
        .ok_or("Could not determine home directory")?;
    Ok(home.join(".tairseach"))
}
```

**Customization:** Currently not supported (hardcoded). Could be extended with:

```rust
ProxyServer::new(Some(PathBuf::from("/custom/path.sock")))
```

### Socket Permissions

**Mode:** 0600 (owner read/write only)

```rust
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(
        &self.socket_path,
        std::fs::Permissions::from_mode(0o600)
    )?;
}
```

**Why 0600?**
- Prevents other users from reading/writing the socket
- Only the app owner can connect
- Combined with UID check for defense-in-depth

### Connection Limits

**Current:** Unlimited

**Concurrency:** Each connection runs in its own Tokio task — bounded only by OS limits (file descriptors, memory).

**Future consideration:** Add max connection limit:

```rust
if state.active_connections.read().await >= MAX_CONNECTIONS {
    warn!("Max connections reached, rejecting new connection");
    return Ok(());
}
```

## Error Handling

### Connection Errors

**Scenario:** Client sends malformed data

```rust
Err(e) => {
    error!("Read error: {}", e);
    break; // Close connection
}
```

**Behavior:** Close connection, log error, do NOT crash server.

### Parse Errors

**Scenario:** Invalid JSON

```json
{"jsonrpc":"2.0","id":1,method:"contacts.list"}  // Missing quotes around method
```

**Response:**

```json
{
  "jsonrpc": "2.0",
  "id": null,
  "error": {
    "code": -32700,
    "message": "expected `:` at line 1 column 22"
  }
}
```

**Note:** `id` is `null` because we couldn't parse the request to extract the real ID.

### Handler Errors

**Scenario:** Handler returns error

```rust
JsonRpcResponse::error(
    id,
    -32603,
    "Failed to access contacts: permission denied",
    None
)
```

**Response:**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32603,
    "message": "Failed to access contacts: permission denied"
  }
}
```

### Panic Recovery

**Current:** No explicit panic recovery — if handler panics, the task panics, connection closes.

**Future improvement:**

```rust
let response = tokio::task::spawn(async move {
    handlers.handle(request).await
})
.await;

match response {
    Ok(resp) => resp,
    Err(e) => {
        // Task panicked
        JsonRpcResponse::internal_error(id, format!("Handler panicked: {}", e))
    }
}
```

## Performance Characteristics

### Throughput

**Bottleneck:** Handler execution time (varies by operation)

**Socket I/O:** Negligible overhead — Unix domain sockets are ~2x faster than TCP localhost.

**Concurrency:** Each connection is independent — no lock contention on request handling.

**Benchmark data:** Not yet collected.

### Latency

**Typical request latency:**
- Parse + dispatch: < 1ms
- Permission check: < 1ms (cached)
- Handler execution: 1ms - 500ms (depends on operation)
- Total: Dominated by handler logic

**Network latency:** None — Unix domain socket is in-memory.

### Resource Usage

**Memory:**
- Server struct: ~1KB
- Per-connection: ~64KB (Tokio task stack)
- Handler registry: ~100KB (loaded at startup)
- Auth broker: Varies (tokens in memory)

**File Descriptors:**
- 1 for socket listener
- 2 per active connection (read + write halves)

## Monitoring & Observability

### Server Status

**Command:** `server.status`

**Request:**

```json
{"jsonrpc":"2.0","id":1,"method":"server.status","params":{}}
```

**Response:**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "running": true,
    "socket_path": "/Users/geilt/.tairseach/tairseach.sock",
    "connection_count": 42,
    "active_connections": 3,
    "uptime_seconds": 3600
  }
}
```

### Logging

**Backend:** `tracing` crate

**Log Levels:**
- `debug`: Connection accept/close, request method names
- `info`: Server start/stop, lifecycle events
- `warn`: Rejected connections, recoverable errors
- `error`: Critical errors, handler panics

**Example logs:**

```
2026-02-13T01:23:45Z INFO  Proxy server listening on "/Users/geilt/.tairseach/tairseach.sock"
2026-02-13T01:23:46Z DEBUG Connection from PID: 12345, UID: 501
2026-02-13T01:23:46Z DEBUG Routing tool call: contacts.list
2026-02-13T01:23:47Z WARN  Rejecting connection from UID 502
```

### Metrics

**Available in ProxyState:**
- `connection_count`: Total connections since start
- `active_connections`: Currently open connections

**Future:** Expose via Prometheus endpoint or log periodically.

## Security Considerations

### UID Verification

**Attack:** Attacker runs process as different user, tries to connect to socket.

**Defense:** UID check rejects connection if `peer_uid != my_uid`.

**Limitation:** If socket file permissions are misconfigured (e.g., 0666), UID check is the only defense.

### Socket File Location

**Path:** `~/.tairseach/tairseach.sock`

**Security:**
- Parent directory (`~/.tairseach`) is user-owned
- Socket file created with mode 0600 on bind
- Removed on shutdown

**Risk:** If `~/.tairseach` is world-writable, attacker could replace socket with symlink → denial of service.

**Mitigation:** Ensure parent directory is 0700 (see `permissions` module).

### Request Validation

**Input validation:** Minimal — only checks `jsonrpc` version and method name non-empty.

**Param validation:** Delegated to handlers.

**Risk:** Malformed params can crash handlers if they don't validate input.

**Mitigation:** Use typed param structs with `serde(deny_unknown_fields)`.

### Denial of Service

**Attack vectors:**
1. **Connection flood:** Open many connections to exhaust file descriptors.
2. **Large payload:** Send huge JSON payload to exhaust memory.
3. **Slow read:** Open connection, read slowly to exhaust connection pool.

**Current mitigations:**
- OS-level file descriptor limit
- No explicit connection limit (TODO)
- No payload size limit (TODO)
- Connection timeout: None (TODO)

**Recommended improvements:**
1. Add max connection limit (e.g., 100)
2. Add max payload size (e.g., 10MB)
3. Add per-connection timeout (e.g., 60s idle)

## Testing

### Unit Tests

**File:** `server.rs`

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_default_socket_path() {
        let path = default_socket_path();
        assert!(path.ends_with("tairseach.sock"));
    }
}
```

### Integration Tests

**Manual testing:**

```bash
# Start Tairseach.app
open ~/Applications/Tairseach.app

# Test socket with socat
echo '{"jsonrpc":"2.0","id":1,"method":"server.status","params":{}}' | \
  socat - UNIX-CONNECT:~/.tairseach/tairseach.sock

# Expected output:
# {"jsonrpc":"2.0","id":1,"result":{"running":true,...}}
```

**Automated testing:**

```rust
// TODO: Create integration test harness
#[tokio::test]
async fn test_socket_request_response() {
    let server = ProxyServer::new(Some(temp_socket_path()));
    tokio::spawn(async move { server.start().await });
    
    let mut stream = UnixStream::connect(temp_socket_path()).await?;
    stream.write_all(b'{"jsonrpc":"2.0","id":1,"method":"server.status","params":{}}\n').await?;
    
    let mut response = String::new();
    let mut reader = BufReader::new(stream);
    reader.read_line(&mut response).await?;
    
    let parsed: JsonRpcResponse = serde_json::from_str(&response)?;
    assert_eq!(parsed.jsonrpc, "2.0");
    assert!(parsed.result.is_some());
}
```

## Common Issues & Solutions

### Issue: Socket file already exists

**Error:** `Address already in use`

**Cause:** Previous instance didn't clean up, or another process bound the socket.

**Solution:**

```bash
rm ~/.tairseach/tairseach.sock
```

**Prevention:** Server removes socket file on shutdown. If crash occurs, file remains.

### Issue: Permission denied connecting to socket

**Error:** `Permission denied`

**Cause:** Socket file has wrong permissions, or client UID != server UID.

**Solution:**

```bash
ls -la ~/.tairseach/tairseach.sock
# Should show: srw------- (0600)

# If wrong:
chmod 600 ~/.tairseach/tairseach.sock
```

### Issue: Connection rejected silently

**Symptom:** Client connects, immediately disconnects, no response.

**Cause:** UID check failed.

**Debug:**

```bash
# Check server logs
tail -f ~/.tairseach/logs/proxy.log | grep "Rejecting connection"
```

### Issue: Request hangs indefinitely

**Symptom:** Client sends request, never gets response.

**Possible causes:**
1. Handler is blocked (synchronous I/O on async task)
2. Deadlock in handler (awaiting lock that's never released)
3. Network partition (impossible for Unix socket)

**Debug:**

```bash
# Check if handler is running
ps aux | grep tairseach
# If CPU usage is 0%, handler is blocked

# Check Tokio metrics (if enabled)
curl http://localhost:9999/metrics
```

## Future Improvements

### Planned (v2 Roadmap)

1. **Connection limits** — Max 100 concurrent connections
2. **Payload size limits** — Max 10MB request size
3. **Connection timeouts** — 60s idle timeout
4. **Metrics export** — Prometheus endpoint for monitoring
5. **Request tracing** — Correlation ID for distributed tracing

### Considered (Not Prioritized)

1. **TLS support** — Encrypt socket traffic (low value for localhost)
2. **Multiple socket paths** — Different sockets for different purposes
3. **Hot reload** — Restart server without losing connections (complex, low value)
4. **Bidirectional communication** — Server pushes events to client (use Tauri events instead)

## Related Documentation

- **[handlers.md](handlers.md)** — Request dispatch and handler implementation
- **[router.md](router.md)** — Capability routing (v2 architecture)
- **[mcp-bridge.md](mcp-bridge.md)** — MCP protocol bridge that connects to this server
- **[auth-system.md](auth-system.md)** — Credential management for handlers

---

*The socket is the threshold — it guards the passage but remembers every crossing.*

🌬️ **Senchán Torpéist**
