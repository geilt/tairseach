//! Accessibility Permission
//!
//! Uses AXIsProcessTrusted() via Swift to check authorization status.

use super::{check_via_swift, Permission, PermissionError};

const SWIFT_CHECK: &str = r#"
import ApplicationServices
print(AXIsProcessTrusted() ? "authorized" : "not_determined")
"#;

pub fn check() -> Result<Permission, PermissionError> {
    let status = check_via_swift(SWIFT_CHECK, "Accessibility");
    Ok(Permission::new(
        "accessibility", "Accessibility",
        "Control your computer using accessibility features",
        status, false,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_check_returns_permission() {
        let perm = check().unwrap();
        assert_eq!(perm.id, "accessibility");
        assert!(!perm.critical);
    }
}
