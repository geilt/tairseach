# Heartbeat Migration Plan: State Files → On-Demand Tairseach MCP

Date: 2026-02-13 (CST)  
Author: Fedelm (analysis)

---

## Executive Decision

**Recommendation: Hybrid migration (not full immediate cutover).**

Use **on-demand MCP calls as primary**, keep **state-file compatibility cache as fallback** during transition.

Why:
- Current heartbeat and at least one other consumer (`global-entry-monitor.py`) still depend on file snapshots.
- MCP/socket introduces a runtime dependency (Tairseach availability) that file reads do not have.
- Hybrid keeps reliability while allowing immediate migration of heartbeat logic to live data.

**Confidence: High**

---

## What Changes in `HEARTBEAT.md`

Current `HEARTBEAT.md` is file-centric (`read ~/.openclaw/state/*.json`).  
After migration, it should become **skill-centric**:

1. Call Tairseach skill endpoints first (sleep/calendar) for live data.
2. Evaluate urgency from normalized response, not raw file format.
3. If MCP call fails/unavailable, read cached state snapshot.
4. If both fail, return degraded-but-safe behavior (`HEARTBEAT_OK` only when no urgent signal is known, otherwise explicit degraded warning).

Net shift:
- From “read static snapshots” → “query live capability with graceful fallback.”

**Confidence: High**

---

## New Heartbeat Flow (Target)

```text
Heartbeat trigger
  ↓
Call skill: tairseach.get_sleep_state (live)
Call skill: tairseach.get_calendar_window (live)
  ↓
If live success: normalize + triage urgency
If live failure: fallback to cached snapshots
If cache also unavailable: mark degraded state and continue with conservative policy
  ↓
Deduplicate alerts via heartbeat-state.json
  ↓
Emit HEARTBEAT_OK or urgent items
```

### Contract for each skill response (recommended)
Each response should include:
- `source`: `mcp_live | cache | fallback_api`
- `stale`: boolean
- `last_success`: ISO timestamp
- `error`: nullable structured error
- `data`: normalized payload used by heartbeat rules

This keeps triage logic stable while transport changes underneath.

**Confidence: High**

---

## Latency Impact

### Baseline comparison
- State file read: ~**0–1 ms** (effectively near-instant)
- MCP/socket call: ~**50–200 ms** expected envelope (local tests also observed much lower in happy path)

### Practical effect
- Heartbeat likely adds ~100–400 ms total for two live checks in normal conditions.
- This is operationally acceptable for periodic heartbeat cadence.
- Main risk is not raw latency; it is **availability variance** when Tairseach is down.

**Latency risk level: Low-Medium**  
**Confidence: Medium-High**

---

## Tairseach Not Running: Fallback Chain Design

### Required chain
1. **Primary:** MCP live call via Tairseach skill
2. **Secondary:** Read `~/.openclaw/state/*.json` cache snapshot
3. **Tertiary (optional):** direct provider API fallback (if already implemented/approved)
4. **Final safety mode:** return degraded status; suppress false precision; avoid hard-failing heartbeat loop

### Degraded behavior policy
- If calendar live+cache unavailable: do not claim “no upcoming events”; report “calendar status unavailable.”
- If sleep live+cache unavailable: do not infer readiness; report “sleep status unavailable.”
- Preserve ability to raise urgent alerts when any credible signal exists.

**Confidence: High**

---

## Recommended Migration Approach

## Phase 1 (Now): Hybrid Enablement
- Add skill wrappers:
  - `get_sleep_state()` (returns normalized score, wake status, freshness metadata)
  - `get_calendar_window()` (next 48h normalized events + freshness metadata)
- Update heartbeat logic to call wrappers first.
- Keep existing state files populated (compatibility).

## Phase 2: Consumer Migration
- Migrate non-heartbeat consumers (notably `global-entry-monitor.py`) off direct state-file dependence.
- Keep cache write path for rollback confidence.

## Phase 3: Controlled Cutover
- Gate via feature flag: `HEARTBEAT_DATA_SOURCE=live|hybrid|file`.
- Default to `hybrid`; promote to `live` after stability window.

## Phase 4: Decommission
- Remove launchd polling only after:
  - no production consumers require files,
  - degraded handling verified,
  - rollback path documented.

**Overall recommendation:** **Hybrid first, full cutover later.**  
**Confidence: High**

---

## New `HEARTBEAT.md` Draft (Proposed)

```md
# HEARTBEAT.md

You are performing a periodic heartbeat check. Triage what needs attention RIGHT NOW.

## Tool Rules (CRITICAL)

- Prefer skill calls for live data (sleep + calendar).
- Use `read` only for fallback cache/state and dedupe memory.

## Response Protocol

- If NOTHING is urgent -> reply ONLY: 'HEARTBEAT_OK'
- If ANYTHING needs attention -> list ALL urgent items briefly. Do NOT include HEARTBEAT_OK when alerting.
- Always check EVERY category below for urgency.
- If data is unavailable, report degraded status explicitly (do not fabricate normal state).

## Data Sources (priority)

1. Live skill calls:
   - `tairseach.get_sleep_state`
   - `tairseach.get_calendar_window`
2. Fallback cache:
   - `~/.openclaw/state/oura-sleep.json`
   - `~/.openclaw/state/calendar.json`

## Urgency Criteria

### Calendar
- FLAG any event starting within 2 hours
- SKIP events more than 2 hours away, all-day events
- If data unavailable, include: "Calendar status unavailable (degraded)"

### Sleep
- FLAG sleep score below 60 (suggest taking it easy)
- FLAG likely asleep during business hours (internal note; no external alert)
- SKIP normal scores
- If data unavailable, include: "Sleep status unavailable (degraded)"

### General
- Track prior alerts in `~/suibhne/memory/heartbeat-state.json` (no duplicate alerts)
- If late night (11pm–8am) and nothing urgent -> HEARTBEAT_OK
- Greet once per wake cycle if recently woke and not greeted today

## Triage now.
```

---

## Risk Assessment (with Confidence)

1. **Runtime availability regression (High risk)**  
   Live-only heartbeat fails hard if Tairseach/socket unavailable.  
   **Mitigation:** hybrid fallback chain + degraded mode.  
   **Confidence: High**

2. **Consumer breakage (High risk)**  
   Removing state files early breaks existing file-based consumers.  
   **Mitigation:** maintain compatibility cache until all consumers migrated.  
   **Confidence: High**

3. **Latency increase (Low-Medium risk)**  
   Live calls slower than file reads, but still acceptable for heartbeat cadence.  
   **Mitigation:** bounded timeouts, parallel calls, cache fallback.  
   **Confidence: Medium-High**

4. **Schema drift / normalization errors (Medium risk)**  
   Heartbeat rules depend on derived fields currently precomputed by scripts.  
   **Mitigation:** stabilize response contract (`source/stale/last_success/error/data`).  
   **Confidence: Medium-High**

5. **Operational observability gap (Medium risk)**  
   Failures may become silent without explicit degraded signaling.  
   **Mitigation:** include degraded status in heartbeat output and logs.  
   **Confidence: High**

---

## Final Call

Do **not** do a full immediate cutover.

Adopt **hybrid primary-live + cache fallback** now, migrate consumers, then remove polling scripts after a stability window and feature-flagged validation.

**Strategic confidence: High**
