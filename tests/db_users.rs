//! User round trips through Postgres (LINKS-44).
//!
//! Replaces the `#[ignore]`d `tests/integration_example.rs`, whose three cases
//! never ran: nothing executed the `tests/` targets and CI had no database.

#![cfg(feature = "server")]

mod common;

use rusty_links::models::{create_user, find_user_by_email, CreateUser, User};

/// The insert stores a hash, never the password, and the stored hash is what
/// the login route verifies against.
#[tokio::test]
async fn create_user_stores_a_verifiable_hash() {
    let pool = common::test_pool().await;
    let email = common::unique_email();

    let created = create_user(
        &pool,
        CreateUser {
            email: email.clone(),
            password: common::TEST_PASSWORD.to_string(),
        },
    )
    .await
    .expect("user must be created");

    let stored = find_user_by_email(&pool, &email)
        .await
        .expect("lookup must succeed")
        .expect("the row must be readable back");

    assert_eq!(stored.id, created.id);
    assert_eq!(stored.email, email);
    assert_ne!(stored.password_hash, common::TEST_PASSWORD);
    assert!(stored.verify_password(common::TEST_PASSWORD));
    assert!(!stored.verify_password("wrong password"));

    common::delete_user(&pool, created.id).await;
}

/// `users.email` is UNIQUE, so the second insert is rejected by the database
/// rather than silently overwriting the first account.
#[tokio::test]
async fn duplicate_email_is_rejected() {
    let pool = common::test_pool().await;
    let email = common::unique_email();

    let first = create_user(
        &pool,
        CreateUser {
            email: email.clone(),
            password: common::TEST_PASSWORD.to_string(),
        },
    )
    .await
    .expect("first user must be created");

    let second = create_user(
        &pool,
        CreateUser {
            email: email.clone(),
            password: "AnotherPassword123!".to_string(),
        },
    )
    .await;

    assert!(
        second.is_err(),
        "a second account cannot take a taken email"
    );

    common::delete_user(&pool, first.id).await;
}

/// Two accounts are two rows, addressable independently.
#[tokio::test]
async fn users_get_distinct_ids() {
    let pool = common::test_pool().await;

    let alice = common::new_user(&pool).await;
    let bob = common::new_user(&pool).await;

    assert_ne!(alice.id, bob.id);
    assert_ne!(alice.email, bob.email);

    let found = User::find_by_id(&pool, alice.id)
        .await
        .expect("lookup must succeed")
        .expect("alice must be readable by id");
    assert_eq!(found.email, alice.email);

    common::delete_user(&pool, alice.id).await;
    common::delete_user(&pool, bob.id).await;
}
