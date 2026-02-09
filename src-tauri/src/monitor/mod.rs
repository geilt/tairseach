//! Activity monitoring
//!
//! Real-time monitoring of MCP tool calls and system events.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEvent {
    pub id: String,
    pub timestamp: String,
    pub event_type: String,
    pub source: String,
    pub message: String,
    pub metadata: Option<serde_json::Value>,
}

#[tauri::command]
pub async fn get_events(limit: Option<usize>) -> Result<Vec<ActivityEvent>, String> {
    let _limit = limit.unwrap_or(100);
    // TODO: Retrieve events from event store
    Ok(vec![])
}
