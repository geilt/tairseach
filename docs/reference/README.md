# Tairseach API Reference

**Human-readable reference documentation for Tairseach integration points.**

This directory contains exhaustive reference material for developers building with or extending Tairseach.

## Documentation Index

| Document | Purpose |
|----------|---------|
| [socket-protocol.md](./socket-protocol.md) | JSON-RPC 2.0 protocol specification, message format, error codes |
| [handler-reference.md](./handler-reference.md) | Complete handler method catalog with params, responses, examples |
| [manifest-schema.md](./manifest-schema.md) | Manifest format (v1.0.0), all fields, validation rules |
| [credential-types.md](./credential-types.md) | Built-in credential schemas, fields, how to add custom types |
| [permission-types.md](./permission-types.md) | All 11 TCC permissions, macOS identifiers, behavior |
| [tauri-commands.md](./tauri-commands.md) | All `#[tauri::command]` functions callable from the frontend |
| [mcp-tools.md](./mcp-tools.md) | MCP-exposed tools, parameters, responses |
| [config-reference.md](./config-reference.md) | Configuration files, formats, defaults |
| [environment.md](./environment.md) | Environment variables, paths, socket location |

## Quick Start

### Socket Connection

```javascript
// Connect to Tairseach socket (default: ~/.tairseach/tairseach.sock)
const socket = await connectToSocket();

// Send JSON-RPC request
const response = await socket.call({
  jsonrpc: "2.0",
  id: 1,
  method: "contacts.list",
  params: { limit: 10 }
});
```

### Common Patterns

**List contacts:**
```json
{"jsonrpc":"2.0","id":1,"method":"contacts.list","params":{"limit":50,"offset":0}}
```

**Check permission:**
```json
{"jsonrpc":"2.0","id":2,"method":"permissions.check","params":{"permission":"contacts"}}
```

**Get OAuth token:**
```json
{"jsonrpc":"2.0","id":3,"method":"auth.token","params":{"provider":"google","account":"user@example.com"}}
```

## Error Codes

| Code | Name | Meaning |
|------|------|---------|
| -32700 | Parse error | Invalid JSON received |
| -32600 | Invalid request | JSON-RPC structure invalid |
| -32601 | Method not found | Method does not exist |
| -32602 | Invalid params | Parameters missing or wrong type |
| -32603 | Internal error | Server-side error |
| -32001 | Permission denied | macOS permission not granted |
| -32000 | Handler error | Handler-specific error (see `data`) |
| -32002 | Not found | Resource not found |

## Version

- **Manifest Version:** `1.0.0`
- **JSON-RPC Version:** `2.0`
- **MCP Protocol:** `2025-03-26`

## Architecture

```
┌─────────────────┐
│  MCP Client     │ ← MCP Protocol (stdio/SSE)
│  (OpenClaw)     │
└────────┬────────┘
         │
         ↓
┌─────────────────┐
│ tairseach-mcp   │ ← Translates MCP → Socket JSON-RPC
│   (server)      │
└────────┬────────┘
         │
         ↓ (Unix socket)
┌─────────────────┐
│ Tairseach Proxy │ ← JSON-RPC 2.0 Socket Server
│   (Tauri App)   │   - Manifest Registry
└────────┬────────┘   - Capability Router
         │            - Permission Checks
         ↓            - Auth Broker
┌─────────────────┐
│ macOS Frameworks│ ← CNContacts, EKEventKit, etc.
└─────────────────┘
```

---

*Generated: 2025-02-13*  
*Source: Lorgaire (tracker dalta of Senchán)*
