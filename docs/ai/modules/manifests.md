# Manifests Module

> **Location:** `src-tauri/src/manifest/`  
> **Files:** 4  
> **Lines:** 510  
> **Purpose:** Capability manifest loading, validation, and registry

---

## Overview

Manifests are JSON files that declare tools, their requirements (credentials, permissions), and implementation details. The manifest system enables dynamic capability loading without code changes — add a manifest file, restart Tairseach, and the tool is available.

**Storage:** `~/.tairseach/manifests/*.json`

**Schema Version:** `1.0.0`

---

## File Listing

| File | Lines | Purpose |
|------|-------|---------|
| `types.rs` | ~200 | Manifest schema types (Rust structs matching JSON) |
| `loader.rs` | ~120 | File I/O, JSON parsing, validation |
| `registry.rs` | ~140 | In-memory registry + hot-reload watcher |
| `mod.rs` | ~50 | Module exports |

---

## Key Types

### From `types.rs`

```rust
pub struct Manifest {
    pub manifest_version: String,
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub category: String,
    pub requires: Requirements,
    pub tools: Vec<Tool>,
    pub implementation: Implementation,
}

pub struct Requirements {
    pub credentials: Vec<CredentialRequirement>,
    pub permissions: Vec<PermissionRequirement>,
}

pub struct CredentialRequirement {
    pub id: String,
    pub provider: Option<String>,
    pub kind: Option<String>,
    pub scopes: Vec<String>,
    pub optional: bool,
}

pub struct PermissionRequirement {
    pub name: String,
    pub optional: bool,
    pub reason: Option<String>,
}

pub struct Tool {
    pub name: String,
    pub title: Option<String>,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub annotations: HashMap<String, serde_json::Value>,
    pub requires: Option<Requirements>,  // Per-tool overrides
}

pub enum Implementation {
    Internal {
        module: String,
        methods: HashMap<String, String>,
    },
    Script {
        runtime: String,
        entrypoint: String,
        args: Vec<String>,
        env: HashMap<String, String>,
        tool_bindings: HashMap<String, ScriptToolBinding>,
    },
    Proxy {
        base_url: String,
        auth: ProxyAuth,
        tool_bindings: HashMap<String, ProxyToolBinding>,
    },
}

pub struct ProxyAuth {
    pub strategy: String,
    pub credential_id: String,
    pub header_name: Option<String>,
    pub query_param: Option<String>,
    pub token_field: Option<String>,
}

pub struct ProxyToolBinding {
    pub method: String,
    pub path: String,
    pub query: HashMap<String, String>,
    pub body_template: Option<serde_json::Value>,
    pub headers: HashMap<String, String>,
    pub response_path: Option<String>,
}

pub struct ScriptToolBinding {
    pub action: String,
    pub input_mode: Option<String>,
    pub output_mode: Option<String>,
}
```

---

## Manifest Examples

### Internal Implementation

```json
{
  "manifest_version": "1.0.0",
  "id": "com.naonur.tairseach.gmail",
  "name": "Gmail",
  "description": "Gmail API integration",
  "version": "1.0.0",
  "category": "communication",
  "requires": {
    "credentials": [
      {
        "id": "google_oauth",
        "provider": "google",
        "scopes": [
          "https://www.googleapis.com/auth/gmail.modify"
        ]
      }
    ],
    "permissions": []
  },
  "tools": [
    {
      "name": "list_messages",
      "description": "List Gmail messages",
      "inputSchema": {
        "type": "object",
        "properties": {
          "query": { "type": "string" },
          "maxResults": { "type": "number" }
        }
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "messages": { "type": "array" }
        }
      }
    }
  ],
  "implementation": {
    "type": "internal",
    "module": "gmail",
    "methods": {
      "list_messages": "list_messages",
      "get_message": "get_message"
    }
  }
}
```

---

### Proxy Implementation

```json
{
  "manifest_version": "1.0.0",
  "id": "com.naonur.tairseach.oura",
  "name": "Oura Ring",
  "description": "Oura Ring API integration",
  "version": "1.0.0",
  "category": "health",
  "requires": {
    "credentials": [
      {
        "id": "oura_api_key",
        "kind": "api_key"
      }
    ]
  },
  "tools": [
    {
      "name": "get_sleep",
      "description": "Get sleep data",
      "inputSchema": {
        "type": "object",
        "properties": {
          "start_date": { "type": "string" },
          "end_date": { "type": "string" }
        }
      },
      "outputSchema": {
        "type": "object"
      }
    }
  ],
  "implementation": {
    "type": "proxy",
    "baseUrl": "https://api.ouraring.com/v2",
    "auth": {
      "strategy": "oauth2",
      "credentialId": "oura_api_key",
      "headerName": "Authorization",
      "tokenField": "access_token"
    },
    "toolBindings": {
      "get_sleep": {
        "method": "GET",
        "path": "/usercollection/sleep",
        "query": {
          "start_date": "{{start_date}}",
          "end_date": "{{end_date}}"
        },
        "responsePath": "$.data"
      }
    }
  }
}
```

---

### Script Implementation

```json
{
  "manifest_version": "1.0.0",
  "id": "com.naonur.tairseach.onepassword",
  "name": "1Password",
  "description": "1Password CLI integration",
  "version": "1.0.0",
  "category": "security",
  "tools": [
    {
      "name": "list_items",
      "description": "List 1Password items",
      "inputSchema": {
        "type": "object",
        "properties": {
          "vault": { "type": "string" }
        }
      },
      "outputSchema": {
        "type": "array"
      }
    }
  ],
  "implementation": {
    "type": "script",
    "runtime": "op-helper",
    "entrypoint": "",
    "toolBindings": {
      "list_items": {
        "action": "list",
        "input_mode": "json",
        "output_mode": "json"
      }
    }
  }
}
```

---

## ManifestRegistry API

```rust
impl ManifestRegistry {
    pub fn new() -> Self
    
    pub async fn load_from_disk(&self) -> Result<(), String>
    
    pub async fn start_watcher(&self) -> Result<(), String>
    
    pub fn get_manifest(&self, id: &str) -> Option<Arc<Manifest>>
    
    pub fn get_tool(&self, manifest_id: &str, tool_name: &str) -> Option<Arc<Tool>>
    
    pub fn find_tool(&self, method: &str) -> Option<(Arc<Manifest>, Arc<Tool>)>
    
    pub fn list_manifests(&self) -> Vec<Arc<Manifest>>
    
    pub fn list_tools(&self) -> Vec<(String, Arc<Tool>)>
}
```

**Usage:**
```rust
let registry = ManifestRegistry::new();
registry.load_from_disk().await?;

// Find tool by method name
if let Some((manifest, tool)) = registry.find_tool("gmail.list_messages") {
    println!("Found tool: {} in {}", tool.name, manifest.name);
}

// Start hot-reload watcher
tokio::spawn(async move {
    registry.start_watcher().await.unwrap();
});
```

---

## Hot-Reload Watcher

```rust
// registry.rs
pub async fn start_watcher(&self) -> Result<(), String> {
    use notify::{Watcher, RecursiveMode, Event, EventKind};
    
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    
    let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
        if let Ok(event) = res {
            if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                tx.blocking_send(event).ok();
            }
        }
    })?;
    
    watcher.watch(&manifests_dir(), RecursiveMode::NonRecursive)?;
    
    while let Some(_event) = rx.recv().await {
        tracing::info!("Manifest file changed, reloading...");
        self.load_from_disk().await?;
    }
    
    Ok(())
}
```

**Behavior:** Watches `~/.tairseach/manifests/` for changes and reloads manifests automatically.

---

## Validation

```rust
impl Manifest {
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
        
        // Validate tool names are unique
        let mut seen = std::collections::HashSet::new();
        for tool in &self.tools {
            if !seen.insert(&tool.name) {
                return Err(format!("Duplicate tool name: {}", tool.name));
            }
        }
        
        Ok(())
    }
}
```

---

## Dependencies

| Module | Imports |
|--------|---------|
| **router** | Uses `ManifestRegistry` to find tools and dispatch |
| **common/paths** | `manifests_dir()` for storage location |
| **notify** | File system watching for hot-reload |

---

## Adding a New Manifest

1. **Create JSON file** in `~/.tairseach/manifests/`
   ```bash
   touch ~/.tairseach/manifests/my-capability.json
   ```

2. **Define structure** (see examples above)

3. **Validate schema:**
   ```json
   {
     "manifest_version": "1.0.0",
     "id": "com.example.my-capability",
     "name": "My Capability",
     "description": "...",
     "version": "1.0.0",
     "category": "...",
     "requires": { ... },
     "tools": [ ... ],
     "implementation": { ... }
   }
   ```

4. **Restart Tairseach** (or wait for hot-reload)

5. **Test via MCP:**
   ```bash
   cd crates/tairseach-mcp
   cargo run -- --socket ~/.tairseach/socket
   # Call your new tool
   ```

**See also:** [patterns/manifest-pattern.md](../patterns/manifest-pattern.md)

---

## Monitoring

Manifests are exposed via Tauri commands:

```rust
#[tauri::command]
async fn get_all_manifests() -> Result<Vec<Manifest>, String>

#[tauri::command]
async fn get_manifest_summary() -> Result<Value, String>
```

Frontend can display loaded manifests in Monitor view.

---

*For routing logic, see [router.md](router.md)*
