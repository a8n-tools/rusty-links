//! Authentication API endpoints
//!
//! Standalone mode: JWT-based auth with register, login, refresh, me, check-setup.
//! Hosted mode: OIDC owns the login flow; only `/me` is served here.
//!
//! The router (see [`crate::api::create_router`]) only mounts the standalone
//! handlers when running in standalone mode, so they return 404 in hosted mode.

use crate::error::AppError;
use crate::models::{check_user_exists, User};
use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use sqlx::PgPool;

use crate::auth::jwt::{create_jwt, generate_refresh_token, hash_refresh_token};
use crate::auth::known_device::{
    delete_for_user, hash_device_id, known_device_state, list_for_user, normalize_device_id,
    record_submitted_device,
};
use crate::auth::location_alert::{resolve_notify_new_location, spawn_new_location_check};
use crate::auth::login_approval::{approval_hold, request_login_approval};
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
/// this one. It is also the only place a device is recorded from a sign-in
/// (LINKS-45), which is what makes "recorded only when a sign-in COMPLETES"
/// structural: a held attempt returns before reaching this function, so it can
/// never make its device look familiar.
async fn establish_jwt_session(
    pool: &PgPool,
    config: &Config,
    user: &User,
    device_id: Option<&str>,
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

    // Only the digest is stored (LINKS-59); the token itself leaves in the
    // response body and is never written down.
    sqlx::query("INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)")
        .bind(user.id)
        .bind(hash_refresh_token(&refresh_token).as_slice())
        .bind(expires_at)
        .execute(pool)
        .await?;

    record_submitted_device(pool, user.id, device_id).await?;

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

    // Never gated by the LINKS-35 / LINKS-45 approval gate: this creates the
    // first user, so there is no prior country to differ from and no recorded
    // device to be new against, and holding it would mean the first user could
    // never sign in.
    let device_id = normalize_device_id(request.device_id.as_deref());
    let response = establish_jwt_session(&pool, &config, &user, device_id.as_deref()).await?;

    // Baseline this account's country so the next login from elsewhere alerts.
    // `establish_jwt_session` baselined its device the same way.
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

    // Never gated by the LINKS-35 / LINKS-45 approval gate, for the same reason
    // as setup: the account is being created, so it has neither a prior country
    // nor a recorded device.
    let device_id = normalize_device_id(request.device_id.as_deref());
    let response = establish_jwt_session(&pool, &config, &user, device_id.as_deref()).await?;

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

    let device_id = normalize_device_id(request.device_id.as_deref());

    // LINKS-35 / LINKS-45: the credential check passed, so this is the point
    // where a session would be issued and therefore where the gate belongs.
    // With the gate on and either the country or the device new to this
    // account, nothing is issued: the sign-in is held until the owner approves
    // it from an emailed link. The LINKS-27 alert is not also fired, because
    // the approval mail reports the same event and additionally asks for a
    // decision, and neither `last_login_country` nor `known_devices` is written
    // so an unapproved attempt cannot look familiar next time.
    if config.mail.login_approval_enabled {
        // The third field is the per-user alert opt-out, deliberately unused: a
        // preference set from a session must not switch a security gate off.
        if let Some((email, previous, _opt_out)) = User::get_login_location(&pool, user.id).await? {
            let known = known_device_state(&pool, user.id, device_id.as_deref()).await?;
            if let Some(hold) = approval_hold(
                &config.mail,
                previous.as_deref(),
                &headers,
                known,
                device_id.as_deref(),
            ) {
                let device_hash = device_id.as_deref().map(hash_device_id);
                request_login_approval(
                    &pool,
                    &config,
                    user.id,
                    &email,
                    &hold,
                    device_hash.as_deref(),
                    &headers,
                )
                .await?;
                return Err(AppError::ApprovalRequired);
            }
        }
    }

    // Alert on a sign-in from a country this account has not used before.
    spawn_new_location_check(&pool, &config.mail, user.id, &headers);

    let response = establish_jwt_session(&pool, &config, &user, device_id.as_deref()).await?;

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
///
/// The row is found by the SHA-256 of what was presented, never by the value
/// itself (LINKS-59), and the `DELETE ... RETURNING` is what makes the rotation
/// single-use in the same way the LINKS-35 guarded claim does: two concurrent
/// refreshes both name the row, only one deletes it, and the loser gets no row
/// back rather than a second session from one token.
pub async fn refresh_handler(
    State(pool): State<PgPool>,
    State(config): State<Config>,
    Json(request): Json<RefreshRequest>,
) -> Result<impl IntoResponse, AppError> {
    let (user_id, expires_at) = sqlx::query_as::<_, (uuid::Uuid, chrono::DateTime<chrono::Utc>)>(
        "DELETE FROM refresh_tokens WHERE token_hash = $1 RETURNING user_id, expires_at",
    )
    .bind(hash_refresh_token(&request.refresh_token).as_slice())
    .fetch_optional(&pool)
    .await?
    .ok_or(AppError::SessionExpired)?;

    // An expired row is claimed and dropped rather than left to the sweep, so a
    // token that has run out cannot be presented twice.
    if expires_at < chrono::Utc::now() {
        return Err(AppError::SessionExpired);
    }

    // Load user
    let user = User::find_by_id(&pool, user_id)
        .await?
        .ok_or(AppError::SessionExpired)?;

    // Not gated by LINKS-35 / LINKS-45: this continues a session an earlier
    // completed sign-in already issued, it is not a sign-in of its own. No
    // device is recorded for the same reason: the sign-in that minted this
    // refresh token already recorded one.
    Ok(Json(
        establish_jwt_session(&pool, &config, &user, None).await?,
    ))
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

/// Build the `/me` payload for the session's account. Both the GET and the
/// PATCH answer with it, so a client sees one shape either way.
async fn current_user_info(
    state: &crate::api::AppState,
    auth_user: &AuthenticatedUser,
) -> Result<crate::server_functions::auth::UserInfo, AppError> {
    let user = User::find_by_id(&state.pool, auth_user.user_id)
        .await?
        .ok_or(AppError::SessionExpired)?;

    // Read the opt-out through the reader the alert path itself uses, so what
    // a client is shown is exactly what decides whether the mail goes out.
    let (_, _, notify_new_location) = User::get_login_location(&state.pool, auth_user.user_id)
        .await?
        .ok_or(AppError::SessionExpired)?;

    let maintenance_mode = state
        .maintenance_mode
        .load(std::sync::atomic::Ordering::Relaxed);

    Ok(crate::server_functions::auth::UserInfo {
        id: user.id.to_string(),
        email: user.email,
        name: user.name,
        is_admin: user.is_admin,
        maintenance_mode,
        auth_via_oidc: auth_user.auth_via_oidc,
        notify_new_location,
    })
}

/// GET /api/auth/me
///
/// Returns information about the currently authenticated user. The
/// [`AuthenticatedUser`] extractor resolves the session per mode at runtime
/// (JWT bearer in standalone, `rl_session` cookie or `at+jwt` bearer in
/// hosted). `maintenance_mode` is always reported (it only ever flips in
/// hosted mode) so admin UIs can show a banner, and `notify_new_location`
/// reports whether new-location sign-in alerts are on for this account.
pub async fn me_handler(
    State(state): State<crate::api::AppState>,
    auth_user: AuthenticatedUser,
) -> Result<Json<crate::server_functions::auth::UserInfo>, AppError> {
    let info = current_user_info(&state, &auth_user).await?;

    tracing::debug!(user_id = %info.id, "User info requested");

    Ok(Json(info))
}

/// PATCH /api/auth/me
///
/// Account settings for the signed-in user (LINKS-33): turn this account's
/// new-location sign-in alerts off or back on. Answers with the same payload
/// as the GET, so a client can render the saved state without a second call.
///
/// Semantics that matter here:
/// - The account is always the session's. The request body carries no id, so
///   one user can never flip another user's setting.
/// - An absent key means "not submitted" and leaves the stored value alone; an
///   explicit `false` persists. Every field this endpoint ever grows must keep
///   that rule, or a patch touching one setting would clobber another.
/// - A non-boolean is rejected rather than coerced into an accidental opt-out.
pub async fn update_me_handler(
    State(state): State<crate::api::AppState>,
    auth_user: AuthenticatedUser,
    request: Result<Json<crate::server_functions::auth::UpdateMeRequest>, JsonRejection>,
) -> Result<Json<crate::server_functions::auth::UserInfo>, AppError> {
    // Answer a malformed body in this crate's error envelope (400) instead of
    // axum's bare 422, so a client sees one error shape from this endpoint.
    let Json(request) = request.map_err(|rejection| AppError::Validation {
        field: "body".to_string(),
        message: rejection.body_text(),
    })?;

    let info = current_user_info(&state, &auth_user).await?;

    let desired =
        resolve_notify_new_location(info.notify_new_location, request.notify_new_location);

    if desired != info.notify_new_location {
        // The session's id is the only account this write can reach.
        if !User::set_notify_new_location(&state.pool, auth_user.user_id, desired).await? {
            return Err(AppError::SessionExpired);
        }
        tracing::info!(
            user_id = %auth_user.user_id,
            notify_new_location = desired,
            "Account settings updated"
        );
    }

    Ok(Json(crate::server_functions::auth::UserInfo {
        notify_new_location: desired,
        ..info
    }))
}

/// Header a client presents its device id in, so the device list can say which
/// row is "this browser".
///
/// A header rather than a query parameter so the id stays out of URLs, access
/// logs and `Referer`. It is NOT gated like the LINKS-31 forwarded headers,
/// and does not need to be: those assert network facts a client must not be
/// able to claim, while this one is a client-owned value by design, sent with
/// every sign-in already. Forging it can only mislabel a row as "this browser"
/// in the forger's own view of their own account, and it selects nothing:
/// revocation takes a row id, never a device id.
const DEVICE_ID_HEADER: &str = "X-Device-Id";

/// GET /api/auth/devices
///
/// The devices this account is recognised from (LINKS-55), so a user can see
/// which browsers satisfy the LINKS-45 trigger and revoke one that should not.
///
/// The account is the session's and appears in the WHERE clause, so the
/// response can only ever describe the caller. `device_id_hash` is never
/// returned and never even selected: the "this browser" flag is resolved by
/// comparing hashes inside the query.
pub async fn list_devices_handler(
    State(state): State<crate::api::AppState>,
    auth_user: AuthenticatedUser,
    headers: HeaderMap,
) -> Result<Json<Vec<crate::server_functions::auth::KnownDeviceInfo>>, AppError> {
    let current = normalize_device_id(
        headers
            .get(DEVICE_ID_HEADER)
            .and_then(|value| value.to_str().ok()),
    )
    .map(|id| hash_device_id(&id));

    let devices = list_for_user(&state.pool, auth_user.user_id, current.as_deref()).await?;

    tracing::debug!(
        user_id = %auth_user.user_id,
        devices = devices.len(),
        "Known devices listed"
    );

    Ok(Json(devices))
}

/// DELETE /api/auth/devices/{id}
///
/// Revoke one device this account is recognised from (LINKS-55).
///
/// Deliberately NOT password-confirmed, unlike the storefront equivalent
/// (SF-131) where `/account` is a password-confirmed zone. Three reasons, in
/// order of weight:
///
/// - Revocation cannot grant anything. It issues no session and relaxes no
///   check for the caller; it can only move the account toward being held MORE
///   often, or back to the zero-devices baseline it shipped at.
/// - The attacker a prompt would stop cannot use what it protects. Someone
///   holding a stolen session but not the password gains nothing from
///   disarming the device trigger, because they cannot sign in at all; someone
///   who does hold the password satisfies the prompt and is not stopped by it.
/// - It is unimplementable in hosted mode. An OIDC account's `password_hash`
///   is the sentinel `'!sso:no-password'` (see `auth::oidc_rp::jit`), so no
///   input could ever verify, and the control would be dead on a deployment
///   that can still list rows.
///
/// A prompt here would also be stricter than `PATCH /api/auth/me`, which
/// silences the new-location alert outright and is not confirmed either. The
/// session is the authority for both.
pub async fn revoke_device_handler(
    State(state): State<crate::api::AppState>,
    auth_user: AuthenticatedUser,
    Path(device_id): Path<uuid::Uuid>,
) -> Result<StatusCode, AppError> {
    // Ownership rides in the WHERE clause, so a row belonging to another
    // account matches nothing and is reported exactly as a row that never
    // existed. Nothing is read before the delete, so there is no window in
    // which another account's row is loaded at all.
    if !delete_for_user(&state.pool, device_id, auth_user.user_id).await? {
        return Err(AppError::NotFound {
            resource: "device".to_string(),
            id: device_id.to_string(),
        });
    }

    tracing::info!(
        user_id = %auth_user.user_id,
        device_id = %device_id,
        "Known device revoked"
    );

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, RwLock};
    use std::time::Duration;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt; // for `oneshot`
    use uuid::Uuid;

    use crate::auth::jwt::create_jwt;
    use crate::auth::oidc_rs::OidcVerifier;
    use crate::config::{Config, MailConfig, OidcConfig};

    /// Baseline config. A non-empty `issuer` selects hosted mode; empty is
    /// standalone. `database_url` points at a closed port on purpose: these
    /// tests assert what happens before any query runs, so a request that does
    /// reach the database fails fast and visibly instead of hanging.
    fn config_with_issuer(issuer: &str) -> Config {
        Config {
            database_url: "postgres://user:pass@127.0.0.1:1/rusty_links_test".to_string(),
            app_port: 4002,
            update_interval_days: 30,
            log_level: "info".to_string(),
            update_interval_hours: 24,
            batch_size: 50,
            jitter_percent: 20,
            host_url: "http://localhost:4002".to_string(),
            webhook_secret: "test-webhook-secret".to_string(),
            oidc: OidcConfig {
                issuer: issuer.to_string(),
                audience: "http://localhost:4002/api".to_string(),
                jwks_url: String::new(),
                jwks_cache_ttl: 300,
                client_id: String::new(),
                client_secret: String::new(),
                redirect_uri: "http://localhost:4002/oauth2/callback".to_string(),
                post_logout_redirect_uri: "http://localhost:4002/".to_string(),
                leeway_seconds: 30,
                lifecycle_jti_cache_ttl: 300,
                session_ttl_seconds: 1_209_600,
            },
            jwt_secret: "test_secret".to_string(),
            jwt_expiry_hours: 1,
            refresh_token_expiry_days: 7,
            account_lockout_attempts: 5,
            account_lockout_duration_minutes: 30,
            allow_registration: true,
            mail: MailConfig::default(),
            trusted_proxy_cidrs: Vec::new(),
        }
    }

    fn api_router(config: Config) -> axum::Router {
        let pool = PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(100))
            .connect_lazy(&config.database_url)
            .expect("lazy pool");
        let verifier = Arc::new(OidcVerifier::new(config.oidc.clone()));
        crate::api::create_router(
            pool,
            config,
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
            Arc::new(RwLock::new(None)),
            verifier,
        )
    }

    /// A standalone-mode session for `user_id`.
    fn session_token(config: &Config, user_id: Uuid) -> String {
        create_jwt(
            "alice@example.com",
            user_id,
            false,
            &config.jwt_secret,
            config.jwt_expiry_hours,
        )
        .expect("jwt")
    }

    async fn patch_me(
        router: axum::Router,
        token: Option<&str>,
        body: serde_json::Value,
    ) -> StatusCode {
        let mut builder = Request::builder()
            .method("PATCH")
            .uri("/auth/me")
            .header("Content-Type", "application/json");
        if let Some(token) = token {
            builder = builder.header("Authorization", format!("Bearer {token}"));
        }
        let request = builder.body(Body::from(body.to_string())).unwrap();
        router.oneshot(request).await.unwrap().status()
    }

    // The settings pair is account-owned, so both halves are mounted in both
    // deployment modes (not a 404 routing miss).
    #[tokio::test]
    async fn me_is_readable_and_patchable_in_both_modes() {
        for issuer in ["", "https://issuer.example"] {
            for method in ["GET", "PATCH"] {
                let request = Request::builder()
                    .method(method)
                    .uri("/auth/me")
                    .header("Content-Type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap();
                let status = api_router(config_with_issuer(issuer))
                    .oneshot(request)
                    .await
                    .unwrap()
                    .status();
                assert_ne!(
                    status,
                    StatusCode::NOT_FOUND,
                    "{method} /auth/me should be mounted with issuer {issuer:?}"
                );
            }
        }
    }

    // No session, no write: the endpoint never acts on an unauthenticated
    // request, so there is no path to another account's row.
    #[tokio::test]
    async fn patch_me_without_a_session_is_unauthorized() {
        let config = config_with_issuer("");
        let status = patch_me(
            api_router(config),
            None,
            serde_json::json!({"notify_new_location": false}),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    // A non-boolean is rejected outright rather than coerced into an
    // accidental opt-out, and the request never reaches the database.
    #[tokio::test]
    async fn patch_me_rejects_a_non_boolean_flag() {
        let config = config_with_issuer("");
        let token = session_token(&config, Uuid::new_v4());
        for bad in [
            serde_json::json!({"notify_new_location": "false"}),
            serde_json::json!({"notify_new_location": 0}),
        ] {
            let status = patch_me(api_router(config_with_issuer("")), Some(&token), bad).await;
            assert_eq!(status, StatusCode::BAD_REQUEST);
        }
    }

    // The write targets the session's account. An id in the body is inert: it
    // does not survive parsing (see the `UpdateMeRequest` tests), so the
    // request is accepted on its merits and the only row the handler goes on to
    // touch is the session user's. With the database out of reach that shows up
    // as a 500 rather than the 400/401 a rejected request would give, which is
    // what proves the body id was neither honored nor fatal.
    #[tokio::test]
    async fn patch_me_ignores_an_account_id_in_the_body() {
        let config = config_with_issuer("");
        let session_user = Uuid::new_v4();
        let other_user = Uuid::new_v4();
        let token = session_token(&config, session_user);

        let status = patch_me(
            api_router(config),
            Some(&token),
            serde_json::json!({
                "notify_new_location": false,
                "user_id": other_user.to_string(),
                "id": other_user.to_string(),
                "email": "someone-else@example.com",
            }),
        )
        .await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }
}
