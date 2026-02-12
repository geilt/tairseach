//! Token Store v2
//!
//! Unified encrypted credential store at `~/.tairseach/credentials.enc.json`.
//! Schema metadata (no secrets) at `~/.tairseach/credentials.schema.json`.
//! Master key derived from machine identity (no Keychain prompts).

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::{info, warn};
use zeroize::Zeroizing;

use super::credential_types::CredentialTypeRegistry;
use super::crypto;
use super::{AccountInfo, TokenRecord};

// ── Public Types ────────────────────────────────────────────────────────────

/// Credential metadata (no secrets)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialMetadata {
    #[serde(rename = "type", alias = "cred_type")]
    pub cred_type: String,
    #[serde(rename = "label", alias = "account")]
    pub account: String,
    pub provider: String,
    pub added: String,
    pub last_refreshed: Option<String>,
}

/// Trait for resolving credentials from 1Password
#[async_trait]
pub trait OnePasswordResolver: Send + Sync {
    async fn resolve_from_1password(
        &self,
        provider: &str,
        account: &str,
    ) -> Result<HashMap<String, String>, String>;
}

/// Credential file format version
const SCHEMA_VERSION: u32 = 2;

// ── File Formats ────────────────────────────────────────────────────────────

/// Encrypted credential entry
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CredentialEntry {
    encrypted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    algorithm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    iv: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tag: Option<String>,
    data: String,
}

/// Credentials file (encrypted)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CredentialsFile {
    version: u32,
    credentials: HashMap<String, CredentialEntry>,
}

impl Default for CredentialsFile {
    fn default() -> Self {
        Self {
            version: SCHEMA_VERSION,
            credentials: HashMap::new(),
        }
    }
}

/// Schema entry (metadata, no secrets)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SchemaEntry {
    provider: String,
    account: String,
    #[serde(rename = "type")]
    cred_type: String,
    scopes: Vec<String>,
    added: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_refreshed: Option<String>,
}

/// Schema file (unencrypted metadata)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SchemaFile {
    version: u32,
    entries: HashMap<String, SchemaEntry>,
}

impl Default for SchemaFile {
    fn default() -> Self {
        Self {
            version: SCHEMA_VERSION,
            entries: HashMap::new(),
        }
    }
}

// ── TokenStore ──────────────────────────────────────────────────────────────

pub struct TokenStore {
    /// Base directory: `~/.tairseach/`
    base_dir: PathBuf,
    /// Master encryption key (zeroized on drop)
    master_key: Zeroizing<[u8; 32]>,
    /// Credentials file
    credentials: CredentialsFile,
    /// Schema file
    schema: SchemaFile,
    /// Credential type registry
    credential_types: CredentialTypeRegistry,
    /// 1Password credential cache (provider:account -> fields)
    onepassword_cache: HashMap<String, HashMap<String, String>>,
}

impl TokenStore {
    /// Create a new TokenStore, deriving the master key and loading credentials.
    pub async fn new() -> Result<Self, String> {
        let home = dirs::home_dir().ok_or("Could not determine home directory")?;
        let base_dir = home.join(".tairseach");

        // Ensure base directory exists
        fs::create_dir_all(&base_dir)
            .map_err(|e| format!("Failed to create .tairseach dir: {}", e))?;

        // Set restrictive permissions on base directory
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&base_dir, fs::Permissions::from_mode(0o700))
                .map_err(|e| format!("Failed to set base dir permissions: {}", e))?;
        }

        // Derive master key (v2)
        let master_key = crypto::derive_master_key()?;

        // Load or initialize credentials file
        let cred_path = base_dir.join("credentials.enc.json");
        let credentials = if cred_path.exists() {
            let data = fs::read_to_string(&cred_path)
                .map_err(|e| format!("Failed to read credentials file: {}", e))?;
            serde_json::from_str(&data)
                .map_err(|e| format!("Failed to parse credentials file: {}", e))?
        } else {
            CredentialsFile::default()
        };

        // Load or initialize schema file
        let schema_path = base_dir.join("credentials.schema.json");
        let schema = if schema_path.exists() {
            let data = fs::read_to_string(&schema_path)
                .map_err(|e| format!("Failed to read schema file: {}", e))?;
            serde_json::from_str(&data)
                .map_err(|e| format!("Failed to parse schema file: {}", e))?
        } else {
            SchemaFile::default()
        };

        let mut store = Self {
            base_dir,
            master_key: Zeroizing::new(master_key),
            credentials,
            schema,
            credential_types: CredentialTypeRegistry::new(),
            onepassword_cache: HashMap::new(),
        };

        // Migrate from v1 if old auth/ directory exists
        store.migrate_from_v1().await?;

        // Auto-encrypt any plaintext entries
        let needs_flush = store.auto_encrypt_plaintext()?;
        if needs_flush {
            store.flush_credentials()?;
        }

        info!(
            "Token store v{} initialized ({} credentials)",
            SCHEMA_VERSION,
            store.credentials.credentials.len()
        );

        Ok(store)
    }

    /// List all accounts (no secrets).
    pub fn list_accounts(&self) -> Vec<AccountInfo> {
        self.schema
            .entries
            .values()
            .map(|e| AccountInfo {
                provider: e.provider.clone(),
                account: e.account.clone(),
                scopes: e.scopes.clone(),
                expiry: String::new(), // loaded from token on demand
                last_refreshed: e.last_refreshed.clone().unwrap_or_default(),
            })
            .collect()
    }

    /// Retrieve a decrypted token record.
    pub fn get_token(
        &self,
        provider: &str,
        account: &str,
    ) -> Result<Option<TokenRecord>, String> {
        let key = credential_key(provider, account);

        let entry = match self.credentials.credentials.get(&key) {
            Some(e) => e,
            None => return Ok(None),
        };

        if !entry.encrypted {
            return Err("Credential is not encrypted (should not happen after auto-encrypt)".to_string());
        }

        // Decode base64 fields
        let iv = BASE64.decode(entry.iv.as_ref().ok_or("Missing IV")?)
            .map_err(|e| format!("Invalid IV base64: {}", e))?;
        let tag = BASE64.decode(entry.tag.as_ref().ok_or("Missing tag")?)
            .map_err(|e| format!("Invalid tag base64: {}", e))?;
        let ciphertext = BASE64.decode(&entry.data)
            .map_err(|e| format!("Invalid ciphertext base64: {}", e))?;

        // Reconstruct the format expected by decrypt(): nonce || ciphertext+tag
        let mut encrypted_blob = Vec::with_capacity(iv.len() + ciphertext.len() + tag.len());
        encrypted_blob.extend_from_slice(&iv);
        encrypted_blob.extend_from_slice(&ciphertext);
        encrypted_blob.extend_from_slice(&tag);

        // Decrypt
        let decrypted = Zeroizing::new(crypto::decrypt(&self.master_key, &encrypted_blob)?);

        // Parse JSON
        let record: TokenRecord = serde_json::from_slice(&*decrypted)
            .map_err(|e| format!("Failed to parse token JSON: {}", e))?;

        Ok(Some(record))
    }

    /// Save (create or update) an encrypted token record.
    pub fn save_token(&mut self, record: &TokenRecord) -> Result<(), String> {
        let key = credential_key(&record.provider, &record.account);

        // Serialize token to JSON
        let json = serde_json::to_vec(record)
            .map_err(|e| format!("Failed to serialize token: {}", e))?;

        // Encrypt and store
        let entry = encrypt_to_entry(&self.master_key, &json)?;
        self.credentials.credentials.insert(key.clone(), entry);

        // Update schema
        let now = chrono::Utc::now().to_rfc3339();
        let schema_entry = SchemaEntry {
            provider: record.provider.clone(),
            account: record.account.clone(),
            cred_type: "oauth2".to_string(),
            scopes: record.scopes.clone(),
            added: self
                .schema
                .entries
                .get(&key)
                .map(|e| e.added.clone())
                .unwrap_or_else(|| now.clone()),
            last_refreshed: Some(now),
        };

        self.schema.entries.insert(key, schema_entry);

        // Flush both files
        self.flush_credentials()?;
        self.flush_schema()?;

        Ok(())
    }

    /// Delete a token record.
    pub fn delete_token(&mut self, provider: &str, account: &str) -> Result<(), String> {
        let key = credential_key(provider, account);

        if self.credentials.credentials.remove(&key).is_none() {
            return Err(format!("No credential found for {}:{}", provider, account));
        }

        self.schema.entries.remove(&key);

        self.flush_credentials()?;
        self.flush_schema()?;

        Ok(())
    }

    /// Load the gog passphrase from encrypted storage.
    pub fn load_gog_passphrase(&self) -> Result<Option<String>, String> {
        // Store gog passphrase as a special credential entry
        match self.get_token("_internal", "gog_passphrase") {
            Ok(Some(record)) => Ok(Some(record.access_token.clone())),
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Save the gog passphrase as an encrypted credential entry.
    pub fn save_gog_passphrase(&mut self, passphrase: &str) -> Result<(), String> {
        // Store as a synthetic TokenRecord under _internal:gog_passphrase
        let record = super::TokenRecord {
            provider: "_internal".to_string(),
            account: "gog_passphrase".to_string(),
            client_id: String::new(),
            client_secret: String::new(),
            token_type: "passphrase".to_string(),
            access_token: passphrase.to_string(),
            refresh_token: String::new(),
            expiry: "9999-12-31T23:59:59Z".to_string(),
            scopes: vec![],
            issued_at: chrono::Utc::now().to_rfc3339(),
            last_refreshed: String::new(),
        };

        self.save_token(&record)?;
        info!("Saved gog passphrase to encrypted credential store");
        Ok(())
    }

    // ── Generic Credential Methods ─────────────────────────────────────────

    /// Store a generic credential (non-OAuth)
    pub fn store_credential(
        &mut self,
        provider: &str,
        account: &str,
        cred_type: &str,
        fields: HashMap<String, String>,
        label: Option<&str>,
    ) -> Result<(), String> {
        // Validate against schema if known type
        if let Some(schema) = self.credential_types.get(cred_type) {
            schema.validate(&fields)?;
        }

        let key = credential_key(provider, account);

        // Serialize fields to JSON
        let json = serde_json::to_vec(&fields)
            .map_err(|e| format!("Failed to serialize credential: {}", e))?;

        // Encrypt and store
        let entry = encrypt_to_entry(&self.master_key, &json)?;
        self.credentials.credentials.insert(key.clone(), entry);

        // Update schema
        let now = chrono::Utc::now().to_rfc3339();
        let schema_entry = SchemaEntry {
            provider: provider.to_string(),
            account: label.unwrap_or(account).to_string(),
            cred_type: cred_type.to_string(),
            scopes: vec![], // Not applicable for generic credentials
            added: self
                .schema
                .entries
                .get(&key)
                .map(|e| e.added.clone())
                .unwrap_or_else(|| now.clone()),
            last_refreshed: Some(now),
        };

        self.schema.entries.insert(key, schema_entry);

        // Flush both files
        self.flush_credentials()?;
        self.flush_schema()?;

        info!("Stored credential {}:{} (type: {})", provider, account, cred_type);
        Ok(())
    }

    /// Retrieve a generic credential
    pub fn get_credential(
        &self,
        provider: &str,
        account: &str,
    ) -> Result<Option<HashMap<String, String>>, String> {
        let key = credential_key(provider, account);

        let entry = match self.credentials.credentials.get(&key) {
            Some(e) => e,
            None => return Ok(None),
        };

        if !entry.encrypted {
            return Err("Credential is not encrypted".to_string());
        }

        // Decode and decrypt (same as get_token)
        let iv = BASE64.decode(entry.iv.as_ref().ok_or("Missing IV")?)
            .map_err(|e| format!("Invalid IV base64: {}", e))?;
        let tag = BASE64.decode(entry.tag.as_ref().ok_or("Missing tag")?)
            .map_err(|e| format!("Invalid tag base64: {}", e))?;
        let ciphertext = BASE64.decode(&entry.data)
            .map_err(|e| format!("Invalid ciphertext base64: {}", e))?;

        let mut encrypted_blob = Vec::with_capacity(iv.len() + ciphertext.len() + tag.len());
        encrypted_blob.extend_from_slice(&iv);
        encrypted_blob.extend_from_slice(&ciphertext);
        encrypted_blob.extend_from_slice(&tag);

        let decrypted = Zeroizing::new(crypto::decrypt(&self.master_key, &encrypted_blob)?);

        // Try parsing as TokenRecord first (backward compat), then as generic credential
        if let Ok(token) = serde_json::from_slice::<TokenRecord>(&*decrypted) {
            // Convert TokenRecord to field map for consistency
            let mut fields = HashMap::new();
            fields.insert("access_token".to_string(), token.access_token.clone());
            if !token.refresh_token.is_empty() {
                fields.insert("refresh_token".to_string(), token.refresh_token.clone());
            }
            return Ok(Some(fields));
        }

        // Parse as generic credential
        let fields: HashMap<String, String> = serde_json::from_slice(&*decrypted)
            .map_err(|e| format!("Failed to parse credential JSON: {}", e))?;

        Ok(Some(fields))
    }

    /// List all credentials (metadata only)
    pub fn list_credentials(&self) -> Vec<CredentialMetadata> {
        self.schema
            .entries
            .iter()
            .map(|(_key, entry)| CredentialMetadata {
                provider: entry.provider.clone(),
                account: entry.account.clone(),
                cred_type: entry.cred_type.clone(),
                added: entry.added.clone(),
                last_refreshed: entry.last_refreshed.clone(),
            })
            .collect()
    }

    /// Delete a credential
    pub fn delete_credential(&mut self, provider: &str, account: &str) -> Result<(), String> {
        // Alias to delete_token for now
        self.delete_token(provider, account)
    }

    /// Resolve credential with fallback chain:
    /// 1. Local encrypted store
    /// 2. 1Password vault (if configured)
    /// 3. Not found error
    pub async fn resolve_credential(
        &mut self,
        provider: &str,
        account: Option<&str>,
        onepassword_client: Option<&dyn OnePasswordResolver>,
    ) -> Result<HashMap<String, String>, String> {
        let account_key = account.unwrap_or("default");

        // 1. Try local store first
        if let Some(fields) = self.get_credential(provider, account_key)? {
            info!("Resolved credential {}:{} from local store", provider, account_key);
            return Ok(fields);
        }

        // 2. Try 1Password fallback if client provided
        if let Some(op_client) = onepassword_client {
            let cache_key = format!("{}:{}", provider, account_key);
            
            // Check cache first
            if let Some(cached) = self.onepassword_cache.get(&cache_key) {
                info!("Resolved credential {}:{} from 1Password cache", provider, account_key);
                return Ok(cached.clone());
            }

            // Fetch from 1Password
            match op_client.resolve_from_1password(provider, account_key).await {
                Ok(fields) => {
                    info!("Resolved credential {}:{} from 1Password", provider, account_key);
                    
                    // Cache it locally
                    self.store_credential(provider, account_key, provider, fields.clone(), None)?;
                    self.onepassword_cache.insert(cache_key, fields.clone());
                    
                    return Ok(fields);
                }
                Err(e) => {
                    warn!("1Password resolution failed for {}:{}: {}", provider, account_key, e);
                }
            }
        }

        // 3. Not found
        Err(format!(
            "No credential found for {}:{} (checked local store{})",
            provider,
            account_key,
            if onepassword_client.is_some() { " and 1Password" } else { "" }
        ))
    }

    /// Access the credential type registry
    pub fn credential_types(&self) -> &CredentialTypeRegistry {
        &self.credential_types
    }

    /// Access the credential type registry (mutable)
    pub fn credential_types_mut(&mut self) -> &mut CredentialTypeRegistry {
        &mut self.credential_types
    }

    // ── Internal ────────────────────────────────────────────────────────────

    fn flush_credentials(&self) -> Result<(), String> {
        let path = self.base_dir.join("credentials.enc.json");
        let json = serde_json::to_string_pretty(&self.credentials)
            .map_err(|e| format!("Failed to serialize credentials: {}", e))?;
        fs::write(&path, &json)
            .map_err(|e| format!("Failed to write credentials file: {}", e))?;

        // Set restrictive file permissions (0600)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
                .map_err(|e| format!("Failed to set credentials file permissions: {}", e))?;
        }

        Ok(())
    }

    fn flush_schema(&self) -> Result<(), String> {
        let path = self.base_dir.join("credentials.schema.json");
        let json = serde_json::to_string_pretty(&self.schema)
            .map_err(|e| format!("Failed to serialize schema: {}", e))?;
        fs::write(&path, &json)
            .map_err(|e| format!("Failed to write schema file: {}", e))?;

        // Set readable permissions (0644)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o644))
                .map_err(|e| format!("Failed to set schema file permissions: {}", e))?;
        }

        Ok(())
    }

    /// Auto-encrypt any plaintext entries.
    /// Returns true if any entries were encrypted (requiring flush).
    fn auto_encrypt_plaintext(&mut self) -> Result<bool, String> {
        let mut modified = false;

        let keys: Vec<String> = self.credentials.credentials.keys().cloned().collect();

        for key in keys {
            let entry = self.credentials.credentials.get(&key).unwrap();
            
            if !entry.encrypted {
                info!("Auto-encrypting plaintext credential: {}", key);
                let new_entry = encrypt_to_entry(&self.master_key, entry.data.as_bytes())?;
                self.credentials.credentials.insert(key, new_entry);
                modified = true;
            }
        }

        Ok(modified)
    }

    /// Migrate from v1 (individual .json.enc files + Keychain) to v2.
    async fn migrate_from_v1(&mut self) -> Result<(), String> {
        let old_auth_dir = self.base_dir.join("auth");
        
        if !old_auth_dir.exists() {
            // No v1 data to migrate
            return Ok(());
        }

        info!("Detected v1 auth directory, attempting migration");

        #[cfg(feature = "keychain-migration")]
        {
            self.do_v1_migration(&old_auth_dir)
        }

        #[cfg(not(feature = "keychain-migration"))]
        {
            warn!("V1 migration requires keychain-migration feature - skipping");
            Ok(())
        }
    }

    #[cfg(feature = "keychain-migration")]
    fn do_v1_migration(&mut self, old_auth_dir: &std::path::Path) -> Result<(), String> {
        // Try to read old master key from Keychain
            let old_key = match crypto::read_keychain_master_key() {
                Ok(key) => {
                    info!("Retrieved v1 master key from Keychain");
                    key
                }
                Err(e) => {
                    warn!("Could not read v1 master key from Keychain: {}", e);
                    warn!("Skipping v1 migration - manual token re-import required");
                    return Ok(());
                }
            };

            // Find all .json.enc files
            let providers_dir = old_auth_dir.join("providers");
            if !providers_dir.exists() {
                return Ok(());
            }

            let mut migrated_count = 0;

            for provider_dir in fs::read_dir(&providers_dir)
                .map_err(|e| format!("Failed to read providers dir: {}", e))?
            {
                let provider_dir = provider_dir
                    .map_err(|e| format!("Failed to read provider dir entry: {}", e))?;
                
                if !provider_dir.file_type()
                    .map_err(|e| format!("Failed to get file type: {}", e))?
                    .is_dir()
                {
                    continue;
                }

                for entry in fs::read_dir(provider_dir.path())
                    .map_err(|e| format!("Failed to read provider subdir: {}", e))?
                {
                    let entry = entry
                        .map_err(|e| format!("Failed to read entry: {}", e))?;
                    
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("enc") {
                        // Decrypt with old key
                        let encrypted = fs::read(&path)
                            .map_err(|e| format!("Failed to read v1 token file: {}", e))?;
                        
                        match crypto::decrypt(&old_key, &encrypted) {
                            Ok(decrypted) => {
                                let record: TokenRecord = serde_json::from_slice(&decrypted)
                                    .map_err(|e| format!("Failed to parse v1 token: {}", e))?;
                                
                                info!(
                                    "Migrating token: {}:{}",
                                    record.provider, record.account
                                );
                                
                                // Re-encrypt with new key and save
                                self.save_token(&record)?;
                                migrated_count += 1;
                            }
                            Err(e) => {
                                warn!("Failed to decrypt v1 token at {:?}: {}", path, e);
                            }
                        }
                    }
                }
            }

            if migrated_count > 0 {
                info!("Successfully migrated {} tokens from v1 to v2", migrated_count);
                
                // Optionally delete v1 Keychain entry
                match crypto::delete_keychain_master_key() {
                    Ok(()) => info!("Deleted v1 master key from Keychain"),
                    Err(e) => warn!("Could not delete v1 Keychain key: {}", e),
                }
            }

            Ok(())
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Build the credential key for a provider:account pair.
fn credential_key(provider: &str, account: &str) -> String {
    format!("{}:{}", provider, account)
}

/// Encrypt plaintext bytes and return a CredentialEntry.
/// Splits AES-256-GCM output (nonce || ciphertext || tag) into base64 components.
fn encrypt_to_entry(key: &[u8; 32], plaintext: &[u8]) -> Result<CredentialEntry, String> {
    let blob = crypto::encrypt(key, plaintext)?;
    if blob.len() < 12 + 16 {
        return Err("Encrypted blob too short".to_string());
    }
    let iv = &blob[0..12];
    let ciphertext_and_tag = &blob[12..];
    let tag_offset = ciphertext_and_tag.len() - 16;
    let ciphertext = &ciphertext_and_tag[..tag_offset];
    let tag = &ciphertext_and_tag[tag_offset..];

    Ok(CredentialEntry {
        encrypted: true,
        algorithm: Some("aes-256-gcm".to_string()),
        iv: Some(BASE64.encode(iv)),
        tag: Some(BASE64.encode(tag)),
        data: BASE64.encode(ciphertext),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_key() {
        let key = credential_key("google", "alex@example.com");
        assert_eq!(key, "google:alex@example.com");
    }

    #[test]
    fn test_credential_key_deterministic() {
        let k1 = credential_key("google", "alex@example.com");
        let k2 = credential_key("google", "alex@example.com");
        assert_eq!(k1, k2);
    }
}
