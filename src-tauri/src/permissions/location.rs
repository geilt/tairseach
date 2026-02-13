//! Location Permission (CoreLocation)

use super::{Permission, PermissionError};

#[cfg(target_os = "macos")]
mod native {
    use super::super::{status_from_raw, Permission, PermissionError, PermissionStatus};
    use objc2_core_location::CLLocationManager;
    use std::time::Duration;

    #[allow(deprecated)]
    pub fn check() -> Result<Permission, PermissionError> {
        let status = unsafe {
            let manager = CLLocationManager::new();
            if !manager.locationServicesEnabled() {
                PermissionStatus::Denied
            } else {
                status_from_raw(manager.authorizationStatus().0 as isize)
            }
        };
        Ok(Permission::new("location", "Location", "Access to determine your geographic location", status, false))
    }

    pub fn trigger_registration() -> Result<(), PermissionError> {
        tracing::info!("Triggering location permission via native CLLocationManager...");
        let manager = unsafe { CLLocationManager::new() };
        unsafe { manager.requestWhenInUseAuthorization(); }
        std::thread::sleep(Duration::from_millis(500));
        tracing::info!("Location registration trigger complete");
        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub fn check() -> Result<Permission, PermissionError> { native::check() }
#[cfg(target_os = "macos")]
pub fn trigger_registration() -> Result<(), PermissionError> { native::trigger_registration() }

#[cfg(not(target_os = "macos"))]
pub fn check() -> Result<Permission, PermissionError> {
    super::non_macos_permission("location", "Location", "Access to determine your geographic location", false)
}
#[cfg(not(target_os = "macos"))]
pub fn trigger_registration() -> Result<(), PermissionError> { super::non_macos_trigger() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_check_returns_permission() {
        let perm = check().unwrap();
        assert_eq!(perm.id, "location");
    }
}
