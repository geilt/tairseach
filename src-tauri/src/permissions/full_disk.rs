//! Full Disk Access Permission
//!
//! Checks by probing protected file system locations.

use super::{Permission, PermissionError, PermissionStatus};
use std::fs;
use std::path::PathBuf;

pub fn check() -> Result<Permission, PermissionError> {
    let status = get_authorization_status();
    Ok(Permission::new("full_disk_access", "Full Disk Access", "Access to protected files and folders", status, true))
}

fn get_authorization_status() -> PermissionStatus {
    let protected_paths = [
        PathBuf::from("/Library/Application Support/com.apple.TCC/TCC.db"),
    ];

    let user_tcc = dirs::home_dir()
        .map(|h| h.join("Library/Application Support/com.apple.TCC/TCC.db"));

    for path in protected_paths.iter().chain(user_tcc.as_ref().into_iter()) {
        if !path.exists() { continue; }
        match fs::read(path) {
            Ok(_) => return PermissionStatus::Granted,
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => return PermissionStatus::Denied,
            _ => {}
        }
    }

    PermissionStatus::Unknown
}

#[allow(dead_code)]
pub mod protected_paths {
    use std::path::PathBuf;
    pub fn mail_data() -> Option<PathBuf> { dirs::home_dir().map(|h| h.join("Library/Mail")) }
    pub fn messages_data() -> Option<PathBuf> { dirs::home_dir().map(|h| h.join("Library/Messages")) }
    pub fn safari_data() -> Option<PathBuf> { dirs::home_dir().map(|h| h.join("Library/Safari")) }
    pub fn system_tcc_db() -> PathBuf { PathBuf::from("/Library/Application Support/com.apple.TCC/TCC.db") }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_check_returns_permission() {
        let perm = check().unwrap();
        assert_eq!(perm.id, "full_disk_access");
        assert!(perm.critical);
    }
}
