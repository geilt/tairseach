# Tairseach Optimization Reference

**Version:** 1.0  
**Date:** 2026-02-13  
**Purpose:** Comprehensive optimization patterns for Tairseach refactoring  
**Audience:** All agents working on the Tairseach codebase  

**Primary Goal:** Eliminate duplication (DRY). Secondary: improve performance and maintainability.

---

## Table of Contents

1. [Rust Micro-Optimizations](#rust-micro-optimizations)
2. [Rust Macro-Optimizations](#rust-macro-optimizations)
3. [Vue/TypeScript Micro-Optimizations](#vuetypescript-micro-optimizations)
4. [Vue/TypeScript Macro-Optimizations](#vuetypescript-macro-optimizations)
5. [Tauri-Specific Patterns](#tauri-specific-patterns)
6. [Quick Reference Matrix](#quick-reference-matrix)

---

## Rust Micro-Optimizations

### R-MICRO-001: Use `&str` Instead of `&String` in Function Parameters

**Problem:** `&String` forces callers to allocate when they have string literals or slices.

**Priority:** Must-do

**When to use:**
- All function parameters that don't need to own the string
- Any function that only reads string data

**When NOT to use:**
- When you need to take ownership (use `String`)
- When you need to mutate the string (use `&mut String`)

**Before:**
```rust
fn process_name(name: &String) -> String {
    format!("Hello, {}", name)
}

// Caller must allocate unnecessarily
let name = String::from("Alice");
process_name(&name);
```

**After:**
```rust
fn process_name(name: &str) -> String {
    format!("Hello, {}", name)
}

// Can use string literals directly
process_name("Alice");
```

**Source:** Rust Performance Book - Heap Allocations

---

### R-MICRO-002: Use `Cow<'static, str>` for Mixed Static/Dynamic Strings

**Problem:** Mixing static string literals and dynamic strings forces allocation for static content.

**Priority:** Nice-to-have (high impact where applicable)

**When to use:**
- API responses with mixed static/dynamic fields
- Error messages (static codes + dynamic details)
- Configuration that's mostly static with some dynamic values

**When NOT to use:**
- When all strings are dynamic (just use `String`)
- When all strings are static (just use `&'static str`)
- Simple cases where clarity matters more than allocation

**Before:**
```rust
struct ApiResponse {
    status: String,  // Always allocates even for "success" literal
    message: String,
}

fn success() -> ApiResponse {
    ApiResponse {
        status: "success".to_string(),  // Unnecessary allocation!
        message: "Operation completed".to_string(),
    }
}
```

**After:**
```rust
use std::borrow::Cow;

struct ApiResponse {
    status: Cow<'static, str>,
    message: Cow<'static, str>,
}

fn success() -> ApiResponse {
    ApiResponse {
        status: Cow::Borrowed("success"),  // Zero allocations
        message: Cow::Borrowed("Operation completed"),
    }
}

fn dynamic_error(details: String) -> ApiResponse {
    ApiResponse {
        status: Cow::Borrowed("error"),
        message: Cow::Owned(details),  // Only allocates for dynamic part
    }
}
```

**Source:** Leapcell Blog - Zero-Cost String Handling with Cow

---

### R-MICRO-003: Avoid Unnecessary `.to_string()` / `.clone()`

**Problem:** Each clone allocates. Unnecessary clones from careless refactoring waste memory and CPU.

**Priority:** Must-do

**When to use:**
- When you genuinely need an owned copy
- When lifetime constraints force it

**When NOT to use:**
- As a quick fix for borrow checker errors (refactor instead)
- For shared data that could use `Rc`/`Arc`
- When a reference would work

**Before:**
```rust
fn log_and_return(msg: &str) -> String {
    println!("{}", msg.clone());  // Unnecessary clone!
    msg.to_string()
}
```

**After:**
```rust
fn log_and_return(msg: &str) -> String {
    println!("{}", msg);  // No clone needed for reading
    msg.to_string()
}
```

**Source:** Llogiq - Rust Performance Pitfalls

---

### R-MICRO-004: Use `thiserror` for Custom Error Types

**Problem:** String errors lose type information. Manual `Display` implementations are boilerplate.

**Priority:** Must-do (enables better macro patterns)

**When to use:**
- All error types in the application
- Any Result<T, E> where E needs structure

**When NOT to use:**
- Never (always prefer typed errors)

**Before:**
```rust
#[tauri::command]
fn read_file(path: String) -> Result<Vec<u8>, String> {
    std::fs::read(&path).map_err(|e| e.to_string())?;
    // Loss of error type information
}
```

**After:**
```rust
use thiserror::Error;

#[derive(Debug, Error)]
enum AppError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[tauri::command]
fn read_file(path: String) -> Result<Vec<u8>, AppError> {
    let data = std::fs::read(&path)?;  // Automatically converts with #[from]
    Ok(data)
}
```

**Source:** Tauri Documentation - Error Handling

---

### R-MICRO-005: Serde Optimization Attributes

**Problem:** Default serde behavior serializes everything, even when unnecessary.

**Priority:** Nice-to-have

**When to use:**
- Large structs with many fields
- APIs with consistent naming conventions (camelCase/snake_case)
- Optional fields that are often None

**When NOT to use:**
- When you need full control over every field
- When struct is small (< 5 fields)

**Before:**
```rust
#[derive(Serialize, Deserialize)]
struct UserProfile {
    user_id: String,
    user_name: String,
    user_email: String,
    optional_field: Option<String>,  // Serializes as "optional_field": null
}
```

**After:**
```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]  // Auto-converts to camelCase
struct UserProfile {
    user_id: String,
    user_name: String,
    user_email: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]  // Skip if None
    optional_field: Option<String>,
    
    #[serde(default)]  // Provides default if missing during deserialization
    settings: UserSettings,
}

#[derive(Serialize, Deserialize, Default)]
struct UserSettings {
    theme: String,
}
```

**Source:** Rust Serde Documentation

---

### R-MICRO-006: Async Commands with Proper Blocking

**Problem:** CPU-bound work in async commands blocks the async runtime. Tauri IPC can freeze UI.

**Priority:** Must-do (for heavy operations)

**When to use:**
- File I/O operations
- CPU-intensive computations (hashing, parsing)
- Blocking database queries

**When NOT to use:**
- Already-async operations (network requests via `reqwest`)
- Trivial operations (< 1ms)

**Before:**
```rust
#[tauri::command]
async fn hash_large_file(path: String) -> Result<String, String> {
    // This blocks the async executor!
    let data = std::fs::read(&path).map_err(|e| e.to_string())?;
    let hash = expensive_hash(&data);
    Ok(hash)
}
```

**After:**
```rust
use tokio::task;

#[tauri::command]
async fn hash_large_file(path: String) -> Result<String, AppError> {
    // Move blocking work to thread pool
    let hash = task::spawn_blocking(move || {
        let data = std::fs::read(&path)?;
        let hash = expensive_hash(&data);
        Ok::<_, std::io::Error>(hash)
    })
    .await
    .map_err(|e| AppError::JoinError(e.to_string()))??;
    
    Ok(hash)
}
```

**Source:** Tokio Documentation, Tauri Best Practices

---

### R-MICRO-007: Reuse Collections (Avoid Repeated Allocations)

**Problem:** Creating new `Vec`/`HashMap` in loops causes many allocations.

**Priority:** Nice-to-have (high impact in hot loops)

**When to use:**
- Loops that process many items
- Functions called repeatedly with temporary collections

**When NOT to use:**
- When collection needs to outlive the scope
- When clarity suffers significantly

**Before:**
```rust
for item in items {
    let mut results = Vec::new();  // Allocates every iteration!
    process_item(item, &mut results);
    output.extend(results);
}
```

**After:**
```rust
let mut results = Vec::new();
for item in items {
    results.clear();  // Reuse allocation
    process_item(item, &mut results);
    output.extend(results.iter());
}
```

**Source:** Rust Performance Book - Reusing Collections

---

## Rust Macro-Optimizations

### R-MACRO-001: Handler Trait Abstraction

**Problem:** 16 handlers with 80% identical structure = massive duplication.

**Priority:** Must-do

**When to use:**
- Any time you have > 3 similar handler functions
- When handlers share error handling, auth, logging

**When NOT to use:**
- Handlers are genuinely different (< 50% overlap)

**Before:**
```rust
// handler_a.rs
pub async fn handle_a(
    req: Request,
    state: Arc<AppState>,
) -> Result<Response, Error> {
    // Auth check
    let user = authenticate(&req)?;
    // Validate
    validate_request(&req)?;
    // Process
    let result = do_work_a(&req, &state).await?;
    // Log
    log_request("handle_a", &user);
    Ok(result)
}

// handler_b.rs - 90% the same!
pub async fn handle_b(
    req: Request,
    state: Arc<AppState>,
) -> Result<Response, Error> {
    let user = authenticate(&req)?;
    validate_request(&req)?;
    let result = do_work_b(&req, &state).await?;
    log_request("handle_b", &user);
    Ok(result)
}
```

**After:**
```rust
// handlers/mod.rs
use async_trait::async_trait;

#[async_trait]
pub trait Handler: Send + Sync {
    type Request: DeserializeOwned;
    type Response: Serialize;
    
    fn name(&self) -> &'static str;
    
    async fn execute(
        &self,
        req: Self::Request,
        state: &AppState,
        user: &User,
    ) -> Result<Self::Response, AppError>;
    
    // Default implementations for common logic
    async fn handle(
        &self,
        req: Request,
        state: Arc<AppState>,
    ) -> Result<Response, AppError> {
        let user = authenticate(&req)?;
        validate_request(&req)?;
        
        let parsed: Self::Request = serde_json::from_value(req.body)?;
        let result = self.execute(parsed, &state, &user).await?;
        
        log_request(self.name(), &user);
        Ok(Response::json(result))
    }
}

// handlers/handler_a.rs
pub struct HandlerA;

#[async_trait]
impl Handler for HandlerA {
    type Request = RequestA;
    type Response = ResponseA;
    
    fn name(&self) -> &'static str { "handler_a" }
    
    async fn execute(
        &self,
        req: RequestA,
        state: &AppState,
        user: &User,
    ) -> Result<ResponseA, AppError> {
        // Only business logic here - no boilerplate!
        do_work_a(req, state).await
    }
}
```

**Impact:** 16 handlers × ~50 lines boilerplate = 800 lines eliminated.

---

### R-MACRO-002: Shared Error Type Hierarchy

**Problem:** Each module defining its own error types causes conversion boilerplate.

**Priority:** Must-do

**When to use:**
- Application-wide error handling
- When errors cross module boundaries

**When NOT to use:**
- Never (unified errors are always better)

**Before:**
```rust
// auth/mod.rs
#[derive(Debug)]
pub enum AuthError {
    InvalidToken,
    Expired,
}

// handlers/mod.rs
#[derive(Debug)]
pub enum HandlerError {
    Auth(AuthError),  // Manual conversion needed
    Database(DbError),
}

impl From<AuthError> for HandlerError {
    fn from(e: AuthError) -> Self {
        HandlerError::Auth(e)
    }
}
// ... repeat for every error type
```

**After:**
```rust
// error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Authentication failed: {0}")]
    Auth(#[from] AuthError),
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Invalid request: {0}")]
    Validation(String),
    
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use AppError::*;
        let error_obj = match self {
            Auth(_) => json!({ "kind": "auth", "message": self.to_string() }),
            Database(_) => json!({ "kind": "database", "message": self.to_string() }),
            Validation(msg) => json!({ "kind": "validation", "message": msg }),
            Io(_) => json!({ "kind": "io", "message": self.to_string() }),
        };
        error_obj.serialize(serializer)
    }
}

// Now all handlers can use Result<T, AppError> and ? operator works automatically
```

**Impact:** Enables seamless error propagation across all 16 handlers.

---

### R-MACRO-003: Module Organization with Preludes

**Problem:** Import lists repeated across files. No central place for common imports.

**Priority:** Nice-to-have

**When to use:**
- Large codebases (> 10 modules)
- When same imports appear in > 5 files

**When NOT to use:**
- Small projects (< 5 modules)
- When imports are genuinely unique per file

**Before:**
```rust
// Every handler file:
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use crate::error::AppError;
use crate::state::AppState;
use crate::auth::User;
```

**After:**
```rust
// prelude.rs
pub use serde::{Serialize, Deserialize};
pub use std::sync::Arc;
pub use crate::error::AppError;
pub use crate::state::AppState;
pub use crate::auth::User;
pub use crate::types::*;

// Now in each file:
use crate::prelude::*;
```

**Source:** Rust API Guidelines - Module Organization

---

### R-MACRO-004: JSON-RPC Dispatcher Trait

**Problem:** Manual method name matching duplicated for each handler.

**Priority:** Must-do (eliminates huge switch statement)

**When to use:**
- Any JSON-RPC or similar dispatch system
- When you have > 5 command/handler mappings

**When NOT to use:**
- Tauri already handles this (`generate_handler!` macro)

**Before:**
```rust
async fn dispatch(method: &str, params: Value) -> Result<Value, Error> {
    match method {
        "handler_a" => handle_a(params).await,
        "handler_b" => handle_b(params).await,
        // ... repeat 14 more times
        _ => Err(Error::UnknownMethod),
    }
}
```

**After:**
```rust
use lazy_static::lazy_static;
use std::collections::HashMap;

type HandlerFn = Box<dyn Fn(Value) -> BoxFuture<'static, Result<Value, Error>> + Send + Sync>;

lazy_static! {
    static ref HANDLERS: HashMap<&'static str, HandlerFn> = {
        let mut m = HashMap::new();
        m.insert("handler_a", Box::new(|p| Box::pin(handle_a(p))) as HandlerFn);
        m.insert("handler_b", Box::new(|p| Box::pin(handle_b(p))) as HandlerFn);
        // ... still repetitive but isolated to one place
        m
    };
}

async fn dispatch(method: &str, params: Value) -> Result<Value, Error> {
    let handler = HANDLERS
        .get(method)
        .ok_or(Error::UnknownMethod)?;
    handler(params).await
}

// Better: use inventory crate for automatic registration
// See: https://docs.rs/inventory/latest/inventory/
```

**Note:** For Tauri specifically, use `tauri::generate_handler!` which does this automatically.

---

### R-MACRO-005: Utility Modules for Common Operations

**Problem:** Same validation/formatting/extraction logic scattered everywhere.

**Priority:** Must-do

**When to use:**
- Validation logic used in > 2 places
- Formatters, parsers, converters
- Common extractors (get field from JSON, parse header, etc.)

**When NOT to use:**
- Logic only used once

**Before:**
```rust
// In handler_a.rs
let id = req.params
    .get("id")
    .and_then(|v| v.as_str())
    .ok_or(Error::MissingField("id"))?;

// In handler_b.rs - exact same code!
let user_id = req.params
    .get("user_id")
    .and_then(|v| v.as_str())
    .ok_or(Error::MissingField("user_id"))?;
```

**After:**
```rust
// utils/extractors.rs
pub fn extract_string(obj: &Value, key: &str) -> Result<String, AppError> {
    obj.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::MissingField(key.to_string()))
}

pub fn extract_u64(obj: &Value, key: &str) -> Result<u64, AppError> {
    obj.get(key)
        .and_then(|v| v.as_u64())
        .ok_or_else(|| AppError::MissingField(key.to_string()))
}

// In handlers:
let id = extract_string(&req.params, "id")?;
let user_id = extract_u64(&req.params, "user_id")?;
```

---

## Vue/TypeScript Micro-Optimizations

### V-MICRO-001: Use `shallowRef` for Large Objects

**Problem:** Deep reactivity on large data structures causes performance overhead.

**Priority:** Must-do (for large data)

**When to use:**
- Objects with > 100 properties or deep nesting
- External library instances (ECharts, D3, Monaco Editor)
- Large arrays of data (> 1000 items)

**When NOT to use:**
- Small objects (< 20 properties)
- When you need reactivity on nested properties
- Forms where individual fields must be reactive

**Before:**
```ts
import { ref } from 'vue'

// Deep reactivity - tracks every nested property
const largeDataset = ref({
  items: new Array(10000).fill({ id: 0, name: '', value: 0 })
})

// Changing nested property triggers full reactivity
largeDataset.value.items[0].value = 100  // Slow!
```

**After:**
```ts
import { shallowRef, triggerRef } from 'vue'

// Only top-level is reactive
const largeDataset = shallowRef({
  items: new Array(10000).fill({ id: 0, name: '', value: 0 })
})

// Mutate nested property - no reactivity yet
largeDataset.value.items[0].value = 100

// Trigger reactivity manually when done
triggerRef(largeDataset)

// Or replace entire object (preferred)
largeDataset.value = {
  items: newItems
}
```

**Source:** Vue 3 Documentation - Reactivity Advanced

---

### V-MICRO-002: Prefer `computed` Over Watchers for Derived State

**Problem:** Watchers create imperative logic. `computed` is declarative and auto-cached.

**Priority:** Must-do

**When to use:**
- Deriving state from other state
- Filtering/mapping/reducing reactive data
- Any transformation that doesn't have side effects

**When NOT to use:**
- When you need side effects (use `watch` or `watchEffect`)
- When you need to debounce/throttle

**Before:**
```ts
import { ref, watch } from 'vue'

const users = ref<User[]>([])
const activeUsers = ref<User[]>([])

watch(users, (newUsers) => {
  // Imperative - manually sync derived state
  activeUsers.value = newUsers.filter(u => u.active)
}, { immediate: true })
```

**After:**
```ts
import { ref, computed } from 'vue'

const users = ref<User[]>([])

// Declarative - automatically updates and caches
const activeUsers = computed(() => 
  users.value.filter(u => u.active)
)
```

**Source:** Vue 3 Documentation - Computed Properties

---

### V-MICRO-003: Template Optimizations (`v-once`, `v-memo`)

**Problem:** Re-rendering static content wastes CPU.

**Priority:** Nice-to-have

**When to use:**
- Static headers, footers
- Content that never changes after initial render
- Large lists where only a few items change

**When NOT to use:**
- Content that updates frequently
- Simple templates (overhead not worth it)

**Before:**
```vue
<template>
  <div>
    <header>
      <h1>{{ appTitle }}</h1>  <!-- Re-renders on every update! -->
    </header>
    <ul>
      <li v-for="item in items" :key="item.id">
        {{ item.name }} - {{ item.value }}
      </li>
    </ul>
  </div>
</template>
```

**After:**
```vue
<template>
  <div>
    <header v-once>
      <!-- Renders once, never updates -->
      <h1>{{ appTitle }}</h1>
    </header>
    <ul>
      <li 
        v-for="item in items" 
        :key="item.id"
        v-memo="[item.value]"  <!-- Only re-render if value changes -->
      >
        {{ item.name }} - {{ item.value }}
      </li>
    </ul>
  </div>
</template>
```

**Source:** Vue 3 Documentation - Template Directives

---

### V-MICRO-004: Clean Up Event Listeners in `onUnmounted`

**Problem:** Forgetting to remove listeners causes memory leaks.

**Priority:** Must-do

**When to use:**
- Any `addEventListener` call
- Any interval/timeout
- Any external subscription

**When NOT to use:**
- When using Vue's template event handlers (`@click`, etc.)

**Before:**
```ts
import { onMounted } from 'vue'

onMounted(() => {
  window.addEventListener('resize', handleResize)
  // ❌ Memory leak - never removed!
})
```

**After:**
```ts
import { onMounted, onUnmounted } from 'vue'

onMounted(() => {
  window.addEventListener('resize', handleResize)
})

onUnmounted(() => {
  window.removeEventListener('resize', handleResize)
})
```

**Even better - use a composable:**
```ts
// composables/useEventListener.ts
import { onMounted, onUnmounted } from 'vue'

export function useEventListener(
  target: EventTarget,
  event: string,
  handler: EventListener
) {
  onMounted(() => target.addEventListener(event, handler))
  onUnmounted(() => target.removeEventListener(event, handler))
}

// Usage:
useEventListener(window, 'resize', handleResize)
```

**Source:** Vue 3 Composables Documentation

---

### V-MICRO-005: Avoid Reactive Wrapping of Non-Data Objects

**Problem:** Making component instances or class instances reactive causes overhead.

**Priority:** Must-do

**When to use:**
- External library instances
- Class instances
- DOM elements

**When NOT to use:**
- Plain data objects (use `reactive` or `ref`)

**Before:**
```ts
import { ref } from 'vue'
import { Editor } from 'monaco-editor'

const editor = ref<Editor | null>(null)  // ❌ Makes editor instance reactive!

onMounted(() => {
  editor.value = Editor.create(...)
  // Vue warning: Component made reactive
})
```

**After:**
```ts
import { shallowRef } from 'vue'
import { Editor } from 'monaco-editor'

const editor = shallowRef<Editor | null>(null)  // ✅ Only reference is reactive

onMounted(() => {
  editor.value = Editor.create(...)  // No warning
})
```

**Source:** Vue 3 Documentation - Performance Warning

---

## Vue/TypeScript Macro-Optimizations

### V-MACRO-001: Extract Composables for Shared Logic

**Problem:** Same logic repeated across components violates DRY.

**Priority:** Must-do

**When to use:**
- Logic used in > 2 components
- State + behavior that forms a cohesive unit
- Integration with external APIs/libraries

**When NOT to use:**
- Logic is truly component-specific
- Logic is only used once

**Before:**
```ts
// ComponentA.vue
const loading = ref(false)
const error = ref<string | null>(null)
const data = ref<User[]>([])

async function fetchUsers() {
  loading.value = true
  error.value = null
  try {
    const response = await fetch('/api/users')
    data.value = await response.json()
  } catch (e) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}

// ComponentB.vue - same code repeated!
const loading = ref(false)
const error = ref<string | null>(null)
const data = ref<Product[]>([])

async function fetchProducts() {
  loading.value = true
  error.value = null
  try {
    const response = await fetch('/api/products')
    data.value = await response.json()
  } catch (e) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}
```

**After:**
```ts
// composables/useFetch.ts
import { ref } from 'vue'

export function useFetch<T>(url: string) {
  const loading = ref(false)
  const error = ref<string | null>(null)
  const data = ref<T | null>(null)
  
  async function execute() {
    loading.value = true
    error.value = null
    try {
      const response = await fetch(url)
      data.value = await response.json()
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Unknown error'
    } finally {
      loading.value = false
    }
  }
  
  return { loading, error, data, execute }
}

// ComponentA.vue
const { loading, error, data, execute } = useFetch<User[]>('/api/users')
onMounted(() => execute())

// ComponentB.vue
const { loading, error, data, execute } = useFetch<Product[]>('/api/products')
onMounted(() => execute())
```

**Impact:** Eliminates hundreds of lines of duplicated async/error/loading logic.

**Source:** Vue 3 Composables Guide

---

### V-MACRO-002: Single-Purpose Composables Over Domain Composables

**Problem:** Large "domain" composables (like `useCart` with 20 methods) are hard to maintain and test.

**Priority:** Must-do (for large domains)

**When to use:**
- Domains with > 5 related operations
- When testing becomes difficult

**When NOT to use:**
- Simple domains with 2-3 operations
- When operations are tightly coupled

**Before:**
```ts
// composables/useCart.ts - 500+ lines!
export function useCart() {
  const cart = ref<Cart>({ items: [] })
  
  const addItem = (item: Item) => { /* 20 lines */ }
  const removeItem = (id: string) => { /* 15 lines */ }
  const updateQuantity = (id: string, qty: number) => { /* 25 lines */ }
  const clearCart = () => { /* 10 lines */ }
  const calculateTotal = () => { /* 30 lines */ }
  const applyDiscount = (code: string) => { /* 40 lines */ }
  const checkout = async () => { /* 60 lines */ }
  // ... 10 more methods
  
  return {
    cart,
    addItem,
    removeItem,
    updateQuantity,
    clearCart,
    calculateTotal,
    applyDiscount,
    checkout,
    // ... 10 more
  }
}
```

**After:**
```ts
// composables/cart/useCartState.ts
export function useCartState() {
  const cart = ref<Cart>({ items: [] })
  return { cart }
}

// composables/cart/useAddToCart.ts
export function useAddToCart() {
  const { cart } = useCartState()
  
  const addItem = (item: Item) => {
    // Focused, testable logic
  }
  
  return { addItem }
}

// composables/cart/useRemoveFromCart.ts
export function useRemoveFromCart() {
  const { cart } = useCartState()
  
  const removeItem = (id: string) => {
    // Focused, testable logic
  }
  
  return { removeItem }
}

// composables/cart/useCartCheckout.ts
export function useCartCheckout() {
  const { cart } = useCartState()
  
  const checkout = async () => {
    // Focused, testable logic
  }
  
  return { checkout }
}

// In component:
const { addItem } = useAddToCart()
const { checkout } = useCartCheckout()
```

**Impact:** Each composable is < 50 lines, easily testable, single responsibility.

**Source:** Vue Storefront Best Practices

---

### V-MACRO-003: Unified Tauri Invoke Wrapper

**Problem:** Every invoke call duplicates error handling, loading states, type assertions.

**Priority:** Must-do

**When to use:**
- All Tauri invoke calls
- Any API call pattern

**When NOT to use:**
- Never (always wrap repetitive patterns)

**Before:**
```ts
// ComponentA.vue
const loading = ref(false)
const error = ref<string | null>(null)

async function loadData() {
  loading.value = true
  error.value = null
  try {
    const result = await invoke('get_data', { id: 123 })
    // ... use result
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}

// ComponentB.vue - same pattern!
const loading = ref(false)
const error = ref<string | null>(null)

async function saveData() {
  loading.value = true
  error.value = null
  try {
    await invoke('save_data', { data: formData })
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}
```

**After:**
```ts
// composables/useTauriCommand.ts
import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'

export function useTauriCommand<T = void, P = Record<string, unknown>>(
  command: string
) {
  const loading = ref(false)
  const error = ref<string | null>(null)
  const data = ref<T | null>(null)
  
  async function execute(payload?: P): Promise<T | null> {
    loading.value = true
    error.value = null
    try {
      const result = await invoke<T>(command, payload)
      data.value = result
      return result
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e)
      error.value = errorMsg
      throw e
    } finally {
      loading.value = false
    }
  }
  
  return { loading, error, data, execute }
}

// ComponentA.vue
const { loading, error, data, execute } = useTauriCommand<DataResponse>('get_data')
execute({ id: 123 })

// ComponentB.vue
const { loading, error, execute } = useTauriCommand('save_data')
execute({ data: formData })
```

---

### V-MACRO-004: Pinia Store Organization - Module Stores

**Problem:** Single giant store becomes unmaintainable.

**Priority:** Must-do (for large apps)

**When to use:**
- App has > 3 distinct domains
- State doesn't all need to be shared globally

**When NOT to use:**
- Small apps with < 50 lines of store code

**Before:**
```ts
// stores/index.ts - 1000+ lines!
export const useMainStore = defineStore('main', () => {
  // Auth state
  const user = ref<User | null>(null)
  const token = ref<string | null>(null)
  
  // Cart state
  const cartItems = ref<CartItem[]>([])
  
  // UI state
  const theme = ref('light')
  const sidebarOpen = ref(false)
  
  // Settings state
  const settings = ref<Settings>({})
  
  // ... hundreds more lines
})
```

**After:**
```ts
// stores/auth.ts
export const useAuthStore = defineStore('auth', () => {
  const user = ref<User | null>(null)
  const token = ref<string | null>(null)
  
  const login = async (credentials: Credentials) => { /* ... */ }
  const logout = () => { /* ... */ }
  
  return { user, token, login, logout }
})

// stores/cart.ts
export const useCartStore = defineStore('cart', () => {
  const items = ref<CartItem[]>([])
  
  const addItem = (item: CartItem) => { /* ... */ }
  const removeItem = (id: string) => { /* ... */ }
  
  return { items, addItem, removeItem }
})

// stores/ui.ts
export const useUiStore = defineStore('ui', () => {
  const theme = ref('light')
  const sidebarOpen = ref(false)
  
  const toggleSidebar = () => { sidebarOpen.value = !sidebarOpen.value }
  
  return { theme, sidebarOpen, toggleSidebar }
})

// In components:
import { useAuthStore } from '@/stores/auth'
import { useCartStore } from '@/stores/cart'

const authStore = useAuthStore()
const cartStore = useCartStore()
```

---

### V-MACRO-005: Web Worker for Heavy Operations

**Problem:** Heavy computations block UI thread.

**Priority:** Nice-to-have (must-do for CPU-heavy operations)

**When to use:**
- Large data parsing (CSV, JSON > 1MB)
- Cryptography, hashing
- Image processing
- Complex calculations

**When NOT to use:**
- Operations < 50ms
- When data transfer cost > computation cost

**Before:**
```ts
// Runs on main thread - freezes UI!
const processLargeDataset = (data: LargeData[]) => {
  return data.map(item => {
    // Complex computation
    return complexTransform(item)
  })
}

const result = processLargeDataset(largeDataset)  // UI frozen!
```

**After:**
```ts
// workers/dataProcessor.ts
self.onmessage = (e) => {
  const data = e.data
  const result = data.map(item => complexTransform(item))
  self.postMessage(result)
}

// composables/useDataWorker.ts
export function useDataWorker() {
  const worker = new Worker(new URL('../workers/dataProcessor.ts', import.meta.url))
  
  const process = (data: LargeData[]): Promise<ProcessedData[]> => {
    return new Promise((resolve) => {
      worker.onmessage = (e) => resolve(e.data)
      worker.postMessage(data)
    })
  }
  
  onUnmounted(() => worker.terminate())
  
  return { process }
}

// In component:
const { process } = useDataWorker()
const result = await process(largeDataset)  // UI stays responsive!
```

---

### V-MACRO-006: Polling Consolidation Pattern

**Problem:** Multiple components polling independently wastes resources.

**Priority:** Nice-to-have (must-do if you have > 3 pollers)

**When to use:**
- Multiple pollers with same or similar intervals
- Polling the same backend resource

**When NOT to use:**
- Each poller has unique requirements
- Only 1-2 pollers in entire app

**Before:**
```ts
// ComponentA.vue
const intervalA = setInterval(() => {
  fetchDataA()
}, 5000)

// ComponentB.vue
const intervalB = setInterval(() => {
  fetchDataB()
}, 5000)

// ComponentC.vue
const intervalC = setInterval(() => {
  fetchDataC()
}, 5000)

// 3 separate intervals all running!
```

**After:**
```ts
// composables/usePolling.ts
import { ref, onUnmounted } from 'vue'

type PollingCallback = () => void | Promise<void>

const pollingCallbacks = new Set<PollingCallback>()
let pollingInterval: number | null = null

export function usePolling(callback: PollingCallback, intervalMs = 5000) {
  const start = () => {
    pollingCallbacks.add(callback)
    
    if (!pollingInterval) {
      pollingInterval = setInterval(() => {
        pollingCallbacks.forEach(cb => cb())
      }, intervalMs)
    }
  }
  
  const stop = () => {
    pollingCallbacks.delete(callback)
    
    if (pollingCallbacks.size === 0 && pollingInterval) {
      clearInterval(pollingInterval)
      pollingInterval = null
    }
  }
  
  onUnmounted(stop)
  
  return { start, stop }
}

// ComponentA.vue
const { start } = usePolling(fetchDataA)
onMounted(start)

// ComponentB.vue
const { start } = usePolling(fetchDataB)
onMounted(start)

// Now only ONE interval handles all polling!
```

---

## Tauri-Specific Patterns

### T-001: Batch IPC Calls to Reduce Overhead

**Problem:** Each `invoke` has serialization/IPC overhead. Hundreds of small calls are slow.

**Priority:** Must-do (for high-frequency operations)

**When to use:**
- Loading initial app data (many resources at once)
- Bulk operations (save 100 items)
- Related operations that can be grouped

**When NOT to use:**
- Operations need to happen at different times
- Operations are independent and timing matters

**Before:**
```ts
// 10 separate IPC calls!
const user = await invoke('get_user')
const settings = await invoke('get_settings')
const preferences = await invoke('get_preferences')
const notifications = await invoke('get_notifications')
const messages = await invoke('get_messages')
// ... 5 more
```

**After:**
```rust
// Rust side
#[derive(Serialize)]
struct AppInitData {
    user: User,
    settings: Settings,
    preferences: Preferences,
    notifications: Vec<Notification>,
    messages: Vec<Message>,
}

#[tauri::command]
async fn init_app_data(state: State<'_, AppState>) -> Result<AppInitData, AppError> {
    let user = get_user(&state).await?;
    let settings = get_settings(&state).await?;
    let preferences = get_preferences(&state).await?;
    let notifications = get_notifications(&state).await?;
    let messages = get_messages(&state).await?;
    
    Ok(AppInitData {
        user,
        settings,
        preferences,
        notifications,
        messages,
    })
}

// Frontend
const appData = await invoke<AppInitData>('init_app_data')
// Single IPC call with all data!
```

**Impact:** 10 IPC round-trips → 1 IPC round-trip

**Source:** Tauri Performance Discussions

---

### T-002: Use `tauri::ipc::Response` for Large Binary Data

**Problem:** Serde serialization is slow for large binary data (files, images).

**Priority:** Must-do (for files > 100KB)

**When to use:**
- Returning file contents
- Image data
- Any binary data > 100KB

**When NOT to use:**
- Small JSON responses
- Structured data (use serde)

**Before:**
```rust
#[tauri::command]
async fn read_image(path: String) -> Result<Vec<u8>, AppError> {
    let bytes = tokio::fs::read(&path).await?;
    Ok(bytes)  // Slow! Serialized as JSON array of numbers
}
```

**After:**
```rust
use tauri::ipc::Response;

#[tauri::command]
async fn read_image(path: String) -> Result<Response, AppError> {
    let bytes = tokio::fs::read(&path).await?;
    Ok(Response::new(bytes))  // Fast! Sent as ArrayBuffer
}
```

**Source:** Tauri Documentation - Calling Rust

---

### T-003: State Management - Use Tauri State for Rust, Pinia for Frontend

**Problem:** Confusion about where state belongs. Duplicated state across boundary.

**Priority:** Must-do

**When to use:**
- **Tauri State:** Database connections, file handles, API clients, app configuration
- **Pinia State:** UI state, user preferences, temporary data, derived state

**When NOT to use:**
- Don't duplicate state across both (pick one source of truth)

**Before:**
```rust
// ❌ Storing UI state in Rust
struct AppState {
    db: Database,
    sidebar_open: bool,  // UI state doesn't belong here!
    current_theme: String,  // This should be in frontend
}
```

**After:**
```rust
// ✅ Rust state: resources and backend concerns
struct AppState {
    db: Database,
    config: Config,
    api_client: ApiClient,
}

// Frontend Pinia: UI and derived state
export const useUiStore = defineStore('ui', () => {
  const sidebarOpen = ref(false)
  const currentTheme = ref('light')
  return { sidebarOpen, currentTheme }
})
```

**Guideline:**
- If state controls Rust behavior → Tauri State
- If state controls UI rendering → Pinia
- If state is from backend but used in UI → Fetch to Pinia via invoke

---

### T-004: Use Events for Backend → Frontend Push

**Problem:** Frontend polling for updates wastes resources.

**Priority:** Nice-to-have (must-do for real-time features)

**When to use:**
- File watcher notifications
- Background task progress
- Real-time updates from backend services

**When NOT to use:**
- Request-response patterns (use commands)
- Data that doesn't change frequently

**Before:**
```ts
// ❌ Polling every 1s for file changes
setInterval(async () => {
  const changed = await invoke('check_file_changed')
  if (changed) {
    // Update UI
  }
}, 1000)
```

**After:**
```rust
// Rust side - emit events
use tauri::Manager;

async fn watch_file(app: AppHandle, path: String) {
    let mut watcher = FileWatcher::new(&path);
    
    while let Some(event) = watcher.next().await {
        app.emit_all("file-changed", FileChangePayload {
            path: path.clone(),
            timestamp: chrono::Utc::now(),
        }).ok();
    }
}

#[tauri::command]
async fn start_watching(app: AppHandle, path: String) {
    tokio::spawn(watch_file(app, path));
    Ok(())
}

// Frontend - listen for events
import { listen } from '@tauri-apps/api/event'

const unlisten = await listen<FileChangePayload>('file-changed', (event) => {
  console.log('File changed:', event.payload.path)
  // Update UI reactively
})
```

**Source:** Tauri Event System Documentation

---

### T-005: Type-Safe Command Wrappers

**Problem:** Raw `invoke` calls have no type safety. Easy to get command names wrong.

**Priority:** Nice-to-have (high value for maintainability)

**When to use:**
- All invoke calls
- Large apps with many commands

**When NOT to use:**
- Prototypes (overhead not worth it)

**Before:**
```ts
// ❌ No type safety, magic strings
const user = await invoke('get_user', { id: 123 })
const result = await invoke('save_data', { data: formData })

// Typos compile but fail at runtime:
const broken = await invoke('get_usr', { id: 123 })  // Oops!
```

**After:**
```ts
// api/commands.ts
import { invoke } from '@tauri-apps/api/core'

export interface GetUserParams {
  id: number
}

export interface User {
  id: number
  name: string
  email: string
}

export const commands = {
  async getUser(params: GetUserParams): Promise<User> {
    return invoke<User>('get_user', params)
  },
  
  async saveData(data: FormData): Promise<void> {
    return invoke('save_data', { data })
  },
  
  // ... all commands typed
} as const

// In component - fully typed!
const user = await commands.getUser({ id: 123 })
//     ^? User

// Typo caught at compile time:
const broken = await commands.getUsr({ id: 123 })  // ❌ Compile error!
```

---

### T-006: Plugin Pattern for Shared Backend Functionality

**Problem:** Same backend logic needed by multiple commands leads to duplication.

**Priority:** Nice-to-have

**When to use:**
- Auth/permission checking used by many commands
- Logging/telemetry
- Database transaction management

**When NOT to use:**
- Logic only used by one command

**Before:**
```rust
#[tauri::command]
async fn command_a(state: State<'_, AppState>) -> Result<ResponseA, AppError> {
    // Auth check duplicated
    let user = get_current_user(&state)?;
    require_permission(&user, "action_a")?;
    
    // Logging duplicated
    log::info!("Command A called by {}", user.id);
    
    // Business logic
    do_work_a(&state).await
}

#[tauri::command]
async fn command_b(state: State<'_, AppState>) -> Result<ResponseB, AppError> {
    // Same auth check duplicated
    let user = get_current_user(&state)?;
    require_permission(&user, "action_b")?;
    
    // Same logging duplicated
    log::info!("Command B called by {}", user.id);
    
    // Business logic
    do_work_b(&state).await
}
```

**After:**
```rust
// plugins/auth_plugin.rs
pub struct AuthPlugin;

impl AuthPlugin {
    pub fn check_permission(
        state: &AppState,
        required: &str,
    ) -> Result<User, AppError> {
        let user = get_current_user(state)?;
        require_permission(&user, required)?;
        Ok(user)
    }
}

// plugins/logging_plugin.rs
pub struct LoggingPlugin;

impl LoggingPlugin {
    pub fn log_command(command: &str, user: &User) {
        log::info!("Command {} called by {}", command, user.id);
    }
}

// Now commands are clean
#[tauri::command]
async fn command_a(state: State<'_, AppState>) -> Result<ResponseA, AppError> {
    let user = AuthPlugin::check_permission(&state, "action_a")?;
    LoggingPlugin::log_command("command_a", &user);
    
    do_work_a(&state).await  // Only business logic here
}

#[tauri::command]
async fn command_b(state: State<'_, AppState>) -> Result<ResponseB, AppError> {
    let user = AuthPlugin::check_permission(&state, "action_b")?;
    LoggingPlugin::log_command("command_b", &user);
    
    do_work_b(&state).await
}
```

---

## Quick Reference Matrix

| Pattern ID | Name | Priority | Impact | Effort |
|------------|------|----------|--------|--------|
| R-MICRO-001 | Use `&str` over `&String` | Must-do | Medium | Low |
| R-MICRO-002 | `Cow<'static, str>` | Nice-to-have | High | Medium |
| R-MICRO-003 | Avoid unnecessary clones | Must-do | High | Low |
| R-MICRO-004 | `thiserror` for errors | Must-do | Medium | Low |
| R-MICRO-005 | Serde attributes | Nice-to-have | Medium | Low |
| R-MICRO-006 | Async + `spawn_blocking` | Must-do | High | Medium |
| R-MICRO-007 | Reuse collections | Nice-to-have | High | Low |
| **R-MACRO-001** | **Handler trait abstraction** | **Must-do** | **Very High** | **High** |
| **R-MACRO-002** | **Unified error types** | **Must-do** | **Very High** | **Medium** |
| R-MACRO-003 | Module preludes | Nice-to-have | Low | Low |
| R-MACRO-004 | JSON-RPC dispatcher | Must-do | High | Medium |
| **R-MACRO-005** | **Utility modules** | **Must-do** | **High** | **Medium** |
| **V-MICRO-001** | **`shallowRef` for large objects** | **Must-do** | **High** | **Low** |
| V-MICRO-002 | `computed` over watchers | Must-do | Medium | Low |
| V-MICRO-003 | Template optimizations | Nice-to-have | Medium | Low |
| V-MICRO-004 | Clean up listeners | Must-do | High | Low |
| V-MICRO-005 | Avoid reactive wrapping | Must-do | Medium | Low |
| **V-MACRO-001** | **Extract composables** | **Must-do** | **Very High** | **Medium** |
| **V-MACRO-002** | **Single-purpose composables** | **Must-do** | **High** | **Medium** |
| **V-MACRO-003** | **Unified Tauri wrapper** | **Must-do** | **Very High** | **Medium** |
| **V-MACRO-004** | **Module stores (Pinia)** | **Must-do** | **High** | **Medium** |
| V-MACRO-005 | Web workers | Nice-to-have | High | High |
| V-MACRO-006 | Polling consolidation | Nice-to-have | Medium | Medium |
| **T-001** | **Batch IPC calls** | **Must-do** | **Very High** | **Medium** |
| T-002 | `ipc::Response` for binary | Must-do | High | Low |
| T-003 | State boundaries | Must-do | Medium | Low |
| T-004 | Events for push | Nice-to-have | High | Medium |
| T-005 | Typed command wrappers | Nice-to-have | Medium | Medium |
| T-006 | Plugin pattern | Nice-to-have | Medium | Medium |

**Legend:**
- **Must-do:** Apply during refactor
- **Nice-to-have:** Apply if time permits or context fits
- **Situational:** Evaluate case-by-case

---

## Sources

1. **Rust Performance Book** - Nicholas Nethercote  
   https://nnethercote.github.io/perf-book/

2. **Llogiq on Rust Performance Pitfalls**  
   https://llogiq.github.io/2017/06/01/perf-pitfalls.html

3. **Leapcell - Zero-Cost String Handling with Cow**  
   https://leapcell.io/blog/zero-cost-string-handling-in-rust-web-apis-with-cow

4. **Vue 3 Official Documentation - Composables**  
   https://vuejs.org/guide/reusability/composables.html

5. **Vue 3 Reactivity Advanced API**  
   https://vuejs.org/api/reactivity-advanced

6. **Tauri v2 IPC Documentation**  
   https://v2.tauri.app/concept/inter-process-communication/

7. **Tauri v2 Calling Rust from Frontend**  
   https://v2.tauri.app/develop/calling-rust/

8. **DEV Community - Vue Composable Best Practices**  
   https://dev.to/jacobandrewsky/good-practices-and-design-patterns-for-vue-composables-24lk

---

## Revision History

- **v1.0** (2026-02-13): Initial compilation by Lorgaire (Senchán's dalta)

---

*End of Optimization Reference*
