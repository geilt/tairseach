//! Reminders Handler
//!
//! Handles reminder-related JSON-RPC methods using EventKit.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{error, info};

use super::common::*;
use super::super::protocol::JsonRpcResponse;

/// Reminder list representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderList {
    pub id: String,
    pub title: String,
    pub color: Option<String>,
}

/// Reminder representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reminder {
    pub id: String,
    pub title: String,
    #[serde(rename = "listId")]
    pub list_id: String,
    #[serde(rename = "isCompleted")]
    pub is_completed: bool,
    pub priority: i32,
    #[serde(rename = "dueDate")]
    pub due_date: Option<String>,
    pub notes: Option<String>,
}

/// Handle reminder-related methods
pub async fn handle(action: &str, params: &Value, id: Value) -> JsonRpcResponse {
    match action {
        "lists" => handle_list_reminder_lists(params, id).await,
        "list" => handle_list_reminders(params, id).await,
        "create" => handle_create_reminder(params, id).await,
        "complete" => handle_complete_reminder(params, id).await,
        "uncomplete" => handle_uncomplete_reminder(params, id).await,
        "delete" => handle_delete_reminder(params, id).await,
        _ => method_not_found(id, &format!("reminders.{}", action)),
    }
}

/// List all reminder lists
async fn handle_list_reminder_lists(_params: &Value, id: Value) -> JsonRpcResponse {
    let lists = fetch_reminder_lists().await;
    
    ok(
        id,
        serde_json::json!({
            "lists": lists,
            "count": lists.len(),
        }),
    )
}

/// List reminders in a specific list
async fn handle_list_reminders(params: &Value, id: Value) -> JsonRpcResponse {
    let list_id = optional_string(params, "listId");
    let include_completed = bool_with_default(params, "includeCompleted", false);
    
    let reminders = fetch_reminders(list_id, include_completed).await;
    
    ok(
        id,
        serde_json::json!({
            "reminders": reminders,
            "count": reminders.len(),
            "includeCompleted": include_completed,
        }),
    )
}

/// Create a new reminder
async fn handle_create_reminder(params: &Value, id: Value) -> JsonRpcResponse {
    let title = match require_string(params, "title", &id) {
        Ok(t) => t,
        Err(response) => return response,
    };
    
    let list_id = optional_string(params, "listId");
    let due_date = optional_string(params, "dueDate");
    let notes = optional_string(params, "notes");
    let priority = params.get("priority").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    
    match create_reminder(title, list_id, due_date, notes, priority).await {
        Some(reminder) => ok(
            id,
            serde_json::json!({
                "created": true,
                "reminder": reminder,
            }),
        ),
        None => error(id, -32003, "Failed to create reminder"),
    }
}

/// Mark a reminder as complete
async fn handle_complete_reminder(params: &Value, id: Value) -> JsonRpcResponse {
    let reminder_id = match require_string(params, "id", &id) {
        Ok(rid) => rid,
        Err(response) => return response,
    };
    
    if set_reminder_completed(reminder_id, true).await {
        ok(
            id,
            serde_json::json!({
                "completed": true,
                "id": reminder_id,
            }),
        )
    } else {
        error(id, -32003, "Failed to complete reminder")
    }
}

/// Mark a reminder as incomplete
async fn handle_uncomplete_reminder(params: &Value, id: Value) -> JsonRpcResponse {
    let reminder_id = match require_string(params, "id", &id) {
        Ok(rid) => rid,
        Err(response) => return response,
    };
    
    if set_reminder_completed(reminder_id, false).await {
        ok(
            id,
            serde_json::json!({
                "uncompleted": true,
                "id": reminder_id,
            }),
        )
    } else {
        error(id, -32003, "Failed to uncomplete reminder")
    }
}

/// Delete a reminder
async fn handle_delete_reminder(params: &Value, id: Value) -> JsonRpcResponse {
    let reminder_id = match require_string(params, "id", &id) {
        Ok(rid) => rid,
        Err(response) => return response,
    };
    
    if delete_reminder(reminder_id).await {
        ok(
            id,
            serde_json::json!({
                "deleted": true,
                "id": reminder_id,
            }),
        )
    } else {
        error(id, -32003, "Failed to delete reminder")
    }
}

// ============================================================================
// Native EventKit integration via JXA
// ============================================================================

/// Fetch all reminder lists using JXA
#[cfg(target_os = "macos")]
async fn fetch_reminder_lists() -> Vec<ReminderList> {
    use std::process::Command;
    use tokio::time::{timeout, Duration};
    
    info!("Fetching reminder lists via JXA");
    
    let script = r#"
        ObjC.import('EventKit');
        
        var store = $.EKEventStore.alloc.init;
        var calendars = store.calendarsForEntityType($.EKEntityTypeReminder);
        
        var result = [];
        for (var i = 0; i < calendars.count; i++) {
            var cal = calendars.objectAtIndex(i);
            result.push({
                id: ObjC.unwrap(cal.calendarIdentifier),
                title: ObjC.unwrap(cal.title),
                color: null
            });
        }
        
        JSON.stringify(result);
    "#;
    
    let output_future = tokio::task::spawn_blocking(move || {
        Command::new("osascript")
            .arg("-l")
            .arg("JavaScript")
            .arg("-e")
            .arg(script)
            .output()
    });
    
    let output = match timeout(Duration::from_secs(10), output_future).await {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => {
            error!("Failed to execute osascript: {}", e);
            return Vec::new();
        }
        Err(_) => {
            error!("JXA reminder lists fetch timed out after 10 seconds");
            return Vec::new();
        }
    };
    
    match output {
        Ok(out) => {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                serde_json::from_str::<Vec<ReminderList>>(stdout.trim()).unwrap_or_default()
            } else {
                error!("JXA reminder lists failed: {}", String::from_utf8_lossy(&out.stderr));
                Vec::new()
            }
        }
        Err(e) => {
            error!("Failed to execute osascript: {}", e);
            Vec::new()
        }
    }
}

/// Fetch reminders from a list (or all lists if None)
#[cfg(target_os = "macos")]
async fn fetch_reminders(list_id: Option<&str>, include_completed: bool) -> Vec<Reminder> {
    use std::process::Command;
    use tokio::time::{timeout, Duration};
    
    info!("Fetching reminders via JXA: list_id={:?}, include_completed={}", list_id, include_completed);
    
    let script = format!(
        r#"
        ObjC.import('EventKit');
        ObjC.import('Foundation');
        
        var store = $.EKEventStore.alloc.init;
        
        // Get calendars (reminder lists)
        var calendars = {calendar_filter};
        if (!calendars) {{
            calendars = store.calendarsForEntityType($.EKEntityTypeReminder);
        }}
        
        // Create predicate for incomplete reminders
        var predicate = store.predicateForRemindersInCalendars(calendars);
        
        // Fetch reminders synchronously (JXA doesn't handle callbacks well)
        var semaphore = $.NSCondition.alloc.init;
        var reminders = null;
        
        store.fetchRemindersMatchingPredicateCompletion(predicate, function(fetchedReminders) {{
            reminders = fetchedReminders;
            semaphore.lock;
            semaphore.signal;
            semaphore.unlock;
        }});
        
        semaphore.lock;
        while (reminders === null) {{
            semaphore.waitUntilDate($.NSDate.dateWithTimeIntervalSinceNow(0.1));
        }}
        semaphore.unlock;
        
        var result = [];
        if (reminders) {{
            var isoFormatter = $.NSDateFormatter.alloc.init;
            isoFormatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ssZZZZZ";
            
            for (var i = 0; i < reminders.count; i++) {{
                var rem = reminders.objectAtIndex(i);
                
                // Skip completed if not requested
                if (!{include_completed} && rem.completed) continue;
                
                var dueDate = null;
                if (rem.dueDateComponents) {{
                    var calendar = $.NSCalendar.currentCalendar;
                    var date = calendar.dateFromComponents(rem.dueDateComponents);
                    if (date) {{
                        dueDate = ObjC.unwrap(isoFormatter.stringFromDate(date));
                    }}
                }}
                
                result.push({{
                    id: ObjC.unwrap(rem.calendarItemIdentifier),
                    title: ObjC.unwrap(rem.title) || '',
                    listId: ObjC.unwrap(rem.calendar.calendarIdentifier),
                    isCompleted: rem.completed,
                    priority: rem.priority,
                    dueDate: dueDate,
                    notes: ObjC.unwrap(rem.notes) || null
                }});
            }}
        }}
        
        JSON.stringify(result);
        "#,
        calendar_filter = list_id
            .map(|id| format!(
                "$.NSArray.arrayWithObject(store.calendarWithIdentifier($.NSString.stringWithString('{}')))",
                id.replace('\'', "\\'")
            ))
            .unwrap_or_else(|| "null".to_string()),
        include_completed = if include_completed { "true" } else { "false" }
    );
    
    let output_future = tokio::task::spawn_blocking(move || {
        Command::new("osascript")
            .arg("-l")
            .arg("JavaScript")
            .arg("-e")
            .arg(&script)
            .output()
    });
    
    let output = match timeout(Duration::from_secs(15), output_future).await {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => {
            error!("Failed to execute osascript: {}", e);
            return Vec::new();
        }
        Err(_) => {
            error!("JXA reminders fetch timed out after 15 seconds");
            return Vec::new();
        }
    };
    
    match output {
        Ok(out) => {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                match serde_json::from_str::<Vec<Reminder>>(stdout.trim()) {
                    Ok(reminders) => {
                        info!("Fetched {} reminders", reminders.len());
                        reminders
                    }
                    Err(e) => {
                        error!("Failed to parse reminders JSON: {} — raw: {}", e, &stdout.trim()[..stdout.len().min(200)]);
                        Vec::new()
                    }
                }
            } else {
                error!("JXA reminders fetch failed: {}", String::from_utf8_lossy(&out.stderr));
                Vec::new()
            }
        }
        Err(e) => {
            error!("Failed to execute osascript: {}", e);
            Vec::new()
        }
    }
}

/// Create a new reminder
#[cfg(target_os = "macos")]
async fn create_reminder(
    title: &str,
    list_id: Option<&str>,
    due_date: Option<&str>,
    notes: Option<&str>,
    priority: i32,
) -> Option<Reminder> {
    use std::process::Command;
    use tokio::time::{timeout, Duration};
    
    info!("Creating reminder via JXA: {}", title);
    
    let script = format!(
        r#"
        ObjC.import('EventKit');
        ObjC.import('Foundation');
        
        var store = $.EKEventStore.alloc.init;
        
        // Get calendar
        var calendar = {calendar_expr};
        if (!calendar) {{
            calendar = store.defaultCalendarForNewReminders;
        }}
        
        if (!calendar) {{
            'null';
        }} else {{
            var reminder = $.EKReminder.reminderWithEventStore(store);
            reminder.title = '{title}';
            reminder.calendar = calendar;
            reminder.priority = {priority};
            {notes_line}
            {due_date_line}
            
            var error = $();
            var success = store.saveReminderCommitError(reminder, true, error);
            
            if (success) {{
                var isoFormatter = $.NSDateFormatter.alloc.init;
                isoFormatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ssZZZZZ";
                
                var dueDate = null;
                if (reminder.dueDateComponents) {{
                    var cal = $.NSCalendar.currentCalendar;
                    var date = cal.dateFromComponents(reminder.dueDateComponents);
                    if (date) {{
                        dueDate = ObjC.unwrap(isoFormatter.stringFromDate(date));
                    }}
                }}
                
                JSON.stringify({{
                    id: ObjC.unwrap(reminder.calendarItemIdentifier),
                    title: ObjC.unwrap(reminder.title),
                    listId: ObjC.unwrap(reminder.calendar.calendarIdentifier),
                    isCompleted: reminder.completed,
                    priority: reminder.priority,
                    dueDate: dueDate,
                    notes: ObjC.unwrap(reminder.notes) || null
                }});
            }} else {{
                'null';
            }}
        }}
        "#,
        calendar_expr = list_id
            .map(|id| format!("store.calendarWithIdentifier($.NSString.stringWithString('{}'))", id.replace('\'', "\\'")))
            .unwrap_or_else(|| "null".to_string()),
        title = title.replace('\'', "\\'"),
        priority = priority,
        notes_line = notes
            .map(|n| format!("reminder.notes = '{}';", n.replace('\'', "\\'")))
            .unwrap_or_default(),
        due_date_line = due_date
            .map(|d| format!(
                r#"
                var dueDateStr = '{}';
                var formatter = $.NSDateFormatter.alloc.init;
                formatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ss";
                formatter.timeZone = $.NSTimeZone.localTimeZone;
                if (dueDateStr.length === 10) dueDateStr += 'T09:00:00';
                var parsedDate = formatter.dateFromString(dueDateStr.substring(0, 19));
                if (parsedDate) {{
                    var cal = $.NSCalendar.currentCalendar;
                    var components = cal.componentsFromDate(
                        $.NSCalendarUnitYear | $.NSCalendarUnitMonth | $.NSCalendarUnitDay | $.NSCalendarUnitHour | $.NSCalendarUnitMinute,
                        parsedDate
                    );
                    reminder.dueDateComponents = components;
                }}
                "#,
                d.replace('\'', "\\'")
            ))
            .unwrap_or_default(),
    );
    
    let output_future = tokio::task::spawn_blocking(move || {
        Command::new("osascript")
            .arg("-l")
            .arg("JavaScript")
            .arg("-e")
            .arg(&script)
            .output()
    });
    
    let output = match timeout(Duration::from_secs(10), output_future).await {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => {
            error!("Failed to execute osascript: {}", e);
            return None;
        }
        Err(_) => {
            error!("JXA reminder create timed out after 10 seconds");
            return None;
        }
    };
    
    match output {
        Ok(out) => {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if stdout == "null" {
                    error!("JXA returned null for reminder creation");
                    None
                } else {
                    serde_json::from_str::<Reminder>(&stdout).ok()
                }
            } else {
                error!("JXA reminder create failed: {}", String::from_utf8_lossy(&out.stderr));
                None
            }
        }
        Err(e) => {
            error!("Failed to execute osascript: {}", e);
            None
        }
    }
}

/// Set reminder completion status
#[cfg(target_os = "macos")]
async fn set_reminder_completed(reminder_id: &str, completed: bool) -> bool {
    use std::process::Command;
    use tokio::time::{timeout, Duration};
    
    info!("Setting reminder {} completed={}", reminder_id, completed);
    
    let script = format!(
        r#"
        ObjC.import('EventKit');
        
        var store = $.EKEventStore.alloc.init;
        var reminder = store.calendarItemWithIdentifier($.NSString.stringWithString('{reminder_id}'));
        
        if (reminder && reminder.isKindOfClass($.EKReminder)) {{
            reminder.completed = {completed};
            var error = $();
            var success = store.saveReminderCommitError(reminder, true, error);
            success ? 'true' : 'false';
        }} else {{
            'false';
        }}
        "#,
        reminder_id = reminder_id.replace('\'', "\\'"),
        completed = if completed { "true" } else { "false" }
    );
    
    let output_future = tokio::task::spawn_blocking(move || {
        Command::new("osascript")
            .arg("-l")
            .arg("JavaScript")
            .arg("-e")
            .arg(&script)
            .output()
    });
    
    let output = match timeout(Duration::from_secs(10), output_future).await {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => {
            error!("Failed to execute osascript: {}", e);
            return false;
        }
        Err(_) => {
            error!("JXA reminder completion status update timed out after 10 seconds");
            return false;
        }
    };
    
    match output {
        Ok(out) => {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                stdout == "true"
            } else {
                error!("JXA reminder complete failed: {}", String::from_utf8_lossy(&out.stderr));
                false
            }
        }
        Err(e) => {
            error!("Failed to execute osascript: {}", e);
            false
        }
    }
}

/// Delete a reminder
#[cfg(target_os = "macos")]
async fn delete_reminder(reminder_id: &str) -> bool {
    use std::process::Command;
    use tokio::time::{timeout, Duration};
    
    info!("Deleting reminder {}", reminder_id);
    
    let script = format!(
        r#"
        ObjC.import('EventKit');
        
        var store = $.EKEventStore.alloc.init;
        var reminder = store.calendarItemWithIdentifier($.NSString.stringWithString('{reminder_id}'));
        
        if (reminder && reminder.isKindOfClass($.EKReminder)) {{
            var error = $();
            var success = store.removeReminderCommitError(reminder, true, error);
            success ? 'true' : 'false';
        }} else {{
            'false';
        }}
        "#,
        reminder_id = reminder_id.replace('\'', "\\'")
    );
    
    let output_future = tokio::task::spawn_blocking(move || {
        Command::new("osascript")
            .arg("-l")
            .arg("JavaScript")
            .arg("-e")
            .arg(&script)
            .output()
    });
    
    let output = match timeout(Duration::from_secs(10), output_future).await {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => {
            error!("Failed to execute osascript: {}", e);
            return false;
        }
        Err(_) => {
            error!("JXA reminder delete timed out after 10 seconds");
            return false;
        }
    };
    
    match output {
        Ok(out) => {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                stdout == "true"
            } else {
                error!("JXA reminder delete failed: {}", String::from_utf8_lossy(&out.stderr));
                false
            }
        }
        Err(e) => {
            error!("Failed to execute osascript: {}", e);
            false
        }
    }
}

// Non-macOS stubs
#[cfg(not(target_os = "macos"))]
async fn fetch_reminder_lists() -> Vec<ReminderList> { Vec::new() }

#[cfg(not(target_os = "macos"))]
async fn fetch_reminders(_list_id: Option<&str>, _include_completed: bool) -> Vec<Reminder> { Vec::new() }

#[cfg(not(target_os = "macos"))]
async fn create_reminder(
    _title: &str,
    _list_id: Option<&str>,
    _due_date: Option<&str>,
    _notes: Option<&str>,
    _priority: i32,
) -> Option<Reminder> { None }

#[cfg(not(target_os = "macos"))]
async fn set_reminder_completed(_reminder_id: &str, _completed: bool) -> bool { false }

#[cfg(not(target_os = "macos"))]
async fn delete_reminder(_reminder_id: &str) -> bool { false }
