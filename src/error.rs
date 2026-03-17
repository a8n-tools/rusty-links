//! Error handling framework for Rusty Links
//!
//! This module provides a comprehensive error handling system with:
//! - Unified AppError type for all application errors
//! - Automatic conversions from common error types
//! - HTTP status code mapping for API responses
//! - User-friendly error messages for frontend
//! - Detailed error logging for debugging
//!
//! # Design Philosophy
//!
//! 1. **Single Error Type**: AppError is used throughout the application
//! 2. **Context Preservation**: Errors include relevant context (field names, resource IDs, etc.)
//! 3. **User-Friendly Messages**: Frontend receives clear, actionable error messages
//! 4. **Detailed Logging**: Logs include full error chains for debugging
//! 5. **HTTP Mapping**: Each error variant maps to appropriate HTTP status code

use serde::Serialize;
use std::fmt;

/// Main application error type.
///
/// This enum represents all possible errors that can occur in the application.
/// Each variant includes context-specific information to aid in debugging and
/// provide meaningful error messages to users.
#[derive(Debug)]
pub enum AppError {
    /// Database operation failed
    ///
    /// Wraps sqlx::Error for database-related failures including:
    /// - Connection failures
    /// - Query execution errors
    /// - Transaction failures
    /// - Row not found errors
    Database(sqlx::Error),

    /// Configuration error
    ///
    /// Occurs during application startup when configuration is invalid or missing.
    /// The String contains a descriptive error message.
    Configuration(String),

    /// Validation error for user input
    ///
    /// Used when user-provided data fails validation rules.
    /// Includes the field name and a user-friendly error message.
    ///
    /// # Example
    /// ```
    /// AppError::Validation {
    ///     field: "email".to_string(),
    ///     message: "Email must contain @ symbol".to_string()
    /// }
    /// ```
    Validation {
        /// Name of the field that failed validation
        field: String,
        /// User-friendly validation error message
        message: String,
    },

    /// Authentication failed - invalid credentials
    ///
    /// Used when login attempt fails due to incorrect email or password.
    /// Generic message to avoid leaking information about which field was wrong.
    InvalidCredentials,

    /// Session has expired
    ///
    /// The user's session is no longer valid and they need to log in again.
    SessionExpired,

    /// User is not authorized to access the requested resource
    ///
    /// The user is authenticated but lacks permission for this operation.
    Unauthorized,

    /// Operation is forbidden
    ///
    /// The user is authenticated but the operation is not allowed.
    Forbidden(String),

    /// Requested resource was not found
    ///
    /// Includes the resource type (e.g., "user", "link") and identifier.
    ///
    /// # Example
    /// ```
    /// AppError::NotFound {
    ///     resource: "link".to_string(),
    ///     id: "550e8400-e29b-41d4-a716-446655440000".to_string()
    /// }
    /// ```
    NotFound {
        /// Type of resource (e.g., "user", "link", "category")
        resource: String,
        /// Identifier of the resource (UUID, email, etc.)
        id: String,
    },

    /// Duplicate resource - unique constraint violation
    ///
    /// Occurs when trying to create a resource that already exists.
    /// Includes the field name that must be unique.
    ///
    /// # Example
    /// ```
    /// AppError::Duplicate {
    ///     field: "email".to_string()
    /// }
    /// ```
    Duplicate {
        /// Name of the field that must be unique
        field: String,
    },

    /// External service error (GitHub API, web scraping, etc.)
    ///
    /// Wraps errors from external HTTP APIs and web scraping operations.
    /// The String contains details about which service failed and why.
    ExternalService(String),

    /// I/O error (file operations, network, etc.)
    ///
    /// Wraps std::io::Error for file system and network operations.
    Io(std::io::Error),

    /// JSON serialization/deserialization error
    ///
    /// Wraps serde_json::Error for JSON parsing failures.
    Json(serde_json::Error),

    /// Account is locked due to too many failed login attempts
    AccountLocked,

    /// Membership required (SaaS mode)
    ///
    /// The user is authenticated but does not have an active membership.
    /// The String contains the membership page URL for the frontend to redirect to.
    MembershipRequired(String),

    /// Internal server error
    ///
    /// Used for unexpected errors that don't fit other categories.
    /// The String contains details for logging; users see a generic message.
    Internal(String),
}

/// API error response structure for JSON responses
///
/// This struct is serialized and sent to the frontend when an error occurs.
/// It provides a consistent error format across all API endpoints.
///
/// # Example JSON Response
/// ```json
/// {
///     "error": "Email must be unique",
///     "code": "DUPLICATE_FIELD",
///     "status": 409
/// }
/// ```
#[derive(Debug, Serialize)]
pub struct ApiErrorResponse {
    /// User-friendly error message
    pub error: String,
    /// Machine-readable error code
    pub code: String,
    /// HTTP status code
    pub status: u16,
}

impl AppError {
    /// Create a validation error
    ///
    /// # Arguments
    /// * `field` - Name of the field that failed validation
    /// * `message` - User-friendly validation error message
    ///
    /// # Example
    /// ```
    /// let error = AppError::validation("email", "Email must contain @ symbol");
    /// ```
    pub fn validation(field: &str, message: &str) -> Self {
        AppError::Validation {
            field: field.to_string(),
            message: message.to_string(),
        }
    }

    /// Create a not found error
    ///
    /// # Arguments
    /// * `resource` - Type of resource (e.g., "user", "link", "category")
    /// * `id` - Identifier of the resource
    ///
    /// # Example
    /// ```
    /// let error = AppError::not_found("link", "550e8400-e29b-41d4-a716-446655440000");
    /// ```
    pub fn not_found(resource: &str, id: &str) -> Self {
        AppError::NotFound {
            resource: resource.to_string(),
            id: id.to_string(),
        }
    }

    /// Create a duplicate field error
    ///
    /// # Arguments
    /// * `field` - Name of the field that must be unique
    ///
    /// # Example
    /// ```
    /// let error = AppError::duplicate("email");
    /// ```
    pub fn duplicate(field: &str) -> Self {
        AppError::Duplicate {
            field: field.to_string(),
        }
    }

    /// Create a forbidden operation error
    pub fn forbidden(message: &str) -> Self {
        AppError::Forbidden(message.to_string())
    }

    /// Get the HTTP status code for this error
    ///
    /// Maps each error variant to an appropriate HTTP status code
    /// for use in API responses.
    ///
    /// # Status Code Mapping
    /// - 400 Bad Request: Validation errors
    /// - 401 Unauthorized: Invalid credentials, session expired
    /// - 403 Forbidden: Unauthorized access
    /// - 404 Not Found: Resource not found
    /// - 409 Conflict: Duplicate resources
    /// - 500 Internal Server Error: Database, I/O, JSON, Internal errors
    /// - 502 Bad Gateway: External service errors
    /// - 503 Service Unavailable: Configuration errors
    pub fn status_code(&self) -> u16 {
        match self {
            AppError::Validation { .. } => 400,
            AppError::InvalidCredentials => 401,
            AppError::SessionExpired => 401,
            AppError::Unauthorized => 403,
            AppError::Forbidden(_) => 403,
            AppError::NotFound { .. } => 404,
            AppError::Duplicate { .. } => 409,
            AppError::MembershipRequired(_) => 403,
            AppError::AccountLocked => 429,
            AppError::Database(_) => 500,
            AppError::Io(_) => 500,
            AppError::Json(_) => 500,
            AppError::Internal(_) => 500,
            AppError::ExternalService(_) => 502,
            AppError::Configuration(_) => 503,
        }
    }

    /// Get a machine-readable error code
    ///
    /// Returns a stable error code that can be used by frontend
    /// for error handling logic and internationalization.
    pub fn error_code(&self) -> &str {
        match self {
            AppError::Database(_) => "DATABASE_ERROR",
            AppError::Configuration(_) => "CONFIGURATION_ERROR",
            AppError::Validation { .. } => "VALIDATION_ERROR",
            AppError::InvalidCredentials => "INVALID_CREDENTIALS",
            AppError::SessionExpired => "SESSION_EXPIRED",
            AppError::Unauthorized => "UNAUTHORIZED",
            AppError::Forbidden(_) => "FORBIDDEN",
            AppError::MembershipRequired(_) => "MEMBERSHIP_REQUIRED",
            AppError::NotFound { .. } => "NOT_FOUND",
            AppError::Duplicate { .. } => "DUPLICATE_FIELD",
            AppError::AccountLocked => "ACCOUNT_LOCKED",
            AppError::ExternalService(_) => "EXTERNAL_SERVICE_ERROR",
            AppError::Io(_) => "IO_ERROR",
            AppError::Json(_) => "JSON_ERROR",
            AppError::Internal(_) => "INTERNAL_ERROR",
        }
    }

    /// Convert error to API response structure
    ///
    /// Creates an ApiErrorResponse with user-friendly message,
    /// error code, and HTTP status code.
    pub fn to_response(&self) -> ApiErrorResponse {
        ApiErrorResponse {
            error: self.user_message(),
            code: self.error_code().to_string(),
            status: self.status_code(),
        }
    }

    /// Get user-friendly error message
    ///
    /// Returns a message suitable for displaying to end users.
    /// Avoids exposing internal implementation details or sensitive information.
    fn user_message(&self) -> String {
        match self {
            AppError::Database(_) => {
                "A database error occurred. Please try again later.".to_string()
            }
            AppError::Configuration(msg) => format!("Configuration error: {}", msg),
            AppError::Validation { field, message } => {
                format!("{}: {}", field, message)
            }
            AppError::InvalidCredentials => "Invalid email or password.".to_string(),
            AppError::SessionExpired => {
                "Your session has expired. Please log in again.".to_string()
            }
            AppError::Unauthorized => "You are not authorized to access this resource.".to_string(),
            AppError::Forbidden(msg) => msg.clone(),
            AppError::MembershipRequired(_) => "Membership required".to_string(),
            AppError::NotFound { resource, .. } => {
                format!("{} not found.", capitalize_first(resource))
            }
            AppError::Duplicate { field } => {
                format!("{} already exists.", capitalize_first(field))
            }
            AppError::AccountLocked => {
                "Account is temporarily locked due to too many failed login attempts. Please try again later.".to_string()
            }
            AppError::ExternalService(msg) => {
                format!("External service error: {}", msg)
            }
            AppError::Io(_) => "An I/O error occurred. Please try again.".to_string(),
            AppError::Json(_) => "Invalid JSON data.".to_string(),
            AppError::Internal(_) => {
                "An internal error occurred. Please try again later.".to_string()
            }
        }
    }
}

/// Implement Display for user-friendly error messages
///
/// Used when converting errors to strings for display to users.
impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.user_message())
    }
}

/// Implement std::error::Error trait
///
/// Provides source() method for error chain traversal and
/// allows AppError to be used anywhere std::error::Error is expected.
impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Database(e) => Some(e),
            AppError::Io(e) => Some(e),
            AppError::Json(e) => Some(e),
            _ => None,
        }
    }
}

// Automatic conversions from common error types

/// Convert sqlx::Error to AppError
///
/// Handles special cases:
/// - RowNotFound becomes NotFound error
/// - Unique constraint violations become Duplicate errors
/// - Other errors wrapped as Database errors
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            // RowNotFound is a common case we can handle specifically
            sqlx::Error::RowNotFound => AppError::NotFound {
                resource: "resource".to_string(),
                id: "unknown".to_string(),
            },
            // Check for unique constraint violations
            sqlx::Error::Database(ref db_err) if db_err.is_unique_violation() => {
                // Try to extract field name from error message
                // PostgreSQL format: "duplicate key value violates unique constraint \"constraint_name\""
                let field = extract_field_from_constraint(db_err.message());
                AppError::Duplicate { field }
            }
            // All other database errors
            _ => AppError::Database(err),
        }
    }
}

/// Convert std::io::Error to AppError
impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err)
    }
}

/// Convert serde_json::Error to AppError
impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Json(err)
    }
}

/// Convert sqlx::migrate::MigrateError to AppError
impl From<sqlx::migrate::MigrateError> for AppError {
    fn from(err: sqlx::migrate::MigrateError) -> Self {
        AppError::Internal(format!("Migration error: {}", err))
    }
}

/// Convert reqwest::Error to AppError
impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        AppError::ExternalService(format!("HTTP request failed: {}", err))
    }
}

/// Convert url::ParseError to AppError
impl From<url::ParseError> for AppError {
    fn from(err: url::ParseError) -> Self {
        AppError::Validation {
            field: "url".to_string(),
            message: format!("Invalid URL: {}", err),
        }
    }
}

// Helper functions

/// Capitalize the first letter of a string
///
/// Used to format error messages with proper capitalization.
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Extract field name from PostgreSQL constraint name
///
/// Attempts to parse the field name from a unique constraint error.
/// Falls back to "field" if unable to parse.
///
/// # Examples
/// - "uq_users_email" -> "email"
/// - "uq_links_user_domain_path" -> "user_domain_path"
fn extract_field_from_constraint(message: &str) -> String {
    // Look for constraint name in quotes
    if let Some(start) = message.find('"') {
        if let Some(end) = message[start + 1..].find('"') {
            let constraint_name = &message[start + 1..start + 1 + end];

            // PostgreSQL constraint naming convention: uq_table_field or idx_table_field
            // Try to extract the field part
            let parts: Vec<&str> = constraint_name.split('_').collect();
            if parts.len() >= 3 {
                // Skip "uq" or "idx" and table name, join the rest
                return parts[2..].join("_");
            }
        }
    }

    // Fallback
    "field".to_string()
}

// HTTP response conversion for Axum

/// Implement IntoResponse for AppError to enable automatic error conversion in Axum handlers
///
/// This allows handlers to return `Result<T, AppError>` and have errors automatically
/// converted to proper HTTP responses with appropriate status codes and JSON bodies.
impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        use axum::Json;

        // Log the error for debugging
        match &self {
            AppError::InvalidCredentials => {
                tracing::warn!("Authentication failed: Invalid credentials");
            }
            AppError::SessionExpired => {
                tracing::debug!("Session expired (details logged by auth middleware)");
            }
            AppError::Unauthorized => {
                tracing::warn!("Unauthorized access attempt");
            }
            AppError::Forbidden(msg) => {
                tracing::warn!(message = %msg, "Forbidden operation");
            }
            AppError::Validation { field, message } => {
                tracing::info!(field = %field, message = %message, "Validation error");
            }
            AppError::Duplicate { field } => {
                tracing::info!(field = %field, "Duplicate field error");
            }
            AppError::NotFound { resource, id } => {
                tracing::debug!(resource = %resource, id = %id, "Resource not found");
            }
            AppError::MembershipRequired(url) => {
                tracing::info!(redirect = %url, "Membership required");
            }
            AppError::AccountLocked => {
                tracing::warn!("Account locked due to too many failed attempts");
            }
            AppError::Database(e) => {
                tracing::error!(error = %e, "Database error");
            }
            AppError::Internal(msg) => {
                tracing::error!(message = %msg, "Internal error");
            }
            AppError::Configuration(msg) => {
                tracing::error!(message = %msg, "Configuration error");
            }
            AppError::ExternalService(msg) => {
                tracing::error!(message = %msg, "External service error");
            }
            AppError::Io(e) => {
                tracing::error!(error = %e, "I/O error");
            }
            AppError::Json(e) => {
                tracing::error!(error = %e, "JSON error");
            }
        }

        // MembershipRequired includes a redirect URL for the frontend
        if let AppError::MembershipRequired(ref redirect_url) = self {
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({
                    "error": "Membership required",
                    "code": "MEMBERSHIP_REQUIRED",
                    "status": 403,
                    "redirect": redirect_url,
                })),
            )
                .into_response();
        }

        // Convert error to API response
        let response = self.to_response();
        let status_code =
            StatusCode::from_u16(response.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        (status_code, Json(response)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error() {
        let error = AppError::validation("email", "Invalid email format");
        assert_eq!(error.status_code(), 400);
        assert_eq!(error.error_code(), "VALIDATION_ERROR");
        assert!(error.to_string().contains("email"));
        assert!(error.to_string().contains("Invalid email format"));
    }

    #[test]
    fn test_not_found_error() {
        let error = AppError::not_found("user", "123");
        assert_eq!(error.status_code(), 404);
        assert_eq!(error.error_code(), "NOT_FOUND");
        assert!(error.to_string().contains("User not found"));
    }

    #[test]
    fn test_duplicate_error() {
        let error = AppError::duplicate("email");
        assert_eq!(error.status_code(), 409);
        assert_eq!(error.error_code(), "DUPLICATE_FIELD");
        assert!(error.to_string().contains("Email already exists"));
    }

    #[test]
    fn test_api_error_response() {
        let error = AppError::validation("password", "Password too short");
        let response = error.to_response();

        assert_eq!(response.status, 400);
        assert_eq!(response.code, "VALIDATION_ERROR");
        assert!(response.error.contains("password"));
    }

    #[test]
    fn test_capitalize_first() {
        assert_eq!(capitalize_first("user"), "User");
        assert_eq!(capitalize_first("email"), "Email");
        assert_eq!(capitalize_first(""), "");
    }

    #[test]
    fn test_forbidden_error() {
        let error = AppError::forbidden("Admin access required");
        assert_eq!(error.status_code(), 403);
        assert_eq!(error.error_code(), "FORBIDDEN");
        assert!(error.to_string().contains("Admin access required"));
    }

    #[test]
    fn test_invalid_credentials_error() {
        let error = AppError::InvalidCredentials;
        assert_eq!(error.status_code(), 401);
        assert_eq!(error.error_code(), "INVALID_CREDENTIALS");
        assert!(error.to_string().contains("Invalid email or password"));
    }

    #[test]
    fn test_session_expired_error() {
        let error = AppError::SessionExpired;
        assert_eq!(error.status_code(), 401);
        assert_eq!(error.error_code(), "SESSION_EXPIRED");
        assert!(error.to_string().contains("session has expired"));
    }

    #[test]
    fn test_unauthorized_error() {
        let error = AppError::Unauthorized;
        assert_eq!(error.status_code(), 403);
        assert_eq!(error.error_code(), "UNAUTHORIZED");
    }

    #[test]
    fn test_account_locked_error() {
        let error = AppError::AccountLocked;
        assert_eq!(error.status_code(), 429);
        assert_eq!(error.error_code(), "ACCOUNT_LOCKED");
        assert!(error.to_string().contains("temporarily locked"));
    }

    #[test]
    fn test_membership_required_error() {
        let error = AppError::MembershipRequired("https://example.com/membership".to_string());
        assert_eq!(error.status_code(), 403);
        assert_eq!(error.error_code(), "MEMBERSHIP_REQUIRED");
        assert!(error.to_string().contains("Membership required"));
    }

    #[test]
    fn test_configuration_error() {
        let error = AppError::Configuration("Missing DATABASE_URL".to_string());
        assert_eq!(error.status_code(), 503);
        assert_eq!(error.error_code(), "CONFIGURATION_ERROR");
        assert!(error.to_string().contains("Missing DATABASE_URL"));
    }

    #[test]
    fn test_external_service_error() {
        let error = AppError::ExternalService("GitHub API timeout".to_string());
        assert_eq!(error.status_code(), 502);
        assert_eq!(error.error_code(), "EXTERNAL_SERVICE_ERROR");
        assert!(error.to_string().contains("GitHub API timeout"));
    }

    #[test]
    fn test_internal_error() {
        let error = AppError::Internal("unexpected state".to_string());
        assert_eq!(error.status_code(), 500);
        assert_eq!(error.error_code(), "INTERNAL_ERROR");
        // Internal details should NOT be exposed to user
        assert!(!error.to_string().contains("unexpected state"));
        assert!(error.to_string().contains("internal error"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let error: AppError = io_err.into();
        assert_eq!(error.status_code(), 500);
        assert_eq!(error.error_code(), "IO_ERROR");
    }

    #[test]
    fn test_json_error_conversion() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let error: AppError = json_err.into();
        assert_eq!(error.status_code(), 500);
        assert_eq!(error.error_code(), "JSON_ERROR");
        assert!(error.to_string().contains("Invalid JSON"));
    }

    #[test]
    fn test_url_parse_error_conversion() {
        let url_err = url::Url::parse("not a url").unwrap_err();
        let error: AppError = url_err.into();
        assert_eq!(error.status_code(), 400);
        assert_eq!(error.error_code(), "VALIDATION_ERROR");
    }

    #[test]
    fn test_extract_field_from_constraint_standard() {
        let msg = r#"duplicate key value violates unique constraint "uq_users_email""#;
        assert_eq!(extract_field_from_constraint(msg), "email");
    }

    #[test]
    fn test_extract_field_from_constraint_compound() {
        let msg = r#"duplicate key value violates unique constraint "uq_links_user_domain_path""#;
        assert_eq!(extract_field_from_constraint(msg), "user_domain_path");
    }

    #[test]
    fn test_extract_field_from_constraint_fallback() {
        let msg = "some other error message without quotes";
        assert_eq!(extract_field_from_constraint(msg), "field");
    }

    #[test]
    fn test_error_source_chain() {
        use std::error::Error;

        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "disk full");
        let error = AppError::Io(io_err);
        assert!(error.source().is_some());

        let error = AppError::InvalidCredentials;
        assert!(error.source().is_none());
    }

    #[test]
    fn test_all_status_codes_are_valid_http() {
        let errors: Vec<AppError> = vec![
            AppError::validation("f", "m"),
            AppError::InvalidCredentials,
            AppError::SessionExpired,
            AppError::Unauthorized,
            AppError::forbidden("msg"),
            AppError::not_found("r", "id"),
            AppError::duplicate("f"),
            AppError::AccountLocked,
            AppError::MembershipRequired("url".to_string()),
            AppError::Configuration("c".to_string()),
            AppError::ExternalService("e".to_string()),
            AppError::Internal("i".to_string()),
        ];

        for error in errors {
            let code = error.status_code();
            assert!(
                (100..600).contains(&code),
                "Invalid HTTP status code: {}",
                code
            );
        }
    }
}
