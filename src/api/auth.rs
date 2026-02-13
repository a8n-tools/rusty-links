//! Authentication API endpoints
//!
//! Standalone mode: JWT-based auth with register, login, refresh, me, check-setup
//! SaaS mode: Minimal auth (parent app handles authentication)

use crate::error::AppError;
use crate::models::{check_user_exists, User};
use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;
use sqlx::PgPool;

/// Response for check-setup endpoint
#[derive(Debug, Serialize)]
pub struct CheckSetupResponse {
    pub setup_required: bool,
}

/// GET /api/auth/check-setup
pub async fn check_setup_handler(
    State(pool): State<PgPool>,
) -> Result<Json<CheckSetupResponse>, AppError> {
    let user_exists = check_user_exists(&pool).await?;

    tracing::debug!(user_exists = user_exists, "Setup check");

    Ok(Json(CheckSetupResponse {
        setup_required: !user_exists,
    }))
}

// ── Standalone mode handlers ──────────────────────────────────────────

#[cfg(feature = "standalone")]
use crate::auth::jwt::{create_jwt, generate_refresh_token};
#[cfg(feature = "standalone")]
use crate::auth::middleware::Claims;
#[cfg(feature = "standalone")]
use crate::config::Config;
#[cfg(feature = "standalone")]
use crate::models::{create_user, find_user_by_email, verify_password, CreateUser};
#[cfg(feature = "standalone")]
use crate::security;
#[cfg(feature = "standalone")]
use crate::server_functions::auth::{
    AuthResponse, LoginRequest, RefreshRequest, SetupRequest,
};

/// POST /api/auth/setup (standalone)
///
/// Creates the first user account during initial application setup.
/// The first user is automatically an admin.
#[cfg(feature = "standalone")]
pub async fn setup_handler(
    State(pool): State<PgPool>,
    State(config): State<Config>,
    Json(request): Json<SetupRequest>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(email = %request.email, "Setup request received");

    // Check if setup has already been completed
    if check_user_exists(&pool).await? {
        tracing::warn!("Setup attempt when user already exists");
        return Err(AppError::Unauthorized);
    }

    // Validate password complexity
    security::validate_password(&request.password).map_err(|msg| {
        AppError::Validation {
            field: "password".to_string(),
            message: msg,
        }
    })?;

    // Create the first user (will automatically be admin)
    let user = User::create(&pool, &request.email, &request.password, &request.name).await?;
    tracing::info!(user_id = %user.id, email = %user.email, "First user created");

    // Create JWT + refresh token
    let token = create_jwt(
        &user.email,
        user.id,
        user.is_admin,
        &config.jwt_secret,
        config.jwt_expiry_hours,
    )
    .map_err(|e| AppError::Internal(format!("Failed to create JWT: {}", e)))?;

    let refresh_token = generate_refresh_token();

    // Store refresh token in database
    let expires_at =
        chrono::Utc::now() + chrono::Duration::days(config.refresh_token_expiry_days);
    sqlx::query("INSERT INTO refresh_tokens (user_id, token, expires_at) VALUES ($1, $2, $3)")
        .bind(user.id)
        .bind(&refresh_token)
        .bind(expires_at)
        .execute(&pool)
        .await?;

    tracing::info!(user_id = %user.id, "Setup completed successfully");

    Ok(Json(AuthResponse {
        token,
        refresh_token,
        email: user.email,
        is_admin: user.is_admin,
    }))
}

/// POST /api/auth/register (standalone)
///
/// Register a new user account.
#[cfg(feature = "standalone")]
pub async fn register_handler(
    State(pool): State<PgPool>,
    State(config): State<Config>,
    Json(request): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(email = %request.email, "Registration attempt");

    // Check if registration is allowed
    if !config.allow_registration {
        return Err(AppError::Forbidden(
            "Registration is disabled".to_string(),
        ));
    }

    // Validate password complexity
    security::validate_password(&request.password).map_err(|msg| {
        AppError::Validation {
            field: "password".to_string(),
            message: msg,
        }
    })?;

    // Create user
    let user = create_user(
        &pool,
        CreateUser {
            email: request.email.clone(),
            password: request.password,
        },
    )
    .await?;

    // Create JWT + refresh token
    let token = create_jwt(
        &user.email,
        user.id,
        user.is_admin,
        &config.jwt_secret,
        config.jwt_expiry_hours,
    )
    .map_err(|e| AppError::Internal(format!("Failed to create JWT: {}", e)))?;

    let refresh_token = generate_refresh_token();

    let expires_at =
        chrono::Utc::now() + chrono::Duration::days(config.refresh_token_expiry_days);
    sqlx::query("INSERT INTO refresh_tokens (user_id, token, expires_at) VALUES ($1, $2, $3)")
        .bind(user.id)
        .bind(&refresh_token)
        .bind(expires_at)
        .execute(&pool)
        .await?;

    tracing::info!(user_id = %user.id, email = %user.email, "Registration successful");

    Ok(Json(AuthResponse {
        token,
        refresh_token,
        email: user.email,
        is_admin: user.is_admin,
    }))
}

/// POST /api/auth/login (standalone)
///
/// Authenticate user with email and password.
#[cfg(feature = "standalone")]
pub async fn login_handler(
    State(pool): State<PgPool>,
    State(config): State<Config>,
    Json(request): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(email = %request.email, "Login attempt");

    // Check account lockout
    if security::is_account_locked(
        &pool,
        &request.email,
        config.account_lockout_attempts,
        config.account_lockout_duration_minutes,
    )
    .await
    {
        tracing::warn!(email = %request.email, "Login attempt on locked account");
        return Err(AppError::AccountLocked);
    }

    // Find user by email
    let user = find_user_by_email(&pool, &request.email)
        .await?
        .ok_or_else(|| {
            tracing::warn!(email = %request.email, "Login failed: User not found");
            AppError::InvalidCredentials
        })?;

    // Verify password
    if !verify_password(&request.password, &user.password_hash)? {
        // Record failed attempt
        security::record_login_attempt(&pool, &request.email, false).await;
        tracing::warn!(
            email = %request.email,
            user_id = %user.id,
            "Login failed: Invalid password"
        );
        return Err(AppError::InvalidCredentials);
    }

    // Record successful attempt
    security::record_login_attempt(&pool, &request.email, true).await;

    // Create JWT + refresh token
    let token = create_jwt(
        &user.email,
        user.id,
        user.is_admin,
        &config.jwt_secret,
        config.jwt_expiry_hours,
    )
    .map_err(|e| AppError::Internal(format!("Failed to create JWT: {}", e)))?;

    let refresh_token = generate_refresh_token();

    let expires_at =
        chrono::Utc::now() + chrono::Duration::days(config.refresh_token_expiry_days);
    sqlx::query("INSERT INTO refresh_tokens (user_id, token, expires_at) VALUES ($1, $2, $3)")
        .bind(user.id)
        .bind(&refresh_token)
        .bind(expires_at)
        .execute(&pool)
        .await?;

    tracing::info!(
        user_id = %user.id,
        email = %user.email,
        "Login successful"
    );

    Ok(Json(AuthResponse {
        token,
        refresh_token,
        email: user.email,
        is_admin: user.is_admin,
    }))
}

/// POST /api/auth/refresh (standalone)
///
/// Rotate refresh token and issue a new JWT.
#[cfg(feature = "standalone")]
pub async fn refresh_handler(
    State(pool): State<PgPool>,
    State(config): State<Config>,
    Json(request): Json<RefreshRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Look up the refresh token
    let row = sqlx::query_as::<_, (uuid::Uuid, uuid::Uuid, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, user_id, expires_at FROM refresh_tokens WHERE token = $1",
    )
    .bind(&request.refresh_token)
    .fetch_optional(&pool)
    .await?
    .ok_or(AppError::SessionExpired)?;

    let (token_id, user_id, expires_at) = row;

    // Check expiration
    if expires_at < chrono::Utc::now() {
        // Delete expired token
        sqlx::query("DELETE FROM refresh_tokens WHERE id = $1")
            .bind(token_id)
            .execute(&pool)
            .await?;
        return Err(AppError::SessionExpired);
    }

    // Delete old refresh token
    sqlx::query("DELETE FROM refresh_tokens WHERE id = $1")
        .bind(token_id)
        .execute(&pool)
        .await?;

    // Load user
    let user = User::find_by_id(&pool, user_id)
        .await?
        .ok_or(AppError::SessionExpired)?;

    // Create new JWT + refresh token
    let token = create_jwt(
        &user.email,
        user.id,
        user.is_admin,
        &config.jwt_secret,
        config.jwt_expiry_hours,
    )
    .map_err(|e| AppError::Internal(format!("Failed to create JWT: {}", e)))?;

    let new_refresh_token = generate_refresh_token();

    let new_expires_at =
        chrono::Utc::now() + chrono::Duration::days(config.refresh_token_expiry_days);
    sqlx::query("INSERT INTO refresh_tokens (user_id, token, expires_at) VALUES ($1, $2, $3)")
        .bind(user.id)
        .bind(&new_refresh_token)
        .bind(new_expires_at)
        .execute(&pool)
        .await?;

    Ok(Json(AuthResponse {
        token,
        refresh_token: new_refresh_token,
        email: user.email,
        is_admin: user.is_admin,
    }))
}

/// GET /api/auth/me (standalone)
///
/// Returns information about the currently authenticated user.
#[cfg(feature = "standalone")]
pub async fn me_handler(
    State(pool): State<PgPool>,
    claims: Claims,
) -> Result<Json<crate::server_functions::auth::UserInfo>, AppError> {
    let user_id: uuid::Uuid = claims
        .user_id
        .parse()
        .map_err(|_| AppError::SessionExpired)?;

    let user = User::find_by_id(&pool, user_id)
        .await?
        .ok_or(AppError::SessionExpired)?;

    tracing::debug!(user_id = %user.id, "User info requested");

    Ok(Json(crate::server_functions::auth::UserInfo {
        id: user.id.to_string(),
        email: user.email,
        name: user.name,
        is_admin: user.is_admin,
    }))
}

// ── SaaS mode handlers ───────────────────────────────────────────────

#[cfg(feature = "saas")]
use crate::auth::saas_auth;
#[cfg(feature = "saas")]
use axum_extra::extract::CookieJar;

/// GET /api/auth/me (saas)
///
/// Returns user info from the parent app's cookie.
#[cfg(feature = "saas")]
pub async fn me_handler(
    jar: CookieJar,
) -> Result<Json<crate::server_functions::auth::UserInfo>, AppError> {
    let claims = saas_auth::get_user_from_cookie(&jar).ok_or(AppError::SessionExpired)?;

    Ok(Json(crate::server_functions::auth::UserInfo {
        id: claims.user_id,
        email: claims.email.unwrap_or_default(),
        name: String::new(),
        is_admin: false,
    }))
}
