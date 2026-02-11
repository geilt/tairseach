# Tairseach Testing Guide

## Prerequisites

1. **Tairseach app running**
   ```bash
   # Check if running
   ps aux | grep -i tairseach | grep -v grep
   
   # Start if needed
   open -a Tairseach
   ```

2. **Socket exists**
   ```bash
   ls -la ~/.tairseach/tairseach.sock
   ```

3. **Manifests deployed**
   ```bash
   find ~/.tairseach/manifests -name "*.json" | wc -l
   # Should show 11+ files
   ```

## Build Both Binaries

```bash
cd /Users/geilt/environment/tairseach
cargo build --release
```

Expected output:
- `target/release/tairseach` (main Tauri app binary)
- `target/release/tairseach-mcp` (MCP bridge binary)

## Test MCP Bridge

### 1. Initialize Handshake

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' | ./target/release/tairseach-mcp
```

Expected response:
```json
{
  "jsonrpc":"2.0",
  "id":1,
  "result":{
    "protocolVersion":"2025-03-26",
    "capabilities":{"tools":{"listChanged":true},"resources":{"subscribe":false,"listChanged":false}},
    "serverInfo":{"name":"tairseach-mcp","version":"0.2.0"},
    "instructions":"Tairseach provides local macOS capability tools..."
  }
}
```

### 2. Tool Discovery

```bash
(./target/release/tairseach-mcp <<EOF
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
EOF
) | tail -1 | jq '.result.tools | length'
```

Expected: `35` (number of exposed tools)

### 3. End-to-End Tool Call

```bash
(./target/release/tairseach-mcp <<EOF
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"tairseach_server_status","arguments":{}}}
EOF
) | tail -1 | jq '.result'
```

Expected response:
```json
{
  "content": [
    {
      "type": "text",
      "text": "{\"status\":\"running\",\"version\":\"0.1.0\"}"
    }
  ],
  "isError": false
}
```

### 4. Test with MCP Inspector

```bash
npx @modelcontextprotocol/inspector ./target/release/tairseach-mcp
```

Then:
1. Open browser to inspector URL
2. Click "Initialize" in the UI
3. Browse available tools
4. Test tool calls interactively

## Test Direct Socket Communication

```bash
# Test socket directly (bypassing MCP bridge)
echo '{"jsonrpc":"2.0","id":1,"method":"server.status","params":{}}' | nc -U ~/.tairseach/tairseach.sock
```

Expected:
```json
{"jsonrpc":"2.0","id":1,"result":{"status":"running","version":"0.1.0"}}
```

## Verify Binary Builds

```bash
# Check both binaries exist
ls -lh target/release/tairseach
ls -lh target/release/tairseach-mcp

# Run help to verify execution
./target/release/tairseach-mcp --help
```

## Troubleshooting

### "Connection refused (os error 61)"

**Cause:** Tairseach app not running or socket not listening

**Fix:**
```bash
# Start Tairseach
open -a Tairseach

# Wait 1-2 seconds for socket to initialize
sleep 2

# Verify socket exists
test -S ~/.tairseach/tairseach.sock && echo "Socket ready" || echo "Socket missing"
```

### "Method not found: server_status"

**Cause:** Method name mismatch between MCP bridge and socket server

**Expected behavior:** MCP bridge should translate `tairseach_server_status` → `server.status` using manifest mappings

**Debug:**
```bash
# Check manifest has correct mapping
cat ~/.tairseach/manifests/core/server.json | jq '.implementation.methods'
```

### "No tools loaded"

**Cause:** Manifests not found at `~/.tairseach/manifests/`

**Fix:**
```bash
# Check manifest directory exists
ls ~/.tairseach/manifests/

# Redeploy manifests if needed (implementation-specific)
```

## Test Coverage

✅ MCP initialize handshake
✅ Tool discovery from manifests (35 tools)
✅ Tool name prefixing (`tairseach_*`)
✅ Method name translation (manifest `implementation.methods`)
✅ Socket connection and communication
✅ End-to-end tool call with result
✅ Error handling (connection refused, method not found)
✅ Both binaries build in release mode

## Performance

Release builds use:
- LTO (Link-Time Optimization)
- `opt-level = "z"` (size optimization)
- `codegen-units = 1`
- Symbol stripping

Build times (M1 Mac):
- `tairseach-mcp`: ~4-5 seconds
- `tairseach`: ~50-60 seconds (Tauri + all dependencies)

---

*Updated for v0.2.0*
