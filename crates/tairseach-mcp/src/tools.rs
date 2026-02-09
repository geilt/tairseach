use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{json, Value};
use tairseach_protocol::{JsonRpcRequest as SocketRequest, SocketClient};

use crate::protocol::{McpTool, ToolAnnotations, ToolContent, ToolsCallResponse, ToolsListResponse};

#[derive(Debug, Clone)]
pub struct ToolIndexEntry {
    pub socket_method: String,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    id: String,
    #[serde(default)]
    tools: Vec<ManifestTool>,
    implementation: ManifestImplementation,
}

#[derive(Debug, Deserialize)]
struct ManifestTool {
    name: String,
    description: String,
    #[serde(rename = "inputSchema")]
    input_schema: Value,
    #[serde(default)]
    mcp_expose: Option<bool>,
    #[serde(default)]
    annotations: Option<ManifestAnnotations>,
}

#[derive(Debug, Deserialize)]
struct ManifestAnnotations {
    #[serde(rename = "readOnlyHint")]
    read_only_hint: Option<bool>,
    #[serde(rename = "destructiveHint")]
    destructive_hint: Option<bool>,
    #[serde(rename = "idempotentHint")]
    idempotent_hint: Option<bool>,
    #[serde(rename = "openWorldHint")]
    open_world_hint: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ManifestImplementation {
    methods: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ToolRegistry {
    tools: Vec<McpTool>,
    allowlist: HashMap<String, ToolIndexEntry>,
}

impl ToolRegistry {
    pub fn load() -> anyhow::Result<Self> {
        let base = manifest_base_dir()?;
        let mut files = Vec::new();
        collect_json_files(&base, &mut files)?;

        let mut tools = Vec::new();
        let mut allowlist = HashMap::new();

        for file in files {
            let content = fs::read_to_string(&file)?;
            let manifest: Manifest = serde_json::from_str(&content)?;

            for t in manifest.tools {
                if t.mcp_expose == Some(false) {
                    continue;
                }

                let Some(socket_method) = manifest.implementation.methods.get(&t.name).cloned() else {
                    continue;
                };

                let mcp_name = format!("tairseach_{}_{}", manifest.id.replace('-', "_"), t.name);
                if allowlist
                    .insert(
                        mcp_name.clone(),
                        ToolIndexEntry {
                            socket_method,
                        },
                    )
                    .is_some()
                {
                    anyhow::bail!("duplicate MCP tool name in allowlist: {}", mcp_name);
                }

                tools.push(McpTool {
                    name: mcp_name,
                    description: t.description,
                    input_schema: t.input_schema,
                    annotations: t.annotations.map(|a| ToolAnnotations {
                        read_only_hint: a.read_only_hint,
                        destructive_hint: a.destructive_hint,
                        idempotent_hint: a.idempotent_hint,
                        open_world_hint: a.open_world_hint,
                    }),
                });
            }
        }

        tools.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(Self { tools, allowlist })
    }

    pub fn list_response(&self) -> ToolsListResponse {
        ToolsListResponse {
            tools: self.tools.clone(),
            next_cursor: None,
        }
    }

    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<ToolsCallResponse, ToolCallError> {
        let Some(entry) = self.allowlist.get(name) else {
            return Err(ToolCallError::UnknownTool(name.to_string()));
        };

        let mut client = SocketClient::connect()
            .await
            .map_err(|e| ToolCallError::Upstream(format!("socket connect failed: {}", e)))?;

        let mut req = SocketRequest::new(entry.socket_method.clone(), arguments);
        req.id = Some(json!(1));
        let resp = client
            .call(req)
            .await
            .map_err(|e| ToolCallError::Upstream(format!("socket call failed: {}", e)))?;

        if let Some(err) = resp.error {
            return Ok(ToolsCallResponse {
                content: vec![ToolContent {
                    content_type: "text".to_string(),
                    text: serde_json::to_string(&json!({"error": err})).unwrap_or_else(|_| "{}".to_string()),
                }],
                is_error: true,
            });
        }

        Ok(ToolsCallResponse {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: serde_json::to_string(&resp.result.unwrap_or(Value::Null))
                    .unwrap_or_else(|_| "null".to_string()),
            }],
            is_error: false,
        })
    }
}

#[derive(Debug)]
pub enum ToolCallError {
    UnknownTool(String),
    Upstream(String),
}

fn manifest_base_dir() -> anyhow::Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("home directory unavailable"))?;
    Ok(home.join(".tairseach").join("manifests"))
}

fn collect_json_files(dir: &Path, out: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(&path, out)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
            out.push(path);
        }
    }

    Ok(())
}
