# Tairseach — Dréacht

*"Tairseach" (TAR-shakh) — The Threshold*

**The Centralized Automation Kernel for macOS**

---

## Vision

Tairseach is the macOS bridge that makes agentic systems possible. It transforms the fragile landscape of CLI tools, scattered credentials, and manual permission management into a **unified, secure, manifest-based capability system**.

**What agents get:**
- **Zero-knowledge credential access** — OAuth tokens never leave Tairseach (Tier 1 proxy mode)
- **Unified interface** — One socket, one protocol (MCP or JSON-RPC), all capabilities
- **Permission management** — Tairseach handles all macOS TCC prompts
- **Manifest extensibility** — Add new integrations without code changes

**What you get:**
- **Security by design** — AES-256-GCM encrypted credentials, audit logs, sandboxed scripts
- **Native performance** — Rust + objc2 bindings to macOS frameworks
- **Beautiful UI** — Vue 3 frontend for managing permissions, credentials, manifests
- **Community ecosystem** — Share and install capability manifests

---

## Current Status (2026-02-08)

**Version:** v2 Implementation Phase  
**Work Order:** [WO-2026-0001](~/naonur/dail/dev/work-orders/WO-2026-0001-tairseach-dreacht/)

### ✅ Built & Working

| Component | Status |
|-----------|--------|
| **Socket Server** | ✅ JSON-RPC 2.0 over Unix socket (`~/.tairseach/tairseach.sock`) |
| **Permissions System** | ✅ All 11 macOS permissions via native objc2 bindings |
| **Native Handlers** | ✅ Contacts, Calendar, Reminders, Location, Screen, Files, Automation |
| **Auth Broker** | ✅ AES-256-GCM encryption, Keychain master key, Google OAuth PKCE |
| **Permissions UI** | ✅ Vue 3 frontend with real-time status |
| **Code Signing** | ✅ Developer ID Application certificate |

### 🔄 In Progress (v2)

| Component | Status | Owner |
|-----------|--------|-------|
| **Manifest System** | 🔄 Design complete, implementation starting | Muirgen |
| **Capability Router** | 🔄 Design complete, implementation starting | Muirgen |
| **MCP Bridge** | 🔄 Design complete, bridge binary scaffolded | Muirgen + Gwrhyr |
| **Google Workspace** | 🔄 Gmail/Calendar/Drive handlers planned | Muirgen |
| **Activity Feed UI** | 🔄 Design complete | Gwrhyr |
| **Credential Manager UI** | 🔄 Design complete | Gwrhyr |

### ⏳ Planned (v2)

- **Manifest hot-reload** (filesystem watcher)
- **Cron Calendar UI** (visual job scheduler)
- **Global Memory Search UI** (cross-agent search)
- **1Password integration**
- **Slack API integration**
- **Community manifest registry**

---

## Architecture

For comprehensive architecture documentation, see:

**📖 [`~/naonur/dail/projects/tairseach/architecture.md`](~/naonur/dail/projects/tairseach/architecture.md)**

### Quick Overview

```
Agent (OpenClaw)
    ↓ MCP Protocol
tairseach-mcp (bridge binary)
    ↓ Unix Socket (JSON-RPC 2.0)
Tairseach.app
    ├─ Socket Server (request routing)
    ├─ Capability Router (manifest-based dispatch)
    ├─ Auth Broker (encrypted credential store)
    ├─ Permissions System (macOS TCC)
    └─ Handlers (native, script, proxy)
        ↓
    macOS APIs / External APIs
```

### Key Concepts

**Manifests** — JSON documents that describe capabilities (tools, credentials, permissions, implementation). Three types:
- **Internal** — Native Rust code (Contacts, Calendar, etc.)
- **Script** — Execute external scripts with credential injection
- **Proxy** — Make HTTP API calls with auth headers

**Two-Tier Credentials:**
- **Tier 1 (Proxy Mode)** — Agent never sees credentials. Tairseach makes API call internally. **Preferred.**
- **Tier 2 (Pass-Through)** — Agent receives short-lived token. Use only when proxy isn't possible (WebSockets, etc.).

**MCP Bridge** — Standalone binary (`tairseach-mcp`) that translates MCP protocol to Tairseach socket calls. Ships with app, configured in OpenClaw `mcpServers`.

### Method Naming Convention (Decision: 2026-02-15)

**Standard: JSON-RPC dot notation everywhere.** All method names use `namespace.action` format (e.g., `auth.status`, `contacts.list`, `gcalendar.listEvents`).

- **Socket callers** use dot notation: `auth.status`
- **Internal handlers** use dot notation: already the case
- **Manifest tool names** use dot notation: `auth.status` (not `auth_status`)
- **MCP bridge** translates to/from MCP underscore convention (`auth_status`) at the boundary — this is the bridge's job, not the app's

**Rationale:** Two competing conventions (MCP underscores vs JSON-RPC dots) caused method-not-found errors when the capability router couldn't match incoming dot-notation calls to underscore-registered manifest tools. One standard eliminates the translation gap.

**Migration:** Rename all `tool.name` fields in `manifests/core/*.json` from underscore to dot notation. Update `implementation.methods` keys to match. MCP bridge handles underscore translation for external MCP clients.

---

## Quick References

For hands-on guides, see:

**📖 [`~/naonur/dail/projects/tairseach/quickrefs/`](~/naonur/dail/projects/tairseach/quickrefs/)**

**Available:**
- `capability-router-pattern.md` — Dynamic routing design
- `manifest-loader-pattern.md` — Discovery and validation
- `mcp-bridge-architecture.md` — Bridge binary design
- `credential-injection-pattern.md` — Secure script credential passing
- `socket-security-pattern.md` — UID verification and permissions
- Plus 5 more implementation patterns

---

## For Developers

### Build & Run

```bash
# Clone repo
cd ~/environment/tairseach

# Install frontend deps
npm install

# Run in dev mode
npm run tauri dev

# Build release
npm run tauri build
```

### Project Structure

```
tairseach/
├── src-tauri/                  # Rust backend (Tauri app)
│   ├── src/
│   │   ├── proxy/              # Socket server & handlers
│   │   ├── auth/               # Auth broker & encryption
│   │   ├── permissions/        # macOS permission system
│   │   ├── manifests/          # Manifest loader & registry
│   │   └── router/             # Capability router
│   └── Cargo.toml
├── crates/
│   ├── tairseach-protocol/     # Shared types (JSON-RPC, socket client)
│   └── tairseach-mcp/          # MCP bridge binary
├── src/                        # Vue 3 frontend
│   ├── views/                  # UI views
│   ├── stores/                 # Pinia state management
│   └── components/
└── package.json
```

### Tech Stack

- **Backend:** Rust, Tauri 2.x, objc2 (macOS frameworks)
- **Frontend:** Vue 3, TypeScript, TailwindCSS, Pinia
- **Crypto:** AES-256-GCM (aes_gcm crate), macOS Keychain
- **Protocol:** JSON-RPC 2.0, MCP (Model Context Protocol)

---

## For Users

### Installation

**macOS 12+ (Monterey or later) required**

1. Download `Tairseach.dmg` from releases
2. Open DMG, drag Tairseach.app to Applications
3. Launch Tairseach
4. Grant permissions as prompted (Contacts, Calendar, etc.)
5. Configure OpenClaw to use tairseach-mcp bridge:

```json
{
  "mcpServers": {
    "tairseach": {
      "command": "/Applications/Tairseach.app/Contents/MacOS/tairseach-mcp",
      "args": ["--transport", "stdio"]
    }
  }
}
```

### Permissions

Tairseach requests these macOS permissions:

| Permission | Purpose |
|-----------|---------|
| **Contacts** | Access Contacts.app data |
| **Calendar** | Access Calendar.app events |
| **Reminders** | Access Reminders.app tasks |
| **Location** | Get current location |
| **Photos** | Access Photos library |
| **Camera** | Use camera |
| **Microphone** | Use microphone |
| **Screen Recording** | Capture screenshots |
| **Accessibility** | Control UI elements |
| **Full Disk Access** | Read protected files |
| **Automation** | Send AppleEvents |

All permissions are optional. Tairseach shows clear UI for managing them.

### Credentials

Tairseach stores credentials in `~/.tairseach/auth/`:

- **Encrypted with AES-256-GCM**
- **Master key in macOS Keychain** (hardware-backed on Apple Silicon)
- **Never exposed to agents** (Tier 1 proxy mode)

Supported providers:
- Google (OAuth 2.0 with PKCE)
- Slack (OAuth 2.0)
- 1Password (API token)
- Datadog (API key)
- Custom API keys

---

## Security Model

### Permission Enforcement

Every socket request is gated by permission checks:

```rust
if let Some(perm) = required_permission("contacts.list") {
    if !is_granted(perm) {
        return error("Permission denied");
    }
}
```

Agents cannot bypass macOS TCC.

### Credential Security

**At Rest:**
- AES-256-GCM encrypted files
- Master key in Keychain (never on disk)
- Metadata index doesn't contain secrets

**In Transit:**
- Tier 1: Credentials stay in Tairseach's memory
- Tier 2: Short-lived tokens (max 1 hour), audit logged

**Audit Trail:**
- All credential access logged to `~/.tairseach/logs/audit.jsonl`
- Logs sanitized (no token values)
- Rotated daily, kept 30 days

### Manifest Security

**Trust Levels:**

| Source | Trust | Sandboxing |
|--------|-------|------------|
| Core (ships with app) | Trusted | None |
| Integrations (ships with app) | Trusted | Light |
| Community (user-installed) | **Untrusted** | **Full sandbox** |

**Community Manifest Safeguards:**
- Installation requires user approval
- Scripts run in `sandbox-exec` (macOS)
- Network access opt-in only
- Path restrictions (no `../../` traversal)
- Hash verification (detect tampering)

---

## Known Issues & Limitations

### CRITICAL Security Findings (Must Fix Before v2)

From security audits by Nechtan and Fedelm:

1. ⚠️ **Master key exposed in CLI args** — `security` command shows key in `ps` output
   - **Fix:** Replace with `security-framework` crate
2. ⚠️ **OAuth secrets in CLI args** — `curl` command exposes secrets
   - **Fix:** Replace with `reqwest` or pass via stdin
3. ⚠️ **No UID verification on socket** — 0600 permissions only defense
   - **Fix:** Add explicit UID check on connection
4. ⚠️ **files.write has no path restrictions** — Can write anywhere
   - **Fix:** Implement path allowlist/denylist
5. ⚠️ **automation.run enables arbitrary code execution**
   - **Fix:** Add script allowlist or exclude from MCP exposure

### Incomplete Features

- Calendar `update_event` and `delete_event` are stubbed
- Photos handler not implemented
- MCP bridge is scaffolded but not functional
- Activity Feed, Cron Calendar, Memory Search UIs are placeholders

---

## Roadmap

### Phase 1: Security Hardening (Week 1)

- [ ] Fix CRITICAL findings (master key, OAuth, UID check, path restrictions)
- [ ] Implement credential tier enforcement
- [ ] Add script sandboxing for community manifests

### Phase 2: Manifest System (Week 1-2)

- [ ] Implement manifest loader with validation
- [ ] Implement capability router
- [ ] Build hot-reload with filesystem watcher
- [ ] Create core manifests (contacts, calendar, etc.)

### Phase 3: MCP Bridge (Week 2)

- [ ] Complete tairseach-mcp bridge binary
- [ ] Implement stdio transport
- [ ] Test with OpenClaw integration
- [ ] Write MCP setup quickref

### Phase 4: Google Workspace (Week 2-3)

- [ ] Gmail handler (proxy mode)
- [ ] Calendar API handler (proxy mode)
- [ ] Drive handler (proxy mode)
- [ ] OAuth flow in Tairseach UI

### Phase 5: UI Buildout (Week 3-4)

- [ ] Activity Feed (live audit log)
- [ ] Cron Calendar (scheduled jobs)
- [ ] Memory Search (cross-agent search)
- [ ] Manifest Manager (browse, install, update)
- [ ] Credential Manager (import, rotate, revoke)

### Phase 6: Community Ecosystem (Week 4+)

- [ ] Manifest registry (GitHub-based)
- [ ] Manifest submission workflow
- [ ] Community manifest validation
- [ ] Auto-update system

---

## Contributing

Tairseach is developed by the Naonúr household agents:

| Agent | Role | Model |
|-------|------|-------|
| **Suibhne** | Orchestrator, spec authority | Claude Opus 4.6 |
| **Muirgen** | Primary implementation (Rust) | Kimi K2.5 |
| **Gwrhyr** | MCP protocol, UI performance | GPT-5.3 |
| **Fedelm** | Security architecture | Gemini 3 Pro |
| **Nechtan** | Security audit | Claude Opus 4.6 |
| **Tlachtga** | Build system, CI | GPT-5.3 |
| **Lomna** | Verification, testing | Claude Sonnet 4.5 |
| **Senchán** | Documentation | Claude Sonnet 4.5 |

For detailed agent roles, see `~/naonur/dail/governance/households/`.

### Development Workflow

1. **Design phase** — Write ADRs, specs, threat models in `~/naonur/dail/dev/work-orders/WO-2026-0001-tairseach-dreacht/30-artifacts/`
2. **Implementation** — Agents work in parallel per household assignments
3. **Verification** — Lomna tests all implementations
4. **Documentation** — Senchán writes architecture docs and quickrefs
5. **Review** — Suibhne final review before merge

All work tracked in Work Order WO-2026-0001.

---

## Resources

- **Architecture Documentation:** `~/naonur/dail/projects/tairseach/architecture.md`
- **Quick References:** `~/naonur/dail/projects/tairseach/quickrefs/`
- **Design Artifacts:** `~/naonur/dail/dev/work-orders/WO-2026-0001-tairseach-dreacht/30-artifacts/`
- **Work Order:** `~/naonur/dail/dev/work-orders/WO-2026-0001-tairseach-dreacht/`
- **Codebase:** `~/environment/tairseach/`

---

*🪶 The threshold doesn't just let you through — it holds the keys, knows the way, and guards the passage.*

**Last Updated:** 2026-02-08 by Senchán Torpéist
