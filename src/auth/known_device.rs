//! LINKS-45: recognise the device a sign-in comes from, the second trigger on
//! the LINKS-35 approval gate.
//!
//! The identity is a random id the browser mints once and keeps in
//! `localStorage` (see [`crate::ui::auth_state::device_id`]), sent with the
//! sign-in request and stored server-side only as its SHA-256. Nothing about
//! the browser is inspected. A raw User-Agent is deliberately NOT the identity:
//! it changes on every browser update, so a UA-derived device would hold every
//! user after each browser release, which is both a recurring lockout and a way
//! to train people into approving reflexively, which would destroy the value of
//! the country trigger too. [`crate::auth::middleware::device_info`] still
//! records the User-Agent for the approval page to display, and nothing ever
//! decides on it.
//!
//! The id travels with the REQUEST rather than being set on a response, which
//! is what makes approving terminate. A held sign-in writes the id's hash onto
//! its `pending_login_approvals` row, and claiming the emailed link promotes
//! that hash into `known_devices`. The link is usually opened in a different
//! browser (a mail client), so a value the server set on the approval response
//! would mark the approving device and leave the held one challenged forever.
//!
//! What this detects: a sign-in from a browser profile that has never completed
//! one for this account. What it does NOT detect, and cannot: cleared site
//! data, a private window, a second browser on the same machine, and a
//! genuinely new machine are indistinguishable, because none of them carries
//! the stored id. Nor is it an anti-automation control: a client that sends no
//! id at all is never held on this signal (see [`known_device_state`]), so this
//! is browser hardening for the app's own sign-in form, not a bot gate.
//!
//! The id carries no authority on its own: sending one grants nothing and
//! forging one gains nothing, because a value matching no row is exactly as
//! unrecognised as no value at all.
//!
//! Deploy day: `known_devices` is created empty and is NOT backfilled, so every
//! existing account has zero known devices. That is read as the account's
//! baseline and is never new (see
//! [`crate::auth::location_alert::is_new_device`]), so the first sign-in after
//! the migration records a device rather than being held. Reading it the other
//! way would hold every account at once on deploy day.

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::server_functions::auth::KnownDeviceInfo;

/// Longest device id accepted from a client. The browser mints a 32-character
/// hex UUID; anything far longer is a client bug or someone probing, and it is
/// truncated rather than rejected so a sign-in never fails on it.
pub const MAX_DEVICE_ID_LEN: usize = 128;

/// Normalise the id a client submitted.
///
/// Absent, empty, or whitespace-only all mean "not submitted", which is never
/// "new" (see [`crate::auth::location_alert::is_new_device`]), so a client that
/// sends nothing degrades to exactly the country-only behaviour.
pub fn normalize_device_id(submitted: Option<&str>) -> Option<String> {
    submitted
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(MAX_DEVICE_ID_LEN).collect())
}

/// SHA-256 of a device id, as stored in `known_devices.device_id_hash`. Matches
/// how the approval token is stored, so a database dump yields nothing that can
/// be replayed as a known device.
pub fn hash_device_id(device_id: &str) -> Vec<u8> {
    Sha256::digest(device_id.as_bytes()).to_vec()
}

/// What the account already knows about a submitted device.
///
/// `None` means the account has no recorded device at all, which is its
/// baseline; `Some(true)` that the submitted id is one of them; `Some(false)`
/// that the account has devices and this is not one. The three map onto the
/// `known` argument of [`crate::auth::location_alert::is_new_device`].
///
/// Recognition is scoped to the account, so an id another account made known on
/// the same browser is not recognised for this one. Nothing is queried when no
/// id was submitted, because an unsubmitted id can never be held on.
pub async fn known_device_state(
    pool: &PgPool,
    user_id: Uuid,
    device_id: Option<&str>,
) -> Result<Option<bool>, AppError> {
    let Some(device_id) = device_id else {
        return Ok(None);
    };

    let (account_has_devices, recognized) = sqlx::query_as::<_, (bool, bool)>(
        "SELECT
            EXISTS (SELECT 1 FROM known_devices WHERE user_id = $1),
            EXISTS (SELECT 1 FROM known_devices WHERE user_id = $1 AND device_id_hash = $2)",
    )
    .bind(user_id)
    .bind(hash_device_id(device_id).as_slice())
    .fetch_one(pool)
    .await?;

    Ok(account_has_devices.then_some(recognized))
}

/// Record a device against an account.
///
/// Called only from a sign-in that completes and from a claimed approval, never
/// from a held attempt, so an attempt nobody approves can never make its device
/// look familiar. A repeat is an idempotent touch of `last_seen_at`.
pub async fn record_device(
    pool: &PgPool,
    user_id: Uuid,
    device_id_hash: &[u8],
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO known_devices (user_id, device_id_hash) VALUES ($1, $2)
         ON CONFLICT (user_id, device_id_hash) DO UPDATE SET last_seen_at = NOW()",
    )
    .bind(user_id)
    .bind(device_id_hash)
    .execute(pool)
    .await?;

    Ok(())
}

/// Record the device a completed sign-in submitted, if it submitted one.
///
/// The one call every completion path makes, so "recorded only on completion"
/// cannot drift between the setup, register, and login routes.
pub async fn record_submitted_device(
    pool: &PgPool,
    user_id: Uuid,
    device_id: Option<&str>,
) -> Result<(), AppError> {
    let Some(device_id) = device_id else {
        return Ok(());
    };
    record_device(pool, user_id, &hash_device_id(device_id)).await
}

/// The devices an account is recognised from, newest use first (LINKS-55).
///
/// `current_hash` is the hash of the device id the listing request presented,
/// and the `is_current` flag is computed by comparing it IN THE QUERY. That is
/// deliberate: no `device_id_hash` is ever selected, so none is materialised in
/// a struct, a log line, or a response, and the rule that a hash never leaves
/// the server holds structurally rather than by remembering to omit a field.
///
/// Scoped to `user_id`, so this can only ever describe the calling account.
pub async fn list_for_user(
    pool: &PgPool,
    user_id: Uuid,
    current_hash: Option<&[u8]>,
) -> Result<Vec<KnownDeviceInfo>, AppError> {
    let rows = sqlx::query_as::<_, (Uuid, DateTime<Utc>, DateTime<Utc>, bool)>(
        "SELECT id, first_seen_at, last_seen_at, device_id_hash IS NOT DISTINCT FROM $2::bytea
         FROM known_devices
         WHERE user_id = $1
         ORDER BY last_seen_at DESC, id",
    )
    .bind(user_id)
    .bind(current_hash)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(id, first_seen_at, last_seen_at, is_current)| KnownDeviceInfo {
                id,
                first_seen_at,
                last_seen_at,
                is_current,
            },
        )
        .collect())
}

/// Revoke one device, returning whether a row was removed (LINKS-55).
///
/// The account is part of the WHERE clause rather than something checked after
/// the read, so an id belonging to another account matches nothing, deletes
/// nothing, and is indistinguishable from an id that does not exist. Ownership
/// as a predicate is what keeps a 404 honest: there is no path where a row is
/// found first and the caller is compared to it second.
///
/// Removing the LAST device is allowed and is not a lockout. It returns the
/// account to the zero-devices baseline, where
/// [`crate::auth::location_alert::is_new_device`] never holds, which is where
/// every account already sat on the deploy that created the table. A security
/// control must never be able to make an account unreachable, and revocation
/// can only ever move an account toward being held MORE, or back to baseline.
pub async fn delete_for_user(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool, AppError> {
    let removed = sqlx::query("DELETE FROM known_devices WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?
        .rows_affected();

    Ok(removed > 0)
}

/// A fresh device id, in the shape the browser mints.
///
/// Server-side only so the tests and the client agree on the shape; the real
/// one is minted in the browser and kept there.
pub fn generate_device_id() -> String {
    Uuid::new_v4().simple().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ids are unguessable and unique per mint: a version-4 UUID in its 32-hex
    // form, 122 random bits.
    #[test]
    fn device_ids_are_distinct_and_long() {
        let a = generate_device_id();
        let b = generate_device_id();
        assert_ne!(a, b);
        assert_eq!(a.len(), 32);
    }

    // Only the hash is ever stored, and it is not the id.
    #[test]
    fn the_stored_hash_is_not_the_device_id() {
        let id = generate_device_id();
        let hash = hash_device_id(&id);
        assert_eq!(hash.len(), 32);
        assert_ne!(hash, id.as_bytes());
        assert_eq!(hash, hash_device_id(&id));
        assert_ne!(hash, hash_device_id(&generate_device_id()));
    }

    // Absent, empty and whitespace-only are all "not submitted", so none of
    // them can be held on and none of them can be recorded as a device.
    #[test]
    fn a_blank_device_id_is_not_submitted() {
        assert_eq!(normalize_device_id(None), None);
        assert_eq!(normalize_device_id(Some("")), None);
        assert_eq!(normalize_device_id(Some("   ")), None);
    }

    // A real id survives normalisation, trimmed, and an absurd one is truncated
    // rather than failing the sign-in.
    #[test]
    fn a_submitted_device_id_is_trimmed_and_bounded() {
        let id = generate_device_id();
        assert_eq!(normalize_device_id(Some(&format!(" {id} "))), Some(id));

        let huge = "a".repeat(MAX_DEVICE_ID_LEN * 4);
        let kept = normalize_device_id(Some(&huge)).expect("a long id is still an id");
        assert_eq!(kept.len(), MAX_DEVICE_ID_LEN);
    }
}
