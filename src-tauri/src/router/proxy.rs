//! Proxy Implementation Dispatcher
//!
//! Makes HTTP calls to external APIs with credential injection via auth headers.

use std::collections::HashMap;
use serde_json::Value;
use tracing::{error, info};

use crate::manifest::types::{Manifest, ProxyAuth, ProxyToolBinding, Tool};
use crate::proxy::protocol::JsonRpcResponse;

/// Dispatch to HTTP API with auth header injection
pub async fn dispatch(
    _manifest: &Manifest,
    tool: &Tool,
    params: &Value,
    id: Value,
    base_url: &str,
    auth: &ProxyAuth,
    tool_bindings: &HashMap<String, ProxyToolBinding>,
    credentials: &HashMap<String, Value>,
) -> JsonRpcResponse {
    info!("Proxying HTTP request for tool: {}", tool.name);

    // Get the tool binding
    let binding = match tool_bindings.get(&tool.name) {
        Some(b) => b,
        None => {
            return JsonRpcResponse::error(
                id,
                -32601,
                format!("No proxy binding for tool: {}", tool.name),
                None,
            );
        }
    };

    // Build auth header
    let auth_header = match build_auth_header(auth, credentials) {
        Ok(h) => h,
        Err(e) => {
            return JsonRpcResponse::error(id, -32011, e, None);
        }
    };

    // Build URL with path interpolation
    let path = interpolate_params(&binding.path, params);
    let mut url = format!("{}{}", base_url, path);

    // Add query parameters
    if !binding.query.is_empty() {
        let query_parts: Vec<String> = binding
            .query
            .iter()
            .filter_map(|(key, value_template)| {
                let value = interpolate_params(value_template, params);
                if !value.is_empty() {
                    Some(format!(
                        "{}={}",
                        urlencoding::encode(key),
                        urlencoding::encode(&value)
                    ))
                } else {
                    None
                }
            })
            .collect();

        if !query_parts.is_empty() {
            url.push('?');
            url.push_str(&query_parts.join("&"));
        }
    }

    // Use reqwest for HTTP calls
    let client = reqwest::Client::new();

    let mut request_builder = match binding.method.as_str() {
        "GET" => client.get(&url),
        "POST" => {
            let body = binding.body_template.as_ref().unwrap_or(params);
            client.post(&url).json(body)
        }
        "PUT" => {
            let body = binding.body_template.as_ref().unwrap_or(params);
            client.put(&url).json(body)
        }
        "PATCH" => {
            let body = binding.body_template.as_ref().unwrap_or(params);
            client.patch(&url).json(body)
        }
        "DELETE" => client.delete(&url),
        _ => {
            return JsonRpcResponse::error(
                id,
                -32000,
                format!("Unsupported HTTP method: {}", binding.method),
                None,
            );
        }
    };

    // Add auth header
    request_builder = match auth.strategy.as_str() {
        "oauth2Bearer" => request_builder.bearer_auth(&auth_header),
        "apiKeyHeader" => {
            let default_header = "X-API-Key".to_string();
            let header_name = auth.header_name.as_ref().unwrap_or(&default_header);
            request_builder.header(header_name, &auth_header)
        }
        "apiKeyQuery" => {
            // Auth already in query params via credential interpolation
            request_builder
        }
        "basic" => {
            // auth_header is already "Basic base64(...)"
            request_builder.header("Authorization", &auth_header)
        }
        _ => {
            return JsonRpcResponse::error(
                id,
                -32000,
                format!("Unsupported auth strategy: {}", auth.strategy),
                None,
            );
        }
    };

    // Add custom headers from binding
    for (key, value) in &binding.headers {
        request_builder = request_builder.header(key, value);
    }

    // Execute request
    let response = match request_builder.send().await {
        Ok(r) => r,
        Err(e) => {
            error!("HTTP request failed: {}", e);
            return JsonRpcResponse::error(
                id,
                -32000,
                format!("HTTP request failed: {}", e),
                None,
            );
        }
    };

    let status = response.status();
    if !status.is_success() {
        error!("HTTP request returned error status: {}", status);
        let error_body = response.text().await.unwrap_or_default();
        return JsonRpcResponse::error(
            id,
            -32000,
            format!("HTTP {} error", status),
            Some(serde_json::json!({ "status": status.as_u16(), "body": error_body })),
        );
    }

    // Parse response body
    let body = match response.json::<Value>().await {
        Ok(json) => json,
        Err(e) => {
            error!("Failed to parse response JSON: {}", e);
            return JsonRpcResponse::error(
                id,
                -32000,
                format!("Failed to parse response: {}", e),
                None,
            );
        }
    };

    // Extract response path if specified
    let result = if let Some(ref response_path) = binding.response_path {
        extract_json_path(&body, response_path).unwrap_or(body)
    } else {
        body
    };

    JsonRpcResponse::success(id, result)
}

fn build_auth_header(auth: &ProxyAuth, credentials: &HashMap<String, Value>) -> Result<String, String> {
    let cred = credentials
        .get(&auth.credential_id)
        .ok_or_else(|| format!("Credential not found: {}", auth.credential_id))?;

    match auth.strategy.as_str() {
        "oauth2Bearer" => {
            let token_field = auth.token_field.as_deref().unwrap_or("access_token");
            let token = cred
                .get(token_field)
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("Missing {} in credential", token_field))?;
            Ok(token.to_string())
        }
        "apiKeyHeader" => {
            let key = cred
                .get("api_key")
                .and_then(|v| v.as_str())
                .ok_or("Missing api_key in credential")?;
            Ok(key.to_string())
        }
        "basic" => {
            let username = cred.get("username").and_then(|v| v.as_str()).unwrap_or("");
            let password = cred.get("password").and_then(|v| v.as_str()).unwrap_or("");
            use base64::Engine;
            let encoded = base64::engine::general_purpose::STANDARD
                .encode(format!("{}:{}", username, password));
            Ok(format!("Basic {}", encoded))
        }
        _ => Err(format!("Unsupported auth strategy: {}", auth.strategy)),
    }
}

fn interpolate_params(template: &str, params: &Value) -> String {
    let mut result = template.to_string();

    // Simple interpolation: {field_name} → value
    if let Some(obj) = params.as_object() {
        for (key, value) in obj {
            let placeholder = format!("{{{}}}", key);
            if let Some(val_str) = value.as_str() {
                result = result.replace(&placeholder, val_str);
            } else if let Some(val_num) = value.as_i64() {
                result = result.replace(&placeholder, &val_num.to_string());
            } else if let Some(val_bool) = value.as_bool() {
                result = result.replace(&placeholder, &val_bool.to_string());
            }
        }
    }

    result
}

fn extract_json_path(value: &Value, path: &str) -> Option<Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value;

    for part in parts {
        current = current.get(part)?;
    }

    Some(current.clone())
}
