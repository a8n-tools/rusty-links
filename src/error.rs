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
            AppError::NotFound { .. } => 404,
            AppError::Duplicate { .. } => 409,
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
            AppError::NotFound { .. } => "NOT_FOUND",
            AppError::Duplicate { .. } => "DUPLICATE_FIELD",
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
            AppError::Unauthorized => {
                "You are not authorized to access this resource.".to_string()
            }
            AppError::NotFound { resource, .. } => {
                format!("{} not found.", capitalize_first(resource))
            }
            AppError::Duplicate { field } => {
                format!("{} already exists.", capitalize_first(field))
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
                tracing::info!("Session expired");
            }
            AppError::Unauthorized => {
                tracing::warn!("Unauthorized access attempt");
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

        // Convert error to API response
        let response = self.to_response();
        let status_code = StatusCode::from_u16(response.status)
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

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
}
