//! Automation Permission (AppleEvents)
//!
//! Checks authorization status for sending AppleEvents to other applications.
//! This is required for AppleScript/osascript automation.

use super::{Permission, PermissionError, PermissionStatus};
use std::process::Command;

/// Check Automation permission status
///
/// Note: Automation permissions are per-target-app, not global.
/// We check by attempting a benign AppleScript to System Events,
/// which is commonly needed for automation tasks.
pub fn check() -> Result<Permission, PermissionError> {
    let status = get_authorization_status();

    Ok(Permission::new(
        "automation",
        "Automation",
        "Control other applications via AppleScript",
        status,
        true, // Critical for OpenClaw automation
    ))
}

/// Check automation permission by attempting a benign AppleScript
fn get_authorization_status() -> PermissionStatus {
    // Try a simple, non-destructive AppleScript
    let script = r#"tell application "System Events" to return name"#;

    let output = Command::new("osascript").args(["-e", script]).output();

    match output {
        Ok(result) => {
            if result.status.success() {
                PermissionStatus::Granted
            } else {
                let stderr = String::from_utf8_lossy(&result.stderr);
                if stderr.contains("-1743") || stderr.contains("not allowed") {
                    PermissionStatus::Denied
                } else if stderr.contains("not permitted") {
                    PermissionStatus::NotDetermined
                } else {
                    tracing::warn!("Unknown automation error: {}", stderr);
                    PermissionStatus::Unknown
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to run osascript: {}", e);
            PermissionStatus::Unknown
        }
    }
}

/// Common bundle IDs for automation targets
pub mod targets {
    pub const SYSTEM_EVENTS: &str = "com.apple.systemevents";
    pub const FINDER: &str = "com.apple.finder";
    pub const TERMINAL: &str = "com.apple.Terminal";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_returns_permission() {
        let result = check();
        assert!(result.is_ok());

        let perm = result.unwrap();
        assert_eq!(perm.id, "automation");
        assert!(perm.critical);
    }
}
