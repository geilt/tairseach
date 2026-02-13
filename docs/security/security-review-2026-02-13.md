# Security Review — Tairseach Socket + MCP Bridge Trust Boundary

**Date:** 2026-02-13  
**Reviewer:** Nechtan (Security Specialist)  
**Scope:** Socket permissions, credential flow, MCP bridge trust boundary, file operations, 1Password helper  

---

## 1. Socket Permissions Audit

### 1.1 File Permissions — INFO

**Socket:** `~/.tairseach/tairseach.sock`  
**Observed permissions:** `srw-------` (0600), owner `geilt`, group `staff`  
**Parent directory:** `drwx------` (0700)

The socket is owner-only read/write. The parent directory is owner-only. This is correct.

**Status:** ✅ Properly restricted.

### 1.2 Peer UID Verification — INFO

`server.rs` calls `stream.peer_cred()` and verifies `peer_uid == my_uid` via `libc::getuid()`. Connections from non-matching UIDs are rejected with a warning log. If `peer_cred()` fails (returns `None`), the connection is also rejected.

**Status:** ✅ Defense-in-depth beyond file permissions. Good practice.

### 1.3 No Authentication on Socket Connections — LOW

Beyond UID matching, there is no session token, challenge-response, or per-connection authentication. Any process running as the same user can issue arbitrary JSON-RPC commands.

**Risk:** A compromised process running as `geilt` can invoke any handler — read files via FDA, execute AppleScript, access OAuth tokens, read 1Password secrets.

**Mitigation:** This is standard for same-user Unix sockets (cf. Docker socket, SSH agent). The UID check is the authentication. However, given the breadth of capabilities exposed (FDA file access, automation, credential retrieval), this is worth noting.

**Recommendation:** Consider an optional per-session token or manifest-based capability scoping per client connection. LOW priority — the threat model is already "same-user compromise."

---

## 2. Credential Flow Audit

### 2.1 Master Key Derivation — MEDIUM

**Derivation:** HKDF-SHA256 with input = `{hardware_uuid}:{$USER}:{static_salt}`  
**HKDF salt:** `b"tairseach-credential-store-v2"`  
**HKDF info:** `b"master-key"`  
**Static salt:** `"nechtan-guards-the-secrets"` (hardcoded in source)

The key is **deterministic and machine-bound** — derived from the hardware UUID (via `ioreg`) and the username. It is never stored on disk. It is re-derived fresh each process launch.

**Concern:** The key material is entirely derivable by any process on the same machine that can:
1. Run `ioreg` (any user can)
2. Know `$USER` (trivial)
3. Read the source code or binary (to extract the static salt and HKDF parameters)

This means **any local process can derive the master key and decrypt `credentials.enc.json` directly**, bypassing the socket entirely.

**Severity:** MEDIUM. This is a deliberate design choice (no Keychain prompts, no user interaction) but it means the encryption is protection against **offline theft of the file only** (e.g., backup exposure), not against local process compromise.

**Recommendation:** 
- Document this threat model explicitly — the encryption protects against file exfiltration, not local attackers.
- Consider an optional Keychain-stored component to make derivation require Keychain access (with user approval on first use).

### 2.2 Key Storage — INFO

The master key lives in process memory only, wrapped in `Zeroizing<[u8; 32]>` (zeroize crate). It is zeroed on drop. Never written to disk. The hardware UUID is cached in a `OnceLock` but is not secret (publicly queryable).

**Status:** ✅ Good use of zeroize for memory hygiene.

### 2.3 Credential Flow Trace — INFO

```
MCP client → tools/call → ToolRegistry::call_tool()
  → SocketClient::connect() → Unix socket
  → ProxyServer::handle_connection() [UID verified]
  → HandlerRegistry::handle() → specific handler
  → handler calls get_auth_broker() → AuthBroker::get_token()
  → TokenStore::get_token() → decrypt with master key
  → OAuth token returned to handler
  → handler makes API call, returns result via JSON-RPC
```

OAuth tokens (access_token, refresh_token) are decrypted in memory, used for API calls, and the API response (not the token) is returned to the caller.

### 2.4 Credential Leakage Through JSON-RPC Responses — HIGH

The `auth.get` handler (if exposed) and the generic `get_credential` / `get_token` methods return full credential data including `access_token` and `refresh_token`. If these methods are exposed via the socket (and they are — the handler registry maps `auth.*` methods), **any socket client can retrieve raw OAuth tokens and 1Password secrets**.

Combined with finding 1.3 (no per-connection auth) and 2.1 (derivable master key), this means:
- Any same-user process can connect to the socket and call `auth.get` to extract tokens
- Or derive the key independently and decrypt the file

**Recommendation:** 
- Audit which `auth.*` methods are exposed via the socket. Consider restricting `auth.get` to return only metadata, not raw tokens.
- If raw token retrieval is needed, require an additional authorization step.

---

## 3. MCP Bridge Trust Boundary

### 3.1 Allowlist-Based Tool Exposure — INFO

The MCP bridge (`tairseach-mcp`) loads manifests from `~/.tairseach/manifests/` and builds an allowlist. Only tools with `mcp_expose != false` and a valid method mapping in `implementation.methods` are exposed. Tool names are prefixed with `tairseach_`.

**Status:** ✅ Good — not all socket methods are exposed via MCP. Only manifest-declared tools.

### 3.2 MCP Client → Socket Injection — LOW

The MCP bridge receives `tools/call` requests with a tool name and arguments (JSON). It looks up the tool name in the allowlist, maps it to a socket method name, and forwards the arguments verbatim to the socket.

**Can a malicious MCP client inject arbitrary socket commands?**

**No** — the method name is resolved from the allowlist (manifest-defined), not from client input. The client provides `name` (which must match an allowlisted MCP tool name) and `arguments` (JSON params). The method name sent to the socket comes from the manifest's `implementation.methods` map, not from the client.

However, the `arguments` JSON is forwarded **unmodified** to the socket handler. If a handler has parameter injection vulnerabilities (e.g., path traversal in `files.read`), the MCP client can exploit them.

**Status:** The bridge itself is clean. Security depends on handler-level input validation.

### 3.3 Response Sanitization — LOW

Socket responses are serialized to JSON text and wrapped in MCP `ToolContent` objects. The bridge does **not** sanitize or filter response content. If a socket handler returns sensitive data (e.g., credentials, file contents), it flows directly to the MCP client.

**Recommendation:** Consider response filtering for sensitive fields (tokens, secrets) at the bridge level, or ensure handlers never return raw credentials in tool responses.

---

## 4. File Operations Audit

### 4.1 Path Traversal Protection — INFO

`files.rs` implements a `validate_write_path()` with:
- System path deny list (`/System`, `/Library`, `/usr`, `/bin`, etc.)
- Home directory sensitive path deny (`.ssh`, `.gnupg`, `LaunchAgents`, shell rc files)
- Tairseach config directory deny (`~/.tairseach/`)
- Symlink resolution via `canonicalize()`

Reads also block access to `~/.tairseach/` with a message to use `auth.*` methods instead.

**Status:** ✅ Good defense-in-depth. Paths are absolute-only, canonicalized, and deny-listed.

### 4.2 Arbitrary File Read via FDA — MEDIUM

`files.read` can read **any file on the system** that Tairseach has FDA (Full Disk Access) permission for, except `~/.tairseach/`. This includes:
- `~/Library/Mail/` 
- `~/Library/Messages/`
- Other users' files (if FDA grants it)
- Application data

This is **by design** (Tairseach's purpose is to proxy FDA access), but any same-user process can leverage this to read files it couldn't otherwise access.

**Status:** MEDIUM — design-inherent risk. The deny list for reads is minimal (only `~/.tairseach/`).

**Recommendation:** Consider an allowlist approach for read paths, or at minimum expand the deny list to cover particularly sensitive directories (e.g., `~/Library/Keychains/`).

### 4.3 Directory Listing Depth Limit — INFO

Recursive listing is capped at depth 3 and 1000 entries. Reasonable protection against resource exhaustion.

---

## 5. Automation Handler Audit

### 5.1 Arbitrary AppleScript Execution — HIGH

`automation.run` accepts arbitrary AppleScript (or JXA) source code and executes it via `osascript`. There is **no sandboxing, no script allowlist, no content filtering**. Any socket client can:

- Execute arbitrary shell commands via `do shell script`
- Control any application via Apple Events
- Access Keychain items via `security` CLI within AppleScript
- Modify system settings
- Exfiltrate data

**Severity:** HIGH. This is the most powerful capability exposed through the socket. Combined with the lack of per-connection auth (1.3), any same-user process gets full AppleScript execution.

**Recommendation:**
- Consider a script allowlist or template system instead of arbitrary execution
- Add logging/auditing for all automation.run calls (already has `info!` logging — verify it captures the script content or a hash)
- Consider requiring user confirmation for automation commands via the Tauri UI

### 5.2 Arbitrary Click/Type — MEDIUM

`automation.click` and `automation.type` allow synthetic input events. The click handler uses Swift/CoreGraphics. The type handler uses AppleScript `keystroke`.

**Risk:** UI manipulation, credential entry in dialogs, dismissing security prompts.

**Recommendation:** Log all click/type operations with coordinates and target text.

---

## 6. 1Password Go Helper Audit

### 6.1 Token Passed via stdin JSON — MEDIUM

The Go helper reads a single JSON line from stdin containing `method`, `token` (1Password SA token), and `params`. The SA token is passed **in the JSON payload on stdin**.

**Positive:** Not passed via CLI arguments (would be visible in `ps`), not via environment variable.  
**Concern:** The token must originate from somewhere. The calling Rust code must retrieve it from the credential store and pass it. If the Go binary is invoked with the token, any process that can read `/proc/[pid]/fd/0` (on Linux) or similar could intercept it. On macOS, stdin pipe file descriptors are not trivially accessible to other processes.

**Status:** Acceptable for macOS. The token is ephemeral in the pipe.

### 6.2 Method Validation — INFO

The helper validates `method` against a fixed switch statement (`vaults.list`, `items.list`, `items.get`, `items.create`, `secrets.resolve`). Unknown methods are rejected. Parameters are validated (required fields checked).

**Status:** ✅ Clean input handling.

### 6.3 secrets.resolve Returns Raw Secret Values — INFO

The `secrets.resolve` method returns `{"value": secret}` — the actual secret value. This flows back through the Rust handler, through the socket, potentially through the MCP bridge.

**Status:** By design, but part of the credential leakage surface noted in 2.4.

---

## Summary

| # | Finding | Severity | Status |
|---|---------|----------|--------|
| 1.1 | Socket file permissions (0600) | INFO | ✅ Correct |
| 1.2 | Peer UID verification | INFO | ✅ Good |
| 1.3 | No per-connection authentication | LOW | ⚠️ Accept or mitigate |
| 2.1 | Master key derivable by any local process | MEDIUM | ⚠️ Document threat model |
| 2.2 | Key zeroized in memory | INFO | ✅ Good |
| 2.3 | Credential flow trace | INFO | ✅ Clean |
| 2.4 | Raw credentials exposed via auth.* handlers | HIGH | 🔴 Review exposure |
| 3.1 | Manifest-based MCP allowlist | INFO | ✅ Good |
| 3.2 | No method injection via MCP | LOW | ✅ Clean |
| 3.3 | No response sanitization in bridge | LOW | ⚠️ Consider filtering |
| 4.1 | Write path validation | INFO | ✅ Good |
| 4.2 | Arbitrary FDA file read | MEDIUM | ⚠️ Expand deny list |
| 4.3 | Listing depth limit | INFO | ✅ Good |
| 5.1 | Arbitrary AppleScript execution | HIGH | 🔴 Needs mitigation |
| 5.2 | Synthetic click/type | MEDIUM | ⚠️ Add logging |
| 6.1 | SA token via stdin pipe | MEDIUM | ⚠️ Acceptable |
| 6.2 | Method validation in Go helper | INFO | ✅ Clean |
| 6.3 | Raw secret in resolve response | INFO | By design |

### Critical/High Items Requiring Action

1. **HIGH — auth.* credential exposure (2.4):** Audit which auth methods are reachable via socket and MCP. Consider restricting raw token retrieval.
2. **HIGH — arbitrary AppleScript execution (5.1):** Add UI confirmation, script allowlisting, or at minimum comprehensive audit logging.

### Architecture Notes

The overall design is sound for a **same-user, single-machine capability router**. The primary trust boundary is the Unix socket with UID verification, which is appropriate. The main risks stem from the breadth of capabilities available to any authenticated (same-UID) connection, particularly automation and credential retrieval.

---

*The well has rules. These are the cracks I found in the stones.*

🌊
