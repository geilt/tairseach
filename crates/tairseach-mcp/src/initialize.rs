use crate::protocol::{
    InitializeRequest, InitializeResponse, MCP_PROTOCOL_VERSION, ResourcesCapabilities, ServerCapabilities,
    ServerInfo, ToolsCapabilities,
};

pub fn handle_initialize(_request: InitializeRequest) -> InitializeResponse {
    InitializeResponse {
        protocol_version: MCP_PROTOCOL_VERSION.to_string(),
        capabilities: ServerCapabilities {
            tools: ToolsCapabilities { list_changed: true },
            resources: Some(ResourcesCapabilities {
                subscribe: false,
                list_changed: false,
            }),
        },
        server_info: ServerInfo {
            name: "tairseach-mcp".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        instructions: "Tairseach provides local macOS capability tools. Check permissions first when needed, and confirm destructive actions before calling mutating tools.".to_string(),
    }
}
