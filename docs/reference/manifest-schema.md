# Manifest Schema Reference

**Complete specification for Tairseach capability manifests (v1.0.0).**

Manifests define capabilities (tools/methods) and their implementation bindings.

---

## Overview

**Manifest Version:** `1.0.0`

**Storage Location:** `~/.tairseach/manifests/`

**Format:** JSON files organized by category:
- `core/` — Built-in Tairseach capabilities
- `integrations/` — Third-party integrations

**Hot-Reload:** Manifests are watched for changes and automatically reloaded.

---

## Root Schema

```typescript
interface Manifest {
  manifest_version: string;          // Must be "1.0.0"
  id: string;                        // Unique manifest identifier
  name: string;                      // Human-readable name
  description: string;               // Purpose description
  version: string;                   // Semantic version (e.g., "0.1.0")
  category: string;                  // Category (e.g., "security", "productivity")
  requires?: Requirements;           // Credentials and permissions
  tools: Tool[];                     // Tool definitions (at least one required)
  implementation: Implementation;    // How tools are implemented
  compatibility?: {
    mcpProtocol?: string;            // MCP protocol version (e.g., "2025-03-26")
    os?: string[];                   // Operating systems (e.g., ["macos"])
  }
}
```

**Required fields:**
- `manifest_version`
- `id`
- `name`
- `description`
- `version`
- `category`
- `tools` (array must have at least one tool)
- `implementation`

---

## Requirements

Declare credentials and permissions needed by the manifest.

```typescript
interface Requirements {
  credentials?: CredentialRequirement[];
  permissions?: PermissionRequirement[];
}
```

### CredentialRequirement

```typescript
interface CredentialRequirement {
  id: string;                        // Credential identifier
  provider?: string;                 // Provider type (e.g., "google", "onepassword")
  kind?: string;                     // Credential kind
  scopes?: string[];                 // OAuth scopes (for OAuth providers)
  optional?: boolean;                // Default: false
}
```

**Example:**

```json
{
  "credentials": [
    {
      "id": "google_oauth",
      "provider": "google",
      "scopes": [
        "https://www.googleapis.com/auth/gmail.modify",
        "https://www.googleapis.com/auth/gmail.settings.basic"
      ],
      "optional": false
    }
  ]
}
```

### PermissionRequirement

```typescript
interface PermissionRequirement {
  name: string;                      // Permission name (e.g., "contacts", "calendar")
  optional?: boolean;                // Default: false
  reason?: string;                   // Why this permission is needed
}
```

**Example:**

```json
{
  "permissions": [
    {
      "name": "contacts",
      "optional": false,
      "reason": "Access native Contacts.app to read and modify contacts"
    }
  ]
}
```

**Valid permission names:**
- `contacts`
- `calendar`
- `reminders`
- `location`
- `photos`
- `camera`
- `microphone`
- `screen_recording`
- `accessibility`
- `full_disk_access`
- `automation`

---

## Tool

Define a callable tool/method.

```typescript
interface Tool {
  name: string;                      // Tool identifier (must be valid identifier)
  title?: string;                    // Display title
  description: string;               // What the tool does
  inputSchema: JSONSchema;           // JSON Schema for input parameters
  outputSchema: JSONSchema;          // JSON Schema for output
  annotations?: {
    readOnlyHint?: boolean;          // Tool only reads data
    destructiveHint?: boolean;       // Tool modifies/deletes data
    idempotentHint?: boolean;        // Safe to retry
    openWorldHint?: boolean;         // May access external resources
  };
  requires?: Requirements;           // Tool-specific requirements (override manifest-level)
  mcp_expose?: boolean;              // Expose via MCP (default: true)
}
```

**Tool name validation:**
- Must start with a letter
- Can contain letters, digits, underscores
- No spaces or special characters

**Example:**

```json
{
  "name": "server_status",
  "description": "Get server status and version.",
  "inputSchema": {
    "type": "object",
    "properties": {},
    "additionalProperties": false
  },
  "outputSchema": {
    "type": "object",
    "required": ["status", "version"],
    "properties": {
      "status": {"type": "string"},
      "version": {"type": "string"}
    }
  },
  "annotations": {
    "readOnlyHint": true
  }
}
```

### Input/Output Schemas

Use JSON Schema (draft-07 compatible) to define tool inputs and outputs.

**Common patterns:**

**No parameters:**
```json
{
  "type": "object",
  "properties": {},
  "additionalProperties": false
}
```

**Required string parameter:**
```json
{
  "type": "object",
  "required": ["query"],
  "properties": {
    "query": {"type": "string"}
  },
  "additionalProperties": false
}
```

**Optional parameters with defaults:**
```json
{
  "type": "object",
  "properties": {
    "limit": {"type": "integer", "default": 50},
    "offset": {"type": "integer", "default": 0}
  },
  "additionalProperties": false
}
```

---

## Implementation

Define how tools are implemented. Three types supported:

### Internal Implementation

Tools are implemented by internal Rust handlers.

```typescript
interface InternalImplementation {
  type: "internal";
  module: string;                    // Handler module path
  methods: Record<string, string>;   // Map tool names to method names
}
```

**Example:**

```json
{
  "type": "internal",
  "module": "proxy.handlers.auth",
  "methods": {
    "auth_status": "auth.status",
    "auth_providers": "auth.providers",
    "auth_accounts": "auth.accounts"
  }
}
```

**Method name format:** `<namespace>.<action>`

### Script Implementation

Tools are implemented by external scripts (Python, Node.js, etc.).

```typescript
interface ScriptImplementation {
  type: "script";
  runtime: string;                   // e.g., "python3", "node"
  entrypoint: string;                // Script file path
  args?: string[];                   // Additional arguments
  env?: Record<string, string>;      // Environment variables
  toolBindings: Record<string, ScriptToolBinding>;
}

interface ScriptToolBinding {
  action: string;                    // Script action to invoke
  input_mode?: string;               // How to pass input: "stdin", "args", "file"
  output_mode?: string;              // How to read output: "stdout", "file"
}
```

**Example:**

```json
{
  "type": "script",
  "runtime": "python3",
  "entrypoint": "/path/to/script.py",
  "args": ["--config", "/path/to/config.json"],
  "env": {
    "PYTHONPATH": "/custom/path"
  },
  "toolBindings": {
    "my_tool": {
      "action": "do_thing",
      "input_mode": "stdin",
      "output_mode": "stdout"
    }
  }
}
```

### Proxy Implementation

Tools are proxied to an external HTTP API.

```typescript
interface ProxyImplementation {
  type: "proxy";
  baseUrl: string;                   // Base API URL
  auth: ProxyAuth;                   // Authentication strategy
  toolBindings: Record<string, ProxyToolBinding>;
}

interface ProxyAuth {
  strategy: string;                  // "bearer", "apikey", "basic", etc.
  credentialId: string;              // Credential to use
  headerName?: string;               // HTTP header name
  queryParam?: string;               // Query parameter name
  tokenField?: string;               // Field in credential containing token
}

interface ProxyToolBinding {
  method: string;                    // HTTP method: GET, POST, PUT, DELETE, PATCH
  path: string;                      // API path (can include {placeholders})
  query?: Record<string, string>;    // Static query parameters
  bodyTemplate?: any;                // JSON template for request body
  headers?: Record<string, string>;  // Additional headers
  responsePath?: string;             // JSONPath to extract result
}
```

**Example:**

```json
{
  "type": "proxy",
  "baseUrl": "https://api.example.com",
  "auth": {
    "strategy": "bearer",
    "credentialId": "my_api",
    "headerName": "Authorization",
    "tokenField": "api_key"
  },
  "toolBindings": {
    "fetch_user": {
      "method": "GET",
      "path": "/users/{userId}",
      "headers": {
        "Accept": "application/json"
      },
      "responsePath": "$.data"
    },
    "create_user": {
      "method": "POST",
      "path": "/users",
      "bodyTemplate": {
        "name": "{name}",
        "email": "{email}"
      },
      "responsePath": "$.user"
    }
  }
}
```

**Placeholder substitution:**  
Path and body templates support `{parameterName}` placeholders that are replaced with input values.

---

## Validation Rules

Manifests are validated when loaded:

1. **manifest_version** must be `"1.0.0"`
2. **id** must not be empty
3. **tools** array must contain at least one tool
4. All tool **names** must be valid identifiers
5. Every tool must have a corresponding **binding** in implementation
6. **Internal** implementations: all tools must have method mappings
7. **Script** implementations: all tools must have toolBindings
8. **Proxy** implementations: all tools must have toolBindings

**Validation errors** return detailed messages indicating the problem.

---

## Example: Complete Manifest

```json
{
  "manifest_version": "1.0.0",
  "id": "auth",
  "name": "Auth",
  "description": "OAuth token broker operations.",
  "version": "0.1.0",
  "category": "security",
  "tools": [
    {
      "name": "auth_status",
      "description": "Get auth subsystem status.",
      "inputSchema": {
        "type": "object",
        "properties": {},
        "additionalProperties": false
      },
      "outputSchema": {
        "type": "object"
      },
      "annotations": {
        "readOnlyHint": true
      }
    },
    {
      "name": "auth_providers",
      "description": "List supported providers.",
      "inputSchema": {
        "type": "object",
        "properties": {},
        "additionalProperties": false
      },
      "outputSchema": {
        "type": "object",
        "required": ["providers"],
        "properties": {
          "providers": {"type": "array"}
        },
        "additionalProperties": true
      },
      "annotations": {
        "readOnlyHint": true
      }
    },
    {
      "name": "auth_token",
      "description": "Get access token (sensitive).",
      "mcp_expose": false,
      "inputSchema": {
        "type": "object",
        "required": ["provider", "account"],
        "properties": {
          "provider": {"type": "string"},
          "account": {"type": "string"},
          "scopes": {
            "type": "array",
            "items": {"type": "string"}
          }
        },
        "additionalProperties": false
      },
      "outputSchema": {
        "type": "object"
      },
      "annotations": {
        "readOnlyHint": true
      }
    }
  ],
  "implementation": {
    "type": "internal",
    "module": "proxy.handlers.auth",
    "methods": {
      "auth_status": "auth.status",
      "auth_providers": "auth.providers",
      "auth_token": "auth.token"
    }
  },
  "compatibility": {
    "mcpProtocol": "2025-03-26",
    "os": ["macos"]
  }
}
```

---

## MCP Exposure

Tools with `"mcp_expose": false` are **not** exposed to MCP clients (OpenClaw, Claude Desktop, etc.). Use this for:

- Sensitive operations (e.g., raw token access)
- Internal-only methods
- Deprecated tools

**Default:** `mcp_expose: true`

**MCP tool naming:** Tools exposed to MCP are prefixed with `tairseach_`:
- Manifest tool: `auth_status`
- MCP tool name: `tairseach_auth_status`

---

## File Organization

```
~/.tairseach/manifests/
├── core/
│   ├── server.json         # Server control
│   ├── auth.json           # Auth broker
│   ├── permissions.json    # Permission management
│   ├── contacts.json       # Native contacts
│   ├── calendar.json       # Native calendar
│   ├── reminders.json      # Native reminders
│   ├── location.json       # Location services
│   ├── screen.json         # Screen recording
│   ├── files.json          # File system
│   ├── automation.json     # AppleScript/Automator
│   └── config.json         # Configuration
└── integrations/
    ├── gmail.json          # Gmail API
    ├── gcalendar.json      # Google Calendar API
    ├── onepassword.json    # 1Password
    ├── oura.json           # Oura Ring
    └── jira.json           # Jira Cloud
```

**Manifests are discovered** by recursively scanning these directories for `*.json` files.

---

## Source Files

- `src-tauri/src/manifest/types.rs` — Rust type definitions
- `src-tauri/src/manifest/mod.rs` — Manifest registry and loader
- `src-tauri/src/router/mod.rs` — Capability routing based on manifests

---

*Generated: 2025-02-13*  
*Schema version: 1.0.0*
