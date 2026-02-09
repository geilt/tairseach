//! Photos Permission (Photos.framework)
//!
//! Uses native Objective-C bindings to check and request photos library access.
//! This ensures Tairseach (not osascript) is registered for the permission.

use super::{Permission, PermissionError, PermissionStatus};

#[cfg(target_os = "macos")]
use objc2_photos::{PHPhotoLibrary, PHAccessLevel, PHAuthorizationStatus};
#[cfg(target_os = "macos")]
use block2::RcBlock;
#[cfg(target_os = "macos")]
use std::sync::{Arc, Mutex, Condvar};
#[cfg(target_os = "macos")]
use std::time::Duration;

/// Check Photos permission status using native API
#[cfg(target_os = "macos")]
pub fn check() -> Result<Permission, PermissionError> {
    let status = unsafe {
        let access_level = PHAccessLevel(1); // ReadWrite
        let raw_status = PHPhotoLibrary::authorizationStatusForAccessLevel(access_level);
        
        match raw_status.0 {
            0 => PermissionStatus::NotDetermined,
            1 => PermissionStatus::Restricted,
            2 => PermissionStatus::Denied,
            3 => PermissionStatus::Granted,
            4 => PermissionStatus::Granted,  // Limited
            _ => PermissionStatus::Unknown,
        }
    };

    Ok(Permission::new(
        "photos",
        "Photos",
        "Access to read and modify your photo library",
        status,
        false,
    ))
}

/// Trigger registration by requesting access
#[cfg(target_os = "macos")]
pub fn trigger_registration() -> Result<(), PermissionError> {
    tracing::info!("Triggering photos permission via native PHPhotoLibrary...");
    
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let pair_clone = pair.clone();
    
    let block = RcBlock::new(move |status: PHAuthorizationStatus| {
        tracing::info!("Photos permission callback: status={}", status.0);
        
        let (lock, cvar) = &*pair_clone;
        if let Ok(mut done) = lock.lock() {
            *done = true;
            cvar.notify_one();
        }
    });
    
    tracing::info!("Calling requestAuthorizationForAccessLevel...");
    unsafe {
        let access_level = PHAccessLevel(1); // ReadWrite
        PHPhotoLibrary::requestAuthorizationForAccessLevel_handler(access_level, &block);
    }
    tracing::info!("Request sent, waiting for response...");
    
    let (lock, cvar) = &*pair;
    if let Ok(guard) = lock.lock() {
        let _ = cvar.wait_timeout(guard, Duration::from_secs(30));
    }
    
    tracing::info!("Photos registration trigger complete");
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn check() -> Result<Permission, PermissionError> {
    Ok(Permission::new(
        "photos",
        "Photos",
        "Access to read and modify your photo library",
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
        assert_eq!(perm.id, "photos");
    }
}
