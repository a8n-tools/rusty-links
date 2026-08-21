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
    #[serde(default)]
    pub maintenance_mode: bool,
    /// True when the current session was created via OIDC (SSO login).
    /// False for password-auth sessions and API bearer-token access.
    #[serde(default)]
    pub auth_via_oidc: bool,
    /// Whether a sign-in from a country this account has not used before
    /// raises a security alert (LINKS-27). Changed via `PATCH /api/auth/me`.
    #[serde(default = "alerts_on")]
    pub notify_new_location: bool,
}

/// `users.notify_new_location` defaults to TRUE, so a payload without the key
/// means alerts are on rather than off.
fn alerts_on() -> bool {
    true
}

/// Account settings patch (`PATCH /api/auth/me`).
///
/// Every field is optional and independent: an absent key means "not
/// submitted", so the stored value stands and a request that changes one
/// setting can never clobber another. A non-boolean is rejected rather than
/// coerced. The type carries no account id, so the write always targets the
/// session's own account (LINKS-33).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct UpdateMeRequest {
    /// Whether new-location sign-in alerts stay on for this account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notify_new_location: Option<bool>,
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

/// Log an unauthenticated access attempt to a protected page route.
/// Called from ProtectedLayout when the client-side auth check fails.
#[server]
pub async fn log_unauthenticated_access(path: String) -> Result<(), ServerFnError> {
    let headers: axum::http::HeaderMap = dioxus_fullstack::FullstackContext::extract().await?;
    let connect_info = dioxus_fullstack::FullstackContext::extract::<
        axum::extract::ConnectInfo<std::net::SocketAddr>,
        _,
    >()
    .await
    .ok();

    // Shared reader so this path sees the same LINKS-31 peer-gated headers as
    // every other client-IP reader; the socket peer is the fallback.
    let ip = crate::auth::middleware::client_ip_from_headers(&headers)
        .or_else(|| connect_info.map(|ci| ci.0.ip().to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    tracing::info!(ip = %ip, path = %path, "Unauthenticated access attempt on protected route");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // An absent key is "not submitted", not "turn it off".
    #[test]
    fn update_me_request_absent_key_is_none() {
        let request: UpdateMeRequest = serde_json::from_str("{}").unwrap();
        assert_eq!(request.notify_new_location, None);
    }

    // An explicit value has to survive parsing in both directions, or an
    // opt-out would silently be a no-op.
    #[test]
    fn update_me_request_keeps_an_explicit_value() {
        let off: UpdateMeRequest =
            serde_json::from_str(r#"{"notify_new_location": false}"#).unwrap();
        assert_eq!(off.notify_new_location, Some(false));

        let on: UpdateMeRequest = serde_json::from_str(r#"{"notify_new_location": true}"#).unwrap();
        assert_eq!(on.notify_new_location, Some(true));
    }

    // A non-boolean is rejected (the handler answers 400) rather than coerced
    // into an accidental opt-out.
    #[test]
    fn update_me_request_rejects_a_non_boolean() {
        for bad in [
            r#"{"notify_new_location": "false"}"#,
            r#"{"notify_new_location": 0}"#,
            r#"{"notify_new_location": []}"#,
        ] {
            assert!(
                serde_json::from_str::<UpdateMeRequest>(bad).is_err(),
                "expected {bad} to be rejected"
            );
        }
    }

    // Nothing in the body can name an account: the patch type has no id field,
    // so an id sent alongside the flag does not survive parsing and the handler
    // has nothing but the session id to write with.
    #[test]
    fn update_me_request_drops_an_account_id_from_the_body() {
        let request: UpdateMeRequest = serde_json::from_str(
            r#"{"notify_new_location": false,
                "user_id": "8e1f4dcb-1c2b-4a6f-9f7a-9a0d3a1e5b77",
                "id": "8e1f4dcb-1c2b-4a6f-9f7a-9a0d3a1e5b77",
                "email": "someone-else@example.com"}"#,
        )
        .unwrap();

        assert_eq!(request.notify_new_location, Some(false));
        assert_eq!(
            serde_json::to_value(&request).unwrap(),
            serde_json::json!({"notify_new_location": false}),
            "the parsed patch carries the flag and nothing else"
        );
    }

    // A `/me` payload without the key means alerts are on, matching the column
    // default, so an older server never reads as an opt-out.
    #[test]
    fn user_info_defaults_new_location_alerts_to_on() {
        let info: UserInfo = serde_json::from_str(
            r#"{"id": "1", "email": "user@example.com", "name": "User", "is_admin": false}"#,
        )
        .unwrap();
        assert!(info.notify_new_location);
    }
}
