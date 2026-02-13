//! Automation Permission (AppleEvents)
//!
//! Checks by attempting a benign AppleScript to System Events.

use super::{Permission, PermissionError, PermissionStatus};
use std::process::Command;

pub fn check() -> Result<Permission, PermissionError> {
    let script = r#"tell application "System Events" to return name"#;
    let output = Command::new("osascript").args(["-e", script]).output();

    let status = match output {
        Ok(result) if result.status.success() => PermissionStatus::Granted,
        Ok(result) => {
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
        Err(e) => {
            tracing::error!("Failed to run osascript: {}", e);
            PermissionStatus::Unknown
        }
    };

    Ok(Permission::new("automation", "Automation", "Control other applications via AppleScript", status, true))
}

#[allow(dead_code)]
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
        let perm = check().unwrap();
        assert_eq!(perm.id, "automation");
        assert!(perm.critical);
    }
}
