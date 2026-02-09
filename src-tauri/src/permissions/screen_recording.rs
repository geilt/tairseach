//! Screen Recording Permission
//!
//! Checks authorization status for screen capture/recording.
//! This permission is required for:
//! - Taking screenshots programmatically
//! - Screen recording
//! - Screen sharing

use super::{Permission, PermissionError, PermissionStatus};
use std::process::Command;

/// Check Screen Recording permission status
pub fn check() -> Result<Permission, PermissionError> {
    let status = get_authorization_status();

    Ok(Permission::new(
        "screen_recording",
        "Screen Recording",
        "Record the contents of your screen",
        status,
        false,
    ))
}

/// Trigger screen recording registration by attempting a screen capture
/// This makes Tairseach appear in System Preferences > Privacy > Screen Recording
pub fn trigger_registration() -> Result<(), PermissionError> {
    tracing::info!("Triggering screen recording permission via CGRequestScreenCaptureAccess...");
    
    // Use Swift to call CGRequestScreenCaptureAccess() which:
    // 1. Registers the app for Screen Recording permissions
    // 2. Shows the permission prompt if not already determined
    let swift_code = r#"
import CoreGraphics

// This registers the app and may show a prompt
let result = CGRequestScreenCaptureAccess()
print(result ? "granted" : "requested")
"#;

    let output = Command::new("swift")
        .args(["-e", swift_code])
        .output()
        .map_err(|e| PermissionError::CheckFailed(format!("Failed to run swift: {}", e)))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        tracing::info!("Screen recording registration result: {}", stdout);
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("Screen recording registration completed with warning: {}", stderr);
        // Still return Ok - the registration attempt was made
        Ok(())
    }
}

/// Check screen recording permission using CGPreflightScreenCaptureAccess
fn get_authorization_status() -> PermissionStatus {
    let swift_code = r#"
import CoreGraphics
print(CGPreflightScreenCaptureAccess() ? "authorized" : "not_determined")
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
                    PermissionStatus::NotDetermined
                }
            } else {
                tracing::warn!(
                    "Screen recording permission check failed: {}",
                    String::from_utf8_lossy(&result.stderr)
                );
                PermissionStatus::Unknown
            }
        }
        Err(e) => {
            tracing::error!("Failed to run swift for screen recording check: {}", e);
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
        assert_eq!(perm.id, "screen_recording");
        assert!(!perm.critical);
    }
}
