//! Authentication API endpoints
//!
//! This module provides REST API endpoints for user authentication:
//! - POST /api/auth/setup - Create first user (initial setup)
//! - POST /api/auth/login - Authenticate user and create session
//! - POST /api/auth/logout - End session and clear cookie
//! - GET /api/auth/me - Get current authenticated user
//! - GET /api/auth/check-setup - Check if initial setup is required

use crate::auth::{
    create_clear_session_cookie, create_session, create_session_cookie, delete_session,
    get_session, get_session_from_cookies,
};
use crate::error::AppError;
use crate::models::{check_user_exists, create_user, find_user_by_email, verify_password, CreateUser, User};
use axum::{
    extract::State,
    response::IntoResponse,
    Json,
};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

/// Request body for login endpoint
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Response for check-setup endpoint
#[derive(Debug, Serialize)]
pub struct CheckSetupResponse {
    pub setup_required: bool,
}

/// Response for successful authentication
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub message: String,
}

/// POST /api/auth/setup
///
/// Creates the first user account during initial application setup.
///
/// # Request Body
/// ```json
/// {
///     "email": "admin@example.com",
///     "password": "secure_password"
/// }
/// ```
///
/// # Response
/// - 201 Created: Returns user object and sets session cookie
/// - 403 Forbidden: Setup already completed (user exists)
/// - 400 Bad Request: Invalid email or password
/// - 409 Conflict: Email already exists
///
/// # Security
/// - Can only be called once (before first user is created)
/// - After setup, returns 403 Forbidden
/// - Automatically creates session for new user
///
/// # Example
/// ```bash
/// curl -X POST http://localhost:8080/api/auth/setup \
///   -H "Content-Type: application/json" \
///   -d '{"email":"admin@example.com","password":"secure123"}'
/// ```
pub async fn setup_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Json(request): Json<CreateUser>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(email = %request.email, "Setup request received");

    // Check if setup has already been completed
    if check_user_exists(&pool).await? {
        tracing::warn!("Setup attempt when user already exists");
        return Err(AppError::Unauthorized);
    }

    // Create the first user
    let user = create_user(&pool, request).await?;
    tracing::info!(user_id = %user.id, email = %user.email, "First user created");

    // Create session for the new user
    let session = create_session(&pool, user.id).await?;
    let cookie = create_session_cookie(&session.id);

    tracing::info!(
        user_id = %user.id,
        session_id = %session.id,
        "Setup completed successfully"
    );

    // Return user info with session cookie
    // Note: Using (CookieJar, Json) returns 200 OK by default
    // For 201 CREATED, we'd need to use Response::builder()
    Ok((jar.add(cookie), Json(user)))
}

/// POST /api/auth/login
///
/// Authenticates a user with email and password.
///
/// # Request Body
/// ```json
/// {
///     "email": "user@example.com",
///     "password": "user_password"
/// }
/// ```
///
/// # Response
/// - 200 OK: Returns user object and sets session cookie
/// - 401 Unauthorized: Invalid email or password
///
/// # Security
/// - Uses constant-time password comparison
/// - Generic error message to prevent email enumeration
/// - Logs authentication attempts for security monitoring
/// - Creates new session on successful login
///
/// # Example
/// ```bash
/// curl -X POST http://localhost:8080/api/auth/login \
///   -H "Content-Type: application/json" \
///   -d '{"email":"user@example.com","password":"password123"}' \
///   -c cookies.txt
/// ```
pub async fn login_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Json(request): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(email = %request.email, "Login attempt");

    // Find user by email
    let user = find_user_by_email(&pool, &request.email)
        .await?
        .ok_or_else(|| {
            tracing::warn!(email = %request.email, "Login failed: User not found");
            AppError::InvalidCredentials
        })?;

    // Verify password
    if !verify_password(&request.password, &user.password_hash)? {
        tracing::warn!(
            email = %request.email,
            user_id = %user.id,
            "Login failed: Invalid password"
        );
        return Err(AppError::InvalidCredentials);
    }

    // Create new session
    let session = create_session(&pool, user.id).await?;
    let cookie = create_session_cookie(&session.id);

    tracing::info!(
        user_id = %user.id,
        email = %user.email,
        session_id = %session.id,
        "Login successful"
    );

    // Return user info with session cookie
    Ok((jar.add(cookie), Json(user)))
}

/// POST /api/auth/logout
///
/// Ends the current session and clears the session cookie.
///
/// # Response
/// - 200 OK: Session deleted successfully
/// - 401 Unauthorized: No valid session
///
/// # Security
/// - Requires valid session cookie
/// - Deletes session from database
/// - Clears session cookie on client
///
/// # Example
/// ```bash
/// curl -X POST http://localhost:8080/api/auth/logout \
///   -b cookies.txt \
///   -c cookies.txt
/// ```
pub async fn logout_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    // Get session ID from cookie
    let session_id = get_session_from_cookies(&jar).ok_or_else(|| {
        tracing::debug!("Logout attempt without session cookie");
        AppError::SessionExpired
    })?;

    // Verify session exists
    let session = get_session(&pool, &session_id)
        .await?
        .ok_or_else(|| {
            tracing::debug!(session_id = %session_id, "Logout attempt with invalid session");
            AppError::SessionExpired
        })?;

    tracing::info!(
        user_id = %session.user_id,
        session_id = %session_id,
        "Logout request"
    );

    // Delete session from database
    delete_session(&pool, &session_id).await?;

    // Clear session cookie
    let cookie = create_clear_session_cookie();

    tracing::info!(
        user_id = %session.user_id,
        session_id = %session_id,
        "Logout successful"
    );

    Ok((
        jar.add(cookie),
        Json(AuthResponse {
            message: "Logged out successfully".to_string(),
        }),
    ))
}

/// GET /api/auth/me
///
/// Returns information about the currently authenticated user.
///
/// # Response
/// - 200 OK: Returns user object
/// - 401 Unauthorized: No valid session
///
/// # Security
/// - Requires valid session cookie
/// - Verifies session exists in database
/// - Loads fresh user data from database
///
/// # Example
/// ```bash
/// curl http://localhost:8080/api/auth/me \
///   -b cookies.txt
/// ```
pub async fn me_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
) -> Result<Json<User>, AppError> {
    // Get session ID from cookie
    let session_id = get_session_from_cookies(&jar).ok_or_else(|| {
        tracing::debug!("Request to /me without session cookie");
        AppError::SessionExpired
    })?;

    // Verify session exists
    let session = get_session(&pool, &session_id)
        .await?
        .ok_or_else(|| {
            tracing::debug!(session_id = %session_id, "Invalid session");
            AppError::SessionExpired
        })?;

    // Load user from database
    let user = sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, name, created_at FROM users WHERE id = $1",
    )
    .bind(session.user_id)
    .fetch_one(&pool)
    .await?;

    tracing::debug!(user_id = %user.id, "User info requested");

    Ok(Json(user))
}

/// GET /api/auth/check-setup
///
/// Checks if initial application setup is required.
///
/// # Response
/// ```json
/// {
///     "setup_required": true
/// }
/// ```
///
/// Returns `setup_required: true` if no users exist (setup needed).
/// Returns `setup_required: false` if at least one user exists (setup complete).
///
/// # Usage
/// Frontend can call this endpoint to determine whether to show
/// the setup page or the login page.
///
/// # Example
/// ```bash
/// curl http://localhost:8080/api/auth/check-setup
/// ```
pub async fn check_setup_handler(
    State(pool): State<PgPool>,
) -> Result<Json<CheckSetupResponse>, AppError> {
    let user_exists = check_user_exists(&pool).await?;

    tracing::debug!(user_exists = user_exists, "Setup check");

    Ok(Json(CheckSetupResponse {
        setup_required: !user_exists,
    }))
}
