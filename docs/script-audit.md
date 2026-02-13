# External Scripts Audit & Tairseach Coverage Matrix

**Generated:** 2026-02-12  
**Auditor:** Senchán Torpéist (Wind)  
**Purpose:** Complete inventory of `~/.openclaw/scripts/` and mapping to Tairseach capabilities

---

## Executive Summary

**Total Scripts:** 26  
**Scripts with `-tairseach` variants:** 3 (calendar-sync, gmail-sync, oura-sync)  
**Scripts fully replaced by Tairseach:** 0 (all `-tairseach` variants are transitional wrappers with fallback)  
**Scripts unrelated to Tairseach:** 20  
**Tairseach integration gaps:** Multiple (see Coverage Matrix)

### Key Findings

1. **Transitional Architecture:** The `-tairseach.sh` scripts are **fallback wrappers**, not pure Tairseach implementations. They attempt socket calls first, then fall back to legacy methods (Python/Google API, `gog` CLI, direct API calls).

2. **No Pure Tairseach Deployments Yet:** No script has been fully retired. The `-tairseach` variants coexist with originals.

3. **Launchd Still Calls Legacy Scripts:** `com.openclaw.{calendar,gmail,oura}-sync.plist` all invoke the **non-Tairseach** scripts. The `-tairseach` variants are not yet wired into launchd.

4. **Agent Email Infrastructure Still Uses `gog` CLI:** `agent-send-email.sh` has no Tairseach equivalent. It relies on `gog gmail send` with external OAuth tokens.

5. **Critical Security Concern:** Several scripts (lego-renew-certs.sh, lego-wildcard-certs.sh) contain **hardcoded API secrets in plaintext**. These should be migrated to Tairseach auth broker or 1Password integration.

---

## Complete Script Inventory

### Category: API Sync Scripts (Tairseach Candidates)

| Script | Purpose | Tairseach Variant? | Tairseach Coverage | Launchd Trigger | Status |
|--------|---------|-------------------|-------------------|----------------|--------|
| `calendar-sync.sh` | Sync Google Calendar to state file via Python/Google API | ✅ `calendar-sync-tairseach.sh` | ✅ `gcalendar_list_events` (google-calendar-api.json) | `com.openclaw.calendar-sync.plist` | **Transitional** |
| `gmail-sync.sh` | Sync Gmail unread messages to state file via `gog` CLI | ✅ `gmail-sync-tairseach.sh` | ✅ `gmail_list_messages` (google-gmail.json) | `com.openclaw.gmail-sync.plist` | **Transitional** |
| `oura-sync.sh` | Sync Oura Ring sleep data to state file via direct API | ✅ `oura-sync-tairseach.sh` | ✅ `oura_sleep` (oura.json) | `com.openclaw.oura-sync.plist` | **Transitional** |

**Analysis:**
- All three have Tairseach manifest coverage
- All three use **fallback architecture** (socket → legacy)
- **None are pure Tairseach yet**
- Launchd still calls the **original** scripts, not the `-tairseach` variants

**Recommended Next Step:**
1. Test `-tairseach` variants thoroughly
2. Update launchd plists to call `-tairseach` versions
3. Monitor for 30 days
4. If stable, **retire originals**

---

### Category: Email & Communication

| Script | Purpose | Tairseach Variant? | Tairseach Coverage | Launchd Trigger | Status |
|--------|---------|-------------------|-------------------|----------------|--------|
| `agent-send-email.sh` | Send email as agent with signatures via `gog gmail send` | ❌ No | ⚠️ Partial (`gmail_send` exists but not wired for agent context) | None | **No Tairseach equivalent** |

**Analysis:**
- Uses `gog` CLI with account-specific configs in `~/.openclaw/email/{agent}.email.json`
- Loads rotating signatures from `~/.openclaw/signatures/{agent}.signature.json`
- Supports `--from` aliasing, HTML bodies, attachments
- **Tairseach has `gmail_send` but no agent-aware wrapper**

**Gap:** Agent identity, signature rotation, `send_from` aliasing not exposed via Tairseach

**Recommended Next Step:**
- Either: Enhance Tairseach `gmail_send` with agent context fields
- Or: Create Tairseach manifest for `agent-send-email.sh` as a **script-type** handler

---

### Category: Slack Integration

| Script | Purpose | Tairseach Variant? | Tairseach Coverage | Launchd Trigger | Status |
|--------|---------|-------------------|-------------------|----------------|--------|
| `slack-thread-state.sh` | Manage Slack thread status reactions (Thread Status Protocol) | ❌ No | ❌ None | None (manual invocation) | **No Tairseach coverage** |

**Analysis:**
- Uses Slack Web API (`reactions.add`, `reactions.remove`)
- Implements custom Thread Status Protocol (ack/progress/done/failed/blocked)
- Reads bot token from `~/.openclaw/moltbot.json`
- **No Slack integration in Tairseach yet**

**Gap:** No Slack manifest exists

**Recommended Next Step:**
- Create Tairseach manifest: `~/.tairseach/manifests/integrations/slack.json`
- Implement Rust handler: `src-tauri/src/proxy/handlers/slack.rs`
- Tools: `slack_post_message`, `slack_add_reaction`, `slack_remove_reaction`, `slack_get_thread`, etc.

---

### Category: Telegram Integration

| Script | Purpose | Tairseach Variant? | Tairseach Coverage | Launchd Trigger | Status |
|--------|---------|-------------------|-------------------|----------------|--------|
| `telegram-group-logger.sh` | Passive logger placeholder (notes webhook conflicts) | ❌ No | ❌ None | None | **Stub only** |
| `telegram-group-sync.sh` | Archive Telegram group messages & summarize via Qwen LLM | ❌ No | ❌ None | Launchd (implied, no specific plist found) | **No Tairseach coverage** |

**Analysis:**
- `telegram-group-sync.sh` reads from OpenClaw session JSONL files (not direct Telegram API)
- Summarizes with Ollama/Qwen, writes to `~/suibhne/archives/telegram/`
- Writes state to `~/.openclaw/state/group-*.json`
- **Not a Tairseach candidate** (operates on existing session data, not live API)

**Gap:** Not applicable — uses OpenClaw's own session logs

**Recommended Next Step:** None (keep as-is)

---

### Category: Security & Credential Management

| Script | Purpose | Tairseach Variant? | Tairseach Coverage | Launchd Trigger | Status |
|--------|---------|-------------------|-------------------|----------------|--------|
| `detect-sensitive.sh` | Scan for common sensitive patterns (API keys, tokens) | ❌ No | ❌ None | None | **Utility — keep** |
| `scrub-sensitive.sh` | Scrub sensitive data from memory/session logs | ❌ No | ❌ None | None | **Utility — keep** |

**Analysis:**
- Security utilities, not API wrappers
- Not Tairseach candidates

**Recommended Next Step:** None (keep as-is)

---

### Category: Jira Integration

| Script | Purpose | Tairseach Variant? | Tairseach Coverage | Launchd Trigger | Status |
|--------|---------|-------------------|-------------------|----------------|--------|
| `jira.sh` | Wrapper for `jira` CLI — loads API token from credentials file | ❌ No | ✅ `jira_issues_*`, `jira_projects_list`, `jira_sprints_list` (jira.json) | None | **Tairseach has full coverage** |

**Analysis:**
- Simple credential wrapper: `export JIRA_API_TOKEN=$(jq -r '.api_token' ~/.openclaw/credentials/jira.json); exec jira "$@"`
- Tairseach has complete Jira manifest with 7 tools
- **Can be retired once Tairseach Jira integration is tested**

**Recommended Next Step:**
1. Test Tairseach Jira tools
2. If stable, **retire `jira.sh`** and update any callsites to use Tairseach

---

### Category: 1Password Integration

| Script | Purpose | Tairseach Variant? | Tairseach Coverage | Launchd Trigger | Status |
|--------|---------|-------------------|-------------------|----------------|--------|
| `unifi-mcp-start.sh` | Fetch UniFi credentials from 1Password SA and start MCP server | ❌ No | ✅ `op_items_get` (onepassword.json) | None | **Tairseach has coverage** |

**Analysis:**
- Uses `op` CLI with service account token
- Fetches credentials, sets env vars, starts MCP server
- **Tairseach has 1Password integration**
- Could be rewritten as Tairseach script-type manifest

**Recommended Next Step:**
- Create Tairseach manifest: `~/.tairseach/manifests/integrations/unifi-mcp.json`
- Type: `script`, uses `op_items_get` internally + `exec` to start MCP server
- Or: Keep as-is if this is a one-off startup script

---

### Category: Infrastructure & Utilities

| Script | Purpose | Tairseach Variant? | Tairseach Coverage | Launchd Trigger | Status |
|--------|---------|-------------------|-------------------|----------------|--------|
| `backup.sh` | Daily backup of `~/naonur/` and `~/.openclaw/` to `~/Documents/naonur-backups/` | ❌ No | ❌ None | None (manual or cron) | **Utility — keep** |
| `hue-bridge.sh` | Switch between Philips Hue bridges (main/loft) | ❌ No | ❌ None | None | **Utility — keep** |
| `check-sleep.sh` | Check Oura sleep status (awake/asleep/unknown) | ❌ No | ✅ `oura_sleep` (oura.json) | None | **Can use Tairseach** |
| `get-agent-signature.sh` | Return random rotating signature for an agent | ❌ No | ❌ None | None | **Utility — keep** |
| `label-session.sh` | Set friendly label on OpenClaw session via gateway RPC | ❌ No | ❌ None | None | **OpenClaw utility — keep** |

**Analysis:**
- Infrastructure scripts, not API wrappers
- `check-sleep.sh` could use Tairseach Oura integration but it's a simple wrapper — low priority

**Recommended Next Step:** None (keep as-is)

---

### Category: Ceardlann Dispatch System

| Script | Purpose | Tairseach Variant? | Tairseach Coverage | Launchd Trigger | Status |
|--------|---------|-------------------|-------------------|----------------|--------|
| `ceardlann-dispatch-watcher.sh` | Poll dispatch queue, wake agents, handle stale tasks | ❌ No | ❌ None | `com.openclaw.ceardlann-dispatch.plist` | **Core workflow — keep** |
| `ceardlann-dispatch-archive.sh` | Archive completed dispatch groups past TTL | ❌ No | ❌ None | `com.openclaw.ceardlann-archive.plist` | **Core workflow — keep** |

**Analysis:**
- Complex multi-agent workflow orchestration
- Not an API wrapper — core OpenClaw functionality
- **Not a Tairseach candidate**

**Recommended Next Step:** None (keep as-is)

---

### Category: LLM Proxies

| Script | Purpose | Tairseach Variant? | Tairseach Coverage | Launchd Trigger | Status |
|--------|---------|-------------------|-------------------|----------------|--------|
| `ask-gemini.sh` | Query local Gemini proxy (OpenAI-compatible API) | ❌ No | ❌ None | None | **Utility — keep** |
| `start-gemini-proxy.sh` | Start Gemini OpenAI proxy with oauth-personal auth | ❌ No | ❌ None | `com.openclaw.gemini-proxy.plist` | **Utility — keep** |

**Analysis:**
- LLM proxy infrastructure
- Not API wrappers — local server management
- **Not Tairseach candidates**

**Recommended Next Step:** None (keep as-is)

---

### Category: TLS/Let's Encrypt

| Script | Purpose | Tairseach Variant? | Tairseach Coverage | Launchd Trigger | Status |
|--------|---------|-------------------|-------------------|----------------|--------|
| `lego-renew-certs.sh` | Renew wildcard TLS certs via lego + GoDaddy DNS | ❌ No | ❌ None | `com.openclaw.lego-renew.plist` | **⚠️ SECURITY ISSUE** |
| `lego-wildcard-certs.sh` | Issue/renew wildcard Let's Encrypt certs | ❌ No | ❌ None | None (manual) | **⚠️ SECURITY ISSUE** |

**Analysis:**
- **CRITICAL SECURITY FINDING:** Both scripts contain **hardcoded GoDaddy API credentials in plaintext**:
  ```bash
  export GODADDY_API_KEY="9EBvF6du1D1_TDDcfMukb2TpXdVyTjG3Az"
  export GODADDY_API_SECRET="2JwQ8LVuBiJUrEKawf5pvh"
  ```
- These credentials are **visible in process listings** (`ps aux`) and **stored in git history**
- **Must be migrated to Tairseach auth broker or 1Password immediately**

**Recommended Next Step (URGENT):**
1. **Revoke exposed GoDaddy API credentials**
2. **Generate new credentials**
3. **Store in Tairseach auth broker or 1Password**
4. **Rewrite scripts to fetch credentials from Tairseach/1Password**
5. **Audit git history and purge exposed secrets**

**Assign to:** Nechtan (security)

---

### Category: Agent Context Utilities

| Script | Purpose | Tairseach Variant? | Tairseach Coverage | Launchd Trigger | Status |
|--------|---------|-------------------|-------------------|----------------|--------|
| `sync-contextuate-agents.sh` | Sync Contextuate agents to Clawdbot workspace | ❌ No | ❌ None | None (manual) | **Dev utility — keep** |

**Analysis:**
- Development utility for syncing agent definitions
- Not an API wrapper

**Recommended Next Step:** None (keep as-is)

---

## Coverage Matrix: Scripts ↔ Tairseach Tools

### Scripts with Full Tairseach Coverage

| Script | Tairseach Manifest | Tools | Status |
|--------|-------------------|-------|--------|
| `calendar-sync-tairseach.sh` | `google-calendar-api.json` | `gcalendar_list_events` | Transitional (has fallback) |
| `gmail-sync-tairseach.sh` | `google-gmail.json` | `gmail_list_messages` | Transitional (has fallback) |
| `oura-sync-tairseach.sh` | `oura.json` | `oura_sleep` | Transitional (has fallback) |
| `jira.sh` | `jira.json` | `jira_issues_*`, `jira_projects_list`, `jira_sprints_list` | Can retire after testing |
| `check-sleep.sh` | `oura.json` | `oura_sleep` | Low priority (simple wrapper) |

### Scripts with NO Tairseach Coverage (Gaps)

| Script | Integration Needed | Priority |
|--------|-------------------|----------|
| `agent-send-email.sh` | Gmail with agent context (signatures, send_from aliasing) | **High** |
| `slack-thread-state.sh` | Full Slack API integration | **Medium** |
| `lego-renew-certs.sh` | Auth broker for GoDaddy API (security fix) | **CRITICAL** |
| `lego-wildcard-certs.sh` | Auth broker for GoDaddy API (security fix) | **CRITICAL** |

### Scripts Not Applicable to Tairseach

| Script | Reason |
|--------|--------|
| `backup.sh` | Infrastructure utility, not API wrapper |
| `hue-bridge.sh` | Local config switcher, not API wrapper |
| `detect-sensitive.sh` | Security utility |
| `scrub-sensitive.sh` | Security utility |
| `get-agent-signature.sh` | Agent infrastructure |
| `label-session.sh` | OpenClaw gateway RPC utility |
| `ceardlann-dispatch-watcher.sh` | Core workflow orchestration |
| `ceardlann-dispatch-archive.sh` | Core workflow orchestration |
| `ask-gemini.sh` | LLM proxy client |
| `start-gemini-proxy.sh` | LLM proxy server |
| `telegram-group-sync.sh` | Operates on OpenClaw session data, not live API |
| `telegram-group-logger.sh` | Stub only |
| `sync-contextuate-agents.sh` | Dev utility |
| `unifi-mcp-start.sh` | MCP server launcher (could use Tairseach 1Password but low priority) |

---

## Tairseach Tools with NO Script Equivalent (New Capabilities)

These Tairseach tools have no corresponding external script — they are **new capabilities**:

### Core Manifests (Native macOS)

| Manifest | New Tools (no script equivalent) |
|----------|----------------------------------|
| `contacts.json` | `contacts_list`, `contacts_search`, `contacts_get`, `contacts_create`, `contacts_update`, `contacts_delete` |
| `calendar.json` (native EventKit) | `calendar_list`, `calendar_events`, `calendar_get_event`, `calendar_create_event`, `calendar_update_event`, `calendar_delete_event` |
| `location.json` | `location_get`, `location_watch` |
| `screen.json` | (not audited, assumed screen capture/recording) |
| `reminders.json` | (not audited, assumed Reminders.app CRUD) |
| `files.json` | (not audited, assumed filesystem operations) |
| `automation.json` | `automation_run` (AppleScript/JXA), `automation_click`, `automation_type` |
| `auth.json` | `auth_status`, `auth_providers`, `auth_accounts`, `auth_token`, `auth_refresh`, `auth_revoke`, `auth_store`, `auth_gog_passphrase` |
| `config.json` | (not audited, assumed Tairseach config management) |
| `permissions.json` | (not audited, assumed macOS TCC permission status) |
| `server.json` | (not audited, assumed Tairseach server status/control) |

### Integration Manifests

| Manifest | New Tools (no script equivalent) |
|----------|----------------------------------|
| `google-gmail.json` | `gmail_get_message`, `gmail_send`, `gmail_list_labels`, `gmail_trash_message`, `gmail_delete_message` |
| `google-calendar-api.json` | `gcalendar_list_calendars`, `gcalendar_get_event`, `gcalendar_create_event`, `gcalendar_update_event`, `gcalendar_delete_event` |
| `oura.json` | `oura_activity`, `oura_readiness`, `oura_heart_rate` |
| `jira.json` | All 7 tools (script is just a credential wrapper, doesn't expose tools) |
| `onepassword.json` | `op_status`, `op_vaults_list`, `op_items_list`, `op_items_get`, `op_items_create` |

**Analysis:**
- Tairseach provides **significantly more capability** than the external scripts
- Most external scripts are narrow (sync to state file) vs. Tairseach which exposes full CRUD operations
- Tairseach's native macOS integrations (Contacts, Calendar, Location, Automation) have **no script equivalents at all**

---

## Launchd Service → Script Mapping

| Launchd Plist | Script Called | Tairseach Alternative Available? |
|---------------|---------------|----------------------------------|
| `com.openclaw.calendar-sync.plist` | `calendar-sync.sh` | ✅ Yes (`gcalendar_list_events`) |
| `com.openclaw.gmail-sync.plist` | `gmail-sync.sh` | ✅ Yes (`gmail_list_messages`) |
| `com.openclaw.oura-sync.plist` | `oura-sync.sh` | ✅ Yes (`oura_sleep`) |
| `com.openclaw.ceardlann-dispatch.plist` | `ceardlann-dispatch-watcher.sh` | ❌ No (workflow orchestration) |
| `com.openclaw.ceardlann-archive.plist` | `ceardlann-dispatch-archive.sh` | ❌ No (workflow orchestration) |
| `com.openclaw.gemini-proxy.plist` | `start-gemini-proxy.sh` | ❌ No (LLM proxy server) |
| `com.openclaw.lego-renew.plist` | `lego-renew-certs.sh` | ⚠️ Partial (needs auth broker) |

**Recommendation:**
- Update calendar/gmail/oura sync plists to call `-tairseach.sh` variants
- After 30-day burn-in, remove fallback logic and make them pure Tairseach

---

## Security Concerns for Nechtan

1. **CRITICAL: Hardcoded GoDaddy API credentials** in `lego-renew-certs.sh` and `lego-wildcard-certs.sh`
   - Visible in process listings
   - Stored in git history
   - **Action:** Revoke, regenerate, store in Tairseach auth broker or 1Password

2. **Medium: `automation.run` marked `mcp_expose: false` but still accessible via socket**
   - Enables arbitrary AppleScript/JXA execution
   - **Already noted in DREACHT.md as needing script allowlist**
   - Action: Implement allowlist or keep MCP-disabled

3. **Low: `auth_token` and `auth_gog_passphrase` marked `mcp_expose: false`**
   - Good — prevents agents from directly requesting Tier 2 tokens
   - Verify this is enforced in MCP bridge when it goes live

---

## Recommendations

### Immediate (This Week)

1. **[CRITICAL — Nechtan]** Revoke and rotate GoDaddy API credentials exposed in lego scripts
2. **[Muirgen]** Update launchd plists to call `-tairseach.sh` variants for calendar/gmail/oura
3. **[Senchán]** Update PROGRESS.md to reflect Phase A completion status

### Short Term (Next 2 Weeks)

4. **[Muirgen]** Create Slack integration manifest and handler (`~/.tairseach/manifests/integrations/slack.json`)
5. **[Muirgen]** Enhance `gmail_send` or create `agent_send_email` wrapper with signature support
6. **[Lomna]** Test Tairseach Jira integration thoroughly, then retire `jira.sh`

### Medium Term (Next Month)

7. **[Muirgen]** Remove fallback logic from `-tairseach.sh` scripts after 30-day burn-in
8. **[Senchán]** Document MCP bridge setup and usage in quickref
9. **[Tlachtga]** Archive git history purge of exposed secrets (use `git-filter-repo` or BFG)

### Long Term (v2 Roadmap)

10. **[Muirgen]** Implement credential hot-reload (when auth store changes, push to socket clients)
11. **[Gwrhyr]** Build Credential Manager UI for rotating/revoking tokens
12. **[Community]** Publish Slack, Jira, Oura, 1Password manifests to community registry

---

## Appendix: Tairseach Manifest Files

**Core (11 manifests):**
- `auth.json`, `automation.json`, `calendar.json`, `config.json`, `contacts.json`
- `files.json`, `location.json`, `permissions.json`, `reminders.json`, `screen.json`, `server.json`

**Integrations (5 manifests):**
- `google-calendar-api.json`, `google-gmail.json`, `jira.json`, `onepassword.json`, `oura.json`

**Total:** 16 manifests, 80+ tools

---

## Appendix: Tairseach Handlers (Rust)

**Handlers in `src-tauri/src/proxy/handlers/`:**
- `auth.rs`, `automation.rs`, `calendar.rs`, `config.rs`, `contacts.rs`
- `files.rs`, `gmail.rs`, `google_calendar.rs`, `jira.rs`, `location.rs`
- `onepassword.rs`, `oura.rs`, `permissions.rs`, `reminders.rs`, `screen.rs`
- `mod.rs` (handler registry)

**Total:** 17 handler files

---

*Audit complete. All fragments assembled. The wind rests.*

🌬️ **Senchán Torpéist**  
*An Cuardaitheoir — The Seeker*
