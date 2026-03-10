use axum::{extract::FromRequestParts, http::request::Parts};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

/// Extract the client IP address from request parts.
/// Checks X-Forwarded-For and X-Real-Ip headers first (for reverse proxy setups),
/// then falls back to the connection info.
fn client_ip(parts: &Parts) -> String {
    if let Some(forwarded) = parts
        .headers
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
    {
        if let Some(ip) = forwarded.split(',').next().map(|s| s.trim()) {
            if !ip.is_empty() {
                return ip.to_string();
            }
        }
    }

    if let Some(real_ip) = parts
        .headers
        .get("X-Real-Ip")
        .and_then(|v| v.to_str().ok())
    {
        let trimmed = real_ip.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    if let Some(connect_info) = parts
        .extensions
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
    {
        return connect_info.0.ip().to_string();
    }

    "unknown".to_string()
}

/// JWT claims extracted from Authorization header.
///
/// Use as an Axum extractor to require authentication on a route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub user_id: String,
    pub is_admin: bool,
    pub exp: usize,
}

/// Admin-only claims extractor.
///
/// Same as Claims but also verifies `is_admin == true`.
/// Returns 403 if the user is not an admin.
#[derive(Debug, Clone)]
pub struct AdminClaims(pub Claims);

/// Authenticated user extractor that provides the user_id as Uuid.
///
/// Works in both standalone (JWT) and saas (cookie) modes.
/// Use this as an Axum extractor on protected routes.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
}

#[cfg(feature = "standalone")]
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
    crate::config::Config: axum::extract::FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        use axum::extract::FromRef;

        let config = crate::config::Config::from_ref(state);
        let ip = client_ip(parts);
        let path = parts.uri.path().to_string();

        // Extract Authorization header
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                tracing::info!(ip = %ip, path = %path, "Unauthenticated access attempt (no token)");
                AppError::SessionExpired
            })?;

        // Parse "Bearer <token>"
        let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
            tracing::info!(ip = %ip, path = %path, "Unauthenticated access attempt (malformed header)");
            AppError::SessionExpired
        })?;

        // Decode JWT
        let claims = crate::auth::jwt::decode_jwt(token, &config.jwt_secret).map_err(|_| {
            tracing::info!(ip = %ip, path = %path, "Unauthenticated access attempt (invalid or expired token)");
            AppError::SessionExpired
        })?;

        Ok(claims)
    }
}

#[cfg(feature = "standalone")]
impl<S> FromRequestParts<S> for AdminClaims
where
    S: Send + Sync,
    crate::config::Config: axum::extract::FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let claims = Claims::from_request_parts(parts, state).await?;

        if !claims.is_admin {
            return Err(AppError::Forbidden("Admin access required".to_string()));
        }

        Ok(AdminClaims(claims))
    }
}

#[cfg(feature = "standalone")]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
    crate::config::Config: axum::extract::FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let claims = Claims::from_request_parts(parts, state).await?;
        let user_id: Uuid = claims
            .user_id
            .parse()
            .map_err(|_| AppError::SessionExpired)?;
        Ok(AuthenticatedUser { user_id })
    }
}

#[cfg(all(feature = "saas", not(feature = "standalone")))]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
    crate::config::Config: axum::extract::FromRef<S>,
    sqlx::PgPool: axum::extract::FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        use axum::extract::FromRef;
        use axum_extra::extract::CookieJar;

        let config = crate::config::Config::from_ref(state);
        let pool = sqlx::PgPool::from_ref(state);
        let ip = client_ip(parts);
        let path = parts.uri.path().to_string();

        let jar = CookieJar::from_headers(&parts.headers);
        let claims =
            crate::auth::saas_auth::get_user_from_cookie(&jar, &config.saas_jwt_secret)
                .ok_or_else(|| {
                    tracing::info!(ip = %ip, path = %path, "Unauthenticated access attempt (no valid cookie)");
                    AppError::SessionExpired
                })?;
        let user_id: Uuid = claims.user_id.parse().map_err(|_| {
            tracing::info!(ip = %ip, path = %path, "Unauthenticated access attempt (invalid user ID in cookie)");
            AppError::SessionExpired
        })?;

        // Ensure the SaaS user exists in the local database.
        // The parent app manages authentication; we just need a local user row
        // so that foreign key constraints on links, tags, etc. are satisfied.
        let email = claims
            .email
            .filter(|e| !e.is_empty())
            .unwrap_or_else(|| format!("{}@saas.local", user_id));
        sqlx::query(
            "INSERT INTO users (id, email, password_hash, name) VALUES ($1, $2, '', '') ON CONFLICT (id) DO NOTHING",
        )
        .bind(user_id)
        .bind(&email)
        .execute(&pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, user_id = %user_id, "Failed to provision SaaS user");
            AppError::Database(e)
        })?;

        Ok(AuthenticatedUser { user_id })
    }
}
