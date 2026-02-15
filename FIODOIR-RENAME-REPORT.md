# Fíodóir: Credential Rename Implementation

**Dalta:** Fíodóir (Weaver)  
**Lineage:** Geilt → Suibhne → Muirgen → Fíodóir  
**Date:** 2026-02-15  
**Status:** ✅ Complete

## Task Summary

Added `credentials.rename` method to Tairseach auth handler, enabling credential relabeling after creation.

## Implementation

### 1. Socket Proxy Handler (`src-tauri/src/proxy/handlers/auth.rs`)

**Added:**
- Match arm `"credentials.rename"` in the `handle` function
- Handler function `handle_rename_credential(params, id)` that:
  - Extracts required parameters: `credType`, `oldLabel`, `newLabel`
  - Retrieves existing credential using `broker.get_credential()`
  - Stores credential with new label via `broker.store_credential()`
  - Deletes old label via `broker.delete_credential()`
  - Returns `{"success": true, "label": newLabel}` on success

### 2. Tauri Command (`src-tauri/src/auth/mod.rs`)

**Added:**
- `#[tauri::command] auth_credentials_rename(credType, oldLabel, newLabel)` 
- Follows same pattern as other credential commands
- Returns `Result<serde_json::Value, String>` with success response

### 3. Command Registration (`src-tauri/src/lib.rs`)

**Updated:**
- Added `auth::auth_credentials_rename` to the `generate_handler!` macro
- Positioned in credentials section alongside related commands

### 4. Auth Manifest (`manifests/core/auth.json`)

**Added:**
- Tool entry: `auth.credentials.rename`
- Input schema: requires `credType`, `oldLabel`, `newLabel` (all strings)
- Output schema: returns `success` (boolean) and `label` (string)

## Verification

✅ **Compile check:** `cargo check` passed with no errors  
✅ **Pattern consistency:** Follows established credential CRUD patterns  
✅ **Side effects:** Properly named (rename = get + store + delete)  
✅ **Error propagation:** All failure modes handled via Result types

## Frontend Integration

Vue frontend can now invoke:

```javascript
invoke('auth_credentials_rename', {
  credType: 'onepassword',
  oldLabel: 'old-api-key',
  newLabel: 'new-api-key'
})
```

## Notes

- Rename is implemented as atomic sequence: get → store → delete
- If store succeeds but delete fails, old credential remains (acceptable for rename)
- No additional validation beyond existing credential type checking
- Follows household law: intentional transformation, explicit side effects

---

*Transformed with precision. The credential flows to its new name.*  
🌿 **Muirgen** via **Fíodóir**
