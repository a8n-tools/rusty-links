//! Tag management API endpoints

use crate::auth::middleware::AuthenticatedUser;
use crate::error::AppError;
use crate::models::Tag;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

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
    auth: AuthenticatedUser,
    Json(request): Json<CreateTagRequest>,
) -> Result<impl IntoResponse, AppError> {
    let tag = Tag::create(&pool, auth.user_id, &request.name).await?;
    let response = TagResponse {
        id: tag.id,
        name: tag.name,
        link_count: 0,
    };
    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /api/tags
async fn list_tags(
    State(pool): State<PgPool>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<TagResponse>>, AppError> {
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
    .bind(auth.user_id)
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
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    Tag::delete(&pool, id, auth.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Create the tags router
pub fn create_router() -> Router<super::AppState> {
    Router::new()
        .route("/", post(create_tag).get(list_tags))
        .route("/{id}", axum::routing::delete(delete_tag))
}
