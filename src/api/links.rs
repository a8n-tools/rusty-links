//! Link management API endpoints
//!
//! This module provides REST API endpoints for link management:
//! - POST /api/links - Create a new link
//! - GET /api/links - List all links for the authenticated user
//! - PUT /api/links/:id - Update a link
//! - DELETE /api/links/:id - Delete a link

use crate::auth::{get_session, get_session_from_cookies};
use crate::error::AppError;
use crate::models::{Category, CreateLink, Language, License, Link, LinkSearchParams, LinkWithCategories, Tag, UpdateLink, User};
use crate::scraper;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{post, put},
    Json, Router,
};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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

/// Response structure for paginated links with metadata
#[derive(Debug, serde::Serialize)]
struct PaginatedResponse {
    links: Vec<LinkWithCategories>,
    total: i64,
    page: u32,
    per_page: u32,
    total_pages: u32,
}

/// GET /api/links
///
/// Returns paginated links for the authenticated user with their categories.
/// Supports optional query parameters for filtering, searching, sorting, and pagination.
///
/// # Query Parameters
/// - `query`: Optional text search across title, description, url, domain
/// - `status`: Optional filter by status (active, archived, inaccessible, repo_unavailable)
/// - `is_github`: Optional filter for GitHub repositories only (true/false)
/// - `category_id`: Optional filter by category UUID
/// - `tag_id`: Optional filter by tag UUID
/// - `language_id`: Optional filter by programming language UUID
/// - `license_id`: Optional filter by software license UUID
/// - `sort_by`: Optional sort field (created_at, updated_at, title, github_stars, status) - default: created_at
/// - `sort_order`: Optional sort order (asc, desc) - default: desc
/// - `page`: Optional page number (default: 1)
/// - `per_page`: Optional items per page (default: 20, max: 100)
///
/// # Examples
/// - GET /api/links - All links page 1 (sorted by created_at desc)
/// - GET /api/links?page=2 - Page 2 of links
/// - GET /api/links?per_page=50 - 50 links per page
/// - GET /api/links?query=rust - Search for "rust"
/// - GET /api/links?status=active - Only active links
/// - GET /api/links?is_github=true - Only GitHub repos
/// - GET /api/links?sort_by=title&sort_order=asc - Sort by title A-Z
/// - GET /api/links?sort_by=github_stars&sort_order=desc - Sort by stars (highest first)
/// - GET /api/links?query=rust&status=active&page=2 - Combined filters and pagination
///
/// # Response
/// - 200 OK: Returns paginated links with metadata (total, page, per_page, total_pages)
/// - 401 Unauthorized: No valid session
async fn list_links_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Query(params): Query<LinkSearchParams>,
) -> Result<Json<PaginatedResponse>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;

    tracing::debug!(
        user_id = %user.id,
        query = ?params.query,
        status = ?params.status,
        is_github = ?params.is_github,
        category_id = ?params.category_id,
        tag_id = ?params.tag_id,
        language_id = ?params.language_id,
        license_id = ?params.license_id,
        sort_by = ?params.sort_by,
        sort_order = ?params.sort_order,
        page = ?params.page,
        per_page = ?params.per_page,
        "Fetching links with search params"
    );

    // Use search_paginated function which handles all filtering and pagination
    let paginated = Link::search_paginated(&pool, user.id, &params).await?;

    tracing::debug!(
        user_id = %user.id,
        count = paginated.links.len(),
        total = paginated.total,
        page = paginated.page,
        total_pages = paginated.total_pages,
        "Links fetched"
    );

    // Enrich links with categories, tags, languages, and licenses
    let mut links_with_metadata = Vec::with_capacity(paginated.links.len());
    for link in paginated.links {
        let categories = Link::get_categories(&pool, link.id, user.id).await?;
        let tags = Link::get_tags(&pool, link.id, user.id).await?;
        let languages = Link::get_languages(&pool, link.id, user.id).await?;
        let licenses = Link::get_licenses(&pool, link.id, user.id).await?;

        links_with_metadata.push(LinkWithCategories {
            link,
            categories,
            tags,
            languages,
            licenses,
        });
    }

    Ok(Json(PaginatedResponse {
        links: links_with_metadata,
        total: paginated.total,
        page: paginated.page,
        per_page: paginated.per_page,
        total_pages: paginated.total_pages,
    }))
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

/// Bulk operations request structures
#[derive(Debug, Deserialize)]
struct BulkDeleteRequest {
    link_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
struct BulkCategoryRequest {
    link_ids: Vec<Uuid>,
    category_id: Uuid,
    action: String, // "add" or "remove"
}

#[derive(Debug, Deserialize)]
struct BulkTagRequest {
    link_ids: Vec<Uuid>,
    tag_id: Uuid,
    action: String, // "add" or "remove"
}

/// POST /api/links/bulk/delete
///
/// Delete multiple links at once
///
/// # Request Body
/// ```json
/// {
///     "link_ids": ["uuid1", "uuid2", "uuid3"]
/// }
/// ```
///
/// # Response
/// - 204 No Content: Links deleted successfully
/// - 401 Unauthorized: No valid session
/// - 404 Not Found: One or more links not found or don't belong to user
async fn bulk_delete_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Json(req): Json<BulkDeleteRequest>,
) -> Result<StatusCode, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;

    tracing::info!(
        user_id = %user.id,
        count = req.link_ids.len(),
        "Bulk deleting links"
    );

    // Verify all links belong to user and delete
    for link_id in req.link_ids {
        Link::delete(&pool, link_id, user.id).await?;
    }

    tracing::info!(
        user_id = %user.id,
        "Bulk delete completed"
    );

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/links/bulk/categories
///
/// Add or remove a category from multiple links
///
/// # Request Body
/// ```json
/// {
///     "link_ids": ["uuid1", "uuid2"],
///     "category_id": "category-uuid",
///     "action": "add"
/// }
/// ```
///
/// # Response
/// - 200 OK: Category operation completed
/// - 400 Bad Request: Invalid action (must be 'add' or 'remove')
/// - 401 Unauthorized: No valid session
async fn bulk_category_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Json(req): Json<BulkCategoryRequest>,
) -> Result<StatusCode, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;

    tracing::info!(
        user_id = %user.id,
        count = req.link_ids.len(),
        category_id = %req.category_id,
        action = %req.action,
        "Bulk category operation"
    );

    for link_id in &req.link_ids {
        match req.action.as_str() {
            "add" => Link::add_category(&pool, *link_id, req.category_id, user.id).await?,
            "remove" => Link::remove_category(&pool, *link_id, req.category_id, user.id).await?,
            _ => {
                return Err(AppError::validation(
                    "action",
                    "Must be 'add' or 'remove'",
                ))
            }
        }
    }

    tracing::info!(
        user_id = %user.id,
        "Bulk category operation completed"
    );

    Ok(StatusCode::OK)
}

/// POST /api/links/bulk/tags
///
/// Add or remove a tag from multiple links
///
/// # Request Body
/// ```json
/// {
///     "link_ids": ["uuid1", "uuid2"],
///     "tag_id": "tag-uuid",
///     "action": "add"
/// }
/// ```
///
/// # Response
/// - 200 OK: Tag operation completed
/// - 400 Bad Request: Invalid action (must be 'add' or 'remove')
/// - 401 Unauthorized: No valid session
async fn bulk_tag_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Json(req): Json<BulkTagRequest>,
) -> Result<StatusCode, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;

    tracing::info!(
        user_id = %user.id,
        count = req.link_ids.len(),
        tag_id = %req.tag_id,
        action = %req.action,
        "Bulk tag operation"
    );

    for link_id in &req.link_ids {
        match req.action.as_str() {
            "add" => Link::add_tag(&pool, *link_id, req.tag_id, user.id).await?,
            "remove" => Link::remove_tag(&pool, *link_id, req.tag_id, user.id).await?,
            _ => {
                return Err(AppError::validation(
                    "action",
                    "Must be 'add' or 'remove'",
                ))
            }
        }
    }

    tracing::info!(
        user_id = %user.id,
        "Bulk tag operation completed"
    );

    Ok(StatusCode::OK)
}

/// Export data structures
#[derive(Debug, Serialize)]
struct ExportData {
    exported_at: DateTime<Utc>,
    version: String,
    links: Vec<ExportLink>,
    categories: Vec<Category>,
    tags: Vec<Tag>,
}

#[derive(Debug, Serialize)]
struct ExportLink {
    url: String,
    title: Option<String>,
    description: Option<String>,
    status: String,
    categories: Vec<String>,  // Names, not IDs
    tags: Vec<String>,
    languages: Vec<String>,
    licenses: Vec<String>,
    created_at: DateTime<Utc>,
    is_github_repo: bool,
    github_stars: Option<i32>,
}

/// GET /api/export
///
/// Export all user data as JSON
///
/// # Response
/// - 200 OK: Returns export data with all links, categories, and tags
/// - 401 Unauthorized: No valid session
async fn export_links_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
) -> Result<Json<ExportData>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;

    tracing::info!(user_id = %user.id, "Exporting links");

    // Fetch all user data
    let links = Link::get_all_by_user(&pool, user.id).await?;
    let categories = Category::get_all_by_user(&pool, user.id).await?;
    let tags = Tag::get_all_by_user(&pool, user.id).await?;

    // Convert links to export format
    let mut export_links = Vec::new();
    for link in links {
        // Get associated metadata
        let link_categories = Link::get_categories(&pool, link.id, user.id).await?;
        let link_tags = Link::get_tags(&pool, link.id, user.id).await?;
        let link_languages = Link::get_languages(&pool, link.id, user.id).await?;
        let link_licenses = Link::get_licenses(&pool, link.id, user.id).await?;

        export_links.push(ExportLink {
            url: link.url,
            title: link.title,
            description: link.description,
            status: link.status,
            categories: link_categories.into_iter().map(|c| c.name).collect(),
            tags: link_tags.into_iter().map(|t| t.name).collect(),
            languages: link_languages.into_iter().map(|l| l.name).collect(),
            licenses: link_licenses.into_iter().map(|l| l.name).collect(),
            created_at: link.created_at,
            is_github_repo: link.is_github_repo,
            github_stars: link.github_stars,
        });
    }

    tracing::info!(
        user_id = %user.id,
        link_count = export_links.len(),
        "Export completed"
    );

    Ok(Json(ExportData {
        exported_at: Utc::now(),
        version: "1.0".to_string(),
        links: export_links,
        categories,
        tags,
    }))
}

/// Import data structures
#[derive(Debug, Deserialize)]
struct ImportData {
    links: Vec<ImportLink>,
}

#[derive(Debug, Deserialize)]
struct ImportLink {
    url: String,
    title: Option<String>,
    description: Option<String>,
    categories: Option<Vec<String>>,
    tags: Option<Vec<String>>,
    #[allow(dead_code)]
    languages: Option<Vec<String>>,
    #[allow(dead_code)]
    licenses: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct ImportResult {
    imported: u32,
    skipped: u32,
    errors: Vec<String>,
}

/// POST /api/import
///
/// Import links from JSON data
///
/// # Request Body
/// ```json
/// {
///     "links": [
///         {
///             "url": "https://example.com",
///             "title": "Example",
///             "description": "...",
///             "categories": ["Category1"],
///             "tags": ["tag1", "tag2"]
///         }
///     ]
/// }
/// ```
///
/// # Response
/// - 200 OK: Returns import results with counts and errors
/// - 401 Unauthorized: No valid session
async fn import_links_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Json(data): Json<ImportData>,
) -> Result<Json<ImportResult>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;

    tracing::info!(
        user_id = %user.id,
        link_count = data.links.len(),
        "Starting import"
    );

    let mut imported = 0;
    let mut skipped = 0;
    let mut errors = Vec::new();

    for link_data in data.links {
        // Check if URL already exists
        match Link::exists_by_url(&pool, user.id, &link_data.url).await {
            Ok(true) => {
                skipped += 1;
                continue;
            }
            Ok(false) => {}
            Err(e) => {
                errors.push(format!("{}: failed to check existence: {}", link_data.url, e));
                continue;
            }
        }

        // Create link
        let create_link = CreateLink {
            url: link_data.url.clone(),
            title: link_data.title,
            description: link_data.description,
            logo: None,
        };

        match Link::create(&pool, user.id, create_link).await {
            Ok(link) => {
                // Add categories by name
                if let Some(cats) = link_data.categories {
                    for cat_name in cats {
                        match Category::get_or_create_by_name(&pool, user.id, &cat_name).await {
                            Ok(cat) => {
                                let _ = Link::add_category(&pool, link.id, cat.id, user.id).await;
                            }
                            Err(e) => {
                                tracing::warn!(
                                    link_id = %link.id,
                                    category = %cat_name,
                                    error = %e,
                                    "Failed to add category"
                                );
                            }
                        }
                    }
                }

                // Add tags by name
                if let Some(tag_names) = link_data.tags {
                    for tag_name in tag_names {
                        match Tag::get_or_create_by_name(&pool, user.id, &tag_name).await {
                            Ok(tag) => {
                                let _ = Link::add_tag(&pool, link.id, tag.id, user.id).await;
                            }
                            Err(e) => {
                                tracing::warn!(
                                    link_id = %link.id,
                                    tag = %tag_name,
                                    error = %e,
                                    "Failed to add tag"
                                );
                            }
                        }
                    }
                }

                // Note: languages and licenses would require similar get_or_create functions
                // For now, we'll skip them in import

                imported += 1;
            }
            Err(e) => {
                errors.push(format!("{}: {}", link_data.url, e));
            }
        }
    }

    tracing::info!(
        user_id = %user.id,
        imported = imported,
        skipped = skipped,
        errors = errors.len(),
        "Import completed"
    );

    Ok(Json(ImportResult {
        imported,
        skipped,
        errors,
    }))
}

/// Create the links router
pub fn create_router() -> Router<PgPool> {
    Router::new()
        .route("/", post(create_link_handler).get(list_links_handler))
        .route("/export", axum::routing::get(export_links_handler))
        .route("/import", post(import_links_handler))
        .route("/bulk/delete", post(bulk_delete_handler))
        .route("/bulk/categories", post(bulk_category_handler))
        .route("/bulk/tags", post(bulk_tag_handler))
        .route("/{id}", put(update_link_handler).delete(delete_link_handler))
        .route("/{id}/refresh", post(refresh_link_handler))
        .route("/{id}/refresh-github", post(refresh_github_handler))
        .route("/{id}/categories", post(add_category_handler).get(get_categories_handler))
        .route("/{id}/categories/{category_id}", axum::routing::delete(remove_category_handler))
        .route("/{id}/tags", post(add_tag_handler).get(get_tags_handler))
        .route("/{id}/tags/{tag_id}", axum::routing::delete(remove_tag_handler))
        .route("/{id}/languages", post(add_language_handler).get(get_languages_handler))
        .route("/{id}/languages/{language_id}", axum::routing::delete(remove_language_handler))
        .route("/{id}/licenses", post(add_license_handler).get(get_licenses_handler))
        .route("/{id}/licenses/{license_id}", axum::routing::delete(remove_license_handler))
}
