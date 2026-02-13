//! String Interpolation Utilities
//!
//! Shared logic for parameter and credential interpolation.

use serde_json::Value;
use std::collections::HashMap;

/// Interpolate parameters in a template string
///
/// Replaces placeholders like `{field_name}` with values from params object.
///
/// # Example
/// ```ignore
/// let template = "/users/{user_id}/posts/{post_id}";
/// let params = json!({"user_id": "123", "post_id": "456"});
/// let result = interpolate_params(template, &params);
/// // result == "/users/123/posts/456"
/// ```
pub fn interpolate_params(template: &str, params: &Value) -> String {
    let mut result = template.to_string();

    if let Some(obj) = params.as_object() {
        for (key, value) in obj {
            let placeholder = format!("{{{}}}", key);
            let replacement = match value {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => continue,
            };
            result = result.replace(&placeholder, &replacement);
        }
    }

    result
}

/// Interpolate credential placeholders in environment variable values
///
/// Replaces placeholders like `{credential:id:field}` with credential values.
///
/// # Example
/// ```ignore
/// let template = "Bearer {credential:google-oauth:access_token}";
/// let mut creds = HashMap::new();
/// creds.insert("google-oauth", json!({"access_token": "ya29.abc123"}));
/// let result = interpolate_credentials(template, &creds);
/// // result == "Bearer ya29.abc123"
/// ```
pub fn interpolate_credentials(template: &str, credentials: &HashMap<String, Value>) -> String {
    let mut result = template.to_string();

    while let Some(start) = result.find("{credential:") {
        if let Some(end_pos) = result[start..].find('}') {
            let end = start + end_pos;
            let placeholder = &result[start + 12..end];
            let parts: Vec<&str> = placeholder.split(':').collect();

            if parts.len() == 2 {
                let (cred_id, field) = (parts[0], parts[1]);
                if let Some(cred_value) = credentials.get(cred_id) {
                    if let Some(field_value) = cred_value.get(field).and_then(|v| v.as_str()) {
                        result.replace_range(start..=end, field_value);
                        continue;
                    }
                }
            }

            // If interpolation failed, remove placeholder
            result.replace_range(start..=end, "");
        } else {
            break;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_interpolate_params() {
        let template = "/users/{user_id}/posts/{post_id}";
        let params = json!({"user_id": "123", "post_id": "456"});
        let result = interpolate_params(template, &params);
        assert_eq!(result, "/users/123/posts/456");
    }

    #[test]
    fn test_interpolate_params_with_numbers() {
        let template = "page={page}&limit={limit}";
        let params = json!({"page": 1, "limit": 100});
        let result = interpolate_params(template, &params);
        assert_eq!(result, "page=1&limit=100");
    }

    #[test]
    fn test_interpolate_credentials() {
        let template = "Bearer {credential:google-oauth:access_token}";
        let mut creds = HashMap::new();
        creds.insert(
            "google-oauth".to_string(),
            json!({"access_token": "ya29.abc123"}),
        );
        let result = interpolate_credentials(template, &creds);
        assert_eq!(result, "Bearer ya29.abc123");
    }

    #[test]
    fn test_interpolate_credentials_missing() {
        let template = "Bearer {credential:missing:token}";
        let creds = HashMap::new();
        let result = interpolate_credentials(template, &creds);
        assert_eq!(result, "Bearer "); // Placeholder removed
    }
}
