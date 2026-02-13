use axum::{extract::FromRequestParts, http::request::Parts};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

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

        // Extract Authorization header
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::SessionExpired)?;

        // Parse "Bearer <token>"
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AppError::SessionExpired)?;

        // Decode JWT
        let claims = crate::auth::jwt::decode_jwt(token, &config.jwt_secret)
            .map_err(|_| AppError::SessionExpired)?;

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
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        use axum_extra::extract::CookieJar;

        let jar = CookieJar::from_headers(&parts.headers);
        let claims =
            crate::auth::saas_auth::get_user_from_cookie(&jar).ok_or(AppError::SessionExpired)?;
        let user_id: Uuid = claims
            .user_id
            .parse()
            .map_err(|_| AppError::SessionExpired)?;
        Ok(AuthenticatedUser { user_id })
    }
}
