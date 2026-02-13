# Configuration Reference

**All configuration files used by Tairseach and OpenClaw.**

Tairseach stores configuration in multiple JSON files under `~/.tairseach/` and `~/.openclaw/`.

---

## File Locations

| File | Path | Purpose |
|------|------|---------|
| OpenClaw config | `~/.openclaw/openclaw.json` | OpenClaw agent configuration |
| Node config | `~/.openclaw/node.json` | Node pairing configuration |
| Exec approvals | `~/.openclaw/exec-approvals.json` | Shell command approval rules |
| Google OAuth | `~/.tairseach/auth/google_oauth.json` | Google OAuth client credentials |
| 1Password config | `~/.tairseach/auth/onepassword.json` | 1Password default vault |
| Credentials DB | `~/.tairseach/credentials.db` | Encrypted credential store (SQLite) |
| Token store | `~/.tairseach/tokens.db` | Encrypted OAuth token store (SQLite) |
| Manifests | `~/.tairseach/manifests/**/*.json` | Capability manifests |

---

## ~/.openclaw/openclaw.json

**Main OpenClaw configuration file.**

### Example Structure

```json
{
  "version": "1.0.0",
  "agent": {
    "name": "suibhne",
    "model": "anthropic/claude-sonnet-4-5",
    "temperature": 0.7,
    "max_tokens": 4096
  },
  "providers": {
    "anthropic": {
      "api_key": "sk-ant-..."
    },
    "openai": {
      "api_key": "sk-..."
    }
  },
  "mcpServers": {
    "tairseach": {
      "command": "/Users/user/.tairseach/bin/tairseach-mcp",
      "args": [],
      "env": {}
    }
  },
  "memory": {
    "enabled": true,
    "max_entries": 1000
  },
  "telegram": {
    "enabled": true,
    "bot_token": "...",
    "allowed_users": [137549473]
  }
}
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `version` | string | Config schema version |
| `agent.name` | string | Agent identifier |
| `agent.model` | string | Default LLM model (provider/model format) |
| `agent.temperature` | number | Sampling temperature (0.0-1.0) |
| `agent.max_tokens` | number | Max response tokens |
| `providers` | object | API keys for model providers |
| `mcpServers` | object | MCP server configurations |
| `memory.enabled` | boolean | Enable agent memory |
| `memory.max_entries` | number | Max memory entries |
| `telegram.enabled` | boolean | Enable Telegram integration |
| `telegram.bot_token` | string | Telegram bot token |
| `telegram.allowed_users` | array | Allowed Telegram user IDs |

### Accessing via Tauri

```javascript
const config = await invoke('get_config');
console.log(config.raw.agent.model);

// Update
config.raw.agent.model = 'anthropic/claude-opus-4-5';
await invoke('set_config', { config: config.raw });
```

---

## ~/.openclaw/node.json

**Node pairing configuration.**

### Example

```json
{
  "node_id": "croí",
  "pairing_token": "...",
  "gateway_url": "https://gateway.openclaw.com",
  "paired_at": "2025-02-13T12:00:00Z"
}
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `node_id` | string | Unique node identifier |
| `pairing_token` | string | Authentication token |
| `gateway_url` | string | OpenClaw gateway URL |
| `paired_at` | string | ISO 8601 pairing timestamp |

---

## ~/.openclaw/exec-approvals.json

**Shell command approval rules.**

### Example

```json
{
  "rules": [
    {
      "pattern": "git *",
      "action": "allow"
    },
    {
      "pattern": "rm -rf *",
      "action": "deny"
    },
    {
      "pattern": "*",
      "action": "prompt"
    }
  ],
  "default": "prompt"
}
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `rules` | array | Command approval rules (glob patterns) |
| `rules[].pattern` | string | Glob pattern to match commands |
| `rules[].action` | string | "allow", "deny", or "prompt" |
| `default` | string | Default action if no rule matches |

---

## ~/.tairseach/auth/google_oauth.json

**Google OAuth client credentials.**

### Example

```json
{
  "client_id": "123456789.apps.googleusercontent.com",
  "client_secret": "GOCSPX-...",
  "updated_at": "2025-02-13T12:00:00Z"
}
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `client_id` | string | OAuth 2.0 client ID from Google Cloud Console |
| `client_secret` | string | OAuth 2.0 client secret |
| `updated_at` | string | ISO 8601 timestamp of last update |

### Setup

1. Create OAuth 2.0 credentials in [Google Cloud Console](https://console.cloud.google.com/)
2. Add redirect URI: `http://localhost:8765/callback`
3. Save client ID and secret via Tauri command:

```javascript
await invoke('save_google_oauth_config', {
  clientId: 'YOUR_CLIENT_ID',
  clientSecret: 'YOUR_CLIENT_SECRET'
});
```

### Required Scopes

For Gmail:
- `https://www.googleapis.com/auth/gmail.modify`
- `https://www.googleapis.com/auth/gmail.settings.basic`

For Google Calendar:
- `https://www.googleapis.com/auth/calendar`
- `https://www.googleapis.com/auth/calendar.events`

---

## ~/.tairseach/auth/onepassword.json

**1Password default vault configuration.**

### Example

```json
{
  "default_vault_id": "vault-uuid-here",
  "updated_at": "2025-02-13T12:00:00Z"
}
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `default_vault_id` | string | UUID of default vault |
| `updated_at` | string | ISO 8601 timestamp |

---

## ~/.tairseach/credentials.db

**Encrypted credential store (SQLite database).**

**Schema:**

```sql
CREATE TABLE credentials (
  provider TEXT NOT NULL,
  account TEXT NOT NULL,
  cred_type TEXT NOT NULL,
  fields_encrypted BLOB NOT NULL,
  label TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (provider, account)
);

CREATE TABLE credential_types (
  provider_type TEXT PRIMARY KEY,
  schema_json TEXT NOT NULL,
  custom INTEGER NOT NULL DEFAULT 0
);
```

**Encryption:**
- All `fields_encrypted` blobs are AES-256-GCM encrypted
- Master key derived from machine identity (hardware UUID + username) via HKDF-SHA256
- No keychain prompts required

**Access:** Via `auth.credentials.*` methods only (not direct SQL)

---

## ~/.tairseach/tokens.db

**Encrypted OAuth token store (SQLite database).**

**Schema:**

```sql
CREATE TABLE tokens (
  provider TEXT NOT NULL,
  account TEXT NOT NULL,
  token_encrypted BLOB NOT NULL,
  expiry TEXT NOT NULL,
  scopes TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (provider, account)
);

CREATE TABLE gog_passphrase (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  passphrase_encrypted BLOB NOT NULL
);
```

**Encryption:** Same as credentials.db (AES-256-GCM with machine-derived master key)

**Access:** Via `auth.*` methods only

---

## ~/.tairseach/manifests/

**Capability manifests directory.**

**Structure:**

```
~/.tairseach/manifests/
├── core/
│   ├── auth.json
│   ├── contacts.json
│   ├── calendar.json
│   ├── reminders.json
│   ├── location.json
│   ├── screen.json
│   ├── files.json
│   ├── automation.json
│   ├── config.json
│   ├── permissions.json
│   └── server.json
└── integrations/
    ├── gmail.json
    ├── gcalendar.json
    ├── onepassword.json
    ├── oura.json
    └── jira.json
```

**Format:** See [manifest-schema.md](./manifest-schema.md)

**Hot-Reload:** Manifests are watched for changes and automatically reloaded.

---

## Environment-Specific Files

### macOS

Tairseach is currently macOS-only. Configuration paths use standard macOS conventions:

- `~/.tairseach/` — User-specific Tairseach data
- `~/.openclaw/` — User-specific OpenClaw data

### Permissions

Files in `~/.tairseach/` are:
- Owned by the user
- Mode: `0600` (credentials/tokens DBs), `0644` (JSON configs)
- Not synced to iCloud (excluded via `.nosync` or path exclusion)

---

## Configuration Backup

**Automatic Backups:**

When updating config files via Tauri commands, `.bak` backups are created:

- `openclaw.json.bak`
- `google_oauth.json.bak`
- etc.

**Manual Backup:**

```bash
# Backup all Tairseach config
tar -czf ~/tairseach-backup-$(date +%Y%m%d).tar.gz ~/.tairseach/

# Backup OpenClaw config
tar -czf ~/openclaw-backup-$(date +%Y%m%d).tar.gz ~/.openclaw/
```

**Restore:**

```bash
# Restore Tairseach
tar -xzf ~/tairseach-backup-20250213.tar.gz -C ~/

# Restore OpenClaw
tar -xzf ~/openclaw-backup-20250213.tar.gz -C ~/
```

---

## Default Values

### First Run

On first run, Tairseach creates:

1. `~/.tairseach/` directory
2. `tokens.db` (empty)
3. `credentials.db` (empty, with built-in credential types)
4. `manifests/` directory (populated from embedded manifests)

### Missing Files

If config files are missing, Tairseach/OpenClaw will:

- Create default configs on first access
- Return errors via Tauri commands (e.g., `get_config` fails if `openclaw.json` missing)

---

## Configuration Validation

**On Load:**

- JSON files are validated for syntax
- Schema validation for structured configs (e.g., manifests)
- Invalid configs return detailed error messages

**On Save:**

- Tauri commands validate inputs before writing
- `.bak` backups created before overwriting

---

## Source Files

| File | Purpose |
|------|---------|
| `src-tauri/src/config/mod.rs` | Config file reading/writing |
| `src-tauri/src/auth/store.rs` | Token/credential store |
| `src-tauri/src/manifest/mod.rs` | Manifest loading and validation |

---

*Generated: 2025-02-13*  
*Configuration files documented*
