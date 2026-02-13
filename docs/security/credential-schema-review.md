# Credential Schema Review

> **Reviewed by:** Tuan (Darkness) — Schema/data specialist  
> **Date:** 2026-02-13  
> **Files:** `src-tauri/src/auth/credential_types.rs`, `store.rs`, `crypto.rs`

---

## 1. Registered Credential Types

| Type | Fields | Field Types | supports_multiple |
|------|--------|-------------|-------------------|
| `onepassword` | `service_account_token` | Secret | ✅ |
| `jira` | `host`, `email`, `api_token` | String, String, Secret | ✅ |
| `oura` | `access_token` | Secret | ❌ |
| `github` | `access_token` | Secret | ✅ |
| `linear` | `api_key` | Secret | ❌ |
| `notion` | `access_token` | Secret | ✅ |
| `slack` | `token`, `workspace` (optional) | Secret, String | ✅ |
| `godaddy` | `api_key`, `api_secret` | Secret, Secret | ✅ |

**Total:** 8 built-in types.

## 2. Store Capabilities

The `TokenStore` provides two storage paths:

- **OAuth tokens** via `save_token` / `get_token` — stores `TokenRecord` structs (access_token, refresh_token, client_id, etc.)
- **Generic credentials** via `store_credential` / `get_credential` — stores `HashMap<String, String>` field maps

Both paths use the same AES-256-GCM encryption via `encrypt_to_entry()`. All operations (store, get, list, delete) work for both paths.

**Backward compatibility:** `get_credential` tries parsing as `TokenRecord` first, then falls back to `HashMap`. This means OAuth tokens stored via the old path are still readable through the generic interface.

## 3. Encryption Review

| Property | Value |
|----------|-------|
| Algorithm | AES-256-GCM |
| Key size | 256 bits |
| Nonce | 96 bits, random per encryption |
| Key derivation | HKDF-SHA256 from (hardware UUID, username, static salt) |
| Key storage | Derived at runtime, never persisted. Zeroized on drop. |
| File format | Base64-encoded IV, ciphertext, tag stored separately in JSON |
| File permissions | `0600` (credentials), `0644` (schema metadata) |

**All field types (String, Secret) are encrypted identically.** The `FieldType` enum is a UI hint only — at the storage layer, everything is a string in a `HashMap<String, String>`, encrypted as a single JSON blob. This is correct behavior; field-level encryption would add complexity without security benefit since the entire map shares one encryption context.

## 4. Unknown Type Handling

**Graceful.** In `store_credential`:
```rust
if let Some(schema) = self.credential_types.get(cred_type) {
    schema.validate(&fields)?;
}
```

If the type is unknown, validation is **skipped** — the credential is stored without schema checks. This allows:
- Custom types registered at runtime via `register_custom()`
- Forward compatibility if new types are added later
- Credentials stored before their schema was registered

**Risk:** A typo in `cred_type` would store without validation. Acceptable tradeoff for flexibility.

## 5. Migration Path for New Types

Adding a new built-in type requires:
1. Add schema function (e.g., `fn godaddy_schema()`)
2. Register in `CredentialTypeRegistry::new()`
3. `cargo build`

**No migration needed for existing data.** Credentials already stored under an unknown type gain validation retroactively once the schema is registered. The store never rejects reads based on type — only writes are validated.

Custom types can also be registered at runtime via the `auth.credential_types.custom.create` JSON-RPC method.

## 6. Changes Made

- **Added `godaddy` built-in type** with `api_key` (Secret) and `api_secret` (Secret) fields
- No other types were referenced in handlers but missing from the registry

## 7. Observations

- Schema metadata file (`credentials.schema.json`) has `0644` permissions — intentional, contains no secrets (just provider names, timestamps, types)
- The `built_in` field defaults to `false` in schema definitions but is set to `true` by `register_built_in()` — correct behavior
- 1Password resolution fallback chain is well-structured: local → 1Password → error
- V1→V2 migration is feature-gated behind `keychain-migration` — clean separation
