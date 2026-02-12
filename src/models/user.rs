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
//! Passwords are hashed using bcrypt with a cost factor of 12.

use crate::error::AppError;
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
    /// Password hash (never send to frontend)
    #[serde(skip_serializing)]
    pub password_hash: String,
    /// User's display name
    pub name: String,
    /// Whether the user has admin privileges
    pub is_admin: bool,
    /// Timestamp when user was created
    pub created_at: DateTime<Utc>,
}

impl User {
    /// Create a new user (associated function wrapper)
    pub async fn create(
        pool: &PgPool,
        email: &str,
        password: &str,
        name: &str,
    ) -> Result<Self, AppError> {
        // Validate email format
        validate_email(email)?;

        tracing::info!(email = %email, "Creating new user");

        // Hash password
        let password_hash = hash_password(password)?;

        // Check if this is the first user (make them admin)
        let is_first_user = !check_user_exists(pool).await?;

        // Insert user into database
        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (email, password_hash, name, is_admin)
            VALUES ($1, $2, $3, $4)
            RETURNING id, email, password_hash, name, is_admin, created_at
            "#,
        )
        .bind(email)
        .bind(&password_hash)
        .bind(name)
        .bind(is_first_user)
        .fetch_one(pool)
        .await?;

        tracing::info!(
            user_id = %user.id,
            email = %user.email,
            is_admin = is_first_user,
            "User created successfully"
        );

        Ok(user)
    }

    /// Find a user by email (associated function wrapper)
    pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<Self>, AppError> {
        find_user_by_email(pool, email).await
    }

    /// Find a user by ID
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Self>, AppError> {
        tracing::debug!(user_id = %id, "Looking up user by ID");

        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, email, password_hash, name, is_admin, created_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        if user.is_some() {
            tracing::debug!(user_id = %id, "User found");
        } else {
            tracing::debug!(user_id = %id, "User not found");
        }

        Ok(user)
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
pub async fn create_user(pool: &PgPool, create_user: CreateUser) -> Result<User, AppError> {
    // Validate email format
    validate_email(&create_user.email)?;

    tracing::info!(email = %create_user.email, "Creating new user");

    // Hash password
    let password_hash = hash_password(&create_user.password)?;

    // Check if this is the first user (make them admin)
    let is_first_user = !check_user_exists(pool).await?;

    // Insert user into database
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (email, password_hash, name, is_admin)
        VALUES ($1, $2, $3, $4)
        RETURNING id, email, password_hash, name, is_admin, created_at
        "#,
    )
    .bind(&create_user.email)
    .bind(&password_hash)
    .bind("") // Default empty name for legacy function
    .bind(is_first_user)
    .fetch_one(pool)
    .await?;

    tracing::info!(
        user_id = %user.id,
        email = %user.email,
        is_admin = is_first_user,
        "User created successfully"
    );

    Ok(user)
}

/// Find a user by email address
pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, AppError> {
    tracing::debug!(email = %email, "Looking up user by email");

    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT id, email, password_hash, name, is_admin, created_at
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

/// Verify a password against a bcrypt hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    tracing::debug!("Verifying password");

    #[cfg(feature = "standalone")]
    {
        let result = bcrypt::verify(password, hash).map_err(|e| {
            tracing::error!(error = %e, "Failed to verify password");
            AppError::Internal(format!("Failed to verify password: {}", e))
        })?;

        if result {
            tracing::debug!("Password verification successful");
        } else {
            tracing::debug!("Password verification failed");
        }

        Ok(result)
    }

    #[cfg(not(feature = "standalone"))]
    {
        let _ = (password, hash);
        Err(AppError::Internal(
            "Password verification not available in saas mode".to_string(),
        ))
    }
}

/// Check if any user exists in the database
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
fn validate_email(email: &str) -> Result<(), AppError> {
    if !email.contains('@') {
        return Err(AppError::validation("email", "Email must contain @ symbol"));
    }

    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return Err(AppError::validation("email", "Email format is invalid"));
    }

    let local = parts[0];
    let domain = parts[1];

    if local.is_empty() {
        return Err(AppError::validation(
            "email",
            "Email must have content before @",
        ));
    }

    if domain.is_empty() {
        return Err(AppError::validation(
            "email",
            "Email must have a domain after @",
        ));
    }

    if !domain.contains('.') {
        return Err(AppError::validation(
            "email",
            "Email domain must contain a dot",
        ));
    }

    Ok(())
}

/// Hash a password using bcrypt
fn hash_password(password: &str) -> Result<String, AppError> {
    tracing::debug!("Hashing password");

    #[cfg(feature = "standalone")]
    {
        let hash = bcrypt::hash(password, 12).map_err(|e| {
            tracing::error!(error = %e, "Failed to hash password");
            AppError::Internal(format!("Failed to hash password: {}", e))
        })?;

        tracing::debug!("Password hashed successfully");
        Ok(hash)
    }

    #[cfg(not(feature = "standalone"))]
    {
        let _ = password;
        Err(AppError::Internal(
            "Password hashing not available in saas mode".to_string(),
        ))
    }
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
        assert!(validate_email("userexample.com").is_err());
        assert!(validate_email("@example.com").is_err());
        assert!(validate_email("user@").is_err());
        assert!(validate_email("user@example").is_err());
        assert!(validate_email("user@@example.com").is_err());
    }

    #[cfg(feature = "standalone")]
    #[test]
    fn test_password_hashing_and_verification() {
        let password = "my_secure_password_123";

        let hash = hash_password(password).expect("Hashing should succeed");

        assert!(
            verify_password(password, &hash).expect("Verification should succeed"),
            "Correct password should verify"
        );

        assert!(
            !verify_password("wrong_password", &hash).expect("Verification should succeed"),
            "Incorrect password should not verify"
        );
    }

    #[cfg(feature = "standalone")]
    #[test]
    fn test_password_hash_uniqueness() {
        let password = "same_password";

        let hash1 = hash_password(password).expect("Hashing should succeed");
        let hash2 = hash_password(password).expect("Hashing should succeed");

        assert_ne!(hash1, hash2, "Hashes should be different due to salt");

        assert!(verify_password(password, &hash1).expect("Verification should succeed"));
        assert!(verify_password(password, &hash2).expect("Verification should succeed"));
    }
}
