//! Google Calendar API v3 Client
//!
//! Provides methods for interacting with Google Calendar API:
//! - List calendars
//! - List/get events
//! - Create/update/delete events
//!
//! All methods use the authenticated GoogleClient with Tier 1 proxy mode.

use super::client::GoogleClient;
use super::common::extract_array;
use serde_json::{json, Value};
use tracing::{debug, info};

const CALENDAR_API_BASE: &str = "https://www.googleapis.com/calendar/v3";

pub struct CalendarApi {
    client: GoogleClient,
}

super::google_api_wrapper!(CalendarApi);

impl CalendarApi {
    /// List all calendars for the authenticated user
    ///
    /// # Returns
    /// Array of calendar objects with id, summary, description, timeZone
    pub async fn list_calendars(&self) -> Result<Vec<Value>, String> {
        info!("Listing Google calendars");

        let url = format!("{}/users/me/calendarList", CALENDAR_API_BASE);
        let response = self.client.get(&url, &[]).await?;

        let calendars = extract_array(&response, "items");

        debug!("Retrieved {} calendars", calendars.len());
        Ok(calendars)
    }

    /// List events in a calendar
    ///
    /// # Arguments
    /// * `calendar_id` - Calendar ID (use "primary" for user's primary calendar)
    /// * `time_min` - Lower bound (RFC3339 timestamp, e.g., "2026-02-08T00:00:00Z")
    /// * `time_max` - Upper bound (RFC3339 timestamp)
    /// * `max_results` - Maximum number of events to return
    ///
    /// # Returns
    /// Array of event objects
    pub async fn list_events(
        &self,
        calendar_id: &str,
        time_min: Option<&str>,
        time_max: Option<&str>,
        max_results: Option<usize>,
    ) -> Result<Vec<Value>, String> {
        info!("Listing events for calendar: {}", calendar_id);

        let mut query_params = vec![];
        
        if let Some(min) = time_min {
            query_params.push(("timeMin", min.to_string()));
        }
        
        if let Some(max) = time_max {
            query_params.push(("timeMax", max.to_string()));
        }

        // Order by start time
        query_params.push(("orderBy", "startTime".to_string()));
        query_params.push(("singleEvents", "true".to_string()));

        let url = format!("{}/calendars/{}/events", CALENDAR_API_BASE, calendar_id);
        let events = self.client.get_paginated(&url, &query_params, max_results).await?;

        debug!("Retrieved {} events", events.len());
        Ok(events)
    }

    /// Get a specific event by ID
    ///
    /// # Arguments
    /// * `calendar_id` - Calendar ID
    /// * `event_id` - Event ID
    ///
    /// # Returns
    /// Event object with full details
    pub async fn get_event(
        &self,
        calendar_id: &str,
        event_id: &str,
    ) -> Result<Value, String> {
        info!("Fetching event: {} from calendar: {}", event_id, calendar_id);

        let url = format!(
            "{}/calendars/{}/events/{}",
            CALENDAR_API_BASE, calendar_id, event_id
        );
        self.client.get(&url, &[]).await
    }

    /// Create a new event
    ///
    /// # Arguments
    /// * `calendar_id` - Calendar ID (use "primary" for user's primary calendar)
    /// * `summary` - Event title
    /// * `start` - Start time (RFC3339 timestamp or date-only YYYY-MM-DD)
    /// * `end` - End time (RFC3339 timestamp or date-only YYYY-MM-DD)
    /// * `location` - Event location (optional)
    /// * `description` - Event description (optional)
    ///
    /// # Returns
    /// Created event object with generated ID
    pub async fn create_event(
        &self,
        calendar_id: &str,
        summary: &str,
        start: &str,
        end: &str,
        location: Option<&str>,
        description: Option<&str>,
    ) -> Result<Value, String> {
        info!("Creating event: {} in calendar: {}", summary, calendar_id);

        let mut event = json!({
            "summary": summary,
            "start": self.parse_datetime(start),
            "end": self.parse_datetime(end),
        });

        if let Some(loc) = location {
            event["location"] = json!(loc);
        }

        if let Some(desc) = description {
            event["description"] = json!(desc);
        }

        let url = format!("{}/calendars/{}/events", CALENDAR_API_BASE, calendar_id);
        let response = self.client.post(&url, &event).await?;

        info!("Event created successfully");
        Ok(response)
    }

    /// Update an existing event
    pub async fn update_event(
        &self,
        calendar_id: &str,
        event_id: &str,
        summary: Option<&str>,
        start: Option<&str>,
        end: Option<&str>,
        location: Option<&str>,
        description: Option<&str>,
    ) -> Result<Value, String> {
        info!("Updating event: {} in calendar: {}", event_id, calendar_id);

        // First, get the existing event
        let mut event = self.get_event(calendar_id, event_id).await?;

        // Update only the provided fields
        if let Some(s) = summary {
            event["summary"] = json!(s);
        }
        if let Some(st) = start {
            event["start"] = self.parse_datetime(st);
        }
        if let Some(e) = end {
            event["end"] = self.parse_datetime(e);
        }
        if let Some(loc) = location {
            event["location"] = json!(loc);
        }
        if let Some(desc) = description {
            event["description"] = json!(desc);
        }

        let url = format!(
            "{}/calendars/{}/events/{}",
            CALENDAR_API_BASE, calendar_id, event_id
        );
        let response = self.client.put(&url, &event).await?;

        info!("Event updated successfully");
        Ok(response)
    }

    /// Delete an event
    pub async fn delete_event(
        &self,
        calendar_id: &str,
        event_id: &str,
    ) -> Result<(), String> {
        info!("Deleting event: {} from calendar: {}", event_id, calendar_id);

        let url = format!(
            "{}/calendars/{}/events/{}",
            CALENDAR_API_BASE, calendar_id, event_id
        );
        self.client.delete(&url).await?;

        info!("Event deleted successfully");
        Ok(())
    }

    /// Parse datetime string into Google Calendar format
    /// Supports both RFC3339 timestamps and date-only formats
    fn parse_datetime(&self, dt: &str) -> Value {
        // Check if it's a date-only format (YYYY-MM-DD)
        if dt.len() == 10 && !dt.contains('T') {
            json!({ "date": dt })
        } else {
            // Assume it's an RFC3339 timestamp
            json!({ "dateTime": dt })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_datetime_with_timestamp() {
        let api = CalendarApi {
            client: GoogleClient::new("test".to_string()).unwrap(),
        };

        let result = api.parse_datetime("2026-02-08T15:00:00Z");
        assert!(result.get("dateTime").is_some());
        assert_eq!(result.get("dateTime").unwrap().as_str().unwrap(), "2026-02-08T15:00:00Z");
    }

    #[test]
    fn test_parse_datetime_with_date_only() {
        let api = CalendarApi {
            client: GoogleClient::new("test".to_string()).unwrap(),
        };

        let result = api.parse_datetime("2026-02-08");
        assert!(result.get("date").is_some());
        assert_eq!(result.get("date").unwrap().as_str().unwrap(), "2026-02-08");
    }
}
