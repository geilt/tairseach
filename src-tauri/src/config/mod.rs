//! Configuration management
//!
//! Handles reading and writing OpenClaw configuration.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

fn get_openclaw_config_path() -> PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(".openclaw")
        .join("openclaw.json")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawConfig {
    pub raw: Value,
    pub path: String,
}

#[tauri::command]
pub async fn get_config() -> Result<OpenClawConfig, String> {
    let config_path = get_openclaw_config_path();
    
    if !config_path.exists() {
        return Err(format!("Config file not found at {:?}", config_path));
    }
    
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    
    let raw: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config JSON: {}", e))?;
    
    Ok(OpenClawConfig {
        raw,
        path: config_path.display().to_string(),
    })
}

#[tauri::command]
pub async fn set_config(config: Value) -> Result<(), String> {
    let config_path = get_openclaw_config_path();
    
    // Backup existing config
    if config_path.exists() {
        let backup_path = config_path.with_extension("json.bak");
        std::fs::copy(&config_path, &backup_path)
            .map_err(|e| format!("Failed to create backup: {}", e))?;
    }
    
    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    
    std::fs::write(&config_path, content)
        .map_err(|e| format!("Failed to write config: {}", e))?;
    
    Ok(())
}

/// Get available models for known providers
#[tauri::command]
pub async fn get_provider_models() -> Result<Value, String> {
    // Return known models for common providers
    Ok(serde_json::json!({
        "anthropic": [
            { "id": "claude-opus-4-5", "name": "Claude Opus 4.5", "description": "Most capable model" },
            { "id": "claude-sonnet-4-5", "name": "Claude Sonnet 4.5", "description": "Balanced performance" },
            { "id": "claude-haiku", "name": "Claude Haiku", "description": "Fast and efficient" }
        ],
        "openai": [
            { "id": "gpt-4o", "name": "GPT-4o", "description": "Latest GPT-4 omni model" },
            { "id": "gpt-4-turbo", "name": "GPT-4 Turbo", "description": "Fast GPT-4" },
            { "id": "gpt-3.5-turbo", "name": "GPT-3.5 Turbo", "description": "Fast and affordable" }
        ],
        "openai-codex": [
            { "id": "gpt-5.2", "name": "GPT 5.2", "description": "Latest Codex model" },
            { "id": "o3-mini", "name": "o3 Mini", "description": "Reasoning model" }
        ],
        "google": [
            { "id": "gemini-2.0-flash", "name": "Gemini 2.0 Flash", "description": "Fast multimodal" },
            { "id": "gemini-1.5-pro", "name": "Gemini 1.5 Pro", "description": "Long context" }
        ],
        "kimi-coding": [
            { "id": "k2p5", "name": "Kimi K2P5", "description": "Coding model" }
        ]
    }))
}
