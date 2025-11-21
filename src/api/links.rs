//! Link management API endpoints
//!
//! This module provides REST API endpoints for link management:
//! - POST /api/links - Create a new link
//! - GET /api/links - List all links for the authenticated user
//! - PUT /api/links/:id - Update a link
//! - DELETE /api/links/:id - Delete a link

use crate::auth::{get_session, get_session_from_cookies};
use crate::error::AppError;
use crate::models::{Category, CreateLink, Language, License, Link, LinkWithCategories, Tag, UpdateLink, User};
use crate::scraper;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{post, put},
    Json, Router,
};
use axum_extra::extract::CookieJar;
use sqlx::PgPool;
use uuid::Uuid;

/// Helper function to get authenticated user from session cookie
async fn get_authenticated_user(pool: &PgPool, jar: &CookieJar) -> Result<User, AppError> {
    let session_id = get_session_from_cookies(jar).ok_or_else(|| {
        tracing::debug!("Request without session cookie");
        AppError::SessionExpired
    })?;

    let session = get_session(pool, &session_id)
        .await?
        .ok_or_else(|| {
            tracing::debug!(session_id = %session_id, "Invalid session");
            AppError::SessionExpired
        })?;

    let user = sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, created_at FROM users WHERE id = $1",
    )
    .bind(session.user_id)
    .fetch_one(pool)
    .await?;

    Ok(user)
}

/// POST /api/links
///
/// Creates a new link for the authenticated user.
///
/// # Request Body
/// ```json
/// {
///     "url": "https://example.com/page",
///     "title": "Optional Title",
///     "description": "Optional description"
/// }
/// ```
///
/// # Response
/// - 201 Created: Returns the created link
/// - 400 Bad Request: Invalid URL format
/// - 401 Unauthorized: No valid session
async fn create_link_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Json(mut request): Json<CreateLink>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;

    tracing::info!(
        user_id = %user.id,
        url = %request.url,
        "Creating new link"
    );

    // Check if this is a GitHub repository
    let is_github = crate::github::is_github_repo(&request.url);
    let mut github_metadata = None;

    if is_github {
        tracing::info!(url = %request.url, "Detected GitHub repository URL");

        // Try to fetch GitHub metadata
        if let Some((owner, repo)) = crate::github::parse_repo_from_url(&request.url) {
            match crate::github::fetch_repo_metadata(&owner, &repo).await {
                Ok(metadata) => {
                    tracing::info!(
                        owner = %owner,
                        repo = %repo,
                        stars = metadata.stars,
                        "Successfully fetched GitHub metadata"
                    );

                    // Use GitHub description if user didn't provide one
                    if request.description.is_none() && metadata.description.is_some() {
                        request.description = metadata.description.clone();
                        tracing::debug!("Using GitHub description");
                    }

                    github_metadata = Some(metadata);
                }
                Err(e) => {
                    tracing::warn!(
                        owner = %owner,
                        repo = %repo,
                        error = %e,
                        "Failed to fetch GitHub metadata, continuing with link creation"
                    );
                }
            }
        }
    } else {
        // Not a GitHub repo - try regular web scraping
        if let Ok(metadata) = scraper::scrape_url(&request.url).await {
            // Use scraped data only if user didn't provide it
            if request.title.is_none() && metadata.title.is_some() {
                request.title = metadata.title;
                tracing::debug!("Using scraped title");
            }
            if request.description.is_none() && metadata.description.is_some() {
                request.description = metadata.description;
                tracing::debug!("Using scraped description");
            }
            if request.logo.is_none() && metadata.favicon.is_some() {
                request.logo = metadata.favicon;
                tracing::debug!("Using scraped favicon");
            }
        } else {
            tracing::warn!(url = %request.url, "Failed to scrape URL, continuing with user-provided data");
        }
    }

    // Create the link
    let link = Link::create(&pool, user.id, request).await?;

    // If we have GitHub metadata, update the link with it
    if let Some(metadata) = github_metadata {
        if let Err(e) = Link::update_github_metadata(&pool, link.id, user.id, metadata).await {
            tracing::warn!(
                link_id = %link.id,
                error = %e,
                "Failed to update GitHub metadata after link creation"
            );
        }
    }

    // Fetch the updated link to return with GitHub metadata
    let updated_link = Link::get_by_id(&pool, link.id, user.id).await?;

    Ok((StatusCode::CREATED, Json(updated_link)))
}

/// GET /api/links
///
/// Returns all links for the authenticated user with their categories.
///
/// # Response
/// - 200 OK: Returns array of links with categories
/// - 401 Unauthorized: No valid session
async fn list_links_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
) -> Result<Json<Vec<LinkWithCategories>>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;

    tracing::debug!(user_id = %user.id, "Fetching links");

    let links = Link::get_all_with_categories(&pool, user.id).await?;

    tracing::debug!(user_id = %user.id, count = links.len(), "Links fetched");

    Ok(Json(links))
}

/// PUT /api/links/:id
///
/// Updates an existing link.
///
/// # Request Body
/// ```json
/// {
///     "title": "New Title",
///     "description": "New description",
///     "status": "archived"
/// }
/// ```
///
/// # Response
/// - 200 OK: Returns the updated link
/// - 400 Bad Request: Invalid status value
/// - 401 Unauthorized: No valid session
/// - 404 Not Found: Link not found or doesn't belong to user
async fn update_link_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateLink>,
) -> Result<Json<Link>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;

    tracing::info!(
        user_id = %user.id,
        link_id = %id,
        "Updating link"
    );

    let link = Link::update(&pool, id, user.id, request).await?;

    Ok(Json(link))
}

/// DELETE /api/links/:id
///
/// Deletes a link.
///
/// # Response
/// - 204 No Content: Link deleted successfully
/// - 401 Unauthorized: No valid session
/// - 404 Not Found: Link not found or doesn't belong to user
async fn delete_link_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;

    tracing::info!(
        user_id = %user.id,
        link_id = %id,
        "Deleting link"
    );

    Link::delete(&pool, id, user.id).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, serde::Deserialize)]
struct AddCategoryRequest {
    category_id: uuid::Uuid,
}

/// POST /api/links/:id/categories
async fn add_category_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Path(id): Path<uuid::Uuid>,
    Json(request): Json<AddCategoryRequest>,
) -> Result<Json<Vec<Category>>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    Link::add_category(&pool, id, request.category_id, user.id).await?;
    let categories = Link::get_categories(&pool, id, user.id).await?;
    Ok(Json(categories))
}

/// DELETE /api/links/:id/categories/:category_id
async fn remove_category_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Path((id, category_id)): Path<(uuid::Uuid, uuid::Uuid)>,
) -> Result<Json<Vec<Category>>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    Link::remove_category(&pool, id, category_id, user.id).await?;
    let categories = Link::get_categories(&pool, id, user.id).await?;
    Ok(Json(categories))
}

/// GET /api/links/:id/categories
async fn get_categories_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<Vec<Category>>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    let categories = Link::get_categories(&pool, id, user.id).await?;
    Ok(Json(categories))
}

#[derive(Debug, serde::Deserialize)]
struct AddTagRequest {
    tag_id: uuid::Uuid,
}

/// POST /api/links/:id/tags
async fn add_tag_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Path(id): Path<uuid::Uuid>,
    Json(request): Json<AddTagRequest>,
) -> Result<Json<Vec<Tag>>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    Link::add_tag(&pool, id, request.tag_id, user.id).await?;
    let tags = Link::get_tags(&pool, id, user.id).await?;
    Ok(Json(tags))
}

/// DELETE /api/links/:id/tags/:tag_id
async fn remove_tag_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Path((id, tag_id)): Path<(uuid::Uuid, uuid::Uuid)>,
) -> Result<Json<Vec<Tag>>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    Link::remove_tag(&pool, id, tag_id, user.id).await?;
    let tags = Link::get_tags(&pool, id, user.id).await?;
    Ok(Json(tags))
}

/// GET /api/links/:id/tags
async fn get_tags_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<Vec<Tag>>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    let tags = Link::get_tags(&pool, id, user.id).await?;
    Ok(Json(tags))
}

#[derive(Debug, serde::Deserialize)]
struct AddLanguageRequest {
    language_id: uuid::Uuid,
}

/// POST /api/links/:id/languages
async fn add_language_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Path(id): Path<uuid::Uuid>,
    Json(request): Json<AddLanguageRequest>,
) -> Result<Json<Vec<Language>>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    Link::add_language(&pool, id, request.language_id, user.id).await?;
    let languages = Link::get_languages(&pool, id, user.id).await?;
    Ok(Json(languages))
}

/// DELETE /api/links/:id/languages/:language_id
async fn remove_language_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Path((id, language_id)): Path<(uuid::Uuid, uuid::Uuid)>,
) -> Result<Json<Vec<Language>>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    Link::remove_language(&pool, id, language_id, user.id).await?;
    let languages = Link::get_languages(&pool, id, user.id).await?;
    Ok(Json(languages))
}

/// GET /api/links/:id/languages
async fn get_languages_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<Vec<Language>>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    let languages = Link::get_languages(&pool, id, user.id).await?;
    Ok(Json(languages))
}

#[derive(Debug, serde::Deserialize)]
struct AddLicenseRequest {
    license_id: uuid::Uuid,
}

/// POST /api/links/:id/licenses
async fn add_license_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Path(id): Path<uuid::Uuid>,
    Json(request): Json<AddLicenseRequest>,
) -> Result<Json<Vec<License>>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    Link::add_license(&pool, id, request.license_id, user.id).await?;
    let licenses = Link::get_licenses(&pool, id, user.id).await?;
    Ok(Json(licenses))
}

/// DELETE /api/links/:id/licenses/:license_id
async fn remove_license_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Path((id, license_id)): Path<(uuid::Uuid, uuid::Uuid)>,
) -> Result<Json<Vec<License>>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    Link::remove_license(&pool, id, license_id, user.id).await?;
    let licenses = Link::get_licenses(&pool, id, user.id).await?;
    Ok(Json(licenses))
}

/// GET /api/links/:id/licenses
async fn get_licenses_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<Vec<License>>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    let licenses = Link::get_licenses(&pool, id, user.id).await?;
    Ok(Json(licenses))
}

/// POST /api/links/:id/refresh
///
/// Refresh all metadata for a link (web scraping + GitHub if applicable)
///
/// # Response
/// - 200 OK: Returns the updated link with fresh metadata
/// - 401 Unauthorized: No valid session
/// - 404 Not Found: Link not found or doesn't belong to user
async fn refresh_link_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<Link>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;

    tracing::info!(
        user_id = %user.id,
        link_id = %id,
        "Refreshing metadata for link"
    );

    // Verify link exists and belongs to user
    let link = Link::get_by_id(&pool, id, user.id).await?;

    // If it's a GitHub repository, refresh GitHub metadata
    if link.is_github_repo {
        if let Some((owner, repo)) = crate::github::parse_repo_from_url(&link.url) {
            match crate::github::fetch_repo_metadata(&owner, &repo).await {
                Ok(metadata) => {
                    tracing::info!(
                        link_id = %id,
                        owner = %owner,
                        repo = %repo,
                        stars = metadata.stars,
                        "Successfully fetched GitHub metadata"
                    );

                    if let Err(e) = Link::update_github_metadata(&pool, id, user.id, metadata).await {
                        tracing::warn!(
                            link_id = %id,
                            error = %e,
                            "Failed to update GitHub metadata during refresh"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        link_id = %id,
                        owner = %owner,
                        repo = %repo,
                        error = %e,
                        "Failed to fetch GitHub metadata during refresh"
                    );
                }
            }
        }
    } else {
        // Not a GitHub repo - scrape the URL for metadata
        match scraper::scrape_url(&link.url).await {
            Ok(metadata) => {
                tracing::info!(
                    link_id = %id,
                    has_title = metadata.title.is_some(),
                    has_description = metadata.description.is_some(),
                    "Successfully scraped metadata"
                );

                if let Err(e) = Link::update_scraped_metadata(&pool, id, user.id, metadata).await {
                    tracing::warn!(
                        link_id = %id,
                        error = %e,
                        "Failed to update scraped metadata during refresh"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    link_id = %id,
                    error = %e,
                    "Failed to scrape URL during refresh"
                );
            }
        }
    }

    // Mark the link as refreshed
    Link::mark_refreshed(&pool, id, user.id).await?;

    // Fetch and return updated link
    let updated_link = Link::get_by_id(&pool, id, user.id).await?;

    tracing::info!(
        link_id = %id,
        "Link metadata refreshed successfully"
    );

    Ok(Json(updated_link))
}

/// POST /api/links/:id/refresh-github
///
/// Refresh GitHub metadata for a link
///
/// # Response
/// - 200 OK: Returns the updated link with fresh GitHub metadata
/// - 400 Bad Request: Link is not a GitHub repository
/// - 401 Unauthorized: No valid session
/// - 404 Not Found: Link not found or doesn't belong to user
/// - 502 Bad Gateway: GitHub API request failed
async fn refresh_github_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<Link>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;

    tracing::info!(
        user_id = %user.id,
        link_id = %id,
        "Refreshing GitHub metadata for link"
    );

    // Verify link exists and belongs to user
    let link = Link::get_by_id(&pool, id, user.id).await?;

    // Verify it's a GitHub repository
    if !link.is_github_repo {
        return Err(AppError::validation(
            "link",
            "This link is not a GitHub repository",
        ));
    }

    // Parse owner and repo from URL
    let (owner, repo) = crate::github::parse_repo_from_url(&link.url).ok_or_else(|| {
        AppError::validation(
            "url",
            "Could not parse GitHub owner and repository from URL",
        )
    })?;

    tracing::info!(
        link_id = %id,
        owner = %owner,
        repo = %repo,
        "Fetching fresh GitHub metadata"
    );

    // Fetch latest GitHub metadata
    let metadata = crate::github::fetch_repo_metadata(&owner, &repo).await?;

    // Update the link with fresh metadata
    Link::update_github_metadata(&pool, id, user.id, metadata).await?;

    // Fetch and return updated link
    let updated_link = Link::get_by_id(&pool, id, user.id).await?;

    tracing::info!(
        link_id = %id,
        stars = updated_link.github_stars,
        "GitHub metadata refreshed successfully"
    );

    Ok(Json(updated_link))
}

/// Create the links router
pub fn create_router() -> Router<PgPool> {
    Router::new()
        .route("/", post(create_link_handler).get(list_links_handler))
        .route("/:id", put(update_link_handler).delete(delete_link_handler))
        .route("/:id/refresh", post(refresh_link_handler))
        .route("/:id/refresh-github", post(refresh_github_handler))
        .route("/:id/categories", post(add_category_handler).get(get_categories_handler))
        .route("/:id/categories/:category_id", axum::routing::delete(remove_category_handler))
        .route("/:id/tags", post(add_tag_handler).get(get_tags_handler))
        .route("/:id/tags/:tag_id", axum::routing::delete(remove_tag_handler))
        .route("/:id/languages", post(add_language_handler).get(get_languages_handler))
        .route("/:id/languages/:language_id", axum::routing::delete(remove_language_handler))
        .route("/:id/licenses", post(add_license_handler).get(get_licenses_handler))
        .route("/:id/licenses/:license_id", axum::routing::delete(remove_license_handler))
}
