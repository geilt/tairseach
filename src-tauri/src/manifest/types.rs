//! Manifest Types
//!
//! Rust structs matching the manifest JSON schema v1.0.0.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Manifest schema version
pub const MANIFEST_VERSION: &str = "1.0.0";

/// Capability manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub manifest_version: String,
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub category: String,
    #[serde(default)]
    pub requires: Requirements,
    pub tools: Vec<Tool>,
    pub implementation: Implementation,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Requirements {
    #[serde(default)]
    pub credentials: Vec<CredentialRequirement>,
    #[serde(default)]
    pub permissions: Vec<PermissionRequirement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialRequirement {
    pub id: String,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequirement {
    pub name: String,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    #[serde(default)]
    pub title: Option<String>,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
    #[serde(rename = "outputSchema")]
    pub output_schema: serde_json::Value,
    #[serde(default)]
    pub annotations: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub requires: Option<Requirements>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Implementation {
    Internal {
        module: String,
        #[serde(default)]
        methods: HashMap<String, String>,
    },
    Script {
        runtime: String,
        entrypoint: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: HashMap<String, String>,
        #[serde(rename = "toolBindings")]
        tool_bindings: HashMap<String, ScriptToolBinding>,
    },
    Proxy {
        #[serde(rename = "baseUrl")]
        base_url: String,
        auth: ProxyAuth,
        #[serde(rename = "toolBindings")]
        tool_bindings: HashMap<String, ProxyToolBinding>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptToolBinding {
    pub action: String,
    #[serde(default)]
    pub input_mode: Option<String>,
    #[serde(default)]
    pub output_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyAuth {
    pub strategy: String,
    #[serde(rename = "credentialId")]
    pub credential_id: String,
    #[serde(default, rename = "headerName")]
    pub header_name: Option<String>,
    #[serde(default, rename = "queryParam")]
    pub query_param: Option<String>,
    #[serde(default, rename = "tokenField")]
    pub token_field: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyToolBinding {
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub query: HashMap<String, String>,
    #[serde(default, rename = "bodyTemplate")]
    pub body_template: Option<serde_json::Value>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default, rename = "responsePath")]
    pub response_path: Option<String>,
}

impl Manifest {
    /// Validate manifest structure
    pub fn validate(&self) -> Result<(), String> {
        if self.manifest_version != MANIFEST_VERSION {
            return Err(format!(
                "Unsupported manifest version: {} (expected {})",
                self.manifest_version, MANIFEST_VERSION
            ));
        }

        if self.id.is_empty() {
            return Err("Manifest ID cannot be empty".to_string());
        }

        if self.tools.is_empty() {
            return Err("Manifest must define at least one tool".to_string());
        }

        // Validate tool names (must be valid identifiers)
        for tool in &self.tools {
            if !is_valid_tool_name(&tool.name) {
                return Err(format!("Invalid tool name: {}", tool.name));
            }
        }

        // Validate implementation has bindings for all tools
        match &self.implementation {
            Implementation::Internal { methods, .. } => {
                for tool in &self.tools {
                    if !methods.contains_key(&tool.name) {
                        return Err(format!(
                            "Internal implementation missing method for tool: {}",
                            tool.name
                        ));
                    }
                }
            }
            Implementation::Script { tool_bindings, .. } => {
                for tool in &self.tools {
                    if !tool_bindings.contains_key(&tool.name) {
                        return Err(format!(
                            "Script implementation missing binding for tool: {}",
                            tool.name
                        ));
                    }
                }
            }
            Implementation::Proxy { tool_bindings, .. } => {
                for tool in &self.tools {
                    if !tool_bindings.contains_key(&tool.name) {
                        return Err(format!(
                            "Proxy implementation missing binding for tool: {}",
                            tool.name
                        ));
                    }
                }
            }
        }

        Ok(())
    }
}

fn is_valid_tool_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        && name.chars().next().unwrap().is_ascii_alphabetic()
}
