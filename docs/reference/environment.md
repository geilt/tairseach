# Environment Reference

Runtime paths, configuration files, and environment variables used by Tairseach.

---

## Directory Structure

All Tairseach runtime state lives under `~/.tairseach/`:

```
~/.tairseach/
├── tairseach.sock              # Unix domain socket (MCP bridge)
├── manifests/                  # Deployed MCP manifests
│   ├── core/                   # Core capabilities
│   │   ├── fs.json
│   │   ├── exec.json
│   │   └── ...
│   └── integrations/           # Integration manifests
│       ├── google.json
│       ├── jira.json
│       ├── oura.json
│       └── ...
├── credentials/                # Encrypted credential store
│   ├── store.aes               # AES-GCM encrypted credentials
│   └── salt.hex                # HKDF salt for key derivation
├── config/                     # Application configuration
│   └── proxy.toml              # Proxy server config (port, socket path, etc.)
└── logs/                       # Application logs
    ├── app.log                 # Tauri app logs
    ├── proxy.log               # MCP proxy server logs
    └── auth.log                # Auth flow logs
```

---

## Unix Socket

**Path:** `~/.tairseach/tairseach.sock`  
**Purpose:** MCP (Model Context Protocol) communication socket  
**Owner:** `tairseach-mcp` binary (spawned by Tauri app)  
**Permissions:** `0600` (user read/write only)  
**Protocol:** MCP over JSON-RPC 2.0

**Lifecycle:**
- Created by `tairseach-mcp` on startup
- Removed on clean shutdown
- **Stale socket handling:** If socket exists on startup, server attempts to connect to verify if active. If connection fails, socket is unlinked and recreated.

**Usage:**
- OpenClaw gateway connects via MCP protocol
- Tairseach GUI sends tool invocations
- Activity feed tails structured log events from socket

**Example connection (from OpenClaw):**
```toml
# ~/.openclaw/config.toml
[mcpServers.tairseach]
type = "stdio"
command = "nc"
args = ["-U", "/Users/geilt/.tairseach/tairseach.sock"]
```

---

## Manifests

**Base path:** `~/.tairseach/manifests/`

### Core Manifests (`core/`)

Core capabilities exposed by macOS/Tauri:

- `fs.json` — File system operations (read, write, list, delete)
- `exec.json` — Command execution
- `permissions.json` — macOS permission management
- `notifications.json` — macOS notification center
- `clipboard.json` — System clipboard
- `system.json` — System info (hostname, uptime, etc.)

**Format:**
```json
{
  "id": "core.fs",
  "name": "File System",
  "description": "File system operations",
  "version": "1.0.0",
  "category": "core",
  "tools": [
    {
      "name": "fs.read",
      "description": "Read file contents",
      "inputSchema": {
        "type": "object",
        "properties": {
          "path": { "type": "string" }
        },
        "required": ["path"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "contents": { "type": "string" }
        }
      }
    }
  ]
}
```

### Integration Manifests (`integrations/`)

Third-party service integrations:

- `google.json` — Google Calendar, Gmail, Contacts (OAuth)
- `jira.json` — Jira projects, issues, boards
- `oura.json` — Oura Ring sleep/health data
- `onepassword.json` — 1Password vaults, items (via `op-helper`)
- `slack.json` — Slack channels, messages (future)

**Manifest discovery:**
- Backend scans `~/.tairseach/manifests/**/*.json`
- Cached in memory on startup
- File watcher reloads on changes (via `notify` crate)

**Namespace mapping:**
- Manifest `id` first segment becomes namespace: `google.calendar` → `google`
- Tools prefixed with namespace: `google.calendar.list`

---

## Credentials

**Base path:** `~/.tairseach/credentials/`

### Encrypted Store

**File:** `~/.tairseach/credentials/store.aes`  
**Format:** AES-256-GCM encrypted JSON  
**Key derivation:** HKDF-SHA256 from macOS Keychain master key + file-based salt

**Structure (decrypted):**
```json
{
  "google_oauth": {
    "access_token": "ya29.a0AfH6...",
    "refresh_token": "1//0gZ...",
    "expires_at": "2024-02-13T12:00:00Z",
    "scopes": ["https://www.googleapis.com/auth/calendar"]
  },
  "jira": {
    "api_token": "ATATT3xFfGF0...",
    "email": "user@example.com",
    "base_url": "https://example.atlassian.net"
  }
}
```

**Credential types:**
- `google_oauth` — Google OAuth 2.0 tokens (auto-refresh)
- `jira` — Jira API token + base URL
- `oura` — Oura Personal Access Token
- `1password` — 1Password service account token (stored in macOS Keychain, not file)

### Salt File

**File:** `~/.tairseach/credentials/salt.hex`  
**Format:** 32-byte random salt, hex-encoded  
**Purpose:** Combined with Keychain master key for HKDF  
**Generated:** Once on first credential save  
**Rotation:** Not currently supported (would require re-encryption of all credentials)

**Security model:**
- Master key stored in macOS Keychain (OS-protected)
- Salt stored on disk (not secret, provides key rotation potential)
- Encrypted credentials file useless without Keychain access
- Keychain access requires user authentication (Touch ID/password)

---

## Configuration

**Base path:** `~/.tairseach/config/`

### Proxy Configuration

**File:** `~/.tairseach/config/proxy.toml`  
**Purpose:** MCP proxy server runtime config

**Example:**
```toml
[server]
socket_path = "/Users/geilt/.tairseach/tairseach.sock"
log_path = "/Users/geilt/.tairseach/logs/proxy.log"
log_level = "info"

[manifests]
core_path = "/Users/geilt/.tairseach/manifests/core"
integrations_path = "/Users/geilt/.tairseach/manifests/integrations"
watch = true  # Hot-reload on manifest changes

[credentials]
store_path = "/Users/geilt/.tairseach/credentials/store.aes"
```

**Defaults:**
- Socket path: `~/.tairseach/tairseach.sock`
- Log level: `info`
- Manifest watching: enabled

---

## Logs

**Base path:** `~/.tairseach/logs/`

### App Log

**File:** `~/.tairseach/logs/app.log`  
**Written by:** Tauri app (Rust backend)  
**Format:** Structured JSON lines (one event per line)  
**Rotation:** None (manual cleanup)

**Example entry:**
```json
{
  "timestamp": "2024-02-13T10:30:45.123Z",
  "level": "INFO",
  "target": "tairseach::commands::permissions",
  "message": "Permission check requested",
  "fields": {
    "permission_id": "screen-recording"
  }
}
```

### Proxy Log

**File:** `~/.tairseach/logs/proxy.log`  
**Written by:** `tairseach-mcp` binary  
**Format:** Structured JSON lines  
**Read by:** ActivityView (tails via `get_events` command)

**Example entry:**
```json
{
  "id": "01HPQR7S2T3V4W5X6Y7Z8A9B0C",
  "timestamp": "2024-02-13T10:31:00.456Z",
  "event_type": "tool_invocation",
  "source": "openclaw",
  "message": "Tool executed successfully",
  "metadata": {
    "tool": "gcalendar.listCalendars",
    "namespace": "google",
    "status": "success",
    "duration_ms": 234
  }
}
```

**Event types:**
- `tool_invocation` — MCP tool called
- `auth_flow_start` — OAuth flow initiated
- `auth_flow_complete` — OAuth flow completed
- `credential_refresh` — Token auto-refresh
- `manifest_reload` — Manifest hot-reload event
- `error` — Any error condition

### Auth Log

**File:** `~/.tairseach/logs/auth.log`  
**Written by:** Auth broker (Rust backend)  
**Format:** Structured JSON lines  
**Purpose:** OAuth flow debugging, credential refresh tracking

---

## Binaries

### MCP Bridge

**Dev mode:** `src-tauri/target/debug/tairseach-mcp`  
**Production (bundled):** `Tairseach.app/Contents/MacOS/tairseach-mcp`

**Purpose:** MCP server exposing Tairseach capabilities to OpenClaw  
**Lifecycle:** Spawned by Tauri app on startup via `tauri::process::Command`  
**Shutdown:** Killed when Tauri app quits (child process)

**Invocation:**
```rust
tauri::api::process::Command::new_sidecar("tairseach-mcp")
  .expect("failed to create sidecar command")
  .spawn()
```

### 1Password Helper

**Dev mode:** `src-tauri/bin/op-helper`  
**Production (bundled):** `Tairseach.app/Contents/MacOS/op-helper`

**Purpose:** Wrapper around 1Password SDK (Go binary)  
**Lifecycle:** Invoked on-demand via `std::process::Command`  
**Communication:** JSON over stdin/stdout

**Example invocation:**
```bash
echo '{"vault":"Naonur","item":"OpenAI API Key"}' | op-helper
```

**Response:**
```json
{
  "field": "credential",
  "value": "sk-..."
}
```

---

## Environment Variables

Tairseach does NOT use environment variables for configuration. All config is file-based (TOML).

**Rationale:**
- Avoids env var pollution in macOS app environment
- Config files are version-controlled (via user's dotfiles)
- Easier multi-environment support (dev/prod configs)

**Exception:** Logging level can be overridden via `RUST_LOG` (Rust convention):
```bash
RUST_LOG=debug open -a Tairseach
```

**Supported `RUST_LOG` levels:**
- `error` — Errors only
- `warn` — Warnings + errors
- `info` — Info + warnings + errors (default)
- `debug` — Debug + info + warnings + errors
- `trace` — All logs (very verbose)

**Module-specific logging:**
```bash
RUST_LOG=tairseach::auth=debug,tairseach::proxy=info
```

---

## File Permissions

**Recommended permissions:**

```bash
~/.tairseach/                    # 0700 (user rwx only)
~/.tairseach/tairseach.sock      # 0600 (user rw only)
~/.tairseach/manifests/          # 0755 (user rwx, group/other rx)
~/.tairseach/credentials/        # 0700 (user rwx only)
~/.tairseach/credentials/*.aes   # 0600 (user rw only)
~/.tairseach/config/             # 0755 (user rwx, group/other rx)
~/.tairseach/logs/               # 0755 (user rwx, group/other rx)
```

**Security notes:**
- Socket must be user-only (no group/other access)
- Credentials directory must be user-only (contains encrypted secrets)
- Logs can be group/other readable (structured logs, no secrets)

**Enforcement:**
- App sets permissions on startup via `std::fs::set_permissions`
- Warns on startup if permissions are too permissive

---

## Cleanup

**Manual cleanup (safe):**
```bash
# Remove all Tairseach runtime state
rm -rf ~/.tairseach/
```

**Partial cleanup:**
```bash
# Clear logs only
rm ~/.tairseach/logs/*

# Clear cached manifests (will regenerate on next launch)
rm ~/.tairseach/manifests/**/*.json
```

**Credential removal:**
```bash
# Remove encrypted credentials (requires re-authentication)
rm ~/.tairseach/credentials/store.aes
```

**Uninstall checklist:**
1. Quit Tairseach app
2. Delete app: `rm -rf ~/Applications/Tairseach.app`
3. Delete runtime state: `rm -rf ~/.tairseach/`
4. Delete OpenClaw integration: Remove `[mcpServers.tairseach]` from `~/.openclaw/config.toml`

---

## Path Conventions

**User-specific paths:**
- `~/.tairseach/` — Always expands to current user's home directory
- `~/Applications/Tairseach.app` — User-installed apps (not `/Applications/`)

**Portable paths (used in code):**
```rust
dirs::home_dir()  // ~/.tairseach/ base
dirs::config_dir()  // ~/Library/Application Support/ (not used currently)
dirs::cache_dir()  // ~/Library/Caches/ (not used currently)
```

**Cross-platform note:**
- Tairseach currently macOS-only
- All paths assume POSIX filesystem
- Future Windows support would use `%APPDATA%\Tairseach\` instead of `~/.tairseach/`

---

## OpenClaw Integration Paths

**OpenClaw config file:** `~/.openclaw/config.toml`

**Tairseach MCP server entry:**
```toml
[mcpServers.tairseach]
type = "stdio"
command = "nc"
args = ["-U", "/Users/geilt/.tairseach/tairseach.sock"]
```

**Alternative (future): Direct binary invocation**
```toml
[mcpServers.tairseach]
type = "stdio"
command = "/Users/geilt/Applications/Tairseach.app/Contents/MacOS/tairseach-mcp"
args = ["--socket", "/Users/geilt/.tairseach/tairseach.sock"]
```

**Skill directory (optional):**
```
~/.openclaw/skills/tairseach/
├── SKILL.md                    # Skill instructions for agents
└── examples/                   # Example tool usage
```

---

## Migration Notes

**From v1 (if exists):**
- No v1 exists yet; this is initial release

**Future migrations:**
- Credential store format changes → auto-migration on first launch
- Manifest schema changes → backward-compatible or manual migration script
- Config file changes → TOML parser with fallback defaults

---

## Troubleshooting

**Issue:** Socket permission denied  
**Fix:**
```bash
chmod 600 ~/.tairseach/tairseach.sock
```

**Issue:** Stale socket prevents startup  
**Fix:** Remove stale socket:
```bash
rm ~/.tairseach/tairseach.sock
```
App will recreate on next launch.

**Issue:** Credential decryption fails  
**Cause:** Keychain access denied or salt file corrupt  
**Fix:**
1. Quit Tairseach
2. Remove credentials: `rm -rf ~/.tairseach/credentials/`
3. Relaunch and re-authenticate

**Issue:** Manifest not found  
**Cause:** File watcher missed update or manifest malformed  
**Fix:**
```bash
# Validate manifest JSON
cat ~/.tairseach/manifests/integrations/google.json | jq .
# Restart app to force manifest reload
```

**Issue:** Logs growing too large  
**Fix:** Implement log rotation (not yet built-in):
```bash
# Manual rotation
mv ~/.tairseach/logs/proxy.log ~/.tairseach/logs/proxy.log.1
touch ~/.tairseach/logs/proxy.log
```

---

## Environment Variable Reference (Summary)

| Variable | Purpose | Default |
|----------|---------|---------|
| `RUST_LOG` | Logging verbosity | `info` |
| (none) | App config is file-based | — |

**Note:** Tairseach intentionally avoids env vars for runtime config. All settings are in TOML files.
