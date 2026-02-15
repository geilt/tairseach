//! Profile management
//!
//! Handles agent, tool, and MCP server profiles.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub profile_type: String,
    pub config: serde_json::Value,
    pub enabled: bool,
}

#[tauri::command]
pub async fn profiles_all_list() -> Result<Vec<Profile>, String> {
    // TODO: Load profiles from storage
    Ok(vec![])
}

#[tauri::command]
pub async fn profiles_single_save(profile: Profile) -> Result<Profile, String> {
    // TODO: Save profile to storage
    Ok(profile)
}
