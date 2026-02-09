//! Token Store
//!
//! Encrypted file-based token storage at `~/.tairseach/auth/`.
//! Each token is stored as an encrypted JSON file. A metadata index
//! (`metadata.json`, unencrypted) maps provider:account → file paths.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use tracing::{info, warn};

use super::crypto;
use super::{AccountInfo, TokenRecord};

/// File extension for encrypted token files
const ENC_EXT: &str = "json.enc";
/// Passphrase filename
const GOG_PASSPHRASE_FILE: &str = "gog_passphrase.enc";

// ── Metadata ────────────────────────────────────────────────────────────────

/// Metadata index (unencrypted — contains no secrets)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Metadata {
    version: u32,
    accounts: Vec<MetadataEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetadataEntry {
    provider: String,
    account: String,
    scopes: Vec<String>,
    added: String,
    last_used: String,
    file: String,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            version: 1,
            accounts: Vec::new(),
        }
    }
}

// ── TokenStore ──────────────────────────────────────────────────────────────

pub struct TokenStore {
    /// Base directory: `~/.tairseach/auth/`
    base_dir: PathBuf,
    /// Master encryption key
    master_key: [u8; 32],
    /// In-memory metadata cache
    metadata: Metadata,
}

impl TokenStore {
    /// Create a new TokenStore, initialising the directory structure and
    /// loading (or creating) the master key.
    pub async fn new() -> Result<Self, String> {
        let home = dirs::home_dir().ok_or("Could not determine home directory")?;
        let base_dir = home.join(".tairseach").join("auth");

        // Ensure directory structure
        let providers_dir = base_dir.join("providers").join("google");
        fs::create_dir_all(&providers_dir)
            .map_err(|e| format!("Failed to create auth dir: {}", e))?;

        // Get or create master key
        let master_key = crypto::get_or_create_master_key()?;

        // Load metadata
        let metadata_path = base_dir.join("metadata.json");
        let metadata = if metadata_path.exists() {
            let data = fs::read_to_string(&metadata_path)
                .map_err(|e| format!("Failed to read metadata: {}", e))?;
            serde_json::from_str(&data)
                .map_err(|e| format!("Failed to parse metadata: {}", e))?
        } else {
            let m = Metadata::default();
            let json = serde_json::to_string_pretty(&m)
                .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
            fs::write(&metadata_path, &json)
                .map_err(|e| format!("Failed to write metadata: {}", e))?;
            m
        };

        info!(
            "Token store initialized at {:?} ({} accounts)",
            base_dir,
            metadata.accounts.len()
        );

        Ok(Self {
            base_dir,
            master_key,
            metadata,
        })
    }

    /// List all accounts (no secrets).
    pub fn list_accounts(&self) -> Vec<AccountInfo> {
        self.metadata
            .accounts
            .iter()
            .map(|e| AccountInfo {
                provider: e.provider.clone(),
                account: e.account.clone(),
                scopes: e.scopes.clone(),
                expiry: String::new(), // loaded from token file on demand
                last_refreshed: e.last_used.clone(),
            })
            .collect()
    }

    /// Retrieve a decrypted token record.
    pub fn get_token(
        &self,
        provider: &str,
        account: &str,
    ) -> Result<Option<TokenRecord>, String> {
        let entry = self
            .metadata
            .accounts
            .iter()
            .find(|e| e.provider == provider && e.account == account);

        let entry = match entry {
            Some(e) => e,
            None => return Ok(None),
        };

        let file_path = self.base_dir.join(&entry.file);
        if !file_path.exists() {
            warn!(
                "Metadata references {:?} but file does not exist",
                file_path
            );
            return Ok(None);
        }

        let encrypted = fs::read(&file_path)
            .map_err(|e| format!("Failed to read token file: {}", e))?;

        let decrypted = crypto::decrypt(&self.master_key, &encrypted)?;

        let record: TokenRecord = serde_json::from_slice(&decrypted)
            .map_err(|e| format!("Failed to parse token JSON: {}", e))?;

        Ok(Some(record))
    }

    /// Save (create or update) an encrypted token record.
    pub fn save_token(&mut self, record: &TokenRecord) -> Result<(), String> {
        let file_rel = token_file_path(&record.provider, &record.account);
        let file_abs = self.base_dir.join(&file_rel);

        // Ensure parent directory exists
        if let Some(parent) = file_abs.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create provider dir: {}", e))?;
        }

        // Encrypt and write
        let json = serde_json::to_vec_pretty(record)
            .map_err(|e| format!("Failed to serialize token: {}", e))?;
        let encrypted = crypto::encrypt(&self.master_key, &json)?;
        fs::write(&file_abs, &encrypted)
            .map_err(|e| format!("Failed to write token file: {}", e))?;

        // Update metadata
        let now = chrono::Utc::now().to_rfc3339();
        if let Some(entry) = self
            .metadata
            .accounts
            .iter_mut()
            .find(|e| e.provider == record.provider && e.account == record.account)
        {
            entry.scopes = record.scopes.clone();
            entry.last_used = now;
        } else {
            self.metadata.accounts.push(MetadataEntry {
                provider: record.provider.clone(),
                account: record.account.clone(),
                scopes: record.scopes.clone(),
                added: now.clone(),
                last_used: now,
                file: file_rel,
            });
        }

        self.flush_metadata()?;
        Ok(())
    }

    /// Delete a token record.
    pub fn delete_token(&mut self, provider: &str, account: &str) -> Result<(), String> {
        let idx = self
            .metadata
            .accounts
            .iter()
            .position(|e| e.provider == provider && e.account == account);

        if let Some(idx) = idx {
            let entry = self.metadata.accounts.remove(idx);
            let file_path = self.base_dir.join(&entry.file);
            if file_path.exists() {
                fs::remove_file(&file_path)
                    .map_err(|e| format!("Failed to delete token file: {}", e))?;
            }
            self.flush_metadata()?;
            Ok(())
        } else {
            Err(format!("No token found for {}:{}", provider, account))
        }
    }

    /// Load the gog passphrase from encrypted storage.
    pub fn load_gog_passphrase(&self) -> Result<Option<String>, String> {
        let file_path = self.base_dir.join(GOG_PASSPHRASE_FILE);
        if !file_path.exists() {
            return Ok(None);
        }

        let encrypted = fs::read(&file_path)
            .map_err(|e| format!("Failed to read gog passphrase file: {}", e))?;
        let decrypted = crypto::decrypt(&self.master_key, &encrypted)?;
        let passphrase = String::from_utf8(decrypted)
            .map_err(|e| format!("Invalid gog passphrase encoding: {}", e))?;

        Ok(Some(passphrase))
    }

    /// Save the gog passphrase (encrypted).
    pub fn save_gog_passphrase(&self, passphrase: &str) -> Result<(), String> {
        let file_path = self.base_dir.join(GOG_PASSPHRASE_FILE);
        let encrypted = crypto::encrypt(&self.master_key, passphrase.as_bytes())?;
        fs::write(&file_path, &encrypted)
            .map_err(|e| format!("Failed to write gog passphrase: {}", e))?;
        Ok(())
    }

    // ── Internal ────────────────────────────────────────────────────────────

    fn flush_metadata(&self) -> Result<(), String> {
        let path = self.base_dir.join("metadata.json");
        let json = serde_json::to_string_pretty(&self.metadata)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
        fs::write(&path, &json)
            .map_err(|e| format!("Failed to write metadata: {}", e))?;
        Ok(())
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Build the relative file path for a token.
/// Uses SHA-256 of `provider:account` to derive a non-identifying filename.
fn token_file_path(provider: &str, account: &str) -> String {
    let input = format!("{}:{}", provider, account);
    let hash = Sha256::digest(input.as_bytes());
    let hash_hex = hex::encode(&hash[..8]); // First 8 bytes = 16 hex chars (enough uniqueness)
    format!("providers/{}/{}.{}", provider, hash_hex, ENC_EXT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_file_path() {
        let path = token_file_path("google", "alex@esotech.com");
        assert!(path.starts_with("providers/google/"));
        assert!(path.ends_with(".json.enc"));
    }

    #[test]
    fn test_deterministic_path() {
        let p1 = token_file_path("google", "alex@esotech.com");
        let p2 = token_file_path("google", "alex@esotech.com");
        assert_eq!(p1, p2);
    }

    #[test]
    fn test_different_accounts_different_paths() {
        let p1 = token_file_path("google", "alex@esotech.com");
        let p2 = token_file_path("google", "other@example.com");
        assert_ne!(p1, p2);
    }
}
