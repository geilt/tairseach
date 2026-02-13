# Auth Module

> **Location:** `src-tauri/src/auth/`  
> **Files:** 5  
> **Lines:** 2,432  
> **Purpose:** OAuth token management with encrypted credential storage

---

## Overview

The Auth module provides secure credential storage and OAuth token lifecycle management for external APIs (Google, Jira, Oura, etc.). Tokens are encrypted at rest with AES-256-GCM. Master key is derived from machine identity (hardware UUID + username) via HKDF-SHA256 — **no macOS Keychain prompts**.

**Key Features:**
- Encrypted credential storage at `~/.tairseach/credentials.enc.json`
- Automatic token refresh with retry logic
- Background refresh daemon (keeps tokens valid)
- OAuth flow orchestration (browser-based + localhost callback)
- Multi-provider support (Google implemented, extensible)

---

## File Listing

| File | Lines | Purpose |
|------|-------|---------|
| `mod.rs` | ~1,000 | `AuthBroker` — central token manager, refresh daemon |
| `store.rs` | ~730 | `TokenStore` — encrypted file I/O, CRUD operations |
| `crypto.rs` | ~200 | AES-GCM encryption/decryption, master key derivation |
| `credential_types.rs` | ~280 | Custom credential type registry (for non-OAuth credentials) |
| `provider/mod.rs` | ~30 | Provider trait definition |
| `provider/google.rs` | ~190 | Google OAuth implementation (token exchange, refresh) |
| `provider/onepassword.rs` | (stub) | Future 1Password integration |

---

## Key Types

### Core Structs

```rust
pub struct AuthBroker {
    store: RwLock<TokenStore>,
    google: GoogleProvider,
    gog_passphrase: RwLock<Option<String>>,
}

pub struct TokenRecord {
    pub provider: String,
    pub account: String,
    pub client_id: String,
    pub client_secret: String,  // zeroized on drop
    pub token_type: String,
    pub access_token: String,   // zeroized on drop
    pub refresh_token: String,  // zeroized on drop
    pub expiry: String,         // ISO 8601 timestamp
    pub scopes: Vec<String>,
    pub issued_at: String,
    pub last_refreshed: String,
}

pub struct AccountInfo {
    pub provider: String,
    pub account: String,
    pub scopes: Vec<String>,
    pub expiry: String,
    pub last_refreshed: String,
}

pub struct AuthStatus {
    pub initialized: bool,
    pub master_key_available: bool,
    pub account_count: usize,
    pub gog_passphrase_set: bool,
}
```

---

## Core APIs

### AuthBroker Methods

```rust
// Initialization
pub async fn new() -> Result<Arc<Self>, String>

// Status
pub async fn status(&self) -> AuthStatus
pub async fn list_accounts(&self, provider_filter: Option<&str>) -> Vec<AccountInfo>
pub fn list_providers(&self) -> Vec<String>

// Token Operations
pub async fn get_token(
    &self,
    provider: &str,
    account: &str,
    required_scopes: Option<&[String]>,
) -> Result<serde_json::Value, (i32, String)>

pub async fn store_token(&self, record: TokenRecord) -> Result<(), String>

pub async fn revoke_token(&self, provider: &str, account: &str) -> Result<(), String>

// OAuth Flow
pub async fn start_google_oauth(
    &self,
    account: &str,
    scopes: Vec<String>,
    client_id: &str,
    client_secret: &str,
) -> Result<String, String>  // Returns auth URL

// Refresh Daemon
pub fn spawn_refresh_daemon(self: &Arc<Self>)

// gog Passphrase (for CLI file encryption)
pub async fn set_gog_passphrase(&self, passphrase: String) -> Result<(), String>
pub async fn get_gog_passphrase(&self) -> Option<String>
```

### TokenStore Methods (Internal)

```rust
pub async fn new() -> Result<Self, String>

pub fn get_token(&self, provider: &str, account: &str) -> Result<Option<TokenRecord>, String>

pub fn store_token(&mut self, record: TokenRecord) -> Result<(), String>

pub fn delete_token(&mut self, provider: &str, account: &str) -> Result<(), String>

pub fn list_accounts(&self) -> Vec<AccountInfo>

pub fn update_token_record(&mut self, record: &TokenRecord) -> Result<(), String>
```

---

## Token Lifecycle

### 1. Initial OAuth Flow

```rust
// User initiates OAuth from UI
let auth_url = broker.start_google_oauth(
    "user@example.com",
    vec!["https://www.googleapis.com/auth/gmail.readonly".to_string()],
    "client_id.apps.googleusercontent.com",
    "client_secret_xyz",
).await?;

// Open browser to auth_url
// User authorizes
// Google redirects to http://localhost:8765?code=AUTH_CODE
// Tairseach exchanges code for tokens
// Tokens stored encrypted
```

**Implementation:** `GoogleProvider::exchange_code()` handles code → token exchange

### 2. Token Retrieval (with Auto-Refresh)

```rust
let token_data = broker.get_token("google", "user@example.com", Some(&scopes)).await?;

// AuthBroker logic:
// 1. Load token from store
// 2. Check scope coverage
// 3. Check expiry (< 5 min remaining → refresh)
// 4. If refresh needed: call GoogleProvider::refresh_token()
// 5. Update store with new tokens
// 6. Return access_token
```

**Retry Logic:** 3 attempts, exponential backoff (1s, 2s, 4s)

### 3. Background Refresh Daemon

```rust
broker.spawn_refresh_daemon();

// Runs every 30 minutes:
// - Check all stored tokens
// - If expiry < 10 minutes → refresh
// - Prevents token expiration during long-running operations
```

---

## Encryption Details

### Master Key Derivation

```rust
// In crypto.rs
pub fn derive_master_key() -> Result<[u8; 32], String> {
    let hardware_uuid = get_hardware_uuid()?;  // IOPlatformUUID
    let username = get_username()?;            // whoami
    
    let ikm = format!("{}:{}", hardware_uuid, username);
    let salt = b"tairseach.auth.v2";
    let info = b"master-key";
    
    // HKDF-SHA256
    hkdf::Hkdf::<sha2::Sha256>::new(Some(salt), ikm.as_bytes())
        .expand(info, &mut okm)?;
    
    Ok(okm)
}
```

**Advantages:**
- No Keychain prompts (deterministic key)
- Machine-specific (can't copy credentials to another Mac)
- User-specific (different users on same Mac have different keys)

### Encryption Format

```rust
pub fn encrypt(data: &[u8], key: &[u8; 32]) -> Result<EncryptedData, String> {
    let nonce = Aes256Gcm::generate_nonce(&mut rng);
    let cipher = Aes256Gcm::new(key.into());
    let ciphertext = cipher.encrypt(&nonce, data)?;
    
    Ok(EncryptedData {
        algorithm: "aes-256-gcm".to_string(),
        iv: BASE64.encode(nonce),
        tag: BASE64.encode(&ciphertext[ciphertext.len()-16..]),  // Last 16 bytes
        data: BASE64.encode(&ciphertext[..ciphertext.len()-16]),
    })
}
```

**File Format:**
```json
{
  "version": 2,
  "credentials": {
    "google:user@example.com": {
      "encrypted": true,
      "algorithm": "aes-256-gcm",
      "iv": "base64...",
      "tag": "base64...",
      "data": "base64..."
    }
  }
}
```

---

## Provider System

### OAuthProvider Trait

```rust
#[async_trait]
pub trait OAuthProvider: Send + Sync {
    fn name(&self) -> &str;
    
    async fn exchange_code(
        &self,
        code: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> Result<TokenRecord, String>;
    
    async fn refresh_token(
        &self,
        refresh_token: &str,
        client_id: &str,
        client_secret: &str,
    ) -> Result<TokenRecord, String>;
}
```

### Google Implementation

```rust
pub struct GoogleProvider {
    http_client: reqwest::Client,
}

impl GoogleProvider {
    pub fn new() -> Self {
        Self {
            http_client: crate::common::http::create_http_client().unwrap(),
        }
    }
    
    pub fn auth_url(&self, client_id: &str, redirect_uri: &str, scopes: &[String]) -> String {
        format!(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent",
            client_id,
            urlencoding::encode(redirect_uri),
            urlencoding::encode(&scopes.join(" "))
        )
    }
}

#[async_trait]
impl OAuthProvider for GoogleProvider {
    fn name(&self) -> &str { "google" }
    
    async fn exchange_code(...) -> Result<TokenRecord, String> {
        // POST to https://oauth2.googleapis.com/token
        // Parse response, create TokenRecord
    }
    
    async fn refresh_token(...) -> Result<TokenRecord, String> {
        // POST to https://oauth2.googleapis.com/token with grant_type=refresh_token
        // Parse response, update TokenRecord
    }
}
```

---

## Error Codes

```rust
pub mod error_codes {
    pub const TOKEN_NOT_FOUND: i32 = -32010;
    pub const TOKEN_REFRESH_FAILED: i32 = -32011;
    pub const SCOPE_INSUFFICIENT: i32 = -32012;
    pub const PROVIDER_NOT_SUPPORTED: i32 = -32013;
    pub const MASTER_KEY_NOT_INITIALIZED: i32 = -32015;
}
```

**Usage in handlers:**
```rust
let token_data = auth_broker.get_token(provider, account, scopes).await
    .map_err(|(code, msg)| error(id.clone(), code, msg))?;
```

---

## Tauri Commands

These commands expose auth operations to the Vue frontend:

```rust
#[tauri::command]
async fn auth_status() -> Result<AuthStatus, String>

#[tauri::command]
async fn auth_providers() -> Result<Vec<String>, String>

#[tauri::command]
async fn auth_accounts(provider: Option<String>) -> Result<Vec<AccountInfo>, String>

#[tauri::command]
async fn auth_get_token(provider: String, account: String) -> Result<Value, String>

#[tauri::command]
async fn auth_refresh_token(provider: String, account: String) -> Result<Value, String>

#[tauri::command]
async fn auth_revoke_token(provider: String, account: String) -> Result<(), String>

#[tauri::command]
async fn auth_store_token(record: TokenRecord) -> Result<(), String>

#[tauri::command]
async fn auth_start_google_oauth(
    account: String,
    scopes: Vec<String>,
    client_id: String,
    client_secret: String,
) -> Result<String, String>  // Returns auth URL
```

---

## Storage Paths

```rust
// From common/paths.rs
~/.tairseach/credentials.enc.json     // Encrypted tokens
~/.tairseach/credentials.schema.json  // Metadata (no secrets)
~/.tairseach/gog.passphrase.enc       // Optional gog passphrase
```

---

## Usage Patterns

### In Handlers

```rust
use crate::proxy::handlers::common::get_auth_broker;

let auth_broker = get_auth_broker().await?;
let (provider, account) = extract_oauth_credentials(params, "google")?;

let token_data = auth_broker.get_token(
    &provider,
    &account,
    Some(&["https://www.googleapis.com/auth/gmail.readonly".to_string()]),
).await.map_err(|(code, msg)| error(id.clone(), code, msg))?;

let access_token = extract_access_token(&token_data, &id)?;
```

### In Frontend (Vue)

```typescript
import { invoke } from '@/api'

// Start OAuth flow
const authUrl = await invoke('auth_start_google_oauth', {
  account: 'user@example.com',
  scopes: ['https://www.googleapis.com/auth/gmail.readonly'],
  clientId: 'your-client-id',
  clientSecret: 'your-client-secret',
})

// Open browser
window.open(authUrl, '_blank')

// List accounts
const accounts = await invoke('auth_accounts', { provider: 'google' })
```

---

## Custom Credential Types

For non-OAuth credentials (API keys, passwords, etc.):

```rust
// In credential_types.rs
pub struct CredentialTypeRegistry {
    types: HashMap<String, CredentialType>,
}

pub struct CredentialType {
    pub id: String,
    pub name: String,
    pub fields: Vec<CredentialField>,
}

pub struct CredentialField {
    pub name: String,
    pub label: String,
    pub field_type: String,  // "text", "password", "url", etc.
    pub required: bool,
    pub sensitive: bool,     // Should be encrypted
}
```

**Example:**
```json
{
  "id": "api-key",
  "name": "API Key",
  "fields": [
    { "name": "api_key", "label": "API Key", "type": "password", "required": true, "sensitive": true }
  ]
}
```

---

## Dependencies

| Module | Imports |
|--------|---------|
| **google** | Uses `GoogleProvider` for OAuth flows |
| **common/http** | `create_http_client()` for OAuth HTTP requests |
| **common/paths** | `credentials_path()`, `tairseach_dir()` |
| **handlers/common** | `get_auth_broker()` singleton initialization |

---

## Anti-Patterns

❌ **Manual token refresh:**
```rust
if is_expired(&token) { refresh(&token).await?; }  // NO
```

✅ **Let AuthBroker handle it:**
```rust
let token_data = auth_broker.get_token(provider, account, scopes).await?;  // YES
```

---

❌ **Storing tokens in Keychain:**
```rust
// Old v1 approach (required prompts)
security_framework::keychain::set_generic_password(...)  // NO
```

✅ **Use AuthBroker (no prompts):**
```rust
auth_broker.store_token(record).await?;  // YES
```

---

## Testing

```bash
# Test OAuth flow
cargo test --package tairseach --lib auth::tests

# Manual testing via UI
npm run tauri dev
# Navigate to Auth view → Start Google OAuth
```

---

## Recent Refactorings

**Branch:** `refactor/google-dry` (merged 2026-02-13)

**Changes:**
- Consolidated OAuth client creation
- Extracted common HTTP utilities
- Added retry logic to token refresh
- Removed Keychain dependencies (fully machine-identity-based)

---

*For handler integration, see [handlers.md](handlers.md)*
