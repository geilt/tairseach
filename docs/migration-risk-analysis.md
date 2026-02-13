# Tairseach Script→Skill Migration Risk Analysis

Date: 2026-02-12 (CST)
Author: Fedelm (analysis only)
Scope: retire launchd polling scripts (`oura-sync.sh`, `gmail-sync.sh`, `calendar-sync.sh`) and move to on-demand OpenClaw skills that call `tairseach-mcp` via stdio.

---

## Executive Summary

- **Primary architectural risk:** removing precomputed state files without updating all consumers (notably heartbeat + `global-entry-monitor.py`) will create immediate regressions. **Confidence: High**
- **Primary runtime risk:** on-demand calls add upstream dependency on Tairseach socket availability; state-file reads are near-instant and resilient to transient upstream outages. **Confidence: High**
- **Primary migration opportunity:** large capability expansion from script-only (Oura/Gmail/Calendar polling) to broad tool surface (observed 71 tools in manifests, 63 MCP-exposed). **Confidence: High**
- **Observed count mismatch:** current local manifests do **not** show 69 MCP-exposed tools; they show 71 defined, 8 hidden (`mcp_expose:false`), 63 exposed. **Confidence: High**

---

## 1) Dependency & Removal Risk (Launchd Sync Scripts)

## What currently depends on state files

### Direct operational dependencies
1. `~/naonur/suibhne/HEARTBEAT.md`
   - Reads `~/.openclaw/state/calendar.json`
   - Reads `~/.openclaw/state/oura-sleep.json`
   - Gmail intentionally disabled in heartbeat comments
   - **Confidence: High**

2. `~/.openclaw/scripts/global-entry-monitor.py`
   - Reads `~/.openclaw/state/calendar.json` for conflict checking
   - If calendar state disappears/stales, appointment-fit logic degrades
   - **Confidence: High**

### Non-critical/documentary dependencies
- Multiple memory/docs reference these state files (historical/ops docs), but these are not runtime-critical.
- No active crontab entries found (`crontab -l` => none).
- Runtime scheduling for the 3 sync jobs is via launchd (`com.openclaw.*-sync.plist`, `StartInterval=600`).
- **Confidence: High**

## What happens if Tairseach.app is not running

If using on-demand skill -> `tairseach-mcp` -> socket:
- Tool call fails at socket connect (`socket connect failed` path in `tairseach-mcp/src/tools.rs`).
- No cached state layer unless explicitly preserved.
- User-visible symptom: heartbeat/context tasks lose sleep/calendar freshness at query-time rather than tolerating stale snapshots.
- **Confidence: High**

## Latency difference (measured locally)

Measured on this host:
- State-file read (`oura-sleep.json`): ~**0.017 ms avg** (p95 ~0.019 ms)
- Direct socket call (`server.status` via `socat`): ~**13.478 ms avg** (p95 ~14.039 ms)
- One-shot `tairseach-mcp` spawn + init + call: response returned, but process required forced timeout at ~2s in test harness (indicates lifecycle/exit overhead for naive spawn-per-call approach)

Interpretation:
- Socket call is ~10^3x slower than file read in absolute terms, but still fast for interactive use.
- Spawn-per-call bridge process can dominate latency if not reused.
- **Confidence: Medium-High** (file/socket numbers solid; spawn behavior depends on invocation pattern)

## Socket down scenario and fallback chain

Current launchd scripts already encode fallback behavior (in `*-sync-tairseach.sh` variants):
- Oura: socket -> direct Oura API
- Gmail: socket -> `gog` CLI
- Calendar: socket -> Python Google API

In pure skill/on-demand mode, equivalent fallback must be reintroduced deliberately. Recommended chain:
1. Attempt MCP call (`tairseach_*`)
2. If unavailable, read last-known state snapshot (if maintained)
3. If strict freshness needed, optional direct API fallback
4. Return explicit degraded-status metadata to caller

Without step (2), outages become hard failures. **Confidence: High**

---

## 2) Gap Analysis: MCP Tool Surface vs Current Script Capabilities

## Observed manifest inventory (local)

- Manifest files scanned: 16
- Tool definitions found: 71
- `mcp_expose:false` tools: 8
- MCP-exposed by current manifest policy: 63

> Hidden tools: `server_shutdown`, `config_set`, `files_write`, `automation_run`, `automation_click`, `automation_type`, `auth_token`, `auth_gog_passphrase`

### Full tool list (defined)

- **Auth (8):** auth_status, auth_providers, auth_accounts, auth_token, auth_refresh, auth_revoke, auth_store, auth_gog_passphrase
- **Automation (3):** automation_run, automation_click, automation_type
- **Calendar (EventKit, 6):** calendar_list, calendar_events, calendar_get_event, calendar_create_event, calendar_update_event, calendar_delete_event
- **Config (2):** config_get, config_set
- **Contacts (6):** contacts_list, contacts_search, contacts_get, contacts_create, contacts_update, contacts_delete
- **Files (3):** files_read, files_write, files_list
- **Location (2):** location_get, location_watch
- **Permissions (3):** permissions_check, permissions_list, permissions_request
- **Reminders (6):** reminders_lists, reminders_list, reminders_create, reminders_complete, reminders_uncomplete, reminders_delete
- **Screen (2):** screen_capture, screen_windows
- **Server (2):** server_status, server_shutdown
- **Google Calendar API (6):** gcalendar_list_calendars, gcalendar_list_events, gcalendar_get_event, gcalendar_create_event, gcalendar_update_event, gcalendar_delete_event
- **Google Gmail (6):** gmail_list_messages, gmail_get_message, gmail_send, gmail_list_labels, gmail_trash_message, gmail_delete_message
- **Jira (7):** jira_issues_search, jira_issues_get, jira_issues_create, jira_issues_update, jira_issues_transition, jira_projects_list, jira_sprints_list
- **1Password (5):** op_status, op_vaults_list, op_items_list, op_items_get, op_items_create
- **Oura (4):** oura_sleep, oura_activity, oura_readiness, oura_heart_rate

**Confidence: High**

## Mapping: existing scripts -> replacement tools

| Current script capability | Replacement Tairseach tool(s) | Coverage | Confidence |
|---|---|---|---|
| Oura sleep fetch for date range | `oura_sleep` | Functional fetch parity likely | Medium |
| Gmail unread inbox query | `gmail_list_messages` | Functional parity likely | High |
| Calendar next 48h events | `gcalendar_list_events` | Functional parity likely | High |
| Calendar event CRUD future use | `gcalendar_*` create/update/delete/get | New net capability vs current polling | High |

## New capabilities gained (no current script equivalent)

Large net-new surface beyond polling:
- Contacts CRUD/search
- Reminders CRUD
- Screen capture/window listing
- Location access
- Jira operations
- 1Password item/vault access
- EventKit-native calendar tools (`calendar_*`) in addition to Google Calendar API
- Config/permissions introspection

**Confidence: High**

## Script capabilities with no direct Tairseach equivalent (migration risk)

1. **Precomputed derived state fields** in Oura state (`sleep_score`, stale flags, latest_session normalization)
   - Tool returns raw-ish provider data; transformation layer still needed.
   - **Confidence: Medium**

2. **Persistent cache semantics** (state survives provider/socket downtime)
   - On-demand direct tool call lacks durable snapshot unless explicitly retained.
   - **Confidence: High**

3. **Multi-fallback orchestration baked into scripts** (`socket -> direct API/CLI`)
   - Must be recreated in skill logic or wrapper.
   - **Confidence: High**

4. **Validation/anti-corruption file guardrails** (calendar script avoids overwriting on invalid JSON)
   - Equivalent safeguards required in skill post-processing if writing snapshots.
   - **Confidence: High**

---

## 3) Migration Risk Matrix (scripts to retire)

| Script to retire | Risk | Why | Mitigation | Fallback | Confidence |
|---|---|---|---|---|---|
| `oura-sync.sh` | **Medium** | Heartbeat depends on derived sleep signals; removing state file breaks current heartbeat contract | Add skill wrapper that returns same normalized schema; optionally keep lightweight snapshot write during transition | Read last snapshot if MCP fails; optionally direct API fallback | High |
| `gmail-sync.sh` | **Low-Medium** | Heartbeat email is currently disabled; lower immediate blast radius | Move consumers to on-demand `gmail_list_messages`; preserve error-state structure for observability | If MCP down, skip non-critical email checks with degraded notice | High |
| `calendar-sync.sh` | **High** | Heartbeat + `global-entry-monitor.py` rely on `calendar.json` | Migrate calendar consumers first to skill API or maintain compatibility snapshot until all consumers updated | Snapshot fallback (last-known calendar) + explicit stale timestamp | High |

### Recommended migration order (safest first)

1. **Gmail** (`gmail-sync.sh`) — lowest active dependency pressure
2. **Oura** (`oura-sync.sh`) — moderate dependency, mostly heartbeat logic
3. **Calendar** (`calendar-sync.sh`) — highest coupling; migrate last

**Confidence: High**

---

## 4) Transition Forecast: Skill Wrapper -> Native MCP (mcporter)

## Near-term (current reality: no native MCP in OpenClaw)

Likely stable interim architecture:
- OpenClaw Skill
  - invokes `tairseach-mcp` (stdio)
  - performs `initialize` + `tools/call`
  - applies normalization / fallback / telemetry
- Optional compatibility cache writer for legacy consumers

This creates an adapter boundary now, minimizing future rewrite scope. **Confidence: High**

## When native MCP (`mcporter`) ships

Expected target architecture:
- OpenClaw native MCP client manages server lifecycle and tool discovery
- `tairseach-mcp` configured as MCP server command
- Skills become thin orchestrators (policy + transformation), not transport bridges

### Low-friction transition path

1. Keep skill interface stable (`get_sleep_state`, `get_calendar_window`, etc.)
2. Swap implementation internals from shell/stdio bridge -> native MCP tool call
3. Preserve response schema contracts and fallback semantics
4. Remove compatibility snapshot writes only after all consumers migrated

This yields incremental cutover with minimal downstream breakage. **Confidence: High**

---

## Security Concerns (flag for Nechtan)

1. **Socket dependency trust boundary**
   - Unix socket is `srw-------` (owner-only), good local hardening.
   - Still a high-value local interface to system capabilities.
   - **Confidence: High**

2. **Sensitive auth methods present in manifests**
   - `auth_token` and `auth_gog_passphrase` exist but are hidden from MCP exposure.
   - If raw socket methods are callable by local owner processes, sensitive pathways remain reachable outside MCP exposure controls.
   - **Confidence: Medium**

3. **Capability breadth expansion risk**
   - Migration adds access to 1Password, location, files, contacts, reminders, screen, Jira.
   - Principle-of-least-privilege policy gates should be explicit per skill before broad rollout.
   - **Confidence: High**

4. **Observed Gmail failure in current state**
   - `gmail.json` shows `gog: command not found` under current launchd script path.
   - Indicates existing reliability debt independent of migration.
   - **Confidence: High**

---

## Practical Decision Guidance

- Do **not** remove calendar polling until heartbeat + global-entry consumers are migrated or compatibility snapshots remain.
- Use a **hybrid period**: on-demand skill as primary, snapshot as fallback.
- Standardize degraded responses (`source`, `stale`, `last_success`, `error`) so orchestration can reason safely.

Overall migration viability: **Strong**, if compatibility and fallback layers are explicit.
Confidence in this overall assessment: **High**.
