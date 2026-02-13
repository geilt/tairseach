# Vue Performance Audit (refactor/vue-performance)

## Scope Reviewed

- `src/composables/useActivityFeed.ts`
- `src/composables/useStateCache.ts`
- `src/composables/useStatusPoller.ts`
- `src/composables/useToast.ts`
- `src/composables/useWorkerPoller.ts`
- `src/stores/auth.ts`
- `src/stores/config.ts`
- `src/stores/monitor.ts`
- `src/stores/permissions.ts`
- `src/stores/profiles.ts`
- `src/workers/status-poller.worker.ts`
- `src/main.ts`

## Event Lifecycle / Cleanup Findings

### Fixed

- `useWorkerPoller`: moved to scope-based cleanup and explicit worker unregister/terminate.
- `useActivityFeed`: ensured polling, event listener, and pending `requestAnimationFrame` are cleaned on scope dispose.
- `useStatusPoller`: switched cleanup to `onScopeDispose`.
- `useToast`: timer handles are now tracked and cleared when toasts are removed or shifted out of the stack cap.

### Verified Safe in Scope

- Store modules in this task list do not create event listeners or long-lived interval handles.
- `useStateCache` is synchronous `localStorage` access only.

## requestAnimationFrame Batching

### Implemented

- `useWorkerPoller`: status payload writes are batched via RAF and coalesced (cancels prior frame if a new payload arrives).
- `useActivityFeed`: large feed replacement is applied in RAF to reduce main-thread visual churn.

## Unified Polling Worker

Implemented `src/workers/unified-poller.worker.ts` with:

- `register` / `unregister` API
- per-registration interval management
- callback dispatch by `callbackId`
- result/error messages that include `registrationId`
- invoke bridge to main thread for Tauri command execution

`useWorkerPoller` now registers on mount, unregisters on dispose, and triggers immediate/manual poll via worker messages.

## Virtual Scrolling Assessment

### Candidate Lists

1. **Activity feed** (`useActivityFeed` consumer views)
   - High likelihood to exceed 100 rows.
   - Current limit defaults to 2000 and replaces full array on refresh.

2. **Tool/namespace listings in MCP/monitor views**
   - Lower baseline, but can spike in larger installs.

### Recommendations

1. **Adopt list virtualization for activity feed first**
   - Preferred: `vue-virtual-scroller` or `@tanstack/vue-virtual`.
   - Trigger threshold: ~100 visible items.

2. **Window fetched events**
   - Keep backend query cap, but maintain a smaller actively rendered window (e.g., 200–500).

3. **Stable row keys + memoized derived fields**
   - Ensure row-level computation (formatting/status badges) is not recomputed unnecessarily.

4. **Namespace/tool lists**
   - Add virtualization only if production telemetry shows frequent 100+ row rendering.
