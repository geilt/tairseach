//! Files Handler
//!
//! Handles file operations via Tairseach's Full Disk Access permission.
//! Uses standard Rust `std::fs` — the value is that Tairseach has FDA,
//! so it can access paths that agents cannot (e.g., ~/Library/Mail, etc.).

use serde_json::Value;
use std::fs;
use std::path::Path;
use tracing::{info, warn};

use super::super::protocol::JsonRpcResponse;

/// Maximum file size we'll read into memory (10 MB)
const MAX_READ_SIZE: u64 = 10 * 1024 * 1024;

// ── Path Security ───────────────────────────────────────────────────────────

/// Paths that are explicitly denied for writes (critical system paths)
const WRITE_DENY_LIST: &[&str] = &[
    "/",
    "/System",
    "/Library",
    "/usr",
    "/bin",
    "/sbin",
    "/etc",
    "/var",
    "/dev",
    "/Volumes",
    "/private",
];

/// Validate that a path is safe for writing.
///
/// **SECURITY:** Prevents writes to critical system paths and Tairseach's own
/// auth store. This is defense-in-depth against compromised agents.
fn validate_write_path(path: &Path) -> Result<(), String> {
    // Resolve symlinks to real path
    let canonical = path.canonicalize().unwrap_or_else(|_| {
        // If canonicalize fails (path doesn't exist yet), check the parent
        path.parent()
            .and_then(|p| p.canonicalize().ok())
            .map(|parent| parent.join(path.file_name().unwrap_or_default()))
            .unwrap_or_else(|| path.to_path_buf())
    });

    let path_str = canonical.display().to_string();

    // Check deny list
    for denied_prefix in WRITE_DENY_LIST {
        if path_str == *denied_prefix || path_str.starts_with(&format!("{}/", denied_prefix)) {
            return Err(format!(
                "Writes to {} are not allowed for security reasons",
                denied_prefix
            ));
        }
    }

    // Deny writes to sensitive paths under home directory
    if let Some(home) = dirs::home_dir() {
        let auth_dir = home.join(".tairseach").join("auth");
        if canonical.starts_with(&auth_dir) {
            return Err("Writes to Tairseach auth store are not allowed".to_string());
        }

        let denied_exact = [
            home.join(".zshrc"),
            home.join(".bashrc"),
            home.join(".bash_profile"),
            home.join(".profile"),
        ];

        for denied in denied_exact {
            if canonical == denied {
                return Err(format!(
                    "Writes to {} are not allowed for security reasons",
                    denied.display()
                ));
            }
        }

        let denied_dirs = [
            home.join(".ssh"),
            home.join(".gnupg"),
            home.join("Library").join("LaunchAgents"),
            home.join("Library").join("LaunchDaemons"),
        ];

        for denied in denied_dirs {
            if canonical.starts_with(&denied) {
                return Err(format!(
                    "Writes to {} are not allowed for security reasons",
                    denied.display()
                ));
            }
        }
    }

    Ok(())
}

/// Handle file-related methods
pub async fn handle(action: &str, params: &Value, id: Value) -> JsonRpcResponse {
    match action {
        "read" => handle_read(params, id).await,
        "write" => handle_write(params, id).await,
        "list" => handle_list(params, id).await,
        _ => JsonRpcResponse::method_not_found(id, &format!("files.{}", action)),
    }
}

/// Read a file and return its contents
///
/// Params:
///   - path (required): absolute path to the file
///   - encoding (optional): "utf8" (default) or "base64"
///   - maxSize (optional): maximum bytes to read (default 10MB)
async fn handle_read(params: &Value, id: Value) -> JsonRpcResponse {
    let file_path = match params.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing 'path' parameter");
        }
    };

    let encoding = params
        .get("encoding")
        .and_then(|v| v.as_str())
        .unwrap_or("utf8");

    let max_size = params
        .get("maxSize")
        .and_then(|v| v.as_u64())
        .unwrap_or(MAX_READ_SIZE);

    // Validate the path
    let path = Path::new(file_path);
    if !path.is_absolute() {
        return JsonRpcResponse::invalid_params(id, "Path must be absolute");
    }

    if !path.exists() {
        return JsonRpcResponse::error(
            id,
            -32002,
            format!("File not found: {}", file_path),
            None,
        );
    }

    if !path.is_file() {
        return JsonRpcResponse::error(
            id,
            -32002,
            format!("Not a file: {}", file_path),
            None,
        );
    }

    // Check file size
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            return JsonRpcResponse::error(
                id,
                -32000,
                format!("Failed to read file metadata: {}", e),
                None,
            );
        }
    };

    if metadata.len() > max_size {
        return JsonRpcResponse::error(
            id,
            -32003,
            format!(
                "File too large: {} bytes (max: {} bytes). Use 'maxSize' to increase limit.",
                metadata.len(),
                max_size
            ),
            None,
        );
    }

    info!("Reading file: {} (encoding={})", file_path, encoding);

    match encoding {
        "base64" => {
            use std::io::Read;
            let mut file = match fs::File::open(path) {
                Ok(f) => f,
                Err(e) => {
                    return JsonRpcResponse::error(
                        id,
                        -32000,
                        format!("Failed to open file: {}", e),
                        None,
                    );
                }
            };
            let mut buffer = Vec::new();
            if let Err(e) = file.read_to_end(&mut buffer) {
                return JsonRpcResponse::error(
                    id,
                    -32000,
                    format!("Failed to read file: {}", e),
                    None,
                );
            }

            use base64_encode;
            JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "path": file_path,
                    "encoding": "base64",
                    "content": base64_encode(&buffer),
                    "size": metadata.len(),
                }),
            )
        }
        _ => {
            // UTF-8 text
            match fs::read_to_string(path) {
                Ok(content) => JsonRpcResponse::success(
                    id,
                    serde_json::json!({
                        "path": file_path,
                        "encoding": "utf8",
                        "content": content,
                        "size": metadata.len(),
                    }),
                ),
                Err(e) => {
                    // If UTF-8 fails, suggest base64
                    JsonRpcResponse::error(
                        id,
                        -32000,
                        format!("Failed to read as UTF-8: {}. Try encoding='base64'.", e),
                        None,
                    )
                }
            }
        }
    }
}

/// Write content to a file
///
/// Params:
///   - path (required): absolute path to write to
///   - content (required): content to write
///   - encoding (optional): "utf8" (default) or "base64"
///   - createDirs (optional): create parent directories if missing (default false)
///   - append (optional): append instead of overwrite (default false)
async fn handle_write(params: &Value, id: Value) -> JsonRpcResponse {
    let file_path = match params.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing 'path' parameter");
        }
    };

    let content = match params.get("content").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing 'content' parameter");
        }
    };

    let encoding = params
        .get("encoding")
        .and_then(|v| v.as_str())
        .unwrap_or("utf8");

    let create_dirs = params
        .get("createDirs")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let append = params
        .get("append")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Validate the path
    let path = Path::new(file_path);
    if !path.is_absolute() {
        return JsonRpcResponse::invalid_params(id, "Path must be absolute");
    }

    // SECURITY: Check path restrictions (deny writes to critical system paths)
    if let Err(e) = validate_write_path(path) {
        return JsonRpcResponse::error(
            id,
            -32004,
            format!("Path not allowed for writing: {}", e),
            None,
        );
    }

    // Create parent directories if requested
    if create_dirs {
        if let Some(parent) = path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return JsonRpcResponse::error(
                    id,
                    -32000,
                    format!("Failed to create directories: {}", e),
                    None,
                );
            }
        }
    }

    info!("Writing file: {} (encoding={}, append={})", file_path, encoding, append);

    let bytes = match encoding {
        "base64" => match base64_decode(content) {
            Ok(b) => b,
            Err(e) => {
                return JsonRpcResponse::error(
                    id,
                    -32000,
                    format!("Invalid base64 content: {}", e),
                    None,
                );
            }
        },
        _ => content.as_bytes().to_vec(),
    };

    let result = if append {
        use std::io::Write;
        let mut file = match fs::OpenOptions::new().append(true).create(true).open(path) {
            Ok(f) => f,
            Err(e) => {
                return JsonRpcResponse::error(
                    id,
                    -32000,
                    format!("Failed to open file for appending: {}", e),
                    None,
                );
            }
        };
        file.write_all(&bytes)
    } else {
        fs::write(path, &bytes)
    };

    match result {
        Ok(()) => JsonRpcResponse::success(
            id,
            serde_json::json!({
                "path": file_path,
                "written": true,
                "size": bytes.len(),
                "append": append,
            }),
        ),
        Err(e) => JsonRpcResponse::error(
            id,
            -32000,
            format!("Failed to write file: {}", e),
            None,
        ),
    }
}

/// List directory contents
///
/// Params:
///   - path (required): absolute path to the directory
///   - recursive (optional): recurse into subdirectories (default false, max depth 3)
///   - includeHidden (optional): include hidden files (default false)
///   - limit (optional): maximum entries to return (default 1000)
async fn handle_list(params: &Value, id: Value) -> JsonRpcResponse {
    let dir_path = match params.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing 'path' parameter");
        }
    };

    let recursive = params
        .get("recursive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let include_hidden = params
        .get("includeHidden")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let limit = params
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(1000) as usize;

    let path = Path::new(dir_path);
    if !path.is_absolute() {
        return JsonRpcResponse::invalid_params(id, "Path must be absolute");
    }

    if !path.exists() {
        return JsonRpcResponse::error(
            id,
            -32002,
            format!("Directory not found: {}", dir_path),
            None,
        );
    }

    if !path.is_dir() {
        return JsonRpcResponse::error(
            id,
            -32002,
            format!("Not a directory: {}", dir_path),
            None,
        );
    }

    info!("Listing directory: {} (recursive={}, limit={})", dir_path, recursive, limit);

    let mut entries = Vec::new();
    let max_depth = if recursive { 3 } else { 1 };

    if let Err(e) = list_dir_recursive(path, &mut entries, include_hidden, limit, 0, max_depth) {
        warn!("Error during directory listing: {}", e);
    }

    let truncated = entries.len() >= limit;

    JsonRpcResponse::success(
        id,
        serde_json::json!({
            "path": dir_path,
            "entries": entries,
            "count": entries.len(),
            "truncated": truncated,
        }),
    )
}

/// Recursively list directory contents
fn list_dir_recursive(
    dir: &Path,
    entries: &mut Vec<Value>,
    include_hidden: bool,
    limit: usize,
    depth: usize,
    max_depth: usize,
) -> Result<(), String> {
    if entries.len() >= limit || depth >= max_depth {
        return Ok(());
    }

    let read_dir = fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry_result in read_dir {
        if entries.len() >= limit {
            break;
        }

        let entry = match entry_result {
            Ok(e) => e,
            Err(e) => {
                warn!("Failed to read directory entry: {}", e);
                continue;
            }
        };

        let file_name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files if not requested
        if !include_hidden && file_name.starts_with('.') {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(e) => {
                warn!("Failed to read metadata for {:?}: {}", entry.path(), e);
                continue;
            }
        };

        let entry_type = if metadata.is_dir() {
            "directory"
        } else if metadata.is_file() {
            "file"
        } else if metadata.is_symlink() {
            "symlink"
        } else {
            "other"
        };

        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .ok()
                    .map(|d| d.as_secs())
            });

        entries.push(serde_json::json!({
            "name": file_name,
            "path": entry.path().display().to_string(),
            "type": entry_type,
            "size": if metadata.is_file() { Some(metadata.len()) } else { None },
            "modified": modified,
        }));

        // Recurse into subdirectories
        if metadata.is_dir() && depth + 1 < max_depth {
            let _ = list_dir_recursive(&entry.path(), entries, include_hidden, limit, depth + 1, max_depth);
        }
    }

    Ok(())
}

// ============================================================================
// Base64 helpers (minimal, no external crate needed)
// ============================================================================

/// Simple base64 encoding using the standard alphabet
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    let chunks = data.chunks(3);

    for chunk in chunks {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };

        let n = (b0 << 16) | (b1 << 8) | b2;

        result.push(ALPHABET[((n >> 18) & 63) as usize] as char);
        result.push(ALPHABET[((n >> 12) & 63) as usize] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((n >> 6) & 63) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[(n & 63) as usize] as char);
        } else {
            result.push('=');
        }
    }

    result
}

/// Simple base64 decoding
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    fn decode_char(c: u8) -> Result<u8, String> {
        match c {
            b'A'..=b'Z' => Ok(c - b'A'),
            b'a'..=b'z' => Ok(c - b'a' + 26),
            b'0'..=b'9' => Ok(c - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            b'=' => Ok(0),
            _ => Err(format!("Invalid base64 character: {}", c as char)),
        }
    }

    let input = input.trim();
    if input.len() % 4 != 0 {
        return Err("Invalid base64 length".to_string());
    }

    let mut result = Vec::with_capacity(input.len() * 3 / 4);
    let bytes = input.as_bytes();

    for chunk in bytes.chunks(4) {
        let b0 = decode_char(chunk[0])?;
        let b1 = decode_char(chunk[1])?;
        let b2 = decode_char(chunk[2])?;
        let b3 = decode_char(chunk[3])?;

        let n = ((b0 as u32) << 18) | ((b1 as u32) << 12) | ((b2 as u32) << 6) | (b3 as u32);

        result.push((n >> 16) as u8);
        if chunk[2] != b'=' {
            result.push((n >> 8) as u8);
        }
        if chunk[3] != b'=' {
            result.push(n as u8);
        }
    }

    Ok(result)
}
