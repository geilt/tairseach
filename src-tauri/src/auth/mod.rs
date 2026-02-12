//! Auth Broker
//!
//! Manages OAuth tokens for CLIs and agents via the Tairseach socket proxy.
//! Tokens are encrypted at rest with AES-256-GCM. The master encryption key is
//! derived from machine identity (hardware UUID + username) via HKDF-SHA256 —
//! no Keychain prompts required.
//!
//! See ADR-001 for the full design rationale.

pub mod crypto;
pub mod provider;
pub mod store;

use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use zeroize::{Zeroize, ZeroizeOnDrop};

use self::provider::google::GoogleProvider;
use self::provider::OAuthProvider;
use self::store::TokenStore;

// ── Public types ────────────────────────────────────────────────────────────

/// Stored token record (decrypted form)
#[derive(Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct TokenRecord {
    #[zeroize(skip)]
    pub provider: String,
    #[zeroize(skip)]
    pub account: String,
    #[serde(default)]
    #[zeroize(skip)]
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
    #[zeroize(skip)]
    pub token_type: String,
    pub access_token: String,
    pub refresh_token: String,
    #[zeroize(skip)]
    pub expiry: String,
    #[zeroize(skip)]
    pub scopes: Vec<String>,
    #[serde(default)]
    #[zeroize(skip)]
    pub issued_at: String,
    #[serde(default)]
    #[zeroize(skip)]
    pub last_refreshed: String,
}

// Custom Debug implementation that redacts sensitive fields
impl fmt::Debug for TokenRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TokenRecord")
            .field("provider", &self.provider)
            .field("account", &self.account)
            .field("client_id", &self.client_id)
            .field("client_secret", &"[REDACTED]")
            .field("token_type", &self.token_type)
            .field("access_token", &"[REDACTED]")
            .field("refresh_token", &"[REDACTED]")
            .field("expiry", &self.expiry)
            .field("scopes", &self.scopes)
            .field("issued_at", &self.issued_at)
            .field("last_refreshed", &self.last_refreshed)
            .finish()
    }
}

/// Lightweight account info (no secrets)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub provider: String,
    pub account: String,
    pub scopes: Vec<String>,
    pub expiry: String,
    #[serde(default)]
    pub last_refreshed: String,
}

impl From<&TokenRecord> for AccountInfo {
    fn from(t: &TokenRecord) -> Self {
        Self {
            provider: t.provider.clone(),
            account: t.account.clone(),
            scopes: t.scopes.clone(),
            expiry: t.expiry.clone(),
            last_refreshed: t.last_refreshed.clone(),
        }
    }
}

/// Auth subsystem status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStatus {
    pub initialized: bool,
    pub master_key_available: bool,
    pub account_count: usize,
    pub gog_passphrase_set: bool,
}

/// Auth broker error codes (JSON-RPC custom range)
pub mod error_codes {
    pub const TOKEN_NOT_FOUND: i32 = -32010;
    pub const TOKEN_REFRESH_FAILED: i32 = -32011;
    pub const SCOPE_INSUFFICIENT: i32 = -32012;
    pub const PROVIDER_NOT_SUPPORTED: i32 = -32013;
    #[allow(dead_code)]
    pub const AUTH_FLOW_IN_PROGRESS: i32 = -32014;
    pub const MASTER_KEY_NOT_INITIALIZED: i32 = -32015;
}

// ── Auth Broker ─────────────────────────────────────────────────────────────

/// The central auth broker instance, shared across the application.
pub struct AuthBroker {
    store: RwLock<TokenStore>,
    google: GoogleProvider,
    /// gog file-backend passphrase (generated once, stored encrypted)
    gog_passphrase: RwLock<Option<String>>,
}

impl AuthBroker {
    /// Create and initialize the auth broker.
    pub async fn new() -> Result<Arc<Self>, String> {
        let store = TokenStore::new().await?;

        // Load gog passphrase if it exists
        let gog_passphrase = store.load_gog_passphrase().ok().flatten();

        let broker = Arc::new(Self {
            store: RwLock::new(store),
            google: GoogleProvider::new(),
            gog_passphrase: RwLock::new(gog_passphrase),
        });

        info!("Auth broker initialized");
        Ok(broker)
    }

    /// Get subsystem status.
    pub async fn status(&self) -> AuthStatus {
        let store = self.store.read().await;
        let accounts = store.list_accounts();
        let gog = self.gog_passphrase.read().await;
        AuthStatus {
            initialized: true,
            master_key_available: true, // if we got this far, we have it
            account_count: accounts.len(),
            gog_passphrase_set: gog.is_some(),
        }
    }

    /// List all accounts (no secrets).
    pub async fn list_accounts(&self, provider_filter: Option<&str>) -> Vec<AccountInfo> {
        let store = self.store.read().await;
        let mut accounts = store.list_accounts();
        if let Some(p) = provider_filter {
            accounts.retain(|a| a.provider == p);
        }
        accounts
    }

    /// List supported providers.
    pub fn list_providers(&self) -> Vec<String> {
        vec!["google".to_string()]
    }

    /// Retrieve a valid access token, refreshing if necessary.
    pub async fn get_token(
        &self,
        provider: &str,
        account: &str,
        required_scopes: Option<&[String]>,
    ) -> Result<serde_json::Value, (i32, String)> {
        if provider != "google" {
            return Err((
                error_codes::PROVIDER_NOT_SUPPORTED,
                format!("Unsupported provider: {}", provider),
            ));
        }

        let store = self.store.read().await;
        let mut record = store
            .get_token(provider, account)
            .map_err(|e| (error_codes::TOKEN_NOT_FOUND, e))?
            .ok_or_else(|| {
                (
                    error_codes::TOKEN_NOT_FOUND,
                    format!("No token for {}:{}", provider, account),
                )
            })?;
        drop(store);

        // Check scope coverage
        if let Some(required) = required_scopes {
            for s in required {
                if !record.scopes.iter().any(|existing| existing == s) {
                    return Err((
                        error_codes::SCOPE_INSUFFICIENT,
                        format!("Token missing scope: {}", s),
                    ));
                }
            }
        }

        // Refresh if expired or expiring within 60 seconds
        if is_token_expiring(&record.expiry, 60) {
            match self.refresh_token_internal(&mut record).await {
                Ok(()) => {
                    let mut store = self.store.write().await;
                    store.save_token(&record).map_err(|e| {
                        (error_codes::TOKEN_REFRESH_FAILED, e)
                    })?;
                }
                Err(e) => {
                    // If the token hasn't fully expired yet, return it anyway with a warning
                    if !is_token_expiring(&record.expiry, 0) {
                        warn!("Token refresh failed but token not yet expired: {}", e);
                    } else {
                        return Err((error_codes::TOKEN_REFRESH_FAILED, e));
                    }
                }
            }
        }

        Ok(serde_json::json!({
            "access_token": record.access_token,
            "token_type": record.token_type,
            "expiry": record.expiry,
        }))
    }

    /// Force-refresh a token.
    pub async fn force_refresh(
        &self,
        provider: &str,
        account: &str,
    ) -> Result<serde_json::Value, (i32, String)> {
        if provider != "google" {
            return Err((
                error_codes::PROVIDER_NOT_SUPPORTED,
                format!("Unsupported provider: {}", provider),
            ));
        }

        let store = self.store.read().await;
        let mut record = store
            .get_token(provider, account)
            .map_err(|e| (error_codes::TOKEN_NOT_FOUND, e))?
            .ok_or_else(|| {
                (
                    error_codes::TOKEN_NOT_FOUND,
                    format!("No token for {}:{}", provider, account),
                )
            })?;
        drop(store);

        self.refresh_token_internal(&mut record)
            .await
            .map_err(|e| (error_codes::TOKEN_REFRESH_FAILED, e))?;

        let mut store = self.store.write().await;
        store
            .save_token(&record)
            .map_err(|e| (error_codes::TOKEN_REFRESH_FAILED, e))?;

        Ok(serde_json::json!({
            "access_token": record.access_token,
            "token_type": record.token_type,
            "expiry": record.expiry,
        }))
    }

    /// Revoke and remove a token.
    pub async fn revoke_token(
        &self,
        provider: &str,
        account: &str,
    ) -> Result<(), (i32, String)> {
        if provider != "google" {
            return Err((
                error_codes::PROVIDER_NOT_SUPPORTED,
                format!("Unsupported provider: {}", provider),
            ));
        }

        let store = self.store.read().await;
        let record = store
            .get_token(provider, account)
            .map_err(|e| (error_codes::TOKEN_NOT_FOUND, e))?;
        drop(store);

        // Try to revoke at provider (best-effort)
        if let Some(rec) = &record {
            if let Err(e) = self.google.revoke_token(&rec.access_token).await {
                warn!("Provider-side revocation failed (continuing): {}", e);
            }
        }

        let mut store = self.store.write().await;
        store
            .delete_token(provider, account)
            .map_err(|e| (error_codes::TOKEN_NOT_FOUND, e))?;

        info!("Revoked token for {}:{}", provider, account);
        Ok(())
    }

    /// Store/import a token directly.
    pub async fn store_token(&self, record: TokenRecord) -> Result<(), (i32, String)> {
        let mut store = self.store.write().await;
        store
            .save_token(&record)
            .map_err(|e| (error_codes::MASTER_KEY_NOT_INITIALIZED, e))?;
        info!(
            "Stored token for {}:{} ({} scopes)",
            record.provider,
            record.account,
            record.scopes.len()
        );
        Ok(())
    }

    /// Get or generate the gog file-keyring passphrase.
    pub async fn get_gog_passphrase(&self) -> Result<String, (i32, String)> {
        // Check cached value first
        {
            let cached = self.gog_passphrase.read().await;
            if let Some(p) = cached.as_ref() {
                return Ok(p.clone());
            }
        }

        // Generate a new one
        let passphrase = crypto::generate_passphrase();

        // Persist it (requires write lock on store)
        {
            let mut store = self.store.write().await;
            store
                .save_gog_passphrase(&passphrase)
                .map_err(|e| (error_codes::MASTER_KEY_NOT_INITIALIZED, e))?;
        }

        // Cache it
        {
            let mut cached = self.gog_passphrase.write().await;
            *cached = Some(passphrase.clone());
        }

        info!("Generated and stored new gog passphrase");
        Ok(passphrase)
    }

    /// Start the background token refresh daemon.
    pub fn spawn_refresh_daemon(self: &Arc<Self>) {
        let broker = Arc::clone(self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                broker.refresh_expiring_tokens().await;
            }
        });
        info!("Background token refresh daemon started");
    }

    // ── Internal ────────────────────────────────────────────────────────────

    async fn refresh_token_internal(&self, record: &mut TokenRecord) -> Result<(), String> {
        let tokens = self
            .google
            .refresh_token(
                &record.client_id,
                &record.client_secret,
                &record.refresh_token,
            )
            .await?;

        record.access_token = tokens.access_token;
        if let Some(rt) = tokens.refresh_token {
            record.refresh_token = rt;
        }
        record.expiry = tokens.expiry;
        record.last_refreshed = chrono::Utc::now().to_rfc3339();

        Ok(())
    }

    async fn refresh_expiring_tokens(&self) {
        let store = self.store.read().await;
        let accounts = store.list_accounts();
        drop(store);

        for acct in accounts {
            if is_token_expiring(&acct.expiry, 300) {
                // Expiring within 5 minutes
                info!(
                    "Proactively refreshing token for {}:{}",
                    acct.provider, acct.account
                );

                let store = self.store.read().await;
                let record = match store.get_token(&acct.provider, &acct.account) {
                    Ok(Some(r)) => r,
                    _ => continue,
                };
                drop(store);

                let mut record = record;
                match self.refresh_token_internal(&mut record).await {
                    Ok(()) => {
                        let mut store = self.store.write().await;
                        if let Err(e) = store.save_token(&record) {
                            error!("Failed to save refreshed token: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Background refresh failed for {}:{}: {}",
                            acct.provider, acct.account, e
                        );
                    }
                }
            }
        }
    }
}

// ── Global AuthBroker Instance ──────────────────────────────────────────────

use once_cell::sync::OnceCell;

static AUTH_BROKER_INSTANCE: OnceCell<Arc<AuthBroker>> = OnceCell::new();

/// Get or initialize the global AuthBroker instance.
/// This is shared across both Tauri commands and socket proxy handlers.
pub async fn get_or_init_broker() -> Result<Arc<AuthBroker>, String> {
    if let Some(broker) = AUTH_BROKER_INSTANCE.get() {
        return Ok(Arc::clone(broker));
    }
    
    let broker = AuthBroker::new().await?;
    broker.spawn_refresh_daemon();
    
    match AUTH_BROKER_INSTANCE.set(Arc::clone(&broker)) {
        Ok(_) => Ok(broker),
        Err(_) => {
            // Someone else set it first, use theirs
            Ok(Arc::clone(AUTH_BROKER_INSTANCE.get().unwrap()))
        }
    }
}

// ── Tauri commands ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthState {
    pub authenticated: bool,
    pub method: String,
    pub last_auth: Option<String>,
}

#[tauri::command]
pub async fn authenticate(
    method: String,
    _credential: Option<String>,
) -> Result<AuthState, String> {
    Ok(AuthState {
        authenticated: true,
        method,
        last_auth: None,
    })
}

#[tauri::command]
pub async fn check_auth() -> Result<AuthState, String> {
    Ok(AuthState {
        authenticated: false,
        method: "none".to_string(),
        last_auth: None,
    })
}

#[tauri::command]
pub async fn auth_status() -> Result<AuthStatus, String> {
    let broker = get_or_init_broker().await?;
    Ok(broker.status().await)
}

#[tauri::command]
pub async fn auth_providers() -> Result<Vec<String>, String> {
    let broker = get_or_init_broker().await?;
    Ok(broker.list_providers())
}

#[tauri::command]
pub async fn auth_accounts(provider: Option<String>) -> Result<Vec<AccountInfo>, String> {
    let broker = get_or_init_broker().await?;
    Ok(broker.list_accounts(provider.as_deref()).await)
}

#[tauri::command]
pub async fn auth_get_token(
    provider: String,
    account: String,
    scopes: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    let broker = get_or_init_broker().await?;
    broker
        .get_token(&provider, &account, scopes.as_deref())
        .await
        .map_err(|(_, msg)| msg)
}

#[tauri::command]
pub async fn auth_refresh_token(provider: String, account: String) -> Result<serde_json::Value, String> {
    let broker = get_or_init_broker().await?;
    broker
        .force_refresh(&provider, &account)
        .await
        .map_err(|(_, msg)| msg)
}

#[tauri::command]
pub async fn auth_revoke_token(provider: String, account: String) -> Result<(), String> {
    let broker = get_or_init_broker().await?;
    broker
        .revoke_token(&provider, &account)
        .await
        .map_err(|(_, msg)| msg)
}

#[tauri::command]
pub async fn auth_store_token(record: TokenRecord) -> Result<(), String> {
    let broker = get_or_init_broker().await?;
    broker
        .store_token(record)
        .await
        .map_err(|(_, msg)| msg)
}

#[tauri::command]
pub async fn auth_start_google_oauth(_app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    use provider::google::{generate_code_verifier, generate_code_challenge, GoogleProvider};
    use provider::OAuthProvider;
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::net::TcpListener;
    use tokio::time::{timeout, Duration};
    
    // 1. Generate PKCE pair
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);
    
    // 2. Start local HTTP server on random available port
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("Failed to bind local server: {}", e))?;
    
    let local_addr = listener.local_addr()
        .map_err(|e| format!("Failed to get local address: {}", e))?;
    
    let redirect_uri = format!("http://127.0.0.1:{}", local_addr.port());
    info!("OAuth callback server listening on {}", redirect_uri);
    
    // 3. Build authorize URL with specified scopes
    let scopes = vec![
        "https://mail.google.com/".to_string(),
        "https://www.googleapis.com/auth/calendar".to_string(),
        "https://www.googleapis.com/auth/contacts".to_string(),
        "https://www.googleapis.com/auth/drive".to_string(),
        "openid".to_string(),
        "email".to_string(),
        "profile".to_string(),
    ];
    
    let state = generate_state();
    
    // Load credentials from saved Google OAuth config, fall back to defaults
    let provider = match crate::config::get_google_oauth_config().await {
        Ok(Some(config)) if !config.client_id.is_empty() => {
            info!("Using saved Google OAuth credentials");
            GoogleProvider::with_credentials(config.client_id, config.client_secret)
        }
        _ => {
            warn!("No saved Google OAuth credentials found, using defaults");
            GoogleProvider::new()
        }
    };
    let auth_url = provider.authorize_url(&scopes, &state, &code_challenge, &redirect_uri);
    
    info!("Opening browser for OAuth authorization");
    
    // 4. Open the URL in default browser
    if let Err(e) = open::that(&auth_url) {
        warn!("Failed to open browser automatically: {}. URL: {}", e, auth_url);
        return Err(format!(
            "Could not open browser. Please manually visit: {}",
            auth_url
        ));
    }
    
    // 5. Wait for callback (with timeout)
    let callback_result = timeout(Duration::from_secs(120), async {
        loop {
            let (mut socket, _) = listener.accept().await?;
            
            let mut reader = BufReader::new(&mut socket);
            let mut request_line = String::new();
            reader.read_line(&mut request_line).await?;
            
            // Parse the request line: "GET /path?query HTTP/1.1"
            let parts: Vec<&str> = request_line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }
            
            let path_and_query = parts[1];
            let query = if let Some(idx) = path_and_query.find('?') {
                &path_and_query[idx + 1..]
            } else {
                ""
            };
            
            // Parse query parameters
            let params = parse_query_params(query);
            
            // Check if this is our OAuth callback
            if let (Some(received_code), Some(received_state)) =
                (params.get("code"), params.get("state"))
            {
                // 6. Validate state
                if received_state != &state {
                    let error_html = success_html("Error: Invalid state parameter. Please try again.");
                    send_response(&mut socket, "400 Bad Request", error_html).await?;
                    return Err::<(String, String), std::io::Error>(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "State mismatch",
                    ));
                }
                
                // Send success response to browser
                let success_html = success_html("Authentication successful! You can close this tab.");
                send_response(&mut socket, "200 OK", success_html).await?;
                
                return Ok((received_code.clone(), code_verifier.clone()));
            } else if let Some(error) = params.get("error") {
                let error_desc = params
                    .get("error_description")
                    .map(|s| s.as_str())
                    .unwrap_or("Unknown error");
                let error_html = success_html(&format!("Error: {} - {}", error, error_desc));
                send_response(&mut socket, "400 Bad Request", error_html).await?;
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("{}: {}", error, error_desc),
                ));
            }
        }
    })
    .await
    .map_err(|_| "OAuth flow timed out after 120 seconds".to_string())?
    .map_err(|e| format!("Callback server error: {}", e))?;
    
    let (code, verifier) = callback_result;
    
    // 7. Exchange code for tokens
    info!("Exchanging authorization code for tokens");
    let tokens = provider
        .exchange_code(&code, &verifier, &redirect_uri)
        .await?;
    
    // 8. Fetch user email from Google userinfo endpoint
    let email = fetch_google_email(&tokens.access_token).await?;
    
    // 9. Store token in broker
    let record = TokenRecord {
        provider: "google".to_string(),
        account: email.clone(),
        client_id: provider.client_id.clone(),
        client_secret: provider.client_secret.clone(),
        token_type: tokens.token_type,
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token.unwrap_or_default(),
        expiry: tokens.expiry,
        scopes: tokens.scopes,
        issued_at: chrono::Utc::now().to_rfc3339(),
        last_refreshed: String::new(),
    };
    
    let broker = get_or_init_broker().await?;
    broker.store_token(record).await.map_err(|(_, msg)| msg)?;
    
    info!("Successfully completed OAuth flow for {}", email);
    
    // 10. Return success with account email
    Ok(serde_json::json!({
        "success": true,
        "email": email,
        "message": "Authentication successful"
    }))
}

// ── OAuth Flow Helpers ──────────────────────────────────────────────────────

/// Generate a random state string for CSRF protection
fn generate_state() -> String {
    use rand::Rng;
    let bytes: [u8; 16] = rand::rngs::OsRng.gen();
    hex::encode(bytes)
}

/// Parse URL query parameters into a HashMap
fn parse_query_params(query: &str) -> std::collections::HashMap<String, String> {
    query
        .split('&')
        .filter_map(|part| {
            let mut split = part.splitn(2, '=');
            match (split.next(), split.next()) {
                (Some(key), Some(value)) => {
                    let decoded_value = urlencoding::decode(value).ok()?;
                    Some((key.to_string(), decoded_value.into_owned()))
                }
                _ => None,
            }
        })
        .collect()
}

/// Send HTTP response to the browser
async fn send_response(
    socket: &mut tokio::net::TcpStream,
    status: &str,
    html: String,
) -> std::io::Result<()> {
    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
        status,
        html.len(),
        html
    );
    socket.write_all(response.as_bytes()).await?;
    socket.flush().await?;
    Ok(())
}

/// Generate success/error HTML page
fn success_html(message: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Tairseach - OAuth Authentication</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            margin: 0;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        }}
        .container {{
            background: white;
            padding: 2rem;
            border-radius: 12px;
            box-shadow: 0 10px 40px rgba(0,0,0,0.2);
            text-align: center;
            max-width: 400px;
        }}
        h1 {{
            color: #333;
            margin-bottom: 1rem;
        }}
        p {{
            color: #666;
            line-height: 1.6;
        }}
        .icon {{
            font-size: 3rem;
            margin-bottom: 1rem;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="icon">✨</div>
        <h1>Tairseach</h1>
        <p>{}</p>
    </div>
</body>
</html>"#,
        message
    )
}

/// Fetch user email from Google userinfo endpoint
async fn fetch_google_email(access_token: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;
    
    let response = client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch user info: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("Failed to fetch user info: HTTP {}", response.status()));
    }
    
    let user_info: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse user info: {}", e))?;
    
    user_info
        .get("email")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| "Email not found in user info".to_string())
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Check whether a token's expiry (RFC 3339) is within `margin_secs` of now.
fn is_token_expiring(expiry: &str, margin_secs: i64) -> bool {
    match chrono::DateTime::parse_from_rfc3339(expiry) {
        Ok(exp) => {
            let now = chrono::Utc::now();
            let remaining = exp.signed_duration_since(now).num_seconds();
            remaining < margin_secs
        }
        Err(_) => true, // unparseable ⇒ treat as expired
    }
}
