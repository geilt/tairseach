# Manifest Pattern

> **Copy-paste template for creating a new capability manifest**

---

## File Location

```
~/.tairseach/manifests/integrations/your-service.json
```

Or for core capabilities:
```
~/.tairseach/manifests/core/your-capability.json
```

---

## Minimal Template (Internal Handler)

```json
{
  "manifest_version": "1.0.0",
  "id": "your-service",
  "name": "Your Service",
  "description": "Your Service integration via API",
  "version": "0.1.0",
  "category": "productivity",
  "tools": [
    {
      "name": "yourservice_action",
      "description": "Perform an action with Your Service",
      "mcp_expose": true,
      "inputSchema": {
        "type": "object",
        "required": ["param1"],
        "properties": {
          "param1": {
            "type": "string",
            "description": "Required parameter"
          },
          "param2": {
            "type": "string",
            "description": "Optional parameter"
          }
        },
        "additionalProperties": false
      },
      "outputSchema": {
        "type": "object",
        "required": ["result"],
        "properties": {
          "result": {
            "type": "string"
          },
          "metadata": {
            "type": "object"
          }
        }
      }
    }
  ],
  "implementation": {
    "type": "internal",
    "module": "proxy.handlers.yourservice",
    "methods": {
      "yourservice_action": "yourservice.action"
    }
  },
  "compatibility": {
    "mcpProtocol": "2025-03-26",
    "os": ["macos", "linux", "windows"]
  }
}
```

---

## With OAuth Credentials

```json
{
  "manifest_version": "1.0.0",
  "id": "your-oauth-service",
  "name": "Your OAuth Service",
  "description": "Integration requiring OAuth2",
  "version": "0.1.0",
  "category": "communication",
  "tools": [
    {
      "name": "fetch_data",
      "description": "Fetch data from service",
      "mcp_expose": true,
      "inputSchema": {
        "type": "object",
        "properties": {
          "query": { "type": "string" }
        }
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "data": { "type": "array" }
        }
      },
      "requires": {
        "credentials": [
          {
            "id": "yourservice_oauth",
            "provider": "yourservice",
            "kind": "oauth2",
            "scopes": ["read:data", "write:data"]
          }
        ],
        "permissions": []
      }
    }
  ],
  "implementation": {
    "type": "internal",
    "module": "proxy.handlers.yourservice",
    "methods": {
      "fetch_data": "yourservice.fetchData"
    }
  }
}
```

---

## With macOS Permissions

```json
{
  "manifest_version": "1.0.0",
  "id": "macos-automation",
  "name": "macOS Automation",
  "description": "Automation requiring system permissions",
  "version": "0.1.0",
  "category": "automation",
  "tools": [
    {
      "name": "automation_click",
      "description": "Click at screen coordinates",
      "mcp_expose": false,
      "inputSchema": {
        "type": "object",
        "required": ["x", "y"],
        "properties": {
          "x": { "type": "number" },
          "y": { "type": "number" }
        }
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "clicked": { "type": "boolean" }
        }
      },
      "requires": {
        "permissions": [
          {
            "name": "accessibility",
            "reason": "Required to simulate clicks"
          }
        ]
      },
      "annotations": {
        "destructiveHint": true
      }
    }
  ],
  "implementation": {
    "type": "internal",
    "module": "proxy.handlers.automation",
    "methods": {
      "automation_click": "automation.click"
    }
  },
  "compatibility": {
    "os": ["macos"]
  }
}
```

---

## Proxy Implementation (Direct API Calls)

```json
{
  "manifest_version": "1.0.0",
  "id": "rest-api-service",
  "name": "REST API Service",
  "description": "Direct REST API integration via proxy",
  "version": "0.1.0",
  "category": "data",
  "tools": [
    {
      "name": "get_items",
      "description": "Retrieve items from API",
      "inputSchema": {
        "type": "object",
        "properties": {
          "limit": { "type": "integer" }
        }
      },
      "outputSchema": {
        "type": "array"
      }
    }
  ],
  "implementation": {
    "type": "proxy",
    "baseUrl": "https://api.example.com/v1",
    "auth": {
      "strategy": "bearer",
      "credentialId": "api_token",
      "headerName": "Authorization",
      "tokenField": "access_token"
    },
    "toolBindings": {
      "get_items": {
        "method": "GET",
        "path": "/items",
        "query": {
          "limit": "{{limit}}"
        },
        "headers": {
          "Accept": "application/json"
        },
        "responsePath": "$.data"
      }
    }
  }
}
```

---

## Script Implementation (CLI Tools)

```json
{
  "manifest_version": "1.0.0",
  "id": "cli-tool",
  "name": "CLI Tool",
  "description": "Integration via external CLI binary",
  "version": "0.1.0",
  "category": "utilities",
  "tools": [
    {
      "name": "run_command",
      "description": "Execute CLI command",
      "inputSchema": {
        "type": "object",
        "required": ["action"],
        "properties": {
          "action": { "type": "string" },
          "args": {
            "type": "array",
            "items": { "type": "string" }
          }
        }
      },
      "outputSchema": {
        "type": "object"
      }
    }
  ],
  "implementation": {
    "type": "script",
    "runtime": "cli-tool",
    "entrypoint": "/usr/local/bin/cli-tool",
    "env": {
      "CLI_CONFIG": "~/.config/cli-tool"
    },
    "toolBindings": {
      "run_command": {
        "action": "{{action}}",
        "input_mode": "json",
        "output_mode": "json"
      }
    }
  }
}
```

---

## Field Reference

### Root Level

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `manifest_version` | string | ✅ | Always `"1.0.0"` |
| `id` | string | ✅ | Unique identifier (lowercase, hyphens) |
| `name` | string | ✅ | Display name |
| `description` | string | ✅ | Short description |
| `version` | string | ✅ | Semantic version |
| `category` | string | ✅ | Category (productivity, communication, etc.) |
| `tools` | array | ✅ | Tool definitions |
| `implementation` | object | ✅ | How tools are implemented |
| `compatibility` | object | ❌ | OS/protocol constraints |

### Tool Definition

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | ✅ | Tool identifier (snake_case) |
| `description` | string | ✅ | What the tool does |
| `mcp_expose` | boolean | ❌ | Expose via MCP (default: true) |
| `inputSchema` | object | ✅ | JSON Schema for parameters |
| `outputSchema` | object | ✅ | JSON Schema for result |
| `requires` | object | ❌ | Credentials/permissions needed |
| `annotations` | object | ❌ | Hints (destructiveHint, etc.) |

### Input/Output Schema

Standard JSON Schema with:
- `type`: `object`, `array`, `string`, `number`, `boolean`, `integer`
- `required`: Array of required property names
- `properties`: Object property definitions
- `additionalProperties`: Usually `false` for strict validation

### Requires

```json
{
  "credentials": [
    {
      "id": "credential_id",
      "provider": "google",
      "kind": "oauth2",
      "scopes": ["scope1", "scope2"],
      "optional": false
    }
  ],
  "permissions": [
    {
      "name": "accessibility",
      "optional": false,
      "reason": "Why this permission is needed"
    }
  ]
}
```

### Implementation Types

**Internal:**
```json
{
  "type": "internal",
  "module": "proxy.handlers.your_handler",
  "methods": {
    "tool_name": "namespace.method"
  }
}
```

**Proxy:**
```json
{
  "type": "proxy",
  "baseUrl": "https://api.example.com",
  "auth": { ... },
  "toolBindings": { ... }
}
```

**Script:**
```json
{
  "type": "script",
  "runtime": "binary-name",
  "entrypoint": "/path/to/binary",
  "env": { "KEY": "value" },
  "toolBindings": { ... }
}
```

---

## Checklist

- [ ] Create manifest file in `~/.tairseach/manifests/`
- [ ] Set `manifest_version` to `"1.0.0"`
- [ ] Choose unique `id` (lowercase-hyphenated)
- [ ] Define at least one tool
- [ ] Provide complete `inputSchema` and `outputSchema`
- [ ] Specify credentials if needed (OAuth, API keys)
- [ ] Specify permissions if needed (macOS)
- [ ] Choose implementation type: internal, proxy, or script
- [ ] Set `mcp_expose: true` for MCP access
- [ ] Add `destructiveHint: true` annotation for dangerous operations
- [ ] Validate JSON syntax
- [ ] Restart Tairseach or wait for hot-reload
- [ ] Test via MCP bridge: `tairseach-mcp --socket ~/.tairseach/socket`

---

## Common Categories

- `productivity` — Task management, notes, organization
- `communication` — Email, messaging, chat
- `automation` — System automation, scripting
- `health` — Fitness, wellness tracking
- `finance` — Banking, payments, budgeting
- `development` — Code tools, DevOps
- `security` — Passwords, secrets, authentication
- `data` — APIs, databases, storage
- `utilities` — General-purpose tools

---

## See Also

- [modules/manifests.md](../modules/manifests.md) — Manifest system architecture
- [handler-pattern.md](handler-pattern.md) — Create internal handler implementation
- [modules/router.md](../modules/router.md) — How manifests are routed
