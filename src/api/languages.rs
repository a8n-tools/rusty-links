//! Language management API endpoints

use crate::auth::middleware::AuthenticatedUser;
use crate::error::AppError;
use crate::models::Language;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct LanguageResponse {
    id: Uuid,
    name: String,
    link_count: i64,
}

/// GET /api/languages
async fn list_languages(
    State(pool): State<PgPool>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<LanguageResponse>>, AppError> {
    let languages = sqlx::query_as::<_, (Uuid, String, i64)>(
        r#"
        SELECT l.id, l.name, COUNT(ll.link_id) as link_count
        FROM languages l
        LEFT JOIN link_languages ll ON l.id = ll.language_id
        LEFT JOIN links lnk ON ll.link_id = lnk.id AND lnk.user_id = $1
        WHERE l.user_id IS NULL OR l.user_id = $1
        GROUP BY l.id, l.name
        ORDER BY l.name
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&pool)
    .await?;

    let response: Vec<LanguageResponse> = languages
        .into_iter()
        .map(|(id, name, link_count)| LanguageResponse {
            id,
            name,
            link_count,
        })
        .collect();

    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
struct CreateLanguageRequest {
    name: String,
}

/// POST /api/languages
async fn create_language(
    State(pool): State<PgPool>,
    auth: AuthenticatedUser,
    Json(request): Json<CreateLanguageRequest>,
) -> Result<impl IntoResponse, AppError> {
    let language = Language::create(&pool, auth.user_id, &request.name).await?;
    let response = LanguageResponse {
        id: language.id,
        name: language.name,
        link_count: 0,
    };
    Ok((StatusCode::CREATED, Json(response)))
}

/// DELETE /api/languages/:id
async fn delete_language(
    State(pool): State<PgPool>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let lang = sqlx::query_as::<_, Language>("SELECT * FROM languages WHERE id = $1")
        .bind(id)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::not_found("language", &id.to_string()))?;

    if lang.user_id.is_none() {
        return Err(AppError::forbidden("Cannot delete global language"));
    }

    Language::delete(&pool, id, auth.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn create_router() -> Router<super::AppState> {
    Router::new()
        .route("/", get(list_languages).post(create_language))
        .route("/{id}", axum::routing::delete(delete_language))
}
