# Router Module

> **Location:** `src-tauri/src/router/`  
> **Files:** 5  
> **Lines:** 736  
> **Purpose:** Capability routing and manifest-based dispatch

---

## Overview

The Router module implements manifest-driven capability routing. It parses JSON manifests, validates requirements (credentials, permissions), and dispatches tool calls to the appropriate implementation layer (internal Rust handler, HTTP proxy, or external script).

**Key Responsibilities:**
- Load and validate capability manifests
- Route tool calls based on manifest configuration
- Validate credential and permission requirements before execution
- Dispatch to internal/proxy/script handlers

---

## File Listing

| File | Lines | Purpose |
|------|-------|---------|
| `mod.rs` | ~50 | Module exports |
| `dispatcher.rs` | ~150 | `CapabilityRouter` — main routing logic |
| `internal.rs` | ~200 | Internal handler routing (Rust implementations) |
| `proxy.rs` | ~180 | HTTP proxy routing with OAuth token injection |
| `script.rs` | ~150 | External script execution (Python, Go, etc.) |

---

## Key Types

```rust
pub struct CapabilityRouter {
    registry: Arc<ManifestRegistry>,
    auth_broker: Arc<AuthBroker>,
}

pub enum RouteResult {
    Success(JsonRpcResponse),
    NotFound,  // Method not in manifests
    PermissionDenied(String),
    CredentialMissing(String),
}
```

---

## Core APIs

### CapabilityRouter

```rust
impl CapabilityRouter {
    pub fn new(registry: Arc<ManifestRegistry>, auth_broker: Arc<AuthBroker>) -> Self
    
    pub async fn route(&self, request: &JsonRpcRequest) -> JsonRpcResponse
}
```

**Routing Logic:**
1. Parse method name → find manifest + tool definition
2. Validate requirements (permissions + credentials)
3. Dispatch based on implementation type:
   - `Internal` → `internal::route()`
   - `Proxy` → `proxy::route()`
   - `Script` → `script::route()`

---

## Implementation Types

### Internal (Rust Handlers)

**Manifest Example:**
```json
{
  "implementation": {
    "type": "internal",
    "module": "gmail",
    "methods": {
      "list_messages": "list",
      "get_message": "get"
    }
  }
}
```

**Routing:**
```rust
// internal.rs
pub async fn route(
    module: &str,
    action: &str,
    params: &Value,
    id: Value,
) -> JsonRpcResponse {
    match module {
        "gmail" => crate::proxy::handlers::gmail::handle(action, params, id).await,
        "calendar" => crate::proxy::handlers::calendar::handle(action, params, id).await,
        // ...
        _ => JsonRpcResponse::method_not_found(id, module),
    }
}
```

---

### Proxy (HTTP Forwarding)

**Manifest Example:**
```json
{
  "implementation": {
    "type": "proxy",
    "baseUrl": "https://api.example.com",
    "auth": {
      "strategy": "oauth2",
      "credentialId": "google:user@example.com",
      "headerName": "Authorization",
      "tokenField": "access_token"
    },
    "toolBindings": {
      "list_items": {
        "method": "GET",
        "path": "/items",
        "query": { "limit": "{{limit}}" },
        "responsePath": "$.items"
      }
    }
  }
}
```

**Routing:**
```rust
// proxy.rs
pub async fn route(
    base_url: &str,
    auth_config: &ProxyAuth,
    tool_binding: &ProxyToolBinding,
    params: &Value,
    auth_broker: &AuthBroker,
) -> JsonRpcResponse {
    // 1. Get OAuth token from auth broker
    let token = auth_broker.get_token(&auth_config.credential_id, ...).await?;
    
    // 2. Interpolate params into path/query/body
    let url = interpolate(&tool_binding.path, params);
    let body = interpolate_json(&tool_binding.body_template, params);
    
    // 3. Build HTTP request with auth header
    let request = reqwest::Client::new()
        .request(method, url)
        .header(auth_config.header_name, format!("Bearer {}", token))
        .json(&body);
    
    // 4. Execute and parse response
    let response = request.send().await?;
    let data = response.json().await?;
    
    // 5. Extract result via JSONPath (if response_path specified)
    if let Some(path) = &tool_binding.response_path {
        extract_json_path(&data, path)
    } else {
        data
    }
}
```

---

### Script (External Execution)

**Manifest Example:**
```json
{
  "implementation": {
    "type": "script",
    "runtime": "python3",
    "entrypoint": "scripts/process.py",
    "env": { "API_KEY": "{{credential:api_key}}" },
    "toolBindings": {
      "process_data": {
        "action": "process",
        "input_mode": "json",
        "output_mode": "json"
      }
    }
  }
}
```

**Routing:**
```rust
// script.rs
pub async fn route(
    runtime: &str,
    entrypoint: &str,
    env: &HashMap<String, String>,
    tool_binding: &ScriptToolBinding,
    params: &Value,
    auth_broker: &AuthBroker,
) -> JsonRpcResponse {
    // 1. Resolve credential placeholders in env vars
    let resolved_env = resolve_credentials(env, auth_broker).await?;
    
    // 2. Build command
    let mut cmd = tokio::process::Command::new(runtime);
    cmd.arg(entrypoint)
       .arg(tool_binding.action)
       .envs(resolved_env);
    
    // 3. Write params to stdin (if input_mode == "json")
    if tool_binding.input_mode == Some("json") {
        cmd.stdin(Stdio::piped());
        let mut child = cmd.spawn()?;
        child.stdin.as_mut().unwrap().write_all(serde_json::to_string(params)?.as_bytes()).await?;
    }
    
    // 4. Execute and capture stdout
    let output = cmd.output().await?;
    
    // 5. Parse stdout (if output_mode == "json")
    if tool_binding.output_mode == Some("json") {
        serde_json::from_slice(&output.stdout)?
    } else {
        String::from_utf8(output.stdout)?
    }
}
```

---

## Requirement Validation

Before dispatching, router validates:

### Credential Requirements

```rust
for cred_req in &tool.requires.credentials {
    let exists = auth_broker.get_token(&cred_req.provider, &cred_req.id, Some(&cred_req.scopes)).await;
    
    if exists.is_err() && !cred_req.optional {
        return JsonRpcResponse::error(
            id,
            -32000,
            format!("Missing required credential: {}", cred_req.id),
            None,
        );
    }
}
```

### Permission Requirements

```rust
for perm_req in &tool.requires.permissions {
    let status = permissions::check_permission(&perm_req.name)?;
    
    if status != PermissionStatus::Granted && !perm_req.optional {
        return JsonRpcResponse::permission_denied(id, &perm_req.name, status.as_str());
    }
}
```

---

## String Interpolation

The router uses `common/interpolation.rs` for template substitution:

```rust
// In proxy/script routing
use crate::common::interpolation::interpolate;

// Substitute params into strings
let path = "/users/{{user_id}}/posts/{{post_id}}";
let result = interpolate(path, params);  // "/users/123/posts/456"

// Supports:
// - Direct params: {{param_name}}
// - Nested: {{user.email}}
// - Credentials: {{credential:api_key}}
```

---

## Dependencies

| Module | Imports |
|--------|---------|
| **manifest** | `ManifestRegistry`, `Manifest`, `Tool`, `Implementation` |
| **auth** | `AuthBroker` for credential resolution |
| **permissions** | Permission validation |
| **proxy/handlers** | Internal handler dispatch |
| **common/interpolation** | Template string substitution |

---

## Usage

Router is initialized in `lib.rs`:

```rust
let registry = Arc::new(ManifestRegistry::new());
registry.load_from_disk().await?;

let auth_broker = Arc::new(AuthBroker::new().await?);

let router = Arc::new(CapabilityRouter::new(registry, auth_broker));
```

Handler registry uses it:

```rust
let handlers = HandlerRegistry::with_router(router);

// In handle()
if let Some(router) = &self.router {
    let response = router.route(request).await;
    if !is_method_not_found(&response) {
        return response;
    }
}
// Fall through to legacy routing...
```

---

*For manifest structure, see [manifests.md](manifests.md)*
