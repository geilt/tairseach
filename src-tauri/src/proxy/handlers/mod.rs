//! Handler Registry and Permission Middleware
//!
//! Dispatches JSON-RPC requests to appropriate handlers after checking permissions.

pub mod auth;
pub mod automation;
pub mod calendar;
pub mod config;
pub mod contacts;
pub mod files;
pub mod gmail;
pub mod google_calendar;
pub mod jira;
pub mod location;
pub mod onepassword;
pub mod oura;
pub mod permissions;
pub mod reminders;
pub mod screen;

use serde_json::Value;
use tracing::{debug, warn};

use super::protocol::{JsonRpcRequest, JsonRpcResponse};

/// Permission status
#[derive(Debug, Clone, PartialEq)]
pub enum PermissionStatus {
    Granted,
    Denied,
    NotDetermined,
    Restricted,
    Unknown,
}

impl PermissionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PermissionStatus::Granted => "granted",
            PermissionStatus::Denied => "denied",
            PermissionStatus::NotDetermined => "not_determined",
            PermissionStatus::Restricted => "restricted",
            PermissionStatus::Unknown => "unknown",
        }
    }
}

/// Mapping of methods to required permissions
fn required_permission(method: &str) -> Option<&'static str> {
    match method {
        // Contacts
        "contacts.list" | "contacts.search" | "contacts.get" |
        "contacts.create" | "contacts.update" | "contacts.delete" => Some("contacts"),
        
        // Calendar
        "calendar.list" | "calendar.events" | "calendar.getEvent" |
        "calendar.createEvent" | "calendar.updateEvent" | "calendar.deleteEvent" => Some("calendar"),
        
        // Reminders
        "reminders.lists" | "reminders.list" | "reminders.create" |
        "reminders.complete" | "reminders.delete" => Some("reminders"),
        
        // Location
        "location.get" | "location.watch" => Some("location"),
        
        // Photos
        "photos.albums" | "photos.list" | "photos.get" => Some("photos"),
        
        // Screen capture
        "screen.capture" | "screen.windows" => Some("screen_recording"),
        
        // Automation
        "automation.run" => Some("automation"),
        "automation.click" | "automation.type" => Some("accessibility"),
        
        // Files (Full Disk Access)
        "files.read" | "files.write" | "files.list" => Some("full_disk_access"),
        
        // Camera
        "camera.capture" | "camera.start" | "camera.stop" => Some("camera"),
        
        // Microphone
        "microphone.record" | "microphone.start" | "microphone.stop" => Some("microphone"),
        
        // Auth methods don't require macOS permissions (socket security suffices)
        "auth.status" | "auth.providers" | "auth.accounts" | "auth.list" |
        "auth.token" | "auth.get" | "auth.refresh" | "auth.revoke" |
        "auth.store" | "auth.import" | "auth.gogPassphrase" => None,
        
        // Permission methods don't require special permissions
        "permissions.check" | "permissions.list" | "permissions.request" => None,
        
        // Config methods don't require special permissions
        "config.get" | "config.set" => None,
        
        // Server control methods
        "server.status" | "server.shutdown" => None,
        
        _ => None,
    }
}

/// Check permission status for a given permission name
async fn check_permission_status(permission: &str) -> PermissionStatus {
    // This calls into the existing permissions module
    // For now, we'll implement a simple check - in the full implementation
    // this would call the actual permission checking code
    
    match permission {
        "contacts" => check_contacts_permission().await,
        "calendar" => check_calendar_permission().await,
        "reminders" => check_reminders_permission().await,
        "location" => check_location_permission().await,
        "photos" => check_photos_permission().await,
        "camera" => check_camera_permission().await,
        "microphone" => check_microphone_permission().await,
        "screen_recording" => check_screen_recording_permission().await,
        "accessibility" => check_accessibility_permission().await,
        "full_disk_access" => check_full_disk_access_permission().await,
        "automation" => check_automation_permission().await,
        _ => PermissionStatus::Unknown,
    }
}

// Permission checks - call into the actual permissions module
use crate::permissions as sys_perms;

fn convert_permission_status(status: sys_perms::PermissionStatus) -> PermissionStatus {
    match status {
        sys_perms::PermissionStatus::Granted => PermissionStatus::Granted,
        sys_perms::PermissionStatus::Denied => PermissionStatus::Denied,
        sys_perms::PermissionStatus::NotDetermined => PermissionStatus::NotDetermined,
        sys_perms::PermissionStatus::Restricted => PermissionStatus::Restricted,
        sys_perms::PermissionStatus::Unknown => PermissionStatus::Unknown,
    }
}

async fn check_contacts_permission() -> PermissionStatus {
    match sys_perms::check_permission(sys_perms::ids::CONTACTS) {
        Ok(perm) => convert_permission_status(perm.status),
        Err(_) => PermissionStatus::Unknown,
    }
}

async fn check_calendar_permission() -> PermissionStatus {
    match sys_perms::check_permission(sys_perms::ids::CALENDAR) {
        Ok(perm) => convert_permission_status(perm.status),
        Err(_) => PermissionStatus::Unknown,
    }
}

async fn check_reminders_permission() -> PermissionStatus {
    match sys_perms::check_permission(sys_perms::ids::REMINDERS) {
        Ok(perm) => convert_permission_status(perm.status),
        Err(_) => PermissionStatus::Unknown,
    }
}

async fn check_location_permission() -> PermissionStatus {
    match sys_perms::check_permission(sys_perms::ids::LOCATION) {
        Ok(perm) => convert_permission_status(perm.status),
        Err(_) => PermissionStatus::Unknown,
    }
}

async fn check_photos_permission() -> PermissionStatus {
    match sys_perms::check_permission(sys_perms::ids::PHOTOS) {
        Ok(perm) => convert_permission_status(perm.status),
        Err(_) => PermissionStatus::Unknown,
    }
}

async fn check_camera_permission() -> PermissionStatus {
    match sys_perms::check_permission(sys_perms::ids::CAMERA) {
        Ok(perm) => convert_permission_status(perm.status),
        Err(_) => PermissionStatus::Unknown,
    }
}

async fn check_microphone_permission() -> PermissionStatus {
    match sys_perms::check_permission(sys_perms::ids::MICROPHONE) {
        Ok(perm) => convert_permission_status(perm.status),
        Err(_) => PermissionStatus::Unknown,
    }
}

async fn check_screen_recording_permission() -> PermissionStatus {
    match sys_perms::check_permission(sys_perms::ids::SCREEN_RECORDING) {
        Ok(perm) => convert_permission_status(perm.status),
        Err(_) => PermissionStatus::Unknown,
    }
}

async fn check_accessibility_permission() -> PermissionStatus {
    match sys_perms::check_permission(sys_perms::ids::ACCESSIBILITY) {
        Ok(perm) => convert_permission_status(perm.status),
        Err(_) => PermissionStatus::Unknown,
    }
}

async fn check_full_disk_access_permission() -> PermissionStatus {
    match sys_perms::check_permission(sys_perms::ids::FULL_DISK_ACCESS) {
        Ok(perm) => convert_permission_status(perm.status),
        Err(_) => PermissionStatus::Unknown,
    }
}

async fn check_automation_permission() -> PermissionStatus {
    match sys_perms::check_permission(sys_perms::ids::AUTOMATION) {
        Ok(perm) => convert_permission_status(perm.status),
        Err(_) => PermissionStatus::Unknown,
    }
}

/// Handler registry - dispatches requests to appropriate handlers
pub struct HandlerRegistry {
    // Capability router for manifest-based routing (optional for backward compatibility)
    router: Option<std::sync::Arc<crate::router::CapabilityRouter>>,
}

impl HandlerRegistry {
    pub fn new() -> Self {
        Self { router: None }
    }

    pub fn with_router(router: std::sync::Arc<crate::router::CapabilityRouter>) -> Self {
        Self {
            router: Some(router),
        }
    }
    
    /// Handle a JSON-RPC request
    pub async fn handle(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone().unwrap_or(Value::Null);
        
        debug!("Handling method: {}", request.method);
        
        // Try manifest-based routing first (if router is available)
        if let Some(router) = &self.router {
            let response = router.route(request).await;
            // If router found the method, return its response
            // (method_not_found means it wasn't in the manifest registry)
            if !is_method_not_found(&response) {
                return response;
            }
            // Otherwise, fall through to legacy routing
        }
        
        // Legacy routing: check permissions for known methods
        if let Some(required) = required_permission(&request.method) {
            let status = check_permission_status(required).await;
            
            if status != PermissionStatus::Granted {
                warn!(
                    "Permission denied for method {}: {} is {}",
                    request.method,
                    required,
                    status.as_str()
                );
                return JsonRpcResponse::permission_denied(id, required, status.as_str());
            }
        }
        
        // Dispatch to handler
        let (namespace, action) = request.parse_method();
        
        match namespace {
            "auth" => auth::handle(action, &request.params, id).await,
            "permissions" => permissions::handle(action, &request.params, id).await,
            "contacts" => contacts::handle(action, &request.params, id).await,
            "calendar" => calendar::handle(action, &request.params, id).await,
            "reminders" => reminders::handle(action, &request.params, id).await,
            "location" => location::handle(action, &request.params, id).await,
            "screen" => screen::handle(action, &request.params, id).await,
            "files" => files::handle(action, &request.params, id).await,
            "automation" => automation::handle(action, &request.params, id).await,
            "config" => config::handle(action, &request.params, id).await,
            "gmail" => gmail::handle(action, &request.params, id).await,
            "gcalendar" => google_calendar::handle(action, &request.params, id).await,
            "op" | "onepassword" => onepassword::handle(action, &request.params, id).await,
            "oura" => oura::handle(action, &request.params, id).await,
            "jira" => jira::handle(action, &request.params, id).await,
            "server" => self.handle_server(action, &request.params, id).await,
            _ => JsonRpcResponse::method_not_found(id, &request.method),
        }
    }
    
    /// Handle server control methods
    async fn handle_server(&self, action: &str, _params: &Value, id: Value) -> JsonRpcResponse {
        match action {
            "status" => JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "status": "running",
                    "version": env!("CARGO_PKG_VERSION"),
                }),
            ),
            "shutdown" => {
                // TODO: Implement graceful shutdown
                JsonRpcResponse::success(id, serde_json::json!({"message": "Shutdown initiated"}))
            }
            _ => JsonRpcResponse::method_not_found(id, &format!("server.{}", action)),
        }
    }
}

impl Default for HandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a response is a "method not found" error
fn is_method_not_found(response: &JsonRpcResponse) -> bool {
    if let Some(ref error) = response.error {
        error.code == -32601
    } else {
        false
    }
}
