//! Oura Ring Handler
//!
//! Socket handlers for Oura Ring API v2 methods.
//! Retrieves personal access token from auth broker.

use serde_json::Value;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::{debug, error, info};

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

/// Oura Ring API client
struct OuraApi {
    token: String,
    client: reqwest::Client,
}

impl OuraApi {
    fn new(token: String) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        Ok(Self { token, client })
    }

    async fn get(&self, path: &str, query_params: Vec<(&str, &str)>) -> Result<Value, String> {
        let url = format!("https://api.ouraring.com/v2/usercollection{}", path);
        
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
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
            error!("Oura API error {}: {}", status, body);
            return Err(format!("HTTP {} error: {}", status, body));
        }

        response
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON response: {}", e))
    }

    /// Get sleep data
    async fn sleep(&self, start_date: Option<&str>, end_date: Option<&str>) -> Result<Value, String> {
        let mut params = Vec::new();
        if let Some(start) = start_date {
            params.push(("start_date", start));
        }
        if let Some(end) = end_date {
            params.push(("end_date", end));
        }
        self.get("/sleep", params).await
    }

    /// Get activity data
    async fn activity(&self, start_date: Option<&str>, end_date: Option<&str>) -> Result<Value, String> {
        let mut params = Vec::new();
        if let Some(start) = start_date {
            params.push(("start_date", start));
        }
        if let Some(end) = end_date {
            params.push(("end_date", end));
        }
        self.get("/daily_activity", params).await
    }

    /// Get readiness data
    async fn readiness(&self, start_date: Option<&str>, end_date: Option<&str>) -> Result<Value, String> {
        let mut params = Vec::new();
        if let Some(start) = start_date {
            params.push(("start_date", start));
        }
        if let Some(end) = end_date {
            params.push(("end_date", end));
        }
        self.get("/daily_readiness", params).await
    }

    /// Get heart rate data
    async fn heart_rate(&self, start_datetime: Option<&str>, end_datetime: Option<&str>) -> Result<Value, String> {
        let mut params = Vec::new();
        if let Some(start) = start_datetime {
            params.push(("start_datetime", start));
        }
        if let Some(end) = end_datetime {
            params.push(("end_datetime", end));
        }
        self.get("/heartrate", params).await
    }
}

/// Handle Oura-related methods
pub async fn handle(action: &str, params: &Value, id: Value) -> JsonRpcResponse {
    let auth_broker = match get_broker().await {
        Ok(broker) => broker,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };

    // Retrieve Oura token from auth broker
    let account = params
        .get("account")
        .and_then(|v| v.as_str())
        .unwrap_or("default");

    let token_data = match auth_broker.get_token("oura", account, None).await {
        Ok(data) => data,
        Err((code, msg)) => {
            error!("Failed to get Oura token: {}", msg);
            return JsonRpcResponse::error(id, code, msg, None);
        }
    };

    let access_token = match token_data.get("access_token").and_then(|v| v.as_str()) {
        Some(token) => token.to_string(),
        None => {
            return JsonRpcResponse::error(
                id,
                -32000,
                "Invalid token response: missing access_token".to_string(),
                None,
            );
        }
    };

    // Create API client
    let api = match OuraApi::new(access_token) {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create Oura API client: {}", e);
            return JsonRpcResponse::error(id, -32000, e, None);
        }
    };

    // Dispatch to specific handler
    match action {
        "sleep" => handle_sleep(params, id, api).await,
        "activity" => handle_activity(params, id, api).await,
        "readiness" => handle_readiness(params, id, api).await,
        "heartRate" | "heart_rate" => handle_heart_rate(params, id, api).await,
        _ => JsonRpcResponse::method_not_found(id, &format!("oura.{}", action)),
    }
}

async fn handle_sleep(params: &Value, id: Value, api: OuraApi) -> JsonRpcResponse {
    info!("Handling oura.sleep");

    let start_date = params.get("start_date").or_else(|| params.get("startDate")).and_then(|v| v.as_str());
    let end_date = params.get("end_date").or_else(|| params.get("endDate")).and_then(|v| v.as_str());

    match api.sleep(start_date, end_date).await {
        Ok(data) => {
            debug!("Retrieved sleep data");
            JsonRpcResponse::success(id, data)
        }
        Err(e) => {
            error!("Failed to get sleep data: {}", e);
            JsonRpcResponse::error(id, -32000, e, None)
        }
    }
}

async fn handle_activity(params: &Value, id: Value, api: OuraApi) -> JsonRpcResponse {
    info!("Handling oura.activity");

    let start_date = params.get("start_date").or_else(|| params.get("startDate")).and_then(|v| v.as_str());
    let end_date = params.get("end_date").or_else(|| params.get("endDate")).and_then(|v| v.as_str());

    match api.activity(start_date, end_date).await {
        Ok(data) => {
            debug!("Retrieved activity data");
            JsonRpcResponse::success(id, data)
        }
        Err(e) => {
            error!("Failed to get activity data: {}", e);
            JsonRpcResponse::error(id, -32000, e, None)
        }
    }
}

async fn handle_readiness(params: &Value, id: Value, api: OuraApi) -> JsonRpcResponse {
    info!("Handling oura.readiness");

    let start_date = params.get("start_date").or_else(|| params.get("startDate")).and_then(|v| v.as_str());
    let end_date = params.get("end_date").or_else(|| params.get("endDate")).and_then(|v| v.as_str());

    match api.readiness(start_date, end_date).await {
        Ok(data) => {
            debug!("Retrieved readiness data");
            JsonRpcResponse::success(id, data)
        }
        Err(e) => {
            error!("Failed to get readiness data: {}", e);
            JsonRpcResponse::error(id, -32000, e, None)
        }
    }
}

async fn handle_heart_rate(params: &Value, id: Value, api: OuraApi) -> JsonRpcResponse {
    info!("Handling oura.heartRate");

    let start_datetime = params
        .get("start_datetime")
        .or_else(|| params.get("startDatetime"))
        .and_then(|v| v.as_str());
    let end_datetime = params
        .get("end_datetime")
        .or_else(|| params.get("endDatetime"))
        .and_then(|v| v.as_str());

    match api.heart_rate(start_datetime, end_datetime).await {
        Ok(data) => {
            debug!("Retrieved heart rate data");
            JsonRpcResponse::success(id, data)
        }
        Err(e) => {
            error!("Failed to get heart rate data: {}", e);
            JsonRpcResponse::error(id, -32000, e, None)
        }
    }
}
