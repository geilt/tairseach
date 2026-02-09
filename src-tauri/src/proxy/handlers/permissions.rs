//! Permissions Handler
//!
//! Handles permission-related JSON-RPC methods.

use serde_json::Value;

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
        _ => JsonRpcResponse::method_not_found(id, &format!("permissions.{}", action)),
    }
}

/// Check status of a specific permission
async fn handle_check(params: &Value, id: Value) -> JsonRpcResponse {
    let permission = match params.get("permission").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing 'permission' parameter");
        }
    };
    
    if !ALL_PERMISSIONS.contains(&permission) {
        return JsonRpcResponse::invalid_params(
            id,
            format!("Unknown permission: {}. Valid: {:?}", permission, ALL_PERMISSIONS),
        );
    }
    
    let status = check_permission_status(permission).await;
    
    JsonRpcResponse::success(
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
    
    JsonRpcResponse::success(
        id,
        serde_json::json!({
            "permissions": permissions,
            "total": ALL_PERMISSIONS.len(),
        }),
    )
}

/// Request a permission (triggers UI prompt or opens settings)
async fn handle_request(params: &Value, id: Value) -> JsonRpcResponse {
    let permission = match params.get("permission").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing 'permission' parameter");
        }
    };
    
    if !ALL_PERMISSIONS.contains(&permission) {
        return JsonRpcResponse::invalid_params(
            id,
            format!("Unknown permission: {}", permission),
        );
    }
    
    // TODO: Call the actual permission request functions from crate::permissions
    // For now, return a placeholder response
    JsonRpcResponse::success(
        id,
        serde_json::json!({
            "permission": permission,
            "action": "request_initiated",
            "message": "Permission request has been initiated. Check app UI for prompt.",
        }),
    )
}
