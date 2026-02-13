# Handler Reference

**Complete catalog of all JSON-RPC handler methods.**

This document lists **every handler method**, its parameters, response structure, and examples.

---

## Table of Contents

- [server.*](#server) — Server status and control
- [auth.*](#auth) — OAuth token broker and credentials
- [permissions.*](#permissions) — macOS permission checks
- [contacts.*](#contacts) — Native Contacts.app access
- [calendar.*](#calendar) — Native Calendar.app access
- [reminders.*](#reminders) — Native Reminders.app access
- [location.*](#location) — Core Location access
- [screen.*](#screen) — Screen recording
- [files.*](#files) — File system access
- [automation.*](#automation) — AppleScript/Automator execution
- [config.*](#config) — Server configuration
- [gmail.*](#gmail) — Gmail API (via OAuth)
- [gcalendar.*](#gcalendar) — Google Calendar API
- [op.* / onepassword.*](#onepassword) — 1Password integration
- [oura.*](#oura) — Oura Ring API
- [jira.*](#jira) — Jira Cloud API

---

## server.*

### `server.status`

Get server status and version information.

**Params:** (none)

**Response:**
```json
{
  "status": "running",
  "version": "0.1.0"
}
```

**Example:**
```json
{"jsonrpc":"2.0","id":1,"method":"server.status","params":{}}
```

### `server.shutdown`

Initiate graceful server shutdown.

**Params:** (none)

**Response:**
```json
{
  "message": "Shutdown initiated"
}
```

**Example:**
```json
{"jsonrpc":"2.0","id":2,"method":"server.shutdown","params":{}}
```

**Note:** This method is not exposed via MCP (`mcp_expose: false`).

---

## auth.*

Authentication and credential management methods.

### `auth.status`

Get auth subsystem status.

**Params:** (none)

**Response:**
```json
{
  "initialized": true,
  "providers": ["google"],
  "accounts": 3
}
```

**Example:**
```json
{"jsonrpc":"2.0","id":1,"method":"auth.status","params":{}}
```

### `auth.providers`

List supported OAuth providers.

**Params:** (none)

**Response:**
```json
{
  "providers": ["google"]
}
```

**Example:**
```json
{"jsonrpc":"2.0","id":2,"method":"auth.providers","params":{}}
```

### `auth.accounts` / `auth.list`

List authorized accounts (optionally filtered by provider).

**Params:**
- `provider` (string, optional) — Filter by provider type

**Response:**
```json
{
  "accounts": [
    {
      "provider": "google",
      "account": "user@example.com",
      "scopes": ["https://www.googleapis.com/auth/gmail.modify"],
      "expiry": "2025-02-14T12:00:00Z"
    }
  ],
  "count": 1
}
```

**Example:**
```json
{"jsonrpc":"2.0","id":3,"method":"auth.accounts","params":{"provider":"google"}}
```

### `auth.token` / `auth.get`

Retrieve a valid access token (auto-refreshes if expired).

**Params:**
- `provider` (string, required) — Provider type (e.g., "google")
- `account` (string, required) — Account identifier (e.g., email)
- `scopes` (array of strings, optional) — Requested scopes

**Response:**
```json
{
  "access_token": "ya29.a0...",
  "token_type": "Bearer",
  "expiry": "2025-02-13T12:30:00Z",
  "scopes": ["https://www.googleapis.com/auth/gmail.modify"]
}
```

**Example:**
```json
{
  "jsonrpc":"2.0",
  "id":4,
  "method":"auth.token",
  "params":{
    "provider":"google",
    "account":"user@example.com",
    "scopes":["https://www.googleapis.com/auth/gmail.readonly"]
  }
}
```

**Note:** Sensitive — not exposed via MCP (`mcp_expose: false`).

### `auth.refresh`

Force-refresh a token (ignoring cache).

**Params:**
- `provider` (string, required)
- `account` (string, required)

**Response:** (same as `auth.token`)

**Example:**
```json
{"jsonrpc":"2.0","id":5,"method":"auth.refresh","params":{"provider":"google","account":"user@example.com"}}
```

### `auth.revoke`

Revoke and remove an account's tokens.

**Params:**
- `provider` (string, required)
- `account` (string, required)

**Response:**
```json
{
  "success": true
}
```

**Example:**
```json
{"jsonrpc":"2.0","id":6,"method":"auth.revoke","params":{"provider":"google","account":"user@example.com"}}
```

**Annotations:** `destructiveHint: true`

### `auth.store` / `auth.import`

Store or import a token record directly (bypassing OAuth flow).

**Params:**
- `provider` (string, required) — Provider type (must be in `["google"]`)
- `account` (string, required) — Account identifier (max 256 chars)
- `access_token` (string, required)
- `refresh_token` (string, required)
- `token_type` (string, required) — e.g., "Bearer"
- `expiry` (string, required) — ISO 8601 timestamp
- `scopes` (array of strings, required)

**Response:**
```json
{
  "success": true
}
```

**Example:**
```json
{
  "jsonrpc":"2.0",
  "id":7,
  "method":"auth.store",
  "params":{
    "provider":"google",
    "account":"user@example.com",
    "access_token":"ya29.a0...",
    "refresh_token":"1//0gH...",
    "token_type":"Bearer",
    "expiry":"2025-02-14T00:00:00Z",
    "scopes":["https://www.googleapis.com/auth/gmail.modify"]
  }
}
```

**Validation:**
- Provider must be in supported list (currently: `["google"]`)
- Provider `"_internal"` is reserved
- Account must not be empty and must be ≤ 256 chars
- Access token must not be empty

### `auth.gogPassphrase`

Retrieve the gog file-keyring passphrase (for OpenClaw integration).

**Params:** (none)

**Response:**
```json
{
  "passphrase": "..."
}
```

**Example:**
```json
{"jsonrpc":"2.0","id":8,"method":"auth.gogPassphrase","params":{}}
```

**Note:** Sensitive — not exposed via MCP.

### `auth.credential_types` / `auth.credentialTypes`

List all known credential schemas (built-in + custom).

**Params:** (none)

**Response:**
```json
{
  "types": [
    {
      "provider_type": "onepassword",
      "display_name": "1Password Service Account",
      "description": "1Password Service Account token for API access",
      "fields": [
        {
          "name": "service_account_token",
          "display_name": "Service Account Token",
          "type": "secret",
          "required": true,
          "description": "Service account token from 1Password"
        }
      ],
      "supports_multiple": true,
      "built_in": true
    }
  ]
}
```

**Example:**
```json
{"jsonrpc":"2.0","id":9,"method":"auth.credential_types","params":{}}
```

### `auth.credential_types.custom.create`

Register a custom credential type.

**Params:** (CredentialTypeSchema object)
- `provider_type` (string, required) — Unique type identifier
- `display_name` (string, required)
- `description` (string, required)
- `fields` (array, required) — Array of field definitions
- `supports_multiple` (boolean, required)

**Field definition:**
- `name` (string)
- `display_name` (string)
- `type` (string) — "string" or "secret"
- `required` (boolean)
- `description` (string, optional)

**Response:**
```json
{
  "success": true
}
```

**Example:**
```json
{
  "jsonrpc":"2.0",
  "id":10,
  "method":"auth.credential_types.custom.create",
  "params":{
    "provider_type":"my_api",
    "display_name":"My API",
    "description":"Custom API credentials",
    "fields":[
      {
        "name":"api_key",
        "display_name":"API Key",
        "type":"secret",
        "required":true
      }
    ],
    "supports_multiple":false
  }
}
```

### `auth.credentials.store`

Store a credential.

**Params:**
- `provider` (string, required) — Provider identifier
- `type` (string, required) — Credential type
- `fields` (object, required) — Key-value map of credential fields
- `label` (string, optional) — Account label (default: "default")

**Response:**
```json
{
  "success": true
}
```

**Example:**
```json
{
  "jsonrpc":"2.0",
  "id":11,
  "method":"auth.credentials.store",
  "params":{
    "provider":"onepassword",
    "type":"onepassword",
    "label":"work",
    "fields":{
      "service_account_token":"ops_..."
    }
  }
}
```

### `auth.credentials.get`

Retrieve a credential (uses resolution chain: label → account → default).

**Params:**
- `provider` (string, required)
- `label` (string, optional)

**Response:**
```json
{
  "fields": {
    "service_account_token": "ops_..."
  }
}
```

**Example:**
```json
{"jsonrpc":"2.0","id":12,"method":"auth.credentials.get","params":{"provider":"onepassword","label":"work"}}
```

### `auth.credentials.list`

List all credentials (metadata only, no secrets).

**Params:** (none)

**Response:**
```json
{
  "credentials": [
    {
      "provider": "onepassword",
      "account": "work",
      "type": "onepassword",
      "label": "work"
    }
  ],
  "count": 1
}
```

**Example:**
```json
{"jsonrpc":"2.0","id":13,"method":"auth.credentials.list","params":{}}
```

### `auth.credentials.delete`

Delete a credential.

**Params:**
- `provider` (string, required)
- `label` (string, optional) — Account label (default: "default")

**Response:**
```json
{
  "success": true
}
```

**Example:**
```json
{"jsonrpc":"2.0","id":14,"method":"auth.credentials.delete","params":{"provider":"onepassword","label":"work"}}
```

---

## permissions.*

### `permissions.check`

Check the status of a specific macOS permission.

**Params:**
- `permission` (string, required) — Permission ID (see below)

**Valid permission IDs:**
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

**Response:**
```json
{
  "permission": "contacts",
  "status": "granted",
  "granted": true
}
```

**Status values:**
- `"granted"` — Permission granted
- `"denied"` — Permission denied
- `"not_determined"` — User hasn't been prompted yet
- `"restricted"` — Permission restricted (parental controls, etc.)
- `"unknown"` — Status could not be determined

**Example:**
```json
{"jsonrpc":"2.0","id":1,"method":"permissions.check","params":{"permission":"contacts"}}
```

### `permissions.list`

List all permissions and their current status.

**Params:** (none)

**Response:**
```json
{
  "permissions": [
    {
      "permission": "contacts",
      "status": "granted",
      "granted": true
    },
    {
      "permission": "calendar",
      "status": "not_determined",
      "granted": false
    }
  ],
  "total": 11
}
```

**Example:**
```json
{"jsonrpc":"2.0","id":2,"method":"permissions.list","params":{}}
```

### `permissions.request`

Request a permission (triggers UI prompt or opens System Preferences).

**Params:**
- `permission` (string, required) — Permission ID

**Response:**
```json
{
  "permission": "contacts",
  "action": "request_initiated",
  "message": "Permission request has been initiated. Check app UI for prompt."
}
```

**Example:**
```json
{"jsonrpc":"2.0","id":3,"method":"permissions.request","params":{"permission":"contacts"}}
```

---

## contacts.*

**Requires permission:** `contacts`

### `contacts.list`

List all contacts with optional pagination.

**Params:**
- `limit` (number, optional) — Max results (default: 100)
- `offset` (number, optional) — Offset for pagination (default: 0)

**Response:**
```json
{
  "contacts": [
    {
      "id": "ABC123",
      "firstName": "John",
      "lastName": "Doe",
      "fullName": "John Doe",
      "emails": ["john@example.com"],
      "phones": ["+1234567890"],
      "organization": "Acme Corp"
    }
  ],
  "count": 1,
  "limit": 100,
  "offset": 0
}
```

**Example:**
```json
{"jsonrpc":"2.0","id":1,"method":"contacts.list","params":{"limit":50,"offset":0}}
```

### `contacts.search`

Search contacts by name.

**Params:**
- `query` (string, required) — Search query (matches first/last name)
- `limit` (number, optional) — Max results (default: 50)

**Response:**
```json
{
  "query": "john",
  "contacts": [...],
  "count": 3
}
```

**Example:**
```json
{"jsonrpc":"2.0","id":2,"method":"contacts.search","params":{"query":"john","limit":10}}
```

### `contacts.get`

Get a specific contact by ID.

**Params:**
- `id` (string, required) — Contact ID

**Response:**
```json
{
  "id": "ABC123",
  "firstName": "John",
  "lastName": "Doe",
  "fullName": "John Doe",
  "emails": ["john@example.com"],
  "phones": ["+1234567890"],
  "organization": "Acme Corp"
}
```

**Error:** `-32002` if contact not found

**Example:**
```json
{"jsonrpc":"2.0","id":3,"method":"contacts.get","params":{"id":"ABC123"}}
```

### `contacts.create`

Create a new contact.

**Params:**
- `firstName` (string, optional)
- `lastName` (string, optional)
- `organization` (string, optional)
- `emails` (array of strings, optional)
- `phones` (array of strings, optional)

**At least one** of `firstName`, `lastName`, or `organization` is required.

**Response:**
```json
{
  "created": true,
  "contact": {
    "id": "XYZ789",
    "firstName": "Jane",
    "lastName": "Smith",
    "fullName": "Jane Smith",
    "emails": ["jane@example.com"],
    "phones": [],
    "organization": null
  }
}
```

**Example:**
```json
{
  "jsonrpc":"2.0",
  "id":4,
  "method":"contacts.create",
  "params":{
    "firstName":"Jane",
    "lastName":"Smith",
    "emails":["jane@example.com"]
  }
}
```

### `contacts.update`

Update an existing contact.

**Params:**
- `id` (string, required) — Contact ID
- `firstName` (string, optional)
- `lastName` (string, optional)
- `organization` (string, optional)
- `emails` (array of strings, optional)
- `phones` (array of strings, optional)

**Response:**
```json
{
  "updated": true,
  "contact": {
    "id": "ABC123",
    "firstName": "John",
    "lastName": "Doe Updated",
    "fullName": "John Doe Updated",
    "emails": ["newemail@example.com"],
    "phones": [],
    "organization": "New Corp"
  }
}
```

**Example:**
```json
{
  "jsonrpc":"2.0",
  "id":5,
  "method":"contacts.update",
  "params":{
    "id":"ABC123",
    "organization":"New Corp"
  }
}
```

### `contacts.delete`

Delete a contact.

**Params:**
- `id` (string, required) — Contact ID

**Response:**
```json
{
  "deleted": true,
  "id": "ABC123"
}
```

**Example:**
```json
{"jsonrpc":"2.0","id":6,"method":"contacts.delete","params":{"id":"ABC123"}}
```

---

## calendar.*

**Requires permission:** `calendar`

### `calendar.list`

List all calendars.

**Params:** (none)

**Response:**
```json
{
  "calendars": [
    {
      "id": "CAL-123",
      "title": "Personal",
      "type": "Local",
      "color": "#FF0000",
      "isEditable": true
    }
  ],
  "count": 1
}
```

**Example:**
```json
{"jsonrpc":"2.0","id":1,"method":"calendar.list","params":{}}
```

### `calendar.events`

List events in a date range.

**Params:**
- `start` (string, required) — Start date (ISO 8601 format)
- `end` (string, required) — End date (ISO 8601 format)
- `calendarId` (string, optional) — Filter by calendar ID

**Response:**
```json
{
  "events": [
    {
      "id": "EVT-456",
      "title": "Team Meeting",
      "calendarId": "CAL-123",
      "startDate": "2025-02-14T10:00:00Z",
      "endDate": "2025-02-14T11:00:00Z",
      "isAllDay": false,
      "location": "Conference Room A",
      "notes": "Discuss Q1 goals"
    }
  ],
  "count": 1,
  "start": "2025-02-14T00:00:00Z",
  "end": "2025-02-15T00:00:00Z"
}
```

**Example:**
```json
{
  "jsonrpc":"2.0",
  "id":2,
  "method":"calendar.events",
  "params":{
    "start":"2025-02-14T00:00:00Z",
    "end":"2025-02-21T00:00:00Z",
    "calendarId":"CAL-123"
  }
}
```

### `calendar.getEvent`

Get a specific event by ID.

**Params:**
- `id` (string, required) — Event ID

**Response:**
```json
{
  "id": "EVT-456",
  "title": "Team Meeting",
  "calendarId": "CAL-123",
  "startDate": "2025-02-14T10:00:00Z",
  "endDate": "2025-02-14T11:00:00Z",
  "isAllDay": false,
  "location": "Conference Room A",
  "notes": "Discuss Q1 goals"
}
```

**Error:** `-32002` if event not found

**Example:**
```json
{"jsonrpc":"2.0","id":3,"method":"calendar.getEvent","params":{"id":"EVT-456"}}
```

### `calendar.createEvent`

Create a new calendar event.

**Params:**
- `title` (string, required)
- `start` (string, required) — ISO 8601 timestamp
- `end` (string, required) — ISO 8601 timestamp
- `calendarId` (string, optional) — Calendar to add to (default calendar if omitted)
- `location` (string, optional)
- `notes` (string, optional)
- `isAllDay` (boolean, optional) — Default: false

**Response:**
```json
{
  "created": true,
  "event": {
    "id": "EVT-789",
    "title": "Lunch with Client",
    "calendarId": "CAL-123",
    "startDate": "2025-02-15T12:00:00Z",
    "endDate": "2025-02-15T13:00:00Z",
    "isAllDay": false,
    "location": "Downtown Bistro",
    "notes": null
  }
}
```

**Example:**
```json
{
  "jsonrpc":"2.0",
  "id":4,
  "method":"calendar.createEvent",
  "params":{
    "title":"Lunch with Client",
    "start":"2025-02-15T12:00:00Z",
    "end":"2025-02-15T13:00:00Z",
    "location":"Downtown Bistro"
  }
}
```

### `calendar.updateEvent`

Update an existing event.

**Params:**
- `id` (string, required) — Event ID
- `title` (string, optional)
- `start` (string, optional)
- `end` (string, optional)
- `location` (string, optional)
- `notes` (string, optional)
- `isAllDay` (boolean, optional)

**Response:**
```json
{
  "updated": true,
  "event": {...}
}
```

**Example:**
```json
{
  "jsonrpc":"2.0",
  "id":5,
  "method":"calendar.updateEvent",
  "params":{
    "id":"EVT-456",
    "location":"New Location"
  }
}
```

### `calendar.deleteEvent`

Delete a calendar event.

**Params:**
- `id` (string, required) — Event ID

**Response:**
```json
{
  "deleted": true,
  "id": "EVT-456"
}
```

**Example:**
```json
{"jsonrpc":"2.0","id":6,"method":"calendar.deleteEvent","params":{"id":"EVT-456"}}
```

---

## reminders.*

**Requires permission:** `reminders`

### `reminders.lists`

List all reminder lists.

**Params:** (none)

**Response:**
```json
{
  "lists": [
    {
      "id": "LIST-123",
      "title": "Personal"
    }
  ],
  "count": 1
}
```

### `reminders.list`

List reminders in a specific list.

**Params:**
- `listId` (string, optional) — List ID (default list if omitted)

**Response:**
```json
{
  "reminders": [
    {
      "id": "REM-456",
      "title": "Buy groceries",
      "completed": false,
      "listId": "LIST-123"
    }
  ],
  "count": 1
}
```

### `reminders.create`

Create a new reminder.

**Params:**
- `title` (string, required)
- `listId` (string, optional)
- `dueDate` (string, optional) — ISO 8601 timestamp

**Response:**
```json
{
  "created": true,
  "reminder": {
    "id": "REM-789",
    "title": "Call dentist",
    "completed": false,
    "listId": "LIST-123"
  }
}
```

### `reminders.complete`

Mark a reminder as completed.

**Params:**
- `id` (string, required) — Reminder ID

**Response:**
```json
{
  "completed": true,
  "id": "REM-456"
}
```

### `reminders.delete`

Delete a reminder.

**Params:**
- `id` (string, required)

**Response:**
```json
{
  "deleted": true,
  "id": "REM-456"
}
```

---

## location.*

**Requires permission:** `location`

### `location.get`

Get current device location.

**Params:** (none)

**Response:**
```json
{
  "latitude": 37.7749,
  "longitude": -122.4194,
  "altitude": 10.5,
  "accuracy": 5.0,
  "timestamp": "2025-02-13T12:00:00Z"
}
```

### `location.watch`

Start watching location updates (placeholder — not yet implemented).

---

## screen.*

**Requires permission:** `screen_recording`

### `screen.capture`

Capture a screenshot.

**Params:**
- `display` (number, optional) — Display index (default: 0)

**Response:**
```json
{
  "success": true,
  "path": "/tmp/screenshot-123.png"
}
```

### `screen.windows`

List all visible windows.

**Params:** (none)

**Response:**
```json
{
  "windows": [
    {
      "title": "Safari - Tairseach",
      "owner": "Safari",
      "id": 12345
    }
  ],
  "count": 1
}
```

---

## files.*

**Requires permission:** `full_disk_access`

### `files.read`

Read a file's contents.

**Params:**
- `path` (string, required) — Absolute file path

**Response:**
```json
{
  "path": "/Users/user/test.txt",
  "contents": "file contents here"
}
```

### `files.write`

Write contents to a file.

**Params:**
- `path` (string, required)
- `contents` (string, required)

**Response:**
```json
{
  "success": true,
  "path": "/Users/user/test.txt"
}
```

### `files.list`

List directory contents.

**Params:**
- `path` (string, required) — Directory path

**Response:**
```json
{
  "path": "/Users/user",
  "entries": [
    {"name": "Documents", "type": "directory"},
    {"name": "test.txt", "type": "file", "size": 1024}
  ],
  "count": 2
}
```

---

## automation.*

**Requires permission:** `automation` or `accessibility`

### `automation.run`

Run an AppleScript or Automator workflow.

**Requires:** `automation` permission

**Params:**
- `script` (string, required) — AppleScript source code
- `type` (string, optional) — "applescript" (default) or "automator"

**Response:**
```json
{
  "success": true,
  "output": "script output"
}
```

### `automation.click`

Simulate a mouse click.

**Requires:** `accessibility` permission

**Params:**
- `x` (number, required)
- `y` (number, required)

**Response:**
```json
{
  "success": true
}
```

### `automation.type`

Simulate keyboard typing.

**Requires:** `accessibility` permission

**Params:**
- `text` (string, required)

**Response:**
```json
{
  "success": true
}
```

---

## config.*

### `config.get`

Get configuration value.

**Params:**
- `key` (string, required)

**Response:**
```json
{
  "key": "model_provider",
  "value": "anthropic"
}
```

### `config.set`

Set configuration value.

**Params:**
- `key` (string, required)
- `value` (any, required)

**Response:**
```json
{
  "success": true,
  "key": "model_provider",
  "value": "anthropic"
}
```

---

## gmail.*

**Requires:** OAuth token for `google` provider with Gmail scopes

### `gmail.listMessages` / `gmail.list_messages`

List messages in Gmail.

**Params:**
- `query` (string, optional) — Gmail search query
- `maxResults` / `max_results` (number, optional) — Max results
- `labelIds` / `label_ids` (array of strings, optional) — Filter by labels

**Response:**
```json
{
  "messages": [
    {
      "id": "msg123",
      "threadId": "thread456",
      "snippet": "Message preview..."
    }
  ],
  "count": 1
}
```

### `gmail.getMessage` / `gmail.get_message`

Get a specific message.

**Params:**
- `id` / `messageId` (string, required) — Message ID
- `format` (string, optional) — "full" (default), "metadata", "minimal"

**Response:**
```json
{
  "id": "msg123",
  "threadId": "thread456",
  "labelIds": ["INBOX"],
  "snippet": "...",
  "payload": {...}
}
```

### `gmail.send` / `gmail.sendMessage`

Send an email.

**Params:**
- `to` (array of strings or string, required) — Recipient email(s)
- `subject` (string, required)
- `body` (string, required)
- `cc` (array of strings, optional)
- `bcc` (array of strings, optional)

**Response:**
```json
{
  "id": "msg789",
  "threadId": "thread999",
  "labelIds": ["SENT"]
}
```

### `gmail.listLabels` / `gmail.list_labels`

List all labels.

**Params:** (none)

**Response:**
```json
{
  "labels": [
    {
      "id": "INBOX",
      "name": "INBOX",
      "type": "system"
    }
  ]
}
```

### `gmail.modifyMessage` / `gmail.modify_message`

Modify message labels.

**Params:**
- `id` / `messageId` (string, required)
- `addLabelIds` / `add_label_ids` (array of strings, optional)
- `removeLabelIds` / `remove_label_ids` (array of strings, optional)

**Response:**
```json
{
  "id": "msg123",
  "labelIds": ["INBOX", "IMPORTANT"]
}
```

### `gmail.trashMessage` / `gmail.trash_message`

Move message to trash.

**Params:**
- `id` / `messageId` (string, required)

**Response:**
```json
{
  "id": "msg123",
  "labelIds": ["TRASH"]
}
```

### `gmail.deleteMessage` / `gmail.delete_message`

Permanently delete a message.

**Params:**
- `id` / `messageId` (string, required)

**Response:**
```json
{
  "success": true
}
```

---

## gcalendar.*

**Requires:** OAuth token for `google` provider with Calendar scopes

### `gcalendar.list`

List Google Calendar calendars.

### `gcalendar.events`

List events in a date range.

### `gcalendar.createEvent`

Create a new event.

*(Similar parameter patterns to `calendar.*` methods)*

---

## onepassword.* / op.*

**Requires:** 1Password credential with `service_account_token`

### `onepassword.list_vaults` / `op.list_vaults`

List all vaults.

### `onepassword.list_items` / `op.list_items`

List items in a vault.

**Params:**
- `vault` (string, required) — Vault ID or name

### `onepassword.get_item` / `op.get_item`

Get a specific item.

**Params:**
- `vault` (string, required)
- `item` (string, required) — Item ID or name

---

## oura.*

**Requires:** Oura credential with `access_token`

### `oura.daily_sleep`

Get daily sleep data.

**Params:**
- `start_date` (string, required) — ISO 8601 date
- `end_date` (string, optional)

### `oura.daily_activity`

Get daily activity data.

### `oura.daily_readiness`

Get daily readiness scores.

---

## jira.*

**Requires:** Jira credential with `host`, `email`, `api_token`

### `jira.search`

Search for issues.

**Params:**
- `jql` (string, required) — JQL query

### `jira.get_issue`

Get a specific issue.

**Params:**
- `key` (string, required) — Issue key (e.g., "PROJ-123")

### `jira.create_issue`

Create a new issue.

**Params:**
- `project` (string, required)
- `summary` (string, required)
- `description` (string, optional)
- `issueType` (string, required)

### `jira.update_issue`

Update an issue.

**Params:**
- `key` (string, required)
- `fields` (object, required) — Fields to update

### `jira.transition_issue`

Transition an issue to a new status.

**Params:**
- `key` (string, required)
- `transition` (string, required) — Transition ID or name

---

## Source Files

- `src-tauri/src/proxy/handlers/*.rs` — All handler implementations
- `~/.tairseach/manifests/` — Manifest definitions (tool metadata)

---

*Generated: 2025-02-13*  
*Exhaustive catalog extracted from source code*
