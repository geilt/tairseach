//! Automation Handler
//!
//! Handles AppleScript execution and accessibility automation.
//! Uses NSAppleScript / osascript to execute arbitrary AppleScript
//! through Tairseach's granted Automation permission.

use serde_json::Value;
use tracing::{error, info, warn};

use super::common::*;
use super::super::protocol::JsonRpcResponse;

/// Maximum AppleScript execution time in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Handle automation-related methods
pub async fn handle(action: &str, params: &Value, id: Value) -> JsonRpcResponse {
    match action {
        "run" => handle_run(params, id).await,
        "click" => handle_click(params, id).await,
        "type" => handle_type(params, id).await,
        _ => method_not_found(id, &format!("automation.{}", action)),
    }
}

/// Execute an AppleScript
///
/// Params:
///   - script (required): AppleScript source code to execute
///   - language (optional): "applescript" (default) or "javascript" (for JXA)
///   - timeout (optional): execution timeout in seconds (default 30)
///
/// Returns the script output as a string
async fn handle_run(params: &Value, id: Value) -> JsonRpcResponse {
    let script = match require_string(params, "script", &id) {
        Ok(s) => s,
        Err(response) => return response,
    };

    let language = string_with_default(params, "language", "applescript");
    let timeout_secs = u64_with_default(params, "timeout", DEFAULT_TIMEOUT_SECS);

    if language != "applescript" && language != "javascript" {
        return invalid_params(id, "Invalid language. Use 'applescript' or 'javascript' (JXA).");
    }

    match run_script(script, language, timeout_secs).await {
        Ok(output) => ok(
            id,
            serde_json::json!({
                "output": output,
                "language": language,
            }),
        ),
        Err(e) => generic_error(id, e),
    }
}

/// Click at screen coordinates
///
/// Params:
///   - x (required): X coordinate
///   - y (required): Y coordinate
///   - button (optional): "left" (default), "right"
///   - clicks (optional): number of clicks (default 1)
async fn handle_click(params: &Value, id: Value) -> JsonRpcResponse {
    let x = match require_f64(params, "x", &id) {
        Ok(v) => v,
        Err(response) => return response,
    };

    let y = match require_f64(params, "y", &id) {
        Ok(v) => v,
        Err(response) => return response,
    };

    let button = string_with_default(params, "button", "left");
    let clicks = u64_with_default(params, "clicks", 1);

    match click_at(x, y, button, clicks).await {
        Ok(()) => ok(
            id,
            serde_json::json!({
                "clicked": true,
                "x": x,
                "y": y,
                "button": button,
                "clicks": clicks,
            }),
        ),
        Err(e) => generic_error(id, e),
    }
}

/// Type text using keyboard simulation
///
/// Params:
///   - text (required): text to type
///   - delay (optional): delay between keystrokes in ms (default 0)
async fn handle_type(params: &Value, id: Value) -> JsonRpcResponse {
    let text = match require_string(params, "text", &id) {
        Ok(t) => t,
        Err(response) => return response,
    };

    let delay_ms = u64_with_default(params, "delay", 0);

    match type_text(text, delay_ms).await {
        Ok(()) => ok(
            id,
            serde_json::json!({
                "typed": true,
                "length": text.len(),
            }),
        ),
        Err(e) => generic_error(id, e),
    }
}

// ============================================================================
// AppleScript execution via osascript
// ============================================================================

/// Execute a script via osascript
#[cfg(target_os = "macos")]
async fn run_script(script: &str, language: &str, timeout_secs: u64) -> Result<String, String> {
    use std::process::Command;
    use std::time::Duration;

    let lang_flag = match language {
        "javascript" => "JavaScript",
        _ => "AppleScript",
    };

    info!(
        "Running {} script (timeout={}s, length={})",
        lang_flag,
        timeout_secs,
        script.len()
    );

    let child = Command::new("osascript")
        .arg("-l")
        .arg(lang_flag)
        .arg("-e")
        .arg(script)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn osascript: {}", e))?;

    // Wait with timeout using a spawned tokio task
    let timeout = Duration::from_secs(timeout_secs);

    let result = tokio::task::spawn_blocking(move || {
        let output = child.wait_with_output();
        output
    });

    let output = match tokio::time::timeout(timeout, result).await {
        Ok(Ok(Ok(output))) => output,
        Ok(Ok(Err(e))) => {
            return Err(format!("osascript error: {}", e));
        }
        Ok(Err(e)) => {
            return Err(format!("Task join error: {}", e));
        }
        Err(_) => {
            warn!("Script execution timed out after {}s", timeout_secs);
            return Err(format!(
                "Script execution timed out after {} seconds",
                timeout_secs
            ));
        }
    };

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        info!("Script executed successfully (output length: {})", stdout.len());
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        error!("Script execution failed: {}", stderr);
        Err(format!("Script error: {}", stderr))
    }
}

/// Click at screen coordinates using AppleScript + CGEvent
#[cfg(target_os = "macos")]
async fn click_at(x: f64, y: f64, button: &str, clicks: u64) -> Result<(), String> {
    use std::process::Command;

    let swift_code = format!(
        r#"
import CoreGraphics
import Foundation

let point = CGPoint(x: {x}, y: {y})
let buttonType: CGMouseButton = {button_type}
let downType: CGEventType = {down_type}
let upType: CGEventType = {up_type}

for _ in 0..<{clicks} {{
    guard let downEvent = CGEvent(mouseEventSource: nil, mouseType: downType, mouseCursorPosition: point, mouseButton: buttonType) else {{
        print("Failed to create mouse event")
        exit(1)
    }}
    downEvent.post(tap: .cghidEventTap)

    guard let upEvent = CGEvent(mouseEventSource: nil, mouseType: upType, mouseCursorPosition: point, mouseButton: buttonType) else {{
        print("Failed to create mouse event")
        exit(1)
    }}
    upEvent.post(tap: .cghidEventTap)

    if {clicks} > 1 {{
        Thread.sleep(forTimeInterval: 0.05)
    }}
}}
print("ok")
"#,
        x = x,
        y = y,
        button_type = if button == "right" { ".right" } else { ".left" },
        down_type = if button == "right" { ".rightMouseDown" } else { ".leftMouseDown" },
        up_type = if button == "right" { ".rightMouseUp" } else { ".leftMouseUp" },
        clicks = clicks,
    );

    let output = Command::new("swift")
        .args(["-e", &swift_code])
        .output()
        .map_err(|e| format!("Failed to run swift: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("Click failed: {}", stderr))
    }
}

/// Type text using AppleScript
#[cfg(target_os = "macos")]
async fn type_text(text: &str, delay_ms: u64) -> Result<(), String> {
    use std::process::Command;

    let delay_str = if delay_ms > 0 {
        format!(" with delay of {} milliseconds", delay_ms)
    } else {
        String::new()
    };

    // Escape text for AppleScript string
    let escaped = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"");

    let script = format!(
        r#"tell application "System Events" to keystroke "{escaped}"{delay}"#,
        escaped = escaped,
        delay = delay_str,
    );

    let output = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| format!("Failed to execute osascript: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("Type failed: {}", stderr))
    }
}

#[cfg(not(target_os = "macos"))]
async fn run_script(_script: &str, _language: &str, _timeout_secs: u64) -> Result<String, String> {
    Err("Automation is only available on macOS".to_string())
}

#[cfg(not(target_os = "macos"))]
async fn click_at(_x: f64, _y: f64, _button: &str, _clicks: u64) -> Result<(), String> {
    Err("Click automation is only available on macOS".to_string())
}

#[cfg(not(target_os = "macos"))]
async fn type_text(_text: &str, _delay_ms: u64) -> Result<(), String> {
    Err("Type automation is only available on macOS".to_string())
}
