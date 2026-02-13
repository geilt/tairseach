# 1Password SDK Core Library - Exported Symbols

**Library:** `libop_uniffi_core.dylib` (version 0.4.0)
**Source:** PyPI wheel `onepassword-sdk`
**Platform:** macOS arm64 (aarch64)

## Core FFI Functions

These are the primary functions exposed by the UniFFI interface:

### Client Lifecycle

```c
RustBuffer uniffi_op_uniffi_core_fn_func_init_client(RustBuffer config, RustCallStatus* status);
```
- **Purpose:** Initialize a 1Password SDK client
- **Input:** JSON config with auth token and integration metadata
- **Output:** Client ID as JSON string

```c
RustBuffer uniffi_op_uniffi_core_fn_func_invoke_sync(RustBuffer invocation, RustCallStatus* status);
```
- **Purpose:** Execute SDK operations synchronously
- **Input:** JSON invocation with method name and parameters
- **Output:** JSON response

```c
void uniffi_op_uniffi_core_fn_func_release_client(RustBuffer client_id, RustCallStatus* status);
```
- **Purpose:** Release client resources
- **Input:** Client ID as JSON string

### Buffer Management

```c
RustBuffer ffi_op_uniffi_core_rustbuffer_alloc(i32 size, RustCallStatus* status);
```
- Allocate a RustBuffer of given size

```c
void ffi_op_uniffi_core_rustbuffer_free(RustBuffer buf, RustCallStatus* status);
```
- Free a RustBuffer

```c
RustBuffer ffi_op_uniffi_core_rustbuffer_reserve(RustBuffer buf, i32 additional, RustCallStatus* status);
```
- Reserve additional capacity in a RustBuffer

### Future Handling (Async)

The library also exports async future functions (not used in our sync implementation):

- `ffi_op_uniffi_core_rust_future_poll_*`
- `ffi_op_uniffi_core_rust_future_complete_*`
- `ffi_op_uniffi_core_rust_future_free_*`
- `ffi_op_uniffi_core_rust_future_cancel_*`

## Data Structures

### RustBuffer
```c
struct RustBuffer {
    int32_t capacity;
    int32_t len;
    uint8_t* data;
};
```

### RustCallStatus
```c
struct RustCallStatus {
    int8_t code;         // 0 = success, non-zero = error
    RustBuffer error_buf; // Error message if code != 0
};
```

## JSON Protocols

### init_client Input

```json
{
  "programmingLanguage": "Rust",
  "sdkVersion": "0.4.0",
  "integrationName": "tairseach",
  "integrationVersion": "0.1.0",
  "requestLibraryName": "reqwest",
  "requestLibraryVersion": "0.12",
  "os": "macos",
  "osVersion": "0.0.0",
  "architecture": "aarch64",
  "serviceAccountToken": "ops_..."
}
```

### invoke_sync Input

```json
{
  "invocation": {
    "clientId": "...",
    "parameters": {
      "name": "MethodName",
      "parameters": { /* method-specific params */ }
    }
  }
}
```

## Supported Method Names

Based on the Python SDK analysis:

### Secrets
- `SecretsResolve` - Resolve a single secret reference
- `SecretsResolveAll` - Resolve multiple secret references
- `ValidateSecretReference` - Validate reference syntax
- `GeneratePassword` - Generate a password

### Vaults
- `VaultsList` - List all vaults
- `VaultsGet` - Get vault details
- `VaultsCreate` - Create a vault
- `VaultsUpdate` - Update vault
- `VaultsDelete` - Delete vault

### Items
- `ItemsList` - List items in a vault
- `ItemsGet` - Get a specific item
- `ItemsCreate` - Create an item
- `ItemsUpdate` - Update an item
- `ItemsDelete` - Delete an item
- `ItemsGetAll` - Batch get items
- `ItemsCreateAll` - Batch create items
- `ItemsUpdateAll` - Batch update items
- `ItemsDeleteAll` - Batch delete items

### Groups
- `GroupsList` - List groups
- `GroupsGet` - Get group details
- `GroupsCreate` - Create a group
- `GroupsUpdate` - Update group
- `GroupsDelete` - Delete group

## Security Notes

- All tokens are passed as JSON strings through RustBuffer
- The library uses zeroization for sensitive data in memory
- Service account tokens use the format `ops_<base64-encoded-json>`
- Maximum message size: 50MB (configurable)

## Implementation Status

✅ **Implemented in tairseach:**
- Client initialization
- Synchronous invocation
- Secret resolution
- Vault listing
- Item get/list operations

❌ **Not yet implemented:**
- Async operations (using future functions)
- Batch operations
- Group management
- Vault/item creation/modification

## References

- 1Password SDK Python: https://github.com/1Password/onepassword-sdk-python
- UniFFI Book: https://mozilla.github.io/uniffi-rs/
- PyPI Package: https://pypi.org/project/onepassword-sdk/
