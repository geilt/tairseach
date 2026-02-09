mod initialize;
mod protocol;
mod tools;

use clap::Parser;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use protocol::{error, success, InitializeRequest, JsonRpcRequest, MCP_PROTOCOL_VERSION, ToolsCallRequest};
use tools::{ToolCallError, ToolRegistry};

#[derive(Parser, Debug)]
#[command(name = "tairseach-mcp")]
struct Args {
    #[arg(long, default_value = "stdio")]
    transport: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.transport != "stdio" {
        anyhow::bail!("only stdio transport is supported");
    }

    let registry = ToolRegistry::load()?;

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut stdout = tokio::io::stdout();
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
            Ok(req) => req,
            Err(e) => {
                let resp = error(serde_json::Value::Null, -32700, format!("parse error: {}", e), None);
                stdout.write_all(serde_json::to_string(&resp)?.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;
                continue;
            }
        };

        let Some(id) = request.id.clone() else {
            // JSON-RPC notification: currently only notifications/initialized is expected.
            continue;
        };

        let response = match request.method.as_str() {
            "initialize" => {
                let init: Result<InitializeRequest, _> = serde_json::from_value(request.params);
                match init {
                    Ok(init_req) => {
                        // Accept any client protocol version — respond with our version
                        // per MCP spec: server declares its version, client adapts
                        let result = initialize::handle_initialize(init_req);
                        success(id, serde_json::to_value(result)?)
                    }
                    Err(e) => error(id, -32602, format!("invalid initialize params: {}", e), None),
                }
            }
            "tools/list" => {
                let result = registry.list_response();
                success(id, serde_json::to_value(result)?)
            }
            "tools/call" => {
                let call: Result<ToolsCallRequest, _> = serde_json::from_value(request.params);
                match call {
                    Ok(call_req) => match registry.call_tool(&call_req.name, call_req.arguments).await {
                        Ok(result) => success(id, serde_json::to_value(result)?),
                        Err(ToolCallError::UnknownTool(name)) => {
                            error(id, -32601, format!("unknown tool: {}", name), None)
                        }
                        Err(ToolCallError::Upstream(msg)) => error(id, -32000, msg, None),
                    },
                    Err(e) => error(id, -32602, format!("invalid tools/call params: {}", e), None),
                }
            }
            "notifications/initialized" => success(id, serde_json::json!({})),
            _ => error(id, -32601, format!("method not found: {}", request.method), None),
        };

        stdout.write_all(serde_json::to_string(&response)?.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;
    }

    Ok(())
}
