//! Internal Implementation Dispatcher
//!
//! Routes manifest-registered tools to existing Rust handlers.

use std::collections::HashMap;
use serde_json::Value;
use tracing::debug;

use crate::manifest::types::{Manifest, Tool};
use crate::proxy::handlers;
use crate::proxy::protocol::JsonRpcResponse;

/// Dispatch to internal (existing Rust) handlers
pub async fn dispatch(
    _manifest: &Manifest,
    tool: &Tool,
    params: &Value,
    id: Value,
    module: &str,
    methods: &HashMap<String, String>,
) -> JsonRpcResponse {
    debug!(
        "Dispatching to internal module: {} for tool: {}",
        module, tool.name
    );

    // Get the handler method name for this tool
    let method = match methods.get(&tool.name) {
        Some(m) => m.as_str(),
        None => {
            return JsonRpcResponse::error(
                id,
                -32601,
                format!("No method mapping for tool: {}", tool.name),
                None,
            )
        }
    };

    // Parse the method string (format: "namespace.action")
    let (namespace, action) = if let Some((ns, act)) = method.split_once('.') {
        (ns, act)
    } else {
        return JsonRpcResponse::error(
            id,
            -32601,
            format!("Invalid method format: {}", method),
            None,
        );
    };

    // Route to the appropriate handler module
    match namespace {
        "contacts" => handlers::contacts::handle(action, params, id).await,
        "calendar" => handlers::calendar::handle(action, params, id).await,
        "reminders" => handlers::reminders::handle(action, params, id).await,
        "location" => handlers::location::handle(action, params, id).await,
        "screen" => handlers::screen::handle(action, params, id).await,
        "files" => handlers::files::handle(action, params, id).await,
        "automation" => handlers::automation::handle(action, params, id).await,
        "auth" => handlers::auth::handle(action, params, id).await,
        "permissions" => handlers::permissions::handle(action, params, id).await,
        "config" => handlers::config::handle(action, params, id).await,
        _ => JsonRpcResponse::error(
            id,
            -32601,
            format!("Unknown internal module: {}", namespace),
            None,
        ),
    }
}
