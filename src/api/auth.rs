//! Authentication API endpoints
//!
//! Standalone mode: JWT-based auth with register, login, refresh, me, check-setup.
//! Hosted mode: OIDC owns the login flow; only `/me` is served here.
//!
//! The router (see [`crate::api::create_router`]) only mounts the standalone
//! handlers when running in standalone mode, so they return 404 in hosted mode.

use crate::error::AppError;
use crate::models::{check_user_exists, User};
use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use serde::Serialize;
use sqlx::PgPool;

use crate::auth::jwt::{create_jwt, generate_refresh_token};
use crate::auth::location_alert::spawn_new_location_check;
use crate::auth::login_approval::{approval_country, request_login_approval};
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

/// The one place a standalone session is minted: the access token, the stored
/// refresh token, and the body a client reads them from.
///
/// Every completion path goes through here, so the LINKS-35 gate has a single
/// thing to sit in front of and a second implementation cannot drift away from
/// this one.
async fn establish_jwt_session(
    pool: &PgPool,
    config: &Config,
    user: &User,
) -> Result<AuthResponse, AppError> {
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
        .execute(pool)
        .await?;

    Ok(AuthResponse {
        token,
        refresh_token,
        email: user.email.clone(),
        is_admin: user.is_admin,
    })
}

// ── Standalone mode handlers ──────────────────────────────────────────

/// POST /api/auth/setup (standalone)
///
/// Creates the first user account during initial application setup.
/// The first user is automatically an admin.
pub async fn setup_handler(
    State(pool): State<PgPool>,
    State(config): State<Config>,
    headers: HeaderMap,
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

    // Never gated by the LINKS-35 approval gate: this creates the first user,
    // so there is no prior country to differ from and holding it would mean the
    // first user could never sign in.
    let response = establish_jwt_session(&pool, &config, &user).await?;

    // Baseline this account's country so the next login from elsewhere alerts.
    spawn_new_location_check(&pool, &config.mail, user.id, &headers);

    tracing::info!(user_id = %user.id, "Setup completed successfully");

    Ok(Json(response))
}

/// POST /api/auth/register (standalone)
///
/// Register a new user account.
pub async fn register_handler(
    State(pool): State<PgPool>,
    State(config): State<Config>,
    headers: HeaderMap,
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

    // Never gated by the LINKS-35 approval gate, for the same reason as setup:
    // the account is being created, so it has no prior country.
    let response = establish_jwt_session(&pool, &config, &user).await?;

    // Baseline this account's country so the next login from elsewhere alerts.
    spawn_new_location_check(&pool, &config.mail, user.id, &headers);

    tracing::info!(user_id = %user.id, email = %user.email, "Registration successful");

    Ok(Json(response))
}

/// POST /api/auth/login (standalone)
///
/// Authenticate user with email and password.
pub async fn login_handler(
    State(pool): State<PgPool>,
    State(config): State<Config>,
    headers: HeaderMap,
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

    // LINKS-35: the credential check passed, so this is the point where a
    // session would be issued and therefore where the gate belongs. With the
    // gate on and the country new to this account, nothing is issued: the
    // sign-in is held until the owner approves it from an emailed link. The
    // LINKS-27 alert is not also fired, because the approval mail reports the
    // same event and additionally asks for a decision, and `last_login_country`
    // stays unwritten so an unapproved attempt cannot look familiar next time.
    if config.mail.login_approval_enabled {
        // The third field is the per-user alert opt-out, deliberately unused: a
        // preference set from a session must not switch a security gate off.
        if let Some((email, previous, _opt_out)) = User::get_login_location(&pool, user.id).await? {
            if let Some(country) = approval_country(&config.mail, previous.as_deref(), &headers) {
                request_login_approval(&pool, &config, user.id, &email, &country, &headers).await?;
                return Err(AppError::ApprovalRequired);
            }
        }
    }

    // Alert on a sign-in from a country this account has not used before.
    spawn_new_location_check(&pool, &config.mail, user.id, &headers);

    let response = establish_jwt_session(&pool, &config, &user).await?;

    tracing::info!(
        user_id = %user.id,
        email = %user.email,
        "Login successful"
    );

    Ok(Json(response))
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

    // Not gated by LINKS-35: this continues a session an earlier completed
    // sign-in already issued, it is not a sign-in of its own.
    Ok(Json(establish_jwt_session(&pool, &config, &user).await?))
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
