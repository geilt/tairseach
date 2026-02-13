# MCP Tools Reference

**All MCP-exposed tools from Tairseach manifests.**

Tools exposed via the Model Context Protocol (MCP) for use in OpenClaw, Claude Desktop, and other MCP clients.

---

## Overview

**MCP Protocol Version:** `2025-03-26`

**Tool Naming:** Manifest tools are prefixed with `tairseach_` when exposed to MCP:
- Manifest tool: `auth_status`
- MCP tool name: `tairseach_auth_status`

**Total Tools:** Varies based on loaded manifests (typically 50-70 tools)

**Filtering:** Tools with `"mcp_expose": false` are NOT exposed to MCP clients.

---

## Tool Discovery

MCP clients discover tools via the `tools/list` request:

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/list"
}
```

**Response:**
```json
{
  "tools": [
    {
      "name": "tairseach_auth_status",
      "description": "Get auth subsystem status.",
      "inputSchema": { ... },
      "annotations": {
        "readOnlyHint": true
      }
    }
  ],
  "next_cursor": null
}
```

---

## Tool Invocation

MCP clients invoke tools via `tools/call`:

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "tairseach_contacts_list",
    "arguments": {
      "limit": 10
    }
  }
}
```

**Response:**
```json
{
  "content": [
    {
      "type": "text",
      "text": "{\"contacts\":[...],\"count\":10}"
    }
  ],
  "isError": false
}
```

---

## Tool Categories

Tools are organized by namespace (prefix before `_`):

### server

- `tairseach_server_status` — Get server status and version

### auth

- `tairseach_auth_status` — Get auth subsystem status
- `tairseach_auth_providers` — List supported providers
- `tairseach_auth_accounts` — List authorized accounts
- `tairseach_auth_refresh` — Force refresh token
- `tairseach_auth_revoke` — Revoke account token
- `tairseach_auth_store` — Store/import token record

**Not exposed:**
- `auth_token` — Too sensitive (raw access token)
- `auth_gog_passphrase` — Too sensitive

### permissions

- `tairseach_permissions_check` — Check permission status
- `tairseach_permissions_list` — List all permissions
- `tairseach_permissions_request` — Request permission

### contacts

- `tairseach_contacts_list` — List all contacts
- `tairseach_contacts_search` — Search contacts by name
- `tairseach_contacts_get` — Get specific contact
- `tairseach_contacts_create` — Create new contact
- `tairseach_contacts_update` — Update existing contact
- `tairseach_contacts_delete` — Delete contact

### calendar

- `tairseach_calendar_list` — List calendars
- `tairseach_calendar_events` — List events in date range
- `tairseach_calendar_get_event` — Get specific event
- `tairseach_calendar_create_event` — Create new event
- `tairseach_calendar_update_event` — Update event
- `tairseach_calendar_delete_event` — Delete event

### reminders

- `tairseach_reminders_lists` — List reminder lists
- `tairseach_reminders_list` — List reminders
- `tairseach_reminders_create` — Create reminder
- `tairseach_reminders_complete` — Mark reminder completed
- `tairseach_reminders_delete` — Delete reminder

### location

- `tairseach_location_get` — Get current location

### screen

- `tairseach_screen_capture` — Capture screenshot
- `tairseach_screen_windows` — List visible windows

### files

- `tairseach_files_read` — Read file contents
- `tairseach_files_write` — Write file
- `tairseach_files_list` — List directory contents

### automation

- `tairseach_automation_run` — Run AppleScript/Automator
- `tairseach_automation_click` — Simulate mouse click
- `tairseach_automation_type` — Simulate keyboard typing

### config

- `tairseach_config_get` — Get configuration value
- `tairseach_config_set` — Set configuration value

### gmail

- `tairseach_gmail_list_messages` — List Gmail messages
- `tairseach_gmail_get_message` — Get specific message
- `tairseach_gmail_send` — Send email
- `tairseach_gmail_list_labels` — List labels
- `tairseach_gmail_modify_message` — Modify message labels
- `tairseach_gmail_trash_message` — Move to trash
- `tairseach_gmail_delete_message` — Permanently delete

### gcalendar

- `tairseach_gcalendar_list` — List Google Calendars
- `tairseach_gcalendar_events` — List events
- `tairseach_gcalendar_create_event` — Create event

### onepassword / op

- `tairseach_op_list_vaults` — List vaults
- `tairseach_op_list_items` — List items in vault
- `tairseach_op_get_item` — Get specific item

### oura

- `tairseach_oura_daily_sleep` — Get sleep data
- `tairseach_oura_daily_activity` — Get activity data
- `tairseach_oura_daily_readiness` — Get readiness scores

### jira

- `tairseach_jira_search` — Search issues
- `tairseach_jira_get_issue` — Get specific issue
- `tairseach_jira_create_issue` — Create new issue
- `tairseach_jira_update_issue` — Update issue
- `tairseach_jira_transition_issue` — Transition issue status

---

## Tool Annotations

Tools may include hint annotations for MCP clients:

| Annotation | Meaning |
|------------|---------|
| `readOnlyHint: true` | Tool only reads data (safe) |
| `destructiveHint: true` | Tool modifies or deletes data (requires caution) |
| `idempotentHint: true` | Safe to retry (same result) |
| `openWorldHint: true` | May access external resources (network, APIs) |

**Example:**

```json
{
  "name": "tairseach_contacts_delete",
  "description": "Delete a contact.",
  "inputSchema": { ... },
  "annotations": {
    "destructiveHint": true
  }
}
```

**Use in Claude Desktop:**  
Claude may ask for confirmation before using tools with `destructiveHint: true`.

---

## Input Schemas

All tools include JSON Schema definitions for their parameters.

**Example:**

```json
{
  "name": "tairseach_contacts_search",
  "description": "Search contacts by name",
  "inputSchema": {
    "type": "object",
    "required": ["query"],
    "properties": {
      "query": {
        "type": "string",
        "description": "Search query (matches first/last name)"
      },
      "limit": {
        "type": "integer",
        "default": 50,
        "description": "Maximum results to return"
      }
    },
    "additionalProperties": false
  }
}
```

**Validation:**  
MCP server validates input against the schema before calling the tool.

---

## Tool Loading

Tools are loaded from manifest files in `~/.tairseach/manifests/`:

```
~/.tairseach/manifests/
├── core/
│   ├── auth.json
│   ├── contacts.json
│   ├── calendar.json
│   └── ...
└── integrations/
    ├── gmail.json
    ├── jira.json
    └── ...
```

**Hot-Reload:**  
Manifest changes are automatically detected and reloaded. New tools appear without restarting the MCP server.

---

## Tool-to-Method Mapping

Each MCP tool maps to a socket JSON-RPC method via the manifest's `implementation.methods`:

**Manifest excerpt:**

```json
{
  "tools": [
    { "name": "contacts_list", "description": "..." }
  ],
  "implementation": {
    "type": "internal",
    "methods": {
      "contacts_list": "contacts.list"
    }
  }
}
```

**Flow:**

1. MCP client calls `tairseach_contacts_list`
2. MCP server maps to socket method `contacts.list`
3. Socket server handles `contacts.list` via internal handler
4. Result returned to MCP client

---

## Error Handling

**Socket Errors:**

If the socket call returns an error, the MCP response includes `isError: true`:

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\"error\":{\"code\":-32001,\"message\":\"Permission not granted\",\"data\":{\"permission\":\"contacts\",\"status\":\"not_determined\"}}}"
    }
  ],
  "isError": true
}
```

**Unknown Tool:**

If the tool name is not in the allowlist, MCP server returns an error:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32601,
    "message": "Unknown tool: tairseach_invalid_tool"
  }
}
```

---

## MCP Server Configuration

**Server Executable:** `tairseach-mcp` (crate: `crates/tairseach-mcp`)

**Stdio Transport:** MCP server reads requests from stdin, writes responses to stdout (line-delimited JSON).

**Socket Connection:** MCP server connects to `~/.tairseach/tairseach.sock` to invoke tools.

**OpenClaw Configuration:**

```json
{
  "mcpServers": {
    "tairseach": {
      "command": "/path/to/tairseach-mcp",
      "args": [],
      "env": {}
    }
  }
}
```

---

## Tool Count by Category

Approximate tool counts (varies by installed manifests):

| Category | Tools |
|----------|-------|
| server | 1 |
| auth | 6 |
| permissions | 3 |
| contacts | 6 |
| calendar | 6 |
| reminders | 5 |
| location | 1 |
| screen | 2 |
| files | 3 |
| automation | 3 |
| config | 2 |
| gmail | 7 |
| gcalendar | 3 |
| onepassword | 3 |
| oura | 3 |
| jira | 5 |

**Total:** ~60 tools (excluding `mcp_expose: false` tools)

---

## Adding New Tools

1. **Create or edit manifest** in `~/.tairseach/manifests/`
2. **Add tool definition** with `name`, `description`, `inputSchema`, `outputSchema`
3. **Add method mapping** in `implementation.methods`
4. **Optional:** Set `mcp_expose: false` to hide from MCP

**Example:**

```json
{
  "tools": [
    {
      "name": "my_custom_tool",
      "description": "Does something useful",
      "inputSchema": { "type": "object", "properties": {} },
      "outputSchema": { "type": "object" }
    }
  ],
  "implementation": {
    "type": "internal",
    "methods": {
      "my_custom_tool": "custom.method"
    }
  }
}
```

**Hot-reload:** Tool appears automatically within seconds.

---

## Source Files

| File | Purpose |
|------|---------|
| `crates/tairseach-mcp/src/main.rs` | MCP server entry point |
| `crates/tairseach-mcp/src/tools.rs` | Tool registry and allowlist |
| `crates/tairseach-mcp/src/protocol.rs` | MCP protocol implementation |
| `~/.tairseach/manifests/` | Tool definitions |

---

*Generated: 2025-02-13*  
*60+ MCP tools documented*
