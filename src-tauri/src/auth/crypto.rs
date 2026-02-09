//! Cryptographic utilities for the Auth Broker
//!
//! - AES-256-GCM for token encryption at rest
//! - Master key management via macOS Keychain (Security framework)
//! - Passphrase generation

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use rand::RngCore;
use tracing::info;

/// Keychain service name for the master key
const KEYCHAIN_SERVICE: &str = "com.naonur.tairseach.auth-broker";
/// Keychain account name for the master key
const KEYCHAIN_ACCOUNT: &str = "master-key";

/// AES-256-GCM nonce size (96 bits)
const NONCE_SIZE: usize = 12;
/// AES-256 key size (256 bits)
const KEY_SIZE: usize = 32;

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

// ── Master Key ──────────────────────────────────────────────────────────────

/// Get or create the master encryption key from the macOS Keychain.
///
/// On first call, generates a random 256-bit key and stores it in the Keychain
/// under Tairseach's code-signing identity. Subsequent calls retrieve it.
/// The user approves Keychain access once; after that it's permanent.
///
/// **SECURITY:** Uses native Security framework APIs. The key NEVER appears in
/// process arguments or environment variables.
#[cfg(target_os = "macos")]
pub fn get_or_create_master_key() -> Result<[u8; KEY_SIZE], String> {
    // Try to read existing key
    match read_keychain_item() {
        Ok(key) => {
            info!("Retrieved master key from Keychain");
            Ok(key)
        }
        Err(_) => {
            // Generate new key
            info!("Generating new master key and storing in Keychain");
            let mut key = [0u8; KEY_SIZE];
            OsRng.fill_bytes(&mut key);

            write_keychain_item(&key)?;
            Ok(key)
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn get_or_create_master_key() -> Result<[u8; KEY_SIZE], String> {
    Err("Master key management requires macOS Keychain".to_string())
}

/// Read the master key from macOS Keychain using the Security framework.
///
/// **SECURITY:** Direct API calls, no CLI subprocess, key stays in process memory.
#[cfg(target_os = "macos")]
fn read_keychain_item() -> Result<[u8; KEY_SIZE], String> {
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

/// Store the master key in macOS Keychain using the Security framework.
///
/// **SECURITY:** Direct API calls, no CLI subprocess, key never exposed to ps/pgrep.
#[cfg(target_os = "macos")]
fn write_keychain_item(key: &[u8; KEY_SIZE]) -> Result<(), String> {
    use security_framework::passwords::{delete_generic_password, set_generic_password};

    let hex_str = key_to_hex(key);

    // Delete existing item if present (ignore errors)
    let _ = delete_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT);

    // Store new key
    set_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT, hex_str.as_bytes())
        .map_err(|e| format!("Failed to store master key in Keychain: {}", e))?;

    info!("Master key stored in Keychain");
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

// ── Hex Utilities ───────────────────────────────────────────────────────────

fn key_to_hex(key: &[u8; KEY_SIZE]) -> String {
    hex::encode(key)
}

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
    fn test_hex_roundtrip() {
        let mut key = [0u8; KEY_SIZE];
        OsRng.fill_bytes(&mut key);
        let hex = key_to_hex(&key);
        let back = hex_to_key(&hex).unwrap();
        assert_eq!(key, back);
    }
}
