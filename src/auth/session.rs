//! Session management for user authentication
//!
//! This module provides session management with secure random tokens
//! and cookie-based authentication.
//!
//! # Security
//!
//! - Session IDs are 32-byte random tokens (64 hex characters)
//! - Tokens generated using cryptographically secure random number generator
//! - Sessions stored in database, not in cookies (stateful)
//! - Cookies are HttpOnly, Secure, and SameSite=Lax
//! - Sessions persist indefinitely until logout (no expiration)
//!
//! # Session Lifecycle
//!
//! 1. Login: Create session with `create_session()`, set cookie
//! 2. Request: Extract session ID from cookie, validate with `get_session()`
//! 3. Logout: Delete session with `delete_session()`, clear cookie

use crate::error::AppError;
use axum_extra::extract::cookie::{Cookie, SameSite};
use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

/// Session entity
///
/// Represents an active user session. Sessions are stored in the database
/// and referenced by a secure random token stored in a cookie.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    /// Unique session identifier (secure random token)
    pub id: String,
    /// ID of the user this session belongs to
    pub user_id: Uuid,
    /// Timestamp when session was created
    pub created_at: DateTime<Utc>,
}

/// Cookie name for session ID
pub const SESSION_COOKIE_NAME: &str = "session_id";

/// Create a new session for a user
///
/// Generates a secure random session token and stores it in the database.
/// The token should be set as a cookie on the client.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `user_id` - ID of the user to create a session for
///
/// # Returns
///
/// Returns the created `Session` on success, or an `AppError` on failure.
///
/// # Errors
///
/// - `AppError::Database` - Database operation failed
///
/// # Security
///
/// - Uses cryptographically secure random number generator
/// - Generates 32 bytes of randomness (64 hex characters)
/// - Tokens are globally unique with high probability
///
/// # Example
///
/// ```rust
/// let session = create_session(&pool, user.id).await?;
/// let cookie = create_session_cookie(&session.id);
/// // Set cookie on response
/// ```
pub async fn create_session(pool: &PgPool, user_id: Uuid) -> Result<Session, AppError> {
    // Generate secure random session ID
    let session_id = generate_session_token();

    tracing::info!(
        user_id = %user_id,
        session_id = %session_id,
        "Creating new session"
    );

    // Insert session into database
    let session = sqlx::query_as::<_, Session>(
        r#"
        INSERT INTO sessions (id, user_id)
        VALUES ($1, $2)
        RETURNING id, user_id, created_at
        "#,
    )
    .bind(&session_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    tracing::info!(
        user_id = %user_id,
        session_id = %session_id,
        "Session created successfully"
    );

    Ok(session)
}

/// Get a session by ID
///
/// Looks up a session in the database by its token.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `session_id` - Session token to look up
///
/// # Returns
///
/// Returns `Some(Session)` if found, `None` if not found, or an `AppError` on failure.
///
/// # Errors
///
/// - `AppError::Database` - Database operation failed
///
/// # Example
///
/// ```rust
/// match get_session(&pool, session_id).await? {
///     Some(session) => println!("Valid session for user {}", session.user_id),
///     None => println!("Invalid or expired session"),
/// }
/// ```
pub async fn get_session(pool: &PgPool, session_id: &str) -> Result<Option<Session>, AppError> {
    tracing::debug!(session_id = %session_id, "Looking up session");

    let session = sqlx::query_as::<_, Session>(
        r#"
        SELECT id, user_id, created_at
        FROM sessions
        WHERE id = $1
        "#,
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await?;

    if session.is_some() {
        tracing::debug!(session_id = %session_id, "Session found");
    } else {
        tracing::debug!(session_id = %session_id, "Session not found");
    }

    Ok(session)
}

/// Delete a session (logout)
///
/// Removes a session from the database. This is used during logout.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `session_id` - Session token to delete
///
/// # Returns
///
/// Returns `()` on success (even if session didn't exist), or an `AppError` on failure.
///
/// # Errors
///
/// - `AppError::Database` - Database operation failed
///
/// # Example
///
/// ```rust
/// delete_session(&pool, session_id).await?;
/// // Clear session cookie on response
/// ```
pub async fn delete_session(pool: &PgPool, session_id: &str) -> Result<(), AppError> {
    tracing::info!(session_id = %session_id, "Deleting session");

    let result = sqlx::query(
        r#"
        DELETE FROM sessions
        WHERE id = $1
        "#,
    )
    .bind(session_id)
    .execute(pool)
    .await?;

    tracing::info!(
        session_id = %session_id,
        rows_affected = result.rows_affected(),
        "Session deleted"
    );

    Ok(())
}

/// Delete all sessions for a user
///
/// Removes all sessions for a specific user. This can be used for
/// "logout from all devices" functionality.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `user_id` - ID of the user whose sessions to delete
///
/// # Returns
///
/// Returns `()` on success, or an `AppError` on failure.
///
/// # Errors
///
/// - `AppError::Database` - Database operation failed
///
/// # Example
///
/// ```rust
/// // Logout user from all devices
/// delete_all_user_sessions(&pool, user.id).await?;
/// ```
pub async fn delete_all_user_sessions(pool: &PgPool, user_id: Uuid) -> Result<(), AppError> {
    tracing::info!(user_id = %user_id, "Deleting all sessions for user");

    let result = sqlx::query(
        r#"
        DELETE FROM sessions
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    tracing::info!(
        user_id = %user_id,
        sessions_deleted = result.rows_affected(),
        "All user sessions deleted"
    );

    Ok(())
}

// Cookie helper functions

/// Create a session cookie
///
/// Creates a cookie containing the session ID with secure settings.
///
/// # Cookie Settings
///
/// - Name: "session_id"
/// - HttpOnly: true (JavaScript cannot access)
/// - Secure: true in production (HTTPS only), false in development
/// - SameSite: Lax (CSRF protection)
/// - Path: "/" (available on all routes)
/// - Max-Age: None (session cookie, persists until logout)
///
/// # Arguments
///
/// * `session_id` - Session token to store in cookie
///
/// # Returns
///
/// Returns a configured `Cookie` ready to be set on a response.
///
/// # Security
///
/// - HttpOnly prevents XSS attacks from stealing session token
/// - Secure ensures cookie only sent over HTTPS (in production)
/// - SameSite=Lax provides CSRF protection while allowing normal navigation
/// - No expiration means session persists until explicit logout
///
/// # Example
///
/// ```rust
/// let cookie = create_session_cookie(&session.id);
/// // Set cookie on Axum response
/// ```
pub fn create_session_cookie(session_id: &str) -> Cookie<'static> {
    // In development (debug builds), don't require HTTPS for cookies
    // In production (release builds), require HTTPS
    let is_secure = !cfg!(debug_assertions);

    Cookie::build((SESSION_COOKIE_NAME, session_id.to_string()))
        .http_only(true)
        .secure(is_secure)
        .same_site(SameSite::Lax)
        .path("/")
        // No max_age = session cookie (deleted when browser closes)
        // But server-side session persists, so if browser keeps cookie, session remains valid
        .into()
}

/// Create a cookie to clear the session
///
/// Creates a cookie that will delete the session cookie on the client.
/// This is used during logout.
///
/// # Returns
///
/// Returns a configured `Cookie` that will clear the session cookie.
///
/// # Example
///
/// ```rust
/// let cookie = create_clear_session_cookie();
/// // Set cookie on response to clear session
/// ```
pub fn create_clear_session_cookie() -> Cookie<'static> {
    // In development (debug builds), don't require HTTPS for cookies
    // In production (release builds), require HTTPS
    let is_secure = !cfg!(debug_assertions);

    Cookie::build((SESSION_COOKIE_NAME, ""))
        .http_only(true)
        .secure(is_secure)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(Duration::from_secs(0).try_into().unwrap())
        .into()
}

/// Extract session ID from cookies
///
/// Looks for the session cookie and extracts its value.
///
/// # Arguments
///
/// * `cookies` - Cookie jar from the request
///
/// # Returns
///
/// Returns `Some(String)` with the session ID if the cookie exists,
/// or `None` if the cookie is not present.
///
/// # Example
///
/// ```rust
/// use axum_extra::extract::CookieJar;
///
/// async fn handler(cookies: CookieJar) -> Result<(), AppError> {
///     if let Some(session_id) = get_session_from_cookies(&cookies) {
///         // Validate session
///     } else {
///         // No session cookie
///     }
///     Ok(())
/// }
/// ```
pub fn get_session_from_cookies(cookies: &axum_extra::extract::CookieJar) -> Option<String> {
    cookies
        .get(SESSION_COOKIE_NAME)
        .map(|cookie| cookie.value().to_string())
}

// Private helper functions

/// Generate a secure random session token
///
/// Generates a 32-byte random token and encodes it as hexadecimal.
/// This produces a 64-character hex string.
///
/// # Security
///
/// Uses `rand::thread_rng()` which is cryptographically secure on all platforms.
/// 32 bytes = 256 bits of entropy, making brute force attacks infeasible.
///
/// # Returns
///
/// Returns a 64-character hexadecimal string.
fn generate_session_token() -> String {
    let random_bytes: [u8; 32] = rand::thread_rng().gen();
    hex::encode(random_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_session_token() {
        let token1 = generate_session_token();
        let token2 = generate_session_token();

        // Tokens should be 64 characters (32 bytes hex-encoded)
        assert_eq!(token1.len(), 64);
        assert_eq!(token2.len(), 64);

        // Tokens should be different
        assert_ne!(token1, token2);

        // Tokens should be valid hexadecimal
        assert!(token1.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(token2.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_session_cookie_settings() {
        let cookie = create_session_cookie("test_session_id");

        assert_eq!(cookie.name(), SESSION_COOKIE_NAME);
        assert_eq!(cookie.value(), "test_session_id");
        assert_eq!(cookie.http_only(), Some(true));
        // In debug builds, secure is false; in release builds, secure is true
        let expected_secure = !cfg!(debug_assertions);
        assert_eq!(cookie.secure(), Some(expected_secure));
        assert_eq!(cookie.same_site(), Some(SameSite::Lax));
        assert_eq!(cookie.path(), Some("/"));
        // Session cookie has no max_age
        assert_eq!(cookie.max_age(), None);
    }

    #[test]
    fn test_clear_session_cookie() {
        let cookie = create_clear_session_cookie();

        assert_eq!(cookie.name(), SESSION_COOKIE_NAME);
        assert_eq!(cookie.value(), "");
        // Should have max_age of 0 to delete cookie
        assert_eq!(cookie.max_age(), Some(Duration::from_secs(0).try_into().unwrap()));
    }
}
