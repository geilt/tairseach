//! Location Permission (CoreLocation)
//!
//! Uses native Objective-C bindings to check and request location access.
//! This ensures Tairseach (not osascript) is registered for the permission.

use super::{Permission, PermissionError, PermissionStatus};

#[cfg(target_os = "macos")]
use objc2_core_location::CLLocationManager;
#[cfg(target_os = "macos")]
use std::time::Duration;

/// Check Location permission status using native API
#[cfg(target_os = "macos")]
pub fn check() -> Result<Permission, PermissionError> {
    let status = unsafe {
        let manager = CLLocationManager::new();
        
        // Check if location services are enabled (instance method)
        let services_enabled = manager.locationServicesEnabled();
        
        if !services_enabled {
            PermissionStatus::Denied
        } else {
            // Get authorization status (instance method)
            let raw_status = manager.authorizationStatus();
            
            match raw_status.0 {
                0 => PermissionStatus::NotDetermined,
                1 => PermissionStatus::Restricted,
                2 => PermissionStatus::Denied,
                3 => PermissionStatus::Granted,  // AuthorizedAlways
                4 => PermissionStatus::Granted,  // AuthorizedWhenInUse
                _ => PermissionStatus::Unknown,
            }
        }
    };

    Ok(Permission::new(
        "location",
        "Location",
        "Access to determine your geographic location",
        status,
        false,
    ))
}

/// Trigger registration by requesting access
#[cfg(target_os = "macos")]
pub fn trigger_registration() -> Result<(), PermissionError> {
    tracing::info!("Triggering location permission via native CLLocationManager...");
    
    let manager = unsafe { CLLocationManager::new() };
    
    tracing::info!("Calling requestWhenInUseAuthorization...");
    unsafe {
        manager.requestWhenInUseAuthorization();
    }
    
    // Give macOS time to show the dialog
    std::thread::sleep(Duration::from_millis(500));
    
    tracing::info!("Location registration trigger complete");
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn check() -> Result<Permission, PermissionError> {
    Ok(Permission::new(
        "location",
        "Location",
        "Access to determine your geographic location",
        PermissionStatus::Unknown,
        false,
    ))
}

#[cfg(not(target_os = "macos"))]
pub fn trigger_registration() -> Result<(), PermissionError> {
    Err(PermissionError::CheckFailed("Not supported on this platform".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_returns_permission() {
        let result = check();
        assert!(result.is_ok());
        let perm = result.unwrap();
        assert_eq!(perm.id, "location");
    }
}
