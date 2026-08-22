//! LINKS-33 account settings, end to end through the column (LINKS-44).
//!
//! The router- and serde-level cases live in `src/api/auth.rs`; what they
//! cannot show is which row a write reached. These drive `GET`/`PATCH
//! /api/auth/me` over a real pool and read the answer back out of `users`.

#![cfg(feature = "server")]

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use rusty_links::models::User;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt; // for `oneshot`
use uuid::Uuid;

/// Stored value, read straight from the column rather than from the response.
async fn stored_flag(pool: &PgPool, id: Uuid) -> bool {
    let (_, _, notify) = User::get_login_location(pool, id)
        .await
        .expect("the login-location read must succeed")
        .expect("the user must still exist");
    notify
}

async fn call(
    router: axum::Router,
    method: &str,
    auth: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri("/auth/me")
        .header("Authorization", auth);
    if body.is_some() {
        builder = builder.header("Content-Type", "application/json");
    }
    let request = builder
        .body(match &body {
            Some(value) => Body::from(value.to_string()),
            None => Body::empty(),
        })
        .expect("request must build");

    let response = router.oneshot(request).await.expect("router must answer");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body must read");
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

/// The GET reports what is in the column, not a hardcoded default.
#[tokio::test]
async fn me_reports_the_stored_flag() {
    let pool = common::test_pool().await;
    let config = common::test_config();
    let user = common::new_user(&pool).await;
    let auth = common::bearer(&config, &user);

    let (status, body) = call(
        common::api_router(pool.clone(), config.clone()),
        "GET",
        &auth,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["notify_new_location"], json!(true));
    assert_eq!(body["id"], json!(user.id.to_string()));

    // Flip it underneath the router; the next GET must follow the column.
    User::set_notify_new_location(&pool, user.id, false)
        .await
        .expect("write must succeed");
    let (status, body) = call(common::api_router(pool.clone(), config), "GET", &auth, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["notify_new_location"], json!(false));

    common::delete_user(&pool, user.id).await;
}

/// Both directions persist, and the response echoes what was stored.
#[tokio::test]
async fn patch_persists_false_then_true() {
    let pool = common::test_pool().await;
    let config = common::test_config();
    let user = common::new_user(&pool).await;
    let auth = common::bearer(&config, &user);

    let (status, body) = call(
        common::api_router(pool.clone(), config.clone()),
        "PATCH",
        &auth,
        Some(json!({"notify_new_location": false})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["notify_new_location"], json!(false));
    assert!(!stored_flag(&pool, user.id).await, "false must persist");

    let (status, body) = call(
        common::api_router(pool.clone(), config),
        "PATCH",
        &auth,
        Some(json!({"notify_new_location": true})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["notify_new_location"], json!(true));
    assert!(stored_flag(&pool, user.id).await, "true must persist back");

    common::delete_user(&pool, user.id).await;
}

/// An absent key means "not submitted": the stored value survives the patch.
#[tokio::test]
async fn patch_without_the_key_leaves_the_column_alone() {
    let pool = common::test_pool().await;
    let config = common::test_config();
    let user = common::new_user(&pool).await;
    let auth = common::bearer(&config, &user);

    User::set_notify_new_location(&pool, user.id, false)
        .await
        .expect("write must succeed");

    let (status, body) = call(
        common::api_router(pool.clone(), config),
        "PATCH",
        &auth,
        Some(json!({})),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["notify_new_location"], json!(false));
    assert!(
        !stored_flag(&pool, user.id).await,
        "an empty patch must not re-enable alerts"
    );

    common::delete_user(&pool, user.id).await;
}

/// The account is the session's. A body naming another user flips the caller's
/// row and leaves the named one untouched.
#[tokio::test]
async fn patch_naming_another_account_only_reaches_the_session_row() {
    let pool = common::test_pool().await;
    let config = common::test_config();
    let alice = common::new_user(&pool).await;
    let bob = common::new_user(&pool).await;

    let (status, body) = call(
        common::api_router(pool.clone(), config.clone()),
        "PATCH",
        &common::bearer(&config, &alice),
        Some(json!({"id": bob.id.to_string(), "notify_new_location": false})),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], json!(alice.id.to_string()));
    assert!(!stored_flag(&pool, alice.id).await, "alice's row flipped");
    assert!(stored_flag(&pool, bob.id).await, "bob's row is untouched");

    common::delete_user(&pool, alice.id).await;
    common::delete_user(&pool, bob.id).await;
}

/// No session, no write: the extractor rejects before any handler runs.
#[tokio::test]
async fn patch_without_a_session_changes_nothing() {
    let pool = common::test_pool().await;
    let config = common::test_config();
    let user = common::new_user(&pool).await;

    let (status, _) = call(
        common::api_router(pool.clone(), config),
        "PATCH",
        "Bearer not-a-jwt",
        Some(json!({"notify_new_location": false})),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(stored_flag(&pool, user.id).await, "the column is untouched");

    common::delete_user(&pool, user.id).await;
}

/// The write reports whether it reached a row, which is what turns a deleted
/// account into a 401 instead of a silent success.
#[tokio::test]
async fn set_flag_reports_a_missing_row() {
    let pool = common::test_pool().await;

    let reached = User::set_notify_new_location(&pool, Uuid::new_v4(), false)
        .await
        .expect("the query must run");

    assert!(!reached, "no row matched, so the write reached nothing");
}
