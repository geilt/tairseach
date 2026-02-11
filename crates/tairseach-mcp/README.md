# Tairseach MCP Bridge

A Model Context Protocol (MCP) server that exposes Tairseach's local macOS capabilities to MCP clients like Claude Desktop, OpenClaw, and other MCP-compatible tools.

## Overview

The MCP bridge acts as a protocol translator between MCP clients and the Tairseach Unix socket server. It:

1. **Discovers tools** by reading manifest files from `~/.tairseach/manifests/`
2. **Exposes tools** via the MCP protocol over stdio
3. **Routes calls** through the Tairseach Unix socket at `~/.tairseach/tairseach.sock`
4. **Returns results** in MCP-compliant JSON-RPC format

## Architecture

```
MCP Client (Claude, OpenClaw, etc.)
           ↓ stdio (JSON-RPC)
    tairseach-mcp bridge
           ↓ Unix socket (JSON-RPC)
    tairseach (Tauri app)
           ↓ native APIs
    macOS system services
```

## Building

```bash
# Build MCP bridge only
cargo build --release --package tairseach-mcp

# Build all workspace members (including main app)
cargo build --release

# Binary location
./target/release/tairseach-mcp
```

## Usage

The bridge communicates via stdio using JSON-RPC 2.0:

```bash
./target/release/tairseach-mcp
```

### Example Session

```json
// Initialize
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}

// List tools
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}

// Call a tool
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"tairseach_server_status","arguments":{}}}
```

## Tool Naming

Tools from manifests are exposed with the `tairseach_` prefix:

- Manifest tool: `server_status`
- MCP tool name: `tairseach_server_status`
- Socket method: `server.status` (from manifest's `implementation.methods`)

## Manifest Structure

The bridge reads JSON manifests from `~/.tairseach/manifests/` (recursively):

```json
{
  "id": "server",
  "tools": [
    {
      "name": "server_status",
      "description": "Get server status and version.",
      "inputSchema": { "type": "object", "properties": {} },
      "annotations": { "readOnlyHint": true }
    }
  ],
  "implementation": {
    "type": "internal",
    "methods": {
      "server_status": "server.status"
    }
  }
}
```

Tools are **exposed by default** unless `mcpExpose: false` is set.

## Configuration

The bridge requires:

1. **Tairseach running** — the main app must be active with the socket server listening
2. **Manifests deployed** — tool definitions at `~/.tairseach/manifests/`
3. **Transport mode** — currently only `stdio` is supported (default)

## Testing

### Quick Test

```bash
cd /Users/geilt/environment/tairseach

# Test initialize
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' | ./target/release/tairseach-mcp

# Test tool call (requires Tairseach running)
(./target/release/tairseach-mcp <<EOF
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"tairseach_server_status","arguments":{}}}
EOF
)
```

### With MCP Inspector

```bash
npx @modelcontextprotocol/inspector ./target/release/tairseach-mcp
```

## Integration

### Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "tairseach": {
      "command": "/Users/geilt/environment/tairseach/target/release/tairseach-mcp"
    }
  }
}
```

### OpenClaw

Add to OpenClaw MCP server configuration:

```yaml
name: tairseach
command: /Users/geilt/environment/tairseach/target/release/tairseach-mcp
transport: stdio
```

## Current Capabilities

As of v0.2.0, the bridge exposes **35 tools** across:

- **Auth** — OAuth token management
- **Calendar** — EventKit integration
- **Contacts** — Address book access
- **Files** — Sandboxed file operations
- **Location** — CoreLocation services
- **Permissions** — TCC permission management
- **Reminders** — Task/reminder management
- **Screen** — Screenshot capture, window listing
- **Server** — Status and control

## Protocol Version

MCP Protocol: `2025-03-26`

## Dependencies

- `tairseach-protocol` — shared JSON-RPC types and socket client
- `tokio` — async runtime
- `serde/serde_json` — serialization
- `dirs` — home directory resolution

## Error Handling

- **Socket errors** → `-32000` (upstream error)
- **Unknown tools** → `-32601` (method not found)
- **Parse errors** → `-32700` (parse error)
- **Invalid params** → `-32602` (invalid params)

## Limitations

- Transport: stdio only (no SSE, no HTTP)
- Resources/prompts: not yet implemented
- Notifications: only `initialized` is handled
- Tool discovery: manifest reload requires bridge restart

---

*Built for Tairseach v0.2.0*
