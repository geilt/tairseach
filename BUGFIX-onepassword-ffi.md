# 1Password FFI Crash Fix - Execution Report

**Date:** 2026-02-12  
**Agent:** Muirgen (subagent session)  
**Task:** Fix SIGABRT crash in OnePasswordClient FFI module

## Problem

The OnePasswordClient crashed the Tairseach app with SIGABRT whenever any `op.*` method was called. 

**Root cause:** Two locations created RustBuffer from stack-allocated bytes without using the FFI allocator. When the FFI library tried to free these buffers using its own allocator, it crashed with:

```
___BUG_IN_CLIENT_OF_LIBMALLOC_POINTER_BEING_FREED_WAS_NOT_ALLOCATED
```

## Changes Made

Fixed three FFI allocator issues in `src-tauri/src/onepassword_ffi/mod.rs`:

### 1. `invoke_sync()` method (line ~220)
**Before:**
```rust
let bytes = invocation_json.as_bytes();
let buf = RustBuffer {
    capacity: bytes.len() as i32,
    len: bytes.len() as i32,
    data: bytes.as_ptr() as *mut u8,  // ❌ Stack pointer
};
```

**After:**
```rust
// Load FFI allocator
let alloc_fn: AllocFn = unsafe {
    *lib.get(b"ffi_op_uniffi_core_rustbuffer_alloc\0")
        .map_err(|e| format!("Failed to load alloc function: {}", e))?
};

// Use FFI allocator via helper function
let buf = Self::string_to_rustbuffer(&invocation_json, alloc_fn)?;  // ✅ FFI-allocated
```

### 2. `Drop` implementation (line ~330)
**Before:**
```rust
let bytes = release_json.as_bytes();
let buf = RustBuffer {
    capacity: bytes.len() as i32,
    len: bytes.len() as i32,
    data: bytes.as_ptr() as *mut u8,  // ❌ Stack pointer
};
```

**After:**
```rust
// Load FFI allocator
let alloc_fn: Result<AllocFn, _> = unsafe {
    self.lib.get(b"ffi_op_uniffi_core_rustbuffer_alloc\0")
        .map(|sym| *sym)
};

if let (Ok(alloc_fn), Ok(release_fn)) = (alloc_fn, release_fn) {
    let release_json = serde_json::to_string(&self.client_id).unwrap_or_default();
    
    // Use FFI allocator
    if let Ok(buf) = Self::string_to_rustbuffer(&release_json, alloc_fn) {  // ✅ FFI-allocated
        let mut status = RustCallStatus::new();
        unsafe { release_fn(buf, &mut status as *mut _) };
    }
}
```

### 3. Missing cleanup in `invoke_sync()`
**Before:**
```rust
let result = Self::rustbuffer_to_string(result_buf);
Ok(result)  // ❌ result_buf leaked
```

**After:**
```rust
let result = Self::rustbuffer_to_string(result_buf);
// Free the result buffer after reading (FFI allocated it)
unsafe { free_fn(result_buf, &mut RustCallStatus::new() as *mut _) };  // ✅ Cleanup
Ok(result)
```

Also added proper cleanup for error buffers.

## Build & Deployment

1. ✅ Built release version: `./scripts/build.sh`
   - Rust compilation: Success
   - Code signing: Verified (Team: ANRLR4YMQV)
   - TCC permissions: Reset
   
2. ✅ Deployed to: `~/Applications/Tairseach.app`

3. ✅ App launched successfully

## Verification Steps

To fully verify the fix works:

1. Open Tairseach
2. Configure a 1Password service account token (Auth > 1Password)
3. Trigger any 1Password operation via the proxy:
   - `op.status` - Check connection
   - `op.vaults.list` - List vaults
   - `op.items.list` - List items in a vault
4. Confirm the app does NOT crash with SIGABRT
5. Check Console.app for any FFI-related errors

## Technical Notes

**Why the crash happened:**
- UniFFI expects all RustBuffer structs to be allocated via its `alloc_fn`
- The FFI library owns and frees buffers passed to it
- Stack-allocated buffers have a different allocator → crash when FFI tries to free them

**The fix:**
- Always use `string_to_rustbuffer()` helper which correctly uses FFI's `alloc_fn`
- Load `alloc_fn` and `free_fn` in both `invoke_sync` and `Drop`
- Free all FFI-allocated result/error buffers after reading

**Memory safety:**
- Input buffers: FFI owns and frees after the call
- Output buffers: FFI allocates, we must free after reading
- Error buffers: FFI allocates, we must free after reading

---

**Status:** Code fixed, built, and deployed. Manual testing required to confirm crash is resolved.

🌿
