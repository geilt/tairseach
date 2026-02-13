# Handlers Module

> **Location:** `src-tauri/src/proxy/handlers/`  
> **Files:** 17 (16 handlers + `mod.rs`)  
> **Lines:** 6,367  
> **Purpose:** JSON-RPC method handlers for all capabilities

---

## Overview

Handlers receive JSON-RPC requests from the Unix socket proxy server and execute capability actions. Each handler is responsible for:

1. **Parameter extraction** using `common.rs` utilities
2. **Authentication** via `AuthBroker` for OAuth-based handlers
3. **Permission validation** (enforced by `HandlerRegistry`)
4. **Execution** of the actual capability (API call, system FFI, etc.)
5. **Response construction** using `JsonRpcResponse` helpers

---

## File Listing

| File | Lines | Purpose |
|------|-------|---------|
| `mod.rs` | ~400 | Handler registry + permission middleware |
| `common.rs` | ~380 | **Shared utilities** — param extraction, auth helpers, response builders |
| `auth.rs` | ~450 | Auth broker methods: token get/refresh/revoke, credential CRUD |
| `automation.rs` | ~200 | macOS UI automation via AppleScript/Accessibility |
| `calendar.rs` | ~180 | macOS Calendar API (EventKit FFI) |
| `config.rs` | ~250 | App configuration get/set |
| `contacts.rs` | ~190 | macOS Contacts API (AddressBook FFI) |
| `files.rs` | ~150 | File read/write/list (requires Full Disk Access) |
| `gmail.rs` | ~260 | Gmail API: list/get/send/modify/trash/delete messages + labels |
| `google_calendar.rs` | ~320 | Google Calendar API: events CRUD + calendar list |
| `jira.rs` | ~180 | Jira API: issues, search, comments |
| `location.rs` | ~130 | macOS Core Location API (lat/long) |
| `onepassword.rs` | ~520 | 1Password CLI integration via Go FFI helper |
| `oura.rs` | ~210 | Oura Ring API: sleep/activity/readiness data |
| `permissions.rs` | ~280 | Permission check/request/grant/revoke (TCC interaction) |
| `reminders.rs` | ~160 | macOS Reminders API (EventKit FFI) |
| `screen.rs` | ~140 | Screen capture via macOS APIs |

---

## Key Types

### From `mod.rs`

```rust
pub struct HandlerRegistry {
    router: Option<Arc<CapabilityRouter>>,
}

pub enum PermissionStatus {
    Granted,
    Denied,
    NotDetermined,
    Restricted,
    Unknown,
}
```

**Methods:**
- `HandlerRegistry::new()` — Create registry without router (legacy mode)
- `HandlerRegistry::with_router(router)` — Create with capability router (manifest-aware)
- `HandlerRegistry::handle(&self, request)` — Main dispatch entry point

**Dispatch Logic:**
1. Try manifest-based routing (if router available)
2. Check permissions for known methods via `required_permission()`
3. Parse method into `(namespace, action)`
4. Route to appropriate handler module

---

## Common Utilities (`common.rs`)

### Parameter Extraction

All handlers use these for type-safe parameter extraction:

```rust
// Required parameters
pub fn require_string(params: &Value, key: &str, id: &Value) -> Result<&str, JsonRpcResponse>
pub fn require_f64(params: &Value, key: &str, id: &Value) -> Result<f64, JsonRpcResponse>

// Optional parameters
pub fn optional_string(params: &Value, key: &str) -> Option<&str>
pub fn optional_u64(params: &Value, key: &str) -> Option<u64>
pub fn optional_f64(params: &Value, key: &str) -> Option<f64>
pub fn optional_bool(params: &Value, key: &str) -> Option<bool>
pub fn optional_string_array(params: &Value, key: &str) -> Option<Vec<String>>

// With defaults
pub fn string_with_default(params: &Value, key: &str, default: &str) -> &str
pub fn u64_with_default(params: &Value, key: &str, default: u64) -> u64
pub fn bool_with_default(params: &Value, key: &str, default: bool) -> bool

// Alias fallbacks (for camelCase/snake_case compatibility)
pub fn require_string_or(params: &Value, primary: &str, fallback: &str, id: &Value) -> Result<&str, JsonRpcResponse>
pub fn optional_string_or(params: &Value, primary: &str, fallback: &str) -> Option<&str>
pub fn optional_u64_or(params: &Value, primary: &str, fallback: &str) -> Option<u64>
pub fn u64_or_with_default(params: &Value, primary: &str, fallback: &str, default: u64) -> u64
pub fn optional_string_array_or(params: &Value, primary: &str, fallback: &str) -> Option<Vec<String>>
```

### Auth Helpers

```rust
// Get or initialize shared auth broker
pub async fn get_auth_broker() -> Result<&'static Arc<AuthBroker>, JsonRpcResponse>

// Extract provider + account from params (defaults to "google" if not specified)
pub fn extract_oauth_credentials(params: &Value, default_provider: &str) -> Result<(String, String), JsonRpcResponse>

// Extract access_token from auth broker response
pub fn extract_access_token(token_data: &Value, id: &Value) -> Result<String, JsonRpcResponse>
```

**Pattern for OAuth handlers:**
```rust
let auth_broker = get_auth_broker().await?;
let (provider, account) = extract_oauth_credentials(params, "google")?;

let token_data = auth_broker.get_token(&provider, &account, Some(&scopes)).await
    .map_err(|(code, msg)| error(id.clone(), code, msg))?;

let access_token = extract_access_token(&token_data, &id)?;
```

### Response Helpers

```rust
pub fn ok(id: Value, data: Value) -> JsonRpcResponse
pub fn error(id: Value, code: i32, message: impl Into<String>) -> JsonRpcResponse
pub fn generic_error(id: Value, message: impl Into<String>) -> JsonRpcResponse
pub fn invalid_params(id: Value, message: impl Into<String>) -> JsonRpcResponse
pub fn method_not_found(id: Value, method: &str) -> JsonRpcResponse
pub fn simple_success(id: Value) -> JsonRpcResponse  // Returns { "success": true }
pub fn success_with_count(id: Value, data: Value, count: usize) -> JsonRpcResponse
```

---

## Handler Patterns

### Standard Handler Structure

All handlers follow this template:

```rust
use serde_json::Value;
use tracing::{debug, error, info};

use super::common::*;
use super::super::protocol::JsonRpcResponse;

pub async fn handle(action: &str, params: &Value, id: Value) -> JsonRpcResponse {
    // Dispatch to specific action handler
    match action {
        "method1" | "aliasMethod1" => handle_method1(params, id).await,
        "method2" => handle_method2(params, id).await,
        _ => method_not_found(id, &format!("namespace.{}", action)),
    }
}

async fn handle_method1(params: &Value, id: Value) -> JsonRpcResponse {
    info!("Handling namespace.method1");
    
    // Extract params
    let required_param = match require_string(params, "param", &id) {
        Ok(val) => val,
        Err(response) => return response,
    };
    let optional_param = optional_u64(params, "count").unwrap_or(10);
    
    // Execute logic
    match do_something(required_param, optional_param).await {
        Ok(result) => ok(id, serde_json::json!(result)),
        Err(e) => {
            error!("Failed to do something: {}", e);
            generic_error(id, e)
        }
    }
}
```

### OAuth Handler Pattern

Handlers that use OAuth (Gmail, Google Calendar, Oura, Jira, etc.):

```rust
pub async fn handle(action: &str, params: &Value, id: Value) -> JsonRpcResponse {
    // Get auth broker
    let auth_broker = match get_auth_broker().await {
        Ok(broker) => broker,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };
    
    // Extract OAuth credentials (provider + account)
    let (provider, account) = match extract_oauth_credentials(params, "google") {
        Ok(creds) => creds,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };
    
    // Get access token with required scopes
    let token_data = match auth_broker.get_token(
        &provider,
        &account,
        Some(&["https://www.googleapis.com/auth/gmail.readonly".to_string()]),
    ).await {
        Ok(data) => data,
        Err((code, msg)) => {
            error!("Failed to get OAuth token: {}", msg);
            return error(id, code, msg);
        }
    };
    
    let access_token = match extract_access_token(&token_data, &id) {
        Ok(token) => token,
        Err(response) => return response,
    };
    
    // Create API client and dispatch
    let api_client = ApiClient::new(access_token);
    match action {
        "list" => handle_list(params, id, api_client).await,
        _ => method_not_found(id, &format!("namespace.{}", action)),
    }
}
```

---

## Handler Dependencies

Handlers import from these modules:

| Handler | Imports |
|---------|---------|
| **gmail, google_calendar** | `crate::google::{GmailApi, CalendarApi, GoogleOAuthClient}` |
| **contacts** | `crate::contacts` (AddressBook FFI) |
| **calendar, reminders** | macOS EventKit FFI (inline) |
| **location** | macOS Core Location FFI (inline) |
| **screen** | macOS ScreenCaptureKit FFI (inline) |
| **automation** | macOS Accessibility/AppleScript FFI (inline) |
| **permissions** | `crate::permissions` |
| **config** | `crate::config` |
| **auth** | `crate::auth::{AuthBroker, CredentialStore}` |
| **onepassword** | Go FFI helper (`op-helper` binary bundled with app) |

---

## Permission Enforcement

`HandlerRegistry` enforces permissions via `required_permission()`:

```rust
fn required_permission(method: &str) -> Option<&'static str> {
    match method {
        "contacts.*" => Some("contacts"),
        "calendar.*" => Some("calendar"),
        "reminders.*" => Some("reminders"),
        "location.*" => Some("location"),
        "screen.*" => Some("screen_recording"),
        "automation.click" | "automation.type" => Some("accessibility"),
        "files.*" => Some("full_disk_access"),
        // Auth, config, permissions don't require macOS permissions
        _ => None,
    }
}
```

If permission is not granted, returns:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32003,
    "message": "Permission denied: contacts is denied"
  }
}
```

---

## Adding a New Handler

1. **Create `src-tauri/src/proxy/handlers/your_namespace.rs`**
2. **Add to `mod.rs`:**
   ```rust
   pub mod your_namespace;
   ```
3. **Add to dispatch in `HandlerRegistry::handle()`:**
   ```rust
   "your_namespace" => your_namespace::handle(action, &request.params, id).await,
   ```
4. **Use `common.rs` utilities** — don't recreate param extraction
5. **Follow the pattern:** See [patterns/handler-pattern.md](../patterns/handler-pattern.md)

---

## Testing

Handlers are tested via:

1. **Unit tests** in each handler file (mock params/responses)
2. **Integration tests** via MCP client:
   ```bash
   cd crates/tairseach-mcp
   cargo run -- --socket ~/.tairseach/socket
   # Then call methods via MCP protocol
   ```
3. **Frontend testing** via Tauri UI

---

## Error Codes Reference

Handlers use these JSON-RPC error codes:

| Code | Meaning | Helper |
|------|---------|--------|
| -32600 | Invalid Request | (protocol-level) |
| -32601 | Method Not Found | `method_not_found()` |
| -32602 | Invalid Params | `invalid_params()` |
| -32603 | Internal Error | `generic_error()` |
| -32000 | Generic Error | `error()` with custom code |
| -32003 | Permission Denied | `JsonRpcResponse::permission_denied()` |
| -32010 | Token Not Found | (auth broker) |
| -32011 | Token Refresh Failed | (auth broker) |

See `common/error.rs` for full list.

---

## Anti-Patterns (DON'T DO)

❌ **Inline param extraction:**
```rust
let account = params.get("account").and_then(|v| v.as_str()).ok_or(...)?;  // NO
```

✅ **Use common utilities:**
```rust
let account = require_string(params, "account", &id)?;  // YES
```

---

❌ **Manual token refresh:**
```rust
if token_expired { refresh_token(...).await?; }  // NO
```

✅ **Let auth broker handle it:**
```rust
let token_data = auth_broker.get_token(&provider, &account, Some(&scopes)).await?;  // YES
```

---

❌ **Inline HTTP client:**
```rust
let client = reqwest::Client::new();  // NO
```

✅ **Use common HTTP utilities:**
```rust
use crate::common::http::create_http_client;
let client = create_http_client()?;  // YES
```

---

## Recent Refactorings

**Branch:** `refactor/handler-dry` (merged 2026-02-13)

**Changes:**
- Extracted all param helpers to `common.rs`
- Added alias fallback helpers (`require_string_or`, etc.)
- Extracted OAuth pattern helpers (`get_auth_broker`, `extract_oauth_credentials`)
- Standardized response construction
- Removed ~500 lines of duplicated code across handlers

**Before:** Each handler had its own param extraction logic  
**After:** All handlers use `common.rs` utilities

---

*For implementation templates, see [patterns/handler-pattern.md](../patterns/handler-pattern.md)*
