# Dealbhóir Critical Fix Report

## Scope Completed
Implemented Phase 1 critical audit fixes in `~/worktrees/tairseach/fix-critical`.

## Changes Delivered

### 1) Broken command call fixed (`auth_list_credentials`)
- Updated `src/api/tairseach.ts`:
  - Removed `auth.listCredentials()` (broken `auth_list_credentials` command).
  - Kept `auth.credentialsList()` (`auth_credentials_list`) as the canonical list method.
  - Added missing API wrapper `auth.credentialsRename()` for `auth_credentials_rename`.
- Updated call sites:
  - `src/views/IntegrationsView.vue` now uses `api.auth.credentialsList()`.

### 2) AuthView refactor: zero direct `invoke()` usage
- `src/views/AuthView.vue`:
  - Removed `invoke` import.
  - Replaced all direct backend calls with API layer calls through `api.*`:
    - Credential types/list/store/delete/rename/custom type create
    - Manifest loading
    - 1Password vault list/default vault set
    - Google OAuth config/status/save/test/start flow
  - Result: **no direct `invoke()` calls** in AuthView.

### 3) Google settings merged into Auth flow + API layer consistency
- AuthView already contains Google OAuth UI section; retained and fully API-layer backed.
- `src/views/GoogleSettingsView.vue` was also converted to API-layer calls (no direct invoke).
- No `/settings/google` route found in router at time of pass.
- No Google settings sidebar item found in current nav; no additional removal required.

### 4) Permissions polling updated (removed `setInterval`)
- `src/views/PermissionsView.vue`:
  - Removed `window.setInterval` polling.
  - Integrated `useWorkerPoller(12_000)` and wired refresh behavior via reactive watch.

### 5) MCP merged into Integrations, MCP route/nav removed
- `src/views/IntegrationsView.vue` now includes MCP-specific operational features:
  - Namespace health status per integration card.
  - Proxy/socket/namespace status summary cards.
  - Existing manifest-driven cards with credential/permission status.
  - Expandable tool lists.
  - Per-card tool test actions.
  - OpenClaw install wizard section.
- Updated subtitle context with: **“Bridges to the Otherworld”**.
- Removed MCP route from `src/router/index.ts`.
- Removed MCP sidebar item from `src/components/common/TabNav.vue`.

### 6) Global error boundaries added
- `src/main.ts`:
  - Added `app.config.errorHandler`.
  - Logs to console.
  - Emits toast notification via global toast helper.
  - Added backend beacon placeholder comment.
- `src/App.vue`:
  - Added `onErrorCaptured` secondary boundary.
  - Logs + toast on captured component errors.

### 7) View pattern documentation corrected
- `docs/ai/patterns/view-pattern.md`:
  - Replaced direct `invoke()` template with API-layer pattern (`import { api } from '@/api/tairseach'`).
  - Updated checklist with “never call invoke directly in views”.
  - Replaced polling example to composable-based pattern.

## Supporting Infra Adjustments
- `src/components/common/ToastContainer.vue` switched to shared `useToast()` source.
- `src/composables/useToast.ts` updated:
  - Safe scope handling (`getCurrentScope`).
  - Added exported `showToast` helper for global handlers (`main.ts`, `App.vue`).

## Verification

### Build
- `npm run build` ✅ successful.

### Tauri dev launch
- `npm run tauri dev` ✅ reached running state:
  - Vite started.
  - Rust app compiled and launched (`Running .../target/debug/tairseach`).
- Observed non-blocking pre-existing backend log noise unrelated to this patch (Google refresh token 400 from runtime data).

## Constraints / Notes
- Did not touch `~/environment/tairseach/`.
- Work performed only in `~/worktrees/tairseach/fix-critical`.

## Commit
Prepared for single commit:
`fix: critical audit fixes — API layer, error boundary, view merge`
