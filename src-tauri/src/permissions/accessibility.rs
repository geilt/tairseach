//! Accessibility Permission
//!
//! Checks authorization status for Accessibility features.
//! This permission is required for:
//! - Sending keystrokes to other applications
//! - Reading screen content programmatically
//! - UI automation and testing

use super::{Permission, PermissionError, PermissionStatus};
use std::process::Command;

/// Check Accessibility permission status
pub fn check() -> Result<Permission, PermissionError> {
    let status = get_authorization_status();

    Ok(Permission::new(
        "accessibility",
        "Accessibility",
        "Control your computer using accessibility features",
        status,
        false, // Not critical for core functionality
    ))
}

/// Check accessibility permission using AXIsProcessTrusted
fn get_authorization_status() -> PermissionStatus {
    // Use Swift to call AXIsProcessTrusted()
    let swift_code = r#"
import ApplicationServices
print(AXIsProcessTrusted() ? "authorized" : "not_determined")
"#;

    let output = Command::new("swift")
        .args(["-e", swift_code])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                let stdout = String::from_utf8_lossy(&result.stdout).trim().to_string();
                if stdout == "authorized" {
                    PermissionStatus::Granted
                } else {
                    // AXIsProcessTrusted can't distinguish between denied and not determined
                    PermissionStatus::NotDetermined
                }
            } else {
                tracing::warn!(
                    "Accessibility permission check failed: {}",
                    String::from_utf8_lossy(&result.stderr)
                );
                PermissionStatus::Unknown
            }
        }
        Err(e) => {
            tracing::error!("Failed to run swift for accessibility check: {}", e);
            PermissionStatus::Unknown
        }
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
        assert_eq!(perm.id, "accessibility");
        assert!(!perm.critical);
    }
}
