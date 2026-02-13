# Vue Performance Assessment (Unified Poller + Lifecycle Audit)

## Scope
- `src/composables/*.ts`
- `src/stores/*.ts`
- Relevant `<script setup>` sections with timer/watch lifecycle behavior

## 1) Event Lifecycle Audit

### Verified cleanups (existing)
- `useStatusPoller.ts`: interval + timeout paths are cleared via `stop()` + `onUnmounted()`.
- `useActivityFeed.ts`: polling interval and Tauri event listener are cleaned in `onBeforeUnmount()`.
- `stores/*`: no persistent interval/event listener leaks found.

### Leaks fixed
- `useToast.ts`
  - Added timer registry (`toastTimers`) and cleanup via `onScopeDispose()`.
  - Clears timers when toast is removed/shifted.
- `useWorkerPoller.ts`
  - Added RAF cancellation (`rafId`) on unmount to avoid post-unmount frame updates.
- `ActivityView.vue`
  - Added `onUnmounted` cleanup for filter debounce timeout.
- `PermissionsView.vue`
  - Added tracked timeout cleanup for status-flash animation timers.
- `ConfigView.vue`
  - Added tracked timeout cleanup for transient save message timer.
- `GoogleSettingsView.vue`
  - Added tracked timeout cleanup for feedback timer.
- `AuthView.vue`
  - Consolidated action-message timeout scheduling and cleanup on unmount.

## 2) RAF Assessment (forced reflow risk)

### Observations
- `ActivityView.vue` reads scroll metrics (`scrollHeight`, `scrollTop`, `clientHeight`) and writes `scrollTop`.
- Write path already uses `requestAnimationFrame`, which is correct.

### Where RAF helps (documented guidance)
- If more scroll-linked writes are added in `ActivityView.vue`, keep reads grouped before writes and perform writes in RAF.
- `DashboardView.vue` tween animation already uses RAF; if this pattern expands, consider cancellation tokens per tween to avoid stale frame work during rapid updates.

## 3) Virtual Scroll Assessment

### Already virtualized
- `ActivityView.vue` uses a windowed rendering strategy (`startIndex/endIndex`, spacers). Good for large activity logs.

### Candidates for future virtualization
- `ConfigView.vue`
  - Agent/provider lists may become large in heavily customized setups.
  - Recommendation: add threshold-based virtualization when list size exceeds ~100 items.
- `AuthView.vue`
  - Credential type + credential entries may grow in enterprise usage.
  - Recommendation: virtualize credential rows if lists exceed ~150 rows.

### Non-candidates (currently small, bounded)
- `PermissionsView.vue` (bounded macOS permission set)
- `DashboardView.vue` recent activity (hard-limited request size)
