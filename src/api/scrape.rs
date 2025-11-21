//! Web scraping API endpoint

use crate::auth::{get_session, get_session_from_cookies};
use crate::error::AppError;
use crate::models::User;
use crate::scraper;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

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
struct ScrapeRequest {
    url: String,
}

#[derive(Debug, Serialize)]
struct ScrapeResponse {
    title: Option<String>,
    description: Option<String>,
    favicon: Option<String>,
}

/// POST /api/scrape
///
/// Scrape metadata from a URL
///
/// # Request Body
/// ```json
/// {
///     "url": "https://example.com"
/// }
/// ```
///
/// # Response
/// - 200 OK: Returns scraped metadata
/// - 400 Bad Request: Invalid URL
/// - 401 Unauthorized: No valid session
///
/// # Note
/// Future enhancement: Add rate limiting to prevent abuse
/// Consider limiting to 10 scrapes per minute per user
async fn scrape_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Json(request): Json<ScrapeRequest>,
) -> Result<impl IntoResponse, AppError> {
    let _user = get_authenticated_user(&pool, &jar).await?;

    tracing::info!(url = %request.url, "Scraping URL for metadata");

    // Scrape the URL
    let metadata = scraper::scrape_url(&request.url).await?;

    tracing::info!(
        url = %request.url,
        has_title = metadata.title.is_some(),
        has_description = metadata.description.is_some(),
        has_favicon = metadata.favicon.is_some(),
        "URL scraped successfully"
    );

    let response = ScrapeResponse {
        title: metadata.title,
        description: metadata.description,
        favicon: metadata.favicon,
    };

    Ok((StatusCode::OK, Json(response)))
}

pub fn create_router() -> Router<PgPool> {
    Router::new().route("/", post(scrape_handler))
}
