//! Common Result Type
//!
//! Type alias for application results.

use super::error::AppError;

/// Application result type
///
/// Uses AppError for consistent error handling across the app.
/// Can be converted to Result<T, String> for Tauri commands.
pub type AppResult<T> = Result<T, AppError>;
