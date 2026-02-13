# Auth System Architecture

**Component:** Auth Broker & Credential Store  
**Location:** `src-tauri/src/auth/`  
**Encryption:** AES-256-GCM with HKDF-derived master key  
**Storage:** `~/.tairseach/auth/`

---

## Purpose

The auth system manages OAuth tokens and API credentials for external services. It provides:

1. **Secure storage** — Credentials encrypted at rest with AES-256-GCM
2. **Token management** — Automatic refresh of expiring OAuth tokens
3. **Multi-account support** — Multiple accounts per provider (e.g., work + personal Gmail)
4. **Tier 1 proxy mode** — Credentials never leave Tairseach's memory
5. **OAuth provider abstraction** — Pluggable providers (Google, 1Password, etc.)

## Architecture Overview

```
┌──────────────────────────────────────────────────────────────┐
│                        AuthBroker                             │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  Public API                                            │  │
│  │  • get_token(provider, account, scopes)                │  │
│  │  • store_token(provider, account, token_data)          │  │
│  │  • refresh_token(provider, account)                    │  │
│  │  • revoke_token(provider, account)                     │  │
│  │  • list_accounts() → Vec<AccountInfo>                  │  │
│  └─────────────────────┬──────────────────────────────────┘  │
│                        │                                      │
│  ┌─────────────────────▼──────────────────────────────────┐  │
│  │  TokenStore (RwLock)                                    │  │
│  │  • In-memory cache of decrypted tokens                 │  │
│  │  • Load from disk on demand                            │  │
│  │  • Write to disk on update                             │  │
│  └─────────────────────┬──────────────────────────────────┘  │
│                        │                                      │
│           ┌────────────┴────────────┐                         │
│           │                         │                         │
│  ┌────────▼──────────┐   ┌─────────▼──────────┐             │
│  │  OAuth Providers  │   │  Crypto Module     │             │
│  │  • Google         │   │  • HKDF key derive │             │
│  │  • 1Password      │   │  • AES-256-GCM     │             │
│  │  • (future: Slack)│   │  • Zeroize on drop │             │
│  └───────────────────┘   └────────────────────┘             │
└──────────────────────────────────────────────────────────────┘
            │                           │
            │                           │
   ┌────────▼──────────┐    ┌──────────▼────────────┐
   │  External OAuth   │    │  Encrypted Token Files │
   │  (Google, 1Pass)  │    │  ~/.tairseach/auth/    │
   └───────────────────┘    └───────────────────────┘
```

## File Structure

```
~/.tairseach/auth/
├── tokens/
│   ├── google-me.json.enc          # Encrypted token for google:me
│   ├── google-work.json.enc        # Encrypted token for google:work
│   ├── onepassword-default.json.enc
│   └── oura-default.json.enc
├── metadata.json                   # Unencrypted index (provider:account → filename)
└── gog_passphrase.enc              # gog CLI passphrase (legacy)
```

### Metadata File

**File:** `~/.tairseach/auth/metadata.json`

**Format:**

```json
{
  "accounts": [
    {
      "provider": "google",
      "account": "me",
      "file": "google-me.json.enc",
      "scopes": [
        "https://www.googleapis.com/auth/gmail.readonly",
        "https://www.googleapis.com/auth/calendar"
      ],
      "expiry": "2026-02-14T01:23:45Z",
      "last_refreshed": "2026-02-13T01:23:45Z"
    }
  ]
}
```

**Purpose:**
- Quick lookup (don't need to decrypt to list accounts)
- Expiry tracking for refresh daemon
- No secrets stored (safe to log)

### Encrypted Token File

**File:** `~/.tairseach/auth/tokens/google-me.json.enc`

**Format:** Binary (not JSON!)

```
[ 12-byte nonce ][ encrypted payload ][ 16-byte auth tag ]
```

**Decrypted payload:**

```json
{
  "provider": "google",
  "account": "me",
  "client_id": "...",
  "client_secret": "...",
  "token_type": "Bearer",
  "access_token": "ya29...",
  "refresh_token": "1//...",
  "expiry": "2026-02-13T02:23:45Z",
  "scopes": ["..."],
  "issued_at": "2026-02-13T01:23:45Z",
  "last_refreshed": "2026-02-13T01:23:45Z"
}
```

## Key Modules

### `mod.rs` — AuthBroker Core

**Lines:** ~1000  
**Responsibilities:**
- Token lifecycle management
- Provider dispatch
- Refresh daemon
- Public API for handlers

**Key Type:**

```rust
pub struct AuthBroker {
    store: RwLock<TokenStore>,        // Token cache
    google: GoogleProvider,            // OAuth provider for Google
    gog_passphrase: RwLock<Option<String>>, // gog CLI integration
}
```

**Key Methods:**

```rust
pub async fn get_token(
    &self,
    provider: &str,
    account: &str,
    required_scopes: Option<&[String]>,
) -> Result<serde_json::Value, (i32, String)>
```

**Flow:**
1. Check if token exists in store
2. If not, return `TOKEN_NOT_FOUND` error
3. Check if token expired
4. If expired, call `refresh_token()`
5. If refresh fails, return `TOKEN_REFRESH_FAILED`
6. If scopes insufficient, return `SCOPE_INSUFFICIENT`
7. Return token as JSON (access_token, expiry, etc.)

**Refresh Daemon:**

```rust
pub fn spawn_refresh_daemon(self: &Arc<Self>) {
    let broker = Arc::clone(self);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            broker.refresh_expiring_tokens().await;
        }
    });
}
```

**Refresh logic:**
- Runs every 60 seconds
- Finds tokens expiring in next 5 minutes
- Calls `refresh_token()` preemptively
- Logs errors but doesn't crash

### `store.rs` — Token Store

**Lines:** ~400  
**Responsibilities:**
- Disk I/O for encrypted token files
- In-memory cache of decrypted tokens
- Metadata file management

**Key Type:**

```rust
pub struct TokenStore {
    auth_dir: PathBuf,               // ~/.tairseach/auth
    tokens: HashMap<String, TokenRecord>, // In-memory cache
    master_key: Vec<u8>,             // AES-256 key
}
```

**Key:**

```rust
fn token_key(provider: &str, account: &str) -> String {
    format!("{}:{}", provider, account)
}
```

**Storage Methods:**

```rust
pub async fn save_token(&mut self, record: &TokenRecord) -> Result<(), String>
```

**Flow:**
1. Serialize `TokenRecord` to JSON
2. Encrypt with AES-256-GCM (`crypto::encrypt()`)
3. Write to `tokens/{provider}-{account}.json.enc`
4. Update metadata file
5. Update in-memory cache

```rust
pub async fn load_token(
    &mut self,
    provider: &str,
    account: &str,
) -> Result<TokenRecord, String>
```

**Flow:**
1. Check in-memory cache first
2. If not cached, read from disk
3. Decrypt with AES-256-GCM (`crypto::decrypt()`)
4. Deserialize JSON
5. Store in cache
6. Return

### `crypto.rs` — Encryption

**Lines:** ~200  
**Responsibilities:**
- Master key derivation (HKDF)
- AES-256-GCM encryption/decryption
- Zeroize on drop (prevent key leaks)

**Master Key Derivation:**

```rust
pub fn derive_master_key() -> Result<Vec<u8>, String> {
    // 1. Get machine UUID (hardware identifier)
    let uuid = get_machine_uuid()?;
    
    // 2. Get current username
    let username = whoami::username();
    
    // 3. Combine as salt
    let salt = format!("{}:{}", uuid, username);
    
    // 4. Derive key with HKDF-SHA256
    let hkdf = Hkdf::<Sha256>::new(Some(salt.as_bytes()), b"tairseach-auth");
    let mut key = vec![0u8; 32]; // 256 bits
    hkdf.expand(b"master-key-v1", &mut key)
        .map_err(|e| format!("HKDF expand failed: {}", e))?;
    
    Ok(key)
}
```

**Why this approach?**
- No password prompt required
- Key is derived deterministically from machine + user
- If machine/user changes, key changes (tokens become inaccessible)
- No secrets stored on disk

**Security trade-off:**
- ✅ Pro: Zero-friction UX (no password prompts)
- ✅ Pro: Key never persisted (only in memory)
- ⚠️ Con: If attacker gets shell as your user on your machine, key is derivable
- ⚠️ Con: Cannot decrypt tokens on different machine (by design)

**Encryption:**

```rust
pub fn encrypt(plaintext: &[u8], key: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    
    // Generate random nonce (12 bytes)
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    // Encrypt
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| format!("Encryption failed: {}", e))?;
    
    // Prepend nonce to ciphertext
    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&ciphertext);
    
    Ok(result)
}
```

**Decryption:**

```rust
pub fn decrypt(ciphertext: &[u8], key: &[u8]) -> Result<Vec<u8>, String> {
    if ciphertext.len() < 12 {
        return Err("Ciphertext too short".to_string());
    }
    
    // Extract nonce (first 12 bytes)
    let nonce = Nonce::from_slice(&ciphertext[..12]);
    
    // Decrypt remainder
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let plaintext = cipher
        .decrypt(nonce, &ciphertext[12..])
        .map_err(|e| format!("Decryption failed: {}", e))?;
    
    Ok(plaintext)
}
```

### `credential_types.rs` — Credential Type System

**Lines:** ~300  
**Responsibilities:**
- Credential type definitions (OAuth, API key, etc.)
- Custom credential type registry
- Credential store operations

**Built-in Types:**

```rust
pub const BUILTIN_TYPES: [(&str, &str, &str, &[&str]); 5] = [
    ("google-oauth", "OAuth 2.0", "google", &["access_token", "refresh_token", "expiry"]),
    ("onepassword-token", "API Token", "onepassword", &["api_token"]),
    ("oura-token", "API Token", "oura", &["personal_access_token"]),
    ("jira-token", "API Token", "jira", &["api_token", "email"]),
    ("api-key", "Generic API Key", "generic", &["api_key"]),
];
```

**Custom Types:**

Stored in `~/.tairseach/auth/credential_types.json`:

```json
{
  "types": [
    {
      "id": "slack-bot-token",
      "name": "Slack Bot Token",
      "provider": "slack",
      "fields": ["bot_token", "app_token"]
    }
  ]
}
```

**Purpose:**
- UI can dynamically generate forms for credential input
- Manifests can reference credential types
- Extensible without code changes

### `provider/` — OAuth Provider Implementations

#### `provider/google.rs` — Google OAuth

**Lines:** ~250  
**Responsibilities:**
- OAuth 2.0 PKCE flow
- Token refresh
- Scope validation

**Key Methods:**

```rust
impl OAuthProvider for GoogleProvider {
    async fn refresh_token(
        &self,
        refresh_token: &str,
        client_id: &str,
        client_secret: &str,
    ) -> Result<TokenResponse, String> {
        let client = reqwest::Client::new();
        
        let params = [
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];
        
        let response = client
            .post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await?;
        
        let token_resp: TokenResponse = response.json().await?;
        Ok(token_resp)
    }
}
```

**PKCE Flow (for initial auth):**

1. Frontend calls `auth_start_google_oauth()`
2. Backend generates code_verifier (random)
3. Backend computes code_challenge (SHA256 of verifier)
4. Backend returns auth URL with challenge
5. User opens browser, completes OAuth
6. Google redirects to `http://localhost:8080/callback?code=...`
7. Frontend captures code, calls `auth_exchange_code()`
8. Backend exchanges code for tokens (with code_verifier)
9. Backend stores tokens encrypted

**Why PKCE?**
- More secure than traditional OAuth flow
- No client_secret exposed in frontend
- Recommended by Google for native apps

#### `provider/onepassword.rs` — 1Password CLI

**Lines:** ~150  
**Responsibilities:**
- Wrapper around `op` CLI
- Service account token management
- Item CRUD operations

**Pattern:**

```rust
pub async fn get_item(
    token: &str,
    item_id: &str,
    vault: Option<&str>,
) -> Result<serde_json::Value, String> {
    let mut cmd = Command::new("op");
    cmd.arg("item").arg("get").arg(item_id);
    cmd.arg("--format=json");
    
    if let Some(v) = vault {
        cmd.arg("--vault").arg(v);
    }
    
    cmd.env("OP_SERVICE_ACCOUNT_TOKEN", token);
    
    let output = cmd.output().await?;
    
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    
    Ok(serde_json::from_slice(&output.stdout)?)
}
```

**Note:** 1Password integration uses CLI instead of API because:
- CLI handles authentication automatically
- API requires complex session management
- CLI is well-tested and stable

## Token Lifecycle

### 1. Initial Token Storage

**Trigger:** User links account via frontend

**Flow:**

```
User → AuthView.vue → auth_store_token(provider, account, token_data)
                       ↓
                   AuthBroker::store_token()
                       ↓
                   TokenStore::save_token()
                       ↓
                   crypto::encrypt()
                       ↓
                   Write to ~/.tairseach/auth/tokens/
                       ↓
                   Update metadata.json
```

### 2. Token Retrieval

**Trigger:** Handler needs credentials

**Flow:**

```
Handler → AuthBroker::get_token(provider, account, scopes)
              ↓
          TokenStore::load_token() [check cache first]
              ↓
          Check expiry
              ↓ (if expired)
          AuthBroker::refresh_token()
              ↓
          GoogleProvider::refresh_token()
              ↓
          POST https://oauth2.googleapis.com/token
              ↓
          Update TokenRecord with new access_token
              ↓
          TokenStore::save_token()
              ↓
          Return token to handler
```

### 3. Automatic Refresh

**Trigger:** Refresh daemon (every 60s)

**Flow:**

```
Refresh Daemon (background task)
    ↓
TokenStore::list_accounts()
    ↓
For each account:
    ↓
Check expiry (if < 5 minutes remaining)
    ↓
AuthBroker::refresh_token(provider, account)
    ↓
Update TokenRecord
    ↓
TokenStore::save_token()
    ↓
Log success/failure
```

### 4. Token Revocation

**Trigger:** User revokes account via frontend

**Flow:**

```
User → AuthView.vue → auth_revoke_token(provider, account)
                       ↓
                   AuthBroker::revoke_token()
                       ↓
                   (Optional: Call provider's revoke endpoint)
                       ↓
                   TokenStore::delete_token()
                       ↓
                   Remove file from disk
                       ↓
                   Update metadata.json
                       ↓
                   Remove from cache
```

## Error Handling

### Custom Error Codes

Defined in `auth/mod.rs`:

```rust
pub mod error_codes {
    pub const TOKEN_NOT_FOUND: i32 = -32010;
    pub const TOKEN_REFRESH_FAILED: i32 = -32011;
    pub const SCOPE_INSUFFICIENT: i32 = -32012;
    pub const PROVIDER_NOT_SUPPORTED: i32 = -32013;
    pub const MASTER_KEY_NOT_INITIALIZED: i32 = -32015;
}
```

### Common Errors

#### TOKEN_NOT_FOUND (-32010)

**Cause:** No token stored for `provider:account`

**Response:**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32010,
    "message": "Token not found for google:me"
  }
}
```

**Solution:** User must link account via Auth View

#### TOKEN_REFRESH_FAILED (-32011)

**Cause:** OAuth refresh request failed

**Common reasons:**
- Refresh token expired (> 6 months unused for Google)
- Refresh token revoked by user
- OAuth client credentials invalid
- Network error

**Response:**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32011,
    "message": "Failed to refresh token: invalid_grant"
  }
}
```

**Solution:** User must re-authenticate (unlink + relink account)

#### SCOPE_INSUFFICIENT (-32012)

**Cause:** Token has scopes `[A, B]` but handler needs `[A, B, C]`

**Response:**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32012,
    "message": "Token missing required scopes",
    "data": {
      "has": ["gmail.readonly"],
      "needs": ["gmail.readonly", "gmail.send"]
    }
  }
}
```

**Solution:** User must re-authenticate with broader scopes

#### PROVIDER_NOT_SUPPORTED (-32013)

**Cause:** Manifest requires provider that doesn't exist

**Response:**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32013,
    "message": "Provider 'slack' not supported"
  }
}
```

**Solution:** Implement provider in `auth/provider/`

## Security Considerations

### Encryption Strength

**Algorithm:** AES-256-GCM

**Key derivation:** HKDF-SHA256

**Nonce:** Random 12 bytes (96 bits) per encryption

**Authentication:** 16-byte tag (128 bits)

**Security level:** 256-bit security (quantum-resistant in practice)

### Key Storage

**Where:** Memory only (never written to disk)

**Derivation:** On-demand from machine UUID + username

**Cleanup:** Zeroized on drop (via `zeroize` crate)

**Risk:** If attacker has shell access as your user, they can derive the key and decrypt tokens.

**Mitigation:** This is the trade-off for zero-friction UX. Future: Optional password-based key derivation.

### Token Exposure

**Tier 1 (Proxy Mode):**
- Token never leaves AuthBroker memory
- Handler makes HTTP request on behalf of client
- Client receives API response only

**Tier 2 (Pass-Through):**
- Token returned to handler (still within Tairseach process)
- Handler may pass to client (e.g., MCP bridge)
- Client must not log or persist token

**Current:** All integrations use Tier 1 (no Tier 2 implemented yet)

### Refresh Token Security

**Google OAuth:**
- Refresh token valid for 6 months (if unused)
- Refresh token can be revoked by user at accounts.google.com
- Refresh token is single-use (new refresh token issued on each refresh)

**1Password:**
- Service account token has no expiry
- Must be rotated manually if compromised

### Audit Logging

**Not yet implemented.**

**Future:** Log all token access to `~/.tairseach/logs/auth_audit.jsonl`:

```jsonl
{"ts":"2026-02-13T01:23:45Z","action":"get_token","provider":"google","account":"me","by":"handler:gmail","success":true}
{"ts":"2026-02-13T01:24:00Z","action":"refresh_token","provider":"google","account":"me","success":true}
{"ts":"2026-02-13T01:25:00Z","action":"revoke_token","provider":"google","account":"me","by":"user","success":true}
```

## Configuration

### OAuth Client Credentials

**Google OAuth:**

Stored in `~/.openclaw/openclaw.json`:

```json
{
  "google_oauth": {
    "client_id": "123456-abc.apps.googleusercontent.com",
    "client_secret": "GOCSPX-...",
    "redirect_uri": "http://localhost:8080/callback"
  }
}
```

**Managed via:** Config View → Google Settings

**Security:** client_secret is sensitive but not as sensitive as tokens (it's "public secret" for native apps)

### Default Accounts

**Not configurable yet.**

**Future:** Allow user to set default account per provider:

```json
{
  "auth": {
    "defaults": {
      "google": "me",
      "onepassword": "default",
      "oura": "personal"
    }
  }
}
```

**Behavior:** If handler doesn't specify account, use default.

## Testing

### Unit Tests

**crypto.rs:**

```rust
#[test]
fn test_encrypt_decrypt() {
    let key = vec![0u8; 32];
    let plaintext = b"secret data";
    let ciphertext = encrypt(plaintext, &key).unwrap();
    let decrypted = decrypt(&ciphertext, &key).unwrap();
    assert_eq!(plaintext, decrypted.as_slice());
}
```

### Manual Testing

**Store a token:**

```bash
# Via Tauri devtools console
invoke('auth_store_token', {
  provider: 'test',
  account: 'demo',
  tokenData: {
    access_token: 'test_token_123',
    refresh_token: 'refresh_123',
    expiry: '2026-12-31T23:59:59Z',
    scopes: ['test.read', 'test.write']
  }
})
```

**Retrieve a token:**

```bash
invoke('auth_get_token', {
  provider: 'test',
  account: 'demo'
})
```

**Check encryption:**

```bash
# File should be binary, not readable
cat ~/.tairseach/auth/tokens/test-demo.json.enc
# Output: gibberish (encrypted)

# Metadata should list the account
jq '.accounts[] | select(.provider=="test")' ~/.tairseach/auth/metadata.json
```

## Common Issues & Solutions

### Issue: Token refresh fails with invalid_grant

**Cause:** Refresh token expired or revoked

**Solution:**
1. Go to Auth View
2. Click "Unlink" on the account
3. Click "Link Account" and re-authenticate

### Issue: Master key derivation fails

**Error:** "Could not determine machine UUID"

**Cause:** Running on unsupported platform or permissions issue

**Solution:**
1. Check if `/sys/class/dmi/id/product_uuid` exists (Linux)
2. Check if `ioreg -rd1 -c IOPlatformExpertDevice` works (macOS)
3. Fallback: Use username-only derivation (less secure)

### Issue: Metadata file corrupted

**Error:** "Failed to parse metadata.json"

**Solution:**
1. Backup `~/.tairseach/auth/metadata.json`
2. Delete the file
3. Restart Tairseach (will regenerate from encrypted files)

### Issue: Duplicate tokens for same account

**Cause:** Concurrent token storage operations

**Solution:**
1. Check metadata file for duplicates
2. Remove duplicate entries manually
3. Restart Tairseach

## Future Improvements

### Planned (v2)

1. **Audit logging** — Log all token access for security analysis
2. **Scope validation** — Verify token has required scopes before returning
3. **Token rotation** — Auto-rotate long-lived tokens (1Password)
4. **Backup/restore** — Export/import encrypted tokens

### Considered

1. **Hardware security module** — Use macOS Keychain instead of custom encryption
   - **Pro:** OS-managed security
   - **Con:** Keychain prompts annoy users
2. **Password-based encryption** — Optional password for additional security
   - **Pro:** Protects against local shell access
   - **Con:** UX friction (password prompts)
3. **Multi-device sync** — Sync tokens across machines
   - **Pro:** Convenience
   - **Con:** Complex key distribution problem

## Related Documentation

- **[router.md](router.md)** — How capability router loads credentials
- **[google-integration.md](google-integration.md)** — Google OAuth flow details
- **[handlers.md](handlers.md)** — How handlers request credentials

---

*The wind carries secrets. They must be guarded well.*

🌬️ **Senchán Torpéist**
