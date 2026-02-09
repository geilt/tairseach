//! Manifest Loader
//!
//! Scans manifest directories, parses JSON, validates, returns loaded manifests.

use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

use super::types::Manifest;

/// Load all manifests from a directory tree (recursive)
pub fn load_manifests(base_dir: &Path) -> Result<Vec<Manifest>, String> {
    let mut manifests = Vec::new();

    if !base_dir.exists() {
        info!("Manifest directory does not exist: {:?}", base_dir);
        return Ok(manifests);
    }

    load_manifests_recursive(base_dir, &mut manifests)?;

    info!("Loaded {} manifests from {:?}", manifests.len(), base_dir);
    Ok(manifests)
}

fn load_manifests_recursive(dir: &Path, manifests: &mut Vec<Manifest>) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }

    let entries = fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory {:?}: {}", dir, e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();

        // Skip dotfiles and temp files
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') || name.ends_with(".tmp") || name.ends_with(".swp") {
                continue;
            }
        }

        if path.is_dir() {
            load_manifests_recursive(&path, manifests)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
            match load_manifest_file(&path) {
                Ok(manifest) => manifests.push(manifest),
                Err(e) => {
                    warn!("Failed to load manifest {:?}: {}", path, e);
                    // Continue loading other manifests
                }
            }
        }
    }

    Ok(())
}

fn load_manifest_file(path: &Path) -> Result<Manifest, String> {
    // Check file size (max 1MB)
    let metadata = fs::metadata(path).map_err(|e| format!("Failed to stat file: {}", e))?;
    if metadata.len() > 1_000_000 {
        return Err("Manifest file too large (max 1MB)".to_string());
    }

    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let manifest: Manifest = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    manifest.validate()?;

    info!("Loaded manifest: {} ({})", manifest.name, manifest.id);
    Ok(manifest)
}

/// Get default manifest directory
pub fn default_manifest_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Could not determine home directory")
        .join(".tairseach")
        .join("manifests")
}

/// Get bundled manifests directory (app bundle)
#[cfg(target_os = "macos")]
pub fn bundled_manifest_dir() -> Option<PathBuf> {
    // Get the app bundle path (Resources directory)
    std::env::current_exe()
        .ok()
        .and_then(|exe| {
            exe.parent()
                .and_then(|p| p.parent())
                .map(|p| p.join("Resources").join("manifests").join("core"))
        })
}

#[cfg(not(target_os = "macos"))]
pub fn bundled_manifest_dir() -> Option<PathBuf> {
    None
}
