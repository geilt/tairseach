# Utility Pattern

> **How to add shared utilities (Rust + TypeScript)**

---

## Rust Utilities

### Common Module (`src-tauri/src/common/`)

Shared utilities for all Rust code.

**Structure:**
```
src-tauri/src/common/
├── mod.rs              # Module exports
├── error.rs            # Error types
├── http.rs             # HTTP client factory
├── interpolation.rs    # String interpolation
├── paths.rs            # Path resolution
└── result.rs           # Result type aliases
```

---

### Adding a New Utility

**1. Create the file:**
```bash
touch src-tauri/src/common/your_utility.rs
```

**2. Write the utility:**
```rust
//! Your utility description

/// Your function documentation
pub fn your_function(input: &str) -> Result<String, String> {
    // Implementation
    Ok(input.to_uppercase())
}

/// Another utility function
pub fn another_function(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_your_function() {
        assert_eq!(your_function("hello").unwrap(), "HELLO");
    }
}
```

**3. Export in `mod.rs`:**
```rust
pub mod your_utility;

pub use your_utility::{your_function, another_function};
```

**4. Use anywhere:**
```rust
use crate::common::{your_function, another_function};

let result = your_function("test")?;
let sum = another_function(1, 2);
```

---

### Examples from Existing Utilities

#### HTTP Client (`http.rs`)
```rust
use reqwest::Client;

/// Create HTTP client with default timeout
pub fn create_http_client() -> Result<Client, String> {
    create_http_client_with_timeout(30)
}

/// Create HTTP client with custom timeout
pub fn create_http_client_with_timeout(timeout_secs: u64) -> Result<Client, String> {
    Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
}
```

**Usage:**
```rust
use crate::common::create_http_client;

let client = create_http_client()?;
let response = client.get("https://api.example.com").send().await?;
```

#### Path Resolution (`paths.rs`)
```rust
use std::path::PathBuf;

/// Get Tairseach home directory (~/.tairseach)
pub fn tairseach_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Could not determine home directory")
        .join(".tairseach")
}

/// Get manifests directory (~/.tairseach/manifests)
pub fn manifests_dir() -> PathBuf {
    tairseach_dir().join("manifests")
}

/// Get logs directory (~/.tairseach/logs)
pub fn logs_dir() -> PathBuf {
    tairseach_dir().join("logs")
}
```

**Usage:**
```rust
use crate::common::manifests_dir;

let manifest_path = manifests_dir().join("my-service.json");
```

#### String Interpolation (`interpolation.rs`)
```rust
use serde_json::Value;
use std::collections::HashMap;

/// Interpolate {{variable}} placeholders in strings
pub fn interpolate_params(
    template: &str,
    params: &HashMap<String, Value>
) -> Result<String, String> {
    let mut result = template.to_string();
    
    for (key, value) in params {
        let placeholder = format!("{{{{{}}}}}", key);
        let replacement = match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            _ => value.to_string(),
        };
        result = result.replace(&placeholder, &replacement);
    }
    
    Ok(result)
}
```

**Usage:**
```rust
use crate::common::interpolate_params;
use std::collections::HashMap;
use serde_json::json;

let mut params = HashMap::new();
params.insert("name".to_string(), json!("Alice"));
params.insert("age".to_string(), json!(30));

let template = "Hello {{name}}, you are {{age}} years old";
let result = interpolate_params(template, &params)?;
// "Hello Alice, you are 30 years old"
```

---

## Handler Common Utilities (`src-tauri/src/proxy/handlers/common.rs`)

Shared utilities specifically for handler implementations.

### Parameter Extraction

```rust
use serde_json::Value;

/// Extract optional string parameter
pub fn optional_string<'a>(params: &'a Value, key: &str) -> Option<&'a str> {
    params.get(key)?.as_str()
}

/// Try multiple parameter names (camelCase/snake_case)
pub fn optional_string_or<'a>(
    params: &'a Value,
    key1: &str,
    key2: &str
) -> Option<&'a str> {
    optional_string(params, key1)
        .or_else(|| optional_string(params, key2))
}

/// Extract required string parameter
pub fn required_string<'a>(params: &'a Value, key: &str) -> Result<&'a str, String> {
    optional_string(params, key)
        .ok_or_else(|| format!("Missing required parameter: {}", key))
}
```

**Usage:**
```rust
use super::common::*;

let account = optional_string(params, "account").unwrap_or("default");
let start_date = optional_string_or(params, "start_date", "startDate");
let user_id = required_string(params, "userId")?;
```

### Auth Broker Access

```rust
use std::sync::Arc;
use crate::proxy::auth_broker::AuthBroker;
use crate::proxy::protocol::JsonRpcResponse;

pub async fn get_auth_broker() -> Result<Arc<AuthBroker>, JsonRpcResponse> {
    // Implementation retrieves global auth broker instance
}

pub fn extract_access_token(
    token_data: &Value,
    id: &Value
) -> Result<String, JsonRpcResponse> {
    // Implementation extracts access_token from credential data
}
```

**Usage:**
```rust
let auth_broker = match get_auth_broker().await {
    Ok(broker) => broker,
    Err(mut resp) => {
        resp.id = id;
        return resp;
    }
};

let token_data = auth_broker.get_token("service", "account", None).await?;
let access_token = extract_access_token(&token_data, &id)?;
```

### Response Helpers

```rust
use serde_json::Value;
use crate::proxy::protocol::JsonRpcResponse;

pub fn ok(id: Value, result: Value) -> JsonRpcResponse {
    JsonRpcResponse::success(id, result)
}

pub fn error(id: Value, code: i32, message: String) -> JsonRpcResponse {
    JsonRpcResponse::error(id, code, message)
}

pub fn generic_error(id: Value, message: impl Into<String>) -> JsonRpcResponse {
    JsonRpcResponse::error(id, -32603, message.into())
}

pub fn method_not_found(id: Value, method: &str) -> JsonRpcResponse {
    JsonRpcResponse::error(id, -32601, format!("Method not found: {}", method))
}
```

---

## TypeScript Utilities

### Composables (`src/composables/`)

Reusable Vue composition functions.

**Structure:**
```
src/composables/
├── useActivityFeed.ts
├── useStateCache.ts
├── useStatusPoller.ts
├── useToast.ts
└── useWorkerPoller.ts
```

---

### Creating a New Composable

**1. Create the file:**
```bash
touch src/composables/useYourFeature.ts
```

**2. Write the composable:**
```typescript
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface YourDataType {
  id: string
  name: string
}

export function useYourFeature() {
  const data = ref<YourDataType[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetch() {
    loading.value = true
    error.value = null
    try {
      const result = await invoke<YourDataType[]>('your_command')
      data.value = result
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loading.value = false
    }
  }

  onMounted(() => {
    void fetch()
  })

  return {
    data,
    loading,
    error,
    fetch,
  }
}
```

**3. Use in components:**
```vue
<script setup lang="ts">
import { useYourFeature } from '@/composables/useYourFeature'

const { data, loading, error, fetch } = useYourFeature()
</script>

<template>
  <div v-if="loading">Loading...</div>
  <div v-else-if="error">{{ error }}</div>
  <div v-else>{{ data }}</div>
</template>
```

---

### Examples from Existing Composables

#### Polling Pattern (`useStatusPoller.ts`)
```typescript
import { onBeforeUnmount, onMounted } from 'vue'

export function usePoller(callback: () => void | Promise<void>, intervalMs: number) {
  let timer: number | null = null

  function start() {
    stop()
    timer = window.setInterval(() => {
      void callback()
    }, intervalMs)
  }

  function stop() {
    if (timer !== null) {
      clearInterval(timer)
      timer = null
    }
  }

  onMounted(() => start())
  onBeforeUnmount(() => stop())

  return { start, stop }
}
```

#### State Cache (`useStateCache.ts`)
```typescript
import { ref, Ref } from 'vue'

export function useStateCache<T>(
  key: string,
  fetcher: () => Promise<T>,
  ttlMs = 60000
): {
  data: Ref<T | null>
  loading: Ref<boolean>
  error: Ref<string | null>
  refresh: () => Promise<void>
} {
  const data = ref<T | null>(null) as Ref<T | null>
  const loading = ref(false)
  const error = ref<string | null>(null)
  let lastFetch = 0

  async function refresh(force = false) {
    const now = Date.now()
    if (!force && data.value && now - lastFetch < ttlMs) {
      return
    }

    loading.value = true
    error.value = null
    try {
      data.value = await fetcher()
      lastFetch = now
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loading.value = false
    }
  }

  return { data, loading, error, refresh }
}
```

---

### Shared TypeScript Functions (`src/utils/`)

For pure utility functions not tied to Vue.

**Example:**
```typescript
// src/utils/formatting.ts

export function formatTimestamp(ts: string): string {
  const date = new Date(ts)
  if (Number.isNaN(date.getTime())) return ts
  return date.toLocaleTimeString()
}

export function truncate(str: string, maxLen: number): string {
  return str.length > maxLen ? str.slice(0, maxLen) + '...' : str
}
```

**Usage:**
```typescript
import { formatTimestamp, truncate } from '@/utils/formatting'

const time = formatTimestamp('2025-02-13T01:50:00Z')
const short = truncate('Very long text here', 20)
```

---

## Checklist

### Rust Utilities
- [ ] Create file in `src-tauri/src/common/` or handler-specific location
- [ ] Document with `///` doc comments
- [ ] Write unit tests with `#[test]`
- [ ] Export in `mod.rs`
- [ ] Use consistent error handling (`Result<T, String>`)
- [ ] Run `cargo test` to verify
- [ ] Run `cargo clippy` for linting

### TypeScript Utilities
- [ ] Create file in `src/composables/` or `src/utils/`
- [ ] Export TypeScript interfaces for data types
- [ ] Use `ref`, `computed`, `watch` for reactive state
- [ ] Handle loading/error states
- [ ] Use lifecycle hooks (`onMounted`, `onUnmounted`) for cleanup
- [ ] Test in a component

---

## See Also

- [modules/handlers.md](../modules/handlers.md) — Handler architecture
- [modules/frontend-infra.md](../modules/frontend-infra.md) — Frontend patterns
- [handler-pattern.md](handler-pattern.md) — Using utilities in handlers
- [view-pattern.md](view-pattern.md) — Using composables in views
