//! Reminders Permission (EventKit)

use super::{Permission, PermissionError};

#[cfg(target_os = "macos")]
mod native {
    use super::super::{callback_pair, signal_callback, status_from_raw, wait_for_callback, Permission, PermissionError};
    use objc2::runtime::Bool;
    use objc2_event_kit::{EKEventStore, EKEntityType};
    use objc2_foundation::NSError;
    use block2::RcBlock;

    pub fn check() -> Result<Permission, PermissionError> {
        let status = unsafe {
            status_from_raw(EKEventStore::authorizationStatusForEntityType(EKEntityType::Reminder).0)
        };
        Ok(Permission::new("reminders", "Reminders", "Access to read and modify your reminders", status, false))
    }

    pub fn trigger_registration() -> Result<(), PermissionError> {
        tracing::info!("Triggering reminders permission via native EKEventStore...");
        let store = unsafe { EKEventStore::new() };
        let pair = callback_pair();
        let pair_clone = pair.clone();
        let block = RcBlock::new(move |_granted: Bool, _error: *mut NSError| { signal_callback(&pair_clone); });
        unsafe {
            let block_ptr = &*block as *const _ as *mut _;
            store.requestFullAccessToRemindersWithCompletion(block_ptr);
        }
        wait_for_callback(&pair);
        tracing::info!("Reminders registration trigger complete");
        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub fn check() -> Result<Permission, PermissionError> { native::check() }
#[cfg(target_os = "macos")]
pub fn trigger_registration() -> Result<(), PermissionError> { native::trigger_registration() }

#[cfg(not(target_os = "macos"))]
pub fn check() -> Result<Permission, PermissionError> {
    super::non_macos_permission("reminders", "Reminders", "Access to read and modify your reminders", false)
}
#[cfg(not(target_os = "macos"))]
pub fn trigger_registration() -> Result<(), PermissionError> { super::non_macos_trigger() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_check_returns_permission() {
        let perm = check().unwrap();
        assert_eq!(perm.id, "reminders");
    }
}
