//! Google Calendar Handler
//!
//! Socket handlers for Google Calendar API methods.
//! Retrieves OAuth tokens from auth broker and uses Google API client.

use serde_json::Value;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::{debug, error, info};

use super::super::protocol::JsonRpcResponse;
use crate::auth::AuthBroker;
use crate::google::CalendarApi;

/// Global auth broker instance.
static AUTH_BROKER: OnceCell<Arc<AuthBroker>> = OnceCell::const_new();

/// Get or initialise the auth broker.
async fn get_broker() -> Result<&'static Arc<AuthBroker>, JsonRpcResponse> {
    AUTH_BROKER
        .get_or_try_init(|| async {
            match AuthBroker::new().await {
                Ok(broker) => {
                    broker.spawn_refresh_daemon();
                    Ok(broker)
                }
                Err(e) => Err(e),
            }
        })
        .await
        .map_err(|e| {
            error!("Failed to initialise auth broker: {}", e);
            JsonRpcResponse::error(
                Value::Null,
                crate::auth::error_codes::MASTER_KEY_NOT_INITIALIZED,
                format!("Auth broker init failed: {}", e),
                None,
            )
        })
}

/// Handle Google Calendar-related methods
pub async fn handle(
    action: &str,
    params: &Value,
    id: Value,
) -> JsonRpcResponse {
    let auth_broker = match get_broker().await {
        Ok(broker) => broker,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };
    // Retrieve OAuth token from auth broker
    let (provider, account) = match extract_credentials(params) {
        Ok(creds) => creds,
        Err(response) => return response,
    };

    let token_data = match auth_broker
        .get_token(
            &provider,
            &account,
            Some(&["https://www.googleapis.com/auth/calendar".to_string()]),
        )
        .await
    {
        Ok(data) => data,
        Err((code, msg)) => {
            error!("Failed to get OAuth token: {}", msg);
            return JsonRpcResponse::error(id, code, msg, None);
        }
    };

    let access_token = match token_data.get("access_token").and_then(|v| v.as_str()) {
        Some(token) => token.to_string(),
        None => {
            return JsonRpcResponse::error(
                id,
                -32000,
                "Invalid token response: missing access_token".to_string(),
                None,
            );
        }
    };

    // Create Calendar API client
    let calendar = match CalendarApi::new(access_token) {
        Ok(api) => api,
        Err(e) => {
            error!("Failed to create Calendar API client: {}", e);
            return JsonRpcResponse::error(id, -32000, e, None);
        }
    };

    // Dispatch to specific handler
    match action {
        "list_calendars" | "listCalendars" => handle_list_calendars(id, calendar).await,
        "list_events" | "listEvents" => handle_list_events(params, id, calendar).await,
        "get_event" | "getEvent" => handle_get_event(params, id, calendar).await,
        "create_event" | "createEvent" => handle_create_event(params, id, calendar).await,
        "update_event" | "updateEvent" => handle_update_event(params, id, calendar).await,
        "delete_event" | "deleteEvent" => handle_delete_event(params, id, calendar).await,
        _ => JsonRpcResponse::method_not_found(id, &format!("gcalendar.{}", action)),
    }
}

/// Extract credential provider and account from params
fn extract_credentials(params: &Value) -> Result<(String, String), JsonRpcResponse> {
    let provider = params
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("google")
        .to_string();

    let account = params
        .get("account")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            JsonRpcResponse::invalid_params(
                Value::Null,
                "Missing required parameter: account (Google email address)",
            )
        })?
        .to_string();

    Ok((provider, account))
}

async fn handle_list_calendars(id: Value, calendar: CalendarApi) -> JsonRpcResponse {
    info!("Handling gcalendar.list_calendars");

    match calendar.list_calendars().await {
        Ok(calendars) => {
            debug!("Retrieved {} calendars", calendars.len());
            JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "calendars": calendars,
                    "count": calendars.len(),
                }),
            )
        }
        Err(e) => {
            error!("Failed to list calendars: {}", e);
            JsonRpcResponse::error(id, -32000, e, None)
        }
    }
}

async fn handle_list_events(
    params: &Value,
    id: Value,
    calendar: CalendarApi,
) -> JsonRpcResponse {
    info!("Handling gcalendar.list_events");

    let calendar_id = params
        .get("calendarId")
        .or_else(|| params.get("calendar_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("primary");

    let time_min = params
        .get("timeMin")
        .or_else(|| params.get("time_min"))
        .and_then(|v| v.as_str());

    let time_max = params
        .get("timeMax")
        .or_else(|| params.get("time_max"))
        .and_then(|v| v.as_str());

    let max_results = params
        .get("maxResults")
        .or_else(|| params.get("max_results"))
        .and_then(|v| v.as_u64())
        .map(|n| n as usize);

    match calendar
        .list_events(calendar_id, time_min, time_max, max_results)
        .await
    {
        Ok(events) => {
            debug!("Retrieved {} events", events.len());
            JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "events": events,
                    "count": events.len(),
                }),
            )
        }
        Err(e) => {
            error!("Failed to list events: {}", e);
            JsonRpcResponse::error(id, -32000, e, None)
        }
    }
}

async fn handle_get_event(
    params: &Value,
    id: Value,
    calendar: CalendarApi,
) -> JsonRpcResponse {
    info!("Handling gcalendar.get_event");

    let calendar_id = params
        .get("calendarId")
        .or_else(|| params.get("calendar_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("primary");

    let event_id = match params
        .get("eventId")
        .or_else(|| params.get("event_id"))
        .and_then(|v| v.as_str())
    {
        Some(id) => id,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing required parameter: eventId");
        }
    };

    match calendar.get_event(calendar_id, event_id).await {
        Ok(event) => JsonRpcResponse::success(id, event),
        Err(e) => {
            error!("Failed to get event: {}", e);
            JsonRpcResponse::error(id, -32000, e, None)
        }
    }
}

async fn handle_create_event(
    params: &Value,
    id: Value,
    calendar: CalendarApi,
) -> JsonRpcResponse {
    info!("Handling gcalendar.create_event");

    let calendar_id = params
        .get("calendarId")
        .or_else(|| params.get("calendar_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("primary");

    let summary = match params.get("summary").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing required parameter: summary");
        }
    };

    let start = match params.get("start").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing required parameter: start");
        }
    };

    let end = match params.get("end").and_then(|v| v.as_str()) {
        Some(e) => e,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing required parameter: end");
        }
    };

    let location = params.get("location").and_then(|v| v.as_str());
    let description = params.get("description").and_then(|v| v.as_str());

    match calendar
        .create_event(calendar_id, summary, start, end, location, description)
        .await
    {
        Ok(event) => {
            info!("Event created successfully");
            JsonRpcResponse::success(id, event)
        }
        Err(e) => {
            error!("Failed to create event: {}", e);
            JsonRpcResponse::error(id, -32000, e, None)
        }
    }
}

async fn handle_update_event(
    params: &Value,
    id: Value,
    calendar: CalendarApi,
) -> JsonRpcResponse {
    info!("Handling gcalendar.update_event");

    let calendar_id = params
        .get("calendarId")
        .or_else(|| params.get("calendar_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("primary");

    let event_id = match params
        .get("eventId")
        .or_else(|| params.get("event_id"))
        .and_then(|v| v.as_str())
    {
        Some(id) => id,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing required parameter: eventId");
        }
    };

    let summary = params.get("summary").and_then(|v| v.as_str());
    let start = params.get("start").and_then(|v| v.as_str());
    let end = params.get("end").and_then(|v| v.as_str());
    let location = params.get("location").and_then(|v| v.as_str());
    let description = params.get("description").and_then(|v| v.as_str());

    match calendar
        .update_event(
            calendar_id,
            event_id,
            summary,
            start,
            end,
            location,
            description,
        )
        .await
    {
        Ok(event) => {
            info!("Event updated successfully");
            JsonRpcResponse::success(id, event)
        }
        Err(e) => {
            error!("Failed to update event: {}", e);
            JsonRpcResponse::error(id, -32000, e, None)
        }
    }
}

async fn handle_delete_event(
    params: &Value,
    id: Value,
    calendar: CalendarApi,
) -> JsonRpcResponse {
    info!("Handling gcalendar.delete_event");

    let calendar_id = params
        .get("calendarId")
        .or_else(|| params.get("calendar_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("primary");

    let event_id = match params
        .get("eventId")
        .or_else(|| params.get("event_id"))
        .and_then(|v| v.as_str())
    {
        Some(id) => id,
        None => {
            return JsonRpcResponse::invalid_params(id, "Missing required parameter: eventId");
        }
    };

    match calendar.delete_event(calendar_id, event_id).await {
        Ok(_) => {
            info!("Event deleted successfully");
            JsonRpcResponse::success(id, serde_json::json!({ "deleted": true }))
        }
        Err(e) => {
            error!("Failed to delete event: {}", e);
            JsonRpcResponse::error(id, -32000, e, None)
        }
    }
}
