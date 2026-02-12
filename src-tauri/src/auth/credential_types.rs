//! Credential Type System
//!
//! Defines credential schemas for different integration types.
//! Each credential type specifies required fields, display names, and validation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Field type for credential schemas
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    String,
    Secret,
}

/// A single field in a credential schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialField {
    pub name: String,
    pub display_name: String,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Credential type schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialTypeSchema {
    pub provider_type: String,
    pub display_name: String,
    pub description: String,
    pub fields: Vec<CredentialField>,
    pub supports_multiple: bool,
    #[serde(default)]
    pub built_in: bool,
}

impl CredentialTypeSchema {
    /// Validate that a credential map contains all required fields
    pub fn validate(&self, fields: &HashMap<String, String>) -> Result<(), String> {
        for field in &self.fields {
            if field.required && !fields.contains_key(&field.name) {
                return Err(format!("Missing required field: {}", field.name));
            }
        }
        Ok(())
    }
}

/// Credential Type Registry
pub struct CredentialTypeRegistry {
    schemas: HashMap<String, CredentialTypeSchema>,
}

impl CredentialTypeRegistry {
    /// Create a new registry with built-in schemas
    pub fn new() -> Self {
        let mut registry = Self {
            schemas: HashMap::new(),
        };
        
        // Register built-in types
        registry.register_built_in(Self::onepassword_schema());
        registry.register_built_in(Self::jira_schema());
        registry.register_built_in(Self::oura_schema());
        registry.register_built_in(Self::github_schema());
        registry.register_built_in(Self::linear_schema());
        registry.register_built_in(Self::notion_schema());
        registry.register_built_in(Self::slack_schema());
        
        registry
    }
    
    /// Register a built-in schema
    fn register_built_in(&mut self, mut schema: CredentialTypeSchema) {
        schema.built_in = true;
        self.schemas.insert(schema.provider_type.clone(), schema);
    }
    
    /// Register a custom credential type
    pub fn register_custom(&mut self, schema: CredentialTypeSchema) -> Result<(), String> {
        if schema.built_in {
            return Err("Cannot register built-in schema as custom".to_string());
        }
        
        if self.schemas.contains_key(&schema.provider_type) {
            return Err(format!(
                "Credential type '{}' already exists",
                schema.provider_type
            ));
        }
        
        self.schemas.insert(schema.provider_type.clone(), schema);
        Ok(())
    }
    
    /// Get a schema by provider type
    pub fn get(&self, provider_type: &str) -> Option<&CredentialTypeSchema> {
        self.schemas.get(provider_type)
    }
    
    /// List all registered schemas
    pub fn list(&self) -> Vec<&CredentialTypeSchema> {
        self.schemas.values().collect()
    }
    
    /// Remove a custom credential type
    pub fn remove_custom(&mut self, provider_type: &str) -> Result<(), String> {
        match self.schemas.get(provider_type) {
            Some(schema) if schema.built_in => {
                Err("Cannot remove built-in credential type".to_string())
            }
            Some(_) => {
                self.schemas.remove(provider_type);
                Ok(())
            }
            None => Err(format!("Credential type '{}' not found", provider_type)),
        }
    }
    
    // ── Built-in Schemas ────────────────────────────────────────────────────
    
    fn onepassword_schema() -> CredentialTypeSchema {
        CredentialTypeSchema {
            provider_type: "onepassword".to_string(),
            display_name: "1Password Service Account".to_string(),
            description: "1Password Service Account token for API access".to_string(),
            fields: vec![CredentialField {
                name: "service_account_token".to_string(),
                display_name: "Service Account Token".to_string(),
                field_type: FieldType::Secret,
                required: true,
                description: Some("Service account token from 1Password".to_string()),
            }],
            supports_multiple: true,
            built_in: false,
        }
    }
    
    fn jira_schema() -> CredentialTypeSchema {
        CredentialTypeSchema {
            provider_type: "jira".to_string(),
            display_name: "Jira Cloud".to_string(),
            description: "Jira Cloud API credentials".to_string(),
            fields: vec![
                CredentialField {
                    name: "host".to_string(),
                    display_name: "Jira Host".to_string(),
                    field_type: FieldType::String,
                    required: true,
                    description: Some("Jira instance hostname (e.g. company.atlassian.net)".to_string()),
                },
                CredentialField {
                    name: "email".to_string(),
                    display_name: "Email".to_string(),
                    field_type: FieldType::String,
                    required: true,
                    description: Some("User email for authentication".to_string()),
                },
                CredentialField {
                    name: "api_token".to_string(),
                    display_name: "API Token".to_string(),
                    field_type: FieldType::Secret,
                    required: true,
                    description: Some("API token from Atlassian account settings".to_string()),
                },
            ],
            supports_multiple: true,
            built_in: false,
        }
    }
    
    fn oura_schema() -> CredentialTypeSchema {
        CredentialTypeSchema {
            provider_type: "oura".to_string(),
            display_name: "Oura Ring".to_string(),
            description: "Oura API access token".to_string(),
            fields: vec![CredentialField {
                name: "access_token".to_string(),
                display_name: "Access Token".to_string(),
                field_type: FieldType::Secret,
                required: true,
                description: Some("Personal Access Token from Oura Cloud".to_string()),
            }],
            supports_multiple: false,
            built_in: false,
        }
    }
    
    fn github_schema() -> CredentialTypeSchema {
        CredentialTypeSchema {
            provider_type: "github".to_string(),
            display_name: "GitHub".to_string(),
            description: "GitHub personal access token".to_string(),
            fields: vec![CredentialField {
                name: "access_token".to_string(),
                display_name: "Personal Access Token".to_string(),
                field_type: FieldType::Secret,
                required: true,
                description: Some("GitHub PAT with appropriate scopes".to_string()),
            }],
            supports_multiple: true,
            built_in: false,
        }
    }
    
    fn linear_schema() -> CredentialTypeSchema {
        CredentialTypeSchema {
            provider_type: "linear".to_string(),
            display_name: "Linear".to_string(),
            description: "Linear API key".to_string(),
            fields: vec![CredentialField {
                name: "api_key".to_string(),
                display_name: "API Key".to_string(),
                field_type: FieldType::Secret,
                required: true,
                description: Some("Personal API key from Linear settings".to_string()),
            }],
            supports_multiple: false,
            built_in: false,
        }
    }
    
    fn notion_schema() -> CredentialTypeSchema {
        CredentialTypeSchema {
            provider_type: "notion".to_string(),
            display_name: "Notion".to_string(),
            description: "Notion integration token".to_string(),
            fields: vec![CredentialField {
                name: "access_token".to_string(),
                display_name: "Integration Token".to_string(),
                field_type: FieldType::Secret,
                required: true,
                description: Some("Internal integration secret from Notion".to_string()),
            }],
            supports_multiple: true,
            built_in: false,
        }
    }
    
    fn slack_schema() -> CredentialTypeSchema {
        CredentialTypeSchema {
            provider_type: "slack".to_string(),
            display_name: "Slack".to_string(),
            description: "Slack bot or user token".to_string(),
            fields: vec![
                CredentialField {
                    name: "token".to_string(),
                    display_name: "Bot/User Token".to_string(),
                    field_type: FieldType::Secret,
                    required: true,
                    description: Some("Bot token (xoxb-*) or user token (xoxp-*)".to_string()),
                },
                CredentialField {
                    name: "workspace".to_string(),
                    display_name: "Workspace".to_string(),
                    field_type: FieldType::String,
                    required: false,
                    description: Some("Workspace name or ID (optional)".to_string()),
                },
            ],
            supports_multiple: true,
            built_in: false,
        }
    }
}

impl Default for CredentialTypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_registry_has_built_ins() {
        let registry = CredentialTypeRegistry::new();
        assert!(registry.get("onepassword").is_some());
        assert!(registry.get("jira").is_some());
        assert!(registry.get("oura").is_some());
    }
    
    #[test]
    fn test_onepassword_schema() {
        let registry = CredentialTypeRegistry::new();
        let schema = registry.get("onepassword").unwrap();
        
        assert_eq!(schema.provider_type, "onepassword");
        assert_eq!(schema.fields.len(), 1);
        assert_eq!(schema.fields[0].name, "service_account_token");
        assert_eq!(schema.fields[0].field_type, FieldType::Secret);
        assert!(schema.supports_multiple);
    }
    
    #[test]
    fn test_jira_schema() {
        let registry = CredentialTypeRegistry::new();
        let schema = registry.get("jira").unwrap();
        
        assert_eq!(schema.provider_type, "jira");
        assert_eq!(schema.fields.len(), 3);
        
        let field_names: Vec<&str> = schema.fields.iter().map(|f| f.name.as_str()).collect();
        assert!(field_names.contains(&"host"));
        assert!(field_names.contains(&"email"));
        assert!(field_names.contains(&"api_token"));
    }
    
    #[test]
    fn test_validation() {
        let registry = CredentialTypeRegistry::new();
        let schema = registry.get("onepassword").unwrap();
        
        let mut fields = HashMap::new();
        fields.insert("service_account_token".to_string(), "test_token".to_string());
        
        assert!(schema.validate(&fields).is_ok());
        
        let empty_fields = HashMap::new();
        assert!(schema.validate(&empty_fields).is_err());
    }
    
    #[test]
    fn test_custom_type_registration() {
        let mut registry = CredentialTypeRegistry::new();
        
        let custom_schema = CredentialTypeSchema {
            provider_type: "custom_api".to_string(),
            display_name: "Custom API".to_string(),
            description: "Custom API credentials".to_string(),
            fields: vec![CredentialField {
                name: "api_key".to_string(),
                display_name: "API Key".to_string(),
                field_type: FieldType::Secret,
                required: true,
                description: None,
            }],
            supports_multiple: false,
            built_in: false,
        };
        
        assert!(registry.register_custom(custom_schema).is_ok());
        assert!(registry.get("custom_api").is_some());
    }
    
    #[test]
    fn test_cannot_remove_built_in() {
        let mut registry = CredentialTypeRegistry::new();
        assert!(registry.remove_custom("onepassword").is_err());
    }
}
