//! Screen Handler
//!
//! Handles screen capture and window listing via Core Graphics.
//! Uses Swift/JXA bridge for reliable CGDisplayCreateImage / CGWindowListCopyWindowInfo access.

use serde_json::Value;
use tracing::{error, info};

use super::super::protocol::JsonRpcResponse;

/// Handle screen-related methods
pub async fn handle(action: &str, params: &Value, id: Value) -> JsonRpcResponse {
    match action {
        "capture" => handle_capture(params, id).await,
        "windows" => handle_windows(params, id).await,
        _ => JsonRpcResponse::method_not_found(id, &format!("screen.{}", action)),
    }
}

/// Capture a screenshot
///
/// Params:
///   - display (optional): display index, default 0 (main display)
///   - format (optional): "png" or "jpg", default "png"
///   - path (optional): save path; if not provided, saves to temp file
///
/// Returns the path to the saved screenshot and base64-encoded data
async fn handle_capture(params: &Value, id: Value) -> JsonRpcResponse {
    let display_idx = params.get("display").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let img_format = params
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("png");
    let save_path = params.get("path").and_then(|v| v.as_str());

    if img_format != "png" && img_format != "jpg" && img_format != "jpeg" {
        return JsonRpcResponse::invalid_params(
            id,
            "Invalid format. Use 'png' or 'jpg'.",
        );
    }

    match capture_screen(display_idx, img_format, save_path).await {
        Ok(result) => JsonRpcResponse::success(id, result),
        Err(e) => JsonRpcResponse::error(id, -32000, e, None),
    }
}

/// List visible windows
///
/// Returns a list of windows with their title, owner, bounds, and window ID.
async fn handle_windows(_params: &Value, id: Value) -> JsonRpcResponse {
    match list_windows().await {
        Ok(windows) => JsonRpcResponse::success(
            id,
            serde_json::json!({
                "windows": windows,
                "count": windows.as_array().map(|a| a.len()).unwrap_or(0),
            }),
        ),
        Err(e) => JsonRpcResponse::error(id, -32000, e, None),
    }
}

// ============================================================================
// Native screen capture via Swift
// ============================================================================

/// Capture a screenshot using Swift/CoreGraphics
#[cfg(target_os = "macos")]
async fn capture_screen(
    display_idx: u32,
    img_format: &str,
    save_path: Option<&str>,
) -> Result<Value, String> {
    use std::process::Command;

    let default_path = format!(
        "/tmp/tairseach_screenshot_{}.{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
        img_format
    );
    let output_path = save_path.unwrap_or(&default_path);

    let uti_type = match img_format {
        "jpg" | "jpeg" => "public.jpeg",
        _ => "public.png",
    };

    info!(
        "Capturing screen: display={}, format={}, path={}",
        display_idx, img_format, output_path
    );

    let swift_code = format!(
        r#"
import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers

// Get all displays
let maxDisplays: UInt32 = 16
var displays = [CGDirectDisplayID](repeating: 0, count: Int(maxDisplays))
var displayCount: UInt32 = 0
CGGetActiveDisplayList(maxDisplays, &displays, &displayCount)

let displayIndex: UInt32 = {display}
if displayIndex >= displayCount {{
    print("{{\\"error\\":\\"Display index \(displayIndex) not found. Available: \(displayCount) displays\\"}}")
}} else {{
    let targetDisplay = displays[Int(displayIndex)]
    guard let image = CGDisplayCreateImage(targetDisplay) else {{
        print("{{\\"error\\":\\"Failed to create display image\\"}}")
        exit(1)
    }}

    let url = URL(fileURLWithPath: "{output_path}")
    let uti = "{uti_type}" as CFString

    guard let destination = CGImageDestinationCreateWithURL(url as CFURL, uti, 1, nil) else {{
        print("{{\\"error\\":\\"Failed to create image destination\\"}}")
        exit(1)
    }}

    CGImageDestinationAddImage(destination, image, nil)

    if CGImageDestinationFinalize(destination) {{
        let attrs = try? FileManager.default.attributesOfItem(atPath: "{output_path}")
        let size = (attrs?[.size] as? Int) ?? 0
        print("{{\\"path\\":\\"{output_path}\\",\\"width\\":\(image.width),\\"height\\":\(image.height),\\"fileSize\\":\(size),\\"format\\":\\"{format}\\"}}")
    }} else {{
        print("{{\\"error\\":\\"Failed to write image\\"}}")
    }}
}}
"#,
        display = display_idx,
        output_path = output_path.replace('\\', "\\\\").replace('"', "\\\""),
        uti_type = uti_type,
        format = img_format,
    );

    let output = Command::new("swift")
        .args(["-e", &swift_code])
        .output()
        .map_err(|e| format!("Failed to run swift: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if stdout.is_empty() {
            return Err("No output from screen capture".to_string());
        }

        let parsed: Value = serde_json::from_str(&stdout)
            .map_err(|e| format!("Failed to parse capture result: {} — raw: {}", e, &stdout))?;

        if let Some(err) = parsed.get("error").and_then(|v| v.as_str()) {
            return Err(err.to_string());
        }

        info!("Screenshot captured: {}", output_path);
        Ok(parsed)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        error!("Swift screen capture failed: {}", stderr);
        Err(format!("Screen capture failed: {}", stderr))
    }
}

/// List visible windows using JXA
#[cfg(target_os = "macos")]
async fn list_windows() -> Result<Value, String> {
    use std::process::Command;

    info!("Listing windows via JXA");

    let script = r#"
        ObjC.import('CoreGraphics');
        ObjC.import('Foundation');

        var windowList = $.CGWindowListCopyWindowInfo($.kCGWindowListOptionOnScreenOnly | $.kCGWindowListExcludeDesktopElements, $.kCGNullWindowID);

        var result = [];
        if (windowList) {
            var count = $.CFArrayGetCount(windowList);
            for (var i = 0; i < count; i++) {
                var info = $.CFArrayGetValueAtIndex(windowList, i);
                
                // Get window properties
                var nameRef = $.CFDictionaryGetValue(info, $("kCGWindowName"));
                var ownerRef = $.CFDictionaryGetValue(info, $("kCGWindowOwnerName"));
                var pidRef = $.CFDictionaryGetValue(info, $("kCGWindowOwnerPID"));
                var windowIdRef = $.CFDictionaryGetValue(info, $("kCGWindowNumber"));
                var layerRef = $.CFDictionaryGetValue(info, $("kCGWindowLayer"));
                var boundsRef = $.CFDictionaryGetValue(info, $("kCGWindowBounds"));
                
                var name = nameRef ? ObjC.unwrap($.CFBridgingRelease(nameRef)) : null;
                var owner = ownerRef ? ObjC.unwrap($.CFBridgingRelease(ownerRef)) : 'Unknown';
                
                // Only include windows with a name or meaningful owner
                if (owner && owner !== '' && owner !== 'Window Server') {
                    var entry = {
                        name: name || null,
                        owner: owner,
                    };
                    
                    if (pidRef) entry.pid = $.CFBridgingRelease(pidRef).intValue;
                    if (windowIdRef) entry.windowId = $.CFBridgingRelease(windowIdRef).intValue;
                    if (layerRef) entry.layer = $.CFBridgingRelease(layerRef).intValue;
                    
                    if (boundsRef) {
                        var bounds = ObjC.deepUnwrap($.CFBridgingRelease(boundsRef));
                        entry.bounds = {
                            x: bounds.X,
                            y: bounds.Y,
                            width: bounds.Width,
                            height: bounds.Height,
                        };
                    }
                    
                    result.push(entry);
                }
            }
        }

        JSON.stringify(result);
    "#;

    let output = Command::new("osascript")
        .arg("-l")
        .arg("JavaScript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|e| format!("Failed to execute osascript: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let windows: Value = serde_json::from_str(&stdout)
            .map_err(|e| format!("Failed to parse windows JSON: {} — raw: {}", e, &stdout))?;

        info!(
            "Listed {} windows",
            windows.as_array().map(|a| a.len()).unwrap_or(0)
        );
        Ok(windows)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        error!("JXA window listing failed: {}", stderr);
        Err(format!("Window listing failed: {}", stderr))
    }
}

#[cfg(not(target_os = "macos"))]
async fn capture_screen(
    _display_idx: u32,
    _img_format: &str,
    _save_path: Option<&str>,
) -> Result<Value, String> {
    Err("Screen capture is only available on macOS".to_string())
}

#[cfg(not(target_os = "macos"))]
async fn list_windows() -> Result<Value, String> {
    Err("Window listing is only available on macOS".to_string())
}
