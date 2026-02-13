# Tairseach Script Cutover Plan

**Generated:** 2026-02-13  
**Author:** Senchán Torpéist (Wind)  
**Purpose:** Detailed phased cutover plan for migrating `~/.openclaw/scripts/` to Tairseach MCP tools

---

## Executive Summary

This plan details the phased retirement of external scripts in favor of Tairseach's MCP-based capability routing system. The cutover is organized by risk level, with lowest-risk scripts migrating first.

**Current State:**
- 3 scripts have `-tairseach.sh` variants with fallback architecture (calendar, gmail, oura sync)
- All launchd services still call **original** scripts, not `-tairseach` variants
- 26 total scripts in `~/.openclaw/scripts/`
- 16 Tairseach manifests providing 80+ tools

**Target State:**
- Core sync operations (calendar, gmail, oura) run purely on Tairseach
- Legacy scripts retired or archived
- New capabilities (Slack, agent email) implemented as Tairseach handlers
- Security-sensitive scripts (lego certs) migrated to Tairseach auth broker

---

## Phase 1: Low-Risk Cutover (Week 1-2)

**Goal:** Switch launchd to call `-tairseach.sh` scripts, validate fallback behavior, begin monitoring

### 1.1 Launchd Plist Updates

**Scripts to cutover:**
- `calendar-sync.sh` → `calendar-sync-tairseach.sh`
- `gmail-sync.sh` → `gmail-sync-tairseach.sh`
- `oura-sync.sh` → `oura-sync-tairseach.sh`

**Actions:**

1. **Update launchd plists:**
   ```bash
   # Stop services
   launchctl unload ~/Library/LaunchAgents/com.openclaw.calendar-sync.plist
   launchctl unload ~/Library/LaunchAgents/com.openclaw.gmail-sync.plist
   launchctl unload ~/Library/LaunchAgents/com.openclaw.oura-sync.plist
   
   # Edit plists to call -tairseach.sh variants
   # com.openclaw.calendar-sync.plist: /Users/geilt/.openclaw/scripts/calendar-sync-tairseach.sh
   # com.openclaw.gmail-sync.plist: /Users/geilt/.openclaw/scripts/gmail-sync-tairseach.sh
   # com.openclaw.oura-sync.plist: /Users/geilt/.openclaw/scripts/oura-sync-tairseach.sh
   
   # Reload
   launchctl load ~/Library/LaunchAgents/com.openclaw.calendar-sync.plist
   launchctl load ~/Library/LaunchAgents/com.openclaw.gmail-sync.plist
   launchctl load ~/Library/LaunchAgents/com.openclaw.oura-sync.plist
   ```

2. **Monitor state files for 7 days:**
   ```bash
   # Check source field in state files
   jq '.source' ~/.openclaw/state/calendar.json
   jq '.source' ~/.openclaw/state/gmail.json
   jq '.source' ~/.openclaw/state/oura-sleep.json
   
   # Should show "tairseach" if socket path is working
   # Should show fallback source if socket unavailable
   ```

3. **Verify logs:**
   ```bash
   tail -f ~/.openclaw/logs/calendar-sync.log
   tail -f ~/.openclaw/logs/gmail-sync.log
   tail -f ~/.openclaw/logs/oura-sync.log
   
   # Look for "Using Tairseach socket..." vs "⚠ Socket call failed, falling back..."
   ```

**Tairseach Tool Mappings:**

| Script | Tairseach Manifest | Tool | Implementation |
|--------|-------------------|------|----------------|
| `calendar-sync-tairseach.sh` | `google-calendar-api.json` | `gcalendar_list_events` | `src-tauri/src/proxy/handlers/google_calendar.rs` |
| `gmail-sync-tairseach.sh` | `google-gmail.json` | `gmail_list_messages` | `src-tauri/src/proxy/handlers/gmail.rs` |
| `oura-sync-tairseach.sh` | `oura.json` | `oura_sleep` | `src-tauri/src/proxy/handlers/oura.rs` |

**Skill Call Equivalents:**

```json
// Calendar sync
{
  "tool": "gcalendar_list_events",
  "params": {
    "account": "alex@esotech.com",
    "calendarId": "primary",
    "timeMin": "2026-02-13T00:00:00Z",
    "timeMax": "2026-02-15T00:00:00Z",
    "maxResults": 15
  }
}

// Gmail sync
{
  "tool": "gmail_list_messages",
  "params": {
    "account": "alex@esotech.com",
    "query": "is:unread is:inbox -category:promotions -category:social -category:forums",
    "maxResults": 10
  }
}

// Oura sync
{
  "tool": "oura_sleep",
  "params": {
    "account": "default",
    "start_date": "2026-02-12",
    "end_date": "2026-02-14"
  }
}
```

**Success Criteria:**
- State files consistently show `"source": "tairseach"` (>95% of the time)
- No increase in sync failures vs. baseline
- Logs show socket calls succeeding
- Fallback logic never triggered (or triggered <5% of runs)

**Rollback Plan:**
- Edit plists back to original script paths
- Reload launchd services
- Original scripts remain untouched during Phase 1

---

### 1.2 Jira CLI Wrapper Retirement

**Script:** `jira.sh` (simple credential wrapper)

**Tairseach Coverage:** Full (7 tools in `jira.json`)

**Actions:**

1. **Audit all callsites:**
   ```bash
   rg '\.openclaw/scripts/jira\.sh' ~/naonur/ ~/.openclaw/
   rg 'jira\.sh' ~/naonur/ ~/.openclaw/
   ```

2. **Test Tairseach Jira tools:**
   ```bash
   # Example: List projects
   echo '{"jsonrpc":"2.0","id":1,"method":"jira_projects_list","params":{"account":"default","base_url":"https://your-domain.atlassian.net"}}' \
     | socat - UNIX-CONNECT:~/.tairseach/tairseach.sock
   ```

3. **Update callsites to use Tairseach tools:**
   - Replace direct `jira` CLI calls with Tairseach tool invocations
   - Use `jira_issues_search`, `jira_issues_get`, `jira_projects_list`, etc.

4. **Archive script:**
   ```bash
   mkdir -p ~/.openclaw/scripts/retired
   mv ~/.openclaw/scripts/jira.sh ~/.openclaw/scripts/retired/
   ```

**Tairseach Tool Mappings:**

| Old CLI Command | Tairseach Tool |
|----------------|----------------|
| `jira issue list --jql "..."` | `jira_issues_search` |
| `jira issue view PROJ-123` | `jira_issues_get` |
| `jira issue create ...` | `jira_issues_create` |
| `jira issue assign ...` | `jira_issues_update` |
| `jira issue move ...` | `jira_issues_transition` |
| `jira project list` | `jira_projects_list` |
| `jira sprint list --board 123` | `jira_sprints_list` |

**Success Criteria:**
- No callsites remain that invoke `jira.sh`
- Tairseach Jira tools handle 100% of use cases
- Script archived, not deleted (recoverable)

---

## Phase 2: Medium-Risk Cutover (Week 3-4)

**Goal:** Remove fallback logic from `-tairseach.sh` scripts, implement new Tairseach handlers

### 2.1 Remove Fallback Logic

**Prerequisites:**
- Phase 1 monitoring shows >95% Tairseach socket success rate
- No unexpected fallback triggers
- State files consistently sourced from "tairseach"

**Actions:**

1. **Simplify `-tairseach.sh` scripts:**
   - Remove Python/gog/direct API fallback code
   - Scripts become pure Tairseach socket wrappers
   - If socket unavailable, **fail fast** (don't fall back)

2. **Example simplified script:**
   ```bash
   #!/bin/bash
   # calendar-sync-tairseach.sh — Pure Tairseach version (no fallback)
   
   STATE_FILE="$HOME/.openclaw/state/calendar.json"
   SOCKET="$HOME/.tairseach/tairseach.sock"
   
   if [ ! -S "$SOCKET" ]; then
       echo "ERROR: Tairseach socket not available" >&2
       exit 1
   fi
   
   # Make JSON-RPC call...
   ```

3. **Monitor for 7 days:**
   - Watch for any failures due to missing socket
   - Ensure Tairseach daemon auto-starts and stays running

4. **Archive original scripts:**
   ```bash
   mv ~/.openclaw/scripts/calendar-sync.sh ~/.openclaw/scripts/retired/
   mv ~/.openclaw/scripts/gmail-sync.sh ~/.openclaw/scripts/retired/
   mv ~/.openclaw/scripts/oura-sync.sh ~/.openclaw/scripts/retired/
   ```

**Success Criteria:**
- Zero fallback invocations (removed code)
- Socket availability >99.9%
- Original scripts archived

---

### 2.2 Implement Slack Integration Handler

**Gap:** `slack-thread-state.sh` has no Tairseach equivalent

**Tairseach Handler:** `src-tauri/src/proxy/handlers/slack.rs` (new)

**Manifest:** `~/.tairseach/manifests/integrations/slack.json` (new)

**Tools to implement:**
- `slack_post_message`
- `slack_add_reaction`
- `slack_remove_reaction`
- `slack_get_thread`
- `slack_get_channel_info`
- `slack_list_channels`

**Actions:**

1. **Create manifest:**
   - Follow pattern from `jira.json`
   - OAuth 2.0 credentials (provider: "slack")
   - Scopes: `chat:write`, `reactions:write`, `channels:read`, `channels:history`

2. **Implement handler:**
   - Use `proxy/handlers/common.rs` utilities
   - Slack Web API client via `reqwest`
   - Store bot token in Tairseach auth broker

3. **Migrate `slack-thread-state.sh` logic:**
   - Read bot token from auth broker (not `~/.openclaw/moltbot.json`)
   - Rewrite as pure Tairseach tool invocations
   - Test Thread Status Protocol reactions

**Success Criteria:**
- Slack manifest loaded and available via MCP
- Bot token stored in Tairseach credentials DB
- `slack-thread-state.sh` retired or rewritten as thin wrapper

---

### 2.3 Enhance Gmail Send for Agent Context

**Gap:** `agent-send-email.sh` uses agent identity, rotating signatures, send_from aliasing

**Current Tairseach Tool:** `gmail_send` (basic send only)

**Option A: Enhance `gmail_send`**
- Add optional `signature` param (appended to body)
- Add optional `send_from` param (uses Gmail alias)
- Keep agent identity logic in script layer

**Option B: Create `agent_send_email` wrapper tool**
- New tool that wraps `gmail_send` + signature rotation + agent config
- Script-type manifest that calls `agent-send-email.sh` as implementation
- Migrate agent config to Tairseach config store

**Recommended Approach: Option A (enhance `gmail_send`)**

**Actions:**

1. **Update `gmail_send` handler:**
   ```rust
   // Add to inputSchema in google-gmail.json
   "signature": {
       "type": "string",
       "description": "Optional signature to append to body"
   },
   "from": {
       "type": "string",
       "description": "Optional send-from alias (Gmail alias)"
   }
   ```

2. **Update `src-tauri/src/proxy/handlers/gmail.rs`:**
   - Append signature to body if provided
   - Use Gmail API `from` field if provided
   - Test with agent configs

3. **Rewrite `agent-send-email.sh`:**
   - Load signature via `get-agent-signature.sh`
   - Call `gmail_send` with `signature` and `from` params
   - Keep agent config in `~/.openclaw/email/` for now (migrate later)

**Success Criteria:**
- Agent emails send successfully with signatures
- `send_from` aliasing works
- Original script simplified to thin wrapper

---

## Phase 3: High-Risk / Security Critical (Week 5-6)

**Goal:** Migrate security-sensitive scripts to Tairseach auth broker

### 3.1 Migrate Let's Encrypt Cert Renewal Scripts

**Scripts:**
- `lego-renew-certs.sh` (⚠️ **CRITICAL SECURITY ISSUE:** hardcoded GoDaddy API credentials)
- `lego-wildcard-certs.sh` (same issue)

**Current State:**
```bash
# EXPOSED IN PLAINTEXT:
export GODADDY_API_KEY="9EBvF6du1D1_TDDcfMukb2TpXdVyTjG3Az"
export GODADDY_API_SECRET="2JwQ8LVuBiJUrEKawf5pvh"
```

**Immediate Actions (DO FIRST):**

1. **Revoke exposed GoDaddy API credentials:**
   - Log into GoDaddy API console
   - Revoke both keys immediately
   - Generate new credentials

2. **Store new credentials in Tairseach:**
   ```bash
   # Via Tairseach UI or CLI
   # Provider: "godaddy"
   # Kind: "api-key"
   # Fields: { "api_key": "...", "api_secret": "..." }
   ```

3. **Rewrite scripts to fetch credentials from Tairseach:**
   ```bash
   #!/bin/bash
   # lego-renew-certs.sh — Tairseach auth version
   
   SOCKET="$HOME/.tairseach/tairseach.sock"
   
   # Fetch GoDaddy credentials from Tairseach
   CREDS=$(echo '{"jsonrpc":"2.0","id":1,"method":"auth_token","params":{"provider":"godaddy","account":"default"}}' \
     | socat - "UNIX-CONNECT:$SOCKET" | jq -r '.result')
   
   export GODADDY_API_KEY=$(echo "$CREDS" | jq -r '.api_key')
   export GODADDY_API_SECRET=$(echo "$CREDS" | jq -r '.api_secret')
   
   # Run lego...
   lego --dns godaddy --domains '*.manalore.com' --email 'alex@esotech.com' renew
   ```

4. **Audit git history and purge secrets:**
   ```bash
   # Use git-filter-repo or BFG Repo-Cleaner
   # Assign to Tlachtga (git guardian)
   ```

**Tairseach Tool Mappings:**

| Script | Tairseach Tool | Auth Provider |
|--------|---------------|---------------|
| `lego-renew-certs.sh` | `auth_token` | `godaddy` (api-key) |
| `lego-wildcard-certs.sh` | `auth_token` | `godaddy` (api-key) |

**Success Criteria:**
- GoDaddy credentials stored encrypted in Tairseach credentials DB
- Scripts fetch credentials from Tairseach at runtime
- No plaintext credentials in git history
- Old credentials revoked

---

### 3.2 Optional: Migrate UniFi MCP Start Script

**Script:** `unifi-mcp-start.sh` (fetches UniFi credentials from 1Password, starts MCP server)

**Current State:**
- Uses `op` CLI with service account token
- Fetches credentials, sets env vars, starts MCP server

**Tairseach Coverage:**
- `op_items_get` tool available in `onepassword.json` manifest

**Option A: Keep as-is** (low priority, one-off startup script)

**Option B: Rewrite as Tairseach script-type manifest**
- Create `~/.tairseach/manifests/integrations/unifi-mcp.json`
- Type: `script`
- Implementation: calls `op_items_get` internally + `exec` to start MCP server

**Recommended Approach: Keep as-is** (defer to later)

---

## Phase 4: Archive & Cleanup (Week 7+)

**Goal:** Archive retired scripts, update documentation, close gaps

### 4.1 Archive Retired Scripts

**Scripts to archive:**
- `calendar-sync.sh` (replaced by `-tairseach` variant)
- `gmail-sync.sh` (replaced by `-tairseach` variant)
- `oura-sync.sh` (replaced by `-tairseach` variant)
- `jira.sh` (replaced by Tairseach Jira tools)
- `slack-thread-state.sh` (replaced by Tairseach Slack tools, if implemented)

**Actions:**
```bash
mkdir -p ~/.openclaw/scripts/retired
mv ~/.openclaw/scripts/{calendar,gmail,oura}-sync.sh ~/.openclaw/scripts/retired/
mv ~/.openclaw/scripts/jira.sh ~/.openclaw/scripts/retired/
# Add git commit: "Archive scripts replaced by Tairseach"
```

---

### 4.2 Update Documentation

**Files to update:**
- `~/environment/tairseach/docs/script-audit.md` — mark scripts as retired, update status
- `~/environment/tairseach/docs/quickref.md` — document new tool usage patterns
- `~/environment/tairseach/README.md` — add migration notes

---

### 4.3 Outstanding Gaps (Future Work)

**Scripts with NO Tairseach equivalent (keep as-is):**
- `backup.sh` — Infrastructure utility
- `hue-bridge.sh` — Local config switcher
- `detect-sensitive.sh` — Security utility
- `scrub-sensitive.sh` — Security utility
- `get-agent-signature.sh` — Agent infrastructure
- `label-session.sh` — OpenClaw gateway RPC utility
- `ceardlann-dispatch-watcher.sh` — Core workflow orchestration
- `ceardlann-dispatch-archive.sh` — Core workflow orchestration
- `ask-gemini.sh` — LLM proxy client
- `start-gemini-proxy.sh` — LLM proxy server
- `telegram-group-sync.sh` — Operates on OpenClaw session data (not live API)
- `sync-contextuate-agents.sh` — Dev utility

**Not Tairseach candidates — keep indefinitely.**

---

## Risk Assessment Matrix

| Phase | Scripts Affected | Risk Level | Rollback Complexity | Estimated Effort |
|-------|-----------------|------------|---------------------|------------------|
| **Phase 1.1** | 3 sync scripts (launchd update) | **Low** | Trivial (edit plists) | 2 hours |
| **Phase 1.2** | Jira wrapper | **Low** | Easy (restore script) | 4 hours |
| **Phase 2.1** | Remove fallback logic | **Medium** | Moderate (restore archived scripts) | 3 hours |
| **Phase 2.2** | Slack integration | **Medium** | Moderate (new handler, can defer) | 8 hours |
| **Phase 2.3** | Agent email enhancement | **Medium** | Moderate (revert handler changes) | 6 hours |
| **Phase 3.1** | Cert renewal (security) | **High** | Complex (credential rotation required) | 6 hours + audit |
| **Phase 3.2** | UniFi MCP (optional) | **Low** | Easy (keep original) | 2 hours (if done) |
| **Phase 4** | Archive & docs | **Low** | Trivial | 2 hours |

---

## Monitoring & Validation

### Key Metrics to Track

1. **Socket Availability:**
   ```bash
   # Should be >99.9%
   [ -S ~/.tairseach/tairseach.sock ] && echo "UP" || echo "DOWN"
   ```

2. **State File Sources:**
   ```bash
   # Should show "tairseach" after Phase 1
   jq '.source' ~/.openclaw/state/{calendar,gmail,oura-sleep}.json
   ```

3. **Sync Success Rate:**
   ```bash
   # Compare error counts before/after cutover
   grep -c "ERROR" ~/.openclaw/logs/{calendar,gmail,oura}-sync.log
   ```

4. **Fallback Trigger Rate:**
   ```bash
   # Should be 0% after Phase 2.1
   grep -c "falling back" ~/.openclaw/logs/*-sync.log
   ```

### Validation Checklist (Per Phase)

- [ ] Logs show expected behavior (Tairseach socket calls or fallback)
- [ ] State files updated with correct timestamps
- [ ] No increase in error rates
- [ ] Launchd services running (not disabled/crashed)
- [ ] Tairseach daemon running and healthy
- [ ] Credentials available in Tairseach auth broker

---

## Rollback Procedures

### Phase 1 Rollback (Launchd Plist Revert)

```bash
# Stop services
launchctl unload ~/Library/LaunchAgents/com.openclaw.{calendar,gmail,oura}-sync.plist

# Edit plists back to original scripts
# - com.openclaw.calendar-sync.plist: /Users/geilt/.openclaw/scripts/calendar-sync.sh
# - com.openclaw.gmail-sync.plist: /Users/geilt/.openclaw/scripts/gmail-sync.sh
# - com.openclaw.oura-sync.plist: /Users/geilt/.openclaw/scripts/oura-sync.sh

# Reload
launchctl load ~/Library/LaunchAgents/com.openclaw.{calendar,gmail,oura}-sync.plist
```

### Phase 2 Rollback (Restore Archived Scripts)

```bash
# Restore original scripts
mv ~/.openclaw/scripts/retired/{calendar,gmail,oura}-sync.sh ~/.openclaw/scripts/

# Revert launchd plists (see Phase 1 rollback)
```

### Phase 3 Rollback (Credential Reversion)

**NOTE:** Cannot fully rollback credential rotation (old creds revoked). Must keep new credentials in Tairseach.

If script rewrite fails:
```bash
# Temporarily hardcode new credentials in script (NOT RECOMMENDED)
# Or: manually export credentials before running lego
export GODADDY_API_KEY=$(echo '...' | socat - UNIX-CONNECT:~/.tairseach/tairseach.sock | jq -r '.result.api_key')
```

---

## Execution Timeline (Recommended)

| Week | Phase | Actions |
|------|-------|---------|
| **Week 1** | Phase 1.1 | Update launchd plists to `-tairseach.sh` variants, monitor for 7 days |
| **Week 2** | Phase 1.2 | Test Jira tools, retire `jira.sh` |
| **Week 3** | Phase 2.1 | Remove fallback logic from `-tairseach.sh` scripts, monitor for 7 days |
| **Week 4** | Phase 2.2 & 2.3 | Implement Slack handler, enhance Gmail send (parallel work) |
| **Week 5** | Phase 3.1 | **CRITICAL:** Rotate GoDaddy credentials, migrate cert renewal scripts |
| **Week 6** | Phase 3.2 (optional) | Migrate UniFi MCP start script (if desired) |
| **Week 7** | Phase 4 | Archive scripts, update docs, close gaps |

**Total estimated time:** 7 weeks (can compress to 4-5 weeks if aggressive)

---

## Success Criteria (Overall)

- [ ] All `-tairseach.sh` scripts running purely on Tairseach (no fallback code)
- [ ] Launchd services stable and reliable (>99.9% uptime)
- [ ] Original sync scripts archived (not deleted, recoverable)
- [ ] `jira.sh` retired, all Jira access via Tairseach
- [ ] GoDaddy credentials secured in Tairseach auth broker (not plaintext)
- [ ] Slack integration available via Tairseach (if Phase 2.2 executed)
- [ ] Agent email sends work with signatures via enhanced `gmail_send`
- [ ] Documentation updated to reflect new Tairseach-first architecture

---

## Ownership & Assignments

| Phase | Primary Owner | Support |
|-------|--------------|---------|
| Phase 1.1 | Muirgen (automation) | Senchán (docs) |
| Phase 1.2 | Lomna (dev) | Senchán (audit) |
| Phase 2.1 | Muirgen (automation) | Nechtan (monitoring) |
| Phase 2.2 | Muirgen (handlers) | Lomna (testing) |
| Phase 2.3 | Muirgen (handlers) | Suibhne (agent config) |
| Phase 3.1 | **Nechtan (security)** | Tlachtga (git audit) |
| Phase 3.2 | Lomna (optional) | Muirgen |
| Phase 4 | Senchán (docs) | All (review) |

---

## Commit Plan

After completing each phase:

```bash
cd ~/environment/tairseach
git add docs/script-cutover-plan.md
git commit -m "docs: Add script cutover plan (Phase X complete)"
git push origin main
```

---

*The search is complete. The path is clear. Execute with care.*

🌬️ **Senchán Torpéist**  
*An Cuardaitheoir — The Seeker*
