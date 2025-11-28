//! User model and database operations
//!
//! This module handles user management including:
//! - User creation with secure password hashing
//! - Email validation
//! - Password verification
//! - User lookup operations
//!
//! # Security
//!
//! Passwords are hashed using Argon2id with recommended parameters:
//! - Memory cost: 19456 KiB (19 MiB)
//! - Time cost: 2 iterations
//! - Parallelism: 1 thread
//! - Output length: 32 bytes
//!
//! These parameters are based on OWASP recommendations for password storage.

use crate::error::AppError;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// User entity
///
/// Represents a user in the system with their authentication credentials
/// and metadata.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    /// Unique user identifier
    pub id: Uuid,
    /// User's email address (unique)
    pub email: String,
    /// Argon2 password hash (never send to frontend)
    #[serde(skip_serializing)]
    pub password_hash: String,
    /// User's display name
    pub name: String,
    /// Timestamp when user was created
    pub created_at: DateTime<Utc>,
}

impl User {
    /// Create a new user (associated function wrapper)
    pub async fn create(pool: &PgPool, email: &str, password: &str, name: &str) -> Result<Self, AppError> {
        // Validate email format
        validate_email(email)?;

        tracing::info!(email = %email, "Creating new user");

        // Hash password using Argon2
        let password_hash = hash_password(password)?;

        // Insert user into database
        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (email, password_hash, name)
            VALUES ($1, $2, $3)
            RETURNING id, email, password_hash, name, created_at
            "#,
        )
        .bind(email)
        .bind(&password_hash)
        .bind(name)
        .fetch_one(pool)
        .await?;

        tracing::info!(
            user_id = %user.id,
            email = %user.email,
            "User created successfully"
        );

        Ok(user)
    }

    /// Find a user by email (associated function wrapper)
    pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<Self>, AppError> {
        find_user_by_email(pool, email).await
    }

    /// Verify password against this user's hash
    pub fn verify_password(&self, password: &str) -> bool {
        verify_password(password, &self.password_hash).unwrap_or(false)
    }
}

/// Data for creating a new user
///
/// Contains the email and plain-text password. The password will be
/// hashed before storing in the database.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUser {
    /// Email address for the new user
    pub email: String,
    /// Plain-text password (will be hashed)
    pub password: String,
}

/// Create a new user with secure password hashing
///
/// This function:
/// 1. Validates the email format
/// 2. Hashes the password using Argon2id
/// 3. Inserts the user into the database
/// 4. Returns the created user
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `create_user` - User data (email and plain-text password)
///
/// # Returns
///
/// Returns the created `User` on success, or an `AppError` on failure.
///
/// # Errors
///
/// - `AppError::Validation` - Email format is invalid
/// - `AppError::Duplicate` - Email already exists in database
/// - `AppError::Internal` - Password hashing failed
/// - `AppError::Database` - Database operation failed
///
/// # Security
///
/// - Password is never logged or stored in plain text
/// - Uses Argon2id with OWASP-recommended parameters
/// - Email is logged for audit purposes
///
/// # Example
///
/// ```rust
/// let create_user = CreateUser {
///     email: "user@example.com".to_string(),
///     password: "secure_password".to_string(),
/// };
///
/// let user = create_user(&pool, create_user).await?;
/// ```
pub async fn create_user(pool: &PgPool, create_user: CreateUser) -> Result<User, AppError> {
    // Validate email format
    validate_email(&create_user.email)?;

    tracing::info!(email = %create_user.email, "Creating new user");

    // Hash password using Argon2
    let password_hash = hash_password(&create_user.password)?;

    // Insert user into database
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (email, password_hash, name)
        VALUES ($1, $2, $3)
        RETURNING id, email, password_hash, name, created_at
        "#,
    )
    .bind(&create_user.email)
    .bind(&password_hash)
    .bind("")  // Default empty name for legacy function
    .fetch_one(pool)
    .await?; // Automatic conversion: unique violation → AppError::Duplicate

    tracing::info!(
        user_id = %user.id,
        email = %user.email,
        "User created successfully"
    );

    Ok(user)
}

/// Find a user by email address
///
/// Performs a case-insensitive search for a user by email.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `email` - Email address to search for
///
/// # Returns
///
/// Returns `Some(User)` if found, `None` if not found, or an `AppError` on failure.
///
/// # Errors
///
/// - `AppError::Database` - Database operation failed
///
/// # Example
///
/// ```rust
/// match find_user_by_email(&pool, "user@example.com").await? {
///     Some(user) => println!("Found user: {}", user.email),
///     None => println!("User not found"),
/// }
/// ```
pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, AppError> {
    tracing::debug!(email = %email, "Looking up user by email");

    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT id, email, password_hash, name, created_at
        FROM users
        WHERE LOWER(email) = LOWER($1)
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;

    if user.is_some() {
        tracing::debug!(email = %email, "User found");
    } else {
        tracing::debug!(email = %email, "User not found");
    }

    Ok(user)
}

/// Verify a password against a hash
///
/// Uses Argon2 to verify that a plain-text password matches a hashed password.
///
/// # Arguments
///
/// * `password` - Plain-text password to verify
/// * `hash` - Argon2 password hash to verify against
///
/// # Returns
///
/// Returns `true` if the password matches, `false` if it doesn't, or an `AppError` on failure.
///
/// # Errors
///
/// - `AppError::Internal` - Hash parsing or verification failed
///
/// # Security
///
/// - Uses constant-time comparison to prevent timing attacks
/// - Never logs the password or hash
///
/// # Example
///
/// ```rust
/// let is_valid = verify_password("user_password", &user.password_hash)?;
/// if is_valid {
///     println!("Password is correct");
/// } else {
///     println!("Password is incorrect");
/// }
/// ```
pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    tracing::debug!("Verifying password");

    // Parse the stored hash
    let parsed_hash = PasswordHash::new(hash).map_err(|e| {
        tracing::error!(error = %e, "Failed to parse password hash");
        AppError::Internal(format!("Failed to parse password hash: {}", e))
    })?;

    // Verify the password
    let result = Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok();

    if result {
        tracing::debug!("Password verification successful");
    } else {
        tracing::debug!("Password verification failed");
    }

    Ok(result)
}

/// Check if any user exists in the database
///
/// Used to determine if initial setup is required. If no users exist,
/// the application should show the setup page.
///
/// # Arguments
///
/// * `pool` - Database connection pool
///
/// # Returns
///
/// Returns `true` if at least one user exists, `false` if no users exist,
/// or an `AppError` on failure.
///
/// # Errors
///
/// - `AppError::Database` - Database operation failed
///
/// # Example
///
/// ```rust
/// if !check_user_exists(&pool).await? {
///     println!("No users exist - setup required");
/// } else {
///     println!("Users exist - setup complete");
/// }
/// ```
pub async fn check_user_exists(pool: &PgPool) -> Result<bool, AppError> {
    tracing::debug!("Checking if any users exist");

    let exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(SELECT 1 FROM users LIMIT 1)
        "#,
    )
    .fetch_one(pool)
    .await?;

    tracing::debug!(exists = exists, "User existence check complete");

    Ok(exists)
}

// Private helper functions

/// Validate email format
///
/// Checks that the email:
/// 1. Contains an @ symbol
/// 2. Has content before the @
/// 3. Has a domain after the @
/// 4. Domain contains a dot
///
/// This is a basic validation - RFC-compliant email validation is extremely
/// complex and not necessary for this application.
///
/// # Errors
///
/// Returns `AppError::Validation` if the email format is invalid.
fn validate_email(email: &str) -> Result<(), AppError> {
    // Check if email contains @
    if !email.contains('@') {
        return Err(AppError::validation(
            "email",
            "Email must contain @ symbol",
        ));
    }

    // Split on @ and validate parts
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return Err(AppError::validation("email", "Email format is invalid"));
    }

    let local = parts[0];
    let domain = parts[1];

    // Check local part (before @)
    if local.is_empty() {
        return Err(AppError::validation(
            "email",
            "Email must have content before @",
        ));
    }

    // Check domain part (after @)
    if domain.is_empty() {
        return Err(AppError::validation(
            "email",
            "Email must have a domain after @",
        ));
    }

    // Check domain has at least one dot
    if !domain.contains('.') {
        return Err(AppError::validation(
            "email",
            "Email domain must contain a dot",
        ));
    }

    Ok(())
}

/// Hash a password using Argon2
///
/// Uses Argon2id with OWASP-recommended parameters for password hashing.
///
/// # Security Parameters
///
/// - Algorithm: Argon2id (hybrid of Argon2i and Argon2d)
/// - Memory cost: 19456 KiB (default)
/// - Time cost: 2 iterations (default)
/// - Parallelism: 1 thread (default)
/// - Salt: 128-bit random salt (generated using OsRng)
///
/// # Errors
///
/// Returns `AppError::Internal` if password hashing fails.
fn hash_password(password: &str) -> Result<String, AppError> {
    tracing::debug!("Hashing password");

    // Generate a random salt
    let salt = SaltString::generate(&mut OsRng);

    // Hash the password with Argon2
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to hash password");
            AppError::Internal(format!("Failed to hash password: {}", e))
        })?
        .to_string();

    tracing::debug!("Password hashed successfully");

    Ok(password_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_email_valid() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("test.user@example.co.uk").is_ok());
        assert!(validate_email("user+tag@example.com").is_ok());
    }

    #[test]
    fn test_validate_email_invalid() {
        // Missing @
        assert!(validate_email("userexample.com").is_err());

        // Empty local part
        assert!(validate_email("@example.com").is_err());

        // Empty domain
        assert!(validate_email("user@").is_err());

        // Domain without dot
        assert!(validate_email("user@example").is_err());

        // Multiple @ symbols
        assert!(validate_email("user@@example.com").is_err());
    }

    #[test]
    fn test_password_hashing_and_verification() {
        let password = "my_secure_password_123";

        // Hash the password
        let hash = hash_password(password).expect("Hashing should succeed");

        // Verify correct password
        assert!(
            verify_password(password, &hash).expect("Verification should succeed"),
            "Correct password should verify"
        );

        // Verify incorrect password
        assert!(
            !verify_password("wrong_password", &hash).expect("Verification should succeed"),
            "Incorrect password should not verify"
        );
    }

    #[test]
    fn test_password_hash_uniqueness() {
        let password = "same_password";

        // Hash the same password twice
        let hash1 = hash_password(password).expect("Hashing should succeed");
        let hash2 = hash_password(password).expect("Hashing should succeed");

        // Hashes should be different (due to random salt)
        assert_ne!(hash1, hash2, "Hashes should be different due to salt");

        // Both hashes should verify the same password
        assert!(verify_password(password, &hash1).expect("Verification should succeed"));
        assert!(verify_password(password, &hash2).expect("Verification should succeed"));
    }
}
