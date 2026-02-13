//! Common Error Types
//!
//! Unified error handling with JSON-RPC error code mapping.

use std::fmt;

/// JSON-RPC error codes
///
/// Standard codes: -32768 to -32000
/// Custom codes: -32099 to -32000
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // Standard JSON-RPC errors
    ParseError = -32700,
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,
    
    // Custom Tairseach errors (auth subsystem)
    TokenNotFound = -32010,
    TokenRefreshFailed = -32011,
    ScopeInsufficient = -32012,
    ProviderNotSupported = -32013,
    AuthFlowInProgress = -32014,
    MasterKeyNotInitialized = -32015,
    
    // Permission errors
    PermissionDenied = -32001,
    
    // Generic application error
    GenericError = -32000,
}

impl ErrorCode {
    pub fn code(&self) -> i32 {
        *self as i32
    }
}

/// Application error type with JSON-RPC code
#[derive(Debug)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }
    
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
    
    /// Create a token not found error
    pub fn token_not_found(provider: &str, account: &str) -> Self {
        Self::new(
            ErrorCode::TokenNotFound,
            format!("No token found for {}:{}", provider, account),
        )
    }
    
    /// Create a token refresh failed error
    pub fn token_refresh_failed(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::TokenRefreshFailed, message)
    }
    
    /// Create a scope insufficient error
    pub fn scope_insufficient(scope: &str) -> Self {
        Self::new(
            ErrorCode::ScopeInsufficient,
            format!("Token missing scope: {}", scope),
        )
    }
    
    /// Create a provider not supported error
    pub fn provider_not_supported(provider: &str) -> Self {
        Self::new(
            ErrorCode::ProviderNotSupported,
            format!("Unsupported provider: {}", provider),
        )
    }
    
    /// Create a permission denied error
    pub fn permission_denied(permission: &str, status: &str) -> Self {
        Self::new(
            ErrorCode::PermissionDenied,
            format!("Permission '{}' not granted (status: {})", permission, status),
        )
    }
    
    /// Convert to (code, message) tuple for backward compatibility
    pub fn to_tuple(&self) -> (i32, String) {
        (self.code.code(), self.message.clone())
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code.code(), self.message)
    }
}

impl std::error::Error for AppError {}

// Conversion from String (for compatibility with existing code)
impl From<String> for AppError {
    fn from(message: String) -> Self {
        Self::new(ErrorCode::GenericError, message)
    }
}

impl From<&str> for AppError {
    fn from(message: &str) -> Self {
        Self::new(ErrorCode::GenericError, message)
    }
}

// Convert to String (for Tauri commands that return Result<T, String>)
impl From<AppError> for String {
    fn from(err: AppError) -> String {
        err.message
    }
}
