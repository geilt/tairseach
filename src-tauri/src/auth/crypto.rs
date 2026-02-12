//! Cryptographic utilities for the Auth Broker
//!
//! - AES-256-GCM for token encryption at rest
//! - Master key derivation via HKDF from machine identity
//! - Passphrase generation

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;
use std::process::Command;
use std::sync::OnceLock;
use tracing::info;

/// AES-256-GCM nonce size (96 bits)
const NONCE_SIZE: usize = 12;
/// AES-256 key size (256 bits)
const KEY_SIZE: usize = 32;

/// Static salt for machine identity
const STATIC_SALT: &str = "nechtan-guards-the-secrets";
/// HKDF salt for key derivation
const HKDF_SALT: &[u8] = b"tairseach-credential-store-v2";
/// HKDF info for master key
const HKDF_INFO: &[u8] = b"master-key";

// V1 Keychain constants (only used if keychain-migration feature enabled)
#[cfg(feature = "keychain-migration")]
const KEYCHAIN_SERVICE: &str = "com.naonur.tairseach.auth-broker";
#[cfg(feature = "keychain-migration")]
const KEYCHAIN_ACCOUNT: &str = "master-key";

// ── Encryption / Decryption ─────────────────────────────────────────────────

/// Encrypt plaintext bytes with AES-256-GCM.
/// Returns: nonce (12 bytes) || ciphertext+tag
pub fn encrypt(key: &[u8; KEY_SIZE], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let cipher_key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(cipher_key);

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| format!("Encryption failed: {}", e))?;

    // Prepend nonce
    let mut output = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&ciphertext);
    Ok(output)
}

/// Decrypt data produced by `encrypt()`.
pub fn decrypt(key: &[u8; KEY_SIZE], data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < NONCE_SIZE {
        return Err("Ciphertext too short".to_string());
    }

    let (nonce_bytes, ciphertext) = data.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);

    let cipher_key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(cipher_key);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption failed: {}", e))
}

// ── Master Key (v2: Machine-Derived) ───────────────────────────────────────

/// Derive the master encryption key from machine identity.
///
/// Uses HKDF-SHA256 with inputs:
/// - Hardware UUID (from IOPlatformExpertDevice)
/// - Username (from $USER env var)
/// - Static salt ("nechtan-guards-the-secrets")
///
/// **SECURITY:** Deterministic, machine-bound, no user interaction required.
/// The key is derived fresh on each call — never stored on disk.
#[cfg(target_os = "macos")]
pub fn derive_master_key() -> Result<[u8; KEY_SIZE], String> {
    let hw_uuid = get_hardware_uuid()?;
    let username = std::env::var("USER")
        .map_err(|_| "Could not determine username from $USER")?;

    let ikm = format!("{}:{}:{}", hw_uuid, username, STATIC_SALT);
    
    let hkdf = Hkdf::<Sha256>::new(Some(HKDF_SALT), ikm.as_bytes());
    let mut key = [0u8; KEY_SIZE];
    hkdf.expand(HKDF_INFO, &mut key)
        .map_err(|e| format!("HKDF expand failed: {}", e))?;

    info!("Derived master key from machine identity");
    Ok(key)
}

#[cfg(not(target_os = "macos"))]
pub fn derive_master_key() -> Result<[u8; KEY_SIZE], String> {
    Err("Master key derivation requires macOS hardware UUID".to_string())
}

/// Cached hardware UUID (fetched once per process lifetime).
#[cfg(target_os = "macos")]
static HARDWARE_UUID: OnceLock<String> = OnceLock::new();

/// Retrieve the hardware UUID from macOS IOKit (cached).
#[cfg(target_os = "macos")]
fn get_hardware_uuid() -> Result<String, String> {
    if let Some(uuid) = HARDWARE_UUID.get() {
        return Ok(uuid.clone());
    }
    
    let uuid = fetch_hardware_uuid()?;
    let _ = HARDWARE_UUID.set(uuid.clone());
    Ok(uuid)
}

/// Fetch the hardware UUID from ioreg (internal, uncached).
#[cfg(target_os = "macos")]
fn fetch_hardware_uuid() -> Result<String, String> {
    let output = Command::new("ioreg")
        .args(["-d2", "-c", "IOPlatformExpertDevice"])
        .output()
        .map_err(|e| format!("Failed to execute ioreg: {}", e))?;

    if !output.status.success() {
        return Err("ioreg command failed".to_string());
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| format!("Invalid UTF-8 from ioreg: {}", e))?;

    // Parse: "IOPlatformUUID" = "XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX"
    for line in stdout.lines() {
        if line.contains("IOPlatformUUID") {
            if let Some(start) = line.find('"').and_then(|i| line[i+1..].find('"').map(|j| i+j+2)) {
                if let Some(end) = line[start..].find('"').map(|i| start + i) {
                    let uuid = &line[start..end];
                    if !uuid.is_empty() {
                        return Ok(uuid.to_string());
                    }
                }
            }
        }
    }

    Err("Could not find IOPlatformUUID in ioreg output".to_string())
}

// ── V1 Keychain Migration (feature-gated) ──────────────────────────────────

/// Read the master key from macOS Keychain (v1 migration only).
///
/// **SECURITY:** Direct API calls, no CLI subprocess, key stays in process memory.
#[cfg(all(target_os = "macos", feature = "keychain-migration"))]
pub fn read_keychain_master_key() -> Result<[u8; KEY_SIZE], String> {
    use security_framework::passwords::get_generic_password;

    match get_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT) {
        Ok(password_data) => {
            let hex_str = String::from_utf8(password_data)
                .map_err(|e| format!("Invalid UTF-8 in keychain data: {}", e))?;
            hex_to_key(&hex_str)
        }
        Err(e) => {
            let err_str = format!("{:?}", e);
            if err_str.contains("ItemNotFound") || err_str.contains("not found") {
                Err("Master key not found in Keychain".to_string())
            } else {
                Err(format!("Keychain read error: {}", e))
            }
        }
    }
}

/// Delete the master key from macOS Keychain (v1 cleanup).
#[cfg(all(target_os = "macos", feature = "keychain-migration"))]
pub fn delete_keychain_master_key() -> Result<(), String> {
    use security_framework::passwords::delete_generic_password;
    
    delete_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
        .map_err(|e| format!("Failed to delete keychain master key: {}", e))?;
    
    info!("Deleted v1 master key from Keychain");
    Ok(())
}

// ── Passphrase Generation ───────────────────────────────────────────────────

/// Generate a strong random passphrase for gog's file keyring backend.
/// Returns a 64-character hex string (256 bits of entropy).
pub fn generate_passphrase() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

// ── Hex Utilities (for v1 migration) ────────────────────────────────────────

#[cfg(feature = "keychain-migration")]
fn key_to_hex(key: &[u8; KEY_SIZE]) -> String {
    hex::encode(key)
}

#[cfg(feature = "keychain-migration")]
fn hex_to_key(hex_str: &str) -> Result<[u8; KEY_SIZE], String> {
    let bytes = hex::decode(hex_str).map_err(|e| format!("Invalid hex: {}", e))?;
    if bytes.len() != KEY_SIZE {
        return Err(format!(
            "Key length mismatch: expected {} bytes, got {}",
            KEY_SIZE,
            bytes.len()
        ));
    }
    let mut key = [0u8; KEY_SIZE];
    key.copy_from_slice(&bytes);
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let mut key = [0u8; KEY_SIZE];
        OsRng.fill_bytes(&mut key);

        let plaintext = b"hello, auth broker!";
        let encrypted = encrypt(&key, plaintext).unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_key_fails() {
        let mut key1 = [0u8; KEY_SIZE];
        let mut key2 = [0u8; KEY_SIZE];
        OsRng.fill_bytes(&mut key1);
        OsRng.fill_bytes(&mut key2);

        let encrypted = encrypt(&key1, b"secret").unwrap();
        assert!(decrypt(&key2, &encrypted).is_err());
    }

    #[test]
    fn test_passphrase_length() {
        let p = generate_passphrase();
        assert_eq!(p.len(), 64);
    }

    #[test]
    #[cfg(feature = "keychain-migration")]
    fn test_hex_roundtrip() {
        let mut key = [0u8; KEY_SIZE];
        OsRng.fill_bytes(&mut key);
        let hex = key_to_hex(&key);
        let back = hex_to_key(&hex).unwrap();
        assert_eq!(key, back);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_hardware_uuid_retrieval() {
        // Skip if ioreg is not available (e.g., in CI)
        if std::process::Command::new("which").arg("ioreg").output().ok()
            .map(|o| o.status.success()).unwrap_or(false)
        {
            let uuid = get_hardware_uuid().unwrap();
            assert!(!uuid.is_empty());
            assert!(uuid.contains('-'));
            // UUID format: 8-4-4-4-12 characters
            assert_eq!(uuid.len(), 36);
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_derive_master_key_deterministic() {
        // Skip if ioreg is not available (e.g., in CI)
        if std::process::Command::new("which").arg("ioreg").output().ok()
            .map(|o| o.status.success()).unwrap_or(false)
        {
            let key1 = derive_master_key().unwrap();
            let key2 = derive_master_key().unwrap();
            assert_eq!(key1, key2, "Derived key should be deterministic");
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_derive_master_key_length() {
        // Skip if ioreg is not available (e.g., in CI)
        if std::process::Command::new("which").arg("ioreg").output().ok()
            .map(|o| o.status.success()).unwrap_or(false)
        {
            let key = derive_master_key().unwrap();
            assert_eq!(key.len(), KEY_SIZE);
        }
    }
}
