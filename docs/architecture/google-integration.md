# Google Integration Architecture

**Location:** `src-tauri/src/google/`

Tairseach provides authenticated access to Google APIs (Gmail, Calendar, Drive) via OAuth 2.0. The integration uses **Tier 1 proxy mode** — OAuth tokens are managed by the `AuthBroker` and never exposed to MCP clients.

---

## Architecture Overview

```
MCP Client (Claude Desktop)
      ↓
tairseach-mcp (stdio bridge)
      ↓
Handler (gmail.rs / google_calendar.rs)
      ↓
AuthBroker.get_token("google", "me", scopes)
      ↓
GmailApi / CalendarApi
      ↓
GoogleClient (reqwest + Bearer token)
      ↓
Google API (REST)
```

**Security model:**
- OAuth tokens stored encrypted in `~/.tairseach/auth/tokens/`
- Tokens retrieved by handlers, never exposed to clients
- API calls made server-side (Tairseach process)
- Automatic token refresh when expired

---

## Components

### 1. GoogleClient — Authenticated HTTP Client

**File:** `src-tauri/src/google/client.rs`

Provides authenticated HTTP methods with automatic Bearer token injection.

```rust
pub struct GoogleClient {
    client: reqwest::Client,
    access_token: String,
}

impl GoogleClient {
    pub fn new(access_token: String) -> Result<Self, String>;
    
    pub async fn get(&self, url: &str, query: &[(&str, String)]) -> Result<Value, String>;
    pub async fn post(&self, url: &str, body: &Value) -> Result<Value, String>;
    pub async fn put(&self, url: &str, body: &Value) -> Result<Value, String>;
    pub async fn patch(&self, url: &str, body: &Value) -> Result<Value, String>;
    pub async fn delete(&self, url: &str) -> Result<Value, String>;
    
    pub async fn get_paginated(
        &self,
        url: &str,
        base_query: &[(&str, String)],
        max_results: Option<usize>,
    ) -> Result<Vec<Value>, String>;
}
```

**Features:**
- **Timeouts:** 30s request timeout, 10s connect timeout
- **Bearer auth:** Automatic `Authorization: Bearer <token>` header injection
- **Error handling:** Parses Google API error responses (code + message)
- **Rate limiting:** Detects `429 Too Many Requests` and returns user-friendly error
- **Pagination:** Automatic `nextPageToken` handling for list operations

**Example:**
```rust
let client = GoogleClient::new("ya29.a0...".to_string())?;
let messages = client.get_paginated(
    "https://gmail.googleapis.com/gmail/v1/users/me/messages",
    &[("q", "is:unread".to_string())],
    Some(50),
).await?;
```

---

### 2. GmailApi — Gmail v1 API Client

**File:** `src-tauri/src/google/gmail.rs`

**Base URL:** `https://gmail.googleapis.com/gmail/v1`

**Scopes:**
- `https://www.googleapis.com/auth/gmail.modify` — Read/write messages, labels
- `https://www.googleapis.com/auth/gmail.settings.basic` — Manage settings

#### Methods

| Method | Endpoint | Description |
|--------|----------|-------------|
| `list_messages(query, max_results, label_ids)` | `GET /users/me/messages` | Search messages |
| `get_message(id, format)` | `GET /users/me/messages/{id}` | Get message details |
| `send_message(to, subject, body, cc, bcc)` | `POST /users/me/messages/send` | Send email |
| `list_labels()` | `GET /users/me/labels` | List labels |
| `modify_message(id, add_labels, remove_labels)` | `POST /users/me/messages/{id}/modify` | Modify labels |
| `trash_message(id)` | `POST /users/me/messages/{id}/trash` | Move to trash |
| `delete_message(id)` | `DELETE /users/me/messages/{id}` | Permanently delete |

#### list_messages

```rust
pub async fn list_messages(
    &self,
    query: Option<&str>,
    max_results: Option<usize>,
    label_ids: Option<Vec<String>>,
) -> Result<Vec<Value>, String>
```

**Query syntax:** Same as Gmail web UI (e.g., `is:unread`, `from:user@example.com`, `subject:meeting`)

**Returns:**
```json
[
  {
    "id": "18d2a1b2c3d4e5f6",
    "threadId": "18d2a1b2c3d4e5f6"
  }
]
```

---

#### send_message

```rust
pub async fn send_message(
    &self,
    to: Vec<String>,
    subject: &str,
    body: &str,
    cc: Option<Vec<String>>,
    bcc: Option<Vec<String>>,
) -> Result<Value, String>
```

**Implementation:**
1. Builds RFC 2822 message (headers + body)
2. Base64url-encodes the message (no padding)
3. Sends as `POST /users/me/messages/send` with `{ "raw": "..." }`

**Example:**
```rust
let response = gmail.send_message(
    vec!["alice@example.com".to_string()],
    "Meeting reminder",
    "Don't forget our 2pm meeting!",
    None,  // cc
    None,  // bcc
).await?;
```

**Returns:**
```json
{
  "id": "18d2a1b2c3d4e5f7",
  "threadId": "18d2a1b2c3d4e5f6",
  "labelIds": ["SENT"]
}
```

---

### 3. CalendarApi — Google Calendar v3 API Client

**File:** `src-tauri/src/google/calendar_api.rs`

**Base URL:** `https://www.googleapis.com/calendar/v3`

**Scopes:**
- `https://www.googleapis.com/auth/calendar` — Full calendar access
- `https://www.googleapis.com/auth/calendar.readonly` — Read-only access

#### Methods

| Method | Endpoint | Description |
|--------|----------|-------------|
| `list_calendars()` | `GET /users/me/calendarList` | List calendars |
| `list_events(calendar_id, time_min, time_max, max_results)` | `GET /calendars/{id}/events` | List events |
| `get_event(calendar_id, event_id)` | `GET /calendars/{id}/events/{eventId}` | Get event |
| `create_event(calendar_id, event)` | `POST /calendars/{id}/events` | Create event |
| `update_event(calendar_id, event_id, event)` | `PUT /calendars/{id}/events/{eventId}` | Update event |
| `delete_event(calendar_id, event_id)` | `DELETE /calendars/{id}/events/{eventId}` | Delete event |

#### list_events

```rust
pub async fn list_events(
    &self,
    calendar_id: &str,
    time_min: Option<&str>,
    time_max: Option<&str>,
    max_results: Option<usize>,
) -> Result<Vec<Value>, String>
```

**Time format:** RFC 3339 (e.g., `2024-01-15T09:00:00-08:00`)

**Returns:**
```json
[
  {
    "id": "abc123",
    "summary": "Team standup",
    "start": { "dateTime": "2024-01-15T09:00:00-08:00" },
    "end": { "dateTime": "2024-01-15T09:30:00-08:00" },
    "location": "Zoom",
    "description": "Daily team sync"
  }
]
```

---

#### create_event

```rust
pub async fn create_event(
    &self,
    calendar_id: &str,
    event: Value,
) -> Result<Value, String>
```

**Event schema:**
```json
{
  "summary": "Meeting title",
  "description": "Meeting notes",
  "location": "Conference Room A",
  "start": {
    "dateTime": "2024-01-15T14:00:00-08:00",
    "timeZone": "America/Los_Angeles"
  },
  "end": {
    "dateTime": "2024-01-15T15:00:00-08:00",
    "timeZone": "America/Los_Angeles"
  },
  "attendees": [
    { "email": "alice@example.com" },
    { "email": "bob@example.com" }
  ],
  "reminders": {
    "useDefault": false,
    "overrides": [
      { "method": "email", "minutes": 60 },
      { "method": "popup", "minutes": 10 }
    ]
  }
}
```

**Returns:** Full event object with Google-assigned `id`

---

### 4. OAuth Flow

**Handled by:** `AuthBroker` (see [Auth System](./auth-system.md))

#### Authorization

**NOT implemented in Tairseach** — OAuth tokens must be obtained externally and imported via `auth.store`.

**Future:** PKCE flow with localhost callback server

**Current workflow:**
1. User authorizes via Google OAuth Playground or custom script
2. Import token via socket:
   ```json
   {
     "jsonrpc": "2.0",
     "id": 1,
     "method": "auth.store",
     "params": {
       "provider": "google",
       "account": "me",
       "tokenData": {
         "access_token": "ya29.a0...",
         "refresh_token": "1//0e...",
         "expires_in": 3600,
         "scope": "https://www.googleapis.com/auth/gmail.modify https://www.googleapis.com/auth/calendar",
         "token_type": "Bearer"
       }
     }
   }
   ```

---

#### Token Refresh

**Automatic refresh** when token is expired (based on `expires_in` timestamp).

**Refresh endpoint:** `https://oauth2.googleapis.com/token`

**Request:**
```http
POST https://oauth2.googleapis.com/token
Content-Type: application/x-www-form-urlencoded

client_id={CLIENT_ID}
&client_secret={CLIENT_SECRET}
&refresh_token={REFRESH_TOKEN}
&grant_type=refresh_token
```

**Response:**
```json
{
  "access_token": "ya29.a0...",
  "expires_in": 3600,
  "scope": "...",
  "token_type": "Bearer"
}
```

**Implementation:** `AuthBroker` checks expiry before returning token to handlers; refreshes if needed.

---

### 5. Handler Integration

**Files:**
- `src-tauri/src/proxy/handlers/gmail.rs` — Gmail handler
- `src-tauri/src/proxy/handlers/google_calendar.rs` — Calendar handler

#### Common Pattern

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
    let token_data = match auth_broker.get_token(
        &provider,
        &account,
        Some(&["https://www.googleapis.com/auth/gmail.modify".to_string()]),
    ).await {
        Ok(data) => data,
        Err((code, msg)) => return error(id, code, msg),
    };
    
    // 4. Create API client
    let api = GmailApi::new(token_data.access_token)?;
    
    // 5. Dispatch to action-specific handler
    match action {
        "messages.list" => handle_list_messages(params, id, api).await,
        "messages.send" => handle_send_message(params, id, api).await,
        // ...
    }
}
```

**Key points:**
- Handler never sees raw token (only `token_data.access_token` string)
- Token automatically refreshed by `AuthBroker` if expired
- Errors propagated as JSON-RPC errors

---

## Error Handling

### Google API Errors

**Format:**
```json
{
  "error": {
    "code": 400,
    "message": "Invalid request",
    "errors": [
      {
        "domain": "global",
        "reason": "invalid",
        "message": "Invalid request"
      }
    ]
  }
}
```

**GoogleClient translation:**
```rust
fn extract_error_message(&self, response: &Value, status: StatusCode) -> String {
    if let Some(error_obj) = response.get("error") {
        if let Some(message) = error_obj.get("message").and_then(|v| v.as_str()) {
            let code = error_obj.get("code").and_then(|v| v.as_i64()).unwrap_or(status.as_u16() as i64);
            return format!("Google API error {}: {}", code, message);
        }
    }
    format!("HTTP {} error", status)
}
```

---

### Common Errors

| Code | Reason | Meaning |
|------|--------|---------|
| 400 | Bad Request | Invalid parameters (check input schema) |
| 401 | Unauthorized | Invalid or expired token (should not happen if refresh works) |
| 403 | Forbidden | Insufficient scopes or permission denied |
| 404 | Not Found | Resource doesn't exist (wrong message ID, calendar ID, etc.) |
| 429 | Too Many Requests | Rate limit exceeded |
| 500 | Internal Server Error | Google API issue (retry) |

---

### Handler Error Responses

**Example:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32000,
    "message": "Google API error 403: Insufficient Permission: Request had insufficient authentication scopes."
  }
}
```

**Error code mapping:**
- `-32011` — Auth error (missing/invalid token)
- `-32000` — Upstream API error (Google API error)
- `-32602` — Invalid params (missing required fields)

---

## Pagination

Google APIs use **cursor-based pagination** with `nextPageToken`.

**Pattern:**
1. Make initial request (no `pageToken`)
2. If response has `nextPageToken`, make another request with `pageToken={token}`
3. Repeat until no `nextPageToken` in response

**GoogleClient.get_paginated() implementation:**
```rust
pub async fn get_paginated(
    &self,
    url: &str,
    base_query: &[(&str, String)],
    max_results: Option<usize>,
) -> Result<Vec<Value>, String> {
    let mut all_items = Vec::new();
    let mut page_token: Option<String> = None;
    
    loop {
        let mut query = base_query.to_vec();
        if let Some(ref token) = page_token {
            query.push(("pageToken", token.clone()));
        }
        
        let response = self.get(url, &query).await?;
        
        // Extract items (field name varies: items, messages, events)
        if let Some(items) = response.get("items")
            .or_else(|| response.get("messages"))
            .or_else(|| response.get("events"))
            .and_then(|v| v.as_array())
        {
            all_items.extend(items.clone());
            
            if all_items.len() >= max_results.unwrap_or(usize::MAX) {
                all_items.truncate(max_results.unwrap());
                break;
            }
        }
        
        // Check for next page
        page_token = response.get("nextPageToken").and_then(|v| v.as_str()).map(String::from);
        if page_token.is_none() {
            break;
        }
    }
    
    Ok(all_items)
}
```

**Limits:**
- Gmail: default 100 messages per page
- Calendar: default 250 events per page
- Max results: configurable via handler params

---

## Rate Limiting

**Google API rate limits:**
- **Gmail:** 250 quota units/user/second
- **Calendar:** 60 requests/minute/user

**Quota costs:**
- List messages: 5 units
- Get message: 5 units
- Send message: 100 units

**Handling:**
- GoogleClient detects `429 Too Many Requests`
- Returns user-friendly error: `"Rate limited. Please try again later."`
- Future: Implement exponential backoff

---

## Security Considerations

### Token Storage

- Tokens stored encrypted at rest (AES-256-GCM)
- Encryption key derived from master passphrase (PBKDF2)
- File permissions: `0600` (owner read/write only)

### Token Exposure

- Tokens NEVER sent to MCP clients
- API calls made server-side (Tairseach process)
- Only response data returned to client

### Scope Minimization

Handlers request **only required scopes**:

```rust
auth_broker.get_token(
    "google",
    "me",
    Some(&["https://www.googleapis.com/auth/gmail.readonly".to_string()]),
).await
```

If stored token has insufficient scopes, error is returned (user must re-authorize).

---

## Future Enhancements

### Planned Features

1. **PKCE OAuth Flow**
   - Implement authorization code flow with PKCE
   - Localhost callback server for token exchange
   - User-friendly authorization via browser

2. **Drive API**
   - File upload/download
   - Folder management
   - Sharing permissions

3. **People API**
   - Contact sync (Google Contacts ↔ macOS Contacts)
   - Profile information

4. **Admin SDK**
   - User/group management (for Google Workspace admins)
   - Audit logs

5. **Batch Requests**
   - Combine multiple API calls into single HTTP request
   - Reduce latency for bulk operations

6. **Webhooks**
   - Gmail push notifications via Pub/Sub
   - Calendar change notifications

### Performance Optimizations

- **Connection pooling:** Reuse HTTP connections
- **Request batching:** Group multiple operations
- **Caching:** Cache calendar lists, labels (with TTL)
- **Exponential backoff:** Automatic retry on 429/500 errors

---

## Example Usage

### Send Gmail Message

**MCP tool call:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "tairseach_gmail_send",
    "arguments": {
      "provider": "google",
      "account": "me",
      "to": ["alice@example.com"],
      "subject": "Project update",
      "body": "The feature is ready for review."
    }
  }
}
```

**Socket call (from MCP bridge):**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "gmail.messages.send",
  "params": {
    "provider": "google",
    "account": "me",
    "to": ["alice@example.com"],
    "subject": "Project update",
    "body": "The feature is ready for review."
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "id": "18d2a1b2c3d4e5f7",
    "threadId": "18d2a1b2c3d4e5f6",
    "labelIds": ["SENT"]
  }
}
```

---

### Create Calendar Event

**Socket call:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "gcalendar.events.create",
  "params": {
    "provider": "google",
    "account": "work",
    "calendarId": "primary",
    "summary": "Team standup",
    "start": "2024-01-15T09:00:00-08:00",
    "end": "2024-01-15T09:30:00-08:00",
    "location": "Zoom"
  }
}
```

**Handler transforms to Google Calendar API:**
```json
{
  "summary": "Team standup",
  "start": { "dateTime": "2024-01-15T09:00:00-08:00" },
  "end": { "dateTime": "2024-01-15T09:30:00-08:00" },
  "location": "Zoom"
}
```

**Google API response:**
```json
{
  "id": "abc123",
  "summary": "Team standup",
  "start": { "dateTime": "2024-01-15T09:00:00-08:00" },
  "end": { "dateTime": "2024-01-15T09:30:00-08:00" },
  "location": "Zoom",
  "htmlLink": "https://www.google.com/calendar/event?eid=..."
}
```

---

## References

- [Gmail API Documentation](https://developers.google.com/gmail/api)
- [Google Calendar API Documentation](https://developers.google.com/calendar/api)
- [OAuth 2.0 for Installed Applications](https://developers.google.com/identity/protocols/oauth2/native-app)
- [Google API Error Codes](https://developers.google.com/gmail/api/guides/handle-errors)
- [RFC 2822: Internet Message Format](https://www.rfc-editor.org/rfc/rfc2822)

---

**See also:**
- [Auth System](./auth-system.md) — OAuth token management
- [Handlers](./handlers.md) — Gmail/Calendar handler implementations
- [Router](./router.md) — Manifest-based routing
