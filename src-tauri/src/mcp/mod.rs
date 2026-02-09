//! MCP (Model Context Protocol) Server Module
//!
//! Provides an MCP server for OpenClaw agents to interact with macOS system
//! features through Tairseach.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[allow(dead_code)]
pub struct McpStatus {
    pub running: bool,
    pub port: Option<u16>,
    pub clients: usize,
}

// Placeholder - MCP server will be implemented in a future iteration
