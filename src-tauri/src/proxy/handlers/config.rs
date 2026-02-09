//! Config Handler
//!
//! Handles config-related JSON-RPC methods for reading/writing
//! the OpenClaw configuration file (~/.openclaw/openclaw.json).

use serde_json::Value;
use std::path::PathBuf;
use tracing::{info, warn};

use super::super::protocol::JsonRpcResponse;

/// Handle config-related methods
pub async fn handle(action: &str, params: &Value, id: Value) -> JsonRpcResponse {
    match action {
        "get" => handle_get(params, id).await,
        "set" => handle_set(params, id).await,
        _ => JsonRpcResponse::method_not_found(id, &format!("config.{}", action)),
    }
}

/// Get the config path
fn get_config_path() -> PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(".openclaw")
        .join("openclaw.json")
}

/// Get the full configuration
///
/// Params:
///   - section (optional): return only a specific top-level key (e.g. "agents", "gateway")
async fn handle_get(params: &Value, id: Value) -> JsonRpcResponse {
    let section = params.get("section").and_then(|v| v.as_str());
    let config_path = get_config_path();

    if !config_path.exists() {
        return JsonRpcResponse::error(
            id,
            -32002,
            format!("Config file not found at {:?}", config_path),
            None,
        );
    }

    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(e) => {
            return JsonRpcResponse::error(
                id,
                -32000,
                format!("Failed to read config: {}", e),
                None,
            );
        }
    };

    let config: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            return JsonRpcResponse::error(
                id,
                -32000,
                format!("Failed to parse config JSON: {}", e),
                None,
            );
        }
    };

    if let Some(key) = section {
        match config.get(key) {
            Some(section_value) => JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "section": key,
                    "config": section_value,
                    "path": config_path.display().to_string(),
                }),
            ),
            None => JsonRpcResponse::error(
                id,
                -32002,
                format!("Config section '{}' not found", key),
                None,
            ),
        }
    } else {
        JsonRpcResponse::success(
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
            return JsonRpcResponse::invalid_params(
                id,
                "Missing or invalid 'config' parameter (must be an object)",
            );
        }
    };

    let replace = params
        .get("replace")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let config_path = get_config_path();

    // Read existing config
    let existing: Value = if config_path.exists() {
        let content = match std::fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(e) => {
                return JsonRpcResponse::error(
                    id,
                    -32000,
                    format!("Failed to read existing config: {}", e),
                    None,
                );
            }
        };
        match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                return JsonRpcResponse::error(
                    id,
                    -32000,
                    format!("Failed to parse existing config: {}", e),
                    None,
                );
            }
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
        Err(e) => {
            return JsonRpcResponse::error(
                id,
                -32000,
                format!("Failed to serialize config: {}", e),
                None,
            );
        }
    };

    match std::fs::write(&config_path, &content) {
        Ok(()) => {
            info!("Config updated at {:?}", config_path);
            JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "updated": true,
                    "path": config_path.display().to_string(),
                    "replace": replace,
                }),
            )
        }
        Err(e) => JsonRpcResponse::error(
            id,
            -32000,
            format!("Failed to write config: {}", e),
            None,
        ),
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
