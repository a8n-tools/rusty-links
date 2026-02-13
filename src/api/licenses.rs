//! License management API endpoints

use crate::auth::middleware::AuthenticatedUser;
use crate::error::AppError;
use crate::models::License;
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
struct LicenseResponse {
    id: Uuid,
    name: String,
    acronym: Option<String>,
    link_count: i64,
}

/// GET /api/licenses
async fn list_licenses(
    State(pool): State<PgPool>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<LicenseResponse>>, AppError> {
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
    .bind(auth.user_id)
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
    auth: AuthenticatedUser,
    Json(request): Json<CreateLicenseRequest>,
) -> Result<impl IntoResponse, AppError> {
    let full_name = request.full_name.as_deref().unwrap_or(&request.name);
    let license = License::create(&pool, auth.user_id, &request.name, full_name).await?;
    let response = LicenseResponse {
        id: license.id,
        name: license.full_name,
        acronym: Some(license.name),
        link_count: 0,
    };
    Ok((StatusCode::CREATED, Json(response)))
}

/// DELETE /api/licenses/:id
async fn delete_license(
    State(pool): State<PgPool>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let lic = sqlx::query_as::<_, License>("SELECT * FROM licenses WHERE id = $1")
        .bind(id)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::not_found("license", &id.to_string()))?;

    if lic.user_id.is_none() {
        return Err(AppError::forbidden("Cannot delete global license"));
    }

    License::delete(&pool, id, auth.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn create_router() -> Router<super::AppState> {
    Router::new()
        .route("/", get(list_licenses).post(create_license))
        .route("/{id}", axum::routing::delete(delete_license))
}
