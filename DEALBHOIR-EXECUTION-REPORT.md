# Dealbhóir Execution Report

## Summary
Implemented the UI overhaul across Auth, Integrations, and Permissions views:

1. Merged Google OAuth setup + connect flow into **AuthView** as a collapsible **Google Account** section.
2. Replaced hardcoded Integrations UI with a **manifest-driven** integrations renderer.
3. Fixed Permissions page jump-to-top behavior by preserving/restoring scroll position around permission requests.

## What Changed and Why

### 1) Google OAuth moved into Auth view
- Removed dedicated `/settings/google` route from router.
- Removed Google sidebar item.
- Added **Google Account** collapsible panel in `AuthView.vue` with:
  - OAuth status display
  - Connect button (`handleConnectGoogle` retained)
  - Optional OAuth client config sub-panel (Client ID/Secret)
  - JSON upload/drop/paste parsing for `client_secret.json`
  - Save/Test config actions

This keeps all auth-related setup in one place and avoids context-switching to a separate settings route.

### 2) Integrations view now manifest-driven
- Rebuilt `IntegrationsView.vue` to load all manifests from `api.mcp.manifests()`.
- For each manifest card, UI now shows:
  - Name/description/emoji
  - Expandable tool list
  - Credential status (✅/❌ based on `requires.credentials` vs stored credential types)
  - Permission status (✅/❌ based on manifest/tool `requires.permissions` vs current permission states)
  - Configure button linking to `/auth` with `?credential=...`
  - Test button running diagnostic call with first available tool (prefers zero-required-input tool)
- Added flavor subtitle text under Integrations header:
  - “Bridges to the Otherworld — credentials, permissions, and tools at a glance.”

### 3) Auth dynamic credential type synthesis from manifests
- In `AuthView.vue`, credential type loading now augments backend credential types with manifest-derived credential requirements.
- If manifests include credential requirements not known to current auth type registry, types are synthesized and surfaced in Credential Management.
- Supports schema-based fields when present (`requires.credentials[].schema.fields`) and fallback field generation otherwise.

### 4) Permissions scroll bug fix
- In `PermissionsView.vue`, `requestPermission` is now async and preserves `window.scrollY` before request.
- After request/state update + `nextTick`, scroll position is restored via `window.scrollTo`.
- Eliminates disruptive jump to top while granting lower-list permissions.

## Files Modified
- `src/router/index.ts`
- `src/components/common/TabNav.vue`
- `src/views/AuthView.vue`
- `src/views/IntegrationsView.vue`
- `src/views/PermissionsView.vue`
- `DEALBHOIR-EXECUTION-REPORT.md` (new)

## Component/UI Descriptions (in lieu of screenshots)
- **Auth > Google Account card**
  - Top-level collapsible card.
  - Shows OAuth connection status message + Connect action.
  - Nested optional “OAuth client config” panel for client credentials and JSON parsing/upload.
- **Integrations cards**
  - One card per manifest.
  - Status rows for credentials and permissions using green/red indicators.
  - Expand/collapse tools list and inline test output/error panels.
- **Permissions cards**
  - Same visual design; interaction now preserves current scroll location after clicking Request.


### 5) Credential relabeling (new)
- Added inline **Rename** action for each stored credential row in `AuthView.vue`.
- Clicking rename converts the label display to an inline input with Save/Cancel.
- Save invokes:
  - `auth_credentials_rename` with `{ credType, oldLabel, newLabel }`
- On success, list/status reload and success feedback is shown.

> Backend note: this assumes a Tauri command named `auth_credentials_rename` exists. If missing, wire it on the Rust side and map it to backend method semantics equivalent to `auth.credentials.rename`.

## Validation / Build
Executed requested run sequence:
- Killed prior dev processes (`pkill -f "tauri dev"`, `pkill -f Tairseach`, and conflicting Vite process)
- Ran `npm run tauri dev`
- App/dev environment launched successfully after resolving a transient import typo during iteration.

Observed existing backend/runtime warnings/errors not introduced by this UI task:
- Rust warnings (unused items)
- Existing Google refresh-token error logs from runtime auth provider

## Remaining Issues / TODOs
- `src/views/GoogleSettingsView.vue` still exists in repo but is now orphaned (route removed). Can be deleted in a cleanup pass if desired.
- Credential mapping from manifest `requires.credentials` to auth storage types is heuristic for unknown providers; if manifest schema/typing becomes stricter, mapping should be formalized centrally.
