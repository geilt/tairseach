//! Contacts Permission (CNContactStore)

use super::{Permission, PermissionError};

#[cfg(target_os = "macos")]
mod native {
    use super::super::{callback_pair, signal_callback, status_from_raw, wait_for_callback, Permission, PermissionError};
    use objc2::runtime::Bool;
    use objc2_contacts::{CNContactStore, CNEntityType};
    use objc2_foundation::NSError;
    use block2::StackBlock;

    pub fn check() -> Result<Permission, PermissionError> {
        let status = unsafe {
            let entity_type = CNEntityType(0); // CNEntityTypeContacts
            status_from_raw(CNContactStore::authorizationStatusForEntityType(entity_type).0)
        };
        Ok(Permission::new("contacts", "Contacts", "Access to read and modify your contacts", status, true))
    }

    pub fn trigger_registration() -> Result<(), PermissionError> {
        tracing::info!("Triggering contacts permission via native CNContactStore...");
        let store = unsafe { CNContactStore::new() };
        let pair = callback_pair();
        let pair_clone = pair.clone();
        let block = StackBlock::new(move |_granted: Bool, _error: *mut NSError| { signal_callback(&pair_clone); });
        let entity_type = CNEntityType(0);
        unsafe { store.requestAccessForEntityType_completionHandler(entity_type, &block); }
        wait_for_callback(&pair);
        tracing::info!("Contacts registration trigger complete");
        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub fn check() -> Result<Permission, PermissionError> { native::check() }
#[cfg(target_os = "macos")]
pub fn trigger_registration() -> Result<(), PermissionError> { native::trigger_registration() }

#[cfg(not(target_os = "macos"))]
pub fn check() -> Result<Permission, PermissionError> {
    super::non_macos_permission("contacts", "Contacts", "Access to read and modify your contacts", true)
}
#[cfg(not(target_os = "macos"))]
pub fn trigger_registration() -> Result<(), PermissionError> { super::non_macos_trigger() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_check_returns_permission() {
        let perm = check().unwrap();
        assert_eq!(perm.id, "contacts");
        assert!(perm.critical);
    }
}
