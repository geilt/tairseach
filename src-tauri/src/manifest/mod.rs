//! Manifest System
//!
//! Discovers, validates, loads, and hot-reloads capability manifests from disk.

pub mod loader;
pub mod registry;
pub mod types;

pub use loader::{default_manifest_dir, load_manifests};
pub use registry::ManifestRegistry;
pub use types::{
    Implementation, Manifest, ProxyAuth, ProxyToolBinding, Requirements, Tool, MANIFEST_VERSION,
};
