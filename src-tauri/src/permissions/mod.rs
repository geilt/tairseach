//! macOS Permission Management
//!
//! This module provides a unified interface for checking and requesting
//! macOS system permissions required by Tairseach and OpenClaw agents.

mod accessibility;
mod automation;
mod calendar;
mod camera;
mod contacts;
mod full_disk;
mod location;
mod microphone;
mod photos;
mod reminders;
mod screen_recording;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::{Arc, Condvar, Mutex};

#[cfg(target_os = "macos")]
use std::time::Duration;

/// Permission authorization status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionStatus {
    Granted,
    Denied,
    NotDetermined,
    Restricted,
    Unknown,
}

impl Default for PermissionStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

/// A permission with its current status and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: PermissionStatus,
    pub critical: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_checked: Option<String>,
}

impl Permission {
    /// Create a new Permission with the given status
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        status: PermissionStatus,
        critical: bool,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            status,
            critical,
            last_checked: Some(Utc::now().to_rfc3339()),
        }
    }
}

/// Metadata about a permission type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub critical: bool,
    pub icon: String,
    pub system_pref_pane: String,
}

/// Permission identifiers
pub mod ids {
    pub const CONTACTS: &str = "contacts";
    pub const AUTOMATION: &str = "automation";
    pub const FULL_DISK_ACCESS: &str = "full_disk_access";
    pub const ACCESSIBILITY: &str = "accessibility";
    pub const SCREEN_RECORDING: &str = "screen_recording";
    pub const CALENDAR: &str = "calendar";
    pub const REMINDERS: &str = "reminders";
    pub const PHOTOS: &str = "photos";
    pub const CAMERA: &str = "camera";
    pub const MICROPHONE: &str = "microphone";
    pub const LOCATION: &str = "location";
}

/// Error type for permission operations
#[derive(Debug)]
#[allow(dead_code)]
pub enum PermissionError {
    UnknownPermission(String),
    CheckFailed(String),
    OpenSettingsFailed(String),
}

impl std::fmt::Display for PermissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PermissionError::UnknownPermission(id) => write!(f, "Unknown permission: {}", id),
            PermissionError::CheckFailed(msg) => write!(f, "Check failed: {}", msg),
            PermissionError::OpenSettingsFailed(msg) => write!(f, "Failed to open settings: {}", msg),
        }
    }
}

impl std::error::Error for PermissionError {}

fn get_pane_for_permission(permission_id: &str) -> Option<&'static str> {
    match permission_id {
        ids::CONTACTS => Some("Privacy_Contacts"),
        ids::AUTOMATION => Some("Privacy_Automation"),
        ids::FULL_DISK_ACCESS => Some("Privacy_AllFiles"),
        ids::ACCESSIBILITY => Some("Privacy_Accessibility"),
        ids::SCREEN_RECORDING => Some("Privacy_ScreenCapture"),
        ids::CALENDAR => Some("Privacy_Calendars"),
        ids::REMINDERS => Some("Privacy_Reminders"),
        ids::PHOTOS => Some("Privacy_Photos"),
        ids::CAMERA => Some("Privacy_Camera"),
        ids::MICROPHONE => Some("Privacy_Microphone"),
        ids::LOCATION => Some("Privacy_LocationServices"),
        _ => None,
    }
}

// ============================================================================
// Shared Permission Helpers
// ============================================================================

/// Map a raw authorization status integer (0–5) to PermissionStatus.
/// Works for most macOS frameworks (CNContactStore, EKEventStore, AVCaptureDevice,
/// PHPhotoLibrary, CLLocationManager) which all use the same 0–3(+) convention.
pub(crate) fn status_from_raw(raw: isize) -> PermissionStatus {
    match raw {
        0 => PermissionStatus::NotDetermined,
        1 => PermissionStatus::Restricted,
        2 => PermissionStatus::Denied,
        3 => PermissionStatus::Granted, // Authorized / AuthorizedAlways / FullAccess
        4 => PermissionStatus::Granted, // AuthorizedWhenInUse / Limited / FullAccess (macOS 14+)
        5 => PermissionStatus::Granted, // WriteOnly (macOS 14+)
        _ => PermissionStatus::Unknown,
    }
}

/// Check a permission by running a Swift snippet. Returns Granted if stdout
/// contains "authorized", NotDetermined otherwise. Used for accessibility and
/// screen recording which lack objc2 bindings.
pub(crate) fn check_via_swift(swift_code: &str, permission_name: &str) -> PermissionStatus {
    let output = Command::new("swift").args(["-e", swift_code]).output();

    match output {
        Ok(result) if result.status.success() => {
            let stdout = String::from_utf8_lossy(&result.stdout).trim().to_string();
            if stdout == "authorized" {
                PermissionStatus::Granted
            } else {
                PermissionStatus::NotDetermined
            }
        }
        Ok(result) => {
            tracing::warn!(
                "{} permission check failed: {}",
                permission_name,
                String::from_utf8_lossy(&result.stderr)
            );
            PermissionStatus::Unknown
        }
        Err(e) => {
            tracing::error!("Failed to run swift for {} check: {}", permission_name, e);
            PermissionStatus::Unknown
        }
    }
}

/// Create a non-macOS fallback Permission (Unknown status).
#[cfg(not(target_os = "macos"))]
pub(crate) fn non_macos_permission(
    id: &str,
    name: &str,
    description: &str,
    critical: bool,
) -> Result<Permission, PermissionError> {
    Ok(Permission::new(id, name, description, PermissionStatus::Unknown, critical))
}

/// Non-macOS fallback for trigger_registration.
#[cfg(not(target_os = "macos"))]
pub(crate) fn non_macos_trigger() -> Result<(), PermissionError> {
    Err(PermissionError::CheckFailed("Not supported on this platform".to_string()))
}

/// Callback synchronization pair type.
#[cfg(target_os = "macos")]
pub(crate) type CallbackPair = Arc<(Mutex<bool>, Condvar)>;

/// Helper to wait on a CallbackPair with a 30-second timeout.
#[cfg(target_os = "macos")]
pub(crate) fn wait_for_callback(pair: &CallbackPair) {
    let (lock, cvar): &(Mutex<bool>, Condvar) = &**pair;
    if let Ok(guard) = lock.lock() {
        let _ = cvar.wait_timeout(guard, Duration::from_secs(30));
    }
}

/// Create a new CallbackPair for async callback synchronization.
#[cfg(target_os = "macos")]
pub(crate) fn callback_pair() -> CallbackPair {
    Arc::new((Mutex::new(false), Condvar::new()))
}

/// Signal completion on a callback pair.
#[cfg(target_os = "macos")]
pub(crate) fn signal_callback(pair: &CallbackPair) {
    let (lock, cvar): &(Mutex<bool>, Condvar) = &**pair;
    if let Ok(mut done) = lock.lock() {
        *done = true;
        cvar.notify_one();
    }
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Check the status of a specific permission
#[tauri::command]
pub fn permissions_single_check(permission_id: &str) -> Result<Permission, String> {
    let result: Result<Permission, PermissionError> = match permission_id {
        ids::CONTACTS => contacts::check(),
        ids::AUTOMATION => automation::check(),
        ids::FULL_DISK_ACCESS => full_disk::check(),
        ids::ACCESSIBILITY => accessibility::check(),
        ids::SCREEN_RECORDING => screen_recording::check(),
        ids::CALENDAR => calendar::check(),
        ids::REMINDERS => reminders::check(),
        ids::PHOTOS => photos::check(),
        ids::CAMERA => camera::check(),
        ids::MICROPHONE => microphone::check(),
        ids::LOCATION => location::check(),
        _ => Err(PermissionError::UnknownPermission(permission_id.to_string())),
    };

    result.map_err(|e| e.to_string())
}

/// Check all permissions and return their current status
#[tauri::command]
pub fn check_all_permissions() -> Vec<Permission> {
    let permission_ids = [
        ids::CONTACTS,
        ids::AUTOMATION,
        ids::FULL_DISK_ACCESS,
        ids::ACCESSIBILITY,
        ids::SCREEN_RECORDING,
        ids::CALENDAR,
        ids::REMINDERS,
        ids::PHOTOS,
        ids::CAMERA,
        ids::MICROPHONE,
        ids::LOCATION,
    ];

    permission_ids
        .iter()
        .filter_map(|id| permissions_single_check(id).ok())
        .collect()
}

/// Get all permissions (legacy API - wraps check_all_permissions)
#[tauri::command]
pub async fn permissions_all_get() -> Result<Vec<Permission>, String> {
    Ok(check_all_permissions())
}

/// Grant permission (legacy API - not possible on macOS)
#[tauri::command]
pub async fn permissions_single_grant(_tool: String, _scope: String) -> Result<Permission, String> {
    Err("Use request_permission to open System Preferences. macOS does not allow programmatic granting.".to_string())
}

/// Revoke permission (legacy API - not possible on macOS)
#[tauri::command]
pub async fn permissions_single_revoke(_id: String) -> Result<(), String> {
    Err("Use System Preferences to manage permissions. macOS does not allow programmatic revocation.".to_string())
}

/// Request a permission by triggering the native prompt and/or opening System Preferences
#[tauri::command]
pub fn permissions_single_request(permission_id: &str) -> Result<(), String> {
    // Try to trigger the native permission prompt first
    tracing::info!("Requesting permission: {}", permission_id);
    
    let trigger_result = match permission_id {
        ids::CONTACTS => Some(contacts::trigger_registration()),
        ids::CALENDAR => Some(calendar::trigger_registration()),
        ids::REMINDERS => Some(reminders::trigger_registration()),
        ids::PHOTOS => Some(photos::trigger_registration()),
        ids::CAMERA => Some(camera::trigger_registration()),
        ids::MICROPHONE => Some(microphone::trigger_registration()),
        ids::LOCATION => Some(location::trigger_registration()),
        ids::SCREEN_RECORDING => Some(screen_recording::trigger_registration()),
        // These permissions don't have native prompt triggers
        ids::ACCESSIBILITY | ids::FULL_DISK_ACCESS | ids::AUTOMATION => None,
        _ => None,
    };
    
    if let Some(result) = trigger_result {
        match result {
            Ok(_) => tracing::info!("Permission registration triggered for: {}", permission_id),
            Err(e) => tracing::warn!("Permission registration trigger failed for {}: {}", permission_id, e),
        }
    }
    
    // Then open System Preferences to the appropriate pane
    let pane = get_pane_for_permission(permission_id)
        .ok_or_else(|| format!("Unknown permission: {}", permission_id))?;

    open_system_preferences_internal(pane)
}

/// Trigger permission registration (makes app appear in System Preferences)
#[tauri::command]
pub fn permissions_registration_trigger(permission_id: &str) -> Result<String, String> {
    let result = match permission_id {
        ids::CONTACTS => contacts::trigger_registration(),
        ids::CALENDAR => calendar::trigger_registration(),
        ids::REMINDERS => reminders::trigger_registration(),
        ids::PHOTOS => photos::trigger_registration(),
        ids::CAMERA => camera::trigger_registration(),
        ids::MICROPHONE => microphone::trigger_registration(),
        ids::LOCATION => location::trigger_registration(),
        ids::ACCESSIBILITY => {
            // Accessibility requires using accessibility APIs - just open settings
            return Ok("open_settings_required".to_string());
        }
        ids::SCREEN_RECORDING => screen_recording::trigger_registration(),
        ids::FULL_DISK_ACCESS => {
            // Full disk access must be granted manually
            return Ok("open_settings_required".to_string());
        }
        ids::AUTOMATION => {
            // Automation requires targeting a specific app - just open settings
            return Ok("open_settings_required".to_string());
        }
        _ => return Err(format!("Unknown permission: {}", permission_id)),
    };
    
    result
        .map(|_| "registration_triggered".to_string())
        .map_err(|e| e.to_string())
}

/// Open System Preferences to a specific privacy pane
#[tauri::command]
pub fn permissions_settings_open(pane: &str) -> Result<(), String> {
    open_system_preferences_internal(pane)
}

/// Get all permission definitions with metadata
#[tauri::command]
pub fn permissions_definitions_get() -> Vec<PermissionDefinition> {
    vec![
        PermissionDefinition {
            id: ids::CONTACTS.to_string(),
            name: "Contacts".to_string(),
            description: "Access to read and modify your contacts".to_string(),
            critical: true,
            icon: "📇".to_string(),
            system_pref_pane: "Privacy_Contacts".to_string(),
        },
        PermissionDefinition {
            id: ids::AUTOMATION.to_string(),
            name: "Automation".to_string(),
            description: "Control other applications via AppleScript".to_string(),
            critical: true,
            icon: "🤖".to_string(),
            system_pref_pane: "Privacy_Automation".to_string(),
        },
        PermissionDefinition {
            id: ids::FULL_DISK_ACCESS.to_string(),
            name: "Full Disk Access".to_string(),
            description: "Access to protected files and folders".to_string(),
            critical: true,
            icon: "💾".to_string(),
            system_pref_pane: "Privacy_AllFiles".to_string(),
        },
        PermissionDefinition {
            id: ids::ACCESSIBILITY.to_string(),
            name: "Accessibility".to_string(),
            description: "Control your computer using accessibility features".to_string(),
            critical: false,
            icon: "♿".to_string(),
            system_pref_pane: "Privacy_Accessibility".to_string(),
        },
        PermissionDefinition {
            id: ids::SCREEN_RECORDING.to_string(),
            name: "Screen Recording".to_string(),
            description: "Record the contents of your screen".to_string(),
            critical: false,
            icon: "🖥".to_string(),
            system_pref_pane: "Privacy_ScreenCapture".to_string(),
        },
        PermissionDefinition {
            id: ids::CALENDAR.to_string(),
            name: "Calendar".to_string(),
            description: "Access to read and modify your calendars".to_string(),
            critical: false,
            icon: "📅".to_string(),
            system_pref_pane: "Privacy_Calendars".to_string(),
        },
        PermissionDefinition {
            id: ids::REMINDERS.to_string(),
            name: "Reminders".to_string(),
            description: "Access to read and modify your reminders".to_string(),
            critical: false,
            icon: "🔔".to_string(),
            system_pref_pane: "Privacy_Reminders".to_string(),
        },
        PermissionDefinition {
            id: ids::PHOTOS.to_string(),
            name: "Photos".to_string(),
            description: "Access to your photo library".to_string(),
            critical: false,
            icon: "🖼".to_string(),
            system_pref_pane: "Privacy_Photos".to_string(),
        },
        PermissionDefinition {
            id: ids::CAMERA.to_string(),
            name: "Camera".to_string(),
            description: "Access to your camera".to_string(),
            critical: false,
            icon: "📷".to_string(),
            system_pref_pane: "Privacy_Camera".to_string(),
        },
        PermissionDefinition {
            id: ids::MICROPHONE.to_string(),
            name: "Microphone".to_string(),
            description: "Access to your microphone".to_string(),
            critical: false,
            icon: "🎤".to_string(),
            system_pref_pane: "Privacy_Microphone".to_string(),
        },
        PermissionDefinition {
            id: ids::LOCATION.to_string(),
            name: "Location".to_string(),
            description: "Access to your location".to_string(),
            critical: false,
            icon: "📍".to_string(),
            system_pref_pane: "Privacy_LocationServices".to_string(),
        },
    ]
}

/// Open System Preferences/Settings to a specific Privacy & Security pane
fn open_system_preferences_internal(pane: &str) -> Result<(), String> {
    let url = format!(
        "x-apple.systempreferences:com.apple.preference.security?{}",
        pane
    );

    tracing::info!("Opening System Preferences: {}", url);

    Command::new("open")
        .arg(&url)
        .status()
        .map_err(|e| format!("Failed to open System Preferences: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_definitions() {
        let defs = get_permission_definitions();
        assert_eq!(defs.len(), 11);
        assert!(defs.iter().any(|d| d.id == "contacts"));
        assert!(defs.iter().any(|d| d.id == "automation"));
        assert!(defs.iter().any(|d| d.id == "full_disk_access"));
        assert!(defs.iter().any(|d| d.id == "reminders"));
        assert!(defs.iter().any(|d| d.id == "photos"));
        assert!(defs.iter().any(|d| d.id == "camera"));
        assert!(defs.iter().any(|d| d.id == "microphone"));
        assert!(defs.iter().any(|d| d.id == "location"));
    }

    #[test]
    fn test_pane_mapping() {
        assert_eq!(get_pane_for_permission("contacts"), Some("Privacy_Contacts"));
        assert_eq!(get_pane_for_permission("automation"), Some("Privacy_Automation"));
        assert_eq!(get_pane_for_permission("reminders"), Some("Privacy_Reminders"));
        assert_eq!(get_pane_for_permission("location"), Some("Privacy_LocationServices"));
        assert_eq!(get_pane_for_permission("unknown"), None);
    }
}
