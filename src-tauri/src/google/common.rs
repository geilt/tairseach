//! Shared utilities for Google API modules
//!
//! Common patterns extracted from Calendar and Gmail implementations
//! to reduce repetition across Google API wrappers.

use serde_json::Value;

/// Extract an array field from a JSON response, returning an empty vec if missing.
///
/// Google APIs return list results under varying field names ("items", "labels",
/// "messages", "events", etc.). This normalizes the extraction pattern.
pub fn extract_array(response: &Value, field: &str) -> Vec<Value> {
    response
        .get(field)
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

/// Macro to define a Google API wrapper struct with the standard constructor.
///
/// Every Google API module wraps `GoogleClient` identically:
/// ```ignore
/// pub struct FooApi { client: GoogleClient }
/// impl FooApi {
///     pub fn new(access_token: String) -> Result<Self, String> {
///         let client = GoogleClient::new(access_token)?;
///         Ok(Self { client })
///     }
/// }
/// ```
/// This macro eliminates that boilerplate.
macro_rules! google_api_wrapper {
    ($name:ident) => {
        pub struct $name {
            client: super::client::GoogleClient,
        }

        impl $name {
            /// Create a new API client with an OAuth access token
            pub fn new(access_token: String) -> Result<Self, String> {
                let client = super::client::GoogleClient::new(access_token)?;
                Ok(Self { client })
            }
        }
    };
}

pub(crate) use google_api_wrapper;
