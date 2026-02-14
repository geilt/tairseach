//! Auth Broker
//!
//! Manages OAuth tokens for CLIs and agents via the Tairseach socket proxy.
//! Tokens are encrypted at rest with AES-256-GCM. The master encryption key is
//! derived from machine identity (hardware UUID + username) via HKDF-SHA256 —
//! no Keychain prompts required.
//!
//! See ADR-001 for the full design rationale.

pub mod credential_types;
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

        // Check scope coverage (with superset recognition)
        if let Some(required) = required_scopes {
            for s in required {
                let covered = record.scopes.iter().any(|existing| {
                    existing == s || scope_covers(existing, s)
                });
                if !covered {
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

    // ── Credential Type Registry ────────────────────────────────────────────

    /// List all registered credential types
    pub async fn list_credential_types(&self) -> Vec<credential_types::CredentialTypeSchema> {
        let store = self.store.read().await;
        store
            .credential_types()
            .list()
            .iter()
            .map(|&schema| schema.clone())
            .collect()
    }

    /// Register a custom credential type
    pub async fn register_custom_credential_type(
        &self,
        schema: credential_types::CredentialTypeSchema,
    ) -> Result<(), String> {
        let mut store = self.store.write().await;
        store.credential_types_mut().register_custom(schema)
    }

    // ── Generic Credential Methods ──────────────────────────────────────────

    /// Store a generic credential
    pub async fn store_credential(
        &self,
        provider: &str,
        account: &str,
        cred_type: &str,
        fields: std::collections::HashMap<String, String>,
        label: Option<&str>,
    ) -> Result<(), String> {
        let mut store = self.store.write().await;
        store.store_credential(provider, account, cred_type, fields, label)
    }

    /// Get a credential (local store only for now)
    pub async fn get_credential(
        &self,
        provider: &str,
        label: Option<&str>,
    ) -> Result<std::collections::HashMap<String, String>, String> {
        let store = self.store.read().await;
        let account = label.unwrap_or("default");
        store
            .get_credential(provider, account)
            .transpose()
            .ok_or_else(|| format!("No credential found for {}:{}", provider, account))?
    }

    /// List all credentials (metadata only)
    pub async fn list_credentials(&self) -> Vec<store::CredentialMetadata> {
        let store = self.store.read().await;
        store.list_credentials()
    }

    /// Delete a credential
    pub async fn delete_credential(&self, provider: &str, account: &str) -> Result<(), String> {
        let mut store = self.store.write().await;
        store.delete_credential(provider, account)
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
    let client = crate::common::create_http_client_with_timeout(10)?;
    
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

// ── Credential Commands ─────────────────────────────────────────────────────

#[tauri::command]
pub async fn auth_credential_types() -> Result<Vec<credential_types::CredentialTypeSchema>, String> {
    let broker = get_or_init_broker().await?;
    Ok(broker.list_credential_types().await)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn auth_credentials_store(
    credType: String,
    label: String,
    fields: std::collections::HashMap<String, String>,
) -> Result<(), String> {
    let broker = get_or_init_broker().await?;
    broker
        .store_credential(&credType, &label, &credType, fields, Some(&label))
        .await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn auth_credentials_list(
    credType: Option<String>,
) -> Result<Vec<store::CredentialMetadata>, String> {
    let broker = get_or_init_broker().await?;
    let mut credentials = broker.list_credentials().await;
    
    if let Some(filter_type) = credType {
        credentials.retain(|c| c.cred_type == filter_type);
    }
    
    Ok(credentials)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn auth_credentials_get(
    credType: String,
    label: String,
) -> Result<std::collections::HashMap<String, String>, String> {
    let broker = get_or_init_broker().await?;
    broker.get_credential(&credType, Some(&label)).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn auth_credentials_delete(credType: String, label: String) -> Result<(), String> {
    let broker = get_or_init_broker().await?;
    broker.delete_credential(&credType, &label).await
}

#[tauri::command]
pub async fn auth_credential_types_custom_create(
    provider_type: String,
    display_name: String,
    fields: Vec<credential_types::CredentialField>,
) -> Result<(), String> {
    let broker = get_or_init_broker().await?;
    
    let schema = credential_types::CredentialTypeSchema {
        provider_type,
        display_name,
        description: String::new(),
        fields,
        supports_multiple: false,
        built_in: false,
    };
    
    broker.register_custom_credential_type(schema).await
}

// ── 1Password Commands ──────────────────────────────────────────────────────

#[tauri::command]
pub async fn op_vaults_list() -> Result<serde_json::Value, String> {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;
    
    let socket_path = dirs::home_dir()
        .ok_or("Could not determine home directory")?
        .join(".tairseach/tairseach.sock");
    
    if !socket_path.exists() {
        return Err("Tairseach socket not found".to_string());
    }
    
    let mut stream = UnixStream::connect(&socket_path)
        .map_err(|e| format!("Failed to connect to Tairseach socket: {}", e))?;
    
    // Send JSON-RPC request
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "op.vaults.list",
        "params": {}
    });
    
    let request_str = serde_json::to_string(&request).unwrap() + "\n";
    stream
        .write_all(request_str.as_bytes())
        .map_err(|e| format!("Failed to write to socket: {}", e))?;
    
    // Read response
    let mut buffer = vec![0u8; 65536];
    let n = stream
        .read(&mut buffer)
        .map_err(|e| format!("Failed to read from socket: {}", e))?;
    
    if n == 0 {
        return Err("No response from Tairseach socket".to_string());
    }
    
    let response_str = String::from_utf8_lossy(&buffer[..n]);
    let response: serde_json::Value = serde_json::from_str(&response_str)
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    
    // Check for JSON-RPC error
    if let Some(error) = response.get("error") {
        return Err(format!("RPC error: {}", error));
    }
    
    // Return the result
    Ok(response.get("result").cloned().unwrap_or(response))
}

#[tauri::command]
pub async fn op_config_set_default_vault(vault_id: String) -> Result<(), String> {
    crate::config::save_onepassword_config(Some(vault_id)).await
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Check whether `existing` scope is a known superset that covers `required`.
///
/// Google uses full-access scopes (e.g. `https://mail.google.com/`) that
/// supersede narrower scopes (e.g. `gmail.modify`, `gmail.readonly`).
fn scope_covers(existing: &str, required: &str) -> bool {
    // Google full-access scopes that cover all sub-scopes for that service
    let supersets: &[(&str, &[&str])] = &[
        (
            "https://mail.google.com/",
            &[
                "https://www.googleapis.com/auth/gmail.modify",
                "https://www.googleapis.com/auth/gmail.readonly",
                "https://www.googleapis.com/auth/gmail.send",
                "https://www.googleapis.com/auth/gmail.compose",
                "https://www.googleapis.com/auth/gmail.labels",
                "https://www.googleapis.com/auth/gmail.settings.basic",
                "https://www.googleapis.com/auth/gmail.settings.sharing",
                "https://www.googleapis.com/auth/gmail.metadata",
            ],
        ),
        (
            "https://www.googleapis.com/auth/drive",
            &[
                "https://www.googleapis.com/auth/drive.file",
                "https://www.googleapis.com/auth/drive.readonly",
                "https://www.googleapis.com/auth/drive.metadata",
                "https://www.googleapis.com/auth/drive.metadata.readonly",
            ],
        ),
        (
            "https://www.googleapis.com/auth/calendar",
            &[
                "https://www.googleapis.com/auth/calendar.readonly",
                "https://www.googleapis.com/auth/calendar.events",
                "https://www.googleapis.com/auth/calendar.events.readonly",
            ],
        ),
        (
            "https://www.googleapis.com/auth/contacts",
            &[
                "https://www.googleapis.com/auth/contacts.readonly",
            ],
        ),
    ];

    for (superset, covered) in supersets {
        if existing == *superset && covered.contains(&required) {
            return true;
        }
    }
    false
}

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
