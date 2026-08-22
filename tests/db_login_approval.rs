//! LINKS-35 approval-gate lifecycle against a real Postgres (LINKS-44).
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

#![cfg(feature = "server")]

mod common;

use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use chrono::{DateTime, Duration, Utc};
use rusty_links::auth::login_approval::{
    consume_login_approval, create_router, get_login_approval, request_login_approval,
    ApprovalClaim, ApprovalFailure, ApprovalLookup, APPROVAL_PATH, APPROVAL_TTL_MINUTES,
};
use rusty_links::models::User;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tower::ServiceExt; // for `oneshot`
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
struct ApprovalRow {
    country: String,
    ip: String,
    device: Option<String>,
    expires_at: DateTime<Utc>,
    consumed_at: Option<DateTime<Utc>>,
}

async fn rows_for(pool: &PgPool, user_id: Uuid) -> Vec<ApprovalRow> {
    sqlx::query_as::<_, ApprovalRow>(
        "SELECT country, ip, device, expires_at, consumed_at
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
    let token = format!("test-token-{}", Uuid::new_v4());
    sqlx::query(
        "INSERT INTO pending_login_approvals (user_id, token_hash, country, ip, device, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(user_id)
    .bind(Sha256::digest(token.as_bytes()).to_vec())
    .bind(country)
    .bind("203.0.113.7")
    .bind(Some("seeded-agent"))
    .bind(Utc::now() + ttl)
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
        "DE",
        &headers("203.0.113.7", "curl/8.6.0"),
    )
    .await
    .expect("a held sign-in must write its row");

    let rows = rows_for(&pool, user.id).await;
    assert_eq!(rows.len(), 1, "exactly one pending row");
    assert_eq!(rows[0].country, "DE");
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
        request_login_approval(&pool, &config, user.id, &user.email, "DE", &head)
            .await
            .expect("the retry must be accepted");
    }
    assert_eq!(
        rows_for(&pool, user.id).await.len(),
        1,
        "the second attempt from DE reuses the live link"
    );

    request_login_approval(&pool, &config, user.id, &user.email, "FR", &head)
        .await
        .expect("a different country must be held on its own");
    let countries: Vec<String> = rows_for(&pool, user.id)
        .await
        .into_iter()
        .map(|row| row.country)
        .collect();
    assert_eq!(countries, vec!["DE".to_string(), "FR".to_string()]);

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
        "DE",
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
        "FR",
        &headers("203.0.113.7", "curl/8.6.0"),
    )
    .await
    .expect("the new sign-in must be held");

    let rows = rows_for(&pool, user.id).await;
    assert_eq!(rows.len(), 1, "the expired row was swept");
    assert_eq!(rows[0].country, "FR", "only the new row survives");

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
            assert_eq!(pending.country, "DE");
            assert_eq!(pending.ip, "203.0.113.7");
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
        Ok(ApprovalClaim::Claimed { user_id, country }) => {
            assert_eq!(user_id, user.id);
            assert_eq!(country, "DE");
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

/// Deleting the account takes its pending links with it, so a deleted user
/// leaves nothing claimable behind.
#[tokio::test]
async fn deleting_a_user_cascades_to_pending_rows() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;
    let token = seed(&pool, user.id, "DE", Duration::minutes(15)).await;
    seed(&pool, user.id, "FR", Duration::minutes(15)).await;
    assert_eq!(rows_for(&pool, user.id).await.len(), 2);

    common::delete_user(&pool, user.id).await;

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
