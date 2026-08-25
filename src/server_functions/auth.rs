use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    /// The browser's device id (LINKS-45), recorded as this account's first
    /// known device. See [`LoginRequest::device_id`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
}

/// Request to login
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    /// A random id the browser mints once and keeps in `localStorage`
    /// (LINKS-45), used by the approval gate to tell a device this account has
    /// signed in from before from one it has not. Only its SHA-256 is stored.
    ///
    /// Absent means "not submitted", never "new": a client that leaves it out
    /// gets exactly today's country-only behaviour, so this is hardening for
    /// the app's own sign-in form rather than a control an API client must
    /// satisfy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
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

/// One device an account is recognised from (LINKS-55), as a client sees it.
///
/// The type deliberately has NO field for `known_devices.device_id_hash`. The
/// hash is the credential-shaped half of the device identity and a client has
/// no use for it, so rather than hiding it at the serializer, where it would
/// sit one forgotten attribute away from the wire, there is simply nowhere to
/// put it. `is_current` is resolved server-side by comparing hashes in SQL, so
/// no hash is ever materialised outside the query that reads it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KnownDeviceInfo {
    /// Opaque row id, the handle `DELETE /api/auth/devices/{id}` takes.
    pub id: Uuid,
    /// When this account first completed a sign-in from the device.
    pub first_seen_at: chrono::DateTime<chrono::Utc>,
    /// Touched by every later sign-in from it.
    pub last_seen_at: chrono::DateTime<chrono::Utc>,
    /// Whether the request listing them came from this device, so the UI can
    /// say which row is "this browser" and what removing it costs.
    pub is_current: bool,
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

    // An older client that sends no device id still parses, and reads as "not
    // submitted" rather than as a new device, which is what keeps it on
    // country-only behaviour instead of held (LINKS-45).
    #[test]
    fn login_request_without_a_device_id_parses_as_absent() {
        let request: LoginRequest =
            serde_json::from_str(r#"{"email": "user@example.com", "password": "hunter2hunter2"}"#)
                .unwrap();
        assert_eq!(request.device_id, None);

        let setup: SetupRequest = serde_json::from_str(
            r#"{"email": "user@example.com", "password": "hunter2hunter2", "name": "User"}"#,
        )
        .unwrap();
        assert_eq!(setup.device_id, None);
    }

    // A submitted id survives parsing, and an absent one is not serialized, so
    // the wire shape is unchanged for a client that has none yet.
    #[test]
    fn login_request_carries_a_submitted_device_id() {
        let request: LoginRequest = serde_json::from_str(
            r#"{"email": "user@example.com", "password": "hunter2hunter2", "device_id": "abc123"}"#,
        )
        .unwrap();
        assert_eq!(request.device_id.as_deref(), Some("abc123"));

        let bare = LoginRequest {
            email: "user@example.com".to_string(),
            password: "hunter2hunter2".to_string(),
            device_id: None,
        };
        assert_eq!(
            serde_json::to_value(&bare).unwrap(),
            serde_json::json!({"email": "user@example.com", "password": "hunter2hunter2"}),
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
