//! Link management API endpoints
//!
//! This module provides REST API endpoints for link management:
//! - POST /api/links - Create a new link
//! - GET /api/links - List all links for the authenticated user
//! - PUT /api/links/:id - Update a link
//! - DELETE /api/links/:id - Delete a link

use crate::auth::{get_session, get_session_from_cookies};
use crate::error::AppError;
use crate::models::{CreateLink, Link, UpdateLink, User};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{post, put},
    Json, Router,
};
use axum_extra::extract::CookieJar;
use sqlx::PgPool;
use uuid::Uuid;

/// Helper function to get authenticated user from session cookie
async fn get_authenticated_user(pool: &PgPool, jar: &CookieJar) -> Result<User, AppError> {
    let session_id = get_session_from_cookies(jar).ok_or_else(|| {
        tracing::debug!("Request without session cookie");
        AppError::SessionExpired
    })?;

    let session = get_session(pool, &session_id)
        .await?
        .ok_or_else(|| {
            tracing::debug!(session_id = %session_id, "Invalid session");
            AppError::SessionExpired
        })?;

    let user = sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, created_at FROM users WHERE id = $1",
    )
    .bind(session.user_id)
    .fetch_one(pool)
    .await?;

    Ok(user)
}

/// POST /api/links
///
/// Creates a new link for the authenticated user.
///
/// # Request Body
/// ```json
/// {
///     "url": "https://example.com/page",
///     "title": "Optional Title",
///     "description": "Optional description"
/// }
/// ```
///
/// # Response
/// - 201 Created: Returns the created link
/// - 400 Bad Request: Invalid URL format
/// - 401 Unauthorized: No valid session
async fn create_link_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Json(request): Json<CreateLink>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;

    tracing::info!(
        user_id = %user.id,
        url = %request.url,
        "Creating new link"
    );

    let link = Link::create(&pool, user.id, request).await?;

    Ok((StatusCode::CREATED, Json(link)))
}

/// GET /api/links
///
/// Returns all links for the authenticated user.
///
/// # Response
/// - 200 OK: Returns array of links
/// - 401 Unauthorized: No valid session
async fn list_links_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
) -> Result<Json<Vec<Link>>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;

    tracing::debug!(user_id = %user.id, "Fetching links");

    let links = Link::get_all_by_user(&pool, user.id).await?;

    tracing::debug!(user_id = %user.id, count = links.len(), "Links fetched");

    Ok(Json(links))
}

/// PUT /api/links/:id
///
/// Updates an existing link.
///
/// # Request Body
/// ```json
/// {
///     "title": "New Title",
///     "description": "New description",
///     "status": "archived"
/// }
/// ```
///
/// # Response
/// - 200 OK: Returns the updated link
/// - 400 Bad Request: Invalid status value
/// - 401 Unauthorized: No valid session
/// - 404 Not Found: Link not found or doesn't belong to user
async fn update_link_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateLink>,
) -> Result<Json<Link>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;

    tracing::info!(
        user_id = %user.id,
        link_id = %id,
        "Updating link"
    );

    let link = Link::update(&pool, id, user.id, request).await?;

    Ok(Json(link))
}

/// DELETE /api/links/:id
///
/// Deletes a link.
///
/// # Response
/// - 204 No Content: Link deleted successfully
/// - 401 Unauthorized: No valid session
/// - 404 Not Found: Link not found or doesn't belong to user
async fn delete_link_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;

    tracing::info!(
        user_id = %user.id,
        link_id = %id,
        "Deleting link"
    );

    Link::delete(&pool, id, user.id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Create the links router
pub fn create_router() -> Router<PgPool> {
    Router::new()
        .route("/", post(create_link_handler).get(list_links_handler))
        .route("/:id", put(update_link_handler).delete(delete_link_handler))
}
