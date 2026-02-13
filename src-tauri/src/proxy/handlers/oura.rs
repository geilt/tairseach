//! Oura Ring Handler
//!
//! Socket handlers for Oura Ring API v2 methods.
//! Retrieves personal access token from auth broker.

use serde_json::Value;
use tracing::{debug, error, info};

use super::common::*;
use super::super::protocol::JsonRpcResponse;

/// Oura Ring API client
struct OuraApi {
    token: String,
    client: reqwest::Client,
}

impl OuraApi {
    fn new(token: String) -> Result<Self, String> {
        use crate::common::create_http_client;
        let client = create_http_client()?;
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
    let auth_broker = match get_auth_broker().await {
        Ok(broker) => broker,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };

    let account = optional_string(params, "account").unwrap_or("default");

    let token_data = match auth_broker.get_token("oura", account, None).await {
        Ok(data) => data,
        Err((code, msg)) => {
            error!("Failed to get Oura token: {}", msg);
            return error(id, code, msg);
        }
    };

    let access_token = match extract_access_token(&token_data, &id) {
        Ok(token) => token,
        Err(response) => return response,
    };

    let api = match OuraApi::new(access_token) {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create Oura API client: {}", e);
            return generic_error(id, e);
        }
    };

    match action {
        "sleep" => handle_sleep(params, id, api).await,
        "activity" => handle_activity(params, id, api).await,
        "readiness" => handle_readiness(params, id, api).await,
        "heartRate" | "heart_rate" => handle_heart_rate(params, id, api).await,
        _ => method_not_found(id, &format!("oura.{}", action)),
    }
}

async fn handle_sleep(params: &Value, id: Value, api: OuraApi) -> JsonRpcResponse {
    info!("Handling oura.sleep");

    let start_date = optional_string_or(params, "start_date", "startDate");
    let end_date = optional_string_or(params, "end_date", "endDate");

    match api.sleep(start_date, end_date).await {
        Ok(data) => {
            debug!("Retrieved sleep data");
            ok(id, data)
        }
        Err(e) => {
            error!("Failed to get sleep data: {}", e);
            generic_error(id, e)
        }
    }
}

async fn handle_activity(params: &Value, id: Value, api: OuraApi) -> JsonRpcResponse {
    info!("Handling oura.activity");

    let start_date = optional_string_or(params, "start_date", "startDate");
    let end_date = optional_string_or(params, "end_date", "endDate");

    match api.activity(start_date, end_date).await {
        Ok(data) => {
            debug!("Retrieved activity data");
            ok(id, data)
        }
        Err(e) => {
            error!("Failed to get activity data: {}", e);
            generic_error(id, e)
        }
    }
}

async fn handle_readiness(params: &Value, id: Value, api: OuraApi) -> JsonRpcResponse {
    info!("Handling oura.readiness");

    let start_date = optional_string_or(params, "start_date", "startDate");
    let end_date = optional_string_or(params, "end_date", "endDate");

    match api.readiness(start_date, end_date).await {
        Ok(data) => {
            debug!("Retrieved readiness data");
            ok(id, data)
        }
        Err(e) => {
            error!("Failed to get readiness data: {}", e);
            generic_error(id, e)
        }
    }
}

async fn handle_heart_rate(params: &Value, id: Value, api: OuraApi) -> JsonRpcResponse {
    info!("Handling oura.heartRate");

    let start_datetime = optional_string_or(params, "start_datetime", "startDatetime");
    let end_datetime = optional_string_or(params, "end_datetime", "endDatetime");

    match api.heart_rate(start_datetime, end_datetime).await {
        Ok(data) => {
            debug!("Retrieved heart rate data");
            ok(id, data)
        }
        Err(e) => {
            error!("Failed to get heart rate data: {}", e);
            generic_error(id, e)
        }
    }
}
