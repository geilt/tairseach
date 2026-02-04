# Tairseach — Dréacht

*"Tairseach" (TAR-shakh) — The Threshold*

A macOS bridge application for the Naonúr ecosystem. The threshold between the digital realm and the system beneath.

**Created:** 2026-02-03
**Author:** Suibhne (with Geilt)
**Status:** Planning

---

## Vision

Tairseach is the macOS system bridge for OpenClaw agents. It provides:
1. **Permission Proxy** — Request and manage macOS permissions on behalf of agents
2. **Configuration Manager** — Visual editor for `~/.openclaw.json`
3. **MCP Server** — Model Context Protocol server for efficient agent ↔ OpenClaw communication
4. **Context Monitor** — Real-time token usage tracking (like CodexBar)
5. **Agent Profiles** — Visual identity management for agents
6. **Auth Broker** — Persistent OAuth session management for CLI tools (GOG, etc.)

---

## Architecture Overview

```
┌──────────────────────────────────────────────────────────────────┐
│                        Tairseach.app                              │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │                      Tauri Shell                            │  │
│  │  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌──────────┐ │  │
│  │  │ Rust Core  │ │ MCP Server │ │ Permission │ │  Auth    │ │  │
│  │  │ (Commands) │ │ (Built-in) │ │   Bridge   │ │  Broker  │ │  │
│  │  └────────────┘ └────────────┘ └────────────┘ └──────────┘ │  │
│  │                        ↓                                    │  │
│  │  ┌──────────────────────────────────────────────────────┐  │  │
│  │  │              WebView Frontend                         │  │  │
│  │  │  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐    │  │  │
│  │  │  │Dash │ │Perm │ │Conf │ │Mon  │ │Prof │ │Auth │    │  │  │
│  │  │  │board│ │iss- │ │ig   │ │itor │ │iles │ │     │    │  │  │
│  │  │  │     │ │ions │ │     │ │     │ │     │ │     │    │  │  │
│  │  │  └─────┘ └─────┘ └─────┘ └─────┘ └─────┘ └─────┘    │  │  │
│  │  └──────────────────────────────────────────────────────┘  │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
         │                    │                    │            │
         ▼                    ▼                    ▼            ▼
   ~/.tairseach/      ~/.openclaw.json     OpenClaw       CLI Tools
   (logs, tokens)      (config)            Gateway        (gog, etc.)
```

---

## Technology Stack

| Layer | Technology | Rationale |
|-------|------------|-----------|
| **Shell** | Tauri 2.x | Native macOS, small binary, Rust safety |
| **Backend** | Rust | Permission APIs, file I/O, MCP server |
| **Frontend** | Vue 3 + TypeScript | Component architecture, maintainable |
| **Styling** | TailwindCSS | Utility-first, easy theming |
| **State** | Pinia | Vue's recommended store |
| **IPC** | Tauri Commands + Events | Type-safe Rust ↔ JS bridge |

---

## File Structure

```
tairseach/
├── DREACHT.md                 # This document
├── README.md                  # Project overview
├── Cargo.toml                 # Rust workspace
├── package.json               # Node/frontend deps
├── tauri.conf.json            # Tauri configuration
│
├── src-tauri/                 # Rust backend
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs            # Entry point
│   │   ├── lib.rs             # Command exports
│   │   ├── permissions/       # macOS permission handling
│   │   │   ├── mod.rs
│   │   │   ├── contacts.rs
│   │   │   ├── automation.rs
│   │   │   ├── full_disk.rs
│   │   │   ├── accessibility.rs
│   │   │   ├── screen_recording.rs
│   │   │   └── calendar.rs
│   │   ├── config/            # OpenClaw config management
│   │   │   ├── mod.rs
│   │   │   ├── schema.rs
│   │   │   └── editor.rs
│   │   ├── mcp/               # MCP server
│   │   │   ├── mod.rs
│   │   │   ├── server.rs
│   │   │   └── handlers.rs
│   │   ├── monitor/           # Context usage tracking
│   │   │   ├── mod.rs
│   │   │   └── usage.rs
│   │   ├── profiles/          # Agent profile management
│   │   │   ├── mod.rs
│   │   │   └── storage.rs
│   │   └── auth/              # OAuth broker
│   │       ├── mod.rs
│   │       ├── tokens.rs      # Token storage & refresh
│   │       ├── google.rs      # Google OAuth flow
│   │       └── proxy.rs       # CLI passthrough
│   └── icons/                 # App icons
│
├── src/                       # Vue frontend
│   ├── main.ts
│   ├── App.vue
│   ├── router/
│   │   └── index.ts
│   ├── stores/
│   │   ├── permissions.ts
│   │   ├── config.ts
│   │   ├── monitor.ts
│   │   ├── profiles.ts
│   │   └── auth.ts
│   ├── views/
│   │   ├── DashboardView.vue
│   │   ├── PermissionsView.vue
│   │   ├── ConfigView.vue
│   │   ├── MonitorView.vue
│   │   ├── ProfilesView.vue
│   │   └── AuthView.vue
│   ├── components/
│   │   ├── common/
│   │   │   ├── TabNav.vue
│   │   │   ├── StatusBadge.vue
│   │   │   └── Toast.vue
│   │   ├── permissions/
│   │   │   ├── PermissionCard.vue
│   │   │   └── PermissionStatus.vue
│   │   ├── config/
│   │   │   ├── ConfigSection.vue
│   │   │   ├── ConfigField.vue
│   │   │   └── ArrayEditor.vue
│   │   ├── monitor/
│   │   │   ├── UsageGauge.vue
│   │   │   └── SessionList.vue
│   │   ├── profiles/
│   │   │   ├── AgentCard.vue
│   │   │   └── AvatarUpload.vue
│   │   └── auth/
│   │       ├── ServiceCard.vue
│   │       ├── TokenStatus.vue
│   │       └── OAuthConnect.vue
│   └── assets/
│       └── styles/
│           ├── main.css
│           └── naonur-theme.css
│
└── .tairseach/                # Runtime data (in ~/.tairseach/)
    ├── logs/
    ├── profiles/
    │   └── [agent-id]/
    │       └── avatar.png
    └── cache/
```

---

## Data Storage

**Location:** `~/.tairseach/`

```
~/.tairseach/
├── config.json         # Tairseach's own settings
├── logs/
│   └── tairseach.log   # Application logs
├── profiles/
│   ├── suibhne/
│   │   └── avatar.png
│   ├── becuma/
│   │   └── avatar.png
│   └── muirgen/
│       └── avatar.png
├── tokens/             # OAuth tokens (encrypted)
│   ├── google.keychain # Google OAuth (stored in macOS Keychain)
│   └── services.json   # Service metadata (not secrets)
└── cache/
    └── schema.json     # Cached OpenClaw config schema
```

---

## Design Language — Naonúr Aesthetic

### Color Palette

```css
:root {
  /* Primary */
  --naonur-gold: #C9A227;          /* Accent, buttons, highlights */
  --naonur-gold-dim: #8B7019;      /* Muted gold */
  
  /* Backgrounds */
  --naonur-void: #0A0A0F;          /* Deepest background */
  --naonur-shadow: #12121A;        /* Card backgrounds */
  --naonur-mist: #1A1A24;          /* Elevated surfaces */
  --naonur-fog: #252532;           /* Hover states */
  
  /* Text */
  --naonur-bone: #E8E4D9;          /* Primary text */
  --naonur-ash: #9A978F;           /* Secondary text */
  --naonur-smoke: #5A5850;         /* Disabled text */
  
  /* Status */
  --naonur-moss: #4A7C59;          /* Success, granted */
  --naonur-rust: #8B4513;          /* Warning, pending */
  --naonur-blood: #8B0000;         /* Error, denied */
  --naonur-water: #4A7C8C;         /* Info, neutral */
}
```

### Typography

```css
/* Headings - Cinzel (Celtic/mystical feel) */
--font-display: 'Cinzel', serif;

/* Body - Cormorant Garamond (readable, elegant) */
--font-body: 'Cormorant Garamond', serif;

/* Code/Data - JetBrains Mono */
--font-mono: 'JetBrains Mono', monospace;
```

### Visual Motifs

- **Feathers** — Subtle feather watermarks (Suibhne's symbol)
- **Triquetra** — Celtic three-fold knot for navigation/loading
- **Threshold imagery** — Doorways, borders, liminal spaces
- **Particle effects** — Drifting motes like dust in light beams

---

## Goal 1: Permissions Proxy (PRIORITY)

### Overview

The core function of Tairseach. Provides a unified interface for macOS permissions so agents can request access through Tairseach rather than needing direct system access.

### Permissions to Track

| Permission | TCC Database Key | Critical? | Check Method |
|------------|-----------------|-----------|--------------|
| Contacts | `kTCCServiceAddressBook` | ✅ Yes | `CNContactStore.authorizationStatus` |
| Automation | `kTCCServiceAppleEvents` | ✅ Yes | `AEDeterminePermissionToAutomateTarget` |
| Full Disk Access | `kTCCServiceSystemPolicyAllFiles` | ✅ Yes | Probe `/Library/Application Support/com.apple.TCC/TCC.db` |
| Accessibility | `kTCCServiceAccessibility` | No | `AXIsProcessTrusted` |
| Screen Recording | `kTCCServiceScreenCapture` | No | `CGPreflightScreenCaptureAccess` |
| Calendar | `kTCCServiceCalendar` | No | `EKEventStore.authorizationStatus` |
| Reminders | `kTCCServiceReminders` | No | `EKEventStore.authorizationStatus` |
| Photos | `kTCCServicePhotos` | No | `PHPhotoLibrary.authorizationStatus` |
| Camera | `kTCCServiceCamera` | No | `AVCaptureDevice.authorizationStatus` |
| Microphone | `kTCCServiceMicrophone` | No | `AVCaptureDevice.authorizationStatus` |
| Location | `kTCCServiceLocation` | No | `CLLocationManager.authorizationStatus` |

### Rust Implementation

```rust
// src-tauri/src/permissions/mod.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionStatus {
    Granted,
    Denied,
    NotDetermined,
    Restricted,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: PermissionStatus,
    pub critical: bool,
    pub last_checked: Option<String>,
}

#[tauri::command]
pub fn check_permission(permission_id: &str) -> Result<Permission, String> {
    // Dispatch to specific permission checker
}

#[tauri::command]
pub fn request_permission(permission_id: &str) -> Result<Permission, String> {
    // Open System Preferences to appropriate pane
    // macOS doesn't allow programmatic grant—user must manually enable
}

#[tauri::command]
pub fn check_all_permissions() -> Vec<Permission> {
    // Return status of all tracked permissions
}
```

### Agent Integration

When an agent attempts an action requiring permissions:

```rust
// Permission check middleware for MCP commands
pub async fn with_permission_check<F, T>(
    permission_id: &str,
    action: F,
) -> Result<T, PermissionError>
where
    F: FnOnce() -> Result<T, Error>,
{
    let status = check_permission(permission_id)?;
    
    match status.status {
        PermissionStatus::Granted => action().map_err(PermissionError::ActionFailed),
        _ => {
            // Emit event to frontend to show popup
            emit_permission_needed(permission_id);
            
            // Return structured error for agent
            Err(PermissionError::NotGranted {
                permission: permission_id.to_string(),
                message: format!(
                    "Permission '{}' not granted. User must enable in System Preferences.",
                    status.name
                ),
            })
        }
    }
}
```

### UI Components

**PermissionsView.vue** — Main tab showing all permissions as cards

```
┌─────────────────────────────────────────────────────────────┐
│  🔐 Permissions                                             │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────┐│
│  │ 📇 Contacts     │  │ 🤖 Automation   │  │ 💾 Full Disk ││
│  │                 │  │                 │  │              ││
│  │   ● Granted     │  │   ○ Not Set     │  │  ○ Denied    ││
│  │                 │  │                 │  │              ││
│  │  [Granted ✓]    │  │  [Request]      │  │  [Request]   ││
│  └─────────────────┘  └─────────────────┘  └──────────────┘│
│                                                             │
│  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────┐│
│  │ ♿ Accessibility │  │ 🖥 Screen Rec   │  │ 📅 Calendar  ││
│  │   ...           │  │   ...           │  │   ...        ││
│  └─────────────────┘  └─────────────────┘  └──────────────┘│
│                                                             │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│  Status: 3 of 6 critical permissions granted                │
│  [Refresh All]  [Open System Preferences]                   │
└─────────────────────────────────────────────────────────────┘
```

### Tasks — Goal 1

| Task ID | Description | Agent | Count | Depends On |
|---------|-------------|-------|-------|------------|
| P-001 | Rust permission status check APIs (all 11 permissions) | FORGE | 1 | — |
| P-002 | Swift bridge for permission APIs (FFI) | FORGE | 1 | P-001 |
| P-003 | System Preferences deep-link launcher | FORGE | 1 | — |
| P-004 | Permission needed popup component | CANVAS | 1 | — |
| P-005 | PermissionCard.vue component | CANVAS | 1 | P-004 |
| P-006 | PermissionsView.vue full tab | CANVAS | 1 | P-005 |
| P-007 | Permission store (Pinia) | NEXUS | 1 | P-001 |
| P-008 | Tauri IPC commands for permissions | NEXUS | 1 | P-001, P-007 |
| P-009 | MCP permission-check middleware | CIPHER | 1 | P-001 |
| P-010 | Agent error message formatting | CIPHER | 1 | P-009 |

**Total: 10 tasks, 4 agents**

---

## Goal 2: Configuration Manager

### Overview

Visual editor for `~/.openclaw.json`. Improves on openclaw-monitor with:
- Schema-driven rendering (preserve)
- Client-side validation (improve)
- Dirty tracking per field (improve)
- Better array/object editing (improve)
- Diff view before save (improve)

### Schema Integration

```typescript
// Fetch from OpenClaw gateway
const schema = await invoke('fetch_config_schema');
const config = await invoke('fetch_config');

// Schema-aware rendering
function renderField(path: string, schema: JSONSchema, value: any) {
  const fieldType = inferFieldType(schema);
  const uiHints = schema['x-ui-hints'] || {};
  
  switch (fieldType) {
    case 'boolean': return <ToggleField />;
    case 'enum': return <SelectField options={schema.enum} />;
    case 'integer': return <NumberField min={schema.minimum} max={schema.maximum} />;
    case 'string': return uiHints.sensitive ? <PasswordField /> : <TextField />;
    case 'array': return <ArrayEditor itemSchema={schema.items} />;
    case 'object': return <ObjectEditor properties={schema.properties} />;
  }
}
```

### Validation

```typescript
// Pre-save validation using JSON Schema
import Ajv from 'ajv';

const ajv = new Ajv({ allErrors: true });
const validate = ajv.compile(schema);

function validateConfig(config: object): ValidationResult {
  const valid = validate(config);
  if (!valid) {
    return {
      valid: false,
      errors: validate.errors.map(e => ({
        path: e.instancePath,
        message: e.message,
      })),
    };
  }
  return { valid: true, errors: [] };
}
```

### UI Layout

```
┌─────────────────────────────────────────────────────────────┐
│  ⚙️ Configuration                                           │
├──────────────┬──────────────────────────────────────────────┤
│              │                                              │
│  📡 Gateway  │  Gateway Settings                            │
│  🤖 Agents   │  ─────────────────                           │
│  🔌 Channels │                                              │
│  💾 Memory   │  Port: [18789        ]                       │
│  🎯 Models   │  Token: [••••••••••••] 👁                    │
│  📚 Skills   │  Log Level: [info ▼]                         │
│  🔧 Advanced │                                              │
│              │  [ ] Enable Debug Mode                       │
│              │                                              │
│              │  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│              │                                              │
│              │  [Revert Changes]  [Validate]  [💾 Save]     │
└──────────────┴──────────────────────────────────────────────┘
```

### Tasks — Goal 2

| Task ID | Description | Agent | Count | Depends On |
|---------|-------------|-------|-------|------------|
| C-001 | Rust config file read/write with backup | FORGE | 1 | — |
| C-002 | Gateway schema fetch command | FORGE | 1 | — |
| C-003 | Config store (Pinia) with dirty tracking | NEXUS | 1 | — |
| C-004 | JSON Schema validation integration | NEXUS | 1 | C-002 |
| C-005 | ConfigField.vue (handles all field types) | CANVAS | 1 | — |
| C-006 | ArrayEditor.vue (add/remove/reorder) | CANVAS | 1 | C-005 |
| C-007 | ConfigSection.vue (collapsible sections) | CANVAS | 1 | C-005 |
| C-008 | ConfigView.vue full tab | CANVAS | 1 | C-006, C-007 |
| C-009 | Diff viewer before save | CANVAS | 1 | C-003 |
| C-010 | Config save with gateway restart | NEXUS | 1 | C-001 |

**Total: 10 tasks, 3 agents**

---

## Goal 3: MCP Server

### Overview

Built-in Model Context Protocol server that OpenClaw agents can use for efficient system operations. This runs as part of Tairseach, not a separate process.

### MCP Endpoints

| Tool | Description | Permission Required |
|------|-------------|---------------------|
| `tairseach.permissions.check` | Check permission status | None |
| `tairseach.permissions.request` | Request permission (opens UI) | None |
| `tairseach.config.get` | Get OpenClaw config value | None |
| `tairseach.config.set` | Set OpenClaw config value | None |
| `tairseach.contacts.list` | List contacts | Contacts |
| `tairseach.contacts.search` | Search contacts | Contacts |
| `tairseach.automation.run` | Run AppleScript | Automation |
| `tairseach.files.read` | Read protected file | Full Disk Access |
| `tairseach.files.write` | Write protected file | Full Disk Access |
| `tairseach.screenshot` | Capture screenshot | Screen Recording |
| `tairseach.calendar.events` | List calendar events | Calendar |

### Server Implementation

```rust
// src-tauri/src/mcp/server.rs

use tokio::net::TcpListener;
use serde_json::Value;

pub struct McpServer {
    listener: TcpListener,
    permission_checker: PermissionChecker,
}

impl McpServer {
    pub async fn start(port: u16) -> Result<Self, Error> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
        // ...
    }
    
    pub async fn handle_request(&self, request: McpRequest) -> McpResponse {
        // Check permissions before executing
        if let Some(required_permission) = request.tool.required_permission() {
            let status = self.permission_checker.check(required_permission)?;
            if status != PermissionStatus::Granted {
                return McpResponse::error(
                    "permission_denied",
                    format!("Permission '{}' required", required_permission),
                );
            }
        }
        
        // Execute tool
        match request.tool.as_str() {
            "tairseach.permissions.check" => self.handle_permissions_check(request.args),
            "tairseach.contacts.list" => self.handle_contacts_list(request.args),
            // ...
        }
    }
}
```

### Configuration

```json
// In ~/.openclaw.json
{
  "mcpServers": {
    "tairseach": {
      "command": "open",
      "args": ["-a", "Tairseach", "--args", "--mcp-stdio"],
      "env": {}
    }
  }
}
```

### Tasks — Goal 3

| Task ID | Description | Agent | Count | Depends On |
|---------|-------------|-------|-------|------------|
| M-001 | MCP protocol implementation (JSON-RPC) | CIPHER | 1 | — |
| M-002 | TCP listener with connection handling | CIPHER | 1 | M-001 |
| M-003 | STDIO mode for MCP integration | CIPHER | 1 | M-001 |
| M-004 | Permission-check middleware | CIPHER | 1 | P-001, M-001 |
| M-005 | Contacts tools (list, search) | FORGE | 1 | M-001 |
| M-006 | Automation tools (run AppleScript) | FORGE | 1 | M-001 |
| M-007 | File access tools (read, write) | FORGE | 1 | M-001 |
| M-008 | Screenshot tool | FORGE | 1 | M-001 |
| M-009 | Calendar tools | FORGE | 1 | M-001 |
| M-010 | MCP server settings UI | CANVAS | 1 | M-001 |

**Total: 10 tasks, 3 agents**

---

## Goal 4: Context Monitor

### Overview

Real-time token usage tracking similar to CodexBar. Shows usage per session, cost estimates, and alerts when approaching limits.

### Data Source

OpenClaw gateway exposes session data via:
- `sessions_list` — List active sessions
- `session_status` — Get usage for a session
- WebSocket events for real-time updates

### UI Design

```
┌─────────────────────────────────────────────────────────────┐
│  📊 Monitor                                                  │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Total Usage Today                     Cost: $4.23          │
│  ██████████████████░░░░░░░░░░ 68%     ───────────────────  │
│  136,000 / 200,000 tokens              $2.89 input          │
│                                        $1.34 output         │
│                                                             │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                                             │
│  Active Sessions                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ 🪶 suibhne:main           claude-opus  45,230 tokens │  │
│  │    Last: 2 min ago        ████████░░░░  45%         │  │
│  ├──────────────────────────────────────────────────────┤  │
│  │ 🌊 muirgen:slack:naonur   claude-opus  32,100 tokens │  │
│  │    Last: 15 min ago       █████░░░░░░░  32%         │  │
│  ├──────────────────────────────────────────────────────┤  │
│  │ 🔥 becuma:telegram        kimi         8,450 tokens  │  │
│  │    Last: 1 hour ago       ██░░░░░░░░░░  8%          │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  [Refresh]  [Export CSV]  [Clear History]                   │
└─────────────────────────────────────────────────────────────┘
```

### Tasks — Goal 4

| Task ID | Description | Agent | Count | Depends On |
|---------|-------------|-------|-------|------------|
| T-001 | Gateway sessions API integration | NEXUS | 1 | — |
| T-002 | WebSocket connection for real-time updates | NEXUS | 1 | T-001 |
| T-003 | Usage store (Pinia) with history | NEXUS | 1 | T-001 |
| T-004 | UsageGauge.vue component | CANVAS | 1 | — |
| T-005 | SessionList.vue component | CANVAS | 1 | T-001 |
| T-006 | MonitorView.vue full tab | CANVAS | 1 | T-004, T-005 |
| T-007 | Cost calculation utilities | NEXUS | 1 | — |
| T-008 | CSV export functionality | NEXUS | 1 | T-003 |
| T-009 | Usage alerts/notifications | CANVAS | 1 | T-003 |

**Total: 9 tasks, 2 agents**

---

## Goal 5: Agent Profiles

### Overview

Visual identity management for agents. Store profile pictures and metadata in `~/.tairseach/profiles/`.

### Profile Structure

```json
// ~/.tairseach/profiles/suibhne/profile.json
{
  "agentId": "suibhne",
  "displayName": "Buile Suibhne",
  "avatarPath": "avatar.png",
  "emoji": "🪶",
  "color": "#C9A227",
  "description": "The mad king, now digital geilt",
  "createdAt": "2026-01-25T00:00:00Z",
  "updatedAt": "2026-02-03T00:00:00Z"
}
```

### UI Design

```
┌─────────────────────────────────────────────────────────────┐
│  👤 Agent Profiles                                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐   │
│  │   [Avatar]    │  │   [Avatar]    │  │   [Avatar]    │   │
│  │      🪶       │  │      🌊       │  │      🔥       │   │
│  │               │  │               │  │               │   │
│  │   Suibhne     │  │   Muirgen     │  │   Becuma      │   │
│  │  claude-opus  │  │  claude-opus  │  │    kimi       │   │
│  │               │  │               │  │               │   │
│  │    [Edit]     │  │    [Edit]     │  │    [Edit]     │   │
│  └───────────────┘  └───────────────┘  └───────────────┘   │
│                                                             │
│  ┌───────────────┐                                          │
│  │      ➕       │                                          │
│  │   Add Agent   │                                          │
│  └───────────────┘                                          │
│                                                             │
└─────────────────────────────────────────────────────────────┘

Edit Modal:
┌─────────────────────────────────────────┐
│  Edit Profile: Suibhne                  │
├─────────────────────────────────────────┤
│                                         │
│  ┌─────────┐  Display Name:             │
│  │ [image] │  [Buile Suibhne        ]   │
│  │   🪶    │                            │
│  │ Change  │  Emoji: [🪶]               │
│  └─────────┘                            │
│              Color: [█ #C9A227]         │
│                                         │
│              Description:               │
│              [The mad king, now...]     │
│                                         │
│         [Cancel]  [Save]                │
└─────────────────────────────────────────┘
```

### Tasks — Goal 5

| Task ID | Description | Agent | Count | Depends On |
|---------|-------------|-------|-------|------------|
| A-001 | Profile storage module (Rust) | FORGE | 1 | — |
| A-002 | Avatar image handling (resize, save) | FORGE | 1 | A-001 |
| A-003 | Profiles store (Pinia) | NEXUS | 1 | A-001 |
| A-004 | AgentCard.vue component | CANVAS | 1 | — |
| A-005 | AvatarUpload.vue component | CANVAS | 1 | A-002 |
| A-006 | ProfileEditModal.vue | CANVAS | 1 | A-004, A-005 |
| A-007 | ProfilesView.vue full tab | CANVAS | 1 | A-004, A-006 |
| A-008 | Sync with OpenClaw agent configs | NEXUS | 1 | A-001 |

**Total: 8 tasks, 3 agents**

---

## Goal 6: Auth Broker (OAuth Persistence)

### Overview

CLI tools like GOG (Google Workspace CLI) have a persistent problem: each CLI invocation is a new process, so macOS "Always Allow" prompts for Keychain access don't persist. The user has to click "Allow" repeatedly.

**The Problem:**
```
$ gog gmail send ...
[macOS Keychain prompt: "gog wants to access your login keychain"]
→ User clicks "Always Allow"
→ Next invocation: same prompt appears again (new process, new permission check)
```

**The Solution:**
Tairseach acts as a persistent OAuth broker. CLI tools communicate with Tairseach (which is already running and has Keychain access granted once), and Tairseach handles the actual OAuth token management.

### Architecture

```
┌─────────────────┐     ┌─────────────────────────────────────┐
│   CLI Tool      │     │           Tairseach.app              │
│   (gog, etc.)   │     │                                      │
│                 │     │  ┌──────────────────────────────┐   │
│  $ tairseach    │────►│  │       Auth Broker             │   │
│    gog gmail    │     │  │  ┌─────────┐  ┌───────────┐  │   │
│    send ...     │     │  │  │ Token   │  │ OAuth     │  │   │
│                 │◄────│  │  │ Store   │  │ Refresh   │  │   │
│  (gets result)  │     │  │  │(Keychn) │  │ Logic     │  │   │
│                 │     │  │  └─────────┘  └───────────┘  │   │
└─────────────────┘     │  └──────────────────────────────┘   │
                        │                                      │
                        │  macOS Keychain (accessed ONCE)     │
                        └─────────────────────────────────────┘
```

### Supported Services

| Service | CLI Tool | OAuth Scopes |
|---------|----------|--------------|
| Google Workspace | GOG | Gmail, Calendar, Drive, Contacts |
| Microsoft 365 | (future) | Outlook, OneDrive |
| GitHub | gh (passthrough) | repo, gist, etc. |

### Token Storage

Tokens stored in **macOS Keychain** via Security framework:
- Service: `com.tairseach.oauth.<provider>`
- Account: User's email/ID
- Encrypted at rest by macOS

```rust
// src-tauri/src/auth/tokens.rs

use security_framework::keychain::SecKeychain;

pub struct TokenStore {
    keychain: SecKeychain,
}

impl TokenStore {
    pub fn get_token(&self, service: &str) -> Result<OAuthToken, Error> {
        let item = self.keychain.find_generic_password(
            &format!("com.tairseach.oauth.{}", service),
            &self.account_id,
        )?;
        serde_json::from_slice(&item.password)
    }
    
    pub fn store_token(&self, service: &str, token: &OAuthToken) -> Result<(), Error> {
        self.keychain.set_generic_password(
            &format!("com.tairseach.oauth.{}", service),
            &self.account_id,
            &serde_json::to_vec(token)?,
        )
    }
    
    pub async fn refresh_if_needed(&self, service: &str) -> Result<OAuthToken, Error> {
        let token = self.get_token(service)?;
        if token.is_expired() {
            let refreshed = self.refresh_token(service, &token.refresh_token).await?;
            self.store_token(service, &refreshed)?;
            Ok(refreshed)
        } else {
            Ok(token)
        }
    }
}
```

### CLI Passthrough

CLI tools invoke Tairseach instead of directly calling the service:

```bash
# Instead of:
$ gog gmail send --to foo@bar.com --subject "Hello" --body "World"

# Use:
$ tairseach gog gmail send --to foo@bar.com --subject "Hello" --body "World"

# Or configure GOG to use Tairseach as its credential helper
```

Tairseach's CLI interface:

```bash
# Passthrough to GOG with Tairseach-managed credentials
tairseach gog <gog-args>

# Direct token management
tairseach auth status              # Show connected services
tairseach auth connect google      # Initiate OAuth flow
tairseach auth disconnect google   # Revoke and remove tokens
tairseach auth refresh google      # Force token refresh
```

### OAuth Flow

1. User clicks "Connect Google" in Tairseach UI
2. Tairseach opens browser to Google OAuth consent
3. Google redirects to `tairseach://oauth/callback?code=...`
4. Tairseach exchanges code for tokens
5. Tokens stored in macOS Keychain
6. macOS prompts ONCE for Keychain access
7. User clicks "Always Allow"
8. All future CLI invocations use Tairseach → no more prompts

### UI Design

```
┌─────────────────────────────────────────────────────────────┐
│  🔑 Connected Services                                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  🔵 Google Workspace                    ● Connected  │   │
│  │     geiltalasdair@gmail.com                         │   │
│  │     Scopes: Gmail, Calendar, Drive, Contacts        │   │
│  │     Token expires: 47 minutes                       │   │
│  │     Last used: 2 minutes ago                        │   │
│  │                                                     │   │
│  │     [Refresh Token]  [Disconnect]                   │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  ⬜ Microsoft 365                       ○ Not Setup  │   │
│  │                                                     │   │
│  │     [Connect Microsoft Account]                     │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                                             │
│  CLI Usage:                                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  $ tairseach gog gmail send --to ...                │   │
│  │  $ tairseach gog calendar list                      │   │
│  │  $ tairseach gog drive list                         │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### GOG Integration

Update GOG to optionally use Tairseach as credential provider:

```bash
# GOG config (~/.config/gog/config.yaml)
credential_helper: tairseach

# Or environment variable
GOG_CREDENTIAL_HELPER=tairseach gog gmail send ...
```

When configured, GOG calls:
```bash
tairseach auth get-token google
```

And receives a valid access token on stdout.

### Tasks — Goal 6

| Task ID | Description | Agent | Count | Depends On |
|---------|-------------|-------|-------|------------|
| O-001 | Keychain token storage module (Rust) | FORGE | 1 | — |
| O-002 | Google OAuth flow implementation | CIPHER | 1 | O-001 |
| O-003 | Token refresh logic | CIPHER | 1 | O-001, O-002 |
| O-004 | Deep link handler (`tairseach://`) | FORGE | 1 | — |
| O-005 | CLI passthrough command (`tairseach gog ...`) | FORGE | 1 | O-001 |
| O-006 | Auth store (Pinia) | NEXUS | 1 | O-001 |
| O-007 | ServiceCard.vue component | CANVAS | 1 | — |
| O-008 | OAuthConnect.vue (OAuth flow UI) | CANVAS | 1 | O-002 |
| O-009 | AuthView.vue full tab | CANVAS | 1 | O-007, O-008 |
| O-010 | Token status display (expiry, last used) | CANVAS | 1 | O-006 |
| O-011 | GOG credential helper integration | NEXUS | 1 | O-005 |
| O-012 | Microsoft OAuth (future, stub) | CIPHER | 1 | O-002 |

**Total: 12 tasks, 4 agents**

---

## Shared Infrastructure Tasks

| Task ID | Description | Agent | Count | Depends On |
|---------|-------------|-------|-------|------------|
| S-001 | Tauri project scaffold with Vue 3 | NEXUS | 1 | — |
| S-002 | Naonúr theme CSS (colors, typography) | CANVAS | 1 | — |
| S-003 | Tab navigation component | CANVAS | 1 | S-002 |
| S-004 | Dashboard view (overview) | CANVAS | 1 | S-003 |
| S-005 | Toast notification system | CANVAS | 1 | — |
| S-006 | App icon design (Naonúr themed) | CANVAS | 1 | — |
| S-007 | GitHub repo setup + CI | NEXUS | 1 | S-001 |
| S-008 | README and documentation | ECHO | 1 | All |

**Total: 8 tasks, 3 agents**

---

## Task Summary

| Goal | Tasks | Primary Agents |
|------|-------|----------------|
| **Shared Infrastructure** | 8 | NEXUS, CANVAS, ECHO |
| **1. Permissions Proxy** | 10 | FORGE, CANVAS, NEXUS, CIPHER |
| **2. Configuration Manager** | 10 | FORGE, NEXUS, CANVAS |
| **3. MCP Server** | 10 | CIPHER, FORGE, CANVAS |
| **4. Context Monitor** | 9 | NEXUS, CANVAS |
| **5. Agent Profiles** | 8 | FORGE, NEXUS, CANVAS |
| **6. Auth Broker** | 12 | FORGE, CIPHER, NEXUS, CANVAS |
| **Total** | **67 tasks** | |

### Agent Allocation

| Agent | Specialization | Task Count |
|-------|----------------|------------|
| **FORGE** | Rust backend, system APIs, FFI | 15 |
| **CANVAS** | Vue components, UI/UX, styling | 22 |
| **NEXUS** | State management, integrations, IPC | 16 |
| **CIPHER** | MCP protocol, security, OAuth | 9 |
| **ECHO** | Documentation | 1 |

### Recommended Parallel Execution

**Phase 1: Foundation** (S-001 → S-007)
- NEXUS: Tauri scaffold, stores setup
- CANVAS: Theme, navigation, dashboard

**Phase 2: Permissions** (P-001 → P-010) — PRIORITY
- FORGE: Rust permission APIs
- CANVAS: Permission UI components
- NEXUS: IPC bridge

**Phase 3: Config + Monitor** (C-001 → C-010, T-001 → T-009)
- FORGE: Config file handling
- NEXUS: Gateway integration
- CANVAS: Config + Monitor views

**Phase 4: MCP + Profiles + Auth** (M-001 → M-010, A-001 → A-008, O-001 → O-012)
- CIPHER: MCP server + OAuth flows
- FORGE: System tools + Keychain integration
- CANVAS: Profile UI + Auth UI
- NEXUS: GOG integration

---

## Open Questions

1. **MCP Port** — What port should the MCP server listen on? Default 18799?
2. **Gateway Auth** — How should Tairseach authenticate with the gateway? Same token as CLI?
3. **Auto-start** — Should Tairseach auto-start on login? Menu bar mode?
4. **Update mechanism** — How will Tairseach update itself?

---

## References

- [Tauri 2.0 Documentation](https://tauri.app/v2/)
- [macOS TCC Database](https://www.rainforestqa.com/blog/macos-tcc-db-deep-dive)
- [MCP Protocol Spec](https://modelcontextprotocol.io)
- [OpenClaw Documentation](https://docs.openclaw.ai)
- [Dieah Project](~/environment/dieah/) — UI scaffold reference
- [openclaw-monitor](~/environment/openclaw-monitor/) — Config manager reference

---

*🪶 The threshold awaits.*
