//! LINKS-35 / LINKS-45 approval-gate lifecycle against a real Postgres (LINKS-44).
//!
//! Every statement in `src/auth/login_approval.rs` used to be compile-checked
//! only: the suite ran against a lazy pool pointed at a closed port, so the
//! dedup select, the guarded claim and its affected-row count, the failure
//! classification, the expiry sweep and the migration's own `UNIQUE` /
//! `ON DELETE CASCADE` were verified by hand once and never again.
//!
//! Only the token's SHA-256 is stored and the plaintext is never returned, so
//! the read and claim cases seed their own row with a token they know. The
//! production insert is covered separately, through `request_login_approval`.
//!
//! These cases run single-threaded (see scripts/check-db-tests-ran.nu): the
//! sweep deletes every expired row in the database, so two cases at once would
//! delete each other's expired fixture.
//!
//! The device trigger's own reads and writes live in `db_known_devices.rs`;
//! what is covered here is the half that runs through a pending row: the hold
//! carrying the submitted device, and the claim promoting it.

#![cfg(feature = "server")]

mod common;

use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use chrono::{DateTime, Duration, Utc};
use rusty_links::auth::known_device::{hash_device_id, known_device_state};
use rusty_links::auth::login_approval::{
    consume_login_approval, create_router, get_login_approval, request_login_approval,
    ApprovalClaim, ApprovalFailure, ApprovalLookup, Hold, HoldReason, APPROVAL_PATH,
    APPROVAL_TTL_MINUTES,
};
use rusty_links::models::User;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tower::ServiceExt; // for `oneshot`
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
struct ApprovalRow {
    country: Option<String>,
    ip: String,
    device: Option<String>,
    expires_at: DateTime<Utc>,
    consumed_at: Option<DateTime<Utc>>,
    reason: String,
    device_id_hash: Option<Vec<u8>>,
}

/// A country hold, the LINKS-35 shape, so the cases that are about the country
/// read the same as they did before LINKS-45.
fn country_hold(country: &str) -> Hold {
    Hold {
        reason: HoldReason::NewCountry,
        country: Some(country.to_string()),
    }
}

async fn rows_for(pool: &PgPool, user_id: Uuid) -> Vec<ApprovalRow> {
    sqlx::query_as::<_, ApprovalRow>(
        "SELECT country, ip, device, expires_at, consumed_at, reason, device_id_hash
         FROM pending_login_approvals WHERE user_id = $1 ORDER BY created_at",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .expect("pending rows must be readable")
}

/// Seed a row the test knows the token for. `ttl` may be negative, which is how
/// an already-expired row is made.
async fn seed(pool: &PgPool, user_id: Uuid, country: &str, ttl: Duration) -> String {
    seed_row(
        pool,
        user_id,
        Some(country),
        ttl,
        HoldReason::NewCountry,
        None,
    )
    .await
}

/// The general form: a hold on either trigger, with or without a country and
/// with or without a submitted device.
async fn seed_row(
    pool: &PgPool,
    user_id: Uuid,
    country: Option<&str>,
    ttl: Duration,
    reason: HoldReason,
    device_id: Option<&str>,
) -> String {
    let token = format!("test-token-{}", Uuid::new_v4());
    sqlx::query(
        "INSERT INTO pending_login_approvals
            (user_id, token_hash, country, ip, device, expires_at, reason, device_id_hash)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(user_id)
    .bind(Sha256::digest(token.as_bytes()).to_vec())
    .bind(country)
    .bind("203.0.113.7")
    .bind(Some("seeded-agent"))
    .bind(Utc::now() + ttl)
    .bind(reason.as_str())
    .bind(device_id.map(hash_device_id))
    .execute(pool)
    .await
    .expect("fixture row must insert");
    token
}

fn headers(ip: &str, agent: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("X-Forwarded-For", ip.parse().expect("header value"));
    headers.insert("User-Agent", agent.parse().expect("header value"));
    headers
}

/// Make a device known to an account, as a completed sign-in would.
async fn seed_known_device(pool: &PgPool, user_id: Uuid, device_id: &str) {
    sqlx::query("INSERT INTO known_devices (user_id, device_id_hash) VALUES ($1, $2)")
        .bind(user_id)
        .bind(hash_device_id(device_id))
        .execute(pool)
        .await
        .expect("fixture device must insert");
}

async fn consumed_at(pool: &PgPool, token: &str) -> Option<DateTime<Utc>> {
    sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
        "SELECT consumed_at FROM pending_login_approvals WHERE token_hash = $1",
    )
    .bind(Sha256::digest(token.as_bytes()).to_vec())
    .fetch_one(pool)
    .await
    .expect("the seeded row must still be there")
}

// ── Insert and dedup ─────────────────────────────────────────────────────────

/// The insert records what the owner needs to judge the attempt, and dates the
/// row from the TTL constant rather than from whatever the caller passed.
#[tokio::test]
async fn holding_a_sign_in_records_one_row() {
    let pool = common::test_pool().await;
    let config = common::test_config();
    let user = common::new_user(&pool).await;

    request_login_approval(
        &pool,
        &config,
        user.id,
        &user.email,
        &country_hold("DE"),
        None,
        &headers("203.0.113.7", "curl/8.6.0"),
    )
    .await
    .expect("a held sign-in must write its row");

    let rows = rows_for(&pool, user.id).await;
    assert_eq!(rows.len(), 1, "exactly one pending row");
    assert_eq!(rows[0].country.as_deref(), Some("DE"));
    assert_eq!(rows[0].reason, "new_country");
    assert_eq!(rows[0].device_id_hash, None, "no device id was submitted");
    assert_eq!(rows[0].ip, "203.0.113.7");
    assert_eq!(rows[0].device.as_deref(), Some("curl/8.6.0"));
    assert!(rows[0].consumed_at.is_none(), "a fresh row is unconsumed");

    let ttl = rows[0].expires_at - Utc::now();
    assert!(
        ttl > Duration::minutes(APPROVAL_TTL_MINUTES - 1)
            && ttl <= Duration::minutes(APPROVAL_TTL_MINUTES),
        "expiry is now + APPROVAL_TTL_MINUTES, got {ttl}"
    );

    common::delete_user(&pool, user.id).await;
}

/// The dedup select is what stops a retried sign-in mailing a second link,
/// and it is scoped per country rather than per user.
#[tokio::test]
async fn a_live_link_suppresses_a_second_one_for_the_same_country() {
    let pool = common::test_pool().await;
    let config = common::test_config();
    let user = common::new_user(&pool).await;
    let head = headers("203.0.113.7", "curl/8.6.0");

    for _ in 0..2 {
        request_login_approval(
            &pool,
            &config,
            user.id,
            &user.email,
            &country_hold("DE"),
            None,
            &head,
        )
        .await
        .expect("the retry must be accepted");
    }
    assert_eq!(
        rows_for(&pool, user.id).await.len(),
        1,
        "the second attempt from DE reuses the live link"
    );

    request_login_approval(
        &pool,
        &config,
        user.id,
        &user.email,
        &country_hold("FR"),
        None,
        &head,
    )
    .await
    .expect("a different country must be held on its own");
    let countries: Vec<Option<String>> = rows_for(&pool, user.id)
        .await
        .into_iter()
        .map(|row| row.country)
        .collect();
    assert_eq!(
        countries,
        vec![Some("DE".to_string()), Some("FR".to_string())]
    );

    common::delete_user(&pool, user.id).await;
}

/// A consumed row is not a live one, so signing in again from the same country
/// is held afresh instead of being deduped against a spent link.
#[tokio::test]
async fn a_consumed_link_does_not_suppress_the_next_one() {
    let pool = common::test_pool().await;
    let config = common::test_config();
    let user = common::new_user(&pool).await;
    let token = seed(&pool, user.id, "DE", Duration::minutes(15)).await;

    assert!(matches!(
        consume_login_approval(&pool, &token).await,
        Ok(ApprovalClaim::Claimed { .. })
    ));

    request_login_approval(
        &pool,
        &config,
        user.id,
        &user.email,
        &country_hold("DE"),
        None,
        &headers("203.0.113.7", "curl/8.6.0"),
    )
    .await
    .expect("the next sign-in must be held");

    assert_eq!(
        rows_for(&pool, user.id).await.len(),
        2,
        "the spent row stays, and a new live one joins it"
    );

    common::delete_user(&pool, user.id).await;
}

/// The sweep runs on the request path, so an expired row does not accumulate.
#[tokio::test]
async fn requesting_an_approval_sweeps_expired_rows() {
    let pool = common::test_pool().await;
    let config = common::test_config();
    let user = common::new_user(&pool).await;
    seed(&pool, user.id, "DE", Duration::minutes(-1)).await;
    assert_eq!(rows_for(&pool, user.id).await.len(), 1);

    request_login_approval(
        &pool,
        &config,
        user.id,
        &user.email,
        &country_hold("FR"),
        None,
        &headers("203.0.113.7", "curl/8.6.0"),
    )
    .await
    .expect("the new sign-in must be held");

    let rows = rows_for(&pool, user.id).await;
    assert_eq!(rows.len(), 1, "the expired row was swept");
    assert_eq!(
        rows[0].country.as_deref(),
        Some("FR"),
        "only the new row survives"
    );

    common::delete_user(&pool, user.id).await;
}

// ── Reads ────────────────────────────────────────────────────────────────────

/// A read, including the one a link scanner triggers, must leave the row
/// claimable.
#[tokio::test]
async fn reading_an_approval_does_not_consume_it() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;
    let token = seed(&pool, user.id, "DE", Duration::minutes(15)).await;

    match get_login_approval(&pool, &token).await {
        Ok(ApprovalLookup::Valid(pending)) => {
            assert_eq!(pending.country.as_deref(), Some("DE"));
            assert_eq!(pending.ip, "203.0.113.7");
            assert_eq!(pending.reason, HoldReason::NewCountry);
        }
        _ => panic!("a live token must read as valid"),
    }
    assert!(consumed_at(&pool, &token).await.is_none());

    let response = create_router(pool.clone())
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("{APPROVAL_PATH}?token={token}"))
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("router must answer");
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        consumed_at(&pool, &token).await.is_none(),
        "a GET must never burn the link"
    );

    common::delete_user(&pool, user.id).await;
}

/// The read tells the three failures apart, which is what lets the page say
/// "already used" rather than "not valid".
#[tokio::test]
async fn reads_classify_missing_used_and_expired_tokens() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;

    let missing = get_login_approval(&pool, "never-issued").await;
    assert!(matches!(
        missing,
        Ok(ApprovalLookup::Invalid(ApprovalFailure::NotFound))
    ));

    let used = seed(&pool, user.id, "DE", Duration::minutes(15)).await;
    consume_login_approval(&pool, &used)
        .await
        .expect("the claim must run");
    assert!(matches!(
        get_login_approval(&pool, &used).await,
        Ok(ApprovalLookup::Invalid(ApprovalFailure::AlreadyUsed))
    ));

    let expired = seed(&pool, user.id, "FR", Duration::minutes(-1)).await;
    assert!(matches!(
        get_login_approval(&pool, &expired).await,
        Ok(ApprovalLookup::Invalid(ApprovalFailure::Expired))
    ));

    common::delete_user(&pool, user.id).await;
}

// ── Claims ───────────────────────────────────────────────────────────────────

/// The guarded UPDATE matches once. The second claim affects no row, which is
/// why `consumed_at` does not move and the page reports "already used".
#[tokio::test]
async fn a_link_claims_once_then_never_again() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;
    let token = seed(&pool, user.id, "DE", Duration::minutes(15)).await;

    match consume_login_approval(&pool, &token).await {
        Ok(ApprovalClaim::Claimed {
            user_id, country, ..
        }) => {
            assert_eq!(user_id, user.id);
            assert_eq!(country.as_deref(), Some("DE"));
        }
        _ => panic!("the first claim must win"),
    }
    let first = consumed_at(&pool, &token)
        .await
        .expect("consumed_at is set");

    assert!(
        matches!(
            consume_login_approval(&pool, &token).await,
            Ok(ApprovalClaim::Rejected(ApprovalFailure::AlreadyUsed))
        ),
        "the second claim must be rejected"
    );
    assert_eq!(
        consumed_at(&pool, &token).await,
        Some(first),
        "the second claim affected no row, so the timestamp is unchanged"
    );

    common::delete_user(&pool, user.id).await;
}

/// Two clicks at once: the row lock serialises them and exactly one matches.
#[tokio::test]
async fn concurrent_claims_produce_exactly_one_winner() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;
    let token = seed(&pool, user.id, "DE", Duration::minutes(15)).await;

    let (left, right) = {
        let (pool_a, pool_b) = (pool.clone(), pool.clone());
        let (token_a, token_b) = (token.clone(), token.clone());
        tokio::join!(
            tokio::spawn(async move { consume_login_approval(&pool_a, &token_a).await }),
            tokio::spawn(async move { consume_login_approval(&pool_b, &token_b).await }),
        )
    };

    let outcomes = [
        left.expect("task must not panic").expect("claim must run"),
        right.expect("task must not panic").expect("claim must run"),
    ];
    let winners = outcomes
        .iter()
        .filter(|outcome| matches!(outcome, ApprovalClaim::Claimed { .. }))
        .count();
    let losers = outcomes
        .iter()
        .filter(|outcome| {
            matches!(
                outcome,
                ApprovalClaim::Rejected(ApprovalFailure::AlreadyUsed)
            )
        })
        .count();

    assert_eq!(winners, 1, "exactly one concurrent claim may win");
    assert_eq!(losers, 1, "the loser is told the link was already used");

    common::delete_user(&pool, user.id).await;
}

/// An expired row is never claimable, and the failed claim leaves it untouched
/// rather than marking it consumed.
#[tokio::test]
async fn an_expired_link_claims_nothing() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;
    let token = seed(&pool, user.id, "DE", Duration::minutes(-1)).await;

    assert!(matches!(
        consume_login_approval(&pool, &token).await,
        Ok(ApprovalClaim::Rejected(ApprovalFailure::Expired))
    ));
    assert!(
        consumed_at(&pool, &token).await.is_none(),
        "a rejected claim must not consume the row"
    );

    common::delete_user(&pool, user.id).await;
}

/// A token approves its own row's sign-in and nobody else's, because the user
/// id comes off the claimed row rather than from the caller.
#[tokio::test]
async fn a_token_only_ever_names_its_own_user() {
    let pool = common::test_pool().await;
    let alice = common::new_user(&pool).await;
    let bob = common::new_user(&pool).await;
    let token = seed(&pool, bob.id, "FR", Duration::minutes(15)).await;

    match consume_login_approval(&pool, &token).await {
        Ok(ApprovalClaim::Claimed { user_id, .. }) => {
            assert_eq!(user_id, bob.id);
            assert_ne!(user_id, alice.id);
        }
        _ => panic!("bob's token must claim bob's row"),
    }
    assert!(
        rows_for(&pool, alice.id).await.is_empty(),
        "alice has no pending rows to disturb"
    );

    common::delete_user(&pool, alice.id).await;
    common::delete_user(&pool, bob.id).await;
}

// ── Approval completes the sign-in ───────────────────────────────────────────

/// Approving records the country as the account's known one, so the next
/// sign-in from there is no longer new. That write is the whole point of the
/// POST, and it is the LINKS-27 column.
#[tokio::test]
async fn approving_records_the_country_on_the_user() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;
    let token = seed(&pool, user.id, "DE", Duration::minutes(15)).await;

    let before = User::get_login_location(&pool, user.id)
        .await
        .expect("read must succeed")
        .expect("user must exist");
    assert_eq!(before.1, None, "no country is known before approval");

    let response = create_router(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(APPROVAL_PATH)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::from(format!("token={token}")))
                .expect("request must build"),
        )
        .await
        .expect("router must answer");
    assert_eq!(response.status(), StatusCode::OK);

    let after = User::get_login_location(&pool, user.id)
        .await
        .expect("read must succeed")
        .expect("user must exist");
    assert_eq!(
        after.1.as_deref(),
        Some("DE"),
        "the approved country is now the account's known one"
    );
    assert!(
        consumed_at(&pool, &token).await.is_some(),
        "the POST claims the link"
    );

    common::delete_user(&pool, user.id).await;
}

// ── The device trigger through a pending row (LINKS-45) ──────────────────────

/// A device-only hold carries no country when nothing resolved one, which is
/// the default deployment, and records which trigger fired so the page and the
/// mail say "device" rather than naming a country that was never new.
#[tokio::test]
async fn a_device_only_hold_records_its_reason_and_no_country() {
    let pool = common::test_pool().await;
    let config = common::test_config();
    let user = common::new_user(&pool).await;
    let device = hash_device_id("device-b");

    request_login_approval(
        &pool,
        &config,
        user.id,
        &user.email,
        &Hold {
            reason: HoldReason::NewDevice,
            country: None,
        },
        Some(&device),
        &headers("203.0.113.7", "curl/8.6.0"),
    )
    .await
    .expect("a device-only hold must write its row");

    let rows = rows_for(&pool, user.id).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].country, None, "no country resolved, so none stored");
    assert_eq!(rows[0].reason, "new_device");
    assert_eq!(
        rows[0].device_id_hash.as_deref(),
        Some(device.as_slice()),
        "the submitted device rides on the row so the claim can promote it"
    );

    common::delete_user(&pool, user.id).await;
}

/// The dedup key is the attempt, not just the country: retrying from the same
/// browser reuses the live link, while a different browser is a distinct event
/// that gets its own link and its own mail.
#[tokio::test]
async fn the_dedup_is_per_attempt_not_per_user() {
    let pool = common::test_pool().await;
    let config = common::test_config();
    let user = common::new_user(&pool).await;
    let head = headers("203.0.113.7", "curl/8.6.0");
    let hold = Hold {
        reason: HoldReason::NewDevice,
        country: None,
    };

    let first = hash_device_id("device-b");
    for _ in 0..2 {
        request_login_approval(
            &pool,
            &config,
            user.id,
            &user.email,
            &hold,
            Some(&first),
            &head,
        )
        .await
        .expect("the retry must be accepted");
    }
    assert_eq!(
        rows_for(&pool, user.id).await.len(),
        1,
        "the same browser retrying reuses the live link"
    );

    let second = hash_device_id("device-c");
    request_login_approval(
        &pool,
        &config,
        user.id,
        &user.email,
        &hold,
        Some(&second),
        &head,
    )
    .await
    .expect("a different device must be held on its own");
    assert_eq!(
        rows_for(&pool, user.id).await.len(),
        2,
        "a different browser is a distinct attempt and gets its own link"
    );

    common::delete_user(&pool, user.id).await;
}

/// A client that varies its device id every attempt cannot mail an unbounded
/// stream of links. The cap still holds the sign-in; it only stops the mail.
#[tokio::test]
async fn live_approval_links_are_capped_per_user() {
    let pool = common::test_pool().await;
    let config = common::test_config();
    let user = common::new_user(&pool).await;
    let head = headers("203.0.113.7", "curl/8.6.0");
    let hold = Hold {
        reason: HoldReason::NewDevice,
        country: None,
    };

    for attempt in 0..6 {
        let device = hash_device_id(&format!("device-{attempt}"));
        request_login_approval(
            &pool,
            &config,
            user.id,
            &user.email,
            &hold,
            Some(&device),
            &head,
        )
        .await
        .expect("every attempt is still held, capped or not");
    }

    assert_eq!(
        rows_for(&pool, user.id).await.len(),
        3,
        "the cap bounds the live links, and so the mail, at three"
    );

    common::delete_user(&pool, user.id).await;
}

/// The whole point of the device trigger terminating: approving promotes the
/// device the HELD sign-in submitted into `known_devices`, so the next sign-in
/// from that browser is no longer new. Recording it from the request rather
/// than from a response is what makes this the held browser and not the one
/// that happened to open the mail.
#[tokio::test]
async fn approving_records_the_device_so_the_next_sign_in_is_not_held() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;
    let other = common::new_user(&pool).await;

    // Give the account a device first, so it has a baseline and "device-b" is
    // genuinely new rather than the never-held zero-devices case.
    seed_known_device(&pool, user.id, "device-a").await;
    assert_eq!(
        known_device_state(&pool, user.id, Some("device-b"))
            .await
            .expect("the lookup must run"),
        Some(false),
        "the account has devices and this is not one of them"
    );

    let token = seed_row(
        &pool,
        user.id,
        None,
        Duration::minutes(15),
        HoldReason::NewDevice,
        Some("device-b"),
    )
    .await;

    let response = create_router(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(APPROVAL_PATH)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::from(format!("token={token}")))
                .expect("request must build"),
        )
        .await
        .expect("router must answer");
    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        known_device_state(&pool, user.id, Some("device-b"))
            .await
            .expect("the lookup must run"),
        Some(true),
        "the approved device is now one the account has signed in from"
    );
    assert_eq!(
        known_device_state(&pool, other.id, Some("device-b"))
            .await
            .expect("the lookup must run"),
        None,
        "approving for one account records nothing for another"
    );

    common::delete_user(&pool, user.id).await;
    common::delete_user(&pool, other.id).await;
}

/// An attempt nobody approves records nothing, so it cannot make its device
/// look familiar on the next try. This is the invariant that stops the gate
/// from being talked out of itself.
#[tokio::test]
async fn an_unapproved_hold_never_records_its_device() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;
    seed_known_device(&pool, user.id, "device-a").await;

    seed_row(
        &pool,
        user.id,
        None,
        Duration::minutes(15),
        HoldReason::NewDevice,
        Some("device-b"),
    )
    .await;

    assert_eq!(
        known_device_state(&pool, user.id, Some("device-b"))
            .await
            .expect("the lookup must run"),
        Some(false),
        "writing the pending row records no device"
    );

    common::delete_user(&pool, user.id).await;
}

/// A sign-in that trips BOTH triggers is held once and approved once: the one
/// claim records both baselines, so the next sign-in from that browser in that
/// country is new on neither and the user is never asked twice.
#[tokio::test]
async fn approving_a_both_triggers_hold_records_both_baselines() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;
    seed_known_device(&pool, user.id, "device-a").await;

    let token = seed_row(
        &pool,
        user.id,
        Some("DE"),
        Duration::minutes(15),
        HoldReason::NewCountryAndDevice,
        Some("device-b"),
    )
    .await;

    let response = create_router(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(APPROVAL_PATH)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::from(format!("token={token}")))
                .expect("request must build"),
        )
        .await
        .expect("router must answer");
    assert_eq!(response.status(), StatusCode::OK);

    let after = User::get_login_location(&pool, user.id)
        .await
        .expect("read must succeed")
        .expect("user must exist");
    assert_eq!(after.1.as_deref(), Some("DE"), "the country was recorded");
    assert_eq!(
        known_device_state(&pool, user.id, Some("device-b"))
            .await
            .expect("the lookup must run"),
        Some(true),
        "and so was the device, from the same single approval"
    );

    common::delete_user(&pool, user.id).await;
}

/// A hold whose client submitted no device id claims cleanly and records no
/// device, so the country half keeps working for a client that has none.
#[tokio::test]
async fn approving_a_hold_with_no_device_records_only_the_country() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;
    let token = seed(&pool, user.id, "DE", Duration::minutes(15)).await;

    match consume_login_approval(&pool, &token).await {
        Ok(ApprovalClaim::Claimed {
            country,
            device_id_hash,
            ..
        }) => {
            assert_eq!(country.as_deref(), Some("DE"));
            assert_eq!(device_id_hash, None, "nothing to promote");
        }
        _ => panic!("the claim must win"),
    }

    let devices: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM known_devices WHERE user_id = $1")
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .expect("the count must run");
    assert_eq!(
        devices, 0,
        "no device id was submitted, so none is recorded"
    );

    common::delete_user(&pool, user.id).await;
}

// ── Migration constraints ────────────────────────────────────────────────────

/// `token_hash` is UNIQUE, so a hash collision is rejected by the database
/// instead of producing two rows one token can reach.
#[tokio::test]
async fn a_duplicate_token_hash_is_rejected() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;
    let token = seed(&pool, user.id, "DE", Duration::minutes(15)).await;

    let duplicate = sqlx::query(
        "INSERT INTO pending_login_approvals (user_id, token_hash, country, ip, expires_at)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(user.id)
    .bind(Sha256::digest(token.as_bytes()).to_vec())
    .bind("FR")
    .bind("203.0.113.9")
    .bind(Utc::now() + Duration::minutes(15))
    .execute(&pool)
    .await;

    let code = duplicate
        .expect_err("a duplicate token_hash must be rejected")
        .as_database_error()
        .and_then(|error| error.code().map(|code| code.to_string()));
    assert_eq!(
        code.as_deref(),
        Some("23505"),
        "the rejection is a unique violation"
    );

    common::delete_user(&pool, user.id).await;
}

/// Deleting the account takes its pending links and its known devices with it,
/// so a deleted user leaves nothing claimable and nothing recognisable behind.
#[tokio::test]
async fn deleting_a_user_cascades_to_pending_rows() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;
    let token = seed(&pool, user.id, "DE", Duration::minutes(15)).await;
    seed(&pool, user.id, "FR", Duration::minutes(15)).await;
    seed_known_device(&pool, user.id, "device-a").await;
    assert_eq!(rows_for(&pool, user.id).await.len(), 2);

    common::delete_user(&pool, user.id).await;

    let devices: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM known_devices WHERE user_id = $1")
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .expect("the count must run");
    assert_eq!(devices, 0, "ON DELETE CASCADE removed the known devices");

    assert!(
        rows_for(&pool, user.id).await.is_empty(),
        "ON DELETE CASCADE removed the pending rows"
    );
    assert!(
        matches!(
            get_login_approval(&pool, &token).await,
            Ok(ApprovalLookup::Invalid(ApprovalFailure::NotFound))
        ),
        "the emailed link is dead once the account is gone"
    );
}
