# Frontend Architecture

Tairseach is a Vue 3 + TypeScript + Tauri application providing a desktop GUI for managing OpenClaw gateway and node configuration, permissions, MCP tools, and activity monitoring.

---

## Tech Stack

- **Vue 3** — Composition API, SFC `<script setup>` pattern
- **TypeScript** — Full type safety across views, stores, and composables
- **Pinia** — State management
- **Vue Router** — Client-side routing with KeepAlive caching
- **Tauri 2** — Rust backend bridge via `@tauri-apps/api/core` `invoke()`
- **Vite** — Build tooling and dev server
- **TailwindCSS** — Utility-first styling with custom "naonur" color palette

---

## Project Structure

```
src/
├── App.vue                 # Root component: TabNav sidebar + RouterView
├── main.ts                 # Vue app bootstrap
├── router/
│   └── index.ts            # Route definitions
├── views/                  # Page-level components (routed)
│   ├── DashboardView.vue
│   ├── ActivityView.vue
│   ├── MonitorView.vue
│   ├── MCPView.vue
│   ├── ConfigView.vue
│   ├── PermissionsView.vue
│   ├── IntegrationsView.vue
│   ├── AuthView.vue
│   ├── ProfilesView.vue
│   └── GoogleSettingsView.vue
├── components/             # Reusable UI components
│   ├── common/
│   │   ├── StatusBadge.vue
│   │   ├── TabNav.vue
│   │   ├── Toast.vue
│   │   └── ToastContainer.vue
│   └── config/
│       ├── ConfigSection.vue
│       ├── ConfigInput.vue
│       ├── ConfigSelect.vue
│       ├── ConfigToggle.vue
│       ├── AgentCard.vue
│       └── ProviderCard.vue
├── stores/                 # Pinia state management
│   ├── config.ts           # OpenClaw gateway/node config
│   ├── permissions.ts      # macOS permission statuses
│   ├── monitor.ts          # Token usage and session monitoring (stub)
│   ├── profiles.ts         # Agent profile management (stub)
│   └── auth.ts             # Credential/auth state (stub)
├── composables/            # Reusable reactive logic
│   ├── useWorkerPoller.ts  # Web Worker-based status polling
│   ├── useActivityFeed.ts  # Real-time activity log feed
│   ├── useStateCache.ts    # localStorage persistence
│   └── useToast.ts         # Toast notification helper
└── workers/                # Web Workers
    ├── status-poller.worker.ts  # Deprecated (shim)
    └── unified-poller.worker.ts # Proxy + namespace status polling
```

---

## Views

Each view is a top-level routed page. All views follow a standard pattern:

- Use `onMounted`/`onActivated` to load/refresh data
- Use Pinia stores or composables for state
- Use `requestAnimationFrame` for non-blocking UI updates
- Cache state with `useStateCache` for instant perceived load times

### DashboardView.vue
**Route:** `/`  
**Purpose:** System health overview  
**Features:**
- Proxy status (connected/disconnected)
- MCP bridge status (tools exposed)
- Permissions summary (granted count)
- Manifest summary (capabilities/tools/MCP exposed count)
- Recent activity feed (last 10 events)
- Quick action buttons → `/permissions`, `/activity`

Caches state in localStorage (`dashboard` key) for instant perceived load. Uses `requestAnimationFrame` for smooth number tweening animations.

### ActivityView.vue
**Route:** `/activity`  
**Purpose:** Real-time MCP tool invocation log  
**Features:**
- Virtual scrolling (46px rows, overscan 8)
- Namespace filtering dropdown
- Auto-scroll to bottom on new events (unless user is hovering)
- Color-coded operation types (read/write/destructive/error)
- Columns: Time, Client, Namespace, Tool, Status

Uses `useActivityFeed` composable (polls proxy.log every 1.5s, parses structured events). Parses backend events and infers operation type from tool name patterns.

### MonitorView.vue
**Route:** `/monitor`  
**Purpose:** (Coming soon) Real-time token usage tracking  
**Current State:** Placeholder with icon and "Coming Soon" message

### MCPView.vue
**Route:** `/mcp`  
**Purpose:** MCP tool manifest browser and test interface  
**Features:**
- Proxy status card (running/stopped)
- Socket status card (alive/dead)
- Tool count summary
- Tool browser grouped by category (core/integrations)
- Connection status indicator per namespace (green/red dot)
- Expandable tool schemas (input/output)
- Tool tester modal (execute any tool with JSON params)
- OpenClaw integration installer (writes MCP server config to `~/.openclaw/config.toml`)

Uses `useWorkerPoller` for live proxy + namespace connection status. Invokes `get_all_manifests` to load tools, `test_mcp_tool` to execute test calls.

### ConfigView.vue
**Route:** `/config`  
**Purpose:** Edit OpenClaw gateway or node configuration  
**Features:**
- Detects environment type (gateway vs. node) via `get_environment` command
- **Gateway mode:**
  - Agent list (add/remove agents, set workspace/model)
  - Custom providers (Ollama, local APIs)
  - Gateway settings (port, bind, auth mode)
  - Channels (Telegram, Slack) — read-only preview
  - Tools (web search, agent-to-agent) — read-only preview
  - Raw JSON view
- **Node mode:**
  - Node identity (node ID, display name)
  - Gateway connection (host, port, TLS)
  - Exec approvals (pre-approved command patterns)
- Save/revert dirty tracking
- Persists to `~/.openclaw/config.toml` (gateway) or `~/.openclaw/node.toml` (node)

Uses `useConfigStore` (Pinia). Caches config in localStorage for instant load. Supports deep path updates via `updateConfigValue()`.

### PermissionsView.vue
**Route:** `/permissions`  
**Purpose:** Manage macOS TCC permissions  
**Features:**
- Grid of permission cards (icon, name, description, status badge)
- Critical permission badge (ring highlight)
- Request buttons open System Preferences to correct pane
- Auto-refresh every 12s (silent)
- Flash animation on status change
- Summary: "X of Y granted"

Uses `usePermissionsStore`. Backend commands: `check_all_permissions`, `request_permission`, `open_settings`. Statuses: granted, denied, not_determined, restricted.

### IntegrationsView.vue
**Route:** `/integrations`  
**Purpose:** Connected services and tool availability  
**Features:**
- Summary stats (total integrations, connected count, total tools)
- Connected integrations (expandable tool lists, test button)
- Available integrations (connect button → `/auth`)
- Tool tester modal (same as MCP view, reusable pattern)

Integrations are hard-coded configs mapped to credential types (`google_oauth`, `1password`, `oura`, `jira`). Loads tools from MCP manifests, groups by namespace prefix.

### AuthView.vue
**Route:** `/auth`  
**Purpose:** Credential management (not documented yet)

### ProfilesView.vue
**Route:** `/profiles`  
**Purpose:** Agent profile management (not documented yet)

### GoogleSettingsView.vue
**Route:** `/settings/google`  
**Purpose:** Google OAuth configuration (not documented yet)

---

## Shared Components

### Common

**StatusBadge.vue**  
Colored badge for permission/status display. Props: `status` (granted|denied), `size` (sm|md|lg).

**TabNav.vue**  
Sidebar navigation. Uses `vue-router` `<RouterLink>` with active state styling.

**Toast.vue**  
Single toast notification. Props: `message`, `type`, `duration`. Auto-dismiss with animation.

**ToastContainer.vue**  
Toast stack container. Manages multiple toasts, stacks vertically.

### Config

**ConfigSection.vue**  
Collapsible section for config forms. Props: `title`, `icon`, `description`, `badge`, `defaultOpen`.

**ConfigInput.vue**  
Styled input field. Props: `modelValue`, `label`, `description`, `monospace`, `disabled`, `type`.

**ConfigSelect.vue**  
Styled dropdown. Props: `modelValue`, `options`, `label`, `description`.

**ConfigToggle.vue**  
Checkbox toggle. Props: `modelValue`, `label`, `description`.

**AgentCard.vue**  
Card for agent config row. Props: `agent`. Emits: `remove`.

**ProviderCard.vue**  
Card for custom provider config row. Props: `name`, `config`. Emits: `remove`.

---

## State Management (Pinia Stores)

### config.ts
**Purpose:** OpenClaw gateway/node configuration  
**State:**
- `config` — gateway config object (`OpenClawConfig`)
- `nodeConfig` — node config object (`NodeConfig`)
- `execApprovals` — exec approval list (node only)
- `environment` — detected environment type (gateway|node)
- `providerModels` — available models per provider
- `dirty` — has unsaved changes
- `loading`, `saving`, `error`

**Actions:**
- `loadConfig()` — fetch config from backend
- `saveConfig()` — persist to TOML
- `loadNodeConfig()` — fetch node config
- `loadExecApprovals()` — fetch exec approvals
- `loadEnvironment()` — detect gateway vs. node
- `updateConfigValue(path, value)` — deep path update
- `addAgent(agent)`, `removeAgent(id)`, `updateAgent(id, updates)`

**Persistence:** `useStateCache('config')` — caches entire config tree for instant load.

### permissions.ts
**Purpose:** macOS permission status tracking  
**State:**
- `permissions` — array of `{ id, name, description, status, critical }`
- `definitions` — icon/metadata map
- `loading`, `error`

**Actions:**
- `loadPermissions()` — poll `check_all_permissions`
- `requestPermission(id)` — trigger permission prompt
- `openSettings(pane)` — open System Preferences

**Persistence:** `useStateCache('permissions')`.

### monitor.ts, profiles.ts, auth.ts
**Status:** Stub stores, minimal or placeholder functionality.

---

## Composables

### useWorkerPoller.ts
**Purpose:** Worker-based proxy + namespace status polling  
**Returns:** `{ proxyStatus, socketAlive, namespaceStatuses, refresh }`  
**Implementation:**
- Spawns `unified-poller.worker.ts` Web Worker
- Worker runs interval timer, posts `{ type: 'invoke', command, params }` messages
- Composable bridges `invoke()` calls to Tauri (main thread only)
- Worker posts `{ type: 'status-update', data }` with results
- Composable updates reactive refs via `requestAnimationFrame`

### useActivityFeed.ts
**Purpose:** Real-time activity log parsing  
**Returns:** `{ loading, entries, namespaces, lastUpdated, refresh }`  
**Implementation:**
- Polls `get_events({ limit })` every 1.5s
- Parses `BackendEvent[]` into typed `ActivityEntry[]`
- Infers namespace from tool name prefix (e.g., `gcalendar.list` → `gcalendar`)
- Infers operation type from tool name patterns (delete/create/etc.)
- Optionally listens to `tairseach://activity` event bus for live push

### useStateCache.ts
**Purpose:** Persist state to `localStorage`  
**Functions:**
- `loadStateCache<T>(name)` — load cached state
- `saveStateCache<T>(name, data)` — save with timestamp
- `clearStateCache(name)` — remove entry

**Schema:** `{ data: T, lastUpdated: string }`  
**Key prefix:** `tairseach_cache_`

### useToast.ts
**Purpose:** Show toast notifications (not documented in detail yet)

---

## Workers

### unified-poller.worker.ts
**Purpose:** Background polling for proxy + namespace status  
**Pattern:**
- Web Worker runs outside main thread
- Cannot call `invoke()` directly (Tauri API main-thread only)
- Posts `{ type: 'invoke', command, params, id }` to main thread
- Main thread calls `invoke()`, posts result back
- Worker updates internal state, posts `{ type: 'status-update', data }` to main thread
- Main thread uses `requestAnimationFrame` to update Vue refs (no tearing)

**Interval:** Configurable (default 15s)

---

## Router

**File:** `src/router/index.ts`  
**History mode:** `createWebHistory()` (HTML5 pushState)  
**Routes:**
- `/` → DashboardView
- `/permissions` → PermissionsView
- `/config` → ConfigView
- `/settings/google` → GoogleSettingsView
- `/monitor` → MonitorView
- `/activity` → ActivityView
- `/profiles` → ProfilesView
- `/auth` → AuthView
- `/mcp` → MCPView
- `/integrations` → IntegrationsView

**KeepAlive:** `App.vue` wraps RouterView with `<KeepAlive :max="5">` — caches up to 5 route components for instant back navigation.

**Title management:** `router.afterEach()` updates `document.title` based on `route.meta.title`.

---

## Build & Dev

**Dev server:** `npm run dev` → Vite dev server at `http://localhost:1420`  
**Type checking:** `vue-tsc --noEmit`  
**Build:** `npm run build` → compiles to `dist/`  
**Tauri dev:** `cargo tauri dev` or `npm run tauri dev`  
**Tauri build:** `cargo tauri build` or `npm run app:build`

---

## Styling

**Framework:** TailwindCSS with custom config  
**Custom classes:**
- `.naonur-card` — standard card container
- `.btn`, `.btn-primary`, `.btn-secondary`, `.btn-ghost` — button styles
- `.threshold-line` — decorative separator

**Color palette:**
- `naonur-void` — dark background
- `naonur-bone` — primary text
- `naonur-ash`, `naonur-smoke`, `naonur-fog` — muted grays
- `naonur-gold` — accent/headings
- `naonur-moss` — success green
- `naonur-blood`, `naonur-rust` — error/warning reds

**Fonts:**
- `.font-display` — headings
- `.font-body` — body text
- `.font-mono` — code/paths

**Transitions:**
- Page transitions: `.page-enter-active`, `.page-leave-active` (35ms linear opacity)
- Tween animations: `tween(from, to, setter, duration)` helper in DashboardView

---

## Performance Optimizations

1. **State caching:** All stores cache state in localStorage for perceived instant load
2. **Silent background refresh:** `loadX({ silent: true })` refreshes data without loading spinners
3. **KeepAlive route caching:** Previous views stay in memory (max 5)
4. **Virtual scrolling:** ActivityView renders only visible rows (46px + overscan)
5. **Web Workers:** Status polling off main thread, bridged via `invoke()`
6. **requestAnimationFrame batching:** All state updates from workers go through rAF to avoid tearing
7. **Shallow refs:** Stores use `shallowRef` for large objects to avoid deep reactivity overhead

---

## Type Safety

All backend commands are typed via TypeScript interfaces:

```typescript
invoke<ProxyStatus>('get_proxy_status')
invoke<Manifest[]>('get_all_manifests')
invoke<{ raw: OpenClawConfig, path: string }>('get_config')
```

Stores define explicit types for all state. No `any` types in production code.

---

## State Flow Example

**Dashboard load sequence:**

1. User navigates to `/`
2. `DashboardView` `onMounted()` → `hydrateCache()` (instant perceived load from localStorage)
3. `loadDashboard(silent: true)` → parallel `invoke()` calls for live data
4. Backend returns updated state
5. `persistCache()` → saves to localStorage
6. Vue reactive refs update → DOM re-renders

---

## Backend Communication

All backend calls use `invoke()` from `@tauri-apps/api/core`:

```typescript
import { invoke } from '@tauri-apps/api/core'

const result = await invoke<ReturnType>('command_name', { param: value })
```

Commands are defined in Rust backend (`src-tauri/src/commands/`). See backend docs for full command reference.

---

## Testing

No formal test suite yet. Manual testing workflow:

1. `cargo tauri dev` — run in dev mode
2. Verify each view loads
3. Test permission requests
4. Test config save/revert
5. Test tool execution in MCP view
6. Monitor console for errors

---

## Future Enhancements

- [ ] Full AuthView implementation (credential CRUD)
- [ ] ProfilesView agent profile editor
- [ ] MonitorView live token usage tracking
- [ ] Real-time event bus integration (`tairseach://` event protocol)
- [ ] Dark/light theme toggle
- [ ] Keyboard shortcuts
- [ ] Unified search across tools/config/activity
- [ ] Export activity log to CSV
- [ ] MCP manifest editor (create/edit tool definitions)
