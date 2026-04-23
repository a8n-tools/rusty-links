//! OIDC Relying Party (BFF) — Authorization Code + PKCE flow for browser clients.
//!
//! Routes:
//! - `GET  /oauth2/login`               — start the OIDC auth flow
//! - `GET  /oauth2/callback`            — exchange code for tokens, create session
//! - `GET  /oauth2/logout`              — RP-initiated logout
//! - `POST /oauth2/backchannel-logout`  — receive OIDC Back-Channel Logout tokens
//! - `POST /oauth2/lifecycle-event`     — receive SaaS user lifecycle events

pub mod jit;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::Utc;
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use time::Duration as CookieDuration;
use uuid::Uuid;

use crate::auth::oidc_rs::OidcVerifier;
use crate::config::OidcConfig;
use crate::error::AppError;

// ── Sub-state for this router ─────────────────────────────────────────────────

#[derive(Clone)]
pub struct OidcRpState {
    pub pool: PgPool,
    pub config: OidcConfig,
    pub verifier: Arc<OidcVerifier>,
    pub jti_cache: Arc<moka::future::Cache<String, ()>>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn random_b64url(n: usize) -> String {
    let mut buf = vec![0u8; n];
    rand::rng().fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(&buf)
}

fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn hash_session_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

fn build_session_cookie(token: &str, ttl_seconds: u64, secure: bool) -> Cookie<'static> {
    Cookie::build(("rl_session", token.to_string()))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(CookieDuration::seconds(ttl_seconds as i64))
        .build()
}

fn clear_session_cookie(secure: bool) -> Cookie<'static> {
    Cookie::build(("rl_session", ""))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(CookieDuration::ZERO)
        .build()
}

// ── Query / form parameter types ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct LoginQuery {
    pub return_to: Option<String>,
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Deserialize)]
pub struct BackchannelLogoutForm {
    pub logout_token: String,
}

#[derive(Deserialize)]
pub struct LifecycleEventForm {
    pub lifecycle_event: String,
}

// ── Token exchange response ───────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct TokenResponse {
    #[allow(dead_code)]
    access_token: String,
    id_token: String,
    #[allow(dead_code)]
    refresh_token: Option<String>,
    #[allow(dead_code)]
    expires_in: Option<u64>,
}

#[derive(serde::Deserialize)]
struct TokenErrorResponse {
    error: String,
    error_description: Option<String>,
}

// ── Row types for runtime queries ─────────────────────────────────────────────

struct RpSessionRow {
    id: Uuid,
    nonce: String,
    code_verifier: String,
    return_to: Option<String>,
    expires_at: chrono::DateTime<Utc>,
}

struct UserSessionRow {
    user_id: Uuid,
    session_version: i32,
    expires_at: chrono::DateTime<Utc>,
    user_session_version: i32,
    suspended_at: Option<chrono::DateTime<Utc>>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// `GET /oauth2/login`
///
/// Generates PKCE materials, stores them in `rp_sessions`, and redirects the
/// browser to the SaaS authorization endpoint.
pub async fn login(
    State(state): State<OidcRpState>,
    Query(params): Query<LoginQuery>,
) -> Result<Response, AppError> {
    if !state.config.enabled() {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }

    let pkce_state = random_b64url(32);
    let nonce = random_b64url(32);
    let code_verifier = random_b64url(43);
    let code_challenge = pkce_challenge(&code_verifier);

    let session_id = Uuid::new_v4();
    let expires_at = Utc::now() + chrono::Duration::minutes(10);

    sqlx::query(
        "INSERT INTO rp_sessions (id, state, nonce, code_verifier, return_to, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(session_id)
    .bind(&pkce_state)
    .bind(&nonce)
    .bind(&code_verifier)
    .bind(&params.return_to)
    .bind(expires_at)
    .execute(&state.pool)
    .await?;

    let issuer = state.config.issuer.trim_end_matches('/');
    let scopes = "openid email offline_access";
    let auth_url = format!(
        "{issuer}/oauth2/authorize\
?response_type=code\
&client_id={client_id}\
&redirect_uri={redirect_uri}\
&scope={scopes}\
&state={state_param}\
&nonce={nonce}\
&code_challenge={challenge}\
&code_challenge_method=S256",
        client_id = urlencoding::encode(&state.config.client_id),
        redirect_uri = urlencoding::encode(&state.config.redirect_uri),
        scopes = urlencoding::encode(scopes),
        state_param = urlencoding::encode(&pkce_state),
        nonce = urlencoding::encode(&nonce),
        challenge = urlencoding::encode(&code_challenge),
    );

    Ok(Redirect::to(&auth_url).into_response())
}

/// `GET /oauth2/callback`
///
/// Exchanges the authorization code for tokens, validates the ID token,
/// JIT-provisions the local user, and sets the `rl_session` cookie.
pub async fn callback(
    State(state): State<OidcRpState>,
    jar: CookieJar,
    Query(params): Query<CallbackQuery>,
) -> Result<Response, AppError> {
    if !state.config.enabled() {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }

    // Surface IdP errors.
    if let Some(err) = &params.error {
        let desc = params.error_description.as_deref().unwrap_or(err.as_str());
        tracing::warn!(error = %err, description = %desc, "IdP returned error at callback");
        let redirect = format!(
            "/login?error={}&error_description={}",
            urlencoding::encode(err),
            urlencoding::encode(desc),
        );
        return Ok(Redirect::to(&redirect).into_response());
    }

    let code = params
        .code
        .as_deref()
        .ok_or_else(|| AppError::Internal("Missing 'code' parameter in callback".into()))?;
    let state_param = params
        .state
        .as_deref()
        .ok_or_else(|| AppError::Internal("Missing 'state' parameter in callback".into()))?;

    // Look up and validate the PKCE session.
    let rp_session = sqlx::query_as::<_, (Uuid, String, String, Option<String>, chrono::DateTime<Utc>)>(
        "SELECT id, nonce, code_verifier, return_to, expires_at
         FROM rp_sessions WHERE state = $1",
    )
    .bind(state_param)
    .fetch_optional(&state.pool)
    .await?
    .map(|(id, nonce, code_verifier, return_to, expires_at)| RpSessionRow {
        id,
        nonce,
        code_verifier,
        return_to,
        expires_at,
    })
    .ok_or_else(|| AppError::Internal("Unknown or expired state parameter".into()))?;

    if rp_session.expires_at < Utc::now() {
        sqlx::query("DELETE FROM rp_sessions WHERE id = $1")
            .bind(rp_session.id)
            .execute(&state.pool)
            .await
            .ok();
        return Err(AppError::Internal(
            "Login session expired; please try again".into(),
        ));
    }

    let return_to = rp_session.return_to.clone();
    let nonce = rp_session.nonce.clone();
    let code_verifier = rp_session.code_verifier.clone();

    // Exchange the authorization code for tokens.
    let token_url = format!(
        "{}/oauth2/token",
        state.config.issuer.trim_end_matches('/')
    );
    let resp = state
        .verifier
        .http
        .post(&token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", state.config.redirect_uri.as_str()),
            ("client_id", state.config.client_id.as_str()),
            ("client_secret", state.config.client_secret.as_str()),
            ("code_verifier", code_verifier.as_str()),
        ])
        .send()
        .await
        .map_err(|e| AppError::ExternalService(format!("Token endpoint request failed: {e}")))?;

    if !resp.status().is_success() {
        let err: TokenErrorResponse = resp.json().await.unwrap_or(TokenErrorResponse {
            error: "server_error".into(),
            error_description: None,
        });
        tracing::warn!(error = %err.error, "Token endpoint returned error");
        return Err(AppError::Internal(format!(
            "Token exchange failed: {}",
            err.error_description.unwrap_or(err.error)
        )));
    }

    let tokens: TokenResponse = resp
        .json()
        .await
        .map_err(|e| AppError::ExternalService(format!("Failed to parse token response: {e}")))?;

    // Validate the ID token.
    let id_claims = state
        .verifier
        .verify_id_token(&tokens.id_token, &nonce)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "ID token validation failed");
            e
        })?;

    // JIT-provision (or load) the local user.
    let provisioned = match jit::load_or_provision(&state.pool, &id_claims).await {
        Ok(p) => p,
        Err(AppError::Forbidden(msg)) => {
            let redirect = format!(
                "/login?error=access_denied&error_description={}",
                urlencoding::encode(&msg),
            );
            return Ok(Redirect::to(&redirect).into_response());
        }
        Err(e) => return Err(e),
    };

    // Delete the consumed PKCE session row.
    sqlx::query("DELETE FROM rp_sessions WHERE id = $1")
        .bind(rp_session.id)
        .execute(&state.pool)
        .await
        .ok();

    // Generate session token, hash it, and store in user_sessions.
    let session_token = random_b64url(32);
    let token_hash = hash_session_token(&session_token);
    let expires_at =
        Utc::now() + chrono::Duration::seconds(state.config.session_ttl_seconds as i64);

    sqlx::query(
        "INSERT INTO user_sessions (session_token_hash, user_id, session_version, expires_at)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(token_hash.as_slice())
    .bind(provisioned.id)
    .bind(provisioned.session_version)
    .bind(expires_at)
    .execute(&state.pool)
    .await?;

    let secure = state.config.redirect_uri.starts_with("https://");
    let cookie = build_session_cookie(&session_token, state.config.session_ttl_seconds, secure);
    let jar = jar.add(cookie);

    let destination = return_to
        .filter(|s: &String| s.starts_with('/'))
        .unwrap_or_else(|| "/links".to_string());

    Ok((jar, Redirect::to(&destination)).into_response())
}

/// `GET /oauth2/logout`
///
/// Clears the local session and redirects to the SaaS `end_session_endpoint`.
pub async fn logout(
    State(state): State<OidcRpState>,
    jar: CookieJar,
) -> Result<Response, AppError> {
    if !state.config.enabled() {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }

    if let Some(cookie) = jar.get("rl_session") {
        let token_hash = hash_session_token(cookie.value());
        sqlx::query("DELETE FROM user_sessions WHERE session_token_hash = $1")
            .bind(token_hash.as_slice())
            .execute(&state.pool)
            .await
            .ok();
    }

    let secure = state.config.redirect_uri.starts_with("https://");
    let cleared = clear_session_cookie(secure);
    let jar = jar.remove(cleared);

    let logout_url = if state.config.post_logout_redirect_uri.is_empty() {
        format!(
            "{}/oauth2/logout",
            state.config.issuer.trim_end_matches('/')
        )
    } else {
        format!(
            "{}/oauth2/logout?post_logout_redirect_uri={}",
            state.config.issuer.trim_end_matches('/'),
            urlencoding::encode(&state.config.post_logout_redirect_uri),
        )
    };

    Ok((jar, Redirect::to(&logout_url)).into_response())
}

/// `POST /oauth2/backchannel-logout`
///
/// Receives an OIDC Back-Channel Logout Token and increments `session_version`
/// for the matching user, instantly invalidating all existing sessions.
///
/// Returns 200 even when no user is found (per OIDC Back-Channel Logout 1.0 §2.5).
pub async fn backchannel_logout(
    State(state): State<OidcRpState>,
    Form(form): Form<BackchannelLogoutForm>,
) -> Result<Response, AppError> {
    if !state.config.enabled() {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }

    let claims = match state.verifier.verify_logout_token(&form.logout_token).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "Back-channel logout token rejected");
            return Ok(StatusCode::BAD_REQUEST.into_response());
        }
    };

    if let Some(sub) = &claims.sub {
        if let Ok(saas_uuid) = sub.parse::<Uuid>() {
            let result = sqlx::query(
                "UPDATE users SET session_version = session_version + 1
                 WHERE saas_user_id = $1",
            )
            .bind(saas_uuid)
            .execute(&state.pool)
            .await;

            match result {
                Ok(r) if r.rows_affected() > 0 => {
                    tracing::info!(saas_user_id = %saas_uuid, "Back-channel logout: session_version incremented");
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Back-channel logout: DB update failed");
                }
                _ => {}
            }
        }
    }

    Ok(StatusCode::OK.into_response())
}

/// `POST /oauth2/lifecycle-event`
///
/// Receives a signed lifecycle-event token from the SaaS OP and applies
/// the appropriate state change.
pub async fn lifecycle_event(
    State(state): State<OidcRpState>,
    Form(form): Form<LifecycleEventForm>,
) -> Result<Response, AppError> {
    if !state.config.enabled() {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }

    let claims = match state.verifier.verify_lifecycle_token(&form.lifecycle_event).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "Lifecycle event token rejected");
            return Ok(StatusCode::BAD_REQUEST.into_response());
        }
    };

    // Idempotency: skip if we've already processed this jti.
    if state.jti_cache.get(&claims.jti).await.is_some() {
        tracing::debug!(jti = %claims.jti, "Lifecycle event already processed; ignoring");
        return Ok(StatusCode::OK.into_response());
    }

    let event = match claims.lifecycle_event() {
        Some(e) => e.clone(),
        None => {
            tracing::debug!(jti = %claims.jti, "Lifecycle event with unknown schema; ignoring");
            return Ok(StatusCode::OK.into_response());
        }
    };

    let subject_id = match event.subject.id.parse::<Uuid>() {
        Ok(u) => u,
        Err(_) => {
            tracing::warn!(subject = %event.subject.id, "Lifecycle event subject is not a UUID");
            return Ok(StatusCode::OK.into_response());
        }
    };

    match event.event_type.as_str() {
        "user.suspended" => {
            sqlx::query(
                "UPDATE users SET suspended_at = NOW(), session_version = session_version + 1
                 WHERE saas_user_id = $1",
            )
            .bind(subject_id)
            .execute(&state.pool)
            .await?;
        }
        "user.unsuspended" => {
            sqlx::query("UPDATE users SET suspended_at = NULL WHERE saas_user_id = $1")
                .bind(subject_id)
                .execute(&state.pool)
                .await?;
        }
        "user.deleted" => {
            sqlx::query("DELETE FROM users WHERE saas_user_id = $1")
                .bind(subject_id)
                .execute(&state.pool)
                .await?;
        }
        "entitlement.revoked" => {
            sqlx::query(
                "UPDATE users SET session_version = session_version + 1
                 WHERE saas_user_id = $1",
            )
            .bind(subject_id)
            .execute(&state.pool)
            .await?;
        }
        "entitlement.granted" => {
            // No-op: the next login will succeed.
        }
        unknown => {
            tracing::debug!(event_type = %unknown, jti = %claims.jti, "Unknown lifecycle event type; ignoring");
        }
    }

    state.jti_cache.insert(claims.jti.clone(), ()).await;

    tracing::info!(
        jti = %claims.jti,
        event_type = %event.event_type,
        subject = %subject_id,
        "Lifecycle event processed"
    );

    Ok(StatusCode::OK.into_response())
}

// ── Session resolution helper (used by middleware and server functions) ───────

/// Look up a user by raw session cookie value.  Returns `None` if the session
/// is missing, expired, or has been invalidated via `session_version`.
pub async fn get_user_from_session(
    pool: &PgPool,
    session_token: &str,
) -> Result<Option<Uuid>, AppError> {
    let token_hash = hash_session_token(session_token);

    let row = sqlx::query_as::<_, (Uuid, i32, chrono::DateTime<Utc>, i32, Option<chrono::DateTime<Utc>>)>(
        "SELECT us.user_id, us.session_version, us.expires_at,
                u.session_version AS user_session_version, u.suspended_at
         FROM user_sessions us
         JOIN users u ON u.id = us.user_id
         WHERE us.session_token_hash = $1",
    )
    .bind(token_hash.as_slice())
    .fetch_optional(pool)
    .await?
    .map(|(user_id, session_version, expires_at, user_session_version, suspended_at)| UserSessionRow {
        user_id,
        session_version,
        expires_at,
        user_session_version,
        suspended_at,
    });

    match row {
        None => Ok(None),
        Some(r) => {
            if r.expires_at < Utc::now() {
                return Ok(None);
            }
            if r.session_version != r.user_session_version {
                return Ok(None);
            }
            if r.suspended_at.is_some() {
                return Ok(None);
            }
            Ok(Some(r.user_id))
        }
    }
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn create_router(
    pool: PgPool,
    config: OidcConfig,
    verifier: Arc<OidcVerifier>,
) -> Router {
    let jti_cache = Arc::new(
        moka::future::Cache::builder()
            .time_to_live(Duration::from_secs(config.lifecycle_jti_cache_ttl))
            .build(),
    );

    let state = OidcRpState {
        pool,
        config,
        verifier,
        jti_cache,
    };

    Router::new()
        .route("/oauth2/login", get(login))
        .route("/oauth2/callback", get(callback))
        .route("/oauth2/logout", get(logout))
        .route("/oauth2/backchannel-logout", post(backchannel_logout))
        .route("/oauth2/lifecycle-event", post(lifecycle_event))
        .with_state(state)
}
