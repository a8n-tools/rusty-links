//! LINKS-27: detect a significant login-location change and notify the user.
//!
//! Ported from menkent (MKT-61) by way of storefront (SF-28). The country is
//! resolved at the edge from the reverse proxy's `X-IPCountry` header (see
//! [`crate::auth::middleware::client_country`]) rather than an in-process geoip
//! database, so there is nothing to provision or license. Granularity is
//! country-level.
//!
//! The header carries the same trust level as the forwarded IP, so LINKS-31
//! gates both on the socket peer: `X-Forwarded-For`, `X-Real-Ip`, and
//! `X-IPCountry` are read only when the peer is a configured trusted proxy (see
//! [`crate::auth::trusted_proxy`]), and are ignored otherwise. Behind the
//! geoblock edge they are authoritative; off it (a direct client, no configured
//! proxy, an unset header) the country is `None` and no alert ever fires, so a
//! forged header can neither raise a false alarm nor suppress a real one.
//!
//! The per-user opt-out (`users.notify_new_location`) is the user's to set:
//! `GET /api/auth/me` reports it and `PATCH /api/auth/me` changes it, always
//! for the session's own account (LINKS-33).

use std::sync::OnceLock;
use std::time::Duration;

use axum::http::HeaderMap;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::middleware::{client_country, client_ip_from_headers, device_info};
use crate::config::MailConfig;
use crate::error::AppError;
use crate::models::User;

/// Best-effort cap of one alert per user per country per day. The durable
/// dedup is `last_login_country`; this only blunts a burst within one process.
static ALERT_DEDUP: OnceLock<moka::future::Cache<String, ()>> = OnceLock::new();

fn alert_dedup() -> &'static moka::future::Cache<String, ()> {
    ALERT_DEDUP.get_or_init(|| {
        moka::future::Cache::builder()
            .time_to_live(Duration::from_secs(24 * 60 * 60))
            .build()
    })
}

/// Whether a login warrants a new-location alert.
///
/// A change is significant only when a prior country is known and the new one
/// differs. A first-ever login (no `previous`) and a repeat from the same
/// country are both silent; an unresolved `current` never alerts; and a user
/// who has opted out (`notify_new_location == false`) is never alerted.
pub fn should_alert(
    notify_new_location: bool,
    previous: Option<&str>,
    current: Option<&str>,
) -> bool {
    notify_new_location
        && matches!((previous, current), (Some(prev), Some(curr)) if !prev.eq_ignore_ascii_case(curr))
}

/// Resolve an account-settings patch against the stored preference.
///
/// `submitted` is `None` when the request left the key out, which means "not
/// submitted" rather than "turn it off", so the stored value stands; an
/// explicit `Some(false)` is a real opt-out and wins (LINKS-33). Keeping this
/// separate from the write is what stops a patch that touches one setting from
/// silently reverting another.
pub fn resolve_notify_new_location(stored: bool, submitted: Option<bool>) -> bool {
    submitted.unwrap_or(stored)
}

/// Evaluate the new-location alert off the login hot path.
///
/// Resolves the country from the request headers and, when the global kill
/// switch is on and a country is resolvable, spawns a detached task that
/// compares it against the user's last-known country, emails on a significant
/// change, and records the new country for next time. Every failure inside the
/// task is logged, never surfaced, so the alert can never fail or slow a login.
pub fn spawn_new_location_check(
    pool: &PgPool,
    mail: &MailConfig,
    user_id: Uuid,
    headers: &HeaderMap,
) {
    if !mail.login_location_alerts_enabled {
        return;
    }
    let Some(country) = client_country(headers) else {
        return;
    };
    let pool = pool.clone();
    let mail = mail.clone();
    let ip = client_ip_from_headers(headers).unwrap_or_else(|| "unknown".to_string());
    let device = device_info(headers);
    tokio::spawn(async move {
        if let Err(error) =
            maybe_notify_new_location(&pool, &mail, user_id, &country, &ip, device.as_deref()).await
        {
            tracing::warn!(
                user_id = %user_id,
                error = %error,
                "LINKS-27: new-location alert failed"
            );
        }
    });
}

/// Compare the login country against the user's last-known, email on a
/// significant change (subject to the per-user opt-out and a best-effort
/// once-per-country-per-day cap), then record the new country. The last-country
/// update is the durable dedup: a repeat from the same country is silent on the
/// next login regardless of the in-memory cap.
async fn maybe_notify_new_location(
    pool: &PgPool,
    mail: &MailConfig,
    user_id: Uuid,
    country: &str,
    ip: &str,
    device: Option<&str>,
) -> Result<(), AppError> {
    let Some((email, previous, notify_new_location)) =
        User::get_login_location(pool, user_id).await?
    else {
        return Ok(());
    };

    if should_alert(notify_new_location, previous.as_deref(), Some(country))
        && allow_alert(user_id, country).await
    {
        tracing::warn!(
            user_id = %user_id,
            country = %country,
            "LINKS-27: new sign-in from a previously unseen country"
        );
        crate::auth::mailer::send_new_signin_location_email(mail, &email, country, ip, device)
            .await?;
    }

    User::update_last_login_country(pool, user_id, country).await?;
    Ok(())
}

/// True the first time this user is alerted about this country within a day.
async fn allow_alert(user_id: Uuid, country: &str) -> bool {
    let key = format!("{user_id}:{country}");
    let cache = alert_dedup();
    if cache.get(&key).await.is_some() {
        return false;
    }
    cache.insert(key, ()).await;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    // A repeat login from the same country is not a change (case-insensitive:
    // the edge may vary casing between logins).
    #[test]
    fn same_country_does_not_alert() {
        assert!(!should_alert(true, Some("US"), Some("US")));
        assert!(!should_alert(true, Some("us"), Some("US")));
    }

    // A country change alerts.
    #[test]
    fn country_change_alerts() {
        assert!(should_alert(true, Some("US"), Some("DE")));
    }

    // A user's first-ever login (no prior country) is never flagged.
    #[test]
    fn first_login_does_not_alert() {
        assert!(!should_alert(true, None, Some("US")));
    }

    // A private / unresolvable IP yields no country, so no alert (no crash).
    #[test]
    fn unresolved_country_does_not_alert() {
        assert!(!should_alert(true, Some("US"), None));
        assert!(!should_alert(true, None, None));
    }

    // The per-user opt-out suppresses the alert even on a real change. This is
    // the end the LINKS-33 toggle writes to: turning it off silences the mail,
    // turning it back on restores it.
    #[test]
    fn opt_out_suppresses_alert() {
        assert!(!should_alert(false, Some("US"), Some("DE")));
        assert!(should_alert(true, Some("US"), Some("DE")));
    }

    // An absent key means "not submitted", so the stored preference stands
    // rather than silently reverting to the column default.
    #[test]
    fn absent_key_leaves_the_stored_preference_unchanged() {
        assert!(resolve_notify_new_location(true, None));
        assert!(!resolve_notify_new_location(false, None));
    }

    // A submitted value wins in both directions, so an explicit opt-out is a
    // real opt-out and opting back in is possible.
    #[test]
    fn a_submitted_preference_wins_over_the_stored_one() {
        assert!(!resolve_notify_new_location(true, Some(false)));
        assert!(resolve_notify_new_location(false, Some(true)));
        assert!(!resolve_notify_new_location(false, Some(false)));
        assert!(resolve_notify_new_location(true, Some(true)));
    }

    // The edge header is normalized to an uppercase alpha-2 code.
    #[test]
    fn client_country_reads_and_normalizes_the_edge_header() {
        let mut headers = HeaderMap::new();
        headers.insert("X-IPCountry", "us".parse().unwrap());
        assert_eq!(client_country(&headers), Some("US".to_string()));
    }

    // No edge header (a direct client, no geoblock) means no country, so the
    // feature degrades to no alert.
    #[test]
    fn client_country_is_none_when_header_absent() {
        assert_eq!(client_country(&HeaderMap::new()), None);
    }

    // Empty, sentinel, or wrong-shaped values are rejected rather than treated
    // as a country.
    #[test]
    fn client_country_rejects_malformed_values() {
        for bad in ["", "nil", "U", "USA", "1A", "  "] {
            let mut headers = HeaderMap::new();
            headers.insert("X-IPCountry", bad.parse().unwrap());
            assert_eq!(
                client_country(&headers),
                None,
                "expected {bad:?} to be rejected"
            );
        }
    }

    // At most one alert per user per country per day; a different country is a
    // distinct event.
    #[tokio::test]
    async fn alerts_are_deduped_per_user_and_country() {
        let user = Uuid::new_v4();
        assert!(allow_alert(user, "DE").await);
        assert!(!allow_alert(user, "DE").await);
        assert!(allow_alert(user, "US").await);
        assert!(allow_alert(Uuid::new_v4(), "DE").await);
    }
}
