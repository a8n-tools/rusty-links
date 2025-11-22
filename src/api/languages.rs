//! Language management API endpoints

use crate::auth::{get_session, get_session_from_cookies};
use crate::error::AppError;
use crate::models::{Language, User};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

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

#[derive(Debug, Serialize)]
struct LanguageResponse {
    id: Uuid,
    name: String,
    is_global: bool,
}

impl From<Language> for LanguageResponse {
    fn from(lang: Language) -> Self {
        Self {
            id: lang.id,
            name: lang.name,
            is_global: lang.user_id.is_none(),
        }
    }
}

/// GET /api/languages
async fn list_languages(
    State(pool): State<PgPool>,
    jar: CookieJar,
) -> Result<Json<Vec<LanguageResponse>>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    let languages = Language::get_all_available(&pool, user.id).await?;
    let response: Vec<LanguageResponse> = languages.into_iter().map(|l| l.into()).collect();
    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
struct CreateLanguageRequest {
    name: String,
}

/// POST /api/languages
async fn create_language(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Json(request): Json<CreateLanguageRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    let language = Language::create(&pool, user.id, &request.name).await?;
    let response: LanguageResponse = language.into();
    Ok((StatusCode::CREATED, Json(response)))
}

/// DELETE /api/languages/:id
async fn delete_language(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;

    // Check if it's a global language
    let lang = sqlx::query_as::<_, Language>(
        "SELECT * FROM languages WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::not_found("language", &id.to_string()))?;

    if lang.user_id.is_none() {
        return Err(AppError::forbidden("Cannot delete global language"));
    }

    Language::delete(&pool, id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn create_router() -> Router<PgPool> {
    Router::new()
        .route("/", get(list_languages).post(create_language))
        .route("/:id", axum::routing::delete(delete_language))
}
