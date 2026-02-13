# Manifest System Architecture

**Component:** Declarative Capability Definitions  
**Location:** `src-tauri/src/manifest/`, `~/.tairseach/manifests/`  
**Schema Version:** 1.0.0

---

## Purpose

The manifest system provides declarative capability definitions that decouple tool registration from code. Manifests define:

1. **Tools** — Available operations (name, description, input/output schemas)
2. **Requirements** — Permissions and credentials needed
3. **Implementation** — How to execute (internal Rust, HTTP proxy, or external script)

## Architecture

```
~/.tairseach/manifests/
├── core/                  # Built-in macOS integrations
│   ├── contacts.json
│   ├── calendar.json
│   ├── reminders.json
│   ├── auth.json
│   └── permissions.json
└── integrations/          # Cloud services
    ├── google-gmail.json
    ├── google-calendar-api.json
    ├── oura.json
    ├── onepassword.json
    └── jira.json

           ↓ (loaded at startup)

    ManifestRegistry
    ├── manifests: HashMap<String, Manifest>
    └── tool_index: HashMap<String, (ManifestId, Tool)>

           ↓ (queried by router)

    CapabilityRouter
    └── route(tool_name) → find_tool() → dispatch()
```

## Manifest Schema

### Complete Example: Oura Integration

**File:** `~/.tairseach/manifests/integrations/oura.json`

```json
{
  "manifest_version": "1.0.0",
  "id": "oura",
  "name": "Oura Ring",
  "description": "Access Oura Ring health data (sleep, activity, readiness)",
  "version": "0.1.0",
  "category": "health",
  
  "requires": {
    "credentials": [
      {
        "id": "oura-token",
        "provider": "oura",
        "kind": "token",
        "scopes": [],
        "description": "Personal Access Token from cloud.ouraring.com"
      }
    ],
    "permissions": []
  },
  
  "tools": [
    {
      "name": "oura_sleep",
      "title": "Get Sleep Data",
      "description": "Retrieve sleep sessions and scores for a date range",
      "inputSchema": {
        "type": "object",
        "properties": {
          "account": {
            "type": "string",
            "default": "default",
            "description": "Credential account identifier"
          },
          "start_date": {
            "type": "string",
            "pattern": "^\\d{4}-\\d{2}-\\d{2}$",
            "description": "Start date (YYYY-MM-DD)"
          },
          "end_date": {
            "type": "string",
            "pattern": "^\\d{4}-\\d{2}-\\d{2}$",
            "description": "End date (YYYY-MM-DD)"
          }
        },
        "required": ["start_date", "end_date"],
        "additionalProperties": false
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "data": {
            "type": "array",
            "items": { "type": "object" }
          }
        }
      },
      "annotations": {
        "readOnlyHint": true,
        "destructiveHint": false,
        "openWorldHint": true
      }
    },
    {
      "name": "oura_activity",
      "description": "Retrieve daily activity data..."
      // ... similar structure
    }
  ],
  
  "implementation": {
    "type": "proxy",
    "baseUrl": "https://api.ouraring.com/v2",
    "auth": {
      "strategy": "bearer",
      "credentialId": "oura-token",
      "tokenField": "personal_access_token"
    },
    "toolBindings": {
      "oura_sleep": {
        "method": "GET",
        "path": "/usercollection/sleep",
        "query": {
          "start_date": "{start_date}",
          "end_date": "{end_date}"
        },
        "responsePath": "$"
      },
      "oura_activity": {
        "method": "GET",
        "path": "/usercollection/daily_activity",
        "query": {
          "start_date": "{start_date}",
          "end_date": "{end_date}"
        }
      }
    }
  }
}
```

## Implementation Types

### Type 1: Internal (Native Rust)

**Use when:** Accessing macOS APIs, performance critical, or complex logic

**Example:** Contacts

```json
{
  "implementation": {
    "type": "internal",
    "module": "proxy.handlers.contacts",
    "methods": {
      "contacts_list": "contacts.list",
      "contacts_get": "contacts.get",
      "contacts_create": "contacts.create"
    }
  }
}
```

**Dispatch:** Routes to existing handler in `proxy/handlers/contacts.rs`

### Type 2: Proxy (HTTP API)

**Use when:** Calling external REST APIs

**Example:** Gmail

```json
{
  "implementation": {
    "type": "proxy",
    "baseUrl": "https://gmail.googleapis.com/gmail/v1/users/me",
    "auth": {
      "strategy": "bearer",
      "credentialId": "google-oauth",
      "tokenField": "access_token",
      "headerName": "Authorization"
    },
    "toolBindings": {
      "gmail_list_messages": {
        "method": "GET",
        "path": "/messages",
        "query": {
          "q": "{query}",
          "maxResults": "{max_results}"
        },
        "responsePath": "$.messages"
      }
    }
  }
}
```

**Dispatch:** Capability router makes HTTP request, injects auth header

### Type 3: Script (External Process)

**Use when:** Integrating existing CLIs or complex external tools

**Example:** Custom backup script

```json
{
  "implementation": {
    "type": "script",
    "runtime": "bash",
    "entrypoint": "~/.tairseach/scripts/backup.sh",
    "args": ["--mode", "{mode}"],
    "env": {
      "BACKUP_TOKEN": "{credential:backup-api:api_key}"
    },
    "toolBindings": {
      "backup_run": {
        "action": "backup",
        "input_mode": "args",
        "output_mode": "json"
      }
    }
  }
}
```

**Dispatch:** Spawns process with interpolated args/env, captures stdout

## Key Files

### `types.rs` — Schema Definitions (~200 lines)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
    #[serde(rename = "outputSchema")]
    pub output_schema: Value,
    #[serde(default)]
    pub annotations: HashMap<String, Value>,
    #[serde(default)]
    pub requires: Option<Requirements>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Implementation {
    Internal { module: String, methods: HashMap<String, String> },
    Proxy { base_url: String, auth: ProxyAuth, tool_bindings: HashMap<String, ProxyToolBinding> },
    Script { runtime: String, entrypoint: String, tool_bindings: HashMap<String, ScriptToolBinding> },
}
```

### `loader.rs` — Manifest Discovery (~150 lines)

```rust
pub async fn load_manifests(dir: &Path) -> Result<Vec<Manifest>, String> {
    let mut manifests = Vec::new();
    
    for entry in WalkDir::new(dir).filter_entry(|e| !is_hidden(e)) {
        let entry = entry?;
        if entry.path().extension() == Some(OsStr::new("json")) {
            let content = tokio::fs::read_to_string(entry.path()).await?;
            let manifest: Manifest = serde_json::from_str(&content)?;
            manifest.validate()?;
            manifests.push(manifest);
        }
    }
    
    Ok(manifests)
}
```

### `registry.rs` — Hot Reload & Tool Index (~250 lines)

```rust
pub struct ManifestRegistry {
    manifests: RwLock<HashMap<String, Manifest>>,
    tool_index: RwLock<HashMap<String, (String, Tool)>>,
    manifest_dir: PathBuf,
}

impl ManifestRegistry {
    pub async fn load_from_disk(&self) -> Result<(), String> {
        let manifests = load_manifests(&self.manifest_dir).await?;
        
        let mut map = HashMap::new();
        let mut index = HashMap::new();
        
        for manifest in manifests {
            for tool in &manifest.tools {
                index.insert(tool.name.clone(), (manifest.id.clone(), tool.clone()));
            }
            map.insert(manifest.id.clone(), manifest);
        }
        
        *self.manifests.write().await = map;
        *self.tool_index.write().await = index;
        
        Ok(())
    }
    
    pub async fn find_tool(&self, name: &str) -> Option<(Manifest, Tool)> {
        let index = self.tool_index.read().await;
        let (manifest_id, tool) = index.get(name)?.clone();
        
        let manifests = self.manifests.read().await;
        let manifest = manifests.get(&manifest_id)?.clone();
        
        Some((manifest, tool))
    }
    
    pub async fn start_watcher(&self) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx)?;
        watcher.watch(&self.manifest_dir, RecursiveMode::Recursive)?;
        
        tokio::spawn(async move {
            while let Ok(event) = rx.recv() {
                if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                    info!("Manifest change detected, reloading...");
                    if let Err(e) = self.load_from_disk().await {
                        error!("Failed to reload manifests: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }
}
```

## Validation

```rust
impl Manifest {
    pub fn validate(&self) -> Result<(), String> {
        // Version check
        if self.manifest_version != MANIFEST_VERSION {
            return Err(format!("Unsupported version: {}", self.manifest_version));
        }
        
        // Tool name validation
        for tool in &self.tools {
            if !is_valid_identifier(&tool.name) {
                return Err(format!("Invalid tool name: {}", tool.name));
            }
        }
        
        // Implementation completeness
        match &self.implementation {
            Implementation::Internal { methods, .. } => {
                for tool in &self.tools {
                    if !methods.contains_key(&tool.name) {
                        return Err(format!("Missing method mapping for {}", tool.name));
                    }
                }
            }
            Implementation::Proxy { tool_bindings, .. } |
            Implementation::Script { tool_bindings, .. } => {
                for tool in &self.tools {
                    if !tool_bindings.contains_key(&tool.name) {
                        return Err(format!("Missing binding for {}", tool.name));
                    }
                }
            }
        }
        
        Ok(())
    }
}
```

## Tool Annotations

**Purpose:** Hints for AI agents to make better decisions

```json
{
  "annotations": {
    "readOnlyHint": true,      // Doesn't modify data
    "destructiveHint": false,  // Doesn't delete/overwrite
    "idempotentHint": true,    // Safe to retry
    "openWorldHint": true      // Returns exhaustive list
  }
}
```

**Usage:** MCP bridge exposes these in tool metadata

## Hot Reload

**Trigger:** File change in `~/.tairseach/manifests/`

**Flow:**
```
File change → notify::watcher → reload event
               ↓
    ManifestRegistry::load_from_disk()
               ↓
    Parse all manifests
               ↓
    Rebuild tool_index
               ↓
    Swap RwLock contents (atomic)
               ↓
    Next request uses new manifests
```

**Limitation:** Active connections don't see changes until their next request

## Common Patterns

### Multi-Tool Manifest

```json
{
  "tools": [
    {"name": "list", ...},
    {"name": "get", ...},
    {"name": "create", ...},
    {"name": "update", ...},
    {"name": "delete", ...}
  ]
}
```

### Shared Requirements

```json
{
  "requires": {
    "credentials": [{"id": "api-key", ...}],
    "permissions": [{"name": "contacts"}]
  },
  "tools": [
    // All tools inherit these requirements
    {"name": "tool1", ...},
    {"name": "tool2", ...}
  ]
}
```

### Tool-Level Override

```json
{
  "requires": {"permissions": [{"name": "contacts"}]},
  "tools": [
    {
      "name": "read_tool",
      // Uses manifest-level requirement
    },
    {
      "name": "write_tool",
      "requires": {
        "permissions": [
          {"name": "contacts"},
          {"name": "full_disk_access"}  // Additional requirement
        ]
      }
    }
  ]
}
```

## Related Documentation

- **[router.md](router.md)** — How manifests are used for request routing
- **[auth-system.md](auth-system.md)** — Credential loading from manifests
- **[mcp-bridge.md](mcp-bridge.md)** — Manifest exposure via MCP protocol

---

*The map is not the territory, but a good manifest comes close.*

🌬️ **Senchán Torpéist**
