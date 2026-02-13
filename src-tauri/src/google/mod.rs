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
