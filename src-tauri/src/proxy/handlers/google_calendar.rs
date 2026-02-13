//! Google Calendar Handler
//!
//! Socket handlers for Google Calendar API methods.
//! Retrieves OAuth tokens from auth broker and uses Google API client.

use serde_json::Value;
use tracing::{debug, error, info};

use super::common::*;
use super::super::protocol::JsonRpcResponse;
use crate::google::CalendarApi;

/// Handle Google Calendar-related methods
pub async fn handle(
    action: &str,
    params: &Value,
    id: Value,
) -> JsonRpcResponse {
    let auth_broker = match get_auth_broker().await {
        Ok(broker) => broker,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
    };
    
    // Retrieve OAuth token from auth broker
    let (provider, account) = match extract_oauth_credentials(params, "google") {
        Ok(creds) => creds,
        Err(mut resp) => {
            resp.id = id;
            return resp;
        }
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
            return error(id, code, msg);
        }
    };

    let access_token = match extract_access_token(&token_data, &id) {
        Ok(token) => token,
        Err(response) => return response,
    };

    // Create Calendar API client
    let calendar = match CalendarApi::new(access_token) {
        Ok(api) => api,
        Err(e) => {
            error!("Failed to create Calendar API client: {}", e);
            return generic_error(id, e);
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
        _ => method_not_found(id, &format!("gcalendar.{}", action)),
    }
}

async fn handle_list_calendars(id: Value, calendar: CalendarApi) -> JsonRpcResponse {
    info!("Handling gcalendar.list_calendars");

    match calendar.list_calendars().await {
        Ok(calendars) => {
            debug!("Retrieved {} calendars", calendars.len());
            ok(
                id,
                serde_json::json!({
                    "calendars": calendars,
                    "count": calendars.len(),
                }),
            )
        }
        Err(e) => {
            error!("Failed to list calendars: {}", e);
            generic_error(id, e)
        }
    }
}

async fn handle_list_events(
    params: &Value,
    id: Value,
    calendar: CalendarApi,
) -> JsonRpcResponse {
    info!("Handling gcalendar.list_events");

    let calendar_id = optional_string_or(params, "calendarId", "calendar_id").unwrap_or("primary");
    let time_min = optional_string_or(params, "timeMin", "time_min");
    let time_max = optional_string_or(params, "timeMax", "time_max");
    let max_results = optional_u64_or(params, "maxResults", "max_results").map(|n| n as usize);

    match calendar
        .list_events(calendar_id, time_min, time_max, max_results)
        .await
    {
        Ok(events) => {
            debug!("Retrieved {} events", events.len());
            ok(
                id,
                serde_json::json!({
                    "events": events,
                    "count": events.len(),
                }),
            )
        }
        Err(e) => {
            error!("Failed to list events: {}", e);
            generic_error(id, e)
        }
    }
}

async fn handle_get_event(
    params: &Value,
    id: Value,
    calendar: CalendarApi,
) -> JsonRpcResponse {
    info!("Handling gcalendar.get_event");

    let calendar_id = optional_string_or(params, "calendarId", "calendar_id").unwrap_or("primary");
    
    let event_id = match require_string_or(params, "eventId", "event_id", &id) {
        Ok(id) => id,
        Err(response) => return response,
    };

    match calendar.get_event(calendar_id, event_id).await {
        Ok(event) => ok(id, event),
        Err(e) => {
            error!("Failed to get event: {}", e);
            generic_error(id, e)
        }
    }
}

async fn handle_create_event(
    params: &Value,
    id: Value,
    calendar: CalendarApi,
) -> JsonRpcResponse {
    info!("Handling gcalendar.create_event");

    let calendar_id = optional_string_or(params, "calendarId", "calendar_id").unwrap_or("primary");
    
    let summary = match require_string(params, "summary", &id) {
        Ok(s) => s,
        Err(response) => return response,
    };

    let start = match require_string(params, "start", &id) {
        Ok(s) => s,
        Err(response) => return response,
    };

    let end = match require_string(params, "end", &id) {
        Ok(e) => e,
        Err(response) => return response,
    };

    let location = optional_string(params, "location");
    let description = optional_string(params, "description");

    match calendar
        .create_event(calendar_id, summary, start, end, location, description)
        .await
    {
        Ok(event) => {
            info!("Event created successfully");
            ok(id, event)
        }
        Err(e) => {
            error!("Failed to create event: {}", e);
            generic_error(id, e)
        }
    }
}

async fn handle_update_event(
    params: &Value,
    id: Value,
    calendar: CalendarApi,
) -> JsonRpcResponse {
    info!("Handling gcalendar.update_event");

    let calendar_id = optional_string_or(params, "calendarId", "calendar_id").unwrap_or("primary");
    
    let event_id = match require_string_or(params, "eventId", "event_id", &id) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let summary = optional_string(params, "summary");
    let start = optional_string(params, "start");
    let end = optional_string(params, "end");
    let location = optional_string(params, "location");
    let description = optional_string(params, "description");

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
            ok(id, event)
        }
        Err(e) => {
            error!("Failed to update event: {}", e);
            generic_error(id, e)
        }
    }
}

async fn handle_delete_event(
    params: &Value,
    id: Value,
    calendar: CalendarApi,
) -> JsonRpcResponse {
    info!("Handling gcalendar.delete_event");

    let calendar_id = optional_string_or(params, "calendarId", "calendar_id").unwrap_or("primary");
    
    let event_id = match require_string_or(params, "eventId", "event_id", &id) {
        Ok(id) => id,
        Err(response) => return response,
    };

    match calendar.delete_event(calendar_id, event_id).await {
        Ok(_) => {
            info!("Event deleted successfully");
            ok(id, serde_json::json!({ "deleted": true }))
        }
        Err(e) => {
            error!("Failed to delete event: {}", e);
            generic_error(id, e)
        }
    }
}
