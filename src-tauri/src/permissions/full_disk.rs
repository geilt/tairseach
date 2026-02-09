//! Full Disk Access Permission
//!
//! Checks authorization status for Full Disk Access (SystemPolicyAllFiles).
//! This permission is required to access protected locations like:
//! - ~/Library/Mail
//! - ~/Library/Messages
//! - ~/Library/Safari
//! - /Library/Application Support/com.apple.TCC/TCC.db

use super::{Permission, PermissionError, PermissionStatus};
use std::fs;
use std::path::PathBuf;

/// Check Full Disk Access permission status
pub fn check() -> Result<Permission, PermissionError> {
    let status = get_authorization_status();

    Ok(Permission::new(
        "full_disk_access",
        "Full Disk Access",
        "Access to protected files and folders",
        status,
        true, // Critical for file access operations
    ))
}

/// Check Full Disk Access by probing protected locations
fn get_authorization_status() -> PermissionStatus {
    // List of paths that require Full Disk Access to read
    let protected_paths = [
        // TCC database - very reliable indicator
        "/Library/Application Support/com.apple.TCC/TCC.db",
        // User's TCC database (if it exists)
    ];

    // Also try user-specific paths
    let user_tcc = dirs::home_dir()
        .map(|h| h.join("Library/Application Support/com.apple.TCC/TCC.db"));

    for path_str in protected_paths.iter() {
        let path = PathBuf::from(path_str);

        if !path.exists() {
            continue;
        }

        match fs::read(&path) {
            Ok(_) => {
                tracing::debug!("FDA check: Can read {}, permission granted", path.display());
                return PermissionStatus::Granted;
            }
            Err(e) => {
                tracing::debug!("FDA check: Cannot read {}: {}", path.display(), e);
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    return PermissionStatus::Denied;
                }
            }
        }
    }

    // Try user TCC db
    if let Some(path) = user_tcc {
        if path.exists() {
            match fs::read(&path) {
                Ok(_) => return PermissionStatus::Granted,
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                    return PermissionStatus::Denied;
                }
                _ => {}
            }
        }
    }

    PermissionStatus::Unknown
}

/// Additional protected paths that can be checked
#[allow(dead_code)]
pub mod protected_paths {
    use std::path::PathBuf;

    pub fn mail_data() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join("Library/Mail"))
    }

    pub fn messages_data() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join("Library/Messages"))
    }

    pub fn safari_data() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join("Library/Safari"))
    }

    pub fn system_tcc_db() -> PathBuf {
        PathBuf::from("/Library/Application Support/com.apple.TCC/TCC.db")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_returns_permission() {
        let result = check();
        assert!(result.is_ok());

        let perm = result.unwrap();
        assert_eq!(perm.id, "full_disk_access");
        assert!(perm.critical);
    }
}
