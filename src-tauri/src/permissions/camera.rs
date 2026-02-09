//! Camera Permission (AVFoundation)
//!
//! Uses native Objective-C bindings to check and request camera access.
//! This ensures Tairseach (not osascript) is registered for the permission.

use super::{Permission, PermissionError, PermissionStatus};

#[cfg(target_os = "macos")]
use objc2::runtime::Bool;
#[cfg(target_os = "macos")]
use objc2_av_foundation::AVCaptureDevice;
#[cfg(target_os = "macos")]
use objc2_foundation::NSString;
#[cfg(target_os = "macos")]
use block2::RcBlock;
#[cfg(target_os = "macos")]
use std::sync::{Arc, Mutex, Condvar};
#[cfg(target_os = "macos")]
use std::time::Duration;

/// Check Camera permission status using native API
#[cfg(target_os = "macos")]
pub fn check() -> Result<Permission, PermissionError> {
    let status = unsafe {
        // AVMediaTypeVideo = "vide"
        let media_type = NSString::from_str("vide");
        let raw_status = AVCaptureDevice::authorizationStatusForMediaType(&media_type);
        
        match raw_status.0 {
            0 => PermissionStatus::NotDetermined,
            1 => PermissionStatus::Restricted,
            2 => PermissionStatus::Denied,
            3 => PermissionStatus::Granted,
            _ => PermissionStatus::Unknown,
        }
    };

    Ok(Permission::new(
        "camera",
        "Camera",
        "Access to use the camera for capturing images and video",
        status,
        false,
    ))
}

/// Trigger registration by requesting access
#[cfg(target_os = "macos")]
pub fn trigger_registration() -> Result<(), PermissionError> {
    tracing::info!("Triggering camera permission via native AVCaptureDevice...");
    
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let pair_clone = pair.clone();
    
    let block = RcBlock::new(move |granted: Bool| {
        let granted_bool = granted.as_bool();
        tracing::info!("Camera permission callback: granted={}", granted_bool);
        
        let (lock, cvar) = &*pair_clone;
        if let Ok(mut done) = lock.lock() {
            *done = true;
            cvar.notify_one();
        }
    });
    
    tracing::info!("Calling requestAccessForMediaType...");
    unsafe {
        // AVMediaTypeVideo = "vide"
        let media_type = NSString::from_str("vide");
        AVCaptureDevice::requestAccessForMediaType_completionHandler(&media_type, &block);
    }
    tracing::info!("Request sent, waiting for response...");
    
    let (lock, cvar) = &*pair;
    if let Ok(guard) = lock.lock() {
        let _ = cvar.wait_timeout(guard, Duration::from_secs(30));
    }
    
    tracing::info!("Camera registration trigger complete");
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn check() -> Result<Permission, PermissionError> {
    Ok(Permission::new(
        "camera",
        "Camera",
        "Access to use the camera for capturing images and video",
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
        assert_eq!(perm.id, "camera");
    }
}
