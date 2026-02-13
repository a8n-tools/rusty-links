//! Web scraping API endpoint

use crate::auth::middleware::AuthenticatedUser;
use crate::error::AppError;
use crate::scraper;
use axum::{http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

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
async fn scrape_handler(
    _auth: AuthenticatedUser,
    Json(request): Json<ScrapeRequest>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(url = %request.url, "Scraping URL for metadata");

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

pub fn create_router() -> Router<super::AppState> {
    Router::new().route("/", post(scrape_handler))
}
