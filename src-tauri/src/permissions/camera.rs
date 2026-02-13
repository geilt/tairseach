//! Camera Permission (AVFoundation)

use super::{Permission, PermissionError};

#[cfg(target_os = "macos")]
mod native {
    use super::super::{callback_pair, signal_callback, status_from_raw, wait_for_callback, Permission, PermissionError};
    use objc2::runtime::Bool;
    use objc2_av_foundation::AVCaptureDevice;
    use objc2_foundation::NSString;
    use block2::RcBlock;

    const MEDIA_TYPE: &str = "vide"; // AVMediaTypeVideo

    pub fn check() -> Result<Permission, PermissionError> {
        let status = unsafe {
            let media_type = NSString::from_str(MEDIA_TYPE);
            status_from_raw(AVCaptureDevice::authorizationStatusForMediaType(&media_type).0)
        };
        Ok(Permission::new("camera", "Camera", "Access to use the camera for capturing images and video", status, false))
    }

    pub fn trigger_registration() -> Result<(), PermissionError> {
        tracing::info!("Triggering camera permission via native AVCaptureDevice...");
        let pair = callback_pair();
        let pair_clone = pair.clone();
        let block = RcBlock::new(move |_granted: Bool| { signal_callback(&pair_clone); });
        unsafe {
            let media_type = NSString::from_str(MEDIA_TYPE);
            AVCaptureDevice::requestAccessForMediaType_completionHandler(&media_type, &block);
        }
        wait_for_callback(&pair);
        tracing::info!("Camera registration trigger complete");
        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub fn check() -> Result<Permission, PermissionError> { native::check() }
#[cfg(target_os = "macos")]
pub fn trigger_registration() -> Result<(), PermissionError> { native::trigger_registration() }

#[cfg(not(target_os = "macos"))]
pub fn check() -> Result<Permission, PermissionError> {
    super::non_macos_permission("camera", "Camera", "Access to use the camera for capturing images and video", false)
}
#[cfg(not(target_os = "macos"))]
pub fn trigger_registration() -> Result<(), PermissionError> { super::non_macos_trigger() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_check_returns_permission() {
        let perm = check().unwrap();
        assert_eq!(perm.id, "camera");
    }
}
