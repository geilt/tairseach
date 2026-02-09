//! Auth Broker
//!
//! Manages OAuth tokens outside the macOS Keychain, serving them to CLIs and
//! agents via the Tairseach socket proxy. Tokens are encrypted at rest with
//! AES-256-GCM. The master encryption key is stored in the macOS Keychain
//! under a single, permanently-approved item.
//!
//! See ADR-001 for the full design rationale.

pub mod crypto;
pub mod provider;
pub mod store;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use self::provider::google::GoogleProvider;
use self::provider::OAuthProvider;
use self::store::TokenStore;

// ── Public types ────────────────────────────────────────────────────────────

/// Stored token record (decrypted form)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRecord {
    pub provider: String,
    pub account: String,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
    pub token_type: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expiry: String,
    pub scopes: Vec<String>,
    #[serde(default)]
    pub issued_at: String,
    #[serde(default)]
    pub last_refreshed: String,
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

        // Persist it
        {
            let store = self.store.read().await;
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

// ── Tauri commands (kept for backward compat with the stub) ─────────────────

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
