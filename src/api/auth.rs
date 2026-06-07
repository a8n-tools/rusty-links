//! Authentication API endpoints
//!
//! Standalone mode: JWT-based auth with register, login, refresh, me, check-setup.
//! Hosted mode: OIDC owns the login flow; only `/me` is served here.
//!
//! The router (see [`crate::api::create_router`]) only mounts the standalone
//! handlers when running in standalone mode, so they return 404 in hosted mode.

use crate::error::AppError;
use crate::models::{check_user_exists, User};
use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;
use sqlx::PgPool;

use crate::auth::jwt::{create_jwt, generate_refresh_token};
use crate::auth::middleware::{AuthenticatedUser, Claims};
use crate::config::Config;
use crate::models::{
    create_user, find_user_by_email, is_legacy_hash, upgrade_password_hash, verify_password,
    CreateUser,
};
use crate::security;
use crate::server_functions::auth::{AuthResponse, LoginRequest, RefreshRequest, SetupRequest};

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

/// POST /api/auth/setup (standalone)
///
/// Creates the first user account during initial application setup.
/// The first user is automatically an admin.
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
    security::validate_password(&request.password).map_err(|msg| AppError::Validation {
        field: "password".to_string(),
        message: msg,
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
    let expires_at = chrono::Utc::now() + chrono::Duration::days(config.refresh_token_expiry_days);
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
pub async fn register_handler(
    State(pool): State<PgPool>,
    State(config): State<Config>,
    Json(request): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(email = %request.email, "Registration attempt");

    // Check if registration is allowed
    if !config.allow_registration {
        return Err(AppError::Forbidden("Registration is disabled".to_string()));
    }

    // Validate password complexity
    security::validate_password(&request.password).map_err(|msg| AppError::Validation {
        field: "password".to_string(),
        message: msg,
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

    let expires_at = chrono::Utc::now() + chrono::Duration::days(config.refresh_token_expiry_days);
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

    // Migrate legacy bcrypt hash to Argon2id
    if is_legacy_hash(&user.password_hash) {
        if let Err(e) = upgrade_password_hash(&pool, user.id, &request.password).await {
            tracing::warn!(user_id = %user.id, error = %e, "Failed to migrate password hash");
        }
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

    let expires_at = chrono::Utc::now() + chrono::Duration::days(config.refresh_token_expiry_days);
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

/// POST /api/auth/logout (standalone)
///
/// Invalidates all refresh tokens for the current user.
pub async fn logout_handler(
    State(pool): State<PgPool>,
    claims: Claims,
) -> Result<impl IntoResponse, AppError> {
    let user_id: uuid::Uuid = claims
        .user_id
        .parse()
        .map_err(|_| AppError::SessionExpired)?;

    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(user_id)
        .execute(&pool)
        .await?;

    tracing::info!(user_id = %user_id, "User logged out, refresh tokens deleted");

    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// GET /api/auth/me
///
/// Returns information about the currently authenticated user. The
/// [`AuthenticatedUser`] extractor resolves the session per mode at runtime
/// (JWT bearer in standalone, `rl_session` cookie or `at+jwt` bearer in
/// hosted). `maintenance_mode` is always reported (it only ever flips in
/// hosted mode) so admin UIs can show a banner.
pub async fn me_handler(
    State(state): State<crate::api::AppState>,
    auth_user: AuthenticatedUser,
) -> Result<Json<crate::server_functions::auth::UserInfo>, AppError> {
    let user = User::find_by_id(&state.pool, auth_user.user_id)
        .await?
        .ok_or(AppError::SessionExpired)?;

    let maintenance_mode = state
        .maintenance_mode
        .load(std::sync::atomic::Ordering::Relaxed);

    tracing::debug!(user_id = %user.id, "User info requested");

    Ok(Json(crate::server_functions::auth::UserInfo {
        id: user.id.to_string(),
        email: user.email,
        name: user.name,
        is_admin: user.is_admin,
        maintenance_mode,
        auth_via_oidc: auth_user.auth_via_oidc,
    }))
}
