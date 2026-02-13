# Handler Pattern

> **Copy-paste template for adding a new Rust handler**

---

## File Location

```
src-tauri/src/proxy/handlers/YOUR_SERVICE.rs
```

Add to `mod.rs`:
```rust
pub mod your_service;
```

---

## Template

```rust
//! Your Service Handler
//!
//! Socket handlers for Your Service API methods.
//! Retrieves credentials from auth broker.

use serde_json::Value;
use tracing::{debug, error, info};

use super::common::*;
use super::super::protocol::JsonRpcResponse;

/// Your Service API client
struct YourServiceApi {
    token: String,
    client: reqwest::Client,
}

impl YourServiceApi {
    fn new(token: String) -> Result<Self, String> {
        use crate::common::create_http_client;
        let client = create_http_client()?;
        Ok(Self { token, client })
    }

    async fn get(&self, path: &str, query_params: Vec<(&str, &str)>) -> Result<Value, String> {
        let url = format!("https://api.yourservice.com/v1{}", path);
        
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
            error!("YourService API error {}: {}", status, body);
            return Err(format!("HTTP {} error: {}", status, body));
        }

        response
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON response: {}", e))
    }

    async fn your_method(&self, param: Option<&str>) -> Result<Value, String> {
        let mut params = Vec::new();
        if let Some(p) = param {
            params.push(("param_name", p));
        }
        self.get("/your-endpoint", params).await
    }
}

/// Handle YourService-related methods
pub async fn handle(action: &str, params: &Value, id: Value) -> JsonRpcResponse {
    let auth_broker = match get_auth_broker().await {
        Ok(broker) => broker,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };

    let account = optional_string(params, "account").unwrap_or("default");

    let token_data = match auth_broker.get_token("yourservice", account, None).await {
        Ok(data) => data,
        Err((code, msg)) => {
            error!("Failed to get YourService token: {}", msg);
            return error(id, code, msg);
        }
    };

    let access_token = match extract_access_token(&token_data, &id) {
        Ok(token) => token,
        Err(response) => return response,
    };

    let api = match YourServiceApi::new(access_token) {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create YourService API client: {}", e);
            return generic_error(id, e);
        }
    };

    match action {
        "yourMethod" | "your_method" => handle_your_method(params, id, api).await,
        _ => method_not_found(id, &format!("yourservice.{}", action)),
    }
}

async fn handle_your_method(params: &Value, id: Value, api: YourServiceApi) -> JsonRpcResponse {
    info!("Handling yourservice.yourMethod");

    let param = optional_string(params, "param");

    match api.your_method(param).await {
        Ok(data) => {
            debug!("Retrieved data");
            ok(id, data)
        }
        Err(e) => {
            error!("Failed to get data: {}", e);
            generic_error(id, e)
        }
    }
}
```

---

## Checklist

- [ ] Create `src-tauri/src/proxy/handlers/your_service.rs`
- [ ] Add `pub mod your_service;` to `handlers/mod.rs`
- [ ] Define API client struct with methods
- [ ] Implement `handle()` function with auth broker integration
- [ ] Implement method-specific handlers (e.g., `handle_your_method`)
- [ ] Use `common::*` helpers: `optional_string`, `optional_string_or`, `get_auth_broker`, `extract_access_token`
- [ ] Use standard responses: `ok()`, `error()`, `generic_error()`, `method_not_found()`
- [ ] Add tracing: `info!`, `debug!`, `error!`
- [ ] Update router to dispatch to your handler
- [ ] Create corresponding manifest in `~/.tairseach/manifests/`
- [ ] Test via MCP bridge

---

## Common Utilities (from `handlers/common.rs`)

```rust
// Parameter extraction
optional_string(params, "key")              // Option<&str>
optional_string_or(params, "key1", "key2")  // Try key1, fallback to key2
required_string(params, "key")              // Result<&str, String>

// Auth
get_auth_broker().await                     // Result<Arc<AuthBroker>, JsonRpcResponse>
extract_access_token(token_data, id)        // Result<String, JsonRpcResponse>

// Responses
ok(id, result_value)                        // Success response
error(id, error_code, message)              // Error with code
generic_error(id, message)                  // Generic error
method_not_found(id, method_name)           // Unknown method error
```

---

## See Also

- [modules/handlers.md](../modules/handlers.md) — Handler architecture
- [modules/router.md](../modules/router.md) — Routing logic
- [manifest-pattern.md](manifest-pattern.md) — Create corresponding manifest
