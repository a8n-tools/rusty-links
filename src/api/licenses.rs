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
struct LicenseResponse {
    id: Uuid,
    name: String,            // full_name from DB
    acronym: Option<String>, // name (acronym) from DB
    link_count: i64,
}

/// GET /api/licenses
async fn list_licenses(
    State(pool): State<PgPool>,
    jar: CookieJar,
) -> Result<Json<Vec<LicenseResponse>>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;

    // Fetch licenses with link counts
    let licenses = sqlx::query_as::<_, (Uuid, String, String, i64)>(
        r#"
        SELECT l.id, l.full_name, l.name as acronym, COUNT(ll.link_id) as link_count
        FROM licenses l
        LEFT JOIN link_licenses ll ON l.id = ll.license_id
        LEFT JOIN links lnk ON ll.link_id = lnk.id AND lnk.user_id = $1
        WHERE l.user_id IS NULL OR l.user_id = $1
        GROUP BY l.id, l.full_name, l.name
        ORDER BY l.full_name
        "#,
    )
    .bind(user.id)
    .fetch_all(&pool)
    .await?;

    let response: Vec<LicenseResponse> = licenses
        .into_iter()
        .map(|(id, name, acronym, link_count)| LicenseResponse {
            id,
            name,
            acronym: Some(acronym),
            link_count,
        })
        .collect();

    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
struct CreateLicenseRequest {
    name: String,
    full_name: Option<String>,
}

/// POST /api/licenses
async fn create_license(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Json(request): Json<CreateLicenseRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    let full_name = request.full_name.as_deref().unwrap_or(&request.name);
    let license = License::create(&pool, user.id, &request.name, full_name).await?;
    let response = LicenseResponse {
        id: license.id,
        name: license.full_name,
        acronym: Some(license.name),
        link_count: 0, // New license has no links yet
    };
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
    let lic = sqlx::query_as::<_, License>("SELECT * FROM licenses WHERE id = $1")
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
