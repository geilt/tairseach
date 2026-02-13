//! Common Utilities
//!
//! Shared types, error handling, and utility functions used across the application.

pub mod error;
pub mod http;
pub mod interpolation;
pub mod paths;
pub mod result;

#[allow(unused_imports)]
pub use error::{AppError, ErrorCode};
pub use http::{create_http_client, create_http_client_with_timeout};
pub use interpolation::{interpolate_credentials, interpolate_params};
#[allow(unused_imports)]
pub use paths::{logs_dir, manifest_dir, scripts_dir, socket_path, tairseach_dir, tairseach_path};
#[allow(unused_imports)]
pub use result::AppResult;
