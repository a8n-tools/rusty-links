//! LINKS-35: hold a sign-in from a new country until the account owner approves it.
//!
//! LINKS-27 detects a sign-in from a country the account has not used before
//! and emails an alert once the session is already issued. An attacker who
//! reads the mailbox can read that alert and delete it, so on its own it
//! reports the compromise instead of stopping it. This turns the same signal
//! into a gate: no JWT and no refresh token are issued, a
//! `pending_login_approvals` row is written, and the owner is emailed a
//! single-use link. It is notify-and-approve, not a lock: nothing about the
//! account is disabled and an attempt nobody approves simply never completes.
//!
//! Two sign-ins are NEVER gated, by construction, because gating either would
//! lock a real user out: a first-ever sign-in has no prior country for the new
//! one to differ from, and a sign-in whose country does not resolve (no
//! geoblock edge, or no `TRUSTED_PROXY_CIDRS`, which is the default) has
//! nothing to compare. Both fall out of [`is_new_country`], the one definition
//! of "suspicious" this crate has, which the LINKS-27 alert shares.
//!
//! Approving records the country as the account's known one, which is the same
//! write a completed sign-in makes, so the user signs in again and that sign-in
//! completes ungated. Nothing is recorded for an attempt nobody approves, so it
//! cannot make its country look familiar next time.

use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::{Form, Router};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Utc};
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::location_alert::is_new_country;
use crate::auth::middleware::{client_country, client_ip_from_headers, device_info};
use crate::config::{Config, MailConfig};
use crate::error::AppError;
use crate::models::User;

/// How long an emailed approval link stays claimable.
///
/// Approving means leaving the browser for a mail client and reading a message
/// that has to be delivered first, so this is deliberately longer than the
/// access-token lifetime while still keeping a link sitting in a mailbox from
/// becoming a durable credential.
pub const APPROVAL_TTL_MINUTES: i64 = 15;

/// Path the emailed link lands on, mounted at the server root in standalone
/// mode (see `main.rs`). Not under `/api`: a human opens it in a browser.
pub const APPROVAL_PATH: &str = "/auth/approve-login";

const APPROVAL_PAGE: &str = include_str!("approve_login.html");

// ── Decision ─────────────────────────────────────────────────────────────────

/// Whether a sign-in that already passed the credential check must be approved.
///
/// Delegates to [`is_new_country`] so the gate and the LINKS-27 alert can never
/// disagree, which is what keeps a first-ever sign-in and an unresolved country
/// out of both. Deliberately ignores `users.notify_new_location`: that
/// preference is written from an authenticated session, so honoring it here
/// would let anyone holding a session switch the security control off, and an
/// opted-out user would be gated with no mail to approve with.
pub fn should_require_approval(
    enabled: bool,
    previous: Option<&str>,
    current: Option<&str>,
) -> bool {
    enabled && is_new_country(previous, current)
}

/// The country a sign-in must be held on, from what the login route has in
/// hand. `None` means the sign-in completes as it does today.
///
/// The country comes from the LINKS-31 peer-gated header reader, never from the
/// raw request, so a direct client can neither forge a country nor suppress one.
pub fn approval_country(
    mail: &MailConfig,
    previous: Option<&str>,
    headers: &HeaderMap,
) -> Option<String> {
    let current = client_country(headers)?;
    should_require_approval(mail.login_approval_enabled, previous, Some(&current))
        .then_some(current)
}

// ── Token ────────────────────────────────────────────────────────────────────

/// A 256-bit approval token. Only its hash is ever stored.
fn generate_approval_token() -> String {
    let mut buf = [0u8; 32];
    rand::rng().fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

/// SHA-256 of an approval token, as stored in `token_hash`. Matches how
/// `oidc_rp` stores session tokens, so a database dump yields nothing
/// replayable.
fn hash_approval_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

/// The link emailed to the user, which lands on the approval page.
fn build_approval_url(host_url: &str, token: &str) -> String {
    format!(
        "{}{APPROVAL_PATH}?token={}",
        host_url.trim_end_matches('/'),
        urlencoding::encode(token),
    )
}

// ── Storage ──────────────────────────────────────────────────────────────────

/// What the approval page shows before the user commits to approving.
#[derive(Debug, Clone)]
pub struct PendingApproval {
    pub country: String,
    pub ip: String,
    pub device: Option<String>,
    pub requested_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Why a token cannot be acted on. Each maps to its own page message so a user
/// can tell an expired link from one they already used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalFailure {
    NotFound,
    AlreadyUsed,
    Expired,
}

impl ApprovalFailure {
    fn message(self) -> &'static str {
        match self {
            Self::NotFound => "This approval link is not valid. Sign in again to get a new one.",
            Self::AlreadyUsed => "This approval link has already been used. Each link works once.",
            Self::Expired => "This approval link has expired. Sign in again to get a new one.",
        }
    }
}

/// A token read without being consumed.
pub enum ApprovalLookup {
    Valid(PendingApproval),
    Invalid(ApprovalFailure),
}

/// A token the approval POST tried to claim.
pub enum ApprovalClaim {
    Claimed { user_id: Uuid, country: String },
    Rejected(ApprovalFailure),
}

/// Drop rows past their expiry. Best effort: a leftover row is inert because
/// every read is guarded on `expires_at` anyway.
async fn sweep_expired_approvals(pool: &PgPool) {
    if let Err(error) = sqlx::query("DELETE FROM pending_login_approvals WHERE expires_at < NOW()")
        .execute(pool)
        .await
    {
        tracing::warn!(error = %error, "LINKS-35: pending approval cleanup failed");
    }
}

/// Say which of the three failures a token that could not be claimed hit.
async fn classify_failure(pool: &PgPool, token_hash: &[u8]) -> Result<ApprovalFailure, AppError> {
    let row = sqlx::query_as::<_, (Option<DateTime<Utc>>,)>(
        "SELECT consumed_at FROM pending_login_approvals WHERE token_hash = $1",
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await?;

    Ok(match row {
        None => ApprovalFailure::NotFound,
        Some((Some(_),)) => ApprovalFailure::AlreadyUsed,
        Some((None,)) => ApprovalFailure::Expired,
    })
}

/// Hold a sign-in and email the user a single-use approval link.
///
/// Runs ON the login hot path, unlike the LINKS-27 alert: nothing is issued
/// until the row exists and the mail is away, and a delivery failure propagates
/// so the sign-in fails closed rather than completing ungated.
pub async fn request_login_approval(
    pool: &PgPool,
    config: &Config,
    user_id: Uuid,
    email: &str,
    country: &str,
    headers: &HeaderMap,
) -> Result<(), AppError> {
    sweep_expired_approvals(pool).await;

    // One live link per user per country, so retrying the sign-in while the
    // first link is still claimable does not send a second mail.
    let live: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM pending_login_approvals
         WHERE user_id = $1 AND country = $2 AND consumed_at IS NULL AND expires_at > NOW()
         LIMIT 1",
    )
    .bind(user_id)
    .bind(country)
    .fetch_optional(pool)
    .await?;

    if live.is_some() {
        tracing::info!(
            user_id = %user_id,
            country = %country,
            "LINKS-35: sign-in held; a live approval link already exists"
        );
        return Ok(());
    }

    let token = generate_approval_token();
    let expires_at = Utc::now() + chrono::Duration::minutes(APPROVAL_TTL_MINUTES);
    let ip = client_ip_from_headers(headers).unwrap_or_else(|| "unknown".to_string());
    let device = device_info(headers);

    // The row goes in before the mail: a row nobody can reach expires on its
    // own, while a link with no row behind it is dead on arrival.
    sqlx::query(
        "INSERT INTO pending_login_approvals (user_id, token_hash, country, ip, device, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(user_id)
    .bind(hash_approval_token(&token).as_slice())
    .bind(country)
    .bind(&ip)
    .bind(&device)
    .bind(expires_at)
    .execute(pool)
    .await?;

    tracing::warn!(
        user_id = %user_id,
        country = %country,
        "LINKS-35: sign-in from a previously unseen country held for approval"
    );

    crate::auth::mailer::send_login_approval_email(
        &config.mail,
        email,
        country,
        &ip,
        device.as_deref(),
        &build_approval_url(&config.host_url, &token),
        APPROVAL_TTL_MINUTES,
    )
    .await
}

/// Read a pending approval without consuming it, so a link preview is safe.
pub async fn get_login_approval(pool: &PgPool, token: &str) -> Result<ApprovalLookup, AppError> {
    let token_hash = hash_approval_token(token);

    let row = sqlx::query_as::<
        _,
        (
            String,
            String,
            Option<String>,
            DateTime<Utc>,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
        ),
    >(
        "SELECT country, ip, device, created_at, expires_at, consumed_at
         FROM pending_login_approvals WHERE token_hash = $1",
    )
    .bind(token_hash.as_slice())
    .fetch_optional(pool)
    .await?;

    let Some((country, ip, device, requested_at, expires_at, consumed_at)) = row else {
        return Ok(ApprovalLookup::Invalid(ApprovalFailure::NotFound));
    };
    if consumed_at.is_some() {
        return Ok(ApprovalLookup::Invalid(ApprovalFailure::AlreadyUsed));
    }
    if expires_at <= Utc::now() {
        return Ok(ApprovalLookup::Invalid(ApprovalFailure::Expired));
    }

    Ok(ApprovalLookup::Valid(PendingApproval {
        country,
        ip,
        device,
        requested_at,
        expires_at,
    }))
}

/// Claim a pending approval, once.
///
/// The guarded UPDATE is what decides the race: two concurrent clicks both see
/// an unconsumed row, but only one matches and the loser gets no row back. The
/// user id comes off the claimed row and never from the caller, so a token can
/// only ever approve its own user's sign-in.
pub async fn consume_login_approval(pool: &PgPool, token: &str) -> Result<ApprovalClaim, AppError> {
    let token_hash = hash_approval_token(token);

    let claimed = sqlx::query_as::<_, (Uuid, String)>(
        "UPDATE pending_login_approvals SET consumed_at = NOW()
         WHERE token_hash = $1 AND consumed_at IS NULL AND expires_at > NOW()
         RETURNING user_id, country",
    )
    .bind(token_hash.as_slice())
    .fetch_optional(pool)
    .await?;

    match claimed {
        Some((user_id, country)) => Ok(ApprovalClaim::Claimed { user_id, country }),
        None => Ok(ApprovalClaim::Rejected(
            classify_failure(pool, &token_hash).await?,
        )),
    }
}

// ── Page rendering ───────────────────────────────────────────────────────────

/// Escape text before it goes into the page. The IP and the device string are
/// client-influenced, so this is what keeps them out of the markup.
fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn render(
    status: StatusCode,
    heading: &str,
    message: &str,
    details: &str,
    action: &str,
) -> Response {
    let body = APPROVAL_PAGE
        .replace("{{TITLE}}", &format!("Rusty Links - {heading}"))
        .replace("{{HEADING}}", heading)
        .replace("{{MESSAGE}}", message)
        .replace("{{DETAILS}}", details)
        .replace("{{ACTION}}", action);
    (status, Html(body)).into_response()
}

fn failure_page(failure: ApprovalFailure) -> Response {
    render(
        StatusCode::BAD_REQUEST,
        "Approval link unusable",
        failure.message(),
        "",
        "",
    )
}

fn pending_page(token: &str, pending: &PendingApproval) -> Response {
    let details = format!(
        "<dl class=\"approval-details\">\
         <dt>Country</dt><dd>{country}</dd>\
         <dt>IP address</dt><dd>{ip}</dd>\
         <dt>Device</dt><dd>{device}</dd>\
         <dt>Requested</dt><dd>{requested}</dd>\
         <dt>Link expires</dt><dd>{expires}</dd>\
         </dl>",
        country = escape_html(&pending.country),
        ip = escape_html(&pending.ip),
        device = escape_html(pending.device.as_deref().unwrap_or("unknown")),
        requested = escape_html(&pending.requested_at.to_rfc3339()),
        expires = escape_html(&pending.expires_at.to_rfc3339()),
    );
    // Approving is a POST behind a button: mail gateways and link scanners
    // fetch URLs out of messages, and a GET that claimed the token would burn
    // the only link before the user ever saw it.
    let action = format!(
        "<form method=\"post\" action=\"{path}\">\
         <input type=\"hidden\" name=\"token\" value=\"{token}\">\
         <button class=\"approval-button\" type=\"submit\">Approve this sign-in</button>\
         </form>",
        path = APPROVAL_PATH,
        token = escape_html(token),
    );
    render(
        StatusCode::OK,
        "Approve this sign-in?",
        "Someone signed in to your Rusty Links account from a country the account has not been used from before. Approve it only if it was you.",
        &details,
        &action,
    )
}

// ── Handlers ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TokenQuery {
    #[serde(default)]
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct TokenForm {
    #[serde(default)]
    pub token: String,
}

/// `GET /auth/approve-login?token=...`
///
/// Validates the token and shows what is being approved. Deliberately does not
/// consume it, so a link scanner opening the URL cannot burn it.
pub async fn approve_login_page(
    State(pool): State<PgPool>,
    Query(query): Query<TokenQuery>,
) -> Result<Response, AppError> {
    if query.token.is_empty() {
        return Ok(failure_page(ApprovalFailure::NotFound));
    }

    Ok(match get_login_approval(&pool, &query.token).await? {
        ApprovalLookup::Valid(pending) => pending_page(&query.token, &pending),
        ApprovalLookup::Invalid(failure) => failure_page(failure),
    })
}

/// `POST /auth/approve-login`
///
/// Claims the token, once, and records the approved country as the account's
/// known one. That is the same write a completed sign-in makes, so the user's
/// next sign-in from there is no longer new and completes ungated.
pub async fn approve_login_submit(
    State(pool): State<PgPool>,
    Form(form): Form<TokenForm>,
) -> Result<Response, AppError> {
    if form.token.is_empty() {
        return Ok(failure_page(ApprovalFailure::NotFound));
    }

    Ok(match consume_login_approval(&pool, &form.token).await? {
        ApprovalClaim::Claimed { user_id, country } => {
            User::update_last_login_country(&pool, user_id, &country).await?;
            tracing::info!(
                user_id = %user_id,
                country = %country,
                "LINKS-35: sign-in approved"
            );
            render(
                StatusCode::OK,
                "Sign-in approved",
                "Go back to Rusty Links and sign in again. This link cannot be used a second time.",
                "",
                "",
            )
        }
        ApprovalClaim::Rejected(failure) => failure_page(failure),
    })
}

/// The approval page router, mounted at the server root in standalone mode.
///
/// Not mounted in hosted mode: the gate only covers this service's own
/// credential login, so no hosted deployment can ever hold a sign-in.
pub fn create_router(pool: PgPool) -> Router {
    Router::new()
        .route(
            APPROVAL_PATH,
            get(approve_login_page).post(approve_login_submit),
        )
        .with_state(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, RwLock};
    use std::time::Duration;

    use axum::body::Body;
    use axum::http::Request;
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt; // for `oneshot`

    use crate::auth::oidc_rs::OidcVerifier;
    use crate::config::{OidcConfig, SmtpTlsMode};

    fn mail(enabled: bool) -> MailConfig {
        MailConfig {
            login_approval_enabled: enabled,
            ..MailConfig::default()
        }
    }

    fn headers_with_country(country: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(country) = country {
            headers.insert("X-IPCountry", country.parse().unwrap());
        }
        headers
    }

    // ── The gate decision ────────────────────────────────────────────────

    // A country the account has not used before is what the gate holds on.
    #[test]
    fn a_new_country_is_gated_when_enabled() {
        assert!(should_require_approval(true, Some("US"), Some("DE")));
    }

    // A first-ever sign-in has no prior country to differ from, so it is never
    // gated. `setup_handler` creates the first user, so this is not academic:
    // gating it would mean the first user could never sign in.
    #[test]
    fn a_first_ever_sign_in_is_never_gated() {
        assert!(!should_require_approval(true, None, Some("US")));
        assert!(!should_require_approval(true, None, None));
    }

    // An unresolved country (no geoblock edge, no trusted proxy, which is the
    // default) has nothing to compare, so it is never gated.
    #[test]
    fn an_unresolved_country_is_never_gated() {
        assert!(!should_require_approval(true, Some("US"), None));
    }

    // A repeat from the same country is not a change, case-insensitively.
    #[test]
    fn the_same_country_is_never_gated() {
        assert!(!should_require_approval(true, Some("US"), Some("US")));
        assert!(!should_require_approval(true, Some("us"), Some("US")));
    }

    // The kill switch is the outer guard: with it off nothing is ever held,
    // including a real country change.
    #[test]
    fn the_kill_switch_off_gates_nothing() {
        assert!(!should_require_approval(false, Some("US"), Some("DE")));
        assert!(!should_require_approval(false, None, Some("US")));
    }

    // The same two never-gated cases through the header reader the route
    // actually calls, not only through the pure predicate.
    #[test]
    fn header_level_first_sign_in_and_unresolved_country_are_never_gated() {
        // First-ever: the edge resolved a country, but the account has none.
        assert_eq!(
            approval_country(&mail(true), None, &headers_with_country(Some("DE"))),
            None
        );
        // Unresolved: no X-IPCountry at all, which is the current state of a
        // deployment with no geoblock edge.
        assert_eq!(
            approval_country(&mail(true), Some("US"), &headers_with_country(None)),
            None
        );
    }

    // With the gate on and both sides known, the header reader yields the
    // country to hold on, normalized the same way the alert normalizes it.
    #[test]
    fn header_level_new_country_is_gated() {
        assert_eq!(
            approval_country(&mail(true), Some("US"), &headers_with_country(Some("de"))),
            Some("DE".to_string())
        );
    }

    // The kill switch is checked before anything else the route would do.
    #[test]
    fn header_level_kill_switch_off_gates_nothing() {
        assert_eq!(
            approval_country(&mail(false), Some("US"), &headers_with_country(Some("DE"))),
            None
        );
    }

    // A forged X-IPCountry from an untrusted peer never reaches this reader
    // (LINKS-31 strips it upstream), and a malformed value is not a country.
    #[test]
    fn a_malformed_country_header_is_never_gated() {
        for bad in ["", "USA", "1A", "U"] {
            assert_eq!(
                approval_country(&mail(true), Some("US"), &headers_with_country(Some(bad))),
                None,
                "expected {bad:?} to resolve to no country"
            );
        }
    }

    // ── Token handling ───────────────────────────────────────────────────

    // Tokens are unguessable and unique per request.
    #[test]
    fn tokens_are_distinct_and_long() {
        let a = generate_approval_token();
        let b = generate_approval_token();
        assert_ne!(a, b);
        // 32 random bytes, base64url without padding.
        assert_eq!(a.len(), 43);
    }

    // Only the hash is ever stored, and it is not the token.
    #[test]
    fn the_stored_hash_is_not_the_token() {
        let token = generate_approval_token();
        let hash = hash_approval_token(&token);
        assert_eq!(hash.len(), 32);
        assert_ne!(hash, token.as_bytes());
        assert_eq!(hash, hash_approval_token(&token));
        assert_ne!(hash, hash_approval_token(&generate_approval_token()));
    }

    // The emailed link points at the mounted path on the configured host.
    #[test]
    fn the_approval_url_targets_the_mounted_path() {
        let url = build_approval_url("https://links.example.com/", "abc-123");
        assert_eq!(
            url,
            "https://links.example.com/auth/approve-login?token=abc-123"
        );
    }

    // ── Page rendering ───────────────────────────────────────────────────

    // The device string is client-controlled, so it is escaped, not injected.
    #[test]
    fn page_values_are_escaped() {
        let pending = PendingApproval {
            country: "DE".to_string(),
            ip: "203.0.113.7".to_string(),
            device: Some("<script>alert('x')</script>".to_string()),
            requested_at: Utc::now(),
            expires_at: Utc::now(),
        };
        let response = pending_page("tok&en", &pending);
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(escape_html("<script>"), "&lt;script&gt;");
        assert_eq!(escape_html("a&b\"c'd"), "a&amp;b&quot;c&#39;d");
    }

    // ── Route surface ────────────────────────────────────────────────────

    fn config_for_tests() -> Config {
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
                issuer: String::new(),
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
            mail: MailConfig {
                login_approval_enabled: true,
                smtp_tls: SmtpTlsMode::Starttls,
                ..MailConfig::default()
            },
            trusted_proxy_cidrs: Vec::new(),
        }
    }

    /// A pool pointed at a closed port on purpose: these tests assert what
    /// happens before any query runs, so a request that does reach the database
    /// fails fast and visibly instead of hanging.
    fn dead_pool(config: &Config) -> PgPool {
        PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(100))
            .connect_lazy(&config.database_url)
            .expect("lazy pool")
    }

    async fn approval_status(method: &str, uri: &str, body: Option<&str>) -> StatusCode {
        let config = config_for_tests();
        let router = create_router(dead_pool(&config));
        let mut builder = Request::builder().method(method).uri(uri);
        if body.is_some() {
            builder = builder.header("Content-Type", "application/x-www-form-urlencoded");
        }
        let request = builder
            .body(Body::from(body.unwrap_or("").to_string()))
            .unwrap();
        router.oneshot(request).await.unwrap().status()
    }

    // Both halves are mounted, and a request with no token is answered without
    // ever reaching the database.
    #[tokio::test]
    async fn a_missing_token_is_rejected_without_a_query() {
        assert_eq!(
            approval_status("GET", APPROVAL_PATH, None).await,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            approval_status("POST", APPROVAL_PATH, Some("token=")).await,
            StatusCode::BAD_REQUEST
        );
    }

    // A token that is not empty is never trusted on its face: the handler goes
    // to the database for it, which with the pool pointed nowhere surfaces as a
    // 500 rather than an approval page.
    #[tokio::test]
    async fn a_supplied_token_is_always_looked_up() {
        assert_eq!(
            approval_status("GET", &format!("{APPROVAL_PATH}?token=whatever"), None).await,
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            approval_status("POST", APPROVAL_PATH, Some("token=whatever")).await,
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    // ── The login route ──────────────────────────────────────────────────

    fn api_router(config: Config) -> Router {
        let pool = dead_pool(&config);
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

    async fn login_body(country: Option<&str>) -> (StatusCode, String) {
        let router = api_router(config_for_tests());
        let mut builder = Request::builder()
            .method("POST")
            .uri("/auth/login")
            .header("Content-Type", "application/json");
        if let Some(country) = country {
            builder = builder.header("X-IPCountry", country);
        }
        let request = builder
            .body(Body::from(
                serde_json::json!({"email": "alice@example.com", "password": "hunter2hunter2"})
                    .to_string(),
            ))
            .unwrap();
        let response = router.oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .unwrap();
        (status, String::from_utf8_lossy(&bytes).to_string())
    }

    // At the route, with the gate switched on, neither never-gated case can
    // produce an approval-required answer. The account's stored country lives
    // in the database, so the gate cannot fire before that row is read; a login
    // that never gets that far fails as an ordinary error instead.
    #[tokio::test]
    async fn the_login_route_never_holds_an_unresolved_or_first_ever_sign_in() {
        for country in [None, Some("DE")] {
            let (status, body) = login_body(country).await;
            assert_ne!(
                status,
                StatusCode::FORBIDDEN,
                "login with country {country:?} must not be held"
            );
            assert!(
                !body.contains("APPROVAL_REQUIRED"),
                "login with country {country:?} must not be held: {body}"
            );
        }
    }
}
