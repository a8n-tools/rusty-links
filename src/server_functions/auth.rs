use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::{auth::session, models::user::User};
#[cfg(feature = "server")]
use sqlx::PgPool;
#[cfg(feature = "server")]
use std::sync::OnceLock;

#[cfg(feature = "server")]
pub static DB_POOL: OnceLock<PgPool> = OnceLock::new();

#[cfg(feature = "server")]
pub fn set_db_pool(pool: PgPool) {
    DB_POOL.set(pool).ok();
}

/// Request to create first user during setup
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SetupRequest {
    pub email: String,
    pub password: String,
    pub name: String,
}

/// Request to login
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// User info response
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub name: String,
}

/// Check if setup is required (no users exist)
#[server]
pub async fn check_setup() -> Result<bool, ServerFnError> {
    let pool = extract_pool()?;

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    Ok(count.0 == 0)
}

/// Create the first user during setup
#[server]
pub async fn setup(request: SetupRequest) -> Result<UserInfo, ServerFnError> {
    let pool = extract_pool()?;

    // Check if setup is already done
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    if count.0 > 0 {
        return Err(ServerFnError::new("Setup already completed"));
    }

    // Create user
    let user = User::create(pool, &request.email, &request.password, &request.name)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create user: {}", e)))?;

    // Create session
    let session_id = session::create_session(pool, user.id)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create session: {}", e)))?;

    // Set cookie (you'll need to use server context to access response headers)
    // For now, return user info and handle cookie separately

    Ok(UserInfo {
        id: user.id.to_string(),
        email: user.email,
        name: user.name,
    })
}

/// Login with email and password
#[server]
pub async fn login(request: LoginRequest) -> Result<UserInfo, ServerFnError> {
    let pool = extract_pool()?;

    // Find user by email
    let user = User::find_by_email(pool, &request.email)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Invalid credentials"))?;

    // Verify password
    if !user.verify_password(&request.password) {
        return Err(ServerFnError::new("Invalid credentials"));
    }

    // Create session
    let session_id = session::create_session(pool, user.id)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create session: {}", e)))?;

    Ok(UserInfo {
        id: user.id.to_string(),
        email: user.email,
        name: user.name,
    })
}

/// Logout (invalidate session)
#[server]
pub async fn logout() -> Result<(), ServerFnError> {
    let pool = extract_pool()?;

    // Get session ID from cookie (need to implement)
    // For now, just return success

    Ok(())
}

/// Get current user info
#[server]
pub async fn get_current_user() -> Result<Option<UserInfo>, ServerFnError> {
    let pool = extract_pool()?;

    // Get session ID from cookie (need to implement)
    // For now, return None

    Ok(None)
}

#[cfg(feature = "server")]
fn extract_pool() -> Result<&'static PgPool, ServerFnError> {
    DB_POOL
        .get()
        .ok_or_else(|| ServerFnError::new("Database pool not initialized"))
}
