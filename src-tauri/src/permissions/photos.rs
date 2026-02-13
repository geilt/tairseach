//! Photos Permission (Photos.framework)

use super::{Permission, PermissionError};

#[cfg(target_os = "macos")]
mod native {
    use super::super::{callback_pair, signal_callback, status_from_raw, wait_for_callback, Permission, PermissionError};
    use objc2_photos::{PHPhotoLibrary, PHAccessLevel, PHAuthorizationStatus};
    use block2::RcBlock;

    pub fn check() -> Result<Permission, PermissionError> {
        let status = unsafe {
            let access_level = PHAccessLevel(1); // ReadWrite
            status_from_raw(PHPhotoLibrary::authorizationStatusForAccessLevel(access_level).0)
        };
        Ok(Permission::new("photos", "Photos", "Access to read and modify your photo library", status, false))
    }

    pub fn trigger_registration() -> Result<(), PermissionError> {
        tracing::info!("Triggering photos permission via native PHPhotoLibrary...");
        let pair = callback_pair();
        let pair_clone = pair.clone();
        let block = RcBlock::new(move |_status: PHAuthorizationStatus| { signal_callback(&pair_clone); });
        unsafe {
            let access_level = PHAccessLevel(1);
            PHPhotoLibrary::requestAuthorizationForAccessLevel_handler(access_level, &block);
        }
        wait_for_callback(&pair);
        tracing::info!("Photos registration trigger complete");
        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub fn check() -> Result<Permission, PermissionError> { native::check() }
#[cfg(target_os = "macos")]
pub fn trigger_registration() -> Result<(), PermissionError> { native::trigger_registration() }

#[cfg(not(target_os = "macos"))]
pub fn check() -> Result<Permission, PermissionError> {
    super::non_macos_permission("photos", "Photos", "Access to read and modify your photo library", false)
}
#[cfg(not(target_os = "macos"))]
pub fn trigger_registration() -> Result<(), PermissionError> { super::non_macos_trigger() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_check_returns_permission() {
        let perm = check().unwrap();
        assert_eq!(perm.id, "photos");
    }
}
