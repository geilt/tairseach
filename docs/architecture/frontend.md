# Frontend Architecture

**Stack:** Vue 3 + TypeScript + Vite + Tauri 2  
**State:** Pinia stores with localStorage caching  
**Styling:** TailwindCSS + custom Naonúr theme

---

## Application Structure

```
src/
├── App.vue              # Root component with layout + toast container
├── main.ts              # Entry point (Vue, Pinia, Router)
├── router/              # Vue Router config
├── views/               # Top-level route components
├── components/          # Reusable UI components
├── stores/              # Pinia state management
├── composables/         # Vue composition utilities
├── workers/             # Web Worker for background polling
└── assets/              # Static assets (icons, styles)
```

---

## Views (10 Routes)

All views are lazily loaded via Vue Router and cached with `<KeepAlive>` (max 5).

| Route | Component | Purpose |
|-------|-----------|---------|
| `/` | `DashboardView.vue` | System health overview (proxy, permissions, manifests, activity) |
| `/permissions` | `PermissionsView.vue` | macOS permission status and request UI |
| `/config` | `ConfigView.vue` | OpenClaw config editor (agents, providers, models) |
| `/settings/google` | `GoogleSettingsView.vue` | Google OAuth credential management |
| `/monitor` | `MonitorView.vue` | Live activity monitor (WebSocket events) |
| `/activity` | `ActivityView.vue` | Full event log with namespace filtering |
| `/profiles` | `ProfilesView.vue` | Agent profile manager (placeholder) |
| `/auth` | `AuthView.vue` | Auth broker account management (view/revoke tokens) |
| `/integrations` | `IntegrationsView.vue` | Integration hub (placeholder) |
| `/mcp` | `MCPView.vue` | MCP tool browser, test runner, OpenClaw installer |

### View Pattern

Each view:
- Calls `invoke()` from `@tauri-apps/api/core` to interact with Rust backend
- Uses stores for shared state (auth, config, permissions)
- Implements local caching via `useStateCache()` for instant hydration
- Lazy-loads on mount, refreshes on activation

---

## Component Library

### Common Components

**`TabNav.vue`**
- Left sidebar navigation
- Displays logo + brand
- 10 nav items with emoji icons
- Active state derived from current route

**`StatusBadge.vue`**
- Permission status indicator
- Color-coded: granted (green), denied (red), pending (yellow)

**`Toast.vue` + `ToastContainer.vue`**
- Notification system
- 4 variants: success, warning, error, info
- Auto-dismiss after 5s
- Max 5 toasts stacked

### Config Components (`components/config/`)

Specialized inputs for OpenClaw config editing:

- **`AgentCard.vue`** — Agent list item with model/workspace controls
- **`ProviderCard.vue`** — Provider config editor (base URL, API key)
- **`ConfigInput.vue`** — Text input with label
- **`ConfigSelect.vue`** — Dropdown with options
- **`ConfigToggle.vue`** — Boolean switch
- **`ConfigSection.vue`** — Collapsible section wrapper

---

## State Management (Pinia)

### Store Architecture

All stores follow this pattern:
1. **Cache-first hydration** — Load from localStorage on init
2. **Background refresh** — Silent API calls to stay current
3. **Optimistic updates** — UI updates immediately, persists to cache
4. **Dirty tracking** — Compare current state to original for save detection

### Stores

#### `config.ts` (OpenClaw Config)

**State:**
- `config: OpenClawConfig` — Full config object
- `originalConfig: string` — JSON snapshot for dirty detection
- `providerModels: Record<string, ModelOption[]>` — Available models per provider
- `environment: EnvironmentInfo` — Gateway vs Node detection
- `nodeConfig: NodeConfig | null` — Node-specific config (if applicable)
- `execApprovals: ExecApproval[]` — Exec command allowlist

**Actions:**
- `loadConfig()` — Fetch via `get_config` command
- `saveConfig()` — Write via `set_config` command
- `updateAgent(id, updates)` — Modify agent in list
- `getModelsForProvider(provider)` — Merge built-in + custom models
- `parseModelString(str)` / `buildModelString(provider, model)` — Parse `provider/model` format

**Computed:**
- `dirty` — Config changed since load
- `agents` — List of agent configs
- `isNode` / `isGateway` — Environment type

#### `auth.ts` (Auth Broker)

**State:**
- `status: AuthStatus` — Master key status, account count
- `accounts: AccountInfo[]` — Provider accounts with scope + expiry
- `providers: string[]` — Available auth providers

**Actions:**
- `loadStatus()` / `loadAccounts()` / `loadProviders()`
- `getToken(provider, account, scopes)` — Fetch access token
- `refreshToken(provider, account)` — Force token refresh
- `revokeToken(provider, account)` — Delete token
- `storeToken(record)` — Add new OAuth token

#### `permissions.ts` (macOS Permissions)

**State:**
- `permissions: Permission[]` — Current permission statuses
- `definitions: PermissionDefinition[]` — Static permission metadata (name, icon, pref pane)

**Actions:**
- `loadPermissions()` — Check all via `check_all_permissions`
- `checkPermission(id)` — Single permission check
- `refreshPermission(id)` — Re-check and update
- `requestPermission(id)` — Trigger system prompt + poll for status change (30s timeout)
- `openSettings(pane)` — Open System Settings to specific pane

**Computed:**
- `criticalPermissions` — Subset of critical perms
- `criticalGranted` — Count of granted critical perms

#### `monitor.ts` (Activity Events)

**State:**
- `events: ActivityEvent[]` — Recent activity (max 1000)
- `connected: boolean` — Monitor connection status
- `paused: boolean` — User pause state

**Actions:**
- `addEvent(event)` — Push new event to front
- `clearEvents()` — Empty the log
- `togglePause()` — Pause/resume logging

#### `profiles.ts` (Agent Profiles)

**State:**
- `profiles: Profile[]` — Agent/tool profiles (stub)
- `activeProfile: string | null` — Current active profile

**Actions:** Stubbed for future implementation

---

## Composables

### `useStateCache.ts`

Persistent localStorage cache for store state.

**API:**
```ts
loadStateCache<T>(name: string): CachedState<T> | null
saveStateCache<T>(name: string, data: T): CachedState<T>
clearStateCache(name: string): void
cacheKey(name: string): string
```

**Keys:** Prefixed with `tairseach_cache_`  
**Format:** `{ data: T, lastUpdated: string }`

Used by all stores for instant hydration on mount.

### `useStatusPoller.ts`

Main-thread status poller (legacy, being replaced by worker version).

**Features:**
- 15s poll interval
- Sequential checks: proxy status → socket alive → namespace statuses
- 5s timeout per `invoke()` call
- Mutex prevents overlapping polls

**API:**
```ts
const { proxyStatus, namespaceStatuses, socketAlive, isPolling, start, stop, refresh, pollOnce } = useStatusPoller()
```

### `useWorkerPoller.ts`

Web Worker bridge for non-blocking status polling.

**Architecture:**
1. Worker owns scheduling logic
2. Worker sends `invoke` messages to main thread
3. Main thread calls Tauri `invoke()` and returns result
4. Worker sends `status-update` messages back
5. Composable applies updates in `requestAnimationFrame`

**API:**
```ts
const { proxyStatus, socketAlive, namespaceStatuses, refresh } = useWorkerPoller(intervalMs = 15000)
```

### `useActivityFeed.ts`

Activity log with polling and event parsing.

**Features:**
- Polls `get_events` every 1.5s
- Parses backend events into `ActivityEntry` format
- Infers namespace from tool name
- Classifies operation type (read/write/destructive/error)
- Optional live push via Tauri event listener (`tairseach://activity`)

**API:**
```ts
const { loading, entries, namespaces, lastUpdated, refresh } = useActivityFeed(limit = 2000)
```

### `useToast.ts`

Singleton toast notification system.

**API:**
```ts
const { toasts, success, warning, error, info, remove } = useToast()

success("Saved!", 3000)
error("Failed to load config")
```

---

## Web Worker

### `status-poller.worker.ts`

Dedicated worker for MCP status polling.

**Responsibilities:**
- Schedule polls at 15s intervals
- Send `invoke` requests to main thread (bypassing Tauri worker restrictions)
- Consolidate results into single `status-update` message
- Handle timeouts (5s per call) and errors gracefully

**Message Protocol:**

**Incoming:**
```ts
{ type: 'start', intervalMs?: number }
{ type: 'stop' }
{ type: 'invoke-result', id: string, result?: any, error?: string }
```

**Outgoing:**
```ts
{ type: 'invoke', id: string, command: string, params: Record<string, any> }
{ type: 'status-update', data: { proxyStatus, socketAlive, namespaceStatuses } }
```

**Why a worker?**
- Prevents UI jank from blocking polls
- Isolates timing logic from render cycles
- Enables concurrent polling without mutex complexity

---

## API Layer

All backend communication uses Tauri's `invoke()` command from `@tauri-apps/api/core`.

### Key Commands

**Config:**
- `get_config` → `{ raw: OpenClawConfig, path: string }`
- `set_config(config)` → `void`
- `get_provider_models` → `Record<string, ModelOption[]>`
- `get_environment` → `EnvironmentInfo`
- `get_node_config` → `{ config: NodeConfig, path: string }`
- `set_node_config(config)` → `void`
- `get_exec_approvals` → `{ approvals: ExecApproval[], path: string }`
- `set_exec_approvals(approvals)` → `void`

**Permissions:**
- `check_all_permissions` → `Permission[]`
- `check_permission(permissionId)` → `Permission`
- `request_permission(permissionId)` → `void`
- `open_permission_settings(pane)` → `void`
- `get_permission_definitions` → `PermissionDefinition[]`

**MCP Proxy:**
- `get_proxy_status` → `{ running: boolean, socket_path?: string }`
- `check_socket_alive` → `{ alive: boolean }`
- `get_namespace_statuses` → `NamespaceStatus[]`
- `get_all_manifests` → `Manifest[]`
- `get_manifest_summary` → `ManifestSummary`

**Auth:**
- `auth_status` → `AuthStatus`
- `auth_providers` → `string[]`
- `auth_accounts(provider?)` → `AccountInfo[]`
- `auth_get_token(provider, account, scopes?)` → `TokenInfo`
- `auth_refresh_token(provider, account)` → `void`
- `auth_revoke_token(provider, account)` → `void`
- `auth_store_token(record)` → `void`

**Activity:**
- `get_events(limit)` → `ActivityEvent[]`

**Events:** (Tauri event system)
- `tairseach://activity` → Real-time activity push

---

## Styling

**Framework:** TailwindCSS 3.4  
**Theme:** Custom Naonúr palette in `tailwind.config.js`:

```css
/* Key colors */
--naonur-void: #0a0a0a        /* Deep background */
--naonur-shadow: #1a1a1a      /* Sidebar */
--naonur-mist: #2a2a2a        /* Hover states */
--naonur-fog: #3a3a3a         /* Borders */
--naonur-bone: #e8e8e8        /* Primary text */
--naonur-ash: #a8a8a8         /* Secondary text */
--naonur-smoke: #787878       /* Tertiary text */
--naonur-gold: #d4a574        /* Brand accent */
--naonur-moss: #7fb069        /* Success */
--naonur-rust: #c97064        /* Error */
--feather: #f5f5f5            /* Main content bg */
```

**Fonts:**
- Display: Inter (titles)
- Body: System font stack

**Transitions:**
- Page transitions: 35ms opacity fade (fast, minimal)
- Hover states: 140ms ease
- GPU-accelerated via `transform: translateZ(0)`

**Responsive:**
- Sidebar: Fixed 256px on desktop, collapsible on mobile (TODO)
- Grid layouts: 1-3 columns depending on breakpoint

---

## Performance Optimizations

1. **KeepAlive caching** — Cache up to 5 recently visited views
2. **Shallow refs** — Use `shallowRef` for large arrays/objects in stores
3. **localStorage persistence** — Instant hydration, no loading flicker
4. **Web Worker polling** — Non-blocking status checks
5. **RAF batching** — State updates in `requestAnimationFrame` to sync with render cycle
6. **Tween animations** — Smooth number transitions (220ms cubic ease-out)
7. **CSS containment** — `contain: layout style paint` on cards to reduce reflow

---

## Future Improvements

- [ ] TypeScript strict mode (currently permissive)
- [ ] Error boundary components
- [ ] Mobile-responsive sidebar
- [ ] Virtual scrolling for long activity logs
- [ ] Offline-first PWA mode
- [ ] Dark/light theme toggle
- [ ] Agent profile CRUD implementation
- [ ] MCP test runner history
- [ ] Search/filter across all views

---

**Source files analyzed:**
- All 10 views in `src/views/`
- All components in `src/components/`
- All 5 stores in `src/stores/`
- All 5 composables in `src/composables/`
- Web worker in `src/workers/`
- Router config in `src/router/index.ts`
- Main entry point in `src/main.ts`
- Root component in `src/App.vue`

**Last updated:** 2026-02-13
