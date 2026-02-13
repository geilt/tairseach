# Handler Architecture

**Location:** `src-tauri/src/proxy/handlers/`

Handlers implement the JSON-RPC method namespaces served over the Unix domain socket. Each handler module owns a specific domain (auth, contacts, calendar, etc.) and translates JSON-RPC requests into system API calls.

## Overview

Tairseach exposes 15 handler namespaces, registered in `mod.rs`:

| Namespace | Module | Purpose |
|-----------|--------|---------|
| `auth.*` | `auth.rs` | OAuth tokens, credentials, auth broker |
| `permissions.*` | `permissions.rs` | macOS TCC permission checks |
| `contacts.*` | `contacts.rs` | macOS Contacts.framework (CNContact) |
| `calendar.*` | `calendar.rs` | macOS EventKit calendars & events |
| `reminders.*` | `reminders.rs` | macOS EventKit reminders |
| `location.*` | `location.rs` | CoreLocation (GPS/Wi-Fi) |
| `screen.*` | `screen.rs` | Screenshots, window list |
| `files.*` | `files.rs` | Read/write files (with safety checks) |
| `automation.*` | `automation.rs` | AppleScript, JXA, UI automation |
| `config.*` | `config.rs` | OpenClaw config.json access |
| `gmail.*` | `gmail.rs` | Gmail API (OAuth via auth broker) |
| `gcalendar.*` | `google_calendar.rs` | Google Calendar API (OAuth) |
| `op.*` / `onepassword.*` | `onepassword.rs` | 1Password via op-helper binary |
| `oura.*` | `oura.rs` | Oura Ring API |
| `jira.*` | `jira.rs` | Jira REST API |

The `server.*` namespace is implemented directly in the `HandlerRegistry`.

---

## Handler Pattern

All handlers follow a consistent pattern:

```rust
pub async fn handle(action: &str, params: &Value, id: Value) -> JsonRpcResponse {
    match action {
        "list" => handle_list(params, id).await,
        "get" => handle_get(params, id).await,
        "create" => handle_create(params, id).await,
        _ => method_not_found(id, &format!("namespace.{}", action)),
    }
}
```

**Common utilities** (`common.rs`) provide:
- `ok(id, result)` — success response
- `error(id, code, message)` — error response
- `method_not_found(id, method)` — -32601 response
- `required_string(params, key)` — extract required param
- `optional_string(params, key)` — extract optional param
- `get_auth_broker()` — access the shared `AuthBroker`
- `extract_oauth_credentials(params, default_provider)` — parse `provider`/`account`

---

## Permission Enforcement

The `HandlerRegistry` enforces permissions **before** dispatching to handlers:

```rust
fn required_permission(method: &str) -> Option<&'static str> {
    match method {
        "contacts.list" | "contacts.get" | ... => Some("contacts"),
        "calendar.list" | "calendar.events" | ... => Some("calendar"),
        "location.get" | "location.watch" => Some("location"),
        "screen.capture" | "screen.windows" => Some("screen_recording"),
        "automation.click" | "automation.type" => Some("accessibility"),
        "files.read" | "files.write" => Some("full_disk_access"),
        // Auth/config/server methods don't require macOS permissions
        "auth.*" | "config.*" | "server.*" => None,
        _ => None,
    }
}
```

If a permission is not `Granted`, the request is rejected with a `permission_denied` error before reaching the handler.

**Manifest-based routing** (via `CapabilityRouter`) also checks permissions defined in tool manifests.

---

## Handler Details

### `auth.*` — Authentication & Credentials

**Module:** `auth.rs`  
**Purpose:** Manage OAuth tokens, API keys, and custom credentials via the `AuthBroker`.

| Method | Params | Returns | Description |
|--------|--------|---------|-------------|
| `auth.status` | — | `{ initialized, providers }` | Broker health check |
| `auth.providers` | — | `{ providers: [...] }` | Supported OAuth providers |
| `auth.accounts` / `auth.list` | `provider?` | `{ accounts: [...] }` | List authorized accounts |
| `auth.token` / `auth.get` | `provider, account, scopes?` | `{ access_token, ... }` | Get valid token (auto-refresh) |
| `auth.refresh` | `provider, account` | `{ access_token, ... }` | Force-refresh token |
| `auth.revoke` | `provider, account` | `{ success: true }` | Revoke & delete token |
| `auth.store` / `auth.import` | `provider, account, token_data` | `{ success: true }` | Import OAuth token |
| `auth.gogPassphrase` | — | `{ passphrase }` | Get gog keyring passphrase |
| `auth.credential_types` | — | `{ types: [...] }` | List credential schemas |
| `auth.credentials.store` | `credential_id, data` | `{ success: true }` | Store custom credential |
| `auth.credentials.get` | `credential_id` | `{ data }` | Retrieve credential |
| `auth.credentials.list` | `type?` | `{ credentials: [...] }` | List stored credentials |
| `auth.credentials.delete` | `credential_id` | `{ success: true }` | Delete credential |

**No macOS permissions required** — socket authorization suffices.

**Source:** Uses `AuthBroker` (`src-tauri/src/auth/`) for token storage, refresh, and retrieval.

---

### `permissions.*` — macOS Permissions

**Module:** `permissions.rs`  
**Purpose:** Check and request macOS Transparency, Consent, and Control (TCC) permissions.

| Method | Params | Returns | Description |
|--------|--------|---------|-------------|
| `permissions.check` | `permission` | `{ status, usageDescription? }` | Check single permission |
| `permissions.list` | — | `{ permissions: [...] }` | All permissions & their status |
| `permissions.request` | `permission` | `{ status }` | Request permission (triggers prompt) |

**Available permissions:**
- `contacts`, `calendar`, `reminders`
- `location`, `photos`, `camera`, `microphone`
- `screen_recording`, `accessibility`, `automation`
- `full_disk_access`, `bluetooth`

**Statuses:** `granted`, `denied`, `not_determined`, `restricted`, `unknown`

**No macOS permissions required** — this is the permission system itself.

---

### `contacts.*` — Contacts

**Module:** `contacts.rs`  
**Purpose:** Access macOS Contacts.framework (CNContactStore).

| Method | Params | Returns | Description |
|--------|--------|---------|-------------|
| `contacts.list` | `limit?`, `offset?` | `{ contacts: [...], total }` | List all contacts (paginated) |
| `contacts.search` | `query` | `{ contacts: [...] }` | Search by name |
| `contacts.get` | `id` | `{ contact }` | Get contact by CNContact ID |
| `contacts.create` | `firstName, lastName, emails?, phones?` | `{ id, contact }` | Create new contact |
| `contacts.update` | `id, firstName?, lastName?, emails?, phones?` | `{ contact }` | Update contact |
| `contacts.delete` | `id` | `{ success: true }` | Delete contact |

**Requires:** `contacts` permission  
**Implementation:** JXA scripts calling Contacts.app

**Contact schema:**
```json
{
  "id": "CNContact identifier",
  "firstName": "string",
  "lastName": "string",
  "emails": ["user@example.com"],
  "phones": ["+1-555-0100"]
}
```

---

### `calendar.*` — Local Calendar

**Module:** `calendar.rs`  
**Purpose:** Access macOS EventKit calendars & events.

| Method | Params | Returns | Description |
|--------|--------|---------|-------------|
| `calendar.list` | — | `{ calendars: [...] }` | List all calendars |
| `calendar.events` | `calendarId, start, end` | `{ events: [...] }` | List events in date range |
| `calendar.getEvent` | `calendarId, eventId` | `{ event }` | Get single event |
| `calendar.createEvent` | `calendarId, title, start, end, location?, notes?` | `{ eventId, event }` | Create event |
| `calendar.updateEvent` | `calendarId, eventId, title?, start?, end?, ...` | `{ event }` | Update event |
| `calendar.deleteEvent` | `calendarId, eventId` | `{ success: true }` | Delete event |

**Requires:** `calendar` permission  
**Implementation:** JXA scripts calling Calendar.app

---

### `reminders.*` — Reminders

**Module:** `reminders.rs`  
**Purpose:** Access macOS EventKit reminders.

| Method | Params | Returns | Description |
|--------|--------|---------|-------------|
| `reminders.lists` | — | `{ lists: [...] }` | List all reminder lists |
| `reminders.list` | `listId?` | `{ reminders: [...] }` | List reminders (all or specific list) |
| `reminders.create` | `listId, title, notes?, dueDate?` | `{ reminderId, reminder }` | Create reminder |
| `reminders.complete` | `reminderId` | `{ success: true }` | Mark complete |
| `reminders.uncomplete` | `reminderId` | `{ success: true }` | Mark incomplete |
| `reminders.delete` | `reminderId` | `{ success: true }` | Delete reminder |

**Requires:** `reminders` permission  
**Implementation:** JXA scripts calling Reminders.app

---

### `location.*` — Location Services

**Module:** `location.rs`  
**Purpose:** Get GPS/Wi-Fi location via CoreLocation.

| Method | Params | Returns | Description |
|--------|--------|---------|-------------|
| `location.get` | `accuracy?` | `{ latitude, longitude, accuracy, altitude?, ... }` | Current location |
| `location.watch` | — | *stub* | Future: location subscription |

**Requires:** `location` permission  
**Implementation:** Native Swift/Objective-C via CoreLocation (runs in-process on dedicated thread with run loop for delegate callbacks)

**Accuracy levels:** `best`, `nearest_ten_meters`, `hundred_meters`, `kilometer`, `three_kilometers`

---

### `screen.*` — Screen Capture

**Module:** `screen.rs`  
**Purpose:** Take screenshots, list windows.

| Method | Params | Returns | Description |
|--------|--------|---------|-------------|
| `screen.capture` | `display?, format?, path?` | `{ path, data }` | Screenshot (base64 + file path) |
| `screen.windows` | — | `{ windows: [...] }` | List visible windows |

**Requires:** `screen_recording` permission  
**Implementation:** Swift/CoreGraphics for screenshots, JXA for window list

---

### `files.*` — File System

**Module:** `files.rs`  
**Purpose:** Read and write files (with security restrictions).

| Method | Params | Returns | Description |
|--------|--------|---------|-------------|
| `files.read` | `path, encoding?, maxSize?` | `{ content }` | Read file (UTF-8 or base64) |
| `files.write` | `path, content, encoding?` | `{ success: true }` | Write file |
| `files.list` | `path, pattern?` | `{ files: [...] }` | List directory |
| `files.delete` | `path` | `{ success: true }` | Delete file |
| `files.exists` | `path` | `{ exists: bool }` | Check if path exists |

**Requires:** `full_disk_access` permission  
**Security:** Denies writes to:
- `/System/`
- `/Library/Keychains/`
- Tairseach's own auth store (`~/.local/share/tairseach/`)
- OpenClaw's keyring (`~/.local/share/openclaw/keyring/`)

**Limits:** 10 MB default max read size

---

### `automation.*` — AppleScript & UI Automation

**Module:** `automation.rs`  
**Purpose:** Execute AppleScript/JXA, control UI.

| Method | Params | Returns | Description |
|--------|--------|---------|-------------|
| `automation.run` | `script, language?, timeout?` | `{ output }` | Run AppleScript or JXA |
| `automation.click` | `x, y, button?, clicks?` | `{ success: true }` | Click at coordinates |
| `automation.type` | `text, delay?` | `{ success: true }` | Type text via accessibility |

**Requires:**
- `automation.run` → `automation` permission
- `automation.click` / `automation.type` → `accessibility` permission

**Languages:** `applescript` (default), `javascript` (JXA)  
**Timeout:** 30 seconds default

---

### `config.*` — Configuration

**Module:** `config.rs`  
**Purpose:** Access OpenClaw `config.json`.

| Method | Params | Returns | Description |
|--------|--------|---------|-------------|
| `config.get` | `section?` | `{ ... }` | Get full config or specific section |
| `config.set` | `config, replace?` | `{ success: true }` | Update config (merge or replace) |
| `config.environment` | — | `{ type: "gateway" | "node" }` | Detect runtime environment |
| `config.getNodeConfig` | — | `{ ... }` | Read `node.json` |

**No macOS permissions required**

**Paths:**
- Gateway: `~/.openclaw/gateway/config.json`
- Node: `~/.openclaw/node/node.json`

---

### `gmail.*` — Gmail API

**Module:** `gmail.rs`  
**Purpose:** Gmail API via OAuth (Google).

| Method | Params | Returns | Description |
|--------|--------|---------|-------------|
| `gmail.messages.list` | `provider, account, query?, maxResults?` | `{ messages: [...] }` | List messages |
| `gmail.messages.get` | `provider, account, id, format?` | `{ message }` | Get message by ID |
| `gmail.messages.send` | `provider, account, to, subject, body, ...` | `{ id, threadId }` | Send message |
| `gmail.labels.list` | `provider, account` | `{ labels: [...] }` | List labels |
| `gmail.messages.modify` | `provider, account, id, addLabels?, removeLabels?` | `{ message }` | Modify labels |
| `gmail.messages.trash` | `provider, account, id` | `{ success: true }` | Move to trash |
| `gmail.messages.delete` | `provider, account, id` | `{ success: true }` | Permanently delete |

**Requires:** OAuth token with Gmail scopes (`gmail.modify`, `gmail.settings.basic`)  
**Auth:** Retrieved from `AuthBroker` using `provider` & `account` params  
**Implementation:** Uses `GmailApi` client (`src-tauri/src/google/gmail.rs`)

---

### `gcalendar.*` — Google Calendar API

**Module:** `google_calendar.rs`  
**Purpose:** Google Calendar API via OAuth.

| Method | Params | Returns | Description |
|--------|--------|---------|-------------|
| `gcalendar.calendars.list` | `provider, account` | `{ calendars: [...] }` | List calendars |
| `gcalendar.events.list` | `provider, account, calendarId, timeMin?, timeMax?` | `{ events: [...] }` | List events |
| `gcalendar.events.get` | `provider, account, calendarId, eventId` | `{ event }` | Get event |
| `gcalendar.events.create` | `provider, account, calendarId, summary, start, end, ...` | `{ event }` | Create event |
| `gcalendar.events.update` | `provider, account, calendarId, eventId, ...` | `{ event }` | Update event |
| `gcalendar.events.delete` | `provider, account, calendarId, eventId` | `{ success: true }` | Delete event |

**Requires:** OAuth token with Calendar scopes  
**Implementation:** Uses `CalendarApi` client (`src-tauri/src/google/calendar.rs`)

---

### `op.*` / `onepassword.*` — 1Password

**Module:** `onepassword.rs`  
**Purpose:** Access 1Password via `op-helper` Go binary.

| Method | Params | Returns | Description |
|--------|--------|---------|-------------|
| `op.vaults.list` | `provider, account` | `{ vaults: [...] }` | List vaults |
| `op.items.list` | `provider, account, vault?` | `{ items: [...] }` | List items |
| `op.items.get` | `provider, account, item, vault?` | `{ item }` | Get item details |
| `op.secrets.resolve` | `provider, account, reference` | `{ value }` | Resolve secret reference |
| `op.vault.getDefault` | — | `{ vaultId }` | Get default vault |
| `op.vault.setDefault` | `vaultId` | `{ success: true }` | Set default vault |

**Requires:** Service Account token stored in `AuthBroker` under provider `onepassword` or `1password`  
**Implementation:** Calls `op-helper` binary (located at `../op-helper/target/release/op-helper`) with token + params

---

### `oura.*` — Oura Ring

**Module:** `oura.rs`  
**Purpose:** Oura Ring API.

| Method | Params | Returns | Description |
|--------|--------|---------|-------------|
| `oura.sleep` | `provider, account, start_date?, end_date?` | `{ data: [...] }` | Sleep data |
| `oura.activity` | `provider, account, start_date?, end_date?` | `{ data: [...] }` | Activity data |
| `oura.readiness` | `provider, account, start_date?, end_date?` | `{ data: [...] }` | Readiness scores |
| `oura.heart_rate` | `provider, account, start_date?, end_date?` | `{ data: [...] }` | Heart rate data |

**Requires:** Personal access token stored in `AuthBroker`  
**Implementation:** Uses `OuraApi` client

---

### `jira.*` — Jira

**Module:** `jira.rs`  
**Purpose:** Jira REST API.

| Method | Params | Returns | Description |
|--------|--------|---------|-------------|
| `jira.issues.search` | `provider, account, jql, maxResults?` | `{ issues: [...] }` | Search issues |
| `jira.issues.get` | `provider, account, issueKey` | `{ issue }` | Get issue details |
| `jira.issues.create` | `provider, account, project, summary, issueType, ...` | `{ key, id }` | Create issue |
| `jira.issues.update` | `provider, account, issueKey, fields` | `{ success: true }` | Update issue |
| `jira.issues.transition` | `provider, account, issueKey, transitionId, ...` | `{ success: true }` | Transition issue |
| `jira.projects.list` | `provider, account` | `{ projects: [...] }` | List projects |
| `jira.sprints.list` | `provider, account, boardId` | `{ sprints: [...] }` | List sprints |

**Requires:** OAuth token or API key stored in `AuthBroker`  
**Implementation:** Uses `JiraApi` client

---

### `server.*` — Server Control

**Built into `HandlerRegistry`**

| Method | Params | Returns | Description |
|--------|--------|---------|-------------|
| `server.status` | — | `{ status: "running", version }` | Health check |
| `server.shutdown` | — | `{ message }` | Graceful shutdown (stub) |

**No macOS permissions required**

---

## Handler Registration

Handlers are registered in `HandlerRegistry::handle()`:

```rust
match namespace {
    "auth" => auth::handle(action, &request.params, id).await,
    "permissions" => permissions::handle(action, &request.params, id).await,
    "contacts" => contacts::handle(action, &request.params, id).await,
    "calendar" => calendar::handle(action, &request.params, id).await,
    "reminders" => reminders::handle(action, &request.params, id).await,
    "location" => location::handle(action, &request.params, id).await,
    "screen" => screen::handle(action, &request.params, id).await,
    "files" => files::handle(action, &request.params, id).await,
    "automation" => automation::handle(action, &request.params, id).await,
    "config" => config::handle(action, &request.params, id).await,
    "gmail" => gmail::handle(action, &request.params, id).await,
    "gcalendar" => google_calendar::handle(action, &request.params, id).await,
    "op" | "onepassword" => onepassword::handle(action, &request.params, id).await,
    "oura" => oura::handle(action, &request.params, id).await,
    "jira" => jira::handle(action, &request.params, id).await,
    "server" => self.handle_server(action, &request.params, id).await,
    _ => JsonRpcResponse::method_not_found(id, &request.method),
}
```

**Routing precedence:**
1. **Manifest-based routing** (via `CapabilityRouter`) — if router is configured and method is registered in a manifest
2. **Legacy routing** — handler registry dispatch

This allows gradual migration to manifest-based tools while maintaining backward compatibility.

---

## Common Patterns

### OAuth Handler Pattern

Handlers that use OAuth (Gmail, Google Calendar, Jira, Oura) follow this pattern:

```rust
pub async fn handle(action: &str, params: &Value, id: Value) -> JsonRpcResponse {
    // 1. Get auth broker
    let auth_broker = match get_auth_broker().await {
        Ok(b) => b,
        Err(mut resp) => { resp.id = id; return resp; }
    };
    
    // 2. Extract provider/account from params
    let (provider, account) = match extract_oauth_credentials(params, "google") {
        Ok(creds) => creds,
        Err(mut resp) => { resp.id = id; return resp; }
    };
    
    // 3. Get token with required scopes
    let token_data = match auth_broker.get_token(&provider, &account, Some(&[...])).await {
        Ok(data) => data,
        Err((code, msg)) => return error(id, code, msg),
    };
    
    // 4. Create API client
    let api = ApiClient::new(token_data.access_token);
    
    // 5. Dispatch to action-specific handler
    match action {
        "list" => handle_list(params, id, api).await,
        ...
    }
}
```

### JXA Execution Pattern

Handlers for macOS system features (Contacts, Calendar, Reminders, Screen) use JXA:

```rust
async fn handle_list(params: &Value, id: Value) -> JsonRpcResponse {
    let script = r#"
        const app = Application("Contacts");
        app.includeStandardAdditions = true;
        const contacts = app.people();
        return JSON.stringify(contacts.map(c => ({
            id: c.id(),
            firstName: c.firstName(),
            lastName: c.lastName()
        })));
    "#;
    
    match execute_jxa(script).await {
        Ok(output) => {
            let data: Vec<Contact> = serde_json::from_str(&output).unwrap_or_default();
            ok(id, serde_json::json!({ "contacts": data }))
        }
        Err(e) => error(id, -32000, format!("JXA error: {}", e)),
    }
}
```

---

## Security Model

1. **Socket authentication** — only authorized Unix socket connections can call handlers
2. **Permission checks** — macOS TCC permissions enforced before dispatch
3. **Manifest-based authorization** — tools can declare required permissions/credentials
4. **File write restrictions** — critical paths denied in `files.write`
5. **Credential isolation** — OAuth tokens retrieved via `AuthBroker`, never exposed directly
6. **Timeout enforcement** — long-running operations (AppleScript, API calls) have timeouts

---

## Future Enhancements

- **Streaming responses** — for long-running operations (e.g., `location.watch`)
- **Batch operations** — bulk contact/calendar/file operations
- **Webhooks** — callbacks for async events
- **Native implementations** — replace JXA with direct framework calls where performance matters
- **Permission prompting** — automatic permission requests when `not_determined`

---

**See also:**
- [Router Architecture](./router.md) — manifest-based routing
- [Permissions System](./permissions.md) — TCC permission enforcement
- [Auth System](./auth-system.md) — OAuth token management
