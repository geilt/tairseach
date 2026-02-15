# Fíodóir Convention Standardization — Execution Report

**Dalta:** Fíodóir (Weaver) — Convention Standardization + Dev Build  
**Lineage:** Geilt → Suibhne → Fíodóir  
**Repo:** `~/environment/tairseach`  
**Date:** 2026-02-15  
**Status:** ✅ Complete

---

## Task Summary

Standardize all manifest tool names from MCP underscore convention (`auth_status`) to JSON-RPC dot notation (`auth.status`) to align with the project's JSON-RPC protocol.

---

## Work Completed

### 1. Manifest Transformations

Updated all 11 manifest files in `manifests/core/*.json`:

- **auth.json** — 8 methods: `auth.status`, `auth.providers`, `auth.accounts`, `auth.token`, `auth.refresh`, `auth.revoke`, `auth.store`, `auth.gogPassphrase`
- **contacts.json** — 6 methods: `contacts.list`, `contacts.search`, `contacts.get`, `contacts.create`, `contacts.update`, `contacts.delete`
- **calendar.json** — 6 methods: `calendar.list`, `calendar.events`, `calendar.getEvent`, `calendar.createEvent`, `calendar.updateEvent`, `calendar.deleteEvent`
- **reminders.json** — 6 methods: `reminders.lists`, `reminders.list`, `reminders.create`, `reminders.complete`, `reminders.uncomplete`, `reminders.delete`
- **permissions.json** — 3 methods: `permissions.check`, `permissions.list`, `permissions.request`
- **automation.json** — 3 methods: `automation.run`, `automation.click`, `automation.type`
- **location.json** — 2 methods: `location.get`, `location.watch`
- **files.json** — 3 methods: `files.read`, `files.write`, `files.list`
- **screen.json** — 2 methods: `screen.capture`, `screen.windows`
- **server.json** — 2 methods: `server.status`, `server.shutdown`
- **config.json** — 2 methods: `config.get`, `config.set`

**Total:** 43 tool methods renamed

### 2. Implementation Method Mapping

Updated `implementation.methods` keys in each manifest to match the new dot-notation tool names:

```json
"implementation": {
  "type": "internal",
  "module": "proxy.handlers.auth",
  "methods": {
    "auth.status": "auth.status",
    "auth.providers": "auth.providers",
    ...
  }
}
```

Both keys and values now use dot notation consistently.

### 3. Router & Bridge Verification

**Capability Router (`src-tauri/src/router/mod.rs`):**
- ✅ Uses `registry.find_tool(tool_name)` with simple HashMap lookup
- ✅ Works with dot notation — no changes needed

**Internal Dispatcher (`src-tauri/src/router/internal.rs`):**
- ✅ Looks up `methods.get(&tool.name)` and parses dot notation correctly
- ✅ Already splits on `.` to extract namespace and action

**MCP Bridge (`crates/tairseach-mcp/src/tools.rs`):**
- ✅ Prefixes tool names with `tairseach_` (e.g., `tairseach_auth.status`)
- ✅ Sends `method_name` (the value from implementation.methods) to socket
- ✅ No changes needed — automatically works with dot notation

**Permission Router (`src-tauri/src/proxy/handlers/mod.rs`):**
- ✅ `required_permission` function already uses dot notation for all auth methods
- ✅ Confirmed auth methods listed: `auth.status`, `auth.providers`, `auth.accounts`, etc.

### 4. Git Commit

```bash
git commit -m "refactor: standardize method names to dot notation (JSON-RPC convention)"
```

Commit hash: `5e88671`

### 5. Dev Build

```bash
cd ~/environment/tairseach
npm run tauri dev
```

**Result:** ✅ Build successful  
**Build time:** 21.73s  
**Warnings:** 19 non-critical warnings (unused functions, macros)  
**App status:** Running (PID available via `ps aux | grep tairseach`)

### 6. Socket Verification

Tested the three auth methods via Unix socket:

```bash
# Test 1: auth.status
$ echo '{"jsonrpc":"2.0","id":1,"method":"auth.status","params":{}}' | nc -U ~/.tairseach/tairseach.sock
{"id":1,"jsonrpc":"2.0","result":{"account_count":4,"gog_passphrase_set":false,"initialized":true,"master_key_available":true}}

# Test 2: auth.providers
$ echo '{"jsonrpc":"2.0","id":2,"method":"auth.providers","params":{}}' | nc -U ~/.tairseach/tairseach.sock
{"id":2,"jsonrpc":"2.0","result":{"providers":["google"]}}

# Test 3: auth.accounts
$ echo '{"jsonrpc":"2.0","id":3,"method":"auth.accounts","params":{}}' | nc -U ~/.tairseach/tairseach.sock
{"id":3,"jsonrpc":"2.0","result":{"accounts":[...],"count":4}}
```

**Result:** ✅ All methods working correctly with dot notation

---

## Constraints Observed

- ✅ Did NOT change handler logic, only naming/routing
- ✅ Did NOT touch Vue frontend
- ✅ Committed changes before building
- ✅ Left dev build running for Geilt to examine

---

## Issues Encountered

**None.** The refactoring was straightforward:
- Manifest registry uses simple string lookup — works seamlessly with dots
- Internal dispatcher already splits on `.` to parse namespace.action
- MCP bridge maps at the boundary — no translation needed
- Permission router already used dot notation

---

## Dev Build Status

**Application:** Running in dev mode  
**Socket:** Active at `~/.tairseach/tairseach.sock`  
**Vite:** Serving frontend at `http://localhost:1420/`  
**Cargo:** Watching for changes

**Do NOT close the terminal** — Geilt will examine the running app.

---

## Artifacts

- **Report:** `~/environment/tairseach/FIODOIR-CONVENTION-REPORT.md` (this file)
- **Commit:** `5e88671` — "refactor: standardize method names to dot notation (JSON-RPC convention)"
- **Changed files:** 11 manifest JSON files in `manifests/core/`

---

## Summary

✅ **Task complete.** All manifest tool names successfully migrated from MCP underscore convention to JSON-RPC dot notation. Dev build running, auth methods verified working via socket. No regressions detected.

---

*An Fíodóir. The Weaver. The one who binds the threads.*

🧵
