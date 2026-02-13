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

/// Read and deserialize a JSON file, returning a descriptive error on failure.
fn read_json_file<T: serde::de::DeserializeOwned>(path: &PathBuf, label: &str) -> Result<T, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", label, e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse {} JSON: {}", label, e))
}

/// Write a JSON value to a file, creating a `.bak` backup if the file already exists.
/// Ensures parent directories exist.
fn write_json_file_with_backup(path: &PathBuf, value: &impl Serialize, label: &str) -> Result<(), String> {
    if path.exists() {
        let backup_path = path.with_extension("json.bak");
        std::fs::copy(path, &backup_path)
            .map_err(|e| format!("Failed to create {} backup: {}", label, e))?;
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create {} directory: {}", label, e))?;
    }

    let content = serde_json::to_string_pretty(value)
        .map_err(|e| format!("Failed to serialize {}: {}", label, e))?;
    std::fs::write(path, content)
        .map_err(|e| format!("Failed to write {}: {}", label, e))
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
    
    let raw: Value = read_json_file(&config_path, "config")?;
    
    Ok(OpenClawConfig {
        raw,
        path: config_path.display().to_string(),
    })
}

#[tauri::command]
pub async fn set_config(config: Value) -> Result<(), String> {
    let config_path = get_openclaw_config_path();
    write_json_file_with_backup(&config_path, &config, "config")
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

    let config: GoogleOAuthConfig = read_json_file(&path, "Google OAuth config")?;
    Ok(Some(config))
}

#[tauri::command]
pub async fn save_google_oauth_config(client_id: String, client_secret: String) -> Result<(), String> {
    if client_id.trim().is_empty() || client_secret.trim().is_empty() {
        return Err("Client ID and Client Secret are required".to_string());
    }

    let config = GoogleOAuthConfig {
        client_id: client_id.trim().to_string(),
        client_secret: client_secret.trim().to_string(),
        updated_at: Utc::now().to_rfc3339(),
    };

    let path = get_google_oauth_config_path();
    write_json_file_with_backup(&path, &config, "Google OAuth config")
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
        let metadata: Value = read_json_file(&metadata_path, "auth metadata")?;

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
    
    let config: Value = read_json_file(&node_path, "node config")?;
    
    Ok(NodeConfig {
        config,
        path: node_path.display().to_string(),
    })
}

#[tauri::command]
pub async fn set_node_config(config: Value) -> Result<(), String> {
    let node_path = get_node_config_path();
    write_json_file_with_backup(&node_path, &config, "node config")
}

#[tauri::command]
pub async fn get_exec_approvals() -> Result<ExecApprovals, String> {
    let approvals_path = get_exec_approvals_path();
    
    let approvals: Value = if approvals_path.exists() {
        read_json_file(&approvals_path, "exec approvals")?
    } else {
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
    write_json_file_with_backup(&approvals_path, &approvals, "exec approvals")
}

/// Get 1Password configuration
pub async fn get_onepassword_config() -> Result<Option<OnePasswordConfig>, String> {
    let path = get_onepassword_config_path();
    if !path.exists() {
        return Ok(None);
    }

    let config: OnePasswordConfig = read_json_file(&path, "1Password config")?;
    Ok(Some(config))
}

/// Save 1Password configuration
pub async fn save_onepassword_config(default_vault_id: Option<String>) -> Result<(), String> {
    let config = OnePasswordConfig {
        default_vault_id,
        updated_at: Utc::now().to_rfc3339(),
    };

    let path = get_onepassword_config_path();
    write_json_file_with_backup(&path, &config, "1Password config")
}
