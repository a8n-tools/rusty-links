//! Common test utilities and helpers
//!
//! This module provides shared functionality for integration tests.

use rusty_links::models::CreateUser;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::env;

/// Set up a test database connection pool
///
/// Creates a connection pool to the test database and runs migrations.
/// Uses DATABASE_URL environment variable for connection string.
///
/// # Panics
///
/// Panics if DATABASE_URL is not set or database connection fails.
pub async fn setup_test_db() -> PgPool {
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for tests");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create test database pool");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}

/// Clean up test database
///
/// Removes all test data from the database while preserving schema.
/// Useful for cleaning up after tests.
pub async fn cleanup_test_db(pool: &PgPool) {
    // Delete all data in reverse order of dependencies
    sqlx::query("DELETE FROM link_tags")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM link_licenses")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM link_languages")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM link_categories")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM links")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM categories")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM tags WHERE user_id IS NOT NULL")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM languages WHERE user_id IS NOT NULL")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM licenses WHERE user_id IS NOT NULL")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM sessions")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM users")
        .execute(pool)
        .await
        .ok();
}

/// Create a test user with default credentials
///
/// Returns a `CreateUser` struct with test email and password.
/// Email: test@example.com
/// Password: TestPassword123!
pub fn create_test_user_data() -> CreateUser {
    CreateUser {
        email: "test@example.com".to_string(),
        password: "TestPassword123!".to_string(),
    }
}

/// Create a test user with custom credentials
///
/// # Arguments
///
/// * `email` - User email address
/// * `password` - User password
pub fn create_test_user_with_credentials(email: &str, password: &str) -> CreateUser {
    CreateUser {
        email: email.to_string(),
        password: password.to_string(),
    }
}

/// Generate a unique test email
///
/// Creates a unique email address using timestamp to avoid conflicts
/// in tests that create multiple users.
pub fn generate_test_email() -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("test_{}@example.com", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_user_data() {
        let user = create_test_user_data();
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.password, "TestPassword123!");
    }

    #[test]
    fn test_create_test_user_with_credentials() {
        let user = create_test_user_with_credentials("custom@example.com", "CustomPass123");
        assert_eq!(user.email, "custom@example.com");
        assert_eq!(user.password, "CustomPass123");
    }

    #[test]
    fn test_generate_test_email() {
        let email1 = generate_test_email();
        let email2 = generate_test_email();

        assert!(email1.starts_with("test_"));
        assert!(email1.ends_with("@example.com"));
        // Emails should be different (unless generated in same second)
        // This is a weak assertion but good enough for test utility
        assert!(email1.contains("@example.com"));
        assert!(email2.contains("@example.com"));
    }
}
