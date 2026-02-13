# Security Review: 1Password SDK Integration via `corteq-onepassword`

**WO:** WO-2026-0001-tairseach-dreacht  
**Reviewer:** Nechtan  
**Date:** 2026-02-12  
**Status:** CONDITIONAL APPROVAL — one blocking concern (license)

---

## Executive Summary

The proposed migration from raw HTTP calls to the `corteq-onepassword` crate is a **net security improvement**. The crate wraps 1Password's official native SDK via FFI with proper secret zeroization, and eliminates the current pattern of handling raw API tokens and HTTP auth headers directly. However, the **AGPL-3.0-or-later license is a blocking concern** that must be resolved before merging.

---

## 1. Crate Security Evaluation

### 1.1 Architecture

The crate uses `libloading` to dynamically load `libop_uniffi_core` (the official 1Password SDK Core native library) at runtime. Communication happens via UniFFI-style RustBuffer JSON-RPC. This is the same native library used by 1Password's official Python, Ruby, and Go SDKs.

**Assessment: Sound.** Dynamic loading via `libloading` is the standard Rust approach for FFI. The crate is effectively a thin, typed wrapper around 1Password's own SDK.

### 1.2 Supply Chain

**The native library is sourced from PyPI** (`onepassword-sdk` package). The build script:

1. Fetches wheel metadata from `https://pypi.org/pypi/onepassword-sdk/{version}/json`
2. Downloads the platform-specific wheel from `files.pythonhosted.org`
3. **Verifies SHA256 checksum** against PyPI's reported digest
4. Extracts the `.so`/`.dylib` from the wheel

**Concerns:**
- ⚠️ **PyPI as source for a Rust crate is unusual.** The native library is 1Password's official SDK, but it's being distributed through a Python package ecosystem rather than a dedicated artifact repository. This is a supply chain indirection.
- ✅ **SHA256 verification is present** — checksums are fetched from PyPI's JSON API and verified after download.
- ⚠️ **Checksums are not pinned in source** — they are fetched dynamically from PyPI at build time. A compromised PyPI JSON API response could serve both a malicious binary and its matching checksum. Pinning specific SHA256 values in `build.rs` would be stronger.
- ✅ **SDK version is pinned** (`0.3.2`) — not floating.
- ✅ **Bundled libraries with Git LFS** are available as an alternative to runtime download.

**Recommendation:** For Tairseach builds, **use `ONEPASSWORD_LIB_PATH`** with a locally-verified copy of the native library rather than relying on build-time PyPI download. Pin the SHA256 of the `.dylib` you verify.

### 1.3 Memory Zeroization

**Good practices observed:**
- Token wrapped in `SecretString` (from `secrecy` crate) — zeroized on drop
- `config_json.zeroize()` called after passing to FFI init
- `response_json.zeroize()` called after parsing secret values
- `secret.zeroize()` called after conversion to `SecretString`
- `zeroize = { version = "1.8", features = ["derive"] }` in dependencies

**Concern:**
- ⚠️ In `resolve_secret()`, there's `SecretString::from(secret.clone())` followed by `secret.zeroize()`. The `.clone()` creates a temporary `String` that is immediately consumed by `SecretString::from()`. The original is zeroized. This is acceptable — the clone is moved into SecretString, not leaked.

**Assessment: Adequate.** The crate takes zeroization seriously. The patterns are correct.

### 1.4 FFI Safety

**`unsafe` usage:**
- `unsafe impl Send for NativeLibrary {}` / `unsafe impl Sync for NativeLibrary {}` — justified by comment that SDK handles internal synchronization
- `Library::new()` (from `libloading`) — inherently unsafe, loading native code
- All FFI function calls through typed function pointers — unsafe but properly wrapped

**Concerns:**
- ⚠️ The `Send + Sync` impl trusts that the native library is thread-safe. This is reasonable for 1Password's official SDK but is an assumption, not a guarantee.
- ⚠️ The `SdkClient` wraps calls in `Arc<Mutex<_>>` at the client level, providing serialized access. This is conservative and safe.
- The FFI boundary uses RustBuffer-based passing (UniFFI standard) — well-established pattern.

**Assessment: Acceptable risk.** The unsafe code is limited to FFI loading and type erasure, both following established patterns. The Mutex serialization adds defense in depth.

### 1.5 Crate Maturity

- **Version 0.1.5** — very early. No v1.0 stability guarantees.
- **Single maintainer organization** (Trendium-Labs) — bus factor concern.
- **Low download count** on crates.io — limited community vetting.
- **No security audit** published.

**Assessment: Moderate risk.** The crate is young and unaudited. However, the critical security work (encryption, auth) is delegated to 1Password's own native library. The Rust wrapper is thin enough to audit manually.

---

## 2. Token Handling Review

### 2.1 Current Flow

```
Encrypted store (AES-256-GCM) → decrypt at runtime → pass to SDK → SDK handles 1Password auth
```

**This is secure and preferable to `from_env()`.** Here's why:

- `from_env()` reads `OP_SERVICE_ACCOUNT_TOKEN` from environment — env vars are visible in `/proc/pid/environ`, `ps e`, crash dumps, and logging middleware. They persist for process lifetime.
- The encrypted store approach keeps the token encrypted at rest and decrypts only when needed. The decrypted token is passed to the SDK, which internally manages its own session.

**Recommendation:** Keep `from_token()`. Do NOT switch to `from_env()`.

### 2.2 Token Lifetime

**Cache the SDK client, not the token.** Creating an `OnePassword` client for every request means:
- Repeated FFI init calls
- Repeated 1Password auth handshakes
- Unnecessary latency

The SDK client is `Send + Sync` and designed to be shared via `Arc`. The current `OnceCell<Arc<AuthBroker>>` pattern in `onepassword.rs` already follows this model.

**Recommendation:** Create one `OnePassword` client at startup (or lazily), wrap in `Arc`, share across requests. Destroy and recreate only on token rotation or auth failure.

### 2.3 Existing Code Concern

The current `onepassword.rs` handler stores the decrypted SA token in a plain `String` field of `OnePasswordApi`:

```rust
struct OnePasswordApi {
    token: String,  // ← plaintext token in memory, not zeroized
    ...
}
```

This token lives for the lifetime of the API call. When replaced by the SDK, this goes away — the SDK wraps it in `SecretString`. **This is a security improvement.**

---

## 3. Threat Model: Credential Resolution Chain

### 3.1 Chain

```
Local encrypted store (fast) → 1Password SA vault (network) → not found
```

### 3.2 Local Caching of 1Password Secrets

The `resolve_credential()` method in `store.rs` currently **writes 1Password-resolved secrets to the local encrypted store**:

```rust
// Cache it locally
self.store_credential(provider, account_key, provider, fields.clone(), None)?;
```

**Concerns:**

- ⚠️ **Stale secrets.** Once cached locally, rotated 1Password secrets won't update until the cache is invalidated. There is no TTL mechanism.
- ⚠️ **Expanded attack surface.** Secrets that only existed in 1Password's cloud now also exist on disk (encrypted, but still). Compromise of the master key exposes all cached secrets.
- ⚠️ **Master key derivation is deterministic** from `HW_UUID + $USER + static_salt`. Anyone with access to the machine and knowledge of the derivation scheme can reconstruct the key. The HKDF salt and info strings are hardcoded in source. This is acceptable for a desktop app (the threat model assumes the machine is trusted), but be aware of the boundary.

### 3.3 TTL Recommendations

| Secret Type | Recommended TTL | Rationale |
|---|---|---|
| API keys / tokens | 1 hour | Balance between latency and rotation responsiveness |
| Database credentials | 15 minutes | Higher sensitivity, rotate frequently |
| Encryption keys | No caching | Must always be current |
| Service account tokens | Session lifetime | Only rotate when explicitly changed |

**Recommendation:** Add a TTL field to cached entries and re-resolve from 1Password when expired. The `last_refreshed` field in schema already exists — use it for TTL calculation.

---

## 4. License Compatibility — 🚫 BLOCKING

### 4.1 The Problem

`corteq-onepassword` is licensed **AGPL-3.0-or-later**. Tairseach is **private/proprietary**.

AGPL Section 13 requires that if you run a modified version of an AGPL program as a network service, you must offer the **complete source code** of the entire work to users interacting with it over the network.

### 4.2 Is FFI "Linking"?

**Yes, almost certainly.** The crate is compiled directly into Tairseach's binary. `corteq-onepassword` is a Rust library dependency — it's statically linked. Under both GPL and AGPL, static linking creates a "combined work" that must be distributed under the AGPL.

Even if the native library (`libop_uniffi_core`) is dynamically loaded, the **Rust wrapper crate itself** is compiled into your binary. The AGPL applies to the wrapper code.

### 4.3 Implications

If you ship Tairseach with `corteq-onepassword`:
- **You must release Tairseach's entire source code** under AGPL-compatible terms
- If Tairseach provides any network services (it does — the proxy), AGPL Section 13 extends this to network users
- This is incompatible with keeping Tairseach proprietary

### 4.4 Alternatives

1. **Contact Trendium-Labs for a commercial license exemption.** They may offer dual licensing.
2. **Write your own thin wrapper.** The FFI surface is small (~5 functions). The native library itself (`libop_uniffi_core`) is under 1Password's proprietary license, not AGPL. You only need to replicate the wrapper code (UniFFI RustBuffer marshaling).
3. **Use the 1Password CLI (`op`) as a subprocess.** Avoids linking entirely. Slower, but no license concerns.
4. **Use 1Password's Connect Server** — HTTP API, no linking, self-hosted.

**Recommendation:** Option 2 is most practical. The wrapper is thin (~500 lines of meaningful code). Writing your own avoids AGPL entirely while keeping the same native library and performance characteristics.

---

## 5. Findings Summary

| # | Finding | Severity | Blocking? |
|---|---------|----------|-----------|
| F1 | AGPL-3.0 license incompatible with proprietary Tairseach | **Critical** | **YES** |
| F2 | PyPI SHA256 checksums fetched dynamically, not pinned | Medium | No |
| F3 | No TTL on locally-cached 1Password secrets | Medium | No |
| F4 | Crate is v0.1.5 with no security audit and single maintainer | Low | No |
| F5 | Master key derivable from machine identity + source code knowledge | Low | No (acceptable for desktop) |
| F6 | Current handler stores token as plain String (fixed by migration) | Info | No (resolved by this work) |

---

## 6. Recommendations

1. **🚫 Do not merge the AGPL crate into Tairseach.** Write a minimal equivalent wrapper (~500 LOC) under your own license.
2. **Pin native library SHA256** in the build script or use `ONEPASSWORD_LIB_PATH` with a verified local copy.
3. **Add TTL-based cache invalidation** for 1Password-resolved secrets.
4. **Cache the SDK client** (`Arc<OnePassword>`) rather than creating per-request.
5. **Keep `from_token()`** over `from_env()` — the encrypted store approach is superior.

---

*The well has rules. The AGPL is one of them — respect the license or find another path to the water.*

🌊
