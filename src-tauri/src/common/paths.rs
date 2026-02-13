//! Path Utilities
//!
//! Common path resolution for Tairseach directories and files.

use std::path::PathBuf;

/// Get the Tairseach base directory (`~/.tairseach/`)
pub fn tairseach_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Could not determine home directory")?;
    Ok(home.join(".tairseach"))
}

/// Get a path within the Tairseach directory
///
/// # Example
/// ```ignore
/// let manifest_dir = tairseach_path("manifests")?;
/// let socket = tairseach_path("tairseach.sock")?;
/// ```
pub fn tairseach_path(relative_path: &str) -> Result<PathBuf, String> {
    Ok(tairseach_dir()?.join(relative_path))
}

/// Get the Tairseach socket path
pub fn socket_path() -> Result<PathBuf, String> {
    tairseach_path("tairseach.sock")
}

/// Get the manifest directory
pub fn manifest_dir() -> Result<PathBuf, String> {
    tairseach_path("manifests")
}

/// Get the logs directory
pub fn logs_dir() -> Result<PathBuf, String> {
    tairseach_path("logs")
}

/// Get the scripts directory
pub fn scripts_dir() -> Result<PathBuf, String> {
    tairseach_path("scripts")
}
