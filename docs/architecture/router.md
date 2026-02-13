# Router Architecture

**Location:** `src-tauri/src/router/`

The **Capability Router** dynamically dispatches JSON-RPC tool calls based on **manifest-defined implementations**. It provides a unified routing layer that supports:

1. **Internal handlers** — existing Rust handlers (legacy)
2. **Proxy calls** — HTTP API calls with credential injection
3. **Script execution** — external scripts with environment-based credential passing

This architecture enables declarative tool registration via manifests while maintaining backward compatibility with the legacy handler system.

---

## Architecture Overview

```
JSON-RPC Request
      ↓
HandlerRegistry
      ↓
  ┌───────────────────────────┐
  │   CapabilityRouter.route  │
  └───────────────────────────┘
      ↓
  ManifestRegistry lookup
      ↓
  ┌─────────────┬──────────────┬──────────────┐
  │  Internal   │    Proxy     │   Script     │
  │  Dispatch   │   Dispatch   │  Dispatch    │
  └─────────────┴──────────────┴──────────────┘
      ↓               ↓               ↓
  Rust handler    HTTP API      External script
```

---

## Core Components

### CapabilityRouter

**File:** `router/mod.rs`, `router/dispatcher.rs`

The main router struct:

```rust
pub struct CapabilityRouter {
    registry: Arc<ManifestRegistry>,
    auth_broker: Arc<AuthBroker>,
}
```

**Responsibilities:**
- Look up tool definitions in `ManifestRegistry`
- Check required permissions
- Load required credentials from `AuthBroker`
- Dispatch to appropriate implementation type

---

## Routing Flow

### 1. Tool Lookup

```rust
pub async fn route(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
    let tool_name = &request.method;
    
    // Find tool in manifest registry
    let (manifest, tool) = match self.registry.find_tool(tool_name).await {
        Some(result) => result,
        None => return JsonRpcResponse::method_not_found(id, tool_name),
    };
    
    // ...
}
```

If the tool is not found in any loaded manifest, the router returns `method_not_found` (-32601), which signals the `HandlerRegistry` to fall back to legacy routing.

---

### 2. Permission Checks

```rust
// Check manifest-level permissions
for perm_req in &manifest.requires.permissions {
    if let Err(response) = self.check_permission(&perm_req.name, &id).await {
        return response;
    }
}

// Check tool-level permissions (override manifest-level)
if let Some(ref tool_reqs) = tool.requires {
    for perm_req in &tool_reqs.permissions {
        if let Err(response) = self.check_permission(&perm_req.name, &id).await {
            return response;
        }
    }
}
```

**Permission check:**
```rust
async fn check_permission(&self, permission: &str, id: &Value) -> Result<(), JsonRpcResponse> {
    let perm = permissions::check_permission(permission)?;
    
    if perm.status != permissions::PermissionStatus::Granted {
        return Err(JsonRpcResponse::permission_denied(id, permission, ...));
    }
    
    Ok(())
}
```

---

### 3. Credential Loading

```rust
async fn load_credentials(
    &self,
    manifest: &Manifest,
    tool: &Tool,
    params: &Value,
) -> Result<HashMap<String, Value>, JsonRpcResponse> {
    let mut credentials = HashMap::new();
    
    // Manifest-level credentials
    for cred_req in &manifest.requires.credentials {
        if let Some(cred) = self.load_credential(&cred_req.id, &cred_req.provider, params).await? {
            credentials.insert(cred_req.id.clone(), cred);
        }
    }
    
    // Tool-level credentials (override manifest-level)
    if let Some(ref tool_reqs) = tool.requires {
        for cred_req in &tool_reqs.credentials {
            if let Some(cred) = self.load_credential(&cred_req.id, &cred_req.provider, params).await? {
                credentials.insert(cred_req.id.clone(), cred);
            }
        }
    }
    
    Ok(credentials)
}
```

**Credential resolution:**
- Provider inferred from `credential_id` (e.g., `google-oauth` → `google`)
- Account from `params.account` (default: `"me"`)
- Token retrieved from `AuthBroker.get_token(provider, account, scopes)`

---

### 4. Implementation Dispatch

```rust
match &manifest.implementation {
    Implementation::Internal { module, methods } => {
        internal::dispatch(manifest, tool, params, id, module, methods).await
    }
    Implementation::Script { runtime, entrypoint, args, env, tool_bindings } => {
        script::dispatch(manifest, tool, params, id, runtime, entrypoint, args, env, tool_bindings, credentials).await
    }
    Implementation::Proxy { base_url, auth, tool_bindings } => {
        proxy::dispatch(manifest, tool, params, id, base_url, auth, tool_bindings, credentials).await
    }
}
```

---

## Implementation Types

### Internal Implementation

**File:** `router/internal.rs`

Routes to **existing Rust handlers** (backward compatibility).

**Manifest example:**
```json
{
  "implementation": {
    "type": "internal",
    "module": "contacts",
    "methods": {
      "contacts_list": "contacts.list",
      "contacts_get": "contacts.get"
    }
  }
}
```

**Dispatch logic:**
```rust
pub async fn dispatch(
    _manifest: &Manifest,
    tool: &Tool,
    params: &Value,
    id: Value,
    module: &str,
    methods: &HashMap<String, String>,
) -> JsonRpcResponse {
    // Get handler method name
    let method = methods.get(&tool.name)?;
    
    // Parse "namespace.action"
    let (namespace, action) = method.split_once('.')?;
    
    // Route to handler
    match namespace {
        "contacts" => handlers::contacts::handle(action, params, id).await,
        "calendar" => handlers::calendar::handle(action, params, id).await,
        "gmail" => handlers::gmail::handle(action, params, id).await,
        // ...
    }
}
```

**Supported namespaces:**
- `contacts`, `calendar`, `reminders`
- `location`, `screen`, `files`, `automation`
- `auth`, `permissions`, `config`
- `gmail`, `gcalendar`

---

### Proxy Implementation

**File:** `router/proxy.rs`

Makes **HTTP API calls** with credential injection via auth headers.

**Manifest example:**
```json
{
  "implementation": {
    "type": "proxy",
    "baseUrl": "https://api.example.com",
    "auth": {
      "strategy": "oauth2Bearer",
      "credentialId": "example-oauth",
      "tokenField": "access_token"
    },
    "toolBindings": {
      "example_get_user": {
        "method": "GET",
        "path": "/users/{userId}",
        "query": {},
        "headers": {},
        "responsePath": "data.user"
      }
    }
  }
}
```

**Auth strategies:**
- `oauth2Bearer` — `Authorization: Bearer <token>`
- `apiKeyHeader` — Custom header (e.g., `X-API-Key: <key>`)
- `apiKeyQuery` — Query parameter (e.g., `?api_key=<key>`)
- `basic` — `Authorization: Basic <base64(user:pass)>`

**Path interpolation:**
```rust
// params: { "userId": "123" }
// path: "/users/{userId}"
// → "/users/123"
```

**Response extraction:**
```json
// API returns: { "data": { "user": { "id": 1, "name": "Alice" } } }
// responsePath: "data.user"
// → { "id": 1, "name": "Alice" }
```

**HTTP methods:** `GET`, `POST`, `PUT`, `PATCH`, `DELETE`

**Error handling:**
- HTTP errors (4xx, 5xx) → JSON-RPC error with status code
- Network errors → JSON-RPC error
- Invalid JSON response → JSON-RPC error

---

### Script Implementation

**File:** `router/script.rs`

Executes **external scripts** with credential injection via environment variables.

**Manifest example:**
```json
{
  "implementation": {
    "type": "script",
    "runtime": "python3",
    "entrypoint": "~/scripts/example.py",
    "args": [],
    "env": {
      "API_TOKEN": "{credentials.example-api.api_key}",
      "BASE_URL": "https://api.example.com"
    },
    "toolBindings": {
      "example_list": { "action": "list" },
      "example_get": { "action": "get" }
    }
  }
}
```

**Supported runtimes:**
- `bash`, `sh` — Shell scripts
- `python3` — Python scripts
- `node` — Node.js scripts
- `ruby` — Ruby scripts
- `custom` — Custom executable (entrypoint is the binary path)

**Script path resolution:**
- Absolute: `/path/to/script.py` → used as-is
- Home-relative: `~/scripts/example.py` → `$HOME/scripts/example.py`
- Relative: `example.py` → `~/.tairseach/scripts/example.py`

**Credential injection:**
```json
// env template: { "API_TOKEN": "{credentials.example-api.api_key}" }
// credentials: { "example-api": { "api_key": "secret123" } }
// → Environment: API_TOKEN=secret123
```

**Input/Output protocol:**
- **Input:** JSON on stdin:
  ```json
  {
    "tool": "example_list",
    "action": "list",
    "params": { "limit": 10 }
  }
  ```
- **Output:** JSON on stdout:
  ```json
  {
    "items": [...],
    "count": 10
  }
  ```

**Standard environment variables:**
- `TAIRSEACH_TOOL` — Tool name
- `TAIRSEACH_ACTION` — Action name

**Security:**
- Environment is **cleared** before execution (`env_clear()`)
- Only manifest-defined variables are injected
- Timeout: 60 seconds (configurable in future)

**Error handling:**
- Non-zero exit code → JSON-RPC error with stderr
- Timeout → JSON-RPC error
- Invalid JSON output → JSON-RPC error with raw stdout

---

## Routing Precedence

The `HandlerRegistry` tries routing strategies in this order:

1. **Manifest-based routing** (via `CapabilityRouter`)
   - If tool is found in manifest registry → route via manifest implementation
   - If method_not_found → continue to legacy routing

2. **Legacy routing**
   - Check permissions via `required_permission(method)`
   - Dispatch to handler namespace

**Example:**
```rust
impl HandlerRegistry {
    pub async fn handle(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        // 1. Try manifest-based routing
        if let Some(router) = &self.router {
            let response = router.route(request).await;
            if !is_method_not_found(&response) {
                return response;
            }
        }
        
        // 2. Fall back to legacy routing
        if let Some(required) = required_permission(&request.method) {
            // Check permission...
        }
        
        let (namespace, action) = request.parse_method();
        match namespace {
            "auth" => auth::handle(action, params, id).await,
            "contacts" => contacts::handle(action, params, id).await,
            // ...
        }
    }
}
```

This allows **gradual migration** from legacy handlers to manifest-based tools without breaking existing MCP clients.

---

## Common Utilities

**File:** `src-tauri/src/common/interpolation.rs` (implied by usage in proxy/script)

### Parameter Interpolation

Used in proxy path/query construction:

```rust
fn interpolate_params(template: &str, params: &Value) -> String {
    // Replace "{paramName}" with params.paramName
    // Example: "/users/{userId}" + { "userId": "123" } → "/users/123"
}
```

### Credential Interpolation

Used in script environment variable injection:

```rust
fn interpolate_credentials(template: &str, credentials: &HashMap<String, Value>) -> String {
    // Replace "{credentials.cred-id.field}" with actual credential value
    // Example: "{credentials.google-oauth.access_token}" → "ya29.a0..."
}
```

---

## Security Model

### Permission Enforcement
- Permissions checked **before** implementation dispatch
- Manifest-level and tool-level permissions combined
- Denied permissions → immediate error response

### Credential Isolation
- Credentials loaded from `AuthBroker` on-demand
- Never exposed to client (only used internally)
- Proxy: injected via HTTP headers (transient)
- Script: injected via environment variables (cleared after execution)

### Script Security
- Environment **cleared** before script execution
- Only manifest-defined variables injected
- Scripts run with Tairseach's privileges (not elevated)
- Timeout prevents runaway scripts

### HTTP Security
- TLS verification enabled (default `reqwest` behavior)
- Auth headers never logged
- HTTP errors include status but not response bodies with secrets

---

## Future Enhancements

### Planned Features
- **Streaming responses** — SSE/WebSocket support for long-running operations
- **Retries** — automatic retry with exponential backoff for transient failures
- **Caching** — response caching for idempotent operations
- **Rate limiting** — per-tool rate limits
- **Circuit breakers** — fail-fast when upstream is down

### Performance Optimizations
- **Connection pooling** — reuse HTTP connections for proxy calls
- **Script process pooling** — keep interpreter processes alive for repeated calls
- **Parallel credential loading** — load credentials concurrently

### Developer Experience
- **Manifest validation** — schema validation on load
- **Dry-run mode** — test manifest routing without side effects
- **Debug logging** — detailed routing traces

---

## Example Manifests

### Internal Handler Wrapper

Expose existing `contacts.list` handler as an MCP tool:

```json
{
  "name": "contacts-macos",
  "version": "1.0.0",
  "implementation": {
    "type": "internal",
    "module": "contacts",
    "methods": {
      "contacts_list": "contacts.list",
      "contacts_get": "contacts.get"
    }
  },
  "requires": {
    "permissions": [{ "name": "contacts" }],
    "credentials": []
  },
  "tools": [
    {
      "name": "contacts_list",
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
```

### HTTP Proxy Example

Jira API via OAuth:

```json
{
  "name": "jira-cloud",
  "version": "1.0.0",
  "implementation": {
    "type": "proxy",
    "baseUrl": "https://your-domain.atlassian.net/rest/api/3",
    "auth": {
      "strategy": "oauth2Bearer",
      "credentialId": "jira-oauth"
    },
    "toolBindings": {
      "jira_search_issues": {
        "method": "GET",
        "path": "/search",
        "query": { "jql": "{jql}", "maxResults": "{maxResults}" },
        "responsePath": "issues"
      },
      "jira_get_issue": {
        "method": "GET",
        "path": "/issue/{issueKey}"
      }
    }
  },
  "requires": {
    "credentials": [{ "id": "jira-oauth", "provider": "jira" }]
  },
  "tools": [
    {
      "name": "jira_search_issues",
      "description": "Search Jira issues using JQL",
      "inputSchema": {
        "type": "object",
        "properties": {
          "jql": { "type": "string" },
          "maxResults": { "type": "number", "default": 50 }
        },
        "required": ["jql"]
      }
    }
  ]
}
```

### Script Example

Custom Python script:

```json
{
  "name": "weather-api",
  "version": "1.0.0",
  "implementation": {
    "type": "script",
    "runtime": "python3",
    "entrypoint": "~/tairseach/scripts/weather.py",
    "env": {
      "WEATHER_API_KEY": "{credentials.weather-api.api_key}"
    },
    "toolBindings": {
      "weather_current": { "action": "current" },
      "weather_forecast": { "action": "forecast" }
    }
  },
  "requires": {
    "credentials": [{ "id": "weather-api", "provider": "openweathermap" }]
  },
  "tools": [
    {
      "name": "weather_current",
      "description": "Get current weather for a location",
      "inputSchema": {
        "type": "object",
        "properties": {
          "location": { "type": "string" }
        },
        "required": ["location"]
      }
    }
  ]
}
```

---

**See also:**
- [Manifest System](./manifest-system.md) — manifest file format & registry
- [Handlers](./handlers.md) — internal handler implementations
- [Auth System](./auth-system.md) — credential storage & retrieval
- [Permissions](./permissions.md) — macOS TCC permission checks
