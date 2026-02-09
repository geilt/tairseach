# Tairseach — WO-2026-0001 Progress

## [02:47] Project Commenced
- Suibhne orchestrating, delegating to Naonúr
- Phase 1: Stabilize & Verify — dispatching Lomna for socket handler testing
- Build confirmed clean ✅ (app bundles successfully)

## [02:50] Phase 1: Socket Fix Dispatched
- **Muirgen** spawned → diagnose & fix proxy server not creating socket
- Socket file exists from old run but server not binding on fresh launch
- Label: `tairseach-socket-fix`

## [02:55] Phase 1: Socket Server Verified ✅
- Muirgen confirmed socket works — server.status, permissions.list respond correctly
- Issue was stale build artifact, not code bug
- Added tracing for proxy lifecycle
- App rebuilt, signed, socket at `~/.tairseach/tairseach.sock` (0600)

## [03:30] Location Handler Rewrite Complete ✅
- Muirgen rewrote location.get using native objc2 CoreLocation (not Swift subprocess)
- Root cause: macOS TCC grants permissions per-binary, not per-app-bundle
- Swift subprocess ran as /usr/bin/swift which has its own TCC identity
- Now uses define_class! macro for CLLocationManagerDelegate directly in Rust
- Returns real coordinates: 30.54°N, 97.60°W (Round Rock, TX)
- All Phase 1-3 tasks complete. Phase 4 (MCP) designed, ready to implement.

## [03:27] Inspíoráid Canonized
- New term added to GNAS.md for Dúil → Dalta animation
- Daltaí are focused personas, not separate agents
- The Triad of Breath: Geilt-Suibhne → Dúile → Daltaí

## [03:30] Journey Day 12 Published
- "The Threshold and the Breath" pushed to suibhne.bot

## [03:16] Phase 3 Verification Results
- 11/15 passing — auth broker, calendar, contacts, config, automation all ✅
- ❌ location.get still broken — hangs (async CoreLocation issue, not just escaping)
- Auth broker correctly validates providers against allowlist
- Calendar now working with fresh launch
- Dispatching Muirgen for location handler rewrite

## [03:13] Phase 3: Auth Broker Complete ✅
- Muirgen implemented full auth broker per ADR-001
- AES-256-GCM encrypted token store at ~/.tairseach/auth/
- Master key in macOS Keychain (one-time approval)
- Google OAuth PKCE flow (needs real client credentials from Geilt)
- 8 auth.* socket proxy methods
- gog-proxy wrapper script created
- Location handler fix included (Swift string escaping)
- Built and signed clean

## [03:08] Permissions Panel UI Fixed ✅
- **Gwrhyr** fixed layout thrash, stale status, added background polling
- Loading state no longer replaces card grid
- Focused polling after permission request (2s intervals, 30s max)
- Background refresh every 12s while view is active
- Smooth transitions on status changes
- Frontend builds clean, Tauri build awaiting Muirgen's auth module

## [03:05] Full Verification Sweep Results
- 7/16 passing, 1 bug (location.get AppleScript escaping), 8 permission-blocked
- Calendar permission was wiped by TCC reset during rebuild — needs re-grant
- Dispatching Muirgen for location fix, then Phase 3

## [03:02] Phase 1+2: All Handlers + Config Manager Complete ✅
- Muirgen implemented 5 new handlers: location, screen, files, automation, config
- Config Manager frontend was already functional (not just a stub)
- Socket proxy config methods (config.get/config.set) added
- Built and signed clean
- Dispatching Lomna for full verification sweep

## [02:59] Phase 1: Handler Verification Complete ✅
- **Lomna** verified all existing handlers
- Contacts: ✅ full CRUD against real Address Book
- Calendar: ⚠️ handler works, permission not yet granted
- Reminders: ⚠️ handler works, permission not yet granted
- Error handling: ✅ JSON-RPC 2.0 compliant
- Socket stable, all responses within timeout
- **Action needed:** Grant calendar + reminders permissions in System Settings

## [02:56] Phase 1 Verification + Phase 2 Implementation Dispatched
- **Lomna** → testing contacts, calendar, reminders handlers against live data
- **Muirgen** → implementing missing handlers (location, screen, files, automation) + Config Manager
- Both running in parallel

## [02:53] Phase 3: Auth Broker Design Complete ✅
- Fedelm produced ADR-001-auth-broker-design.md
- Key finding: `gog` supports GOG_KEYRING_BACKEND=file — no patching needed
- Wrapper script + Tairseach-managed passphrase is the solution

## [02:50] Phase 3: Auth Broker Design Dispatched (Parallel)
- **Fedelm** spawned → architecture design for Auth Broker
- Will produce ADR-001 in work order decisions folder
- Label: `tairseach-auth-broker-design`

---
