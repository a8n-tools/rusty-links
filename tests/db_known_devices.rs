//! LINKS-45 device recognition against a real Postgres (LINKS-44).
//!
//! The reads and writes `src/auth/known_device.rs` makes, plus the two never-
//! held cases that only exist in the database: an account with no recorded
//! device, which is every account on the deploy that creates the table, and a
//! device recorded against one account not being recognised for another.
//!
//! Only the SHA-256 of a device id is stored, so every case here submits the
//! plaintext id and asserts through the same hashing the login route uses.

#![cfg(feature = "server")]

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use rusty_links::auth::known_device::{
    generate_device_id, hash_device_id, known_device_state, record_device, record_submitted_device,
};
use sqlx::PgPool;
use tower::ServiceExt; // for `oneshot`
use uuid::Uuid;

async fn device_count(pool: &PgPool, user_id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM known_devices WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .expect("the count must run")
}

// ── The three states the gate reads ──────────────────────────────────────────

/// The deploy-day baseline, and the one that matters most: `known_devices` is
/// created empty and is not backfilled, so every account that already exists
/// reads as "no recorded devices". That is `None`, which
/// `location_alert::is_new_device` never holds on, so the migration cannot lock
/// the whole user base out at once.
#[tokio::test]
async fn an_account_with_no_recorded_devices_reads_as_no_baseline() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;

    assert_eq!(device_count(&pool, user.id).await, 0);
    assert_eq!(
        known_device_state(&pool, user.id, Some(&generate_device_id()))
            .await
            .expect("the lookup must run"),
        None,
        "a fresh account has no device baseline, so nothing can be new against it"
    );

    common::delete_user(&pool, user.id).await;
}

/// A device the account has completed a sign-in from reads as recognised, which
/// is the ordinary sign-in and must stay ungated.
#[tokio::test]
async fn a_recorded_device_reads_as_recognized() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;
    let device = generate_device_id();

    record_device(&pool, user.id, &hash_device_id(&device))
        .await
        .expect("the record must run");

    assert_eq!(
        known_device_state(&pool, user.id, Some(&device))
            .await
            .expect("the lookup must run"),
        Some(true)
    );

    common::delete_user(&pool, user.id).await;
}

/// An account that HAS devices, presented with one that is not among them, is
/// the only state the gate holds on.
#[tokio::test]
async fn an_unrecorded_device_on_an_account_with_devices_reads_as_new() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;

    record_device(&pool, user.id, &hash_device_id("device-a"))
        .await
        .expect("the record must run");

    assert_eq!(
        known_device_state(&pool, user.id, Some("device-b"))
            .await
            .expect("the lookup must run"),
        Some(false)
    );

    common::delete_user(&pool, user.id).await;
}

/// A client that submits no device id is never held on the device signal, and
/// the lookup does not even reach the database for it.
#[tokio::test]
async fn an_unsubmitted_device_reads_as_no_baseline() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;
    record_device(&pool, user.id, &hash_device_id("device-a"))
        .await
        .expect("the record must run");

    assert_eq!(
        known_device_state(&pool, user.id, None)
            .await
            .expect("the lookup must run"),
        None,
        "not submitted is never new, whatever the account already knows"
    );

    common::delete_user(&pool, user.id).await;
}

/// Recognition is scoped to the account. A shared browser that one account has
/// made known does not make the second account's first sign-in look familiar.
#[tokio::test]
async fn a_device_known_to_one_account_is_not_known_to_another() {
    let pool = common::test_pool().await;
    let alice = common::new_user(&pool).await;
    let bob = common::new_user(&pool).await;
    let device = generate_device_id();

    record_device(&pool, alice.id, &hash_device_id(&device))
        .await
        .expect("the record must run");
    record_device(&pool, bob.id, &hash_device_id("bobs-other-device"))
        .await
        .expect("the record must run");

    assert_eq!(
        known_device_state(&pool, alice.id, Some(&device))
            .await
            .expect("the lookup must run"),
        Some(true)
    );
    assert_eq!(
        known_device_state(&pool, bob.id, Some(&device))
            .await
            .expect("the lookup must run"),
        Some(false),
        "bob has devices, and alice's is not one of them"
    );

    common::delete_user(&pool, alice.id).await;
    common::delete_user(&pool, bob.id).await;
}

// ── The write ────────────────────────────────────────────────────────────────

/// Signing in again from the same browser touches the one row rather than
/// growing the table, and leaves `first_seen_at` where it was.
#[tokio::test]
async fn recording_the_same_device_twice_touches_one_row() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;
    let hash = hash_device_id(&generate_device_id());

    record_device(&pool, user.id, &hash)
        .await
        .expect("the first record must run");
    let (first_seen, last_seen): (chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>) =
        sqlx::query_as("SELECT first_seen_at, last_seen_at FROM known_devices WHERE user_id = $1")
            .bind(user.id)
            .fetch_one(&pool)
            .await
            .expect("the row must be readable");

    record_device(&pool, user.id, &hash)
        .await
        .expect("the second record must run");

    assert_eq!(device_count(&pool, user.id).await, 1, "one row, not two");
    let (first_after, last_after): (chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>) =
        sqlx::query_as("SELECT first_seen_at, last_seen_at FROM known_devices WHERE user_id = $1")
            .bind(user.id)
            .fetch_one(&pool)
            .await
            .expect("the row must be readable");
    assert_eq!(first_after, first_seen, "first_seen_at is not moved");
    assert!(last_after >= last_seen, "last_seen_at is touched");

    common::delete_user(&pool, user.id).await;
}

/// A completed sign-in that submitted no device id records nothing, which is
/// what keeps an API client from filling the table with rows it can never
/// present again.
#[tokio::test]
async fn recording_an_unsubmitted_device_writes_nothing() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;

    record_submitted_device(&pool, user.id, None)
        .await
        .expect("the no-op must run");

    assert_eq!(device_count(&pool, user.id).await, 0);

    common::delete_user(&pool, user.id).await;
}

/// The `(user_id, device_id_hash)` UNIQUE is what makes the upsert an upsert. A
/// plain insert of the same pair is rejected by the database, so two rows one
/// device could match can never exist.
#[tokio::test]
async fn a_duplicate_device_for_one_user_is_rejected() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;
    let hash = hash_device_id(&generate_device_id());

    record_device(&pool, user.id, &hash)
        .await
        .expect("the record must run");

    let duplicate =
        sqlx::query("INSERT INTO known_devices (user_id, device_id_hash) VALUES ($1, $2)")
            .bind(user.id)
            .bind(&hash)
            .execute(&pool)
            .await;

    let code = duplicate
        .expect_err("a duplicate device must be rejected")
        .as_database_error()
        .and_then(|error| error.code().map(|code| code.to_string()));
    assert_eq!(
        code.as_deref(),
        Some("23505"),
        "the rejection is a unique violation"
    );

    common::delete_user(&pool, user.id).await;
}

// ── The login route, end to end ──────────────────────────────────────────────

/// A first-ever sign-in through the real route is not held, and it seeds the
/// device baseline. This is the case that would lock out every user if "no
/// recorded devices" were read as "new device": the account is created, signs
/// in, and completes, with the gate switched ON throughout.
#[tokio::test]
async fn the_login_route_baselines_a_first_sign_in_rather_than_holding_it() {
    let pool = common::test_pool().await;
    let config = common::test_config();
    assert!(
        config.mail.login_approval_enabled,
        "this case is only meaningful with the gate on"
    );
    let user = common::new_user(&pool).await;
    let device = generate_device_id();

    let (status, body) = login(&pool, &config, &user.email, Some(&device)).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "an account with no recorded device must not be held: {body}"
    );
    assert!(!body.contains("APPROVAL_REQUIRED"), "{body}");
    assert_eq!(
        known_device_state(&pool, user.id, Some(&device))
            .await
            .expect("the lookup must run"),
        Some(true),
        "the completed sign-in recorded its device"
    );

    common::delete_user(&pool, user.id).await;
}

/// The same browser signing in again is not held, because its device is now one
/// the account has used.
#[tokio::test]
async fn the_login_route_does_not_hold_a_known_device() {
    let pool = common::test_pool().await;
    let config = common::test_config();
    let user = common::new_user(&pool).await;
    let device = generate_device_id();

    login(&pool, &config, &user.email, Some(&device)).await;
    let (status, body) = login(&pool, &config, &user.email, Some(&device)).await;

    assert_eq!(status, StatusCode::OK, "a known device is not held: {body}");
    assert_eq!(device_count(&pool, user.id).await, 1, "still one device");

    common::delete_user(&pool, user.id).await;
}

/// A different browser on an account that already has one IS held: no session
/// is issued, the answer is `403 APPROVAL_REQUIRED`, and nothing about the new
/// device is recorded, so the attempt cannot make itself look familiar.
#[tokio::test]
async fn the_login_route_holds_a_new_device() {
    let pool = common::test_pool().await;
    let config = common::test_config();
    let user = common::new_user(&pool).await;

    login(&pool, &config, &user.email, Some("device-a")).await;
    let (status, body) = login(&pool, &config, &user.email, Some("device-b")).await;

    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert!(body.contains("APPROVAL_REQUIRED"), "{body}");
    assert!(
        !body.contains("refresh_token"),
        "a held sign-in issues nothing: {body}"
    );
    assert_eq!(
        known_device_state(&pool, user.id, Some("device-b"))
            .await
            .expect("the lookup must run"),
        Some(false),
        "the held device is not recorded"
    );

    let pending: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM pending_login_approvals WHERE user_id = $1")
            .bind(user.id)
            .fetch_one(&pool)
            .await
            .expect("the count must run");
    assert_eq!(pending, 1, "the hold wrote its approval row");

    common::delete_user(&pool, user.id).await;
}

/// A client that sends no device id at all is never held on the device signal,
/// through the real route, on an account that already has a device. This is the
/// fail-open half: an API client keeps working exactly as it does today.
#[tokio::test]
async fn the_login_route_does_not_hold_a_sign_in_with_no_device_id() {
    let pool = common::test_pool().await;
    let config = common::test_config();
    let user = common::new_user(&pool).await;

    login(&pool, &config, &user.email, Some("device-a")).await;
    let (status, body) = login(&pool, &config, &user.email, None).await;

    assert_eq!(
        status,
        StatusCode::OK,
        "an unsubmitted device id is not a new device: {body}"
    );
    assert_eq!(
        device_count(&pool, user.id).await,
        1,
        "and nothing new is recorded for it"
    );

    common::delete_user(&pool, user.id).await;
}

/// The kill switch disables BOTH triggers, not just the country one: with
/// `LOGIN_APPROVAL_ENABLED` off, a device the account has never seen completes.
#[tokio::test]
async fn the_kill_switch_off_lets_a_new_device_through() {
    let pool = common::test_pool().await;
    let mut config = common::test_config();
    let user = common::new_user(&pool).await;

    login(&pool, &config, &user.email, Some("device-a")).await;

    config.mail.login_approval_enabled = false;
    let (status, body) = login(&pool, &config, &user.email, Some("device-b")).await;

    assert_eq!(
        status,
        StatusCode::OK,
        "the kill switch must disable the device trigger too: {body}"
    );
    assert_eq!(
        device_count(&pool, user.id).await,
        2,
        "the completed sign-in recorded the new device"
    );

    common::delete_user(&pool, user.id).await;
}

/// Approving the hold makes the next sign-in from that same browser complete,
/// which is the loop the whole trigger depends on terminating.
#[tokio::test]
async fn approving_a_held_device_lets_the_next_sign_in_through() {
    let pool = common::test_pool().await;
    let config = common::test_config();
    let user = common::new_user(&pool).await;

    login(&pool, &config, &user.email, Some("device-a")).await;
    let (held, _) = login(&pool, &config, &user.email, Some("device-b")).await;
    assert_eq!(held, StatusCode::FORBIDDEN);

    // Claim the link the hold wrote, the way the emailed page does.
    let hash: Vec<u8> =
        sqlx::query_scalar("SELECT device_id_hash FROM pending_login_approvals WHERE user_id = $1")
            .bind(user.id)
            .fetch_one(&pool)
            .await
            .expect("the hold must carry the submitted device");
    assert_eq!(hash, hash_device_id("device-b"));
    record_device(&pool, user.id, &hash)
        .await
        .expect("the claim's promotion must run");

    let (status, body) = login(&pool, &config, &user.email, Some("device-b")).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "the approved device signs in ungated: {body}"
    );

    common::delete_user(&pool, user.id).await;
}

/// Sign in through the router `main.rs` mounts, with or without a device id.
async fn login(
    pool: &PgPool,
    config: &rusty_links::config::Config,
    email: &str,
    device_id: Option<&str>,
) -> (StatusCode, String) {
    let mut payload = serde_json::json!({
        "email": email,
        "password": common::TEST_PASSWORD,
    });
    if let Some(device_id) = device_id {
        payload["device_id"] = serde_json::Value::String(device_id.to_string());
    }

    let response = common::api_router(pool.clone(), config.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(payload.to_string()))
                .expect("request must build"),
        )
        .await
        .expect("router must answer");

    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("body must read");
    (status, String::from_utf8_lossy(&bytes).to_string())
}
