# Credential Types Reference

**All registered credential schemas with field definitions.**

Tairseach uses a credential type system to define schemas for different integration credentials. Each type specifies required fields, their types (string or secret), and validation rules.

---

## Built-In Credential Types

### 1. onepassword

**Display Name:** 1Password Service Account

**Description:** 1Password Service Account token for API access

**Supports Multiple:** Yes (can have multiple 1Password accounts)

**Fields:**

| Field | Display Name | Type | Required | Description |
|-------|--------------|------|----------|-------------|
| `service_account_token` | Service Account Token | secret | Yes | Service account token from 1Password |

**Example credential storage:**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "auth.credentials.store",
  "params": {
    "provider": "onepassword",
    "type": "onepassword",
    "label": "work",
    "fields": {
      "service_account_token": "ops_..."
    }
  }
}
```

---

### 2. jira

**Display Name:** Jira Cloud

**Description:** Jira Cloud API credentials

**Supports Multiple:** Yes

**Fields:**

| Field | Display Name | Type | Required | Description |
|-------|--------------|------|----------|-------------|
| `host` | Jira Host | string | Yes | Jira instance hostname (e.g. company.atlassian.net) |
| `email` | Email | string | Yes | User email for authentication |
| `api_token` | API Token | secret | Yes | API token from Atlassian account settings |

**Example:**

```json
{
  "provider": "jira",
  "type": "jira",
  "label": "acme-jira",
  "fields": {
    "host": "acme.atlassian.net",
    "email": "user@acme.com",
    "api_token": "ATATT3xFfGF0..."
  }
}
```

---

### 3. oura

**Display Name:** Oura Ring

**Description:** Oura API access token

**Supports Multiple:** No (only one Oura account per system)

**Fields:**

| Field | Display Name | Type | Required | Description |
|-------|--------------|------|----------|-------------|
| `access_token` | Access Token | secret | Yes | Personal Access Token from Oura Cloud |

**Example:**

```json
{
  "provider": "oura",
  "type": "oura",
  "fields": {
    "access_token": "ABCDEF123456..."
  }
}
```

---

### 4. github

**Display Name:** GitHub

**Description:** GitHub personal access token

**Supports Multiple:** Yes

**Fields:**

| Field | Display Name | Type | Required | Description |
|-------|--------------|------|----------|-------------|
| `access_token` | Personal Access Token | secret | Yes | GitHub PAT with appropriate scopes |

**Example:**

```json
{
  "provider": "github",
  "type": "github",
  "label": "personal",
  "fields": {
    "access_token": "ghp_..."
  }
}
```

---

### 5. linear

**Display Name:** Linear

**Description:** Linear API key

**Supports Multiple:** No

**Fields:**

| Field | Display Name | Type | Required | Description |
|-------|--------------|------|----------|-------------|
| `api_key` | API Key | secret | Yes | Personal API key from Linear settings |

**Example:**

```json
{
  "provider": "linear",
  "type": "linear",
  "fields": {
    "api_key": "lin_api_..."
  }
}
```

---

### 6. notion

**Display Name:** Notion

**Description:** Notion integration token

**Supports Multiple:** Yes

**Fields:**

| Field | Display Name | Type | Required | Description |
|-------|--------------|------|----------|-------------|
| `access_token` | Integration Token | secret | Yes | Internal integration secret from Notion |

**Example:**

```json
{
  "provider": "notion",
  "type": "notion",
  "label": "personal-workspace",
  "fields": {
    "access_token": "secret_..."
  }
}
```

---

### 7. slack

**Display Name:** Slack

**Description:** Slack bot or user token

**Supports Multiple:** Yes

**Fields:**

| Field | Display Name | Type | Required | Description |
|-------|--------------|------|----------|-------------|
| `token` | Bot/User Token | secret | Yes | Bot token (xoxb-*) or user token (xoxp-*) |
| `workspace` | Workspace | string | No | Workspace name or ID (optional) |

**Example:**

```json
{
  "provider": "slack",
  "type": "slack",
  "label": "acme-workspace",
  "fields": {
    "token": "xoxb-...",
    "workspace": "acme"
  }
}
```

---

## Field Types

### `string`

Plain text field. Value is stored as-is (encrypted at rest, but not treated as extra-sensitive).

**Use for:**
- Hostnames
- Email addresses
- Workspace identifiers
- Non-secret configuration

### `secret`

Sensitive field. Value is encrypted and never displayed in plain text.

**Use for:**
- API tokens
- Access tokens
- Passwords
- Private keys

---

## Custom Credential Types

You can register custom credential types via the API.

### Register a Custom Type

**Method:** `auth.credential_types.custom.create`

**Params:**

```json
{
  "provider_type": "my_api",
  "display_name": "My API",
  "description": "Credentials for My API service",
  "fields": [
    {
      "name": "api_key",
      "display_name": "API Key",
      "type": "secret",
      "required": true,
      "description": "Your API key from the dashboard"
    },
    {
      "name": "api_secret",
      "display_name": "API Secret",
      "type": "secret",
      "required": true
    },
    {
      "name": "region",
      "display_name": "Region",
      "type": "string",
      "required": false,
      "description": "API region (us-east-1, eu-west-1, etc.)"
    }
  ],
  "supports_multiple": true
}
```

**Constraints:**
- `provider_type` must be unique
- Cannot use `provider_type` of an existing built-in type
- Cannot remove built-in types

**Example Request:**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "auth.credential_types.custom.create",
  "params": {
    "provider_type": "stripe",
    "display_name": "Stripe",
    "description": "Stripe API credentials",
    "fields": [
      {
        "name": "secret_key",
        "display_name": "Secret Key",
        "type": "secret",
        "required": true,
        "description": "Stripe secret key (sk_live_... or sk_test_...)"
      }
    ],
    "supports_multiple": true
  }
}
```

---

## Credential Resolution Chain

When a manifest or handler requests credentials:

1. **By label:** If `label` is specified, fetch credential with that label
2. **By account:** If no label, try `account` field
3. **Default:** If neither, use `"default"` account

**Example:**

```javascript
// Store with label
auth.credentials.store({
  provider: "onepassword",
  type: "onepassword",
  label: "work",
  fields: { service_account_token: "ops_work..." }
});

// Retrieve by label
const creds = auth.credentials.get({ provider: "onepassword", label: "work" });
```

---

## Validation

Credentials are validated against their type schema:

1. **All required fields** must be present
2. **Field types** must match (string vs secret is informational, both store strings)
3. **Unknown fields** are allowed but ignored

**Validation errors** return HTTP 400 with detailed error messages.

---

## Storage

Credentials are stored in the encrypted credential store at:

```
~/.tairseach/credentials.db
```

**Encryption:**
- Master key derived from system keychain (macOS)
- All credential fields encrypted at rest
- `secret` fields never logged or exposed in debug output

---

## API Methods

### List Credential Types

```json
{"jsonrpc":"2.0","id":1,"method":"auth.credential_types","params":{}}
```

**Response:**

```json
{
  "types": [
    {
      "provider_type": "onepassword",
      "display_name": "1Password Service Account",
      "description": "...",
      "fields": [...],
      "supports_multiple": true,
      "built_in": true
    }
  ]
}
```

### Store Credential

```json
{
  "jsonrpc":"2.0",
  "id":2,
  "method":"auth.credentials.store",
  "params":{
    "provider":"jira",
    "type":"jira",
    "label":"acme-jira",
    "fields":{
      "host":"acme.atlassian.net",
      "email":"user@acme.com",
      "api_token":"ATATT..."
    }
  }
}
```

### Get Credential

```json
{"jsonrpc":"2.0","id":3,"method":"auth.credentials.get","params":{"provider":"jira","label":"acme-jira"}}
```

**Response:**

```json
{
  "fields": {
    "host": "acme.atlassian.net",
    "email": "user@acme.com",
    "api_token": "ATATT..."
  }
}
```

### List Credentials (Metadata Only)

```json
{"jsonrpc":"2.0","id":4,"method":"auth.credentials.list","params":{}}
```

**Response:**

```json
{
  "credentials": [
    {
      "provider": "jira",
      "account": "acme-jira",
      "type": "jira",
      "label": "acme-jira"
    }
  ],
  "count": 1
}
```

**Note:** This does NOT return secret values — only metadata.

### Delete Credential

```json
{"jsonrpc":"2.0","id":5,"method":"auth.credentials.delete","params":{"provider":"jira","label":"acme-jira"}}
```

---

## Source Files

- `src-tauri/src/auth/credential_types.rs` — Type registry and built-in schemas
- `src-tauri/src/auth/store.rs` — Encrypted credential storage
- `src-tauri/src/proxy/handlers/auth.rs` — Socket API handlers

---

*Generated: 2025-02-13*  
*7 built-in types + custom type support*
