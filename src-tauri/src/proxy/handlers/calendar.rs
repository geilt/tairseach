//! Calendar Handler
//!
//! Handles calendar-related JSON-RPC methods using EventKit.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{error, info};

use super::common::*;
use super::super::protocol::JsonRpcResponse;

/// Calendar representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calendar {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub calendar_type: String,
    pub color: Option<String>,
    #[serde(rename = "isEditable")]
    pub is_editable: bool,
}

/// Event representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub title: String,
    #[serde(rename = "calendarId")]
    pub calendar_id: String,
    #[serde(rename = "startDate")]
    pub start_date: String,
    #[serde(rename = "endDate")]
    pub end_date: String,
    #[serde(rename = "isAllDay")]
    pub is_all_day: bool,
    pub location: Option<String>,
    pub notes: Option<String>,
}

/// Handle calendar-related methods
pub async fn handle(action: &str, params: &Value, id: Value) -> JsonRpcResponse {
    match action {
        "list" => handle_list_calendars(params, id).await,
        "events" => handle_list_events(params, id).await,
        "getEvent" => handle_get_event(params, id).await,
        "createEvent" => handle_create_event(params, id).await,
        "updateEvent" => handle_update_event(params, id).await,
        "deleteEvent" => handle_delete_event(params, id).await,
        _ => JsonRpcResponse::method_not_found(id, &format!("calendar.{}", action)),
    }
}

/// List all calendars
async fn handle_list_calendars(_params: &Value, id: Value) -> JsonRpcResponse {
    let calendars = fetch_calendars().await;
    
    ok(
        id,
        serde_json::json!({
            "calendars": calendars,
            "count": calendars.len(),
        }),
    )
}

/// List events in a date range
async fn handle_list_events(params: &Value, id: Value) -> JsonRpcResponse {
    let start = match require_string(params, "start", &id) {
        Ok(s) => s,
        Err(response) => return response,
    };
    let end = match require_string(params, "end", &id) {
        Ok(e) => e,
        Err(response) => return response,
    };
    let calendar_id = optional_string(params, "calendarId");
    
    let events = fetch_events(start, end, calendar_id).await;
    
    ok(
        id,
        serde_json::json!({
            "events": events,
            "count": events.len(),
            "start": start,
            "end": end,
        }),
    )
}

/// Get a specific event by ID
async fn handle_get_event(params: &Value, id: Value) -> JsonRpcResponse {
    let event_id = match require_string(params, "id", &id) {
        Ok(e) => e,
        Err(response) => return response,
    };
    
    match fetch_event_by_id(event_id).await {
        Some(event) => ok(id, serde_json::to_value(event).unwrap_or_default()),
        None => error(id, -32002, format!("Event not found: {}", event_id)),
    }
}

/// Create a new event
async fn handle_create_event(params: &Value, id: Value) -> JsonRpcResponse {
    let title = match require_string(params, "title", &id) {
        Ok(t) => t,
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
    let calendar_id = optional_string(params, "calendarId");
    let location = optional_string(params, "location");
    let notes = optional_string(params, "notes");
    let is_all_day = bool_with_default(params, "isAllDay", false);
    
    match create_event(title, start, end, calendar_id, location, notes, is_all_day).await {
        Some(event) => ok(
            id,
            serde_json::json!({
                "created": true,
                "event": event,
            }),
        ),
        None => error(id, -32003, "Failed to create event"),
    }
}

/// Update an existing event
async fn handle_update_event(params: &Value, id: Value) -> JsonRpcResponse {
    let event_id = match require_string(params, "id", &id) {
        Ok(e) => e,
        Err(response) => return response,
    };
    
    // For now, just return success - actual implementation would update the event
    ok(
        id,
        serde_json::json!({
            "updated": true,
            "id": event_id,
            "message": "Event update not yet implemented",
        }),
    )
}

/// Delete an event
async fn handle_delete_event(params: &Value, id: Value) -> JsonRpcResponse {
    let event_id = match require_string(params, "id", &id) {
        Ok(e) => e,
        Err(response) => return response,
    };
    
    // For now, just return success - actual implementation would delete the event
    ok(
        id,
        serde_json::json!({
            "deleted": true,
            "id": event_id,
            "message": "Event deletion not yet implemented",
        }),
    )
}

// ============================================================================
// Native EventKit integration via JXA
// ============================================================================

/// Fetch all calendars using JXA
#[cfg(target_os = "macos")]
async fn fetch_calendars() -> Vec<Calendar> {
    use std::process::Command;
    
    info!("Fetching calendars via JXA");
    
    let script = r#"
        ObjC.import('EventKit');
        
        var store = $.EKEventStore.alloc.init;
        var calendars = store.calendarsForEntityType($.EKEntityTypeEvent);
        
        var result = [];
        for (var i = 0; i < calendars.count; i++) {
            var cal = calendars.objectAtIndex(i);
            result.push({
                id: ObjC.unwrap(cal.calendarIdentifier),
                title: ObjC.unwrap(cal.title),
                type: cal.type === 0 ? 'local' : cal.type === 1 ? 'caldav' : cal.type === 2 ? 'exchange' : 'other',
                color: null, // Color requires CGColor conversion
                isEditable: cal.allowsContentModifications
            });
        }
        
        JSON.stringify(result);
    "#;
    
    let output = Command::new("osascript")
        .arg("-l")
        .arg("JavaScript")
        .arg("-e")
        .arg(script)
        .output();
    
    match output {
        Ok(out) => {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                serde_json::from_str::<Vec<Calendar>>(stdout.trim()).unwrap_or_default()
            } else {
                error!("JXA calendar list failed: {}", String::from_utf8_lossy(&out.stderr));
                Vec::new()
            }
        }
        Err(e) => {
            error!("Failed to execute osascript: {}", e);
            Vec::new()
        }
    }
}

/// Fetch events in a date range using JXA
#[cfg(target_os = "macos")]
async fn fetch_events(start: &str, end: &str, calendar_id: Option<&str>) -> Vec<Event> {
    use std::process::Command;
    
    info!("Fetching events via JXA: {} to {}", start, end);
    
    let script = format!(
        r#"
        ObjC.import('EventKit');
        ObjC.import('Foundation');
        
        var store = $.EKEventStore.alloc.init;
        
        // Parse dates
        var formatter = $.NSDateFormatter.alloc.init;
        formatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ss";
        formatter.timeZone = $.NSTimeZone.localTimeZone;
        
        var startStr = '{start}';
        var endStr = '{end}';
        
        // Handle both ISO 8601 and simple date formats
        if (startStr.length === 10) startStr += 'T00:00:00';
        if (endStr.length === 10) endStr += 'T23:59:59';
        
        var startDate = formatter.dateFromString(startStr.substring(0, 19));
        var endDate = formatter.dateFromString(endStr.substring(0, 19));
        
        if (!startDate || !endDate) {{
            JSON.stringify([]);
        }} else {{
            var calendars = {calendar_filter};
            var predicate = store.predicateForEventsWithStartDateEndDateCalendars(startDate, endDate, calendars);
            var events = store.eventsMatchingPredicate(predicate);
            
            var result = [];
            if (events) {{
                for (var i = 0; i < events.count; i++) {{
                    var ev = events.objectAtIndex(i);
                    
                    var isoFormatter = $.NSDateFormatter.alloc.init;
                    isoFormatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ssZZZZZ";
                    
                    result.push({{
                        id: ObjC.unwrap(ev.eventIdentifier),
                        title: ObjC.unwrap(ev.title) || '',
                        calendarId: ObjC.unwrap(ev.calendar.calendarIdentifier),
                        startDate: ObjC.unwrap(isoFormatter.stringFromDate(ev.startDate)),
                        endDate: ObjC.unwrap(isoFormatter.stringFromDate(ev.endDate)),
                        isAllDay: ev.allDay,
                        location: ObjC.unwrap(ev.location) || null,
                        notes: ObjC.unwrap(ev.notes) || null
                    }});
                }}
            }}
            
            JSON.stringify(result);
        }}
        "#,
        start = start,
        end = end,
        calendar_filter = calendar_id
            .map(|id| format!(
                "$.NSArray.arrayWithObject(store.calendarWithIdentifier($.NSString.stringWithString('{}')))",
                id.replace('\'', "\\'")
            ))
            .unwrap_or_else(|| "null".to_string())
    );
    
    let output = Command::new("osascript")
        .arg("-l")
        .arg("JavaScript")
        .arg("-e")
        .arg(&script)
        .output();
    
    match output {
        Ok(out) => {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                match serde_json::from_str::<Vec<Event>>(stdout.trim()) {
                    Ok(events) => {
                        info!("Fetched {} events", events.len());
                        events
                    }
                    Err(e) => {
                        error!("Failed to parse events JSON: {} — raw: {}", e, &stdout.trim()[..stdout.len().min(200)]);
                        Vec::new()
                    }
                }
            } else {
                error!("JXA events fetch failed: {}", String::from_utf8_lossy(&out.stderr));
                Vec::new()
            }
        }
        Err(e) => {
            error!("Failed to execute osascript: {}", e);
            Vec::new()
        }
    }
}

/// Fetch a single event by ID
#[cfg(target_os = "macos")]
async fn fetch_event_by_id(event_id: &str) -> Option<Event> {
    use std::process::Command;
    
    info!("Fetching event by ID via JXA: {}", event_id);
    
    let script = format!(
        r#"
        ObjC.import('EventKit');
        
        var store = $.EKEventStore.alloc.init;
        var ev = store.eventWithIdentifier($.NSString.stringWithString('{event_id}'));
        
        if (ev) {{
            var isoFormatter = $.NSDateFormatter.alloc.init;
            isoFormatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ssZZZZZ";
            
            JSON.stringify({{
                id: ObjC.unwrap(ev.eventIdentifier),
                title: ObjC.unwrap(ev.title) || '',
                calendarId: ObjC.unwrap(ev.calendar.calendarIdentifier),
                startDate: ObjC.unwrap(isoFormatter.stringFromDate(ev.startDate)),
                endDate: ObjC.unwrap(isoFormatter.stringFromDate(ev.endDate)),
                isAllDay: ev.allDay,
                location: ObjC.unwrap(ev.location) || null,
                notes: ObjC.unwrap(ev.notes) || null
            }});
        }} else {{
            'null';
        }}
        "#,
        event_id = event_id.replace('\'', "\\'")
    );
    
    let output = Command::new("osascript")
        .arg("-l")
        .arg("JavaScript")
        .arg("-e")
        .arg(&script)
        .output();
    
    match output {
        Ok(out) => {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if stdout == "null" {
                    None
                } else {
                    serde_json::from_str::<Event>(&stdout).ok()
                }
            } else {
                error!("JXA event fetch failed: {}", String::from_utf8_lossy(&out.stderr));
                None
            }
        }
        Err(e) => {
            error!("Failed to execute osascript: {}", e);
            None
        }
    }
}

/// Create a new event using JXA
#[cfg(target_os = "macos")]
async fn create_event(
    title: &str,
    start: &str,
    end: &str,
    calendar_id: Option<&str>,
    location: Option<&str>,
    notes: Option<&str>,
    is_all_day: bool,
) -> Option<Event> {
    use std::process::Command;
    
    info!("Creating event via JXA: {}", title);
    
    let script = format!(
        r#"
        ObjC.import('EventKit');
        ObjC.import('Foundation');
        
        var store = $.EKEventStore.alloc.init;
        
        // Get calendar
        var calendar = {calendar_expr};
        if (!calendar) {{
            calendar = store.defaultCalendarForNewEvents;
        }}
        
        // Parse dates
        var formatter = $.NSDateFormatter.alloc.init;
        formatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ss";
        formatter.timeZone = $.NSTimeZone.localTimeZone;
        
        var startStr = '{start}';
        var endStr = '{end}';
        if (startStr.length === 10) startStr += 'T00:00:00';
        if (endStr.length === 10) endStr += 'T23:59:59';
        
        var startDate = formatter.dateFromString(startStr.substring(0, 19));
        var endDate = formatter.dateFromString(endStr.substring(0, 19));
        
        if (!startDate || !endDate || !calendar) {{
            'null';
        }} else {{
            var event = $.EKEvent.eventWithEventStore(store);
            event.title = '{title}';
            event.startDate = startDate;
            event.endDate = endDate;
            event.calendar = calendar;
            event.allDay = {is_all_day};
            {location_line}
            {notes_line}
            
            var error = $();
            var success = store.saveEventSpanError(event, $.EKSpanThisEvent, error);
            
            if (success) {{
                var isoFormatter = $.NSDateFormatter.alloc.init;
                isoFormatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ssZZZZZ";
                
                JSON.stringify({{
                    id: ObjC.unwrap(event.eventIdentifier),
                    title: ObjC.unwrap(event.title),
                    calendarId: ObjC.unwrap(event.calendar.calendarIdentifier),
                    startDate: ObjC.unwrap(isoFormatter.stringFromDate(event.startDate)),
                    endDate: ObjC.unwrap(isoFormatter.stringFromDate(event.endDate)),
                    isAllDay: event.allDay,
                    location: ObjC.unwrap(event.location) || null,
                    notes: ObjC.unwrap(event.notes) || null
                }});
            }} else {{
                'null';
            }}
        }}
        "#,
        calendar_expr = calendar_id
            .map(|id| format!("store.calendarWithIdentifier($.NSString.stringWithString('{}'))", id.replace('\'', "\\'")))
            .unwrap_or_else(|| "null".to_string()),
        start = start,
        end = end,
        title = title.replace('\'', "\\'"),
        is_all_day = if is_all_day { "true" } else { "false" },
        location_line = location
            .map(|l| format!("event.location = '{}';", l.replace('\'', "\\'")))
            .unwrap_or_default(),
        notes_line = notes
            .map(|n| format!("event.notes = '{}';", n.replace('\'', "\\'")))
            .unwrap_or_default(),
    );
    
    let output = Command::new("osascript")
        .arg("-l")
        .arg("JavaScript")
        .arg("-e")
        .arg(&script)
        .output();
    
    match output {
        Ok(out) => {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if stdout == "null" {
                    error!("JXA returned null for event creation");
                    None
                } else {
                    match serde_json::from_str::<Event>(&stdout) {
                        Ok(event) => {
                            info!("Created event: {}", event.id);
                            Some(event)
                        }
                        Err(e) => {
                            error!("Failed to parse created event: {}", e);
                            None
                        }
                    }
                }
            } else {
                error!("JXA event create failed: {}", String::from_utf8_lossy(&out.stderr));
                None
            }
        }
        Err(e) => {
            error!("Failed to execute osascript: {}", e);
            None
        }
    }
}

// Non-macOS stubs
#[cfg(not(target_os = "macos"))]
async fn fetch_calendars() -> Vec<Calendar> { Vec::new() }

#[cfg(not(target_os = "macos"))]
async fn fetch_events(_start: &str, _end: &str, _calendar_id: Option<&str>) -> Vec<Event> { Vec::new() }

#[cfg(not(target_os = "macos"))]
async fn fetch_event_by_id(_event_id: &str) -> Option<Event> { None }

#[cfg(not(target_os = "macos"))]
async fn create_event(
    _title: &str,
    _start: &str,
    _end: &str,
    _calendar_id: Option<&str>,
    _location: Option<&str>,
    _notes: Option<&str>,
    _is_all_day: bool,
) -> Option<Event> { None }
