# Reuse Check - Manifest-Driven Integration UI

## Target

- **What I need:** Unified integration page with collapsible sections, status indicators, and manifest-driven generation
- **Codebase searched:** `src/components/`, `src/views/`, `src/api/`

## Search Results

### Existing Components

**Common Components (all reusable):**
- `SectionHeader.vue` — Page headers with title, icon, description ✅
- `StatusBadge.vue` — Status indicators with icons and colors ✅
- `LoadingState.vue` — Loading spinners ✅
- `ErrorBanner.vue` — Error display with retry ✅
- `EmptyState.vue` — Empty state messages ✅

**Composables:**
- `useWorkerPoller` — Proxy status, namespace status polling ✅

**API Methods:**
- `api.mcp.manifests()` — Load all manifests ✅
- `api.auth.credentialsList()` — List credentials ✅
- `api.permissions.all()` — List permissions ✅
- `api.mcp.testTool()` — Test tools ✅

### Existing Views

- `IntegrationsView.vue` — Current integration list (will replace)
- `MCPView.vue` — MCP-specific view (will merge into IntegrationsView)
- `PermissionsView.vue` — Reference for status icons and grid layout

### Collapsible Pattern

No existing collapsible component found. Will implement inline with Vue's built-in `<Transition>` for collapse animation.

## Decision

- [x] **Reuse** `SectionHeader`, `StatusBadge`, `LoadingState`, `ErrorBanner`, `EmptyState`
- [x] **Reuse** `useWorkerPoller` composable for status polling
- [x] **Reuse** existing API methods
- [x] **Create new** — Collapsible section pattern (simple `v-if` with transition)
- [x] **Replace** — `IntegrationsView.vue` with unified manifest-driven version
- [ ] **Remove** — MCPView route (already not in router)

---

*Gwrhyr Gwalstawt Ieithoedd — Reuse Check Complete*
