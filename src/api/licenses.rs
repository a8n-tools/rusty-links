//! License management API endpoints

use crate::auth::{get_session, get_session_from_cookies};
use crate::error::AppError;
use crate::models::{License, User};
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
struct LicenseResponse {
    id: Uuid,
    name: String,
    is_global: bool,
}

impl From<License> for LicenseResponse {
    fn from(lic: License) -> Self {
        Self {
            id: lic.id,
            name: lic.name,
            is_global: lic.user_id.is_none(),
        }
    }
}

/// GET /api/licenses
async fn list_licenses(
    State(pool): State<PgPool>,
    jar: CookieJar,
) -> Result<Json<Vec<LicenseResponse>>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    let licenses = License::get_all_available(&pool, user.id).await?;
    let response: Vec<LicenseResponse> = licenses.into_iter().map(|l| l.into()).collect();
    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
struct CreateLicenseRequest {
    name: String,
}

/// POST /api/licenses
async fn create_license(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Json(request): Json<CreateLicenseRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    let license = License::create(&pool, user.id, &request.name).await?;
    let response: LicenseResponse = license.into();
    Ok((StatusCode::CREATED, Json(response)))
}

/// DELETE /api/licenses/:id
async fn delete_license(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;

    // Check if it's a global license
    let lic = sqlx::query_as::<_, License>(
        "SELECT * FROM licenses WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::not_found("license", &id.to_string()))?;

    if lic.user_id.is_none() {
        return Err(AppError::forbidden("Cannot delete global license"));
    }

    License::delete(&pool, id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn create_router() -> Router<PgPool> {
    Router::new()
        .route("/", get(list_licenses).post(create_license))
        .route("/{id}", axum::routing::delete(delete_license))
}
