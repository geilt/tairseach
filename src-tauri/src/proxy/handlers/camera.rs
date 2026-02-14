//! Camera Handler
//!
//! Handles camera and microphone access using AVFoundation.
//! Provides device listing and basic capture functionality.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{error, info};

use super::common::*;
use super::super::protocol::JsonRpcResponse;

/// Camera/Microphone device representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    #[serde(rename = "uniqueId")]
    pub unique_id: String,
    #[serde(rename = "localizedName")]
    pub localized_name: String,
    #[serde(rename = "modelId")]
    pub model_id: String,
    #[serde(rename = "deviceType")]
    pub device_type: String,
    #[serde(rename = "isConnected")]
    pub is_connected: bool,
    #[serde(rename = "hasAudio")]
    pub has_audio: bool,
}

/// Handle camera-related methods
pub async fn handle(action: &str, params: &Value, id: Value) -> JsonRpcResponse {
    match action {
        "list" => handle_list_devices(params, id).await,
        "snap" => handle_snap(params, id).await,
        "capture" => handle_snap(params, id).await, // Alias
        _ => method_not_found(id, &format!("camera.{}", action)),
    }
}

/// List available camera/microphone devices
async fn handle_list_devices(params: &Value, id: Value) -> JsonRpcResponse {
    let device_type = string_with_default(params, "type", "all");
    
    match list_devices(device_type).await {
        Ok(devices) => ok(
            id,
            serde_json::json!({
                "devices": devices,
                "count": devices.len(),
            }),
        ),
        Err(e) => generic_error(id, e),
    }
}

/// Capture a photo from the camera
///
/// Params:
///   - deviceId (optional): camera device ID; if not provided, uses default camera
///   - format (optional): "jpg" or "png", default "jpg"
///   - path (optional): save path; if not provided, saves to temp file
async fn handle_snap(params: &Value, id: Value) -> JsonRpcResponse {
    let device_id = optional_string(params, "deviceId");
    let img_format = string_with_default(params, "format", "jpg");
    let save_path = optional_string(params, "path");
    
    if img_format != "png" && img_format != "jpg" && img_format != "jpeg" {
        return invalid_params(id, "Invalid format. Use 'png' or 'jpg'.");
    }
    
    match snap_photo(device_id, img_format, save_path).await {
        Ok(result) => ok(id, result),
        Err(e) => generic_error(id, e),
    }
}

// ============================================================================
// Native AVFoundation integration via Swift
// ============================================================================

/// List camera and microphone devices using Swift
#[cfg(target_os = "macos")]
async fn list_devices(device_type: &str) -> Result<Vec<Device>, String> {
    use std::process::Command;
    
    info!("Listing AV devices via Swift: type={}", device_type);
    
    let media_type_filter = match device_type {
        "camera" | "video" => "video",
        "microphone" | "audio" => "audio",
        _ => "all",
    };
    
    let swift_code = format!(
        r#"
import AVFoundation
import Foundation

var result: [[String: Any]] = []

let includeVideo = "{media_type}" == "all" || "{media_type}" == "video"
let includeAudio = "{media_type}" == "all" || "{media_type}" == "audio"

if includeVideo {{
    let videoDevices = AVCaptureDevice.DiscoverySession(
        deviceTypes: [.builtInWideAngleCamera, .externalUnknown],
        mediaType: .video,
        position: .unspecified
    ).devices
    
    for device in videoDevices {{
        result.append([
            "uniqueId": device.uniqueID,
            "localizedName": device.localizedName,
            "modelId": device.modelID,
            "deviceType": "camera",
            "isConnected": device.isConnected,
            "hasAudio": device.hasMediaType(.audio)
        ])
    }}
}}

if includeAudio {{
    let audioDevices = AVCaptureDevice.DiscoverySession(
        deviceTypes: [.builtInMicrophone, .externalUnknown],
        mediaType: .audio,
        position: .unspecified
    ).devices
    
    for device in audioDevices {{
        result.append([
            "uniqueId": device.uniqueID,
            "localizedName": device.localizedName,
            "modelId": device.modelID,
            "deviceType": "microphone",
            "isConnected": device.isConnected,
            "hasAudio": true
        ])
    }}
}}

if let jsonData = try? JSONSerialization.data(withJSONObject: result, options: []),
   let jsonString = String(data: jsonData, encoding: .utf8) {{
    print(jsonString)
}} else {{
    print("[]")
}}
"#,
        media_type = media_type_filter
    );
    
    let output = Command::new("swift")
        .args(["-e", &swift_code])
        .output()
        .map_err(|e| format!("Failed to run swift: {}", e))?;
    
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str::<Vec<Device>>(stdout.trim())
            .map_err(|e| format!("Failed to parse devices JSON: {}", e))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        error!("Swift device listing failed: {}", stderr);
        Err(format!("Device listing failed: {}", stderr))
    }
}

/// Capture a photo from the camera using Swift
#[cfg(target_os = "macos")]
async fn snap_photo(
    device_id: Option<&str>,
    img_format: &str,
    save_path: Option<&str>,
) -> Result<Value, String> {
    use std::process::Command;
    
    let default_path = format!(
        "/tmp/tairseach_camera_snap_{}.{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
        img_format
    );
    let output_path = save_path.unwrap_or(&default_path);
    
    info!(
        "Capturing photo from camera: device_id={:?}, format={}, path={}",
        device_id, img_format, output_path
    );
    
    let device_selector = if let Some(id) = device_id {
        format!(
            r#"
    guard let device = AVCaptureDevice(uniqueID: "{}") else {{
        print("{{\\"error\\":\\"Camera device not found: {}\\"}}")
        exit(1)
    }}
    "#,
            id.replace('\\', "\\\\").replace('"', "\\\""),
            id.replace('\\', "\\\\").replace('"', "\\\"")
        )
    } else {
        r#"
    guard let device = AVCaptureDevice.default(for: .video) else {
        print("{\"error\":\"No camera device available\"}")
        exit(1)
    }
    "#.to_string()
    };
    
    let uti_type = match img_format {
        "png" => "public.png",
        _ => "public.jpeg",
    };
    
    let swift_code = format!(
        r#"
import AVFoundation
import CoreMedia
import Foundation
import ImageIO
import CoreGraphics

{device_selector}

let session = AVCaptureSession()
session.sessionPreset = .photo

do {{
    let input = try AVCaptureDeviceInput(device: device)
    if session.canAddInput(input) {{
        session.addInput(input)
    }} else {{
        print("{{\\"error\\":\\"Cannot add camera input\\"}}")
        exit(1)
    }}
}} catch {{
    print("{{\\"error\\":\\"Failed to create camera input: \\(error.localizedDescription)\\"}}")
    exit(1)
}}

let output = AVCapturePhotoOutput()
if session.canAddOutput(output) {{
    session.addOutput(output)
}} else {{
    print("{{\\"error\\":\\"Cannot add photo output\\"}}")
    exit(1)
}}

class PhotoCaptureDelegate: NSObject, AVCapturePhotoCaptureDelegate {{
    var photoData: Data?
    var semaphore = DispatchSemaphore(value: 0)
    
    func photoOutput(_ output: AVCapturePhotoOutput, didFinishProcessingPhoto photo: AVCapturePhoto, error: Error?) {{
        if let error = error {{
            print("{{\\"error\\":\\"Photo capture failed: \\(error.localizedDescription)\\"}}")
            semaphore.signal()
            return
        }}
        
        photoData = photo.fileDataRepresentation()
        semaphore.signal()
    }}
}}

session.startRunning()

// Wait a moment for camera to warm up
Thread.sleep(forTimeInterval: 0.5)

let settings = AVCapturePhotoSettings()
let delegate = PhotoCaptureDelegate()

output.capturePhoto(with: settings, delegate: delegate)

// Wait for capture to complete
delegate.semaphore.wait()

session.stopRunning()

guard let data = delegate.photoData else {{
    print("{{\\"error\\":\\"No photo data captured\\"}}")
    exit(1)
}}

// Convert to desired format if needed
let image = NSImage(data: data)
guard let cgImage = image?.cgImage(forProposedRect: nil, context: nil, hints: nil) else {{
    print("{{\\"error\\":\\"Failed to create image\\"}}")
    exit(1)
}}

let url = URL(fileURLWithPath: "{output_path}")
let uti = "{uti_type}" as CFString

guard let destination = CGImageDestinationCreateWithURL(url as CFURL, uti, 1, nil) else {{
    print("{{\\"error\\":\\"Failed to create image destination\\"}}")
    exit(1)
}}

CGImageDestinationAddImage(destination, cgImage, nil)

if CGImageDestinationFinalize(destination) {{
    let attrs = try? FileManager.default.attributesOfItem(atPath: "{output_path}")
    let fileSize = (attrs?[.size] as? Int) ?? 0
    print("{{\\"path\\":\\"{output_path}\\",\\"width\\":\\(cgImage.width),\\"height\\":\\(cgImage.height),\\"fileSize\\":\\(fileSize),\\"format\\":\\"{format}\\"}}")
}} else {{
    print("{{\\"error\\":\\"Failed to write image\\"}}")
}}
"#,
        device_selector = device_selector,
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
            return Err("No output from camera capture".to_string());
        }
        
        let parsed: Value = serde_json::from_str(&stdout)
            .map_err(|e| format!("Failed to parse capture result: {} — raw: {}", e, &stdout))?;
        
        if let Some(err) = parsed.get("error").and_then(|v| v.as_str()) {
            return Err(err.to_string());
        }
        
        info!("Camera snapshot captured: {}", output_path);
        Ok(parsed)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        error!("Swift camera capture failed: {}", stderr);
        Err(format!("Camera capture failed: {}", stderr))
    }
}

// Non-macOS stubs
#[cfg(not(target_os = "macos"))]
async fn list_devices(_device_type: &str) -> Result<Vec<Device>, String> {
    Err("Camera/microphone access is only available on macOS".to_string())
}

#[cfg(not(target_os = "macos"))]
async fn snap_photo(
    _device_id: Option<&str>,
    _img_format: &str,
    _save_path: Option<&str>,
) -> Result<Value, String> {
    Err("Camera capture is only available on macOS".to_string())
}
