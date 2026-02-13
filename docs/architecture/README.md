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
