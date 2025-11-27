//! Tag management API endpoints

use crate::auth::{get_session, get_session_from_cookies};
use crate::error::AppError;
use crate::models::{Tag, User};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

/// Helper to get authenticated user
async fn get_authenticated_user(pool: &PgPool, jar: &CookieJar) -> Result<User, AppError> {
    let session_id = get_session_from_cookies(jar).ok_or(AppError::SessionExpired)?;
    let session = get_session(pool, &session_id).await?.ok_or(AppError::SessionExpired)?;

    sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, created_at FROM users WHERE id = $1",
    )
    .bind(session.user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

#[derive(Debug, Deserialize)]
struct CreateTagRequest {
    name: String,
}

/// POST /api/tags
async fn create_tag(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Json(request): Json<CreateTagRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    let tag = Tag::create(&pool, user.id, &request.name).await?;
    Ok((StatusCode::CREATED, Json(tag)))
}

/// GET /api/tags
async fn list_tags(
    State(pool): State<PgPool>,
    jar: CookieJar,
) -> Result<Json<Vec<Tag>>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    let tags = Tag::get_all_by_user(&pool, user.id).await?;
    Ok(Json(tags))
}

/// DELETE /api/tags/:id
async fn delete_tag(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    Tag::delete(&pool, id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Create the tags router
pub fn create_router() -> Router<PgPool> {
    Router::new()
        .route("/", post(create_tag).get(list_tags))
        .route("/{id}", axum::routing::delete(delete_tag))
}
