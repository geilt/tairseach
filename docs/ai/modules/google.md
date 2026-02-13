# Google Module

> **Location:** `src-tauri/src/google/`  
> **Files:** 4  
> **Lines:** 737  
> **Purpose:** Google OAuth + Gmail + Calendar API integration

---

## Overview

Unified Google API client with OAuth token management, Gmail operations (list/get/send/modify messages), and Google Calendar operations (events CRUD, calendar list).

---

## File Listing

| File | Lines | Purpose |
|------|-------|---------|
| `mod.rs` | ~50 | Module exports |
| `client.rs` | ~190 | `GoogleOAuthClient` — OAuth flow orchestration |
| `gmail.rs` | ~260 | `GmailApi` — Gmail API methods |
| `calendar_api.rs` | ~240 | `CalendarApi` — Google Calendar API methods |

---

## Key Types

```rust
pub struct GoogleOAuthClient {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    http_client: reqwest::Client,
}

pub struct GmailApi {
    access_token: String,
    http_client: reqwest::Client,
}

pub struct CalendarApi {
    access_token: String,
    http_client: reqwest::Client,
}
```

---

## GoogleOAuthClient

```rust
impl GoogleOAuthClient {
    pub fn new(client_id: String, client_secret: String) -> Self
    
    pub fn auth_url(&self, scopes: &[String]) -> String
    
    pub async fn exchange_code(&self, code: &str) -> Result<TokenResponse, String>
    
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<TokenResponse, String>
}
```

**Usage:**
```rust
let oauth_client = GoogleOAuthClient::new(client_id, client_secret);

// Get auth URL
let url = oauth_client.auth_url(&[
    "https://www.googleapis.com/auth/gmail.readonly".to_string(),
]);

// Exchange authorization code for tokens
let tokens = oauth_client.exchange_code(auth_code).await?;
```

---

## GmailApi

```rust
impl GmailApi {
    pub fn new(access_token: String) -> Result<Self, String>
    
    pub async fn list_messages(
        &self,
        query: Option<&str>,
        max_results: Option<usize>,
        label_ids: Option<Vec<String>>,
    ) -> Result<Vec<Value>, String>
    
    pub async fn get_message(&self, message_id: &str) -> Result<Value, String>
    
    pub async fn send_message(&self, raw_message: &str) -> Result<Value, String>
    
    pub async fn modify_message(
        &self,
        message_id: &str,
        add_labels: Option<Vec<String>>,
        remove_labels: Option<Vec<String>>,
    ) -> Result<Value, String>
    
    pub async fn trash_message(&self, message_id: &str) -> Result<Value, String>
    
    pub async fn delete_message(&self, message_id: &str) -> Result<(), String>
    
    pub async fn list_labels(&self) -> Result<Vec<Value>, String>
}
```

**Usage:**
```rust
let gmail = GmailApi::new(access_token)?;

// List messages with query
let messages = gmail.list_messages(
    Some("is:unread"),
    Some(20),
    None,
).await?;

// Get full message
let msg = gmail.get_message("msg_id_123").await?;
```

---

## CalendarApi

```rust
impl CalendarApi {
    pub fn new(access_token: String) -> Result<Self, String>
    
    pub async fn list_calendars(&self) -> Result<Vec<Value>, String>
    
    pub async fn list_events(
        &self,
        calendar_id: &str,
        time_min: Option<&str>,
        time_max: Option<&str>,
        max_results: Option<usize>,
    ) -> Result<Vec<Value>, String>
    
    pub async fn get_event(
        &self,
        calendar_id: &str,
        event_id: &str,
    ) -> Result<Value, String>
    
    pub async fn create_event(
        &self,
        calendar_id: &str,
        event: Value,
    ) -> Result<Value, String>
    
    pub async fn update_event(
        &self,
        calendar_id: &str,
        event_id: &str,
        event: Value,
    ) -> Result<Value, String>
    
    pub async fn delete_event(
        &self,
        calendar_id: &str,
        event_id: &str,
    ) -> Result<(), String>
}
```

**Usage:**
```rust
let calendar = CalendarApi::new(access_token)?;

// List events
let events = calendar.list_events(
    "primary",
    Some("2024-01-01T00:00:00Z"),
    Some("2024-12-31T23:59:59Z"),
    Some(100),
).await?;

// Create event
let event = serde_json::json!({
    "summary": "Team Meeting",
    "start": { "dateTime": "2024-02-15T10:00:00-06:00" },
    "end": { "dateTime": "2024-02-15T11:00:00-06:00" },
});
let created = calendar.create_event("primary", event).await?;
```

---

## Shared HTTP Client

All Google APIs use `common/http::create_http_client()`:

```rust
use crate::common::http::create_http_client;

impl GmailApi {
    pub fn new(access_token: String) -> Result<Self, String> {
        Ok(Self {
            access_token,
            http_client: create_http_client()?,
        })
    }
}
```

---

## Dependencies

| Module | Imports |
|--------|---------|
| **auth** | Calls `GoogleOAuthClient` for token exchange/refresh |
| **handlers/gmail** | Uses `GmailApi` |
| **handlers/google_calendar** | Uses `CalendarApi` |
| **common/http** | `create_http_client()` |

---

*For handler usage, see [handlers.md](handlers.md)*
