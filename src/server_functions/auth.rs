use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

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
    pub is_admin: bool,
}

/// Authentication response with JWT tokens (standalone mode)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub refresh_token: String,
    pub email: String,
    pub is_admin: bool,
}

/// Refresh token request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
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

#[cfg(feature = "server")]
fn extract_pool() -> Result<&'static PgPool, ServerFnError> {
    DB_POOL
        .get()
        .ok_or_else(|| ServerFnError::new("Database pool not initialized"))
}
