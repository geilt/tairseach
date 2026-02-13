# Socket Protocol Reference

**JSON-RPC 2.0 over Unix Domain Socket**

Tairseach exposes a JSON-RPC 2.0 API over a Unix domain socket for local inter-process communication.

---

## Connection

**Socket Path:** `~/.tairseach/tairseach.sock`

**Protocol:** JSON-RPC 2.0 (newline-delimited messages)

**Transport:** Unix domain socket (SOCK_STREAM)

### Connection Example (Rust)

```rust
use tokio::net::UnixStream;
let socket = UnixStream::connect("~/.tairseach/tairseach.sock").await?;
```

---

## Message Format

### Request

```typescript
interface JsonRpcRequest {
  jsonrpc: "2.0";           // Protocol version (required)
  id?: string | number;     // Request ID (omit for notification)
  method: string;           // Method name (e.g., "contacts.list")
  params?: any;             // Method parameters (optional)
}
```

**Example:**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "contacts.list",
  "params": {
    "limit": 50,
    "offset": 0
  }
}
```

### Response (Success)

```typescript
interface JsonRpcResponse {
  jsonrpc: "2.0";
  id: string | number;      // Matches request ID
  result: any;              // Method result
}
```

**Example:**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "contacts": [...],
    "count": 42,
    "limit": 50,
    "offset": 0
  }
}
```

### Response (Error)

```typescript
interface JsonRpcErrorResponse {
  jsonrpc: "2.0";
  id: string | number | null;
  error: {
    code: number;           // Error code (see below)
    message: string;        // Human-readable error
    data?: any;             // Additional error data
  }
}
```

**Example:**

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "error": {
    "code": -32001,
    "message": "Permission not granted",
    "data": {
      "permission": "contacts",
      "status": "not_determined"
    }
  }
}
```

---

## Error Codes

### Standard JSON-RPC Errors

| Code | Name | Description |
|------|------|-------------|
| -32700 | Parse error | Invalid JSON received by server |
| -32600 | Invalid Request | JSON-RPC structure is invalid |
| -32601 | Method not found | Method does not exist or is not available |
| -32602 | Invalid params | Invalid method parameters |
| -32603 | Internal error | Internal JSON-RPC error |

### Tairseach-Specific Errors

| Code | Name | Description |
|------|------|-------------|
| -32001 | Permission denied | Required macOS permission not granted |
| -32000 | Handler error | Generic handler error (see `data` field) |
| -32002 | Not found | Requested resource not found |

### Auth Subsystem Errors

| Code | Name | Description |
|------|------|-------------|
| 4001 | Master key not initialized | Auth broker not initialized |
| 4002 | Unknown provider | Provider type not supported |
| 4003 | Account not found | No account exists for provider/account ID |
| 4004 | Refresh failed | Token refresh attempt failed |
| 4010 | Credential not found | Credential not found in store |
| 4011 | Credential validation failed | Credential failed schema validation |

---

## Method Naming Convention

Methods follow the pattern: `<namespace>.<action>`

**Namespaces:**
- `auth` — OAuth token broker, credentials
- `permissions` — macOS permission checks
- `contacts` — Native Contacts.app access
- `calendar` — Native Calendar.app access
- `reminders` — Native Reminders.app access
- `location` — Core Location access
- `screen` — Screen recording
- `files` — File system access
- `automation` — AppleScript/Automator
- `config` — Server configuration
- `gmail` — Gmail API (via OAuth)
- `gcalendar` — Google Calendar API
- `op` / `onepassword` — 1Password integration
- `oura` — Oura Ring API
- `jira` — Jira Cloud API
- `server` — Server status/control

**Examples:**
- `contacts.list`
- `auth.token`
- `permissions.check`
- `server.status`

---

## Batch Requests

JSON-RPC 2.0 supports sending multiple requests in a single message:

**Request:**

```json
[
  {"jsonrpc":"2.0","id":1,"method":"auth.status","params":{}},
  {"jsonrpc":"2.0","id":2,"method":"permissions.list","params":{}}
]
```

**Response:**

```json
[
  {"jsonrpc":"2.0","id":1,"result":{"initialized":true}},
  {"jsonrpc":"2.0","id":2,"result":{"permissions":[...]}}
]
```

---

## Notifications (No Response)

Requests without an `id` field are notifications — the server will not send a response.

**Example:**

```json
{"jsonrpc":"2.0","method":"log.info","params":{"message":"Starting sync"}}
```

---

## Permission-Gated Methods

Methods requiring macOS permissions are automatically checked by the `HandlerRegistry` middleware. If permission is not granted, the server returns error `-32001` (Permission denied).

**Gated methods:**
- `contacts.*` → requires `contacts` permission
- `calendar.*` → requires `calendar` permission
- `reminders.*` → requires `reminders` permission
- `location.*` → requires `location` permission
- `screen.*` → requires `screen_recording` permission
- `automation.*` → requires `automation` or `accessibility` permission
- `files.*` → requires `full_disk_access` permission

**Ungated methods:**
- `auth.*` — socket security is sufficient
- `permissions.*` — checking permissions doesn't require permissions
- `config.*` — configuration access
- `server.*` — server control

---

## Connection Lifecycle

1. **Connect** to `~/.tairseach/tairseach.sock`
2. **Send** newline-delimited JSON-RPC requests
3. **Receive** newline-delimited JSON-RPC responses
4. **Close** connection when done (or keep alive for multiple requests)

**Keep-Alive:**  
The socket supports persistent connections. You can send multiple requests over the same connection.

---

## Source Files

| File | Purpose |
|------|---------|
| `src-tauri/src/proxy/protocol.rs` | JSON-RPC types and parsing |
| `src-tauri/src/proxy/server.rs` | Socket server implementation |
| `src-tauri/src/proxy/handlers/mod.rs` | Handler registry and permission middleware |

---

## Testing

Use `websocat` or `nc` to test the socket manually:

```bash
# Connect and send a request
echo '{"jsonrpc":"2.0","id":1,"method":"server.status","params":{}}' \
  | websocat unix:~/.tairseach/tairseach.sock
```

**Expected response:**

```json
{"jsonrpc":"2.0","id":1,"result":{"status":"running","version":"0.1.0"}}
```

---

*Source: `src-tauri/src/proxy/protocol.rs`*
