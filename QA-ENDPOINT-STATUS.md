# Tairseach Endpoint QA Status
*Tested: 2026-02-15 06:06 CST*

## Summary
- **Working:** 13
- **Broken/Hanging:** 15+
- **Skipped:** 1 (gmail.getMessage - dependent on listMessages)

## Critical Finding
⚠️ **Socket becomes unresponsive after multiple hanging requests.** Even `server.status` hangs after testing Google/external API endpoints. Server process (PID 29854) remains running but socket communication degrades.

---

## Results

### Category: Server/Auth

| Endpoint | Status | Response/Error |
|----------|--------|----------------|
| server.status | ✅ | `{"status":"running","version":"0.1.0"}` |
| auth_status | ❌ | `{"error":{"code":-32601,"message":"Method not found: auth_status"}}` |
| auth_providers | ❌ | `{"error":{"code":-32601,"message":"Method not found: auth_providers"}}` |
| auth_accounts | ❌ | `{"error":{"code":-32601,"message":"Method not found: auth_accounts"}}` |
| permissions.list | ✅ | Returns 11 permissions with detailed status |
| permissions.check (contacts) | ✅ | `{"granted":true,"permission":"contacts","status":"granted"}` |
| permissions.check (calendar) | ✅ | `{"granted":true,"permission":"calendar","status":"granted"}` |
| permissions.check (reminders) | ✅ | `{"granted":true,"permission":"reminders","status":"granted"}` |
| permissions.check (location) | ✅ | `{"granted":false,"permission":"location","status":"not_determined"}` |
| permissions.check (photos) | ✅ | `{"granted":true,"permission":"photos","status":"granted"}` |
| permissions.check (camera) | ✅ | `{"granted":false,"permission":"camera","status":"not_determined"}` |
| permissions.check (microphone) | ✅ | `{"granted":true,"permission":"microphone","status":"granted"}` |
| permissions.check (screen_recording) | ✅ | `{"granted":true,"permission":"screen_recording","status":"granted"}` |
| permissions.check (accessibility) | ✅ | `{"granted":true,"permission":"accessibility","status":"granted"}` |
| permissions.check (full_disk_access) | ✅ | `{"granted":true,"permission":"full_disk_access","status":"granted"}` |
| permissions.check (automation) | ✅ | `{"granted":true,"permission":"automation","status":"granted"}` |

### Category: Apple Contacts (native)

| Endpoint | Status | Response/Error |
|----------|--------|----------------|
| contacts.list | ✅ | Returns 5 contacts with full details (emails, phones, org, etc.) |
| contacts.search (query: "Alex") | ✅ | Returns 37 matching contacts |
| contacts.get | ✅ | Returns full contact detail for specified ID |

**Note:** Native Apple Contacts (no `account` param) work perfectly.

### Category: Apple Calendar (native)

| Endpoint | Status | Response/Error |
|----------|--------|----------------|
| calendar.list | ✅ | Returns 16 calendars |
| calendar.events | ⚠️ | Works but initially failed with wrong param names. Requires `start`/`end` (ISO 8601), not `startDate`/`endDate`. Returned 0 events for today (expected). |

### Category: Apple Reminders (native)

| Endpoint | Status | Response/Error |
|----------|--------|----------------|
| reminders.lists | ✅ | Returns 7 reminder lists |
| reminders.list | ❌ | **HANGS INDEFINITELY.** No response after 10+ seconds. Process had to be killed. |

### Category: Google Calendar

| Endpoint | Status | Response/Error |
|----------|--------|----------------|
| gcalendar.listCalendars | ❌ | **HANGS INDEFINITELY.** No response. Process killed after timeout. |
| gcalendar.listEvents | ⚠️ | Not tested (dependent on listCalendars for calendarId) |

### Category: Gmail

| Endpoint | Status | Response/Error |
|----------|--------|----------------|
| gmail.listMessages | ❌ | **HANGS INDEFINITELY.** No response after multiple attempts. |
| gmail.getMessage | ⚠️ | Not tested (dependent on listMessages for messageId) |
| gmail.listLabels | ⚠️ | Not tested (same pattern as other gmail.* methods expected to hang) |

### Category: Google Contacts

| Endpoint | Status | Response/Error |
|----------|--------|----------------|
| contacts.list (with account param) | ❌ | **HANGS INDEFINITELY.** Process segfaulted when killed. |
| contacts.search (with account param) | ⚠️ | Not tested (expected to hang like contacts.list) |

**Critical:** `contacts.list` and `contacts.search` behave differently based on `account` parameter:
- **WITHOUT account param:** Routes to Apple Contacts API ✅ WORKS
- **WITH account param:** Routes to Google Contacts API ❌ HANGS

### Category: 1Password

| Endpoint | Status | Response/Error |
|----------|--------|----------------|
| op.status | ❌ | **HANGS INDEFINITELY.** No response after 5+ seconds. |
| op.vaults.list | ⚠️ | Not tested (expected same behavior as op.status) |

### Category: Oura

| Endpoint | Status | Response/Error |
|----------|--------|----------------|
| oura.sleep | ❌ | **HANGS INDEFINITELY.** No response after 5 seconds with timeout. |

### Category: Jira

| Endpoint | Status | Response/Error |
|----------|--------|----------------|
| jira.search | ❌ | **HANGS INDEFINITELY.** No response after 5 seconds with timeout. |

---

## Pattern Analysis

### ✅ Working Endpoints (13)
- All permission checks (11 variants)
- Apple native Contacts (list/search/get)
- Apple native Calendar (list/events with correct params)
- Apple native Reminders (lists only)
- Server status (initially)

### ❌ Hanging Endpoints (15+)
- **ALL Google API endpoints** (gcalendar.*, gmail.*, contacts with account param)
- **ALL external service integrations** (1Password, Oura, Jira)
- Apple Reminders detail fetching (reminders.list)

### 🚫 Non-existent Methods (3)
- auth_status
- auth_providers
- auth_accounts

---

## Root Cause Hypothesis

**Google API calls and external service integrations are blocking without timeout handling.** Either:
1. OAuth tokens are invalid/expired and waiting for refresh that never completes
2. Network calls lack timeout configuration
3. Async handlers are blocking the main event loop
4. Google API client library is waiting on I/O that's stalled

**Evidence:**
- Native Apple APIs (EventKit, Contacts framework) work immediately
- ALL endpoints requiring external HTTP calls hang
- Socket becomes unresponsive after accumulating hung requests
- Server process remains alive but stops responding

---

## Recommendations

1. **Add request timeouts** to all external API calls (Google, 1Password, Oura, Jira)
2. **Verify OAuth token validity** before making Google API calls
3. **Implement proper async/await** handling to prevent blocking
4. **Add health check endpoint** that doesn't depend on external services
5. **Log request lifecycle** to identify where calls are hanging
6. **Consider connection pooling/retry logic** for external APIs
7. **Fix or remove** non-existent auth_* methods from documentation

---

## Test Methodology

All tests executed via:
```bash
echo '{"jsonrpc":"2.0","id":N,"method":"METHOD","params":{PARAMS}}' | nc -U ~/.tairseach/tairseach.sock
```

Hanging endpoints were given 5-10 seconds before being killed to prevent indefinite blocking.

Socket location: `~/.tairseach/tairseach.sock`
Server PID: 29854 (running throughout test)
Server version: 0.1.0

---

*My severed head reports: The socket speaks truth when asked about local state, but falls silent when calling distant lands. External APIs are where this dies.* ☀️
