//! Script Implementation Dispatcher
//!
//! Executes external scripts with credential injection via environment variables.

use std::collections::HashMap;
use std::process::Stdio;
use serde_json::Value;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::{error, info};

use crate::manifest::types::{Manifest, ScriptToolBinding, Tool};
use crate::proxy::protocol::JsonRpcResponse;

/// Dispatch to external script with credential injection
pub async fn dispatch(
    _manifest: &Manifest,
    tool: &Tool,
    params: &Value,
    id: Value,
    runtime: &str,
    entrypoint: &str,
    args: &[String],
    env_template: &HashMap<String, String>,
    tool_bindings: &HashMap<String, ScriptToolBinding>,
    credentials: &HashMap<String, Value>,
) -> JsonRpcResponse {
    info!(
        "Executing script for tool {}: {} {}",
        tool.name, runtime, entrypoint
    );

    // Get the tool binding
    let binding = match tool_bindings.get(&tool.name) {
        Some(b) => b,
        None => {
            return JsonRpcResponse::error(
                id,
                -32601,
                format!("No script binding for tool: {}", tool.name),
                None,
            );
        }
    };

    // Resolve script path (relative to home or absolute)
    let script_path = resolve_script_path(entrypoint);

    // Validate script path exists
    if !script_path.exists() {
        error!("Script not found: {:?}", script_path);
        return JsonRpcResponse::error(
            id,
            -32000,
            format!("Script not found: {}", script_path.display()),
            None,
        );
    }

    // Build environment variables with credential injection
    let mut env_vars = build_env_vars(env_template, credentials);

    // Add standard environment variables
    env_vars.insert("TAIRSEACH_TOOL".to_string(), tool.name.clone());
    env_vars.insert("TAIRSEACH_ACTION".to_string(), binding.action.clone());

    // Determine command to run based on runtime
    let (cmd, cmd_args) = match runtime {
        "bash" => ("bash", vec![script_path.display().to_string()]),
        "sh" => ("sh", vec![script_path.display().to_string()]),
        "python3" => ("python3", vec![script_path.display().to_string()]),
        "node" => ("node", vec![script_path.display().to_string()]),
        "ruby" => ("ruby", vec![script_path.display().to_string()]),
        "custom" => {
            // For custom runtime, entrypoint is the executable
            (entrypoint, args.to_vec())
        }
        _ => {
            return JsonRpcResponse::error(
                id,
                -32000,
                format!("Unsupported script runtime: {}", runtime),
                None,
            );
        }
    };

    // Build input payload for script (JSON on stdin)
    let input_payload = serde_json::json!({
        "tool": tool.name,
        "action": binding.action,
        "params": params,
    });

    let input_json = match serde_json::to_string(&input_payload) {
        Ok(json) => json,
        Err(e) => {
            return JsonRpcResponse::error(
                id,
                -32000,
                format!("Failed to serialize params: {}", e),
                None,
            );
        }
    };

    // Execute script
    let mut child = match Command::new(cmd)
        .args(&cmd_args)
        .env_clear() // SECURITY: Clear inherited environment
        .envs(&env_vars) // Only inject required vars
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to spawn script: {}", e);
            return JsonRpcResponse::error(
                id,
                -32000,
                format!("Script execution failed: {}", e),
                None,
            );
        }
    };

    // Write input to stdin
    if let Some(mut stdin) = child.stdin.take() {
        if let Err(e) = stdin.write_all(input_json.as_bytes()).await {
            error!("Failed to write to script stdin: {}", e);
        }
    }

    // Wait for completion with timeout
    let timeout = std::time::Duration::from_secs(60);
    let output = match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(Ok(out)) => out,
        Ok(Err(e)) => {
            return JsonRpcResponse::error(
                id,
                -32000,
                format!("Script error: {}", e),
                None,
            );
        }
        Err(_) => {
            return JsonRpcResponse::error(id, -32000, "Script execution timed out", None);
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Script failed: {}", stderr);
        return JsonRpcResponse::error(
            id,
            -32000,
            format!("Script exited with error: {}", stderr),
            None,
        );
    }

    // Parse stdout as JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    match serde_json::from_str::<Value>(&stdout) {
        Ok(result) => JsonRpcResponse::success(id, result),
        Err(e) => {
            error!("Script output is not valid JSON: {}", e);
            JsonRpcResponse::error(
                id,
                -32000,
                format!("Script returned invalid JSON: {}", e),
                Some(serde_json::json!({ "raw_output": stdout.to_string() })),
            )
        }
    }
}

fn resolve_script_path(entrypoint: &str) -> std::path::PathBuf {
    if entrypoint.starts_with('/') {
        // Absolute path
        std::path::PathBuf::from(entrypoint)
    } else if entrypoint.starts_with("~/") {
        // Home-relative path
        dirs::home_dir()
            .unwrap()
            .join(entrypoint.strip_prefix("~/").unwrap())
    } else {
        // Relative to ~/.tairseach/scripts/
        dirs::home_dir()
            .unwrap()
            .join(".tairseach")
            .join("scripts")
            .join(entrypoint)
    }
}

fn build_env_vars(
    env_template: &HashMap<String, String>,
    credentials: &HashMap<String, Value>,
) -> HashMap<String, String> {
    let mut env_vars = HashMap::new();

    for (key, value_template) in env_template {
        let value = interpolate_credentials(value_template, credentials);
        env_vars.insert(key.clone(), value);
    }

    env_vars
}

/// Interpolate credential placeholders in environment variable values
/// Example: "{credential:google-oauth:access_token}" → "ya29.a0..."
fn interpolate_credentials(template: &str, credentials: &HashMap<String, Value>) -> String {
    let mut result = template.to_string();

    // Simple placeholder replacement: {credential:id:field}
    while let Some(start) = result.find("{credential:") {
        if let Some(end_pos) = result[start..].find('}') {
            let end = start + end_pos;
            let placeholder = &result[start + 12..end];
            let parts: Vec<&str> = placeholder.split(':').collect();

            if parts.len() == 2 {
                let (cred_id, field) = (parts[0], parts[1]);
                if let Some(cred_value) = credentials.get(cred_id) {
                    if let Some(field_value) = cred_value.get(field).and_then(|v| v.as_str()) {
                        result.replace_range(start..=end, field_value);
                        continue;
                    }
                }
            }

            // If interpolation failed, remove placeholder
            result.replace_range(start..=end, "");
        } else {
            break;
        }
    }

    result
}
