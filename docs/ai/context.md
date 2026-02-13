# Tairseach AI Context

> **Purpose:** Master bootstrap file for AI agents working on Tairseach  
> **Last Updated:** 2026-02-13  
> **Branch:** `main`

---

## 1. Project Identity

**Name:** Tairseach (Irish: "Threshold")  
**Purpose:** macOS system bridge for the Naonúr ecosystem — provides secure capability delegation through MCP protocol  
**Version:** 0.2.0  
**Tech Stack:**
- **Backend:** Rust + Tauri 2.0 (macOS native desktop app)
- **Frontend:** Vue 3 + TypeScript + Pinia (state management)
- **IPC:** Unix domain socket (JSON-RPC 2.0)
- **Protocol:** MCP (Model Context Protocol) for AI agent integration
- **Auth:** AES-GCM encrypted credential store with OAuth 2.0 flow support

**License:** MIT  
**Authors:** Naonúr

---

## 2. Architecture Summary

Tairseach is a **capability router** that bridges AI agents to macOS system APIs through a three-layer architecture:

**Layer 1: Tauri Frontend** — Vue 3 web UI for configuration, monitoring, and permission management. Users configure OAuth credentials, grant system permissions, and monitor active connections. The UI communicates with Rust backend via Tauri IPC commands.

**Layer 2: Capability Router** — Core Rust routing logic that dispatches tool calls to the appropriate implementation layer. Routes are defined by **manifest files** (JSON schemas in `~/.tairseach/manifests/`) which declare tools, credentials, permissions, and implementation types (internal/proxy/script). The router validates requirements, retrieves credentials from the auth broker, and invokes the correct handler.

**Layer 3: Implementation Handlers** — Three execution modes:
- **Internal:** Direct Rust implementations (e.g., macOS permission checks via Objective-C FFI, Google Calendar/Gmail via REST APIs)
- **Proxy:** HTTP forwarding to external APIs with OAuth token injection
- **Script:** Execute external programs (Python, Go, etc.) and parse stdout

**Auth Flow:** OAuth tokens stored encrypted in `~/.tairseach/credentials.db` (AES-GCM). Master key derived from macOS Keychain item. Auto-refresh daemon maintains valid tokens. Handlers request tokens via `AuthBroker::get_token()` which handles refresh/retry transparently.

---

## 3. Module Registry

| Module | Directory | Files | Lines | Status | Key Types/Traits |
|--------|-----------|-------|-------|--------|------------------|
| **auth** | `src-tauri/src/auth/` | 7 | 2,907 | ✅ Stable | `AuthBroker`, `CredentialStore`, `ProviderConfig`, `TokenData` |
| **common** | `src-tauri/src/common/` | 6 | 346 | ✅ Stable | `AppError`, `ErrorCode`, `TairseachResult` |
| **config** | `src-tauri/src/config/` | 1 | 412 | ✅ Stable | Config structs for providers/models |
| **contacts** | `src-tauri/src/contacts/` | 1 | 239 | ✅ Stable | Contact CRUD via macOS APIs |
| **google** | `src-tauri/src/google/` | 5 | 767 | ✅ Stable | `GoogleOAuthClient`, `GmailApi`, `CalendarApi` |
| **manifest** | `src-tauri/src/manifest/` | 4 | 510 | ✅ Stable | `Manifest`, `ManifestRegistry`, `Tool`, `Implementation` |
| **mcp** | `src-tauri/src/mcp/` | 4 | 1,467 | ⚠️ WIP | MCP server implementation (standalone binary) |
| **monitor** | `src-tauri/src/monitor/` | 1 | 462 | ✅ Stable | Activity logging, manifest stats |
| **permissions** | `src-tauri/src/permissions/` | 12 | 1,085 | ✅ Stable | `Permission`, `PermissionStatus`, macOS TCC integration |
| **profiles** | `src-tauri/src/profiles/` | 1 | 26 | ⚠️ Stub | User profiles (placeholder) |
| **proxy** | `src-tauri/src/proxy/` | 3 | 623 | ✅ Stable | `ProxyServer`, `HandlerRegistry`, JSON-RPC protocol |
| **proxy/handlers** | `src-tauri/src/proxy/handlers/` | 17 | 6,088 | ✅ Stable | All capability handlers + shared handler utilities |
| **router** | `src-tauri/src/router/` | 5 | 736 | ✅ Stable | `CapabilityRouter`, routing dispatch logic |
| **frontend/stores** | `src/stores/` | 5 | 960 | ✅ Stable | Pinia stores: auth, config, permissions, profiles, monitor |
| **frontend/views** | `src/views/` | 10 | 3,713 | ✅ Stable | All UI views (Auth, Permissions, Integrations, etc.) |
| **frontend/composables** | `src/composables/` | 6 | 669 | ✅ Stable | Reactive utilities: toast, polling, activity feed, cache |
| **frontend/components** | `src/components/` | 15 | 1,003 | ✅ Stable | Reusable UI components (StatusBadge, TabNav, Toast, etc.) |
| **frontend/api** | `src/api/` | 2 | 320 | ✅ Stable | Typed API layer (`tairseach.ts`, `types.ts`) |
| **frontend/workers** | `src/workers/` | 2 | 57 | ✅ Stable | Unified and legacy polling workers |

**Total Rust:** ~16,669 lines  
**Total Vue/TS:** ~6,722 lines

---

## 4. Utility Registry

### Rust Shared Utilities

| Location | Utilities | Purpose |
|----------|-----------|---------|
| `common/error.rs` | `AppError`, `ErrorCode`, error constructors | Unified error handling with JSON-RPC codes |
| `common/http.rs` | `create_http_client()`, `create_http_client_with_timeout()` | Standard reqwest client creation |
| `common/paths.rs` | `tairseach_dir()`, `credentials_path()`, `manifests_dir()` | Path resolution for app data |
| `common/result.rs` | `TairseachResult<T>` type alias | Standard Result type for the app |
| `common/interpolation.rs` | `interpolate()` | String template substitution for proxy bindings |
| `proxy/handlers/common.rs` | 30+ parameter extraction helpers | `require_string()`, `optional_u64()`, `extract_oauth_credentials()`, etc. |
| `permissions/mod.rs` | `check_permission()`, `request_permission_with_callback()` | Shared TCC permission logic |
| `google/client.rs` | `GoogleOAuthClient::new()`, token exchange | Shared OAuth client for all Google APIs |

### TypeScript Shared Utilities

| Location | Utilities | Purpose |
|----------|-----------|---------|
| `src/api/tairseach.ts` | Typed command wrappers (`authApi`, `permissionsApi`, etc.) | Type-safe Tauri IPC calls grouped by domain |
| `src/api/types.ts` | Shared TypeScript request/response interfaces | Canonical frontend/backend contract types |
| `src/composables/useToast.ts` | `useToast()`, `showToast()` | Global toast notification system |
| `src/composables/useWorkerPoller.ts` | `useWorkerPoller()` | Unified Web Worker-based polling wrapper |
| `src/composables/useStatusPoller.ts` | `useStatusPoller()` | Proxy status polling with retry |
| `src/composables/useActivityFeed.ts` | `useActivityFeed()` | Real-time activity event aggregation |
| `src/composables/useStateCache.ts` | `useStateCache()` | Local storage caching for views |
| `src/workers/unified-poller.worker.ts` | Unified polling worker runtime | Consolidated background polling loop |

**Pattern:** Use these utilities — DO NOT recreate them. If a helper doesn't exist, add it to `src-tauri/src/common/`, `src-tauri/src/proxy/handlers/common.rs`, `src/api/`, or a composable as appropriate.

---

## 5. Branch State

All optimization and refactor branches have been merged into `main`.

| Branch | Status | Purpose |
|--------|--------|---------|
| `main` | ✅ Active | Production baseline + merged optimization work |
| `docs/ai-context` | ✅ Merged | AI context documentation updates |
| `refactor/handler-dry` | ✅ Merged | DRY handler utilities |
| `refactor/rust-core-dry` | ✅ Merged | Core utility extraction |
| `refactor/permissions-dry` | ✅ Merged | Permission helper extraction |
| `refactor/google-dry` | ✅ Merged | Google client consolidation |
| `refactor/vue-dry` | ✅ Merged | Vue component/composable DRY pass |
| `refactor/vue-performance` | ✅ Merged | Frontend polling/rendering optimization |
| `cleanup/dead-code-removal` | ✅ Merged | Dead code cleanup |

**Safe to build on:** `main`

### Recent Changes (2026-02-13)

- Optimization sprint complete; Phase B/C now in progress
- `src-tauri/src/proxy/handlers/common.rs` is now the canonical DRY utility layer for handlers
- Frontend polling is unified around `useWorkerPoller` + `src/workers/unified-poller.worker.ts`
- Typed frontend API layer added under `src/api/` (`tairseach.ts`, `types.ts`)
- AI context docs updated to reflect merged branch state and current codebase size

---

## 6. Patterns to Follow

### DRY (Don't Repeat Yourself) — PRIMARY DIRECTIVE

1. **Handler Pattern:** All handlers use `proxy/handlers/common.rs` utilities
   - Extract params with `require_string()`, `optional_u64()`, etc.
   - Get auth with `get_auth_broker()` → `extract_oauth_credentials()`
   - Return with `ok()`, `error()`, `invalid_params()`
   - **See:** [patterns/handler-pattern.md](patterns/handler-pattern.md)

2. **Path Resolution:** Always use `common/paths.rs`
   - **Never** hardcode `~/.tairseach` or `dirs::home_dir().join(".tairseach")`
   - Use `tairseach_dir()`, `manifests_dir()`, `credentials_path()`

3. **HTTP Clients:** Use `common/http.rs`
   - **Never** build `reqwest::Client` inline
   - Use `create_http_client()` or `create_http_client_with_timeout()`

4. **Error Handling:** Use `common/error.rs`
   - Return `TairseachResult<T>` (alias for `Result<T, AppError>`)
   - Use `AppError::token_not_found()`, `AppError::permission_denied()`, etc.
   - Consistent error codes across the app

5. **Vue Views:** Follow shared state pattern
   - Use `useToast()` for notifications
   - Use `useWorkerPoller()` for background polling (NOT `setInterval` in components)
   - Import types from `@/types`
   - **See:** [patterns/view-pattern.md](patterns/view-pattern.md)

**Reference:** [docs/optimization-reference.md](../optimization-reference.md) for comprehensive patterns

---

## 7. What NOT to Do (Anti-Patterns)

🚫 **Dead MCP Stubs in Handlers**
- Old handlers had unused `mcp_bridge` imports — these are removed
- If a handler doesn't use MCP bridge, don't import it

🚫 **`confirm()` Dialogs in Tauri Commands**
- Blocking dialogs freeze the event loop
- Use async message channels or return confirmation requests to frontend

🚫 **Serde Field Name Mismatches**
- JSON field names must match between Rust structs and Vue API calls
- Use `#[serde(rename = "camelCase")]` for consistency
- Common mismatch: `labelIds` vs `label_ids`

🚫 **Inline `reqwest::Client::builder()` Calls**
- Always use `common/http.rs` utilities

🚫 **Hardcoded Paths**
- Always use `common/paths.rs` utilities

🚫 **Recreating Parameter Extraction**
- Use `proxy/handlers/common.rs` helpers

🚫 **`setInterval()` in Vue Components**
- Use `useWorkerPoller()` or `useStatusPoller()` composables
- Ensures cleanup on unmount and prevents main thread blocking

🚫 **Manual Token Refresh**
- `AuthBroker::get_token()` handles refresh automatically
- Never call refresh endpoints directly

---

## 8. Quick Start for Agents

**Working on a handler?** → Read [modules/handlers.md](modules/handlers.md) + [patterns/handler-pattern.md](patterns/handler-pattern.md)

**Working on auth/credentials?** → Read [modules/auth.md](modules/auth.md)

**Adding a manifest?** → Read [modules/manifests.md](modules/manifests.md) + [patterns/manifest-pattern.md](patterns/manifest-pattern.md)

**Working on permissions?** → Read [modules/permissions.md](modules/permissions.md)

**Working on Vue views?** → Read [modules/frontend-views.md](modules/frontend-views.md) + [patterns/view-pattern.md](patterns/view-pattern.md)

**Working on Google integrations?** → Read [modules/google.md](modules/google.md)

**Working on 1Password?** → Read [modules/onepassword.md](modules/onepassword.md)

**Router architecture?** → Read [modules/router.md](modules/router.md)

**Frontend state management?** → Read [modules/frontend-infra.md](modules/frontend-infra.md)

**General architecture?** → Read [status.md](status.md) for current state

---

## 9. Module Documentation

All modules have detailed docs in `modules/`:

- [auth.md](modules/auth.md) — Authentication & credential management
- [handlers.md](modules/handlers.md) — All 15 capability handlers
- [permissions.md](modules/permissions.md) — macOS permission system
- [router.md](modules/router.md) — Capability routing logic
- [manifests.md](modules/manifests.md) — Manifest system
- [google.md](modules/google.md) — Google API integration
- [mcp-bridge.md](modules/mcp-bridge.md) — MCP server (standalone)
- [frontend-views.md](modules/frontend-views.md) — All Vue views
- [frontend-infra.md](modules/frontend-infra.md) — Vue infrastructure (stores, composables, workers)
- [onepassword.md](modules/onepassword.md) — 1Password integration

---

## 10. Pattern Templates

Copy-paste ready templates in `patterns/`:

- [handler-pattern.md](patterns/handler-pattern.md) — Template for new handlers
- [view-pattern.md](patterns/view-pattern.md) — Template for new Vue views
- [manifest-pattern.md](patterns/manifest-pattern.md) — Template for new manifests
- [utility-pattern.md](patterns/utility-pattern.md) — How to add shared utilities

---

## 11. Development Workflow

1. **Create a branch** from `main` with prefix: `feat/`, `fix/`, `refactor/`, `docs/`
2. **Read relevant module docs** from `modules/` before coding
3. **Follow patterns** from `patterns/` — don't reinvent
4. **Use shared utilities** — check `common/`, `handlers/common.rs`, composables
5. **Test locally** — `npm run tauri dev` for UI, `cargo test` for Rust
6. **Commit with conventional commits** — `feat:`, `fix:`, `refactor:`, `docs:`
7. **PR to main** when ready

**Testing MCP tools:**
```bash
# Start Tairseach UI
npm run tauri dev

# In another terminal
cd crates/tairseach-mcp
cargo run -- --socket ~/.tairseach/socket
```

---

## 12. File Structure Reference

```
tairseach/
├── src-tauri/src/           # Rust backend
│   ├── auth/                # Auth broker + credential store
│   ├── common/              # Shared utilities (error, http, paths, result, interpolation)
│   ├── config/              # App configuration
│   ├── contacts/            # macOS Contacts API
│   ├── google/              # Google OAuth + Gmail + Calendar
│   ├── manifest/            # Manifest loader + registry
│   ├── mcp/                 # MCP server (Tauri integration)
│   ├── monitor/             # Activity logging + stats
│   ├── permissions/         # macOS TCC permission checks
│   ├── profiles/            # User profiles (stub)
│   ├── proxy/               # Unix socket server + JSON-RPC protocol
│   │   └── handlers/        # All capability handlers + shared `common.rs` utilities
│   ├── router/              # Capability routing dispatcher
│   ├── lib.rs               # Tauri app entry point
│   └── main.rs              # Tauri binary entry point
├── src/                     # Vue 3 frontend
│   ├── api/                 # Typed API layer (`tairseach.ts`, `types.ts`)
│   ├── views/               # 10 main views
│   ├── components/          # Reusable components (common, config)
│   ├── stores/              # Pinia stores (auth, config, permissions, monitor, profiles)
│   ├── composables/         # Vue composables (toast, polling, activity, cache)
│   ├── workers/             # Web Workers (unified polling + legacy status poller)
│   ├── router/              # Vue Router
│   └── main.ts              # Vue app entry point
├── crates/
│   ├── tairseach-protocol/  # Shared protocol types
│   └── tairseach-mcp/       # Standalone MCP server binary
├── docs/                    # Documentation
│   ├── ai/                  # AI context (this directory)
│   ├── architecture/        # Architecture diagrams
│   ├── *.md                 # Various analysis docs
│   └── optimization-reference.md  # Comprehensive optimization guide
└── ~/.tairseach/            # Runtime data (created at first launch)
    ├── manifests/           # Capability manifests (JSON)
    └── credentials.db       # Encrypted credential store
```

---

*Context complete. Begin your work by reading the relevant module docs and pattern templates.*
