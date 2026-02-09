use serde_json::json;
use tairseach_protocol::{JsonRpcRequest, SocketClient};

use crate::protocol::{JsonRpcEnvelope, JsonRpcErrorEnvelope, JsonRpcResponseEnvelope};

pub async fn handle_request(req: JsonRpcEnvelope) -> JsonRpcResponseEnvelope {
    let id = req.id.unwrap_or(serde_json::Value::Null);

    match req.method.as_str() {
        "initialize" => JsonRpcResponseEnvelope {
            jsonrpc: "2.0".into(),
            id,
            result: Some(json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {"tools": {"listChanged": true}},
                "serverInfo": {"name": "tairseach-mcp", "version": env!("CARGO_PKG_VERSION")}
            })),
            error: None,
        },
        "notifications/initialized" => JsonRpcResponseEnvelope { jsonrpc: "2.0".into(), id, result: Some(json!({})), error: None },
        "tools/list" => JsonRpcResponseEnvelope {
            jsonrpc: "2.0".into(),
            id,
            result: Some(json!({"tools": []})),
            error: None,
        },
        "tools/call" => {
            let name = req.params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = req.params.get("arguments").cloned().unwrap_or_else(|| json!({}));
            let method = name.strip_prefix("tairseach_").unwrap_or(name).replace('_', ".");

            match SocketClient::connect().await {
                Ok(mut client) => {
                    let socket_req = JsonRpcRequest::new(method, args);
                    match client.call(socket_req).await {
                        Ok(resp) => JsonRpcResponseEnvelope {
                            jsonrpc: "2.0".into(),
                            id,
                            result: Some(json!({"content": [{"type": "text", "text": serde_json::to_string(&resp.result).unwrap_or_else(|_| "null".into())}], "isError": resp.error.is_some()})),
                            error: None,
                        },
                        Err(e) => err(id, -32000, format!("socket call failed: {}", e)),
                    }
                }
                Err(e) => err(id, -32000, format!("socket connect failed: {}", e)),
            }
        }
        _ => err(id, -32601, format!("method not found: {}", req.method)),
    }
}

fn err(id: serde_json::Value, code: i32, message: String) -> JsonRpcResponseEnvelope {
    JsonRpcResponseEnvelope {
        jsonrpc: "2.0".into(),
        id,
        result: None,
        error: Some(JsonRpcErrorEnvelope { code, message, data: None }),
    }
}
