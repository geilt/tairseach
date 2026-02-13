# 1Password Module

> **Location:** `src-tauri/src/proxy/handlers/onepassword.rs` + `op-helper/` (Go binary)  
> **Lines:** ~520 (Rust handler) + ~400 (Go helper)  
> **Purpose:** 1Password CLI integration via FFI helper

---

## Overview

Tairseach integrates with 1Password CLI via a bundled Go helper binary (`op-helper`) that wraps the `op` CLI. The handler communicates with this helper via stdin/stdout JSON.

**Architecture:**
- **Go Helper:** `op-helper/main.go` — wraps `op` CLI, handles auth, returns JSON
- **Rust Handler:** `proxy/handlers/onepassword.rs` — spawns helper, parses JSON
- **Bundling:** Helper is built and bundled into Tauri app bundle

---

## Key Operations

```rust
// In onepassword.rs handler

pub async fn handle(action: &str, params: &Value, id: Value) -> JsonRpcResponse {
    match action {
        "list_vaults" => handle_list_vaults(id).await,
        "list_items" => handle_list_items(params, id).await,
        "get_item" => handle_get_item(params, id).await,
        "create_item" => handle_create_item(params, id).await,
        "set_default_vault" => handle_set_default_vault(params, id).await,
        _ => method_not_found(id, &format!("onepassword.{}", action)),
    }
}
```

---

## Go Helper Protocol

**Execution:**
```rust
let helper_path = get_op_helper_path()?;
let mut cmd = tokio::process::Command::new(&helper_path);
cmd.arg(action)
   .stdin(Stdio::piped())
   .stdout(Stdio::piped())
   .stderr(Stdio::piped());

let mut child = cmd.spawn()?;

// Write params to stdin
if let Some(mut stdin) = child.stdin.take() {
    stdin.write_all(serde_json::to_string(&params)?.as_bytes()).await?;
}

// Read stdout
let output = child.wait_with_output().await?;
let result: Value = serde_json::from_slice(&output.stdout)?;
```

**Helper Commands:**
- `list-vaults` — Returns all 1Password vaults
- `list-items` — Returns items in a vault
- `get-item` — Returns specific item details
- `create-item` — Creates a new item

---

## Bundling

**Build script:** `build.rs` (in `src-tauri/`)
```rust
// Build op-helper
let status = Command::new("go")
    .arg("build")
    .arg("-o")
    .arg("../src-tauri/binaries/op-helper")
    .current_dir("op-helper")
    .status()?;

// Tauri bundles binaries/ into app bundle
```

**Runtime path resolution:**
```rust
fn get_op_helper_path() -> Result<PathBuf, String> {
    #[cfg(debug_assertions)]
    let path = PathBuf::from("binaries/op-helper");
    
    #[cfg(not(debug_assertions))]
    let path = tauri::api::path::resource_dir()
        .ok_or("Could not find resource dir")?
        .join("binaries/op-helper");
    
    Ok(path)
}
```

---

## Go Helper Code Structure

```go
// op-helper/main.go

func main() {
    if len(os.Args) < 2 {
        log.Fatal("Usage: op-helper <action>")
    }
    
    action := os.Args[1]
    
    // Read params from stdin
    var params map[string]interface{}
    json.NewDecoder(os.Stdin).Decode(&params)
    
    // Execute action
    var result interface{}
    switch action {
    case "list-vaults":
        result = listVaults()
    case "list-items":
        result = listItems(params["vault"].(string))
    // ...
    }
    
    // Write result to stdout
    json.NewEncoder(os.Stdout).Encode(result)
}

func listVaults() []Vault {
    output := exec.Command("op", "vault", "list", "--format=json").Output()
    var vaults []Vault
    json.Unmarshal(output, &vaults)
    return vaults
}
```

---

## Tauri Commands

```rust
#[tauri::command]
async fn op_vaults_list() -> Result<Vec<Value>, String>

#[tauri::command]
async fn op_config_set_default_vault(vault: String) -> Result<(), String>
```

---

*For handler integration, see [handlers.md](handlers.md)*
