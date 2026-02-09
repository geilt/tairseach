//! Manifest Registry
//!
//! In-memory registry of loaded manifests with fast tool lookup and hot-reload support.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::loader;
use super::types::{Manifest, Tool};

/// In-memory manifest registry with fast tool lookup
pub struct ManifestRegistry {
    /// All loaded manifests, indexed by manifest ID
    manifests: RwLock<HashMap<String, Arc<Manifest>>>,
    /// Fast tool name → (manifest_id, tool_index) lookup
    tool_index: RwLock<HashMap<String, (String, usize)>>,
}

impl ManifestRegistry {
    pub fn new() -> Self {
        Self {
            manifests: RwLock::new(HashMap::new()),
            tool_index: RwLock::new(HashMap::new()),
        }
    }

    /// Load manifests from the default directory
    pub async fn load_from_disk(&self) -> Result<usize, String> {
        let base_dir = loader::default_manifest_dir();

        let mut all_manifests = Vec::new();

        // Load in precedence order (last wins)
        // 1. Core (lowest precedence)
        let core_dir = base_dir.join("core");
        all_manifests.extend(loader::load_manifests(&core_dir)?);

        // 2. Integrations
        let integrations_dir = base_dir.join("integrations");
        all_manifests.extend(loader::load_manifests(&integrations_dir)?);

        // 3. Community (highest precedence)
        let community_dir = base_dir.join("community");
        all_manifests.extend(loader::load_manifests(&community_dir)?);

        // Also load bundled manifests (lowest precedence, loaded first)
        if let Some(bundled_dir) = loader::bundled_manifest_dir() {
            let bundled = loader::load_manifests(&bundled_dir).unwrap_or_default();
            // Insert bundled at the beginning (so they have lowest precedence)
            all_manifests.splice(0..0, bundled);
        }

        // Build index (later manifests override earlier ones)
        let mut manifests = HashMap::new();
        let mut tool_index = HashMap::new();

        for manifest in all_manifests {
            let manifest_id = manifest.id.clone();

            // Index all tools from this manifest
            for (idx, tool) in manifest.tools.iter().enumerate() {
                // This will overwrite previous entries with same tool name (precedence)
                tool_index.insert(tool.name.clone(), (manifest_id.clone(), idx));
            }

            manifests.insert(manifest_id, Arc::new(manifest));
        }

        let count = manifests.len();
        let tool_count = tool_index.len();

        // Update registry atomically
        *self.manifests.write().await = manifests;
        *self.tool_index.write().await = tool_index;

        info!(
            "Manifest registry initialized: {} manifests, {} tools",
            count, tool_count
        );
        Ok(count)
    }

    /// Look up a tool by name
    pub async fn find_tool(&self, tool_name: &str) -> Option<(Arc<Manifest>, Tool)> {
        let tool_index = self.tool_index.read().await;
        let (manifest_id, tool_idx) = tool_index.get(tool_name)?;

        let manifests = self.manifests.read().await;
        let manifest = manifests.get(manifest_id)?;

        let tool = manifest.tools.get(*tool_idx)?.clone();
        Some((Arc::clone(manifest), tool))
    }

    /// List all registered manifests
    pub async fn list_manifests(&self) -> Vec<Arc<Manifest>> {
        let manifests = self.manifests.read().await;
        manifests.values().cloned().collect()
    }

    /// Get a specific manifest by ID
    pub async fn get_manifest(&self, id: &str) -> Option<Arc<Manifest>> {
        let manifests = self.manifests.read().await;
        manifests.get(id).cloned()
    }

    /// List all registered tool names
    pub async fn list_tool_names(&self) -> Vec<String> {
        let tool_index = self.tool_index.read().await;
        tool_index.keys().cloned().collect()
    }

    /// Start filesystem watcher for hot-reload
    pub async fn start_watcher(self: Arc<Self>) -> Result<(), String> {
        use notify::{Event, RecursiveMode, Watcher};

        let base_dir = loader::default_manifest_dir();

        info!("Starting manifest watcher on {:?}", base_dir);

        let (tx, mut rx) = tokio::sync::mpsc::channel::<Event>(100);

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.blocking_send(event);
            }
        })
        .map_err(|e| format!("Failed to create watcher: {}", e))?;

        watcher
            .watch(&base_dir, RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch directory: {}", e))?;

        // Spawn background task to handle events
        tokio::spawn(async move {
            // Keep watcher alive
            let _watcher = watcher;

            while let Some(event) = rx.recv().await {
                debug!("Manifest filesystem event: {:?}", event);

                // Debounce: wait a bit before reloading
                tokio::time::sleep(Duration::from_millis(200)).await;

                // Drain any pending events during debounce window
                while rx.try_recv().is_ok() {}

                info!("Manifest change detected, reloading...");
                if let Err(e) = self.load_from_disk().await {
                    warn!("Failed to reload manifests: {}", e);
                } else {
                    info!("Manifests reloaded successfully");
                }
            }
        });

        Ok(())
    }
}

impl Default for ManifestRegistry {
    fn default() -> Self {
        Self::new()
    }
}
