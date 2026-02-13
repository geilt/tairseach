//! Jira Handler
//!
//! Socket handlers for Jira Cloud REST API v3 methods.
//! Retrieves API token (basic auth: email + token) from auth broker.

use serde_json::Value;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::{debug, error, info};

use super::common::*;
use super::super::protocol::JsonRpcResponse;
use crate::auth::AuthBroker;

/// Global auth broker instance.
static AUTH_BROKER: OnceCell<Arc<AuthBroker>> = OnceCell::const_new();

/// Get or initialise the auth broker.
async fn get_broker() -> Result<&'static Arc<AuthBroker>, JsonRpcResponse> {
    AUTH_BROKER
        .get_or_try_init(|| async {
            match AuthBroker::new().await {
                Ok(broker) => {
                    broker.spawn_refresh_daemon();
                    Ok(broker)
                }
                Err(e) => Err(e),
            }
        })
        .await
        .map_err(|e| {
            error!("Failed to initialise auth broker: {}", e);
            JsonRpcResponse::error(
                Value::Null,
                crate::auth::error_codes::MASTER_KEY_NOT_INITIALIZED,
                format!("Auth broker init failed: {}", e),
                None,
            )
        })
}

/// Jira API client
struct JiraApi {
    email: String,
    token: String,
    base_url: String,
    client: reqwest::Client,
}

impl JiraApi {
    fn new(email: String, token: String, base_url: String) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        Ok(Self {
            email,
            token,
            base_url,
            client,
        })
    }

    fn basic_auth(&self) -> String {
        let credentials = format!("{}:{}", self.email, self.token);
        format!("Basic {}", base64::Engine::encode(&base64::engine::general_purpose::STANDARD, credentials.as_bytes()))
    }

    async fn get(&self, path: &str, query_params: Vec<(&str, &str)>) -> Result<Value, String> {
        let url = format!("{}/rest/api/3{}", self.base_url, path);
        
        let response = self
            .client
            .get(&url)
            .header("Authorization", self.basic_auth())
            .header("Accept", "application/json")
            .query(&query_params)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());
            error!("Jira API error {}: {}", status, body);
            return Err(format!("HTTP {} error: {}", status, body));
        }

        response
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON response: {}", e))
    }

    async fn post(&self, path: &str, body: Value) -> Result<Value, String> {
        let url = format!("{}/rest/api/3{}", self.base_url, path);
        
        let response = self
            .client
            .post(&url)
            .header("Authorization", self.basic_auth())
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());
            error!("Jira API error {}: {}", status, body);
            return Err(format!("HTTP {} error: {}", status, body));
        }

        response
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON response: {}", e))
    }

    async fn put(&self, path: &str, body: Value) -> Result<Value, String> {
        let url = format!("{}/rest/api/3{}", self.base_url, path);
        
        let response = self
            .client
            .put(&url)
            .header("Authorization", self.basic_auth())
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());
            error!("Jira API error {}: {}", status, body);
            return Err(format!("HTTP {} error: {}", status, body));
        }

        // Some PUT requests return 204 No Content
        if response.status() == 204 {
            return Ok(serde_json::json!({"success": true}));
        }

        response
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON response: {}", e))
    }

    /// Search for issues using JQL
    async fn search_issues(&self, jql: &str, _fields: Option<Vec<String>>, max_results: Option<usize>) -> Result<Value, String> {
        let mut params = vec![("jql", jql)];
        let max_str = max_results.unwrap_or(50).to_string();
        params.push(("maxResults", &max_str));
        
        // Note: Custom fields parameter would require POST with JSON body
        // Using default fields for now
        
        self.get("/search", params).await
    }

    /// Get a specific issue
    async fn get_issue(&self, issue_key: &str) -> Result<Value, String> {
        self.get(&format!("/issue/{}", issue_key), vec![]).await
    }

    /// Create an issue
    async fn create_issue(&self, issue_data: Value) -> Result<Value, String> {
        self.post("/issue", issue_data).await
    }

    /// Update an issue
    async fn update_issue(&self, issue_key: &str, update_data: Value) -> Result<Value, String> {
        self.put(&format!("/issue/{}", issue_key), update_data).await
    }

    /// Transition an issue (change status)
    async fn transition_issue(&self, issue_key: &str, transition_id: &str, fields: Option<Value>) -> Result<Value, String> {
        let mut body = serde_json::json!({
            "transition": {
                "id": transition_id
            }
        });
        
        if let Some(f) = fields {
            body["fields"] = f;
        }
        
        self.post(&format!("/issue/{}/transitions", issue_key), body).await
    }

    /// List projects
    async fn list_projects(&self) -> Result<Value, String> {
        self.get("/project", vec![]).await
    }

    /// List sprints for a board
    async fn list_sprints(&self, board_id: &str) -> Result<Value, String> {
        // Note: Sprints are part of Jira Agile API, not core REST API
        // Using /rest/agile/1.0/ instead
        let url = format!("{}/rest/agile/1.0/board/{}/sprint", self.base_url, board_id);
        
        let response = self
            .client
            .get(&url)
            .header("Authorization", self.basic_auth())
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());
            error!("Jira API error {}: {}", status, body);
            return Err(format!("HTTP {} error: {}", status, body));
        }

        response
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON response: {}", e))
    }
}

/// Handle Jira-related methods
pub async fn handle(action: &str, params: &Value, id: Value) -> JsonRpcResponse {
    let auth_broker = match get_broker().await {
        Ok(broker) => broker,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };

    // Retrieve Jira credentials from auth broker
    let account = string_with_default(params, "account", "default");

    let token_data = match auth_broker.get_token("jira", account, None).await {
        Ok(data) => data,
        Err((code, msg)) => {
            error!("Failed to get Jira token: {}", msg);
            return error(id, code, msg);
        }
    };

    // For Jira, we store email in a custom field or in the account name
    // Let's expect it in params or derive from account
    let email = optional_string(params, "email").unwrap_or(account);

    let api_token = match extract_access_token(&token_data, &id) {
        Ok(token) => token,
        Err(response) => return response,
    };

    // Get base URL from params (required for Jira Cloud)
    let base_url = match require_string_or(params, "base_url", "baseUrl", &id) {
        Ok(url) => url.to_string(),
        Err(response) => return response,
    };

    // Create API client
    let api = match JiraApi::new(email.to_string(), api_token, base_url) {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create Jira API client: {}", e);
            return generic_error(id, e);
        }
    };

    // Dispatch to specific handler
    match action {
        "issues.search" | "issues_search" => handle_search_issues(params, id, api).await,
        "issues.get" | "issues_get" => handle_get_issue(params, id, api).await,
        "issues.create" | "issues_create" => handle_create_issue(params, id, api).await,
        "issues.update" | "issues_update" => handle_update_issue(params, id, api).await,
        "issues.transition" | "issues_transition" => handle_transition_issue(params, id, api).await,
        "projects.list" | "projects_list" => handle_list_projects(id, api).await,
        "sprints.list" | "sprints_list" => handle_list_sprints(params, id, api).await,
        _ => method_not_found(id, &format!("jira.{}", action)),
    }
}

async fn handle_search_issues(params: &Value, id: Value, api: JiraApi) -> JsonRpcResponse {
    info!("Handling jira.issues.search");

    let jql = match require_string(params, "jql", &id) {
        Ok(q) => q,
        Err(response) => return response,
    };

    let fields = optional_string_array(params, "fields");
    let max_results = optional_u64_or(params, "max_results", "maxResults").map(|n| n as usize);

    match api.search_issues(jql, fields, max_results).await {
        Ok(data) => {
            debug!("Retrieved issues");
            ok(id, data)
        }
        Err(e) => {
            error!("Failed to search issues: {}", e);
            generic_error(id, e)
        }
    }
}

async fn handle_get_issue(params: &Value, id: Value, api: JiraApi) -> JsonRpcResponse {
    info!("Handling jira.issues.get");

    let issue_key = match optional_string_or(params, "issue_key", "issueKey")
        .or_else(|| optional_string(params, "key"))
    {
        Some(k) => k,
        None => {
            return invalid_params(id, "Missing required parameter: issue_key");
        }
    };

    match api.get_issue(issue_key).await {
        Ok(issue) => ok(id, issue),
        Err(e) => {
            error!("Failed to get issue: {}", e);
            generic_error(id, e)
        }
    }
}

async fn handle_create_issue(params: &Value, id: Value, api: JiraApi) -> JsonRpcResponse {
    info!("Handling jira.issues.create");

    let issue_data = match params.get("issue") {
        Some(v) => v.clone(),
        None => {
            return invalid_params(id, "Missing required parameter: issue");
        }
    };

    match api.create_issue(issue_data).await {
        Ok(created) => {
            info!("Issue created successfully");
            ok(id, created)
        }
        Err(e) => {
            error!("Failed to create issue: {}", e);
            generic_error(id, e)
        }
    }
}

async fn handle_update_issue(params: &Value, id: Value, api: JiraApi) -> JsonRpcResponse {
    info!("Handling jira.issues.update");

    let issue_key = match optional_string_or(params, "issue_key", "issueKey")
        .or_else(|| optional_string(params, "key"))
    {
        Some(k) => k,
        None => {
            return invalid_params(id, "Missing required parameter: issue_key");
        }
    };

    let update_data = match params.get("update") {
        Some(v) => v.clone(),
        None => {
            return invalid_params(id, "Missing required parameter: update");
        }
    };

    match api.update_issue(issue_key, update_data).await {
        Ok(result) => {
            info!("Issue updated successfully");
            ok(id, result)
        }
        Err(e) => {
            error!("Failed to update issue: {}", e);
            generic_error(id, e)
        }
    }
}

async fn handle_transition_issue(params: &Value, id: Value, api: JiraApi) -> JsonRpcResponse {
    info!("Handling jira.issues.transition");

    let issue_key = match optional_string_or(params, "issue_key", "issueKey")
        .or_else(|| optional_string(params, "key"))
    {
        Some(k) => k,
        None => {
            return invalid_params(id, "Missing required parameter: issue_key");
        }
    };

    let transition_id = match require_string_or(params, "transition_id", "transitionId", &id) {
        Ok(t) => t,
        Err(response) => return response,
    };

    let fields = params.get("fields").cloned();

    match api.transition_issue(issue_key, transition_id, fields).await {
        Ok(result) => {
            info!("Issue transitioned successfully");
            ok(id, result)
        }
        Err(e) => {
            error!("Failed to transition issue: {}", e);
            generic_error(id, e)
        }
    }
}

async fn handle_list_projects(id: Value, api: JiraApi) -> JsonRpcResponse {
    info!("Handling jira.projects.list");

    match api.list_projects().await {
        Ok(projects) => {
            debug!("Retrieved projects");
            ok(id, projects)
        }
        Err(e) => {
            error!("Failed to list projects: {}", e);
            generic_error(id, e)
        }
    }
}

async fn handle_list_sprints(params: &Value, id: Value, api: JiraApi) -> JsonRpcResponse {
    info!("Handling jira.sprints.list");

    let board_id = match require_string_or(params, "board_id", "boardId", &id) {
        Ok(b) => b,
        Err(response) => return response,
    };

    match api.list_sprints(board_id).await {
        Ok(sprints) => {
            debug!("Retrieved sprints for board {}", board_id);
            ok(id, sprints)
        }
        Err(e) => {
            error!("Failed to list sprints: {}", e);
            generic_error(id, e)
        }
    }
}
