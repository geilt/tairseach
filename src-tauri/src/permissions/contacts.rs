//! Contacts Permission (CNContactStore)
//!
//! Uses native Objective-C bindings to check and request contacts access.
//! This ensures Tairseach (not osascript) is registered for the permission.

use super::{Permission, PermissionError, PermissionStatus};

#[cfg(target_os = "macos")]
use objc2::runtime::Bool;
#[cfg(target_os = "macos")]
use objc2_contacts::{CNContactStore, CNEntityType};
#[cfg(target_os = "macos")]
use objc2_foundation::NSError;
#[cfg(target_os = "macos")]
use block2::StackBlock;
#[cfg(target_os = "macos")]
use std::sync::{Arc, Mutex, Condvar};
#[cfg(target_os = "macos")]
use std::time::Duration;

/// Check Contacts permission status using native API
#[cfg(target_os = "macos")]
pub fn check() -> Result<Permission, PermissionError> {
    let status = unsafe {
        let entity_type = CNEntityType(0); // CNEntityTypeContacts
        let raw_status = CNContactStore::authorizationStatusForEntityType(entity_type);
        
        // CNAuthorizationStatus is a newtype around isize
        match raw_status.0 {
            0 => PermissionStatus::NotDetermined,  // CNAuthorizationStatusNotDetermined
            1 => PermissionStatus::Restricted,     // CNAuthorizationStatusRestricted
            2 => PermissionStatus::Denied,         // CNAuthorizationStatusDenied
            3 => PermissionStatus::Granted,        // CNAuthorizationStatusAuthorized
            _ => PermissionStatus::Unknown,
        }
    };

    Ok(Permission::new(
        "contacts",
        "Contacts",
        "Access to read and modify your contacts",
        status,
        true,
    ))
}

/// Trigger registration by requesting access - this makes Tairseach appear in System Preferences
#[cfg(target_os = "macos")]
pub fn trigger_registration() -> Result<(), PermissionError> {
    tracing::info!("Triggering contacts permission via native CNContactStore...");
    
    // Create the contact store on the main thread
    let store = unsafe { CNContactStore::new() };
    
    // Use condition variable to wait for callback
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let pair_clone = pair.clone();
    
    // Create the completion handler block with the correct signature
    let block = StackBlock::new(move |granted: Bool, error: *mut NSError| {
        let granted_bool = granted.as_bool();
        if !error.is_null() {
            tracing::warn!("Contacts permission error (but this is expected on first request)");
        }
        tracing::info!("Contacts permission callback: granted={}", granted_bool);
        
        // Signal completion
        let (lock, cvar) = &*pair_clone;
        if let Ok(mut done) = lock.lock() {
            *done = true;
            cvar.notify_one();
        }
    });
    
    // Request access - CNEntityTypeContacts = 0
    // This MUST trigger the permission dialog
    let entity_type = CNEntityType(0);
    
    tracing::info!("Calling requestAccessForEntityType_completionHandler...");
    unsafe {
        store.requestAccessForEntityType_completionHandler(entity_type, &block);
    }
    tracing::info!("Request sent, waiting for response...");
    
    // Wait for the callback with timeout
    let (lock, cvar) = &*pair;
    if let Ok(guard) = lock.lock() {
        let _ = cvar.wait_timeout(guard, Duration::from_secs(30));
    }
    
    tracing::info!("Contacts registration trigger complete");
    Ok(())
}

/// Fallback for non-macOS (should not be called)
#[cfg(not(target_os = "macos"))]
pub fn check() -> Result<Permission, PermissionError> {
    Ok(Permission::new(
        "contacts",
        "Contacts",
        "Access to read and modify your contacts",
        PermissionStatus::Unknown,
        true,
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
        assert_eq!(perm.id, "contacts");
        assert!(perm.critical);
    }
}
