//! Example integration tests
//!
//! This file demonstrates how to write integration tests using the common test utilities.
//! These tests are disabled by default (using #[ignore]) to avoid requiring a test database
//! in CI until the test suite is fully set up.

mod common;

use rusty_links::models::{create_user, User};

/// Example: Test creating a user
///
/// This test demonstrates:
/// - Setting up a test database
/// - Creating a user
/// - Cleaning up after the test
#[tokio::test]
#[ignore] // Remove this when ready to run integration tests
async fn test_create_user() {
    let pool = common::setup_test_db().await;

    // Create a test user
    let user_data = common::create_test_user_data();
    let result = create_user(&pool, user_data).await;

    assert!(result.is_ok());
    let user = result.unwrap();
    assert_eq!(user.email, "test@example.com");
    assert!(!user.password_hash.is_empty());

    // Clean up
    common::cleanup_test_db(&pool).await;
}

/// Example: Test creating multiple users with unique emails
#[tokio::test]
#[ignore] // Remove this when ready to run integration tests
async fn test_create_multiple_users() {
    let pool = common::setup_test_db().await;

    // Create multiple users with unique emails
    let email1 = common::generate_test_email();
    let email2 = common::generate_test_email();

    let user1_data = common::create_test_user_with_credentials(&email1, "Password1");
    let user2_data = common::create_test_user_with_credentials(&email2, "Password2");

    let user1 = create_user(&pool, user1_data).await.unwrap();
    let user2 = create_user(&pool, user2_data).await.unwrap();

    assert_ne!(user1.id, user2.id);
    assert_ne!(user1.email, user2.email);

    // Clean up
    common::cleanup_test_db(&pool).await;
}

/// Example: Test that duplicate email fails
#[tokio::test]
#[ignore] // Remove this when ready to run integration tests
async fn test_duplicate_email_fails() {
    let pool = common::setup_test_db().await;

    let user_data1 = common::create_test_user_data();
    let user_data2 = common::create_test_user_data(); // Same email

    // First user should succeed
    let result1 = create_user(&pool, user_data1).await;
    assert!(result1.is_ok());

    // Second user with same email should fail
    let result2 = create_user(&pool, user_data2).await;
    assert!(result2.is_err());

    // Clean up
    common::cleanup_test_db(&pool).await;
}

// TODO: Add more integration tests:
// - Test authentication flow (setup, login, logout)
// - Test link creation with metadata extraction
// - Test category hierarchy
// - Test tags, languages, licenses
// - Test search and filtering
// - Test bulk operations
// - Test import/export
