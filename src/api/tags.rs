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
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Helper to get authenticated user
async fn get_authenticated_user(pool: &PgPool, jar: &CookieJar) -> Result<User, AppError> {
    let session_id = get_session_from_cookies(jar).ok_or(AppError::SessionExpired)?;
    let session = get_session(pool, &session_id)
        .await?
        .ok_or(AppError::SessionExpired)?;

    sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, name, created_at FROM users WHERE id = $1",
    )
    .bind(session.user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

#[derive(Debug, Serialize)]
struct TagResponse {
    id: Uuid,
    name: String,
    link_count: i64,
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
    let response = TagResponse {
        id: tag.id,
        name: tag.name,
        link_count: 0, // New tag has no links yet
    };
    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /api/tags
async fn list_tags(
    State(pool): State<PgPool>,
    jar: CookieJar,
) -> Result<Json<Vec<TagResponse>>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;

    // Fetch tags with link counts
    let tags = sqlx::query_as::<_, (Uuid, String, i64)>(
        r#"
        SELECT t.id, t.name, COUNT(lt.link_id) as link_count
        FROM tags t
        LEFT JOIN link_tags lt ON t.id = lt.tag_id
        LEFT JOIN links l ON lt.link_id = l.id AND l.user_id = $1
        WHERE t.user_id = $1
        GROUP BY t.id, t.name
        ORDER BY t.name
        "#,
    )
    .bind(user.id)
    .fetch_all(&pool)
    .await?;

    let response: Vec<TagResponse> = tags
        .into_iter()
        .map(|(id, name, link_count)| TagResponse {
            id,
            name,
            link_count,
        })
        .collect();

    Ok(Json(response))
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
