//! Files Handler
//!
//! Handles file operations via Tairseach's Full Disk Access permission.
//! Uses standard Rust `std::fs` — the value is that Tairseach has FDA,
//! so it can access paths that agents cannot (e.g., ~/Library/Mail, etc.).

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde_json::Value;
use std::fs;
use std::path::Path;
use tracing::{info, warn};

use super::common::*;
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
        let tairseach_dir = home.join(".tairseach");
        if canonical.starts_with(&tairseach_dir) {
            return Err("Writes to Tairseach configuration directory are not allowed".to_string());
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
        _ => method_not_found(id, &format!("files.{}", action)),
    }
}

/// Read a file and return its contents
///
/// Params:
///   - path (required): absolute path to the file
///   - encoding (optional): "utf8" (default) or "base64"
///   - maxSize (optional): maximum bytes to read (default 10MB)
async fn handle_read(params: &Value, id: Value) -> JsonRpcResponse {
    let file_path = match require_string(params, "path", &id) {
        Ok(p) => p,
        Err(response) => return response,
    };

    let encoding = string_with_default(params, "encoding", "utf8");
    let max_size = u64_with_default(params, "maxSize", MAX_READ_SIZE);

    // Validate the path
    let path = Path::new(file_path);
    if !path.is_absolute() {
        return invalid_params(id, "Path must be absolute");
    }

    if !path.exists() {
        return error(id, -32002, format!("File not found: {}", file_path));
    }

    if !path.is_file() {
        return error(id, -32002, format!("Not a file: {}", file_path));
    }

    // SECURITY: Block reads to Tairseach config directory
    // Agents should use auth.get / auth.list instead
    if let Some(home) = dirs::home_dir() {
        let tairseach_dir = home.join(".tairseach");
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if canonical.starts_with(&tairseach_dir) {
            return error(
                id,
                -32004,
                "Reads from Tairseach configuration directory are not allowed. Use auth.* methods instead.",
            );
        }
    }

    // Check file size
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            return generic_error(id, format!("Failed to read file metadata: {}", e));
        }
    };

    if metadata.len() > max_size {
        return error(
            id,
            -32003,
            format!(
                "File too large: {} bytes (max: {} bytes). Use 'maxSize' to increase limit.",
                metadata.len(),
                max_size
            ),
        );
    }

    info!("Reading file: {} (encoding={})", file_path, encoding);

    match encoding {
        "base64" => {
            use std::io::Read;
            let mut file = match fs::File::open(path) {
                Ok(f) => f,
                Err(e) => {
                    return generic_error(id, format!("Failed to open file: {}", e));
                }
            };
            let mut buffer = Vec::new();
            if let Err(e) = file.read_to_end(&mut buffer) {
                return generic_error(id, format!("Failed to read file: {}", e));
            }

            ok(
                id,
                serde_json::json!({
                    "path": file_path,
                    "encoding": "base64",
                    "content": BASE64.encode(&buffer),
                    "size": metadata.len(),
                }),
            )
        }
        _ => {
            // UTF-8 text
            match fs::read_to_string(path) {
                Ok(content) => ok(
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
                    generic_error(id, format!("Failed to read as UTF-8: {}. Try encoding='base64'.", e))
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
    let file_path = match require_string(params, "path", &id) {
        Ok(p) => p,
        Err(response) => return response,
    };

    let content = match require_string(params, "content", &id) {
        Ok(c) => c,
        Err(response) => return response,
    };

    let encoding = string_with_default(params, "encoding", "utf8");
    let create_dirs = bool_with_default(params, "createDirs", false);
    let append = bool_with_default(params, "append", false);

    // Validate the path
    let path = Path::new(file_path);
    if !path.is_absolute() {
        return invalid_params(id, "Path must be absolute");
    }

    // SECURITY: Check path restrictions (deny writes to critical system paths)
    if let Err(e) = validate_write_path(path) {
        return error(id, -32004, format!("Path not allowed for writing: {}", e));
    }

    // Create parent directories if requested
    if create_dirs {
        if let Some(parent) = path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return generic_error(id, format!("Failed to create directories: {}", e));
            }
        }
    }

    info!("Writing file: {} (encoding={}, append={})", file_path, encoding, append);

    let bytes = match encoding {
        "base64" => match BASE64.decode(content) {
            Ok(b) => b,
            Err(e) => {
                return generic_error(id, format!("Invalid base64 content: {}", e));
            }
        },
        _ => content.as_bytes().to_vec(),
    };

    let result = if append {
        use std::io::Write;
        let mut file = match fs::OpenOptions::new().append(true).create(true).open(path) {
            Ok(f) => f,
            Err(e) => {
                return generic_error(id, format!("Failed to open file for appending: {}", e));
            }
        };
        file.write_all(&bytes)
    } else {
        fs::write(path, &bytes)
    };

    match result {
        Ok(()) => ok(
            id,
            serde_json::json!({
                "path": file_path,
                "written": true,
                "size": bytes.len(),
                "append": append,
            }),
        ),
        Err(e) => generic_error(id, format!("Failed to write file: {}", e)),
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
    let dir_path = match require_string(params, "path", &id) {
        Ok(p) => p,
        Err(response) => return response,
    };

    let recursive = bool_with_default(params, "recursive", false);
    let include_hidden = bool_with_default(params, "includeHidden", false);
    let limit = u64_with_default(params, "limit", 1000) as usize;

    let path = Path::new(dir_path);
    if !path.is_absolute() {
        return invalid_params(id, "Path must be absolute");
    }

    if !path.exists() {
        return error(id, -32002, format!("Directory not found: {}", dir_path));
    }

    if !path.is_dir() {
        return error(id, -32002, format!("Not a directory: {}", dir_path));
    }

    info!("Listing directory: {} (recursive={}, limit={})", dir_path, recursive, limit);

    let mut entries = Vec::new();
    let max_depth = if recursive { 3 } else { 1 };

    if let Err(e) = list_dir_recursive(path, &mut entries, include_hidden, limit, 0, max_depth) {
        warn!("Error during directory listing: {}", e);
    }

    let truncated = entries.len() >= limit;

    ok(
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
