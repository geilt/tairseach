//! Permissions Handler
//!
//! Handles permission-related JSON-RPC methods.

use serde_json::Value;

use super::common::*;
use super::super::protocol::JsonRpcResponse;
use super::{check_permission_status, PermissionStatus};

/// All available permissions
const ALL_PERMISSIONS: &[&str] = &[
    "contacts",
    "calendar",
    "reminders",
    "location",
    "photos",
    "camera",
    "microphone",
    "screen_recording",
    "accessibility",
    "full_disk_access",
    "automation",
];

/// Handle permission-related methods
pub async fn handle(action: &str, params: &Value, id: Value) -> JsonRpcResponse {
    match action {
        "check" => handle_check(params, id).await,
        "list" => handle_list(id).await,
        "request" => handle_request(params, id).await,
        _ => method_not_found(id, &format!("permissions.{}", action)),
    }
}

/// Check status of a specific permission
async fn handle_check(params: &Value, id: Value) -> JsonRpcResponse {
    let permission = match require_string(params, "permission", &id) {
        Ok(p) => p,
        Err(response) => return response,
    };
    
    if !ALL_PERMISSIONS.contains(&permission) {
        return invalid_params(
            id,
            format!("Unknown permission: {}. Valid: {:?}", permission, ALL_PERMISSIONS),
        );
    }
    
    let status = check_permission_status(permission).await;
    
    ok(
        id,
        serde_json::json!({
            "permission": permission,
            "status": status.as_str(),
            "granted": status == PermissionStatus::Granted,
        }),
    )
}

/// List all permissions and their status
async fn handle_list(id: Value) -> JsonRpcResponse {
    let mut permissions = Vec::new();
    
    for &permission in ALL_PERMISSIONS {
        let status = check_permission_status(permission).await;
        permissions.push(serde_json::json!({
            "permission": permission,
            "status": status.as_str(),
            "granted": status == PermissionStatus::Granted,
        }));
    }
    
    ok(
        id,
        serde_json::json!({
            "permissions": permissions,
            "total": ALL_PERMISSIONS.len(),
        }),
    )
}

/// Request a permission (triggers UI prompt or opens settings)
async fn handle_request(params: &Value, id: Value) -> JsonRpcResponse {
    let permission = match require_string(params, "permission", &id) {
        Ok(p) => p,
        Err(response) => return response,
    };
    
    if !ALL_PERMISSIONS.contains(&permission) {
        return invalid_params(id, format!("Unknown permission: {}", permission));
    }
    
    // Call the permission request function from crate::permissions
    // This triggers the macOS permission dialog and/or opens System Settings
    match crate::permissions::permissions_single_request(permission) {
        Ok(_) => ok(
            id,
            serde_json::json!({
                "permission": permission,
                "action": "request_triggered",
                "message": "Permission dialog triggered. Check for system prompt or Settings opened.",
            }),
        ),
        Err(e) => generic_error(id, format!("Failed to request permission: {}", e)),
    }
}
