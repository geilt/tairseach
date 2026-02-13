//! Screen Recording Permission
//!
//! Uses CGPreflightScreenCaptureAccess/CGRequestScreenCaptureAccess via Swift.

use super::{check_via_swift, Permission, PermissionError};
use std::process::Command;

const SWIFT_CHECK: &str = r#"
import CoreGraphics
print(CGPreflightScreenCaptureAccess() ? "authorized" : "not_determined")
"#;

pub fn check() -> Result<Permission, PermissionError> {
    let status = check_via_swift(SWIFT_CHECK, "Screen Recording");
    Ok(Permission::new(
        "screen_recording", "Screen Recording",
        "Record the contents of your screen",
        status, false,
    ))
}

pub fn trigger_registration() -> Result<(), PermissionError> {
    tracing::info!("Triggering screen recording permission via CGRequestScreenCaptureAccess...");
    let swift_code = r#"
import CoreGraphics
let result = CGRequestScreenCaptureAccess()
print(result ? "granted" : "requested")
"#;
    let output = Command::new("swift")
        .args(["-e", swift_code])
        .output()
        .map_err(|e| PermissionError::CheckFailed(format!("Failed to run swift: {}", e)))?;

    if output.status.success() {
        tracing::info!("Screen recording registration result: {}", String::from_utf8_lossy(&output.stdout).trim());
    } else {
        tracing::warn!("Screen recording registration warning: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_check_returns_permission() {
        let perm = check().unwrap();
        assert_eq!(perm.id, "screen_recording");
        assert!(!perm.critical);
    }
}
