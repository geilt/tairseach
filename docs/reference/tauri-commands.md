# Tauri Commands Reference

**All `#[tauri::command]` functions callable from the Tauri frontend.**

Tauri commands are Rust functions exposed to the JavaScript frontend. These are used by the Tairseach UI to manage configuration, permissions, auth, and the proxy server.

---

## Categories

- [Proxy Server Control](#proxy-server-control)
- [Permissions](#permissions)
- [Configuration](#configuration)
- [Authentication & Credentials](#authentication--credentials)
- [Monitoring](#monitoring)
- [Profiles](#profiles)
- [Miscellaneous](#miscellaneous)

---

## Proxy Server Control

### `get_proxy_status`

Get the current status of the Unix socket proxy server.

**Returns:**
```typescript
{
  running: boolean;
  socket_path?: string;  // e.g., "/Users/user/.tairseach/tairseach.sock"
}
```

**Example (JavaScript):**
```javascript
const status = await invoke('get_proxy_status');
console.log(status.running);  // true or false
```

### `start_proxy_server`

Start the Unix socket proxy server (if not already running).

**Returns:**
```typescript
{
  running: boolean;
  socket_path: string;
}
```

**Note:** Server auto-starts on app launch.

### `stop_proxy_server`

Stop the Unix socket proxy server.

**Returns:**
```typescript
{
  stopped: boolean;
}
```

---

## Permissions

All permission commands are in `src-tauri/src/permissions/mod.rs`.

### `check_permission`

Check the status of a single permission.

**Params:**
- `permission_id` (string) — Permission ID (e.g., "contacts")

**Returns:**
```typescript
{
  id: string;
  name: string;
  description: string;
  status: "granted" | "denied" | "not_determined" | "restricted" | "unknown";
  critical: boolean;
  last_checked?: string;  // ISO 8601 timestamp
}
```

**Example:**
```javascript
const perm = await invoke('check_permission', { permissionId: 'contacts' });
console.log(perm.status);  // "granted"
```

### `check_all_permissions`

Check all 11 permissions at once.

**Returns:** Array of Permission objects (same structure as `check_permission`)

**Example:**
```javascript
const permissions = await invoke('check_all_permissions');
permissions.forEach(p => console.log(`${p.name}: ${p.status}`));
```

### `request_permission`

Request a permission (triggers native prompt or opens System Preferences).

**Params:**
- `permission_id` (string)

**Returns:** `void` (throws error if failed)

**Example:**
```javascript
await invoke('request_permission', { permissionId: 'contacts' });
```

### `trigger_permission_registration`

Trigger permission registration (makes app appear in System Preferences).

**Params:**
- `permission_id` (string)

**Returns:**
- `"registration_triggered"` — Permission trigger succeeded
- `"open_settings_required"` — Must open System Preferences manually

**Example:**
```javascript
const result = await invoke('trigger_permission_registration', { permissionId: 'screen_recording' });
```

### `open_permission_settings`

Open System Preferences to a specific privacy pane.

**Params:**
- `pane` (string) — Pane ID (e.g., "Privacy_Contacts")

**Returns:** `void`

**Example:**
```javascript
await invoke('open_permission_settings', { pane: 'Privacy_Contacts' });
```

### `get_permission_definitions`

Get metadata for all permission types.

**Returns:**
```typescript
Array<{
  id: string;
  name: string;
  description: string;
  critical: boolean;
  icon: string;
  system_pref_pane: string;
}>
```

**Example:**
```javascript
const defs = await invoke('get_permission_definitions');
const contactsPerm = defs.find(d => d.id === 'contacts');
```

### `get_permissions` (deprecated)

Legacy alias for `check_all_permissions`. Use `check_all_permissions` instead.

### `grant_permission` (not implemented)

Returns error: macOS does not allow programmatic permission granting.

### `revoke_permission` (not implemented)

Returns error: macOS does not allow programmatic permission revocation.

---

## Configuration

All config commands are in `src-tauri/src/config/mod.rs`.

### `get_config`

Get the OpenClaw configuration file.

**Returns:**
```typescript
{
  raw: any;           // Parsed JSON config
  path: string;       // File path
}
```

**Example:**
```javascript
const config = await invoke('get_config');
console.log(config.raw.agent?.model);
```

### `set_config`

Save the OpenClaw configuration file.

**Params:**
- `config` (JSON value) — Full configuration object

**Returns:** `void`

**Example:**
```javascript
await invoke('set_config', { config: { agent: { model: 'claude-sonnet-4-5' } } });
```

**Note:** Creates `.json.bak` backup before overwriting.

### `get_provider_models`

Get list of known models for supported providers.

**Returns:**
```typescript
{
  [provider: string]: Array<{
    id: string;
    name: string;
    description: string;
  }>;
}
```

**Providers:**
- `anthropic`
- `openai`
- `openai-codex`
- `google`
- `kimi-coding`

**Example:**
```javascript
const models = await invoke('get_provider_models');
const anthropicModels = models.anthropic;
```

### `get_google_oauth_config`

Get stored Google OAuth client credentials (if any).

**Returns:**
```typescript
{
  client_id: string;
  client_secret: string;
  updated_at: string;  // ISO 8601
} | null
```

**Example:**
```javascript
const config = await invoke('get_google_oauth_config');
if (config) {
  console.log(config.client_id);
}
```

### `save_google_oauth_config`

Save Google OAuth client credentials.

**Params:**
- `client_id` (string)
- `client_secret` (string)

**Returns:** `void`

**Validation:**
- Both fields required and must not be empty

**Example:**
```javascript
await invoke('save_google_oauth_config', {
  clientId: '123456789.apps.googleusercontent.com',
  clientSecret: 'GOCSPX-...'
});
```

### `test_google_oauth_config`

Test Google OAuth configuration by attempting a token exchange with a dummy code.

**Returns:**
```typescript
{
  success: boolean;
  message: string;
}
```

**Example:**
```javascript
const result = await invoke('test_google_oauth_config');
console.log(result.message);
```

### `get_google_oauth_status`

Check if Google OAuth is configured and has valid tokens.

**Returns:**
```typescript
{
  status: "ready" | "not_configured" | "no_token";
  configured: boolean;
  has_token: boolean;
  message: string;
}
```

**Example:**
```javascript
const status = await invoke('get_google_oauth_status');
if (status.status === 'ready') {
  // OAuth is configured and has tokens
}
```

### `get_environment`

Get information about the OpenClaw environment.

**Returns:**
```typescript
{
  environment_type: string;  // e.g., "macos"
  files: Array<{
    name: string;
    path: string;
  }>;
}
```

**Example:**
```javascript
const env = await invoke('get_environment');
console.log(env.environment_type);
```

### `get_node_config`

Get node configuration.

**Returns:**
```typescript
{
  config: any;
  path: string;
}
```

### `set_node_config`

Save node configuration.

**Params:**
- `config` (JSON value)

**Returns:** `void`

### `get_exec_approvals`

Get exec approval configuration.

**Returns:**
```typescript
{
  approvals: any;
  path: string;
}
```

### `set_exec_approvals`

Save exec approval configuration.

**Params:**
- `approvals` (JSON value)

**Returns:** `void`

---

## Authentication & Credentials

All auth commands are in `src-tauri/src/auth/mod.rs`.

### `authenticate`

Trigger Google OAuth flow (opens browser).

**Params:**
- `scopes` (array of strings) — OAuth scopes to request

**Returns:**
```typescript
{
  success: boolean;
  account?: string;      // Email address
  scopes?: string[];
  message: string;
}
```

**Example:**
```javascript
const result = await invoke('authenticate', {
  scopes: [
    'https://www.googleapis.com/auth/gmail.modify',
    'https://www.googleapis.com/auth/calendar'
  ]
});
```

### `check_auth`

Check if user is authenticated (has valid Google OAuth tokens).

**Returns:**
```typescript
{
  authenticated: boolean;
  account?: string;
  scopes?: string[];
}
```

### `auth_status`

Get auth subsystem status.

**Returns:**
```typescript
{
  initialized: boolean;
  master_key_available: boolean;
  account_count: number;
  gog_passphrase_set: boolean;
}
```

### `auth_providers`

List supported OAuth providers.

**Returns:**
```typescript
{
  providers: string[];  // e.g., ["google"]
}
```

### `auth_accounts`

List authorized accounts.

**Params:**
- `provider` (string, optional) — Filter by provider

**Returns:**
```typescript
{
  accounts: Array<{
    provider: string;
    account: string;
    scopes: string[];
    expiry: string;
    last_refreshed: string;
  }>;
  count: number;
}
```

### `auth_get_token`

Get access token for an account (auto-refreshes if expired).

**Params:**
- `provider` (string)
- `account` (string)
- `scopes` (array of strings, optional)

**Returns:**
```typescript
{
  access_token: string;
  token_type: string;  // "Bearer"
  expiry: string;      // ISO 8601
}
```

### `auth_refresh_token`

Force-refresh a token.

**Params:**
- `provider` (string)
- `account` (string)

**Returns:** (same as `auth_get_token`)

### `auth_revoke_token`

Revoke and remove a token.

**Params:**
- `provider` (string)
- `account` (string)

**Returns:**
```typescript
{
  success: boolean;
}
```

### `auth_store_token`

Store/import a token record directly (bypassing OAuth flow).

**Params:**
- `record` (TokenRecord object)

**Returns:**
```typescript
{
  success: boolean;
}
```

### `auth_start_google_oauth`

Start Google OAuth flow (opens browser, polls for completion).

**Params:**
- `scopes` (array of strings)

**Returns:**
```typescript
{
  success: boolean;
  account: string;
  scopes: string[];
}
```

### `auth_credential_types`

List all registered credential types.

**Returns:**
```typescript
{
  types: Array<CredentialTypeSchema>;
}
```

### `auth_credentials_store`

Store a credential.

**Params:**
- `provider` (string)
- `type` (string)
- `label` (string, optional)
- `fields` (object)

**Returns:**
```typescript
{
  success: boolean;
}
```

### `auth_credentials_list`

List all credentials (metadata only, no secrets).

**Returns:**
```typescript
{
  credentials: Array<{
    provider: string;
    account: string;
    type: string;
    label: string;
  }>;
  count: number;
}
```

### `auth_credentials_get`

Get a credential (includes secret fields).

**Params:**
- `provider` (string)
- `label` (string, optional)

**Returns:**
```typescript
{
  fields: Record<string, string>;
}
```

### `auth_credentials_delete`

Delete a credential.

**Params:**
- `provider` (string)
- `label` (string, optional)

**Returns:**
```typescript
{
  success: boolean;
}
```

### `auth_credential_types_custom_create`

Register a custom credential type.

**Params:**
- `schema` (CredentialTypeSchema object)

**Returns:**
```typescript
{
  success: boolean;
}
```

### `op_vaults_list`

List 1Password vaults (requires 1Password credential).

**Returns:**
```typescript
{
  vaults: Array<{
    id: string;
    name: string;
  }>;
}
```

### `op_config_set_default_vault`

Set default 1Password vault.

**Params:**
- `vault_id` (string)

**Returns:** `void`

---

## Monitoring

All monitor commands are in `src-tauri/src/monitor/mod.rs`.

### `get_events`

Get recent proxy server events (for debugging).

**Returns:**
```typescript
{
  events: Array<{
    timestamp: string;
    type: string;
    data: any;
  }>;
}
```

### `get_manifest_summary`

Get summary of loaded manifests.

**Returns:**
```typescript
{
  total: number;
  by_category: Record<string, number>;
  tool_count: number;
}
```

### `get_all_manifests`

Get full list of loaded manifests.

**Returns:**
```typescript
{
  manifests: Array<Manifest>;
}
```

### `check_socket_alive`

Check if the proxy server socket is responding.

**Returns:**
```typescript
{
  alive: boolean;
  socket_path: string;
}
```

### `test_mcp_tool`

Test an MCP tool by calling it via the socket.

**Params:**
- `tool_name` (string)
- `arguments` (JSON value)

**Returns:**
```typescript
{
  success: boolean;
  result?: any;
  error?: string;
}
```

### `get_namespace_statuses`

Get status for all handler namespaces.

**Returns:**
```typescript
{
  namespaces: Array<{
    name: string;
    available: boolean;
    tool_count: number;
  }>;
}
```

### `install_tairseach_to_openclaw`

Install Tairseach MCP server configuration to OpenClaw.

**Returns:**
```typescript
{
  success: boolean;
  message: string;
}
```

---

## Profiles

All profile commands are in `src-tauri/src/profiles/mod.rs`.

### `get_profiles`

Get list of available configuration profiles.

**Returns:**
```typescript
{
  profiles: Array<{
    name: string;
    path: string;
  }>;
}
```

### `save_profile`

Save current configuration as a named profile.

**Params:**
- `name` (string) — Profile name
- `config` (JSON value) — Configuration to save

**Returns:** `void`

---

## Miscellaneous

### `greet`

Test command that returns a greeting message.

**Params:**
- `name` (string)

**Returns:** `string` (e.g., "Hello, Alice! Welcome to Tairseach.")

**Example:**
```javascript
const greeting = await invoke('greet', { name: 'Alice' });
console.log(greeting);
```

---

## Error Handling

All commands return `Result<T, String>` in Rust, which translates to JavaScript promises:

**Success:**
```javascript
const result = await invoke('command_name', { param: value });
```

**Error:**
```javascript
try {
  await invoke('command_name', { param: value });
} catch (error) {
  console.error(error);  // Error message string
}
```

---

## Source Files

| File | Commands |
|------|----------|
| `src-tauri/src/lib.rs` | Proxy server control, `greet` |
| `src-tauri/src/permissions/mod.rs` | All permission commands |
| `src-tauri/src/config/mod.rs` | All configuration commands |
| `src-tauri/src/auth/mod.rs` | All auth/credential commands |
| `src-tauri/src/monitor/mod.rs` | All monitoring commands |
| `src-tauri/src/profiles/mod.rs` | Profile management |

---

*Generated: 2025-02-13*  
*50+ Tauri commands documented*
