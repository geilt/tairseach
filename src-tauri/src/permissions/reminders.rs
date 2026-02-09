//! Reminders Permission (EventKit)
//!
//! Uses native Objective-C bindings to check and request reminders access.
//! This ensures Tairseach (not osascript) is registered for the permission.

use super::{Permission, PermissionError, PermissionStatus};

#[cfg(target_os = "macos")]
use objc2::runtime::Bool;
#[cfg(target_os = "macos")]
use objc2_event_kit::{EKEventStore, EKEntityType};
#[cfg(target_os = "macos")]
use objc2_foundation::NSError;
#[cfg(target_os = "macos")]
use block2::RcBlock;
#[cfg(target_os = "macos")]
use std::sync::{Arc, Mutex, Condvar};
#[cfg(target_os = "macos")]
use std::time::Duration;

/// Check Reminders permission status using native API
#[cfg(target_os = "macos")]
pub fn check() -> Result<Permission, PermissionError> {
    let status = unsafe {
        let entity_type = EKEntityType::Reminder;
        let raw_status = EKEventStore::authorizationStatusForEntityType(entity_type);
        
        match raw_status.0 {
            0 => PermissionStatus::NotDetermined,
            1 => PermissionStatus::Restricted,
            2 => PermissionStatus::Denied,
            3 => PermissionStatus::Granted,
            4 => PermissionStatus::Granted,  // FullAccess (macOS 14+)
            5 => PermissionStatus::Granted,  // WriteOnly (macOS 14+)
            _ => PermissionStatus::Unknown,
        }
    };

    Ok(Permission::new(
        "reminders",
        "Reminders",
        "Access to read and modify your reminders",
        status,
        false,
    ))
}

/// Trigger registration by requesting access
#[cfg(target_os = "macos")]
pub fn trigger_registration() -> Result<(), PermissionError> {
    tracing::info!("Triggering reminders permission via native EKEventStore...");
    
    let store = unsafe { EKEventStore::new() };
    
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let pair_clone = pair.clone();
    
    let block = RcBlock::new(move |granted: Bool, _error: *mut NSError| {
        let granted_bool = granted.as_bool();
        tracing::info!("Reminders permission callback: granted={}", granted_bool);
        
        let (lock, cvar) = &*pair_clone;
        if let Ok(mut done) = lock.lock() {
            *done = true;
            cvar.notify_one();
        }
    });
    
    tracing::info!("Calling requestFullAccessToRemindersWithCompletion...");
    unsafe {
        // Convert RcBlock to raw pointer as expected by the API
        let block_ptr = &*block as *const _ as *mut _;
        store.requestFullAccessToRemindersWithCompletion(block_ptr);
    }
    tracing::info!("Request sent, waiting for response...");
    
    let (lock, cvar) = &*pair;
    if let Ok(guard) = lock.lock() {
        let _ = cvar.wait_timeout(guard, Duration::from_secs(30));
    }
    
    tracing::info!("Reminders registration trigger complete");
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn check() -> Result<Permission, PermissionError> {
    Ok(Permission::new(
        "reminders",
        "Reminders",
        "Access to read and modify your reminders",
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
        assert_eq!(perm.id, "reminders");
    }
}
