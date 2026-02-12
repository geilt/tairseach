//! Configuration management
//!
//! Handles reading and writing OpenClaw configuration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

fn get_openclaw_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(".openclaw")
}

fn get_openclaw_config_path() -> PathBuf {
    get_openclaw_dir().join("openclaw.json")
}

fn get_node_config_path() -> PathBuf {
    get_openclaw_dir().join("node.json")
}

fn get_exec_approvals_path() -> PathBuf {
    get_openclaw_dir().join("exec-approvals.json")
}

fn get_tairseach_auth_path() -> PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(".tairseach")
        .join("auth")
}

fn get_google_oauth_config_path() -> PathBuf {
    get_tairseach_auth_path().join("google_oauth.json")
}

fn get_onepassword_config_path() -> PathBuf {
    get_tairseach_auth_path().join("onepassword.json")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleOAuthStatus {
    pub status: String,
    pub configured: bool,
    pub has_token: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnePasswordConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_vault_id: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawConfig {
    pub raw: Value,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    pub environment_type: String,
    pub files: Vec<FileInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub config: Value,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecApprovals {
    pub approvals: Value,
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

#[tauri::command]
pub async fn get_google_oauth_config() -> Result<Option<GoogleOAuthConfig>, String> {
    let path = get_google_oauth_config_path();
    if !path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read Google OAuth config: {}", e))?;
    let config: GoogleOAuthConfig = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse Google OAuth config: {}", e))?;

    Ok(Some(config))
}

#[tauri::command]
pub async fn save_google_oauth_config(client_id: String, client_secret: String) -> Result<(), String> {
    if client_id.trim().is_empty() || client_secret.trim().is_empty() {
        return Err("Client ID and Client Secret are required".to_string());
    }

    let dir = get_tairseach_auth_path();
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create auth directory: {}", e))?;

    let config = GoogleOAuthConfig {
        client_id: client_id.trim().to_string(),
        client_secret: client_secret.trim().to_string(),
        updated_at: Utc::now().to_rfc3339(),
    };

    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize Google OAuth config: {}", e))?;
    std::fs::write(get_google_oauth_config_path(), content)
        .map_err(|e| format!("Failed to write Google OAuth config: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn test_google_oauth_config(client_id: String, client_secret: String) -> Result<Value, String> {
    if client_id.trim().is_empty() || client_secret.trim().is_empty() {
        return Err("Client ID and Client Secret are required".to_string());
    }

    let response = reqwest::Client::new()
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id.trim()),
            ("client_secret", client_secret.trim()),
            ("grant_type", "refresh_token"),
            ("refresh_token", "invalid-test-token"),
        ])
        .send()
        .await
        .map_err(|e| format!("Failed to reach Google OAuth endpoint: {}", e))?;

    let status_code = response.status().as_u16();
    let body: Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid Google OAuth response: {}", e))?;

    let err = body
        .get("error")
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    let valid_client = err == "invalid_grant";

    Ok(serde_json::json!({
        "ok": valid_client,
        "statusCode": status_code,
        "error": err,
        "details": body,
        "message": if valid_client {
            "Credentials accepted by Google."
        } else {
            "Google rejected these OAuth client credentials."
        }
    }))
}

#[tauri::command]
pub async fn get_google_oauth_status() -> Result<GoogleOAuthStatus, String> {
    let config = get_google_oauth_config().await?;
    if config.is_none() {
        return Ok(GoogleOAuthStatus {
            status: "not_configured".to_string(),
            configured: false,
            has_token: false,
            message: "Not configured".to_string(),
        });
    }

    let metadata_path = get_tairseach_auth_path().join("metadata.json");
    if metadata_path.exists() {
        let metadata_raw = std::fs::read_to_string(&metadata_path)
            .map_err(|e| format!("Failed to read auth metadata: {}", e))?;
        let metadata: Value = serde_json::from_str(&metadata_raw)
            .map_err(|e| format!("Failed to parse auth metadata: {}", e))?;

        if let Some(account) = metadata
            .get("accounts")
            .and_then(|v| v.as_array())
            .and_then(|accounts| {
                accounts.iter().find(|entry| {
                    entry
                        .get("provider")
                        .and_then(|v| v.as_str())
                        .map(|p| p == "google")
                        .unwrap_or(false)
                })
            })
        {
            if let Some(last_used) = account.get("last_used").and_then(|v| v.as_str()) {
                if let Ok(parsed) = DateTime::parse_from_rfc3339(last_used) {
                    let age = Utc::now().signed_duration_since(parsed.with_timezone(&Utc));
                    if age.num_days() > 30 {
                        return Ok(GoogleOAuthStatus {
                            status: "token_expired".to_string(),
                            configured: true,
                            has_token: true,
                            message: "Token expired".to_string(),
                        });
                    }
                }
            }

            return Ok(GoogleOAuthStatus {
                status: "connected".to_string(),
                configured: true,
                has_token: true,
                message: "Connected".to_string(),
            });
        }
    }

    Ok(GoogleOAuthStatus {
        status: "connected".to_string(),
        configured: true,
        has_token: false,
        message: "Connected".to_string(),
    })
}

#[tauri::command]
pub async fn get_environment() -> Result<EnvironmentInfo, String> {
    let gateway_path = get_openclaw_config_path();
    let node_path = get_node_config_path();
    let exec_approvals_path = get_exec_approvals_path();
    
    let is_gateway = gateway_path.exists();
    let is_node = node_path.exists();
    
    let environment_type = if is_gateway {
        "gateway".to_string()
    } else if is_node {
        "node".to_string()
    } else {
        "unknown".to_string()
    };
    
    let mut files = Vec::new();
    if gateway_path.exists() {
        files.push(FileInfo {
            name: "openclaw.json".to_string(),
            path: gateway_path.display().to_string(),
        });
    }
    if node_path.exists() {
        files.push(FileInfo {
            name: "node.json".to_string(),
            path: node_path.display().to_string(),
        });
    }
    if exec_approvals_path.exists() {
        files.push(FileInfo {
            name: "exec-approvals.json".to_string(),
            path: exec_approvals_path.display().to_string(),
        });
    }
    
    Ok(EnvironmentInfo {
        environment_type,
        files,
    })
}

#[tauri::command]
pub async fn get_node_config() -> Result<NodeConfig, String> {
    let node_path = get_node_config_path();
    
    if !node_path.exists() {
        return Err(format!("Node config not found at {:?}", node_path));
    }
    
    let content = std::fs::read_to_string(&node_path)
        .map_err(|e| format!("Failed to read node config: {}", e))?;
    
    let config: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse node config JSON: {}", e))?;
    
    Ok(NodeConfig {
        config,
        path: node_path.display().to_string(),
    })
}

#[tauri::command]
pub async fn set_node_config(config: Value) -> Result<(), String> {
    let node_path = get_node_config_path();
    
    // Backup existing config
    if node_path.exists() {
        let backup_path = node_path.with_extension("json.bak");
        std::fs::copy(&node_path, &backup_path)
            .map_err(|e| format!("Failed to create backup: {}", e))?;
    }
    
    // Ensure parent directory exists
    if let Some(parent) = node_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    
    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize node config: {}", e))?;
    
    std::fs::write(&node_path, content)
        .map_err(|e| format!("Failed to write node config: {}", e))?;
    
    Ok(())
}

#[tauri::command]
pub async fn get_exec_approvals() -> Result<ExecApprovals, String> {
    let approvals_path = get_exec_approvals_path();
    
    let approvals: Value = if approvals_path.exists() {
        let content = std::fs::read_to_string(&approvals_path)
            .map_err(|e| format!("Failed to read exec approvals: {}", e))?;
        
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse exec approvals JSON: {}", e))?
    } else {
        // Return empty array if file doesn't exist
        serde_json::json!([])
    };
    
    Ok(ExecApprovals {
        approvals,
        path: approvals_path.display().to_string(),
    })
}

#[tauri::command]
pub async fn set_exec_approvals(approvals: Value) -> Result<(), String> {
    let approvals_path = get_exec_approvals_path();
    
    // Backup existing config
    if approvals_path.exists() {
        let backup_path = approvals_path.with_extension("json.bak");
        std::fs::copy(&approvals_path, &backup_path)
            .map_err(|e| format!("Failed to create backup: {}", e))?;
    }
    
    // Ensure parent directory exists
    if let Some(parent) = approvals_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    
    let content = serde_json::to_string_pretty(&approvals)
        .map_err(|e| format!("Failed to serialize exec approvals: {}", e))?;
    
    std::fs::write(&approvals_path, content)
        .map_err(|e| format!("Failed to write exec approvals: {}", e))?;
    
    Ok(())
}

/// Get 1Password configuration
pub async fn get_onepassword_config() -> Result<Option<OnePasswordConfig>, String> {
    let path = get_onepassword_config_path();
    if !path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read 1Password config: {}", e))?;
    let config: OnePasswordConfig = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse 1Password config: {}", e))?;

    Ok(Some(config))
}

/// Save 1Password configuration
pub async fn save_onepassword_config(default_vault_id: Option<String>) -> Result<(), String> {
    let dir = get_tairseach_auth_path();
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create auth directory: {}", e))?;

    let config = OnePasswordConfig {
        default_vault_id,
        updated_at: Utc::now().to_rfc3339(),
    };

    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize 1Password config: {}", e))?;
    std::fs::write(get_onepassword_config_path(), content)
        .map_err(|e| format!("Failed to write 1Password config: {}", e))?;

    Ok(())
}
