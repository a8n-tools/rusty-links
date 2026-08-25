//! LINKS-59: the refresh-token lifecycle against a real Postgres.
//!
//! `refresh_tokens.token` used to hold the value as issued, so a database read,
//! a dump or a backup yielded tokens exchangeable for a live session. A refresh
//! is session continuation rather than a sign-in, so replaying one never tripped
//! the LINKS-35 approval gate: no password, no approval mail, just a session.
//! `20260824000016_hash_refresh_tokens.sql` replaced the column with its
//! SHA-256, and these cases are what hold that shut.
//!
//! What is covered here is the pair of writes no `--lib` run reaches: the issue
//! path storing a digest and nothing else, and the refresh path resolving a
//! presented token through that digest, rotating it, and refusing everything
//! else. The column's shape itself is asserted in `db_schema.rs`.
//!
//! These cases run single-threaded (see scripts/check-db-tests-ran.nu) with the
//! rest of the database-backed suites.

#![cfg(feature = "server")]

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{DateTime, Duration, Utc};
use rusty_links::auth::jwt::hash_refresh_token;
use rusty_links::models::User;
use rusty_links::server_functions::auth::AuthResponse;
use sqlx::PgPool;
use tower::ServiceExt; // for `oneshot`
use uuid::Uuid;

/// Every stored row for one account, newest last.
async fn rows_for(pool: &PgPool, user_id: Uuid) -> Vec<(Vec<u8>, DateTime<Utc>)> {
    sqlx::query_as(
        "SELECT token_hash, expires_at FROM refresh_tokens WHERE user_id = $1 ORDER BY created_at",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .expect("refresh token rows must be readable")
}

/// Sign in through the real route, which is the only place a token is minted.
async fn login(pool: &PgPool, user: &User) -> AuthResponse {
    let response = common::api_router(pool.clone(), common::test_config())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(format!(
                    r#"{{"email":"{}","password":"{}"}}"#,
                    user.email,
                    common::TEST_PASSWORD
                )))
                .expect("request must build"),
        )
        .await
        .expect("router must answer");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "the sign-in must complete"
    );

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body must read");
    serde_json::from_slice(&bytes).expect("the sign-in body must be an AuthResponse")
}

/// Present a token to the refresh route, returning the status and the body.
async fn refresh(pool: &PgPool, token: &str) -> (StatusCode, Vec<u8>) {
    let response = common::api_router(pool.clone(), common::test_config())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/refresh")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "refresh_token": token }).to_string(),
                ))
                .expect("request must build"),
        )
        .await
        .expect("router must answer");

    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body must read");
    (status, bytes.to_vec())
}

/// A row the test knows the token for, hashed the way the application does.
/// `ttl` may be negative, which is how an already-expired row is made.
async fn seed(pool: &PgPool, user_id: Uuid, ttl: Duration) -> String {
    let token = format!("test-refresh-{}", Uuid::new_v4());
    sqlx::query("INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(hash_refresh_token(&token).as_slice())
        .bind(Utc::now() + ttl)
        .execute(pool)
        .await
        .expect("fixture row must insert");
    token
}

// ── Issuing ──────────────────────────────────────────────────────────────────

/// The row holds the digest and nothing else. Whoever reads the table holds a
/// value the refresh route will not accept.
#[tokio::test]
async fn issuing_a_session_stores_only_the_digest() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;

    let issued = login(&pool, &user).await;
    let rows = rows_for(&pool, user.id).await;

    assert_eq!(rows.len(), 1, "one sign-in issues one token");
    let (stored, _) = &rows[0];
    assert_eq!(stored.len(), 32, "a SHA-256 is 32 bytes");
    assert_eq!(*stored, hash_refresh_token(&issued.refresh_token));
    assert_ne!(
        stored.as_slice(),
        issued.refresh_token.as_bytes(),
        "the token itself must never be at rest"
    );

    common::delete_user(&pool, user.id).await;
}

// ── Presenting ───────────────────────────────────────────────────────────────

/// The lookup is by the hash of what was presented, so the token a client holds
/// still finds its row.
#[tokio::test]
async fn a_presented_token_resolves_through_its_hash() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;

    let issued = login(&pool, &user).await;
    let (status, body) = refresh(&pool, &issued.refresh_token).await;
    assert_eq!(status, StatusCode::OK);

    let rotated: AuthResponse =
        serde_json::from_slice(&body).expect("the refresh body must be an AuthResponse");
    assert_eq!(rotated.email, user.email);
    assert_ne!(
        rotated.refresh_token, issued.refresh_token,
        "a refresh hands back a new token"
    );

    common::delete_user(&pool, user.id).await;
}

/// Nothing but the token itself resolves: not a token that was never issued,
/// and not the digest a database reader would be holding.
#[tokio::test]
async fn a_token_that_was_not_issued_is_rejected() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;

    let issued = login(&pool, &user).await;
    assert_eq!(
        refresh(&pool, "never-issued").await.0,
        StatusCode::UNAUTHORIZED
    );

    // What a dump yields is the digest. Presenting it hashes it a second time,
    // so it matches nothing, which is the whole point of the column.
    let digest = hex::encode(hash_refresh_token(&issued.refresh_token));
    assert_eq!(refresh(&pool, &digest).await.0, StatusCode::UNAUTHORIZED);

    assert_eq!(
        rows_for(&pool, user.id).await.len(),
        1,
        "a rejected refresh burns nothing"
    );

    common::delete_user(&pool, user.id).await;
}

// ── Rotation ─────────────────────────────────────────────────────────────────

/// A refresh rotates: the presented row is gone and the token that named it is
/// dead, so a captured token is usable at most once.
#[tokio::test]
async fn refreshing_burns_the_row_it_rotated() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;

    let issued = login(&pool, &user).await;
    let before = rows_for(&pool, user.id).await;

    let (status, body) = refresh(&pool, &issued.refresh_token).await;
    assert_eq!(status, StatusCode::OK);
    let rotated: AuthResponse = serde_json::from_slice(&body).expect("body must parse");

    let after = rows_for(&pool, user.id).await;
    assert_eq!(after.len(), 1, "the old row is replaced, not added to");
    assert_ne!(after[0].0, before[0].0, "the stored digest is the new one");

    assert_eq!(
        refresh(&pool, &issued.refresh_token).await.0,
        StatusCode::UNAUTHORIZED,
        "the rotated token must not work twice"
    );
    assert_eq!(
        refresh(&pool, &rotated.refresh_token).await.0,
        StatusCode::OK,
        "the token the rotation handed back must work"
    );

    common::delete_user(&pool, user.id).await;
}

/// The guarded `DELETE ... RETURNING` is what decides the race, the same way the
/// LINKS-35 claim does: both refreshes name the row, only one deletes it, so one
/// token can never become two sessions.
#[tokio::test]
async fn one_token_yields_one_session_under_concurrency() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;

    let issued = login(&pool, &user).await;
    let (first, second) = tokio::join!(
        refresh(&pool, &issued.refresh_token),
        refresh(&pool, &issued.refresh_token)
    );

    let won = [first.0, second.0]
        .iter()
        .filter(|status| **status == StatusCode::OK)
        .count();
    assert_eq!(
        won, 1,
        "exactly one concurrent refresh may claim the token, got {:?} and {:?}",
        first.0, second.0
    );
    assert_eq!(
        rows_for(&pool, user.id).await.len(),
        1,
        "the loser must not leave a second live session behind"
    );

    common::delete_user(&pool, user.id).await;
}

// ── Expiry ───────────────────────────────────────────────────────────────────

/// An expired token is refused and its row is dropped by the refresh itself,
/// rather than waiting for the scheduler's sweep.
#[tokio::test]
async fn an_expired_token_is_rejected_and_its_row_removed() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;
    let token = seed(&pool, user.id, Duration::minutes(-1)).await;

    assert_eq!(refresh(&pool, &token).await.0, StatusCode::UNAUTHORIZED);
    assert!(
        rows_for(&pool, user.id).await.is_empty(),
        "the expired row is claimed and dropped"
    );

    common::delete_user(&pool, user.id).await;
}

// ── The migration's backfill ─────────────────────────────────────────────────

/// A row written before `20260824000016` still refreshes. The migration hashed
/// the stored plaintext in place with `sha256(convert_to(token, 'UTF8'))`, and
/// this seeds a row through that exact expression, so the case fails the moment
/// Postgres and `hash_refresh_token` stop agreeing on the bytes. That agreement
/// is the whole reason the migration signs nobody out.
#[tokio::test]
async fn a_row_the_migration_backfilled_still_refreshes() {
    let pool = common::test_pool().await;
    let user = common::new_user(&pool).await;

    let token = format!("pre-migration-{}", Uuid::new_v4());
    sqlx::query(
        "INSERT INTO refresh_tokens (user_id, token_hash, expires_at)
         VALUES ($1, sha256(convert_to($2, 'UTF8')), $3)",
    )
    .bind(user.id)
    .bind(&token)
    .bind(Utc::now() + Duration::days(7))
    .execute(&pool)
    .await
    .expect("the backfilled row must insert");

    let stored = rows_for(&pool, user.id).await;
    assert_eq!(
        stored[0].0,
        hash_refresh_token(&token),
        "Postgres and Rust must produce the same digest"
    );
    assert_eq!(
        refresh(&pool, &token).await.0,
        StatusCode::OK,
        "a token issued before the migration must still refresh"
    );

    common::delete_user(&pool, user.id).await;
}
