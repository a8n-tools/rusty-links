//! LINKS-35 / LINKS-45: hold a suspicious sign-in until the account owner approves it.
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
//! Two triggers feed the one gate, and either alone holds: a country the
//! account has not used before (LINKS-35) and a device it has not used before
//! (LINKS-45). One hold covers both, and approving it clears both, so a sign-in
//! that trips both is never held twice.
//!
//! Several sign-ins are NEVER gated, by construction, because gating any of
//! them would lock a real user out: a first-ever sign-in has no prior country
//! and no recorded device; a sign-in whose country does not resolve (no
//! geoblock edge, or no `TRUSTED_PROXY_CIDRS`, which is the default) has
//! nothing to compare; an account with no recorded device has no device
//! baseline, which is every account on the deploy that creates `known_devices`;
//! and a client that submits no device id is never held on the device signal.
//! All of them fall out of [`is_new_country`] and [`is_new_device`], the one
//! definition of "suspicious" this crate has, whose country half the LINKS-27
//! alert shares.
//!
//! Approving records the country as the account's known one and the submitted
//! device as one of the account's, which are the same writes a completed
//! sign-in makes, so the user signs in again and that sign-in completes
//! ungated. Nothing is recorded for an attempt nobody approves, so it cannot
//! make its country or its device look familiar next time.

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

use crate::auth::known_device::record_device;
use crate::auth::location_alert::{is_new_country, is_new_device};
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

/// Which trigger held a sign-in. Both can fire on one sign-in, and then one
/// hold covers both rather than the user being asked twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldReason {
    NewCountry,
    NewDevice,
    NewCountryAndDevice,
}

impl HoldReason {
    /// The value stored in `pending_login_approvals.reason`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NewCountry => "new_country",
            Self::NewDevice => "new_device",
            Self::NewCountryAndDevice => "new_country_and_device",
        }
    }

    /// Read the column back. An unrecognised value reads as the country hold,
    /// which is what every row written before LINKS-45 is.
    fn from_column(value: &str) -> Self {
        match value {
            "new_device" => Self::NewDevice,
            "new_country_and_device" => Self::NewCountryAndDevice,
            _ => Self::NewCountry,
        }
    }

    /// What the owner is told the sign-in was held on.
    pub fn summary(self) -> &'static str {
        match self {
            Self::NewCountry => {
                "from a country your account has not been used from before"
            }
            Self::NewDevice => {
                "from a device your account has not been used from before. Clearing site data, a private window and a different browser all look the same as a new machine here"
            }
            Self::NewCountryAndDevice => {
                "from a country and a device your account has not been used from before"
            }
        }
    }
}

/// Whether a sign-in that already passed the credential check must be approved,
/// and which trigger fired.
///
/// The OR of the two halves of [`crate::auth::location_alert`]'s one definition
/// of "suspicious", so the gate and the LINKS-27 alert can never disagree about
/// a country, and the gate can never disagree with itself about a device. Every
/// never-held case is a case where one of those two returns false, so none of
/// them is a special case here. Deliberately ignores `users.notify_new_location`:
/// that preference is written from an authenticated session, so honoring it here
/// would let anyone holding a session switch the security control off, and an
/// opted-out user would be gated with no mail to approve with.
pub fn approval_reason(
    enabled: bool,
    previous_country: Option<&str>,
    current_country: Option<&str>,
    known_device: Option<bool>,
    submitted_device: Option<&str>,
) -> Option<HoldReason> {
    if !enabled {
        return None;
    }

    match (
        is_new_country(previous_country, current_country),
        is_new_device(known_device, submitted_device),
    ) {
        (true, true) => Some(HoldReason::NewCountryAndDevice),
        (true, false) => Some(HoldReason::NewCountry),
        (false, true) => Some(HoldReason::NewDevice),
        (false, false) => None,
    }
}

/// What a sign-in is held on, once the route has resolved both signals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hold {
    pub reason: HoldReason,
    /// The country the edge resolved, when it resolved one. `None` on a
    /// device-only hold in the default deployment, where nothing resolves a
    /// country at all, which is why `pending_login_approvals.country` is
    /// nullable.
    pub country: Option<String>,
}

/// The hold a sign-in must take, from what the login route has in hand. `None`
/// means the sign-in completes as it does today.
///
/// The country comes from the LINKS-31 peer-gated header reader, never from the
/// raw request, so a direct client can neither forge a country nor suppress one.
/// The device id does come from the request, which is what lets it be attached
/// to the hold and recorded when the approval is claimed; forging one gains
/// nothing, because a value matching no row is as unrecognised as no value.
pub fn approval_hold(
    mail: &MailConfig,
    previous_country: Option<&str>,
    headers: &HeaderMap,
    known_device: Option<bool>,
    submitted_device: Option<&str>,
) -> Option<Hold> {
    let current = client_country(headers);
    let reason = approval_reason(
        mail.login_approval_enabled,
        previous_country,
        current.as_deref(),
        known_device,
        submitted_device,
    )?;

    Some(Hold {
        reason,
        country: current,
    })
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
    pub reason: HoldReason,
    /// `None` when nothing resolved a country, which is the default deployment.
    pub country: Option<String>,
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

/// A token the approval POST tried to claim, and the two baselines the claim
/// promotes: the country becomes the account's known one and the device becomes
/// one of the account's known ones. Either may be absent.
pub enum ApprovalClaim {
    Claimed {
        user_id: Uuid,
        country: Option<String>,
        device_id_hash: Option<Vec<u8>>,
    },
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

/// Most live links one account may hold at a time.
///
/// A ceiling, not a policy: the exact-match dedup below already collapses a
/// retried sign-in, so this only bounds a client that varies its device id on
/// every attempt, which would otherwise mail a fresh link each time. Reaching
/// it still holds the sign-in; it only stops another mail going out.
const MAX_LIVE_APPROVALS: i64 = 3;

/// Hold a sign-in and email the user a single-use approval link.
///
/// Runs ON the login hot path, unlike the LINKS-27 alert: nothing is issued
/// until the row exists and the mail is away, and a delivery failure propagates
/// so the sign-in fails closed rather than completing ungated.
///
/// `device_id_hash` is the device this attempt submitted, which the claim
/// promotes into `known_devices`. `None` when the client submitted no id, and
/// then approving records no device.
pub async fn request_login_approval(
    pool: &PgPool,
    config: &Config,
    user_id: Uuid,
    email: &str,
    hold: &Hold,
    device_id_hash: Option<&[u8]>,
    headers: &HeaderMap,
) -> Result<(), AppError> {
    sweep_expired_approvals(pool).await;

    // One live link per distinct attempt, so retrying the sign-in while the
    // first link is still claimable does not send a second mail, while a
    // genuinely different attempt still gets its own. `IS NOT DISTINCT FROM`
    // because either side may be NULL and `= NULL` would match nothing.
    let (same_attempt, live): (bool, i64) = sqlx::query_as(
        "SELECT
            COUNT(*) FILTER (
                WHERE country IS NOT DISTINCT FROM $2
                  AND device_id_hash IS NOT DISTINCT FROM $3
            ) > 0,
            COUNT(*)
         FROM pending_login_approvals
         WHERE user_id = $1 AND consumed_at IS NULL AND expires_at > NOW()",
    )
    .bind(user_id)
    .bind(hold.country.as_deref())
    .bind(device_id_hash)
    .fetch_one(pool)
    .await?;

    if same_attempt {
        tracing::info!(
            user_id = %user_id,
            reason = hold.reason.as_str(),
            "LINKS-35: sign-in held; a live approval link already covers this attempt"
        );
        return Ok(());
    }

    if live >= MAX_LIVE_APPROVALS {
        tracing::warn!(
            user_id = %user_id,
            reason = hold.reason.as_str(),
            live = live,
            "LINKS-45: sign-in held; the live approval link cap is reached, so no new mail was sent"
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
        "INSERT INTO pending_login_approvals
            (user_id, token_hash, country, ip, device, expires_at, reason, device_id_hash)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(user_id)
    .bind(hash_approval_token(&token).as_slice())
    .bind(hold.country.as_deref())
    .bind(&ip)
    .bind(&device)
    .bind(expires_at)
    .bind(hold.reason.as_str())
    .bind(device_id_hash)
    .execute(pool)
    .await?;

    tracing::warn!(
        user_id = %user_id,
        country = hold.country.as_deref().unwrap_or("unresolved"),
        reason = hold.reason.as_str(),
        "LINKS-35: sign-in held for approval"
    );

    crate::auth::mailer::send_login_approval_email(
        &config.mail,
        email,
        hold,
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
            Option<String>,
            String,
            Option<String>,
            DateTime<Utc>,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
            String,
        ),
    >(
        "SELECT country, ip, device, created_at, expires_at, consumed_at, reason
         FROM pending_login_approvals WHERE token_hash = $1",
    )
    .bind(token_hash.as_slice())
    .fetch_optional(pool)
    .await?;

    let Some((country, ip, device, requested_at, expires_at, consumed_at, reason)) = row else {
        return Ok(ApprovalLookup::Invalid(ApprovalFailure::NotFound));
    };
    if consumed_at.is_some() {
        return Ok(ApprovalLookup::Invalid(ApprovalFailure::AlreadyUsed));
    }
    if expires_at <= Utc::now() {
        return Ok(ApprovalLookup::Invalid(ApprovalFailure::Expired));
    }

    Ok(ApprovalLookup::Valid(PendingApproval {
        reason: HoldReason::from_column(&reason),
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

    let claimed = sqlx::query_as::<_, (Uuid, Option<String>, Option<Vec<u8>>)>(
        "UPDATE pending_login_approvals SET consumed_at = NOW()
         WHERE token_hash = $1 AND consumed_at IS NULL AND expires_at > NOW()
         RETURNING user_id, country, device_id_hash",
    )
    .bind(token_hash.as_slice())
    .fetch_optional(pool)
    .await?;

    match claimed {
        Some((user_id, country, device_id_hash)) => Ok(ApprovalClaim::Claimed {
            user_id,
            country,
            device_id_hash,
        }),
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
        country = escape_html(pending.country.as_deref().unwrap_or("unknown")),
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
        &format!(
            "Someone signed in to your Rusty Links account {}. Approve it only if it was you.",
            pending.reason.summary()
        ),
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
/// Claims the token, once, and records both baselines the approval carries: the
/// country becomes the account's known one and the submitted device becomes one
/// of the account's known devices. Those are the same two writes a completed
/// sign-in makes, so the user's next sign-in from that country on that device is
/// no longer new on either trigger and completes ungated. Recording the device
/// here rather than from a response is what makes approving terminate: the link
/// is usually opened in a different browser, so only the id the held sign-in
/// sent identifies the browser that was actually held.
pub async fn approve_login_submit(
    State(pool): State<PgPool>,
    Form(form): Form<TokenForm>,
) -> Result<Response, AppError> {
    if form.token.is_empty() {
        return Ok(failure_page(ApprovalFailure::NotFound));
    }

    Ok(match consume_login_approval(&pool, &form.token).await? {
        ApprovalClaim::Claimed {
            user_id,
            country,
            device_id_hash,
        } => {
            if let Some(country) = country.as_deref() {
                User::update_last_login_country(&pool, user_id, country).await?;
            }
            if let Some(hash) = device_id_hash.as_deref() {
                record_device(&pool, user_id, hash).await?;
            }
            tracing::info!(
                user_id = %user_id,
                country = country.as_deref().unwrap_or("unresolved"),
                device_recorded = device_id_hash.is_some(),
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
/// credential login, so no hosted deployment can ever hold a sign-in. LINKS-45
/// does not change that. The OIDC-RP callback (`oidc_rp::callback`) stays
/// ungated on the device trigger for the same reasons it is ungated on the
/// country one: the OP owns that credential and its recovery, this service
/// holds none, and a hosted deployment need not configure SMTP, so holding
/// there would turn a control into a lockout with no way back.
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

    // A device the account already knows, which keeps the country cases below
    // about the country alone.
    const KNOWN: (Option<bool>, Option<&str>) = (Some(true), Some("device-a"));
    // A device the account has never seen, on an account that has others.
    const UNKNOWN: (Option<bool>, Option<&str>) = (Some(false), Some("device-b"));
    // An account with no recorded device at all: the deploy-day baseline.
    const NO_DEVICES: (Option<bool>, Option<&str>) = (None, Some("device-b"));
    // A client that submitted nothing, which is every pre-LINKS-45 client.
    const NOT_SUBMITTED: (Option<bool>, Option<&str>) = (Some(false), None);

    fn reason(
        enabled: bool,
        previous: Option<&str>,
        current: Option<&str>,
        device: (Option<bool>, Option<&str>),
    ) -> Option<HoldReason> {
        approval_reason(enabled, previous, current, device.0, device.1)
    }

    fn hold(
        mail_config: &MailConfig,
        previous: Option<&str>,
        headers: &HeaderMap,
        device: (Option<bool>, Option<&str>),
    ) -> Option<Hold> {
        approval_hold(mail_config, previous, headers, device.0, device.1)
    }

    // ── The gate decision: the country trigger ───────────────────────────

    // A country the account has not used before is what the gate holds on.
    #[test]
    fn a_new_country_is_gated_when_enabled() {
        assert_eq!(
            reason(true, Some("US"), Some("DE"), KNOWN),
            Some(HoldReason::NewCountry)
        );
    }

    // A first-ever sign-in has no prior country to differ from and no recorded
    // device, so it is never gated. `setup_handler` creates the first user, so
    // this is not academic: gating it would mean the first user could never
    // sign in.
    #[test]
    fn a_first_ever_sign_in_is_never_gated() {
        assert_eq!(reason(true, None, Some("US"), NO_DEVICES), None);
        assert_eq!(reason(true, None, None, NO_DEVICES), None);
        assert_eq!(reason(true, None, None, (None, None)), None);
    }

    // An unresolved country (no geoblock edge, no trusted proxy, which is the
    // default) has nothing to compare, so it is never gated on the country.
    #[test]
    fn an_unresolved_country_is_never_gated() {
        assert_eq!(reason(true, Some("US"), None, KNOWN), None);
    }

    // A repeat from the same country is not a change, case-insensitively.
    #[test]
    fn the_same_country_is_never_gated() {
        assert_eq!(reason(true, Some("US"), Some("US"), KNOWN), None);
        assert_eq!(reason(true, Some("us"), Some("US"), KNOWN), None);
    }

    // ── The gate decision: the device trigger ────────────────────────────

    // A device the account has not signed in from before is the second trigger,
    // and it holds on its own, from a country the account already uses.
    #[test]
    fn a_new_device_is_gated_when_enabled() {
        assert_eq!(
            reason(true, Some("US"), Some("US"), UNKNOWN),
            Some(HoldReason::NewDevice)
        );
    }

    // A device the account already knows is not held, which is the ordinary
    // sign-in and must stay ordinary.
    #[test]
    fn a_known_device_is_never_gated() {
        assert_eq!(reason(true, Some("US"), Some("US"), KNOWN), None);
    }

    // The deploy-day case: `known_devices` is created empty, so every existing
    // account has none. Holding on that would hold every account at once on the
    // deploy, so an account with no recorded device is a baseline, not a hold.
    #[test]
    fn an_account_with_no_recorded_devices_is_never_gated() {
        assert_eq!(reason(true, Some("US"), Some("US"), NO_DEVICES), None);
    }

    // A client that submits no device id degrades to exactly the country-only
    // behaviour: never held on the device signal, still held on a new country.
    #[test]
    fn a_sign_in_with_no_device_id_is_country_only() {
        assert_eq!(reason(true, Some("US"), Some("US"), NOT_SUBMITTED), None);
        assert_eq!(
            reason(true, Some("US"), Some("DE"), NOT_SUBMITTED),
            Some(HoldReason::NewCountry)
        );
    }

    // ── The two triggers compose ─────────────────────────────────────────

    // Either alone holds, both together hold once and say so, and neither holds
    // nothing. One hold covers both, so a user who changed country AND machine
    // approves once rather than twice.
    #[test]
    fn the_two_triggers_compose_into_one_hold() {
        assert_eq!(
            reason(true, Some("US"), Some("DE"), KNOWN),
            Some(HoldReason::NewCountry)
        );
        assert_eq!(
            reason(true, Some("US"), Some("US"), UNKNOWN),
            Some(HoldReason::NewDevice)
        );
        assert_eq!(
            reason(true, Some("US"), Some("DE"), UNKNOWN),
            Some(HoldReason::NewCountryAndDevice)
        );
        assert_eq!(reason(true, Some("US"), Some("US"), KNOWN), None);
    }

    // The kill switch is the outer guard: with it off nothing is ever held, on
    // EITHER trigger, including both at once.
    #[test]
    fn the_kill_switch_off_gates_nothing() {
        assert_eq!(reason(false, Some("US"), Some("DE"), KNOWN), None);
        assert_eq!(reason(false, Some("US"), Some("US"), UNKNOWN), None);
        assert_eq!(reason(false, Some("US"), Some("DE"), UNKNOWN), None);
        assert_eq!(reason(false, None, Some("US"), NO_DEVICES), None);
    }

    // ── The same decisions through the header reader ─────────────────────

    // The never-gated cases through the reader the route actually calls, not
    // only through the pure predicate.
    #[test]
    fn header_level_never_gated_sign_ins_stay_ungated() {
        // First-ever: the edge resolved a country, but the account has neither
        // a known country nor a recorded device.
        assert_eq!(
            hold(
                &mail(true),
                None,
                &headers_with_country(Some("DE")),
                NO_DEVICES
            ),
            None
        );
        // Unresolved country, on a device the account knows: nothing to hold on.
        assert_eq!(
            hold(&mail(true), Some("US"), &headers_with_country(None), KNOWN),
            None
        );
        // No device id submitted and no country resolved, which is a plain API
        // client against the default deployment.
        assert_eq!(
            hold(
                &mail(true),
                Some("US"),
                &headers_with_country(None),
                NOT_SUBMITTED
            ),
            None
        );
    }

    // With the gate on and both sides known, the reader yields the country to
    // hold on, normalized the same way the alert normalizes it.
    #[test]
    fn header_level_new_country_is_gated() {
        assert_eq!(
            hold(
                &mail(true),
                Some("US"),
                &headers_with_country(Some("de")),
                KNOWN
            ),
            Some(Hold {
                reason: HoldReason::NewCountry,
                country: Some("DE".to_string()),
            })
        );
    }

    // A device-only hold in a deployment that resolves no country carries none,
    // which is why `pending_login_approvals.country` is nullable.
    #[test]
    fn header_level_new_device_is_gated_without_a_country() {
        assert_eq!(
            hold(&mail(true), None, &headers_with_country(None), UNKNOWN),
            Some(Hold {
                reason: HoldReason::NewDevice,
                country: None,
            })
        );
    }

    // The kill switch is checked before anything else the route would do, on
    // both triggers.
    #[test]
    fn header_level_kill_switch_off_gates_nothing() {
        assert_eq!(
            hold(
                &mail(false),
                Some("US"),
                &headers_with_country(Some("DE")),
                UNKNOWN
            ),
            None
        );
    }

    // A forged X-IPCountry from an untrusted peer never reaches this reader
    // (LINKS-31 strips it upstream), and a malformed value is not a country.
    #[test]
    fn a_malformed_country_header_is_never_gated() {
        for bad in ["", "USA", "1A", "U"] {
            assert_eq!(
                hold(
                    &mail(true),
                    Some("US"),
                    &headers_with_country(Some(bad)),
                    KNOWN
                ),
                None,
                "expected {bad:?} to resolve to no country"
            );
        }
    }

    // The stored `reason` round-trips, and a value written before LINKS-45 (or
    // by a newer node during a rollout) reads as the country hold rather than
    // failing the page.
    #[test]
    fn the_hold_reason_round_trips_through_its_column() {
        for expected in [
            HoldReason::NewCountry,
            HoldReason::NewDevice,
            HoldReason::NewCountryAndDevice,
        ] {
            assert_eq!(HoldReason::from_column(expected.as_str()), expected);
        }
        assert_eq!(
            HoldReason::from_column("something_else"),
            HoldReason::NewCountry
        );
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
            reason: HoldReason::NewCountryAndDevice,
            country: Some("DE".to_string()),
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

    async fn login_body(country: Option<&str>, device_id: Option<&str>) -> (StatusCode, String) {
        let router = api_router(config_for_tests());
        let mut builder = Request::builder()
            .method("POST")
            .uri("/auth/login")
            .header("Content-Type", "application/json");
        if let Some(country) = country {
            builder = builder.header("X-IPCountry", country);
        }
        let mut payload = serde_json::json!({
            "email": "alice@example.com",
            "password": "hunter2hunter2",
        });
        if let Some(device_id) = device_id {
            payload["device_id"] = serde_json::Value::String(device_id.to_string());
        }
        let request = builder.body(Body::from(payload.to_string())).unwrap();
        let response = router.oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .unwrap();
        (status, String::from_utf8_lossy(&bytes).to_string())
    }

    // At the route, with the gate switched on, no never-gated case can produce
    // an approval-required answer, with or without a device id. The account's
    // stored country and its known devices both live in the database, so the
    // gate cannot fire before those rows are read; a login that never gets that
    // far fails as an ordinary error instead. The same cases against a real
    // database, where the rows do exist, are in `tests/db_known_devices.rs`.
    #[tokio::test]
    async fn the_login_route_never_holds_an_unresolved_or_first_ever_sign_in() {
        for country in [None, Some("DE")] {
            for device_id in [None, Some("device-a")] {
                let (status, body) = login_body(country, device_id).await;
                assert_ne!(
                    status,
                    StatusCode::FORBIDDEN,
                    "login with country {country:?} and device {device_id:?} must not be held"
                );
                assert!(
                    !body.contains("APPROVAL_REQUIRED"),
                    "login with country {country:?} and device {device_id:?} must not be held: {body}"
                );
            }
        }
    }
}
