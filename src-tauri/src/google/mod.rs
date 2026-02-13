//! Google API Client Module
//!
//! Provides authenticated HTTP client access to Google APIs (Gmail, Calendar, Drive).
//! All methods use Tier 1 (proxy mode) — OAuth tokens never leave Tairseach process.

pub mod client;
pub mod common;
pub mod gmail;
pub mod calendar_api;

pub use gmail::GmailApi;
pub use calendar_api::CalendarApi;

/// Macro to implement the standard Google API wrapper constructor pattern.
/// Each API struct wraps a `GoogleClient` and provides `new(access_token)`.
macro_rules! google_api_wrapper {
    ($name:ident) => {
        impl $name {
            /// Create a new API client with an OAuth access token
            pub fn new(access_token: String) -> Result<Self, String> {
                let client = crate::google::client::GoogleClient::new(access_token)?;
                Ok(Self { client })
            }
        }
    };
}

pub(crate) use google_api_wrapper;
