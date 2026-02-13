//! Config Handler
//!
//! Handles config-related JSON-RPC methods for reading/writing
//! the OpenClaw configuration file (~/.openclaw/openclaw.json).

use serde_json::Value;
use std::path::PathBuf;
use tracing::{info, warn};

use super::common::*;
use super::super::protocol::JsonRpcResponse;

/// Handle config-related methods
pub async fn handle(action: &str, params: &Value, id: Value) -> JsonRpcResponse {
    match action {
        "get" => handle_get(params, id).await,
        "set" => handle_set(params, id).await,
        "environment" => handle_environment(id).await,
        "getNodeConfig" => handle_get_node_config(id).await,
        "setNodeConfig" => handle_set_node_config(params, id).await,
        "getExecApprovals" => handle_get_exec_approvals(id).await,
        "setExecApprovals" => handle_set_exec_approvals(params, id).await,
        _ => method_not_found(id, &format!("config.{}", action)),
    }
}

/// Get the config path (detect gateway vs node)
fn get_config_path() -> PathBuf {
    let base = dirs::home_dir()
        .expect("Could not find home directory")
        .join(".openclaw");
    
    let gateway_path = base.join("openclaw.json");
    let node_path = base.join("node.json");
    
    if gateway_path.exists() {
        gateway_path
    } else {
        node_path
    }
}

/// Get the node config path
fn get_node_config_path() -> PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(".openclaw")
        .join("node.json")
}

/// Get the exec approvals path
fn get_exec_approvals_path() -> PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(".openclaw")
        .join("exec-approvals.json")
}

/// Get the full configuration
///
/// Params:
///   - section (optional): return only a specific top-level key (e.g. "agents", "gateway")
async fn handle_get(params: &Value, id: Value) -> JsonRpcResponse {
    let section = optional_string(params, "section");
    let config_path = get_config_path();

    if !config_path.exists() {
        return error(id, -32002, format!("Config file not found at {:?}", config_path));
    }

    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(e) => return generic_error(id, format!("Failed to read config: {}", e)),
    };

    let config: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => return generic_error(id, format!("Failed to parse config JSON: {}", e)),
    };

    if let Some(key) = section {
        match config.get(key) {
            Some(section_value) => ok(
                id,
                serde_json::json!({
                    "section": key,
                    "config": section_value,
                    "path": config_path.display().to_string(),
                }),
            ),
            None => error(id, -32002, format!("Config section '{}' not found", key)),
        }
    } else {
        ok(
            id,
            serde_json::json!({
                "config": config,
                "path": config_path.display().to_string(),
            }),
        )
    }
}

/// Update the configuration (partial merge)
///
/// Params:
///   - config (required): partial config object to merge into the existing config
///   - replace (optional): if true, replace entirely instead of merging (default false)
async fn handle_set(params: &Value, id: Value) -> JsonRpcResponse {
    let updates = match params.get("config") {
        Some(c) if c.is_object() => c,
        _ => {
            return invalid_params(id, "Missing or invalid 'config' parameter (must be an object)");
        }
    };

    let replace = bool_with_default(params, "replace", false);

    let config_path = get_config_path();

    // Read existing config
    let existing: Value = if config_path.exists() {
        let content = match std::fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(e) => return generic_error(id, format!("Failed to read existing config: {}", e)),
        };
        match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(e) => return generic_error(id, format!("Failed to parse existing config: {}", e)),
        }
    } else {
        serde_json::json!({})
    };

    // Merge or replace
    let new_config = if replace {
        updates.clone()
    } else {
        merge_json(&existing, updates)
    };

    // Backup existing config
    if config_path.exists() {
        let backup_path = config_path.with_extension("json.bak");
        if let Err(e) = std::fs::copy(&config_path, &backup_path) {
            warn!("Failed to create config backup: {}", e);
        }
    }

    // Ensure parent directory exists
    if let Some(parent) = config_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // Write the new config
    let content = match serde_json::to_string_pretty(&new_config) {
        Ok(c) => c,
        Err(e) => return generic_error(id, format!("Failed to serialize config: {}", e)),
    };

    match std::fs::write(&config_path, &content) {
        Ok(()) => {
            info!("Config updated at {:?}", config_path);
            ok(
                id,
                serde_json::json!({
                    "updated": true,
                    "path": config_path.display().to_string(),
                    "replace": replace,
                }),
            )
        }
        Err(e) => generic_error(id, format!("Failed to write config: {}", e)),
    }
}

/// Deep merge two JSON values. The `overlay` takes precedence over `base`.
fn merge_json(base: &Value, overlay: &Value) -> Value {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            let mut merged = base_map.clone();
            for (key, overlay_val) in overlay_map {
                let merged_val = if let Some(base_val) = merged.get(key) {
                    merge_json(base_val, overlay_val)
                } else {
                    overlay_val.clone()
                };
                merged.insert(key.clone(), merged_val);
            }
            Value::Object(merged)
        }
        // For non-objects, overlay wins
        (_, overlay) => overlay.clone(),
    }
}

/// `config.environment` — detect gateway vs node environment
async fn handle_environment(id: Value) -> JsonRpcResponse {
    let base = dirs::home_dir()
        .expect("Could not find home directory")
        .join(".openclaw");
    
    let gateway_path = base.join("openclaw.json");
    let node_path = base.join("node.json");
    let exec_approvals_path = base.join("exec-approvals.json");
    
    let is_gateway = gateway_path.exists();
    let is_node = node_path.exists();
    
    let env_type = if is_gateway {
        "gateway"
    } else if is_node {
        "node"
    } else {
        "unknown"
    };
    
    let mut files = Vec::new();
    if gateway_path.exists() {
        files.push(serde_json::json!({
            "name": "openclaw.json",
            "path": gateway_path.display().to_string(),
        }));
    }
    if node_path.exists() {
        files.push(serde_json::json!({
            "name": "node.json",
            "path": node_path.display().to_string(),
        }));
    }
    if exec_approvals_path.exists() {
        files.push(serde_json::json!({
            "name": "exec-approvals.json",
            "path": exec_approvals_path.display().to_string(),
        }));
    }
    
    ok(
        id,
        serde_json::json!({
            "type": env_type,
            "files": files,
        }),
    )
}

/// `config.getNodeConfig` — read node.json
async fn handle_get_node_config(id: Value) -> JsonRpcResponse {
    let node_path = get_node_config_path();
    
    if !node_path.exists() {
        return error(id, -32002, format!("Node config not found at {:?}", node_path));
    }
    
    let content = match std::fs::read_to_string(&node_path) {
        Ok(c) => c,
        Err(e) => return generic_error(id, format!("Failed to read node config: {}", e)),
    };
    
    let config: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => return generic_error(id, format!("Failed to parse node config JSON: {}", e)),
    };
    
    ok(
        id,
        serde_json::json!({
            "config": config,
            "path": node_path.display().to_string(),
        }),
    )
}

/// `config.setNodeConfig` — write node.json
async fn handle_set_node_config(params: &Value, id: Value) -> JsonRpcResponse {
    let config = match params.get("config") {
        Some(c) if c.is_object() => c,
        _ => {
            return invalid_params(id, "Missing or invalid 'config' parameter (must be an object)");
        }
    };
    
    let node_path = get_node_config_path();
    
    // Backup existing config
    if node_path.exists() {
        let backup_path = node_path.with_extension("json.bak");
        if let Err(e) = std::fs::copy(&node_path, &backup_path) {
            warn!("Failed to create node config backup: {}", e);
        }
    }
    
    // Ensure parent directory exists
    if let Some(parent) = node_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    
    let content = match serde_json::to_string_pretty(config) {
        Ok(c) => c,
        Err(e) => return generic_error(id, format!("Failed to serialize node config: {}", e)),
    };
    
    match std::fs::write(&node_path, &content) {
        Ok(()) => {
            info!("Node config updated at {:?}", node_path);
            ok(
                id,
                serde_json::json!({
                    "updated": true,
                    "path": node_path.display().to_string(),
                }),
            )
        }
        Err(e) => generic_error(id, format!("Failed to write node config: {}", e)),
    }
}

/// `config.getExecApprovals` — read exec-approvals.json
async fn handle_get_exec_approvals(id: Value) -> JsonRpcResponse {
    let approvals_path = get_exec_approvals_path();
    
    if !approvals_path.exists() {
        // Return empty array if file doesn't exist
        return ok(
            id,
            serde_json::json!({
                "approvals": [],
                "path": approvals_path.display().to_string(),
            }),
        );
    }
    
    let content = match std::fs::read_to_string(&approvals_path) {
        Ok(c) => c,
        Err(e) => return generic_error(id, format!("Failed to read exec approvals: {}", e)),
    };
    
    let approvals: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => return generic_error(id, format!("Failed to parse exec approvals JSON: {}", e)),
    };
    
    ok(
        id,
        serde_json::json!({
            "approvals": approvals,
            "path": approvals_path.display().to_string(),
        }),
    )
}

/// `config.setExecApprovals` — write exec-approvals.json
async fn handle_set_exec_approvals(params: &Value, id: Value) -> JsonRpcResponse {
    let approvals = match params.get("approvals") {
        Some(a) => a,
        _ => return invalid_params(id, "Missing 'approvals' parameter"),
    };
    
    let approvals_path = get_exec_approvals_path();
    
    // Backup existing config
    if approvals_path.exists() {
        let backup_path = approvals_path.with_extension("json.bak");
        if let Err(e) = std::fs::copy(&approvals_path, &backup_path) {
            warn!("Failed to create exec approvals backup: {}", e);
        }
    }
    
    // Ensure parent directory exists
    if let Some(parent) = approvals_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    
    let content = match serde_json::to_string_pretty(approvals) {
        Ok(c) => c,
        Err(e) => return generic_error(id, format!("Failed to serialize exec approvals: {}", e)),
    };
    
    match std::fs::write(&approvals_path, &content) {
        Ok(()) => {
            info!("Exec approvals updated at {:?}", approvals_path);
            ok(
                id,
                serde_json::json!({
                    "updated": true,
                    "path": approvals_path.display().to_string(),
                }),
            )
        }
        Err(e) => generic_error(id, format!("Failed to write exec approvals: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_json_simple() {
        let base = serde_json::json!({"a": 1, "b": 2});
        let overlay = serde_json::json!({"b": 3, "c": 4});
        let result = merge_json(&base, &overlay);
        assert_eq!(result, serde_json::json!({"a": 1, "b": 3, "c": 4}));
    }

    #[test]
    fn test_merge_json_nested() {
        let base = serde_json::json!({"a": {"x": 1, "y": 2}, "b": 3});
        let overlay = serde_json::json!({"a": {"y": 99, "z": 100}});
        let result = merge_json(&base, &overlay);
        assert_eq!(
            result,
            serde_json::json!({"a": {"x": 1, "y": 99, "z": 100}, "b": 3})
        );
    }

    #[test]
    fn test_merge_json_replace_scalar() {
        let base = serde_json::json!({"a": "old"});
        let overlay = serde_json::json!({"a": "new"});
        let result = merge_json(&base, &overlay);
        assert_eq!(result, serde_json::json!({"a": "new"}));
    }
}
