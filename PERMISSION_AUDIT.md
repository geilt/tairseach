# Permission Audit Report
**Date:** 2026-02-14  
**Auditor:** Nechtan (Security/Auth)  
**Scope:** JSON-RPC handler permission mapping

## Summary

✅ **PASS** — Permission gates are correctly configured with one minor enhancement recommended.

## Audit Findings

### 1. Permission Mapping Coverage

All handler methods requiring macOS permissions are correctly mapped:

| Handler | Methods | Required Permission | Status |
|---------|---------|-------------------|--------|
| `contacts.*` | list, search, get, create, update, delete | `contacts` | ✅ Correct |
| `calendar.*` | list, events, getEvent, createEvent, updateEvent, deleteEvent | `calendar` | ✅ Correct |
| `reminders.*` | lists, list, create, complete, delete | `reminders` | ✅ Correct |
| `location.*` | get, watch | `location` | ✅ Correct |
| `photos.*` | albums, list, get, search | `photos` | ✅ Correct |
| `screen.*` | capture, windows | `screen_recording` | ✅ Correct |
| `automation.run` | run | `automation` | ✅ Correct |
| `automation.click/type` | click, type | `accessibility` | ✅ Correct |
| `files.*` | read, write, list | `full_disk_access` | ✅ Correct |
| `camera.*` | list, snap, capture, start, stop | `camera` | ✅ Correct |
| `microphone.*` | record, start, stop | `microphone` | ✅ Correct |

### 2. Methods Correctly Exempted from Permission Gates

These handlers do NOT require macOS permissions and are correctly exempted:

- **auth.*** — Uses socket security + OAuth tokens (no macOS permission needed)
- **permissions.*** — Permission management itself (no permission required)
- **config.*** — Configuration management (no permission required)
- **server.*** — Server control (socket-level security sufficient)
- **gmail.*** — Uses OAuth tokens (Google API, not macOS)
- **gcalendar.*** — Uses OAuth tokens (Google API, not macOS)
- **op.*** / **onepassword.*** — Uses 1Password CLI (no macOS permission)
- **oura.*** — Uses Oura API tokens (no macOS permission)
- **jira.*** — Uses Jira API tokens (no macOS permission)

### 3. Permission Name Validation

All permission names match macOS permission IDs defined in `src-tauri/src/permissions/mod.rs`:

```rust
pub const CONTACTS: &str = "contacts";
pub const AUTOMATION: &str = "automation";
pub const FULL_DISK_ACCESS: &str = "full_disk_access";
pub const ACCESSIBILITY: &str = "accessibility";
pub const SCREEN_RECORDING: &str = "screen_recording";
pub const CALENDAR: &str = "calendar";
pub const REMINDERS: &str = "reminders";
pub const PHOTOS: &str = "photos";
pub const CAMERA: &str = "camera";
pub const MICROPHONE: &str = "microphone";
pub const LOCATION: &str = "location";
```

✅ All mappings use correct permission identifiers.

### 4. Missing Methods Check

Verified that no handler methods are missing from the permission map that should require permissions.

**Gmail Handler Methods:**
- `list_messages`, `get_message`, `send`, `list_labels`, `modify_message`, `trash_message`, `delete_message`
- **Status:** ✅ Correctly exempted (OAuth-based, no macOS permission)

**Google Calendar Handler Methods:**
- `list_calendars`, `list_events`, `get_event`, `create_event`, `update_event`, `delete_event`
- **Status:** ✅ Correctly exempted (OAuth-based, no macOS permission)

**1Password Handler Methods:**
- `items.list`, `items.get`, `vaults.list`
- **Status:** ✅ Correctly exempted (uses `op` CLI, no macOS permission)

**Oura Handler Methods:**
- `sleep`, `readiness`, `activity`, `heartRate`
- **Status:** ✅ Correctly exempted (API token-based, no macOS permission)

**Jira Handler Methods:**
- `issues.search`, `issues.get`, `issues.create`, `issues.update`, `issues.transition`, `projects.list`, `sprints.list`
- **Status:** ✅ Correctly exempted (API token-based, no macOS permission)

### 5. Security Boundary Verification

The permission gate in `HandlerRegistry::handle()` correctly:

1. ✅ Checks permissions BEFORE dispatching to handlers
2. ✅ Returns permission_denied with status information
3. ✅ Falls through to dispatcher only if permission is granted
4. ✅ Exempts methods that don't require macOS permissions (returns `None` from `required_permission()`)

### 6. Enhancement: Improved Error Messages

**IMPLEMENTED:** Enhanced `JsonRpcResponse::permission_denied()` to include remediation guidance:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32001,
    "message": "Permission not granted",
    "data": {
      "permission": "camera",
      "status": "denied",
      "remediation": "User must grant permission manually in System Settings > Privacy & Security > Camera"
    }
  }
}
```

The remediation message now tells the agent:
- For `not_determined`: "Call permissions.request with permission='<name>'"
- For `denied`: "User must grant in System Settings > Privacy & Security > <Pane>"
- For `restricted`: "Permission is restricted by system policy"

## Recommendations

### ✅ Completed
1. **Enhanced permission error responses** — Now include remediation guidance
2. **Verified all permission mappings** — No issues found

### Optional Future Enhancements
1. **Consider adding permission logging** — Log all permission denials for security audit trail
2. **Add permission status to server.status response** — Show which permissions are granted/denied in status check
3. **Add telemetry for permission request success rates** — Track how often users grant permissions after receiving remediation guidance

## Conclusion

The permission gate implementation is **secure and correctly configured**. All methods requiring macOS permissions are properly gated, and all methods that should be exempted (OAuth-based APIs, CLI tools) are correctly allowed through.

The enhanced error messages now provide clear remediation guidance, which will significantly improve the developer experience when debugging permission issues.

---

*Audited by Nechtan — Guardian of the Well*  
*The gates hold. The boundaries are respected.*

🌊
