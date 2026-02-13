# Environment Reference

**Runtime paths, config files, and environment variables for Tairseach.**

---

## Directory Structure

### `~/.tairseach/` (Runtime Data)

Primary runtime directory for all Tairseach state.

```
~/.tairseach/
├── tairseach.sock              # Unix socket (MCP proxy server)
├── credentials.enc.json        # Encrypted OAuth tokens (auth broker)
├── credentials.schema.json     # Token schema validation
├── auth/
│   ├── master.key              # Encrypted master key
│   └── <provider>/             # Per-provider auth state
├── manifests/
│   ├── contacts.json           # Contacts API manifest
│   ├── calendar.json           # Calendar API manifest
│   ├── automation.json         # Automation manifest
│   └── ...                     # Other capability manifests
└── libs/
    └── <provider>/             # Provider-specific libraries
```

**Created by:** First launch of Tairseach.app  
**Permissions:** `700` (owner-only access)  
**Cleanup:** Safe to delete (will regenerate on next launch, but auth tokens will be lost)

### `~/Applications/Tairseach.app/` (Installed App)

Tauri-built macOS application bundle.

```
~/Applications/Tairseach.app/
└── Contents/
    ├── MacOS/
    │   ├── Tairseach           # Main GUI binary (Rust + Tauri + Vue)
    │   ├── tairseach-mcp       # MCP proxy server (Rust)
    │   └── op-helper           # 1Password helper (Go)
    ├── Resources/
    │   ├── icon.icns           # App icon
    │   └── dist/               # Vue frontend (HTML/CSS/JS)
    ├── Info.plist              # App metadata + privacy descriptions
    ├── Entitlements.plist      # Granted system permissions
    └── _CodeSignature/         # Code signing data (if signed)
```

**Installation:**
- Dev: `src-tauri/target/release/bundle/macos/Tairseach.app`
- Production: Drag from DMG to `~/Applications/`

### Source Tree (Development)

```
~/environment/tairseach/
├── src/                        # Vue 3 frontend
│   ├── views/                  # Route components
│   ├── components/             # Reusable UI
│   ├── stores/                 # Pinia state
│   ├── composables/            # Vue utilities
│   ├── workers/                # Web Workers
│   ├── router/                 # Vue Router
│   └── assets/                 # Static assets
├── src-tauri/                  # Rust backend
│   ├── src/                    # Tauri app logic
│   ├── binaries/               # Pre-built external binaries
│   │   ├── tairseach-mcp-aarch64-apple-darwin
│   │   └── op-helper-aarch64-apple-darwin
│   ├── helpers/                # External helpers (Go)
│   ├── Cargo.toml              # Rust dependencies
│   ├── tauri.conf.json         # Tauri config
│   ├── Entitlements.plist      # macOS permissions
│   └── target/                 # Build artifacts
├── docs/                       # Documentation
├── scripts/                    # Build scripts
└── package.json                # Node dependencies
```

---

## Binary Locations

### Runtime (Bundled App)

| Binary | Path | Purpose |
|--------|------|---------|
| `Tairseach` | `~/Applications/Tairseach.app/Contents/MacOS/Tairseach` | Main GUI application |
| `tairseach-mcp` | `~/Applications/Tairseach.app/Contents/MacOS/tairseach-mcp` | MCP proxy server (spawned by main app) |
| `op-helper` | `~/Applications/Tairseach.app/Contents/MacOS/op-helper` | 1Password SDK bridge (called via FFI) |

**Access from code:**
```rust
// Tauri resolves bundled binaries automatically
let mcp_bin = tauri::api::path::resolve_resource("tairseach-mcp")?;
let op_bin = tauri::api::path::resolve_resource("op-helper")?;
```

### Build Time (Development)

| Binary | Path | Built By |
|--------|------|----------|
| `tairseach` | `src-tauri/target/release/tairseach` | `cargo build --release` |
| `tairseach-mcp` | `src-tauri/target/release/tairseach-mcp` | `cargo build -p tairseach-mcp --release` |
| `op-helper` | `src-tauri/bin/op-helper` | `src-tauri/helpers/onepassword/build.sh` |

**Staging for bundle:**
```bash
# Copy to binaries/ with arch suffix for Tauri
cp src-tauri/target/release/tairseach-mcp \
   src-tauri/binaries/tairseach-mcp-aarch64-apple-darwin

cp src-tauri/bin/op-helper \
   src-tauri/binaries/op-helper-aarch64-apple-darwin
```

---

## Socket Communication

### Unix Socket

**Path:** `~/.tairseach/tairseach.sock`  
**Type:** Unix domain socket (SOCK_STREAM)  
**Protocol:** JSON-RPC over newline-delimited JSON  
**Permissions:** `600` (owner-only)

**Purpose:**
- MCP protocol bridge (OpenClaw → Tairseach)
- Tool invocation (contacts, calendar, automation, etc.)

**Lifecycle:**
1. Created by `tairseach-mcp` on startup
2. Bound to `~/.tairseach/tairseach.sock`
3. Removed on graceful shutdown
4. Stale socket cleaned up on next start

**Security:**
- File permissions prevent other users from connecting
- No authentication required (local socket only)
- Validates JSON-RPC format before processing

**Test connectivity:**
```bash
# Check if socket exists and is alive
echo '{"jsonrpc":"2.0","method":"ping","id":1}' | nc -U ~/.tairseach/tairseach.sock
```

---

## Configuration Files

### `~/.tairseach/credentials.enc.json`

**Format:** Encrypted JSON (AES-256-GCM)  
**Purpose:** OAuth token storage (Google, etc.)  
**Encryption:** Master key derived from `auth/master.key`

**Structure (decrypted):**
```json
{
  "version": 1,
  "tokens": [
    {
      "provider": "google",
      "account": "user@example.com",
      "client_id": "...",
      "client_secret": "...",
      "access_token": "...",
      "refresh_token": "...",
      "token_type": "Bearer",
      "expiry": "2026-02-13T12:00:00Z",
      "scopes": ["https://www.googleapis.com/auth/calendar"],
      "issued_at": "2026-02-13T10:00:00Z",
      "last_refreshed": "2026-02-13T11:00:00Z"
    }
  ]
}
```

**Access:**
- Read/write via Tauri commands: `auth_get_token`, `auth_store_token`, `auth_revoke_token`
- Never exposed to frontend (decryption happens in Rust)

### `~/.tairseach/credentials.schema.json`

**Purpose:** JSON schema for validating credential format  
**Used by:** Auth broker during token storage

### `~/.tairseach/auth/master.key`

**Format:** Encrypted key file (random 32-byte key + metadata)  
**Purpose:** Master encryption key for credentials  
**Derivation:** HKDF-SHA256 from user-provided passphrase (if set) or auto-generated

**Lifecycle:**
1. Generated on first launch
2. Used to encrypt/decrypt `credentials.enc.json`
3. Can be re-keyed with new passphrase

**Security:**
- File permissions: `600`
- Key never stored in plaintext memory (uses `zeroize` crate)

### Manifest Files

**Location:** `~/.tairseach/manifests/`  
**Format:** JSON (MCP manifest spec)  
**Purpose:** Tool capability definitions for each integration

**Example (`contacts.json`):**
```json
{
  "id": "contacts",
  "name": "Contacts",
  "description": "Access to macOS Contacts",
  "version": "0.1.0",
  "category": "personal",
  "tools": [
    {
      "name": "contacts.search",
      "description": "Search contacts by name",
      "inputSchema": {
        "type": "object",
        "properties": {
          "query": { "type": "string" }
        },
        "required": ["query"]
      }
    }
  ]
}
```

**Loaded by:** MCP proxy on startup  
**Watched for changes:** Yes (via `notify` crate, FSEvents on macOS)

---

## Environment Variables

### Runtime (Tairseach App)

| Variable | Default | Purpose |
|----------|---------|---------|
| `TAIRSEACH_HOME` | `~/.tairseach` | Override runtime directory |
| `TAIRSEACH_MCP_SOCKET` | `~/.tairseach/tairseach.sock` | Override socket path |
| `TAIRSEACH_LOG` | `info` | Logging level (trace/debug/info/warn/error) |
| `TAIRSEACH_NO_PROXY` | (unset) | Disable MCP proxy auto-start |

**Usage:**
```bash
# Enable debug logging
TAIRSEACH_LOG=debug open ~/Applications/Tairseach.app

# Use custom socket path
TAIRSEACH_MCP_SOCKET=/tmp/tairseach.sock open ~/Applications/Tairseach.app
```

### Build Time

| Variable | Default | Purpose |
|----------|---------|---------|
| `CARGO_BUILD_TARGET` | `aarch64-apple-darwin` | Rust target triple |
| `GOOS` | `darwin` | Go target OS (for op-helper) |
| `GOARCH` | `arm64` | Go target architecture |

**Set by:** Build scripts (automatic)

---

## OpenClaw Integration

### MCP Server Discovery

**How OpenClaw finds Tairseach:**

1. Check `~/.openclaw/config.yaml` for `tools.mcp.servers`:
   ```yaml
   tools:
     mcp:
       servers:
         tairseach:
           transport: unix
           socket: ~/.tairseach/tairseach.sock
   ```

2. Or auto-discover via `OPENCLAW_MCP_SERVERS` env var:
   ```bash
   export OPENCLAW_MCP_SERVERS="tairseach:unix:///Users/user/.tairseach/tairseach.sock"
   ```

### Skill Configuration

**Location:** `~/.openclaw/skills/tairseach/`  
**Files:**
- `SKILL.md` — Agent instructions for using Tairseach tools
- `manifest.json` — Skill metadata (auto-generated)

**Installation:**
- Manual: Copy manifests + write SKILL.md
- Automatic: Use "Install to OpenClaw" button in MCPView.vue

---

## State & Cache Files

### Browser localStorage

**Key prefix:** `tairseach_cache_`  
**Purpose:** Frontend state persistence (instant hydration)

**Cached stores:**
- `tairseach_cache_config` — OpenClaw config
- `tairseach_cache_auth` — Auth status + accounts
- `tairseach_cache_permissions` — Permission statuses
- `tairseach_cache_monitor` — Activity events
- `tairseach_cache_profiles` — Agent profiles
- `tairseach_cache_dashboard` — Dashboard summary

**Format:**
```json
{
  "data": { /* store state */ },
  "lastUpdated": "2026-02-13T12:00:00.000Z"
}
```

**Location:** Browser WebKit storage (managed by Tauri)  
**Persistence:** Survives app restart  
**Cleanup:** Clear via DevTools or delete `~/Library/WebKit/...` (complex path)

---

## Logs

### Application Logs

**Console output:**
- Dev mode: Terminal where `npm run dev` was launched
- Release: macOS Console.app → filter by "Tairseach"

**Logging library:** `tracing` (Rust)  
**Log format:**
```
2026-02-13T12:00:00.000Z INFO tairseach: Starting MCP proxy server
2026-02-13T12:00:00.100Z DEBUG tairseach_mcp: Bound to socket: ~/.tairseach/tairseach.sock
```

**Control level:**
```bash
TAIRSEACH_LOG=debug open ~/Applications/Tairseach.app
```

### Activity Events

**Storage:** In-memory (max 2000 events)  
**Access:** Via `get_events` Tauri command  
**Persistence:** None (cleared on app restart)  
**Export:** Copy from Activity view or query via Tauri command

---

## Permissions & Privacy

### macOS Privacy Descriptions

**Location:** `~/Applications/Tairseach.app/Contents/Info.plist`  
**Keys required:**
```xml
<key>NSContactsUsageDescription</key>
<string>Tairseach needs access to Contacts to provide contact management tools via MCP.</string>

<key>NSCalendarsUsageDescription</key>
<string>Tairseach needs access to Calendars to provide event management tools via MCP.</string>

<key>NSRemindersUsageDescription</key>
<string>Tairseach needs access to Reminders to provide task management tools via MCP.</string>

<key>NSPhotoLibraryUsageDescription</key>
<string>Tairseach needs access to Photos to provide photo management tools via MCP.</string>

<key>NSCameraUsageDescription</key>
<string>Tairseach needs access to the Camera for image capture tools via MCP.</string>

<key>NSMicrophoneUsageDescription</key>
<string>Tairseach needs access to the Microphone for audio recording tools via MCP.</string>

<key>NSLocationWhenInUseUsageDescription</key>
<string>Tairseach needs access to Location for location-based tools via MCP.</string>
```

**Applied by:** `scripts/patch-info-plist.sh` (during `app:launch`)

### Permission Storage

**System location:** `/Library/Application Support/com.apple.TCC/TCC.db` (system-managed)  
**User location:** `~/Library/Application Support/com.apple.TCC/TCC.db`

**Not directly editable** — must grant via System Settings:
- System Settings → Privacy & Security → [Category] → Add "Tairseach"

---

## Network & External Services

### Google OAuth

**Endpoints:**
- Authorization: `https://accounts.google.com/o/oauth2/v2/auth`
- Token exchange: `https://oauth2.googleapis.com/token`
- Token refresh: `https://oauth2.googleapis.com/token`
- Token revocation: `https://oauth2.googleapis.com/revoke`

**Redirect URI:** `http://localhost:8080/callback` (local web server)

### 1Password SDK

**Communication:** Local Go helper process (`op-helper`)  
**No network calls** — All vault access via local SDK

---

## Troubleshooting

### Socket Issues

**Problem:** "Failed to connect to socket"

**Solutions:**
1. Check socket exists: `ls -l ~/.tairseach/tairseach.sock`
2. Check permissions: Should be `srw-------` (600)
3. Kill stale processes: `lsof ~/.tairseach/tairseach.sock`
4. Delete stale socket: `rm ~/.tairseach/tairseach.sock` (will recreate on restart)

### Permission Denied

**Problem:** "Permission denied" accessing Contacts/Calendar

**Solutions:**
1. Check System Settings → Privacy & Security → [Category]
2. Verify `Info.plist` has usage descriptions (run `app:launch` not `tauri dev`)
3. Re-grant permissions: Remove from System Settings, re-add

### Auth Token Issues

**Problem:** "Failed to decrypt credentials"

**Solutions:**
1. Check `~/.tairseach/auth/master.key` exists
2. Check `~/.tairseach/credentials.enc.json` permissions (600)
3. Re-initialize auth: Delete `~/.tairseach/auth/` (will regenerate, tokens lost)

### Build Artifacts Not Found

**Problem:** "Binary not found: tairseach-mcp"

**Solutions:**
1. Run `cargo build -p tairseach-mcp --release`
2. Copy to `src-tauri/binaries/tairseach-mcp-aarch64-apple-darwin`
3. Rebuild: `npm run app:build`

---

## Path Summary

| Resource | Path | Type |
|----------|------|------|
| **Runtime Data** |
| Socket | `~/.tairseach/tairseach.sock` | Unix socket |
| Credentials | `~/.tairseach/credentials.enc.json` | Encrypted JSON |
| Master key | `~/.tairseach/auth/master.key` | Binary file |
| Manifests | `~/.tairseach/manifests/*.json` | JSON |
| **Installed App** |
| Main binary | `~/Applications/Tairseach.app/Contents/MacOS/Tairseach` | Executable |
| MCP server | `~/Applications/Tairseach.app/Contents/MacOS/tairseach-mcp` | Executable |
| 1Password helper | `~/Applications/Tairseach.app/Contents/MacOS/op-helper` | Executable |
| Frontend | `~/Applications/Tairseach.app/Contents/Resources/dist/` | Static files |
| **Development** |
| Source | `~/environment/tairseach/` | Directory |
| Build artifacts | `~/environment/tairseach/src-tauri/target/release/` | Directory |
| **OpenClaw Integration** |
| MCP config | `~/.openclaw/config.yaml` | YAML |
| Skill | `~/.openclaw/skills/tairseach/SKILL.md` | Markdown |

---

**Last updated:** 2026-02-13
