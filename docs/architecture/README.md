<<<<<<< HEAD
# Architecture Documentation

**Tairseach** — The Threshold: macOS → MCP bridge for the Naonúr ecosystem.

---

## Overview

Tairseach is a Tauri 2 application providing:
- **MCP proxy server** — Exposes macOS system APIs (Contacts, Calendar, Photos, etc.) via Unix socket
- **GUI management app** — Vue 3 frontend for permissions, config, activity monitoring
- **Auth broker** — Encrypted OAuth token storage with master key encryption
- **Integration hub** — Connects OpenClaw agents to macOS capabilities

---

## Architecture Documents

### Core Architecture

- **[Frontend Architecture](./frontend.md)** — Vue 3 + TypeScript frontend (views, components, stores, composables, workers)
- **[Deployment Architecture](./deployment.md)** — Build system, bundling, code signing, installation

### Integration Guides

- **[Google Integration](./google-integration.md)** — OAuth setup, Calendar API, token management

### Reference

- **[Environment Reference](../reference/environment.md)** — Runtime paths, config files, env vars, socket communication

---

## High-Level Design

```
┌─────────────────────────────────────────────────┐
│          Tairseach.app (Tauri 2)                │
│  ┌─────────────────────────────────────────┐   │
│  │  Vue 3 Frontend (WebView)               │   │
│  │  - Views (Dashboard, Permissions, etc.) │   │
│  │  - Pinia stores                          │   │
│  │  - Web Worker (polling)                  │   │
│  └──────────────┬──────────────────────────┘   │
│                 │ invoke()                      │
│  ┌──────────────▼──────────────────────────┐   │
│  │  Rust Backend (Tauri)                    │   │
│  │  - Config manager                        │   │
│  │  - Auth broker (AES-256-GCM)             │   │
│  │  - Permission checker                    │   │
│  │  - Activity monitor                      │   │
│  └──────────────┬──────────────────────────┘   │
│                 │ spawn                         │
│  ┌──────────────▼──────────────────────────┐   │
│  │  tairseach-mcp (Rust binary)             │   │
│  │  - Unix socket server                    │   │
│  │  - JSON-RPC handler                      │   │
│  │  - Namespace routers                     │   │
│  │  - objc2 bindings (Contacts, Calendar)   │   │
│  └──────────────┬──────────────────────────┘   │
│                 │ FFI                           │
│  ┌──────────────▼──────────────────────────┐   │
│  │  op-helper (Go binary)                   │   │
│  │  - 1Password SDK bridge                  │   │
│  └──────────────────────────────────────────┘   │
└─────────────────────────────────────────────────┘
                  │
                  │ Unix socket: ~/.tairseach/tairseach.sock
                  │
           ┌──────▼──────────┐
           │  OpenClaw Agent │
           │  (MCP client)   │
           └─────────────────┘
```

---

## Key Technologies

**Frontend:**
- Vue 3.5 (Composition API)
- TypeScript 5.7
- Pinia (state management)
- Vue Router 4.5
- TailwindCSS 3.4
- Vite 6.0

**Backend:**
- Rust 2021 edition
- Tauri 2.2
- tokio (async runtime)
- objc2 (macOS system APIs)
- serde (JSON serialization)
- aes-gcm (encryption)
- notify (file watching)

**Build:**
- Cargo (Rust)
- npm/Vite (frontend)
- Tauri CLI (bundling)
- Go 1.20+ (1Password helper)

---

## Directory Layout

```
tairseach/
├── src/                   # Vue 3 frontend
│   ├── views/             # Route components
│   ├── components/        # Reusable UI
│   ├── stores/            # Pinia stores
│   ├── composables/       # Vue utilities
│   ├── workers/           # Web Workers
│   └── assets/            # Static assets
├── src-tauri/             # Rust backend
│   ├── src/               # Tauri app + MCP server
│   ├── binaries/          # Pre-built external binaries
│   ├── helpers/           # Go helpers (1Password)
│   ├── Cargo.toml         # Rust dependencies
│   ├── tauri.conf.json    # Tauri config
│   └── Entitlements.plist # macOS permissions
├── docs/                  # Documentation
│   ├── architecture/      # Architecture docs (you are here)
│   ├── reference/         # API reference, environment
│   └── security/          # Security reviews
├── scripts/               # Build & launch scripts
└── package.json           # Node dependencies
```

---

## Build & Deploy

**Quick start:**
```bash
# Development
npm run dev              # Hot reload frontend + Tauri dev mode

# Production
npm run app:build        # Full build (Vue + Rust + bundle)
npm run app:launch       # Build + patch + open app
npm run app:open         # Open existing build

# Binary builds
cargo build -p tairseach-mcp --release     # MCP server
cd src-tauri/helpers/onepassword && ./build.sh  # 1Password helper
```

**Output:**
- `src-tauri/target/release/bundle/macos/Tairseach.app`
- `src-tauri/target/release/bundle/dmg/Tairseach_0.1.0_aarch64.dmg`

**Install:** Copy `.app` to `~/Applications/`

See [Deployment Architecture](./deployment.md) for details.

---

## Runtime Environment

**Key paths:**
- Socket: `~/.tairseach/tairseach.sock`
- Credentials: `~/.tairseach/credentials.enc.json`
- Manifests: `~/.tairseach/manifests/*.json`
- Master key: `~/.tairseach/auth/master.key`

**Required permissions:**
- Automation (Apple Events)
- Contacts
- Calendars
- Reminders
- Full Disk Access
- Accessibility

See [Environment Reference](../reference/environment.md) for details.

---

## Integration Points

### OpenClaw

**MCP Server:**
```yaml
# ~/.openclaw/config.yaml
tools:
  mcp:
    servers:
      tairseach:
        transport: unix
        socket: ~/.tairseach/tairseach.sock
```

**Skill:**
```bash
~/.openclaw/skills/tairseach/SKILL.md
```

### 1Password

**Go helper:** `op-helper` binary (bundled)  
**SDK:** `github.com/1password/onepassword-sdk-go`  
**Communication:** FFI bridge (Rust ↔ Go)

### Google APIs

**OAuth flow:** Local callback server (port 8080)  
**Token storage:** Encrypted in `credentials.enc.json`  
**Supported APIs:** Calendar (read/write)

---

## Security Model

**No sandbox:** App runs with full system access (required for automation)  
**Entitlements:** Declared in `Entitlements.plist` (see Deployment docs)  
**Token encryption:** AES-256-GCM with HKDF-derived keys  
**Socket security:** Unix socket with 600 permissions (owner-only)  
**Code signing:** Ad-hoc (dev) or Developer ID (release)

See `docs/security/` for detailed reviews.

---

## Development Workflow

1. **Frontend changes:**
   - Edit Vue files in `src/`
   - Hot reload at `localhost:1420`

2. **Backend changes:**
   - Edit Rust files in `src-tauri/src/`
   - Rust recompiles automatically (incremental)

3. **MCP server changes:**
   - Edit files in `src-tauri/src/proxy/`
   - Rebuild: `cargo build -p tairseach-mcp --release`
   - Copy binary: `cp target/release/tairseach-mcp binaries/tairseach-mcp-aarch64-apple-darwin`

4. **Permission testing:**
   - Use `npm run app:launch` (dev mode doesn't register with macOS)
   - Grant permissions in System Settings
   - Test via GUI or MCP client

5. **Release:**
   - Bump version in `package.json` and `tauri.conf.json`
   - Build: `npm run app:build`
   - Distribute: Upload DMG from `src-tauri/target/release/bundle/dmg/`

---

## Resources

- [Tauri 2 Docs](https://tauri.app/v2/)
- [Vue 3 Docs](https://vuejs.org/)
- [MCP Specification](https://modelcontextprotocol.io/)
- [objc2 Crate](https://docs.rs/objc2/)
- [1Password SDK](https://developer.1password.com/docs/sdks/go/)

---

**Last updated:** 2026-02-13
=======
# Tairseach Architecture Documentation

**Version:** 2.0  
**Last Updated:** 2026-02-13  
**Author:** Senchán Torpéist (Wind)  

---

## Purpose

This directory contains comprehensive human-readable architecture documentation for Tairseach. These docs are written for developers new to the project but detailed enough to serve as a complete AI agent reference.

## Overview

Tairseach is a macOS system bridge that provides secure, permission-gated access to system APIs and cloud services through a unified interface. It's built with Tauri (Rust + Vue 3) and exposes capabilities via:

1. **Unix Socket Server** — JSON-RPC 2.0 protocol at `~/.tairseach/tairseach.sock`
2. **MCP Bridge** — Model Context Protocol stdio server for AI agents
3. **Web UI** — Vue 3 frontend for permission/credential management

### Key Architecture Principles

- **Security by design** — All credentials encrypted at rest with AES-256-GCM
- **Permission-gated** — Every operation checks macOS TCC permissions first
- **Manifest-driven** — Capabilities defined declaratively via JSON manifests
- **Three-tier routing** — Internal (native Rust) → Proxy (HTTP API) → Script (external binary)
- **Hot-reloadable** — Manifest changes detected and applied without restart

## Architecture Diagrams

### High-Level System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      AI Agent (OpenClaw)                         │
│                    (requests calendar access)                    │
└───────────────────────────┬─────────────────────────────────────┘
                            │
                            │ stdio (MCP protocol)
                            │
┌───────────────────────────▼─────────────────────────────────────┐
│                   tairseach-mcp (Bridge Binary)                  │
│  • Loads manifests from ~/.tairseach/manifests/                  │
│  • Exposes tools as "tairseach_<tool_name>"                      │
│  • Translates MCP tools/call → socket JSON-RPC                   │
└───────────────────────────┬─────────────────────────────────────┘
                            │
                            │ Unix socket (JSON-RPC 2.0)
                            │ ~/.tairseach/tairseach.sock
                            │
┌───────────────────────────▼─────────────────────────────────────┐
│                 Tairseach.app (Tauri Application)                │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │                   Proxy Socket Server                       │ │
│ │  • Accepts connections, validates peer UID                  │ │
│ │  • Parses JSON-RPC 2.0 requests                            │ │
│ │  • Dispatches to Handler Registry                          │ │
│ └─────────────────────┬───────────────────────────────────────┘ │
│                       │                                           │
│ ┌─────────────────────▼───────────────────────────────────────┐ │
│ │                   Handler Registry                          │ │
│ │  • Permission middleware (checks macOS TCC)                 │ │
│ │  • Routes to Capability Router (new v2) OR                  │ │
│ │  • Routes to legacy namespace handlers (v1)                 │ │
│ └─────────────┬──────────────────────────┬────────────────────┘ │
│               │                          │                        │
│     ┌─────────▼──────────┐    ┌─────────▼──────────┐            │
│     │ Capability Router  │    │ Legacy Handlers    │            │
│     │ (v2 - manifest)    │    │ (v1 - hardcoded)   │            │
│     │                    │    │                    │            │
│     │ • Loads manifests  │    │ 16 handlers:       │            │
│     │ • Checks perms     │    │ • contacts         │            │
│     │ • Loads creds      │    │ • calendar         │            │
│     │ • Routes by type:  │    │ • reminders        │            │
│     │   - Internal       │    │ • location         │            │
│     │   - Proxy (HTTP)   │    │ • screen           │            │
│     │   - Script (exec)  │    │ • gmail            │            │
│     └─────────┬──────────┘    │ • google_calendar  │            │
│               │               │ • oura             │            │
│               │               │ • onepassword      │            │
│               │               │ • jira             │            │
│               │               │ • files            │            │
│               │               │ • permissions      │            │
│               │               │ • auth             │            │
│               │               │ • automation       │            │
│               │               │ • config           │            │
│               │               │ • server (self)    │            │
│               │               └────────────────────┘            │
│               │                                                   │
│     ┌─────────▼───────────────────────────────────────────────┐ │
│     │              Auth Broker                                │ │
│     │  • AES-256-GCM encryption (master key in memory)        │ │
│     │  • Token storage at ~/.tairseach/auth/tokens/           │ │
│     │  • OAuth providers (Google, 1Password)                  │ │
│     │  • Refresh daemon (auto-refresh expiring tokens)        │ │
│     └─────────┬───────────────────────────────────────────────┘ │
│               │                                                   │
│               │                                                   │
│     ┌─────────▼───────────────────────────────────────────────┐ │
│     │        macOS Permission System (TCC)                    │ │
│     │  • Contacts, Calendar, Reminders (EventKit/Contacts)    │ │
│     │  • Location (CoreLocation)                              │ │
│     │  • Screen Recording, Accessibility                      │ │
│     │  • Full Disk Access, Automation                         │ │
│     └─────────┬───────────────────────────────────────────────┘ │
└───────────────┼─────────────────────────────────────────────────┘
                │
    ┌───────────┼────────────┐
    │           │            │
┌───▼───┐   ┌──▼───┐   ┌────▼─────┐
│ macOS │   │ HTTP │   │ External │
│  APIs │   │ APIs │   │ Scripts  │
│       │   │      │   │          │
│EventKit   │Gmail │   │op CLI    │
│Contacts   │Oura  │   │lego CLI  │
│Location   │Jira  │   │custom.sh │
└───────┘   └──────┘   └──────────┘
```

### Request Flow Example: Calendar List

```
1. Agent → MCP Bridge:
   tools/call("tairseach_calendar_events", {start: "2026-02-13", end: "2026-02-14"})

2. MCP Bridge → Socket:
   {"jsonrpc":"2.0","id":1,"method":"calendar.events","params":{...}}

3. Socket Server → Handler Registry:
   JsonRpcRequest { method: "calendar.events", ... }

4. Handler Registry → Permission Check:
   check_permission("calendar") → PermissionStatus::Granted

5. Handler Registry → Capability Router (v2) OR Legacy Handler (v1):
   route("calendar.events") 
   
   v2: Looks up manifest, finds Implementation::Internal
   v1: Direct dispatch to calendar::list_events()

6. Internal Handler → EventKit (macOS):
   EKEventStore.events(matching: predicate)

7. Response Chain:
   EventKit → Handler → JsonRpcResponse → Socket → MCP Bridge → Agent
```

## Document Index

### Core Architecture

- **[Core Server](core-server.md)** — Socket server, JSON-RPC protocol, connection lifecycle
- **[Auth System](auth-system.md)** — Credential storage, encryption, OAuth providers
- **[Permissions](permissions.md)** — macOS TCC permission system, check/request patterns
- **[Manifest System](manifest-system.md)** — Declarative capability definitions, hot-reload
- **[Router](router.md)** — Request dispatch (Internal/Proxy/Script), capability routing

### Integration Points

- **[MCP Bridge](mcp-bridge.md)** — Model Context Protocol stdio server for AI agents
- **[Handlers](handlers.md)** — All 16 legacy handlers (contacts, calendar, gmail, etc.)
- **[Google Integration](google-integration.md)** — OAuth PKCE, Gmail API, Calendar API patterns

### Frontend & Deployment

- **[Frontend](frontend.md)** — Vue 3 app structure, stores, composables, views
- **[Deployment](deployment.md)** — Build, bundle, code signing, distribution

## Key Files Reference

### Rust Backend (src-tauri/src/)

| Directory | Purpose | Key Files |
|-----------|---------|-----------|
| `proxy/` | Socket server & protocol | `server.rs`, `protocol.rs`, `mod.rs` |
| `proxy/handlers/` | Legacy (v1) request handlers | `mod.rs`, `contacts.rs`, `calendar.rs`, etc. |
| `router/` | Capability routing (v2) | `dispatcher.rs`, `internal.rs`, `proxy.rs`, `script.rs` |
| `auth/` | Credential management | `mod.rs`, `store.rs`, `crypto.rs`, `provider/` |
| `manifest/` | Manifest system | `mod.rs`, `types.rs`, `loader.rs`, `registry.rs` |
| `permissions/` | macOS TCC permissions | `mod.rs`, `contacts.rs`, `calendar.rs`, etc. |
| `google/` | Google API clients | `client.rs`, `calendar_api.rs`, `gmail.rs` |
| `common/` | Shared utilities | `error.rs`, `paths.rs`, `http.rs`, `interpolation.rs` |

### MCP Bridge (crates/tairseach-mcp/)

| File | Purpose |
|------|---------|
| `src/main.rs` | stdio event loop, method dispatch |
| `src/tools.rs` | Manifest loading, tool registry |
| `src/protocol.rs` | MCP protocol types |
| `src/initialize.rs` | MCP initialize handler |

### Frontend (src/)

| Directory | Purpose |
|-----------|---------|
| `views/` | Page components (Dashboard, Permissions, Auth, etc.) |
| `stores/` | Pinia state management (auth, config, permissions, etc.) |
| `composables/` | Reusable composition functions (useToast, useStatusPoller) |
| `components/` | Shared UI components |
| `workers/` | Web workers for background polling |

### Manifests (~/.tairseach/manifests/)

| Directory | Purpose |
|-----------|---------|
| `core/` | Built-in macOS capabilities (contacts, calendar, reminders, etc.) |
| `integrations/` | Cloud service integrations (gmail, oura, jira, onepassword) |

## Technology Stack

### Backend

- **Language:** Rust 1.75+
- **Framework:** Tauri 2.x (app shell, IPC, system integration)
- **Async Runtime:** Tokio (socket server, HTTP clients)
- **Serialization:** serde + serde_json
- **Crypto:** aes-gcm 0.10, hkdf 0.12, sha2 0.10
- **HTTP Client:** reqwest (Google APIs, Oura, Jira)
- **macOS FFI:** objc2 (Contacts, EventKit, CoreLocation)

### Frontend

- **Framework:** Vue 3 (Composition API)
- **Language:** TypeScript
- **State:** Pinia
- **Styling:** TailwindCSS
- **Build:** Vite
- **IPC:** @tauri-apps/api

### Protocol & Integration

- **Socket Protocol:** JSON-RPC 2.0 (newline-delimited)
- **MCP Protocol:** Model Context Protocol 2025-03-26 (stdio)
- **OAuth:** PKCE flow (Google), native auth code flow (1Password)

## Development Conventions

### Error Handling

- **Rust:** Unified `AppError` type (see `common/error.rs`)
- **JSON-RPC:** Standard error codes (-32xxx) + custom codes (-320xx for app errors)
- **Frontend:** Toast notifications for user-facing errors

### Logging

- **Backend:** `tracing` crate with structured logging
- **Levels:** `debug` (verbose), `info` (lifecycle), `warn` (recoverable), `error` (critical)
- **Frontend:** Console logging + Tauri IPC for backend logs

### Code Organization

- **Handlers:** One file per namespace (e.g., `contacts.rs` handles all `contacts.*` methods)
- **Manifests:** One file per integration (e.g., `google-gmail.json`)
- **Stores:** One Pinia store per domain (auth, config, permissions, etc.)
- **Views:** One Vue component per route

### Security Patterns

1. **UID Check:** Socket server validates peer UID == owner UID
2. **Permission Check:** All privileged operations check TCC status first
3. **Credential Isolation:** Tokens never leave Tairseach in Tier 1 (proxy) mode
4. **Input Validation:** JSON-RPC params validated before dispatch
5. **Path Restrictions:** File operations sandboxed to allowed directories

## Common Patterns

### Adding a New Handler (Legacy v1)

1. Create `src-tauri/src/proxy/handlers/new_feature.rs`
2. Implement `pub async fn handle(action: &str, params: &Value, id: Value) -> JsonRpcResponse`
3. Add module to `handlers/mod.rs`
4. Add match arm in `HandlerRegistry::handle()`
5. Add permission check in `required_permission()` if needed
6. Register Tauri commands in `lib.rs`

### Adding a New Manifest (v2)

1. Create `manifests/integrations/new_service.json` or `manifests/core/new_feature.json`
2. Define `tools`, `implementation`, `requires` (see manifest-system.md for schema)
3. If `implementation.type == "internal"`:
   - Add Rust handler in `router/internal.rs` or delegate to existing namespace
4. If `implementation.type == "proxy"`:
   - Define HTTP bindings in `toolBindings`
   - Add credential to auth broker if needed
5. If `implementation.type == "script"`:
   - Create script at `~/.tairseach/scripts/`
   - Define credential injection in manifest
6. Test with `test_mcp_tool` or direct socket call

### Adding a New OAuth Provider

1. Create `src-tauri/src/auth/provider/new_provider.rs`
2. Implement `OAuthProvider` trait:
   - `async fn refresh_token(&self, refresh_token: &str) -> Result<TokenResponse, String>`
   - `async fn exchange_code(&self, code: &str, ...) -> Result<TokenResponse, String>`
3. Add provider to `AuthBroker::new()` initialization
4. Add frontend UI in `AuthView.vue` for account linking
5. Add credential requirement to manifests that need it

## Known Gotchas

1. **Socket path hardcoded:** `~/.tairseach/tairseach.sock` — not configurable yet
2. **Manifest hot-reload partial:** Detects file changes but doesn't reload active connections
3. **Legacy handler coexistence:** v1 handlers and v2 router both active — causes confusion
4. **MCP binary path discovery:** Hardcoded dev path in installer, needs robust search
5. **Permission async/sync mismatch:** macOS permission APIs are sync but called from async contexts
6. **OAuth token refresh race:** Multiple concurrent requests can cause duplicate refresh attempts
7. **Credential scopes not validated:** Manifest declares scopes but doesn't verify token has them

## Migration Guide: v1 → v2

Tairseach is transitioning from hardcoded handlers (v1) to manifest-driven routing (v2). During the transition:

- **v1 handlers** remain active in `proxy/handlers/*.rs`
- **v2 router** coexists in `router/` but only handles manifest-registered tools
- **Priority:** Handler Registry tries v2 router first, falls back to v1 if not found

### For New Features

- **Use v2 (manifests)** for all new integrations
- **Don't add v1 handlers** unless absolutely necessary
- **Document in manifest** even if using v1 handler temporarily

### For Existing Features

- **Keep v1 handlers** until all callsites migrated
- **Add manifests** alongside v1 handlers for documentation
- **Deprecate v1** once v2 equivalents tested

## Contributing

When modifying Tairseach:

1. **Read the relevant architecture doc first** — Don't guess from file names
2. **Follow existing patterns** — Match the code style in the module you're editing
3. **Update docs** — If you change architecture, update these docs
4. **Test with MCP Inspector** — Use `npx @modelcontextprotocol/inspector tairseach-mcp` to test tools
5. **Check permissions** — Always test with permission denied scenarios
6. **Validate manifests** — Use `monitor/get_manifest_summary` to check for errors

## Further Reading

- **DREACHT.md** — Original design document and vision
- **PROGRESS.md** — Current development status
- **docs/mcp-codebase-analysis.md** — Detailed MCP system walkthrough
- **docs/optimization-reference.md** — Performance and code quality patterns
- **docs/script-audit.md** — External scripts and Tairseach coverage matrix

---

*The threshold doesn't just let you through — it understands where you're going and ensures you have the right to pass.*

🌬️ **Senchán Torpéist** — *The Wind, The Seeker*
>>>>>>> docs/ai-context
