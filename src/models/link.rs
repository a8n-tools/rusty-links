//! Link model and database operations
//!
//! This module handles bookmark link management including:
//! - Link creation with URL parsing
//! - Link retrieval (single and list)
//! - Link updates and deletion
//!
//! Links are scoped to users - each user can only access their own links.

use crate::error::AppError;
use crate::models::{Category, Language, License, Tag};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use url::Url;
use uuid::Uuid;

/// Link entity
///
/// Represents a bookmarked link with metadata.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Link {
    pub id: Uuid,
    pub user_id: Uuid,
    pub url: String,
    pub domain: String,
    pub path: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub logo: Option<String>,
    pub is_github_repo: bool,
    pub github_stars: Option<i32>,
    pub github_archived: Option<bool>,
    pub github_last_commit: Option<DateTime<Utc>>,
    pub status: String,
    pub consecutive_failures: i32,
    pub refreshed_at: Option<DateTime<Utc>>,
    pub last_checked: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Data for creating a new link
#[derive(Debug, Clone, Deserialize)]
pub struct CreateLink {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub logo: Option<String>,
}

/// Data for updating a link
#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpdateLink {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub logo: Option<String>,
}

/// Search parameters for filtering links
#[derive(Debug, Deserialize, Default)]
pub struct LinkSearchParams {
    pub query: Option<String>,        // Text search in title, description, url, domain
    pub status: Option<String>,       // Filter by status
    pub is_github: Option<bool>,      // Filter GitHub repos only
    pub category_id: Option<Uuid>,    // Filter by category
    pub tag_id: Option<Uuid>,         // Filter by tag
    pub language_id: Option<Uuid>,    // Filter by programming language
    pub license_id: Option<Uuid>,     // Filter by software license
    pub sort_by: Option<String>,      // Sort field: created_at, title, github_stars, status, updated_at
    pub sort_order: Option<String>,   // Sort order: asc, desc (default: desc)
    pub page: Option<u32>,            // Page number (1-indexed)
    pub per_page: Option<u32>,        // Items per page (default: 20, max: 100)
}

/// Paginated links response
#[derive(Debug, Serialize)]
pub struct PaginatedLinks {
    pub links: Vec<Link>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}

impl Link {
    /// Create a new link
    ///
    /// Parses the URL to extract domain and path, then inserts into the database.
    pub async fn create(
        pool: &PgPool,
        user_id: Uuid,
        create_link: CreateLink,
    ) -> Result<Link, AppError> {
        // Parse and validate URL
        let parsed_url = Url::parse(&create_link.url).map_err(|e| {
            AppError::validation("url", &format!("Invalid URL: {}", e))
        })?;

        // Extract domain and path
        let domain = parsed_url
            .host_str()
            .ok_or_else(|| AppError::validation("url", "URL must have a domain"))?
            .to_string();

        let path = {
            let p = parsed_url.path();
            if p.is_empty() || p == "/" {
                None
            } else {
                Some(p.to_string())
            }
        };

        // Check if it's a GitHub repo
        let is_github_repo = domain == "github.com"
            && path.as_ref().map_or(false, |p| {
                let parts: Vec<&str> = p.trim_matches('/').split('/').collect();
                parts.len() >= 2
            });

        tracing::info!(
            user_id = %user_id,
            url = %create_link.url,
            domain = %domain,
            "Creating new link"
        );

        let link = sqlx::query_as::<_, Link>(
            r#"
            INSERT INTO links (user_id, url, domain, path, title, description, logo, is_github_repo)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(&create_link.url)
        .bind(&domain)
        .bind(&path)
        .bind(&create_link.title)
        .bind(&create_link.description)
        .bind(&create_link.logo)
        .bind(is_github_repo)
        .fetch_one(pool)
        .await?;

        tracing::info!(link_id = %link.id, "Link created successfully");

        Ok(link)
    }

    /// Get a link by ID (must belong to user)
    pub async fn get_by_id(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<Link, AppError> {
        let link = sqlx::query_as::<_, Link>(
            r#"
            SELECT * FROM links
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::not_found("link", &id.to_string()))?;

        Ok(link)
    }

    /// Check if a link with the given URL exists for the user
    pub async fn exists_by_url(pool: &PgPool, user_id: Uuid, url: &str) -> Result<bool, AppError> {
        let exists = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(SELECT 1 FROM links WHERE user_id = $1 AND url = $2)
            "#,
        )
        .bind(user_id)
        .bind(url)
        .fetch_one(pool)
        .await?;

        Ok(exists)
    }

    /// Get all links for a user
    pub async fn get_all_by_user(pool: &PgPool, user_id: Uuid) -> Result<Vec<Link>, AppError> {
        let links = sqlx::query_as::<_, Link>(
            r#"
            SELECT * FROM links
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        Ok(links)
    }

    /// Search links with text query and filters
    ///
    /// Searches across title, description, url, and domain fields.
    /// Also supports filtering by status, GitHub repository flag, category, tag, language, and license.
    /// Results can be sorted by various fields in ascending or descending order.
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `user_id` - User ID to scope the search
    /// * `params` - Search parameters (query, status, is_github, category_id, tag_id, language_id, license_id, sort_by, sort_order)
    ///
    /// # Returns
    /// Vector of links matching the search criteria, sorted according to parameters
    pub async fn search(
        pool: &PgPool,
        user_id: Uuid,
        params: &LinkSearchParams,
    ) -> Result<Vec<Link>, AppError> {
        let query_pattern = params.query
            .as_ref()
            .map(|q| format!("%{}%", q.to_lowercase()));

        // Validate and build sort clause to prevent SQL injection
        let sort_field = match params.sort_by.as_deref() {
            Some("title") => "LOWER(l.title)",
            Some("github_stars") => "l.github_stars",
            Some("status") => "l.status",
            Some("updated_at") => "l.updated_at",
            _ => "l.created_at",  // default
        };

        let sort_order = match params.sort_order.as_deref() {
            Some("asc") => "ASC",
            _ => "DESC",  // default
        };

        // Build query with validated ORDER BY clause
        let query_str = format!(
            r#"
            SELECT DISTINCT l.* FROM links l
            LEFT JOIN link_categories lc ON l.id = lc.link_id
            LEFT JOIN link_tags lt ON l.id = lt.link_id
            LEFT JOIN link_languages ll ON l.id = ll.link_id
            LEFT JOIN link_licenses lli ON l.id = lli.link_id
            WHERE l.user_id = $1
            AND ($2::text IS NULL OR
                LOWER(l.title) LIKE $2 OR
                LOWER(l.description) LIKE $2 OR
                LOWER(l.url) LIKE $2 OR
                LOWER(l.domain) LIKE $2)
            AND ($3::text IS NULL OR l.status = $3)
            AND ($4::bool IS NULL OR l.is_github_repo = $4)
            AND ($5::uuid IS NULL OR lc.category_id = $5)
            AND ($6::uuid IS NULL OR lt.tag_id = $6)
            AND ($7::uuid IS NULL OR ll.language_id = $7)
            AND ($8::uuid IS NULL OR lli.license_id = $8)
            ORDER BY {} {} NULLS LAST
            "#,
            sort_field, sort_order
        );

        let links = sqlx::query_as::<_, Link>(&query_str)
            .bind(user_id)
            .bind(&query_pattern)
            .bind(&params.status)
            .bind(params.is_github)
            .bind(params.category_id)
            .bind(params.tag_id)
            .bind(params.language_id)
            .bind(params.license_id)
            .fetch_all(pool)
            .await?;

        Ok(links)
    }

    /// Search links with pagination support
    ///
    /// Similar to search() but returns paginated results with metadata.
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `user_id` - User ID to scope the search
    /// * `params` - Search parameters including pagination (page, per_page)
    ///
    /// # Returns
    /// PaginatedLinks with links for the current page and pagination metadata
    pub async fn search_paginated(
        pool: &PgPool,
        user_id: Uuid,
        params: &LinkSearchParams,
    ) -> Result<PaginatedLinks, AppError> {
        let query_pattern = params.query
            .as_ref()
            .map(|q| format!("%{}%", q.to_lowercase()));

        // Pagination parameters
        let page = params.page.unwrap_or(1).max(1);
        let per_page = params.per_page.unwrap_or(20).min(100);
        let offset = ((page - 1) * per_page) as i64;

        // Build WHERE clause for count query
        let count_query = r#"
            SELECT COUNT(DISTINCT l.id) as count
            FROM links l
            LEFT JOIN link_categories lc ON l.id = lc.link_id
            LEFT JOIN link_tags lt ON l.id = lt.link_id
            LEFT JOIN link_languages ll ON l.id = ll.link_id
            LEFT JOIN link_licenses lli ON l.id = lli.link_id
            WHERE l.user_id = $1
            AND ($2::text IS NULL OR
                LOWER(l.title) LIKE $2 OR
                LOWER(l.description) LIKE $2 OR
                LOWER(l.url) LIKE $2 OR
                LOWER(l.domain) LIKE $2)
            AND ($3::text IS NULL OR l.status = $3)
            AND ($4::bool IS NULL OR l.is_github_repo = $4)
            AND ($5::uuid IS NULL OR lc.category_id = $5)
            AND ($6::uuid IS NULL OR lt.tag_id = $6)
            AND ($7::uuid IS NULL OR ll.language_id = $7)
            AND ($8::uuid IS NULL OR lli.license_id = $8)
        "#;

        let count_result: (i64,) = sqlx::query_as(count_query)
            .bind(user_id)
            .bind(&query_pattern)
            .bind(&params.status)
            .bind(params.is_github)
            .bind(params.category_id)
            .bind(params.tag_id)
            .bind(params.language_id)
            .bind(params.license_id)
            .fetch_one(pool)
            .await?;

        let total = count_result.0;

        // Validate and build sort clause
        let sort_field = match params.sort_by.as_deref() {
            Some("title") => "LOWER(l.title)",
            Some("github_stars") => "l.github_stars",
            Some("status") => "l.status",
            Some("updated_at") => "l.updated_at",
            _ => "l.created_at",
        };

        let sort_order = match params.sort_order.as_deref() {
            Some("asc") => "ASC",
            _ => "DESC",
        };

        // Build query with pagination
        let query_str = format!(
            r#"
            SELECT DISTINCT l.* FROM links l
            LEFT JOIN link_categories lc ON l.id = lc.link_id
            LEFT JOIN link_tags lt ON l.id = lt.link_id
            LEFT JOIN link_languages ll ON l.id = ll.link_id
            LEFT JOIN link_licenses lli ON l.id = lli.link_id
            WHERE l.user_id = $1
            AND ($2::text IS NULL OR
                LOWER(l.title) LIKE $2 OR
                LOWER(l.description) LIKE $2 OR
                LOWER(l.url) LIKE $2 OR
                LOWER(l.domain) LIKE $2)
            AND ($3::text IS NULL OR l.status = $3)
            AND ($4::bool IS NULL OR l.is_github_repo = $4)
            AND ($5::uuid IS NULL OR lc.category_id = $5)
            AND ($6::uuid IS NULL OR lt.tag_id = $6)
            AND ($7::uuid IS NULL OR ll.language_id = $7)
            AND ($8::uuid IS NULL OR lli.license_id = $8)
            ORDER BY {} {} NULLS LAST
            LIMIT $9 OFFSET $10
            "#,
            sort_field, sort_order
        );

        let links = sqlx::query_as::<_, Link>(&query_str)
            .bind(user_id)
            .bind(&query_pattern)
            .bind(&params.status)
            .bind(params.is_github)
            .bind(params.category_id)
            .bind(params.tag_id)
            .bind(params.language_id)
            .bind(params.license_id)
            .bind(per_page as i64)
            .bind(offset)
            .fetch_all(pool)
            .await?;

        let total_pages = if total == 0 {
            1
        } else {
            ((total as f64) / (per_page as f64)).ceil() as u32
        };

        Ok(PaginatedLinks {
            links,
            total,
            page,
            per_page,
            total_pages,
        })
    }

    /// Get links that need refresh (not refreshed in the last N days)
    ///
    /// Returns links that are:
    /// - Active status (not archived or inaccessible)
    /// - Either never refreshed (refreshed_at IS NULL) or last refreshed more than N days ago
    ///
    /// Results are ordered by staleness (oldest first, never refreshed first)
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `stale_days` - Number of days after which a link is considered stale
    /// * `limit` - Maximum number of links to return
    pub async fn get_stale_links(
        pool: &PgPool,
        stale_days: u32,
        limit: i64,
    ) -> Result<Vec<Link>, AppError> {
        let stale_threshold = Utc::now() - chrono::Duration::days(stale_days as i64);

        let links = sqlx::query_as::<_, Link>(
            r#"
            SELECT * FROM links
            WHERE status = 'active'
            AND (refreshed_at IS NULL OR refreshed_at < $1)
            ORDER BY refreshed_at ASC NULLS FIRST
            LIMIT $2
            "#,
        )
        .bind(stale_threshold)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(links)
    }

    /// Update a link
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        update: UpdateLink,
    ) -> Result<Link, AppError> {
        // Validate status if provided
        if let Some(ref status) = update.status {
            let valid_statuses = ["active", "archived", "inaccessible", "repo_unavailable"];
            if !valid_statuses.contains(&status.as_str()) {
                return Err(AppError::validation(
                    "status",
                    "Status must be one of: active, archived, inaccessible, repo_unavailable",
                ));
            }
        }

        let link = sqlx::query_as::<_, Link>(
            r#"
            UPDATE links
            SET
                title = COALESCE($3, title),
                description = COALESCE($4, description),
                status = COALESCE($5, status),
                logo = COALESCE($6, logo),
                updated_at = NOW()
            WHERE id = $1 AND user_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(&update.title)
        .bind(&update.description)
        .bind(&update.status)
        .bind(&update.logo)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::not_found("link", &id.to_string()))?;

        tracing::info!(link_id = %id, "Link updated");

        Ok(link)
    }

    /// Delete a link (requires user_id for verification)
    pub async fn delete(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"
            DELETE FROM links
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found("link", &id.to_string()));
        }

        tracing::info!(link_id = %id, "Link deleted");

        Ok(())
    }

    /// Mark a link as active (sets status to "active")
    pub async fn mark_as_active(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
        Self::update_status(pool, id, "active").await
    }

    /// Add a category to a link
    pub async fn add_category(
        pool: &PgPool,
        link_id: Uuid,
        category_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        // Verify link belongs to user
        let _ = Self::get_by_id(pool, link_id, user_id).await?;

        // Verify category belongs to user
        let _ = Category::get_by_id(pool, category_id, user_id).await?;

        // Insert (ignore if already exists)
        sqlx::query(
            r#"
            INSERT INTO link_categories (link_id, category_id)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(link_id)
        .bind(category_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Remove a category from a link
    pub async fn remove_category(
        pool: &PgPool,
        link_id: Uuid,
        category_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        // Verify link belongs to user
        let _ = Self::get_by_id(pool, link_id, user_id).await?;

        sqlx::query(
            "DELETE FROM link_categories WHERE link_id = $1 AND category_id = $2",
        )
        .bind(link_id)
        .bind(category_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Get categories for a link
    pub async fn get_categories(
        pool: &PgPool,
        link_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Category>, AppError> {
        // Verify link belongs to user
        let _ = Self::get_by_id(pool, link_id, user_id).await?;

        let categories = sqlx::query_as::<_, Category>(
            r#"
            SELECT c.* FROM categories c
            JOIN link_categories lc ON lc.category_id = c.id
            WHERE lc.link_id = $1
            ORDER BY c.name
            "#,
        )
        .bind(link_id)
        .fetch_all(pool)
        .await?;

        Ok(categories)
    }

    /// Add a tag to a link
    pub async fn add_tag(
        pool: &PgPool,
        link_id: Uuid,
        tag_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        let _ = Self::get_by_id(pool, link_id, user_id).await?;
        let _ = Tag::get_by_id(pool, tag_id, user_id).await?;

        sqlx::query(
            r#"
            INSERT INTO link_tags (link_id, tag_id)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(link_id)
        .bind(tag_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Remove a tag from a link
    pub async fn remove_tag(
        pool: &PgPool,
        link_id: Uuid,
        tag_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        let _ = Self::get_by_id(pool, link_id, user_id).await?;

        sqlx::query("DELETE FROM link_tags WHERE link_id = $1 AND tag_id = $2")
            .bind(link_id)
            .bind(tag_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Get tags for a link
    pub async fn get_tags(
        pool: &PgPool,
        link_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Tag>, AppError> {
        let _ = Self::get_by_id(pool, link_id, user_id).await?;

        let tags = sqlx::query_as::<_, Tag>(
            r#"
            SELECT t.* FROM tags t
            JOIN link_tags lt ON lt.tag_id = t.id
            WHERE lt.link_id = $1
            ORDER BY t.name
            "#,
        )
        .bind(link_id)
        .fetch_all(pool)
        .await?;

        Ok(tags)
    }

    /// Add a language to a link
    pub async fn add_language(
        pool: &PgPool,
        link_id: Uuid,
        language_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        let _ = Self::get_by_id(pool, link_id, user_id).await?;

        sqlx::query(
            r#"
            INSERT INTO link_languages (link_id, language_id)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(link_id)
        .bind(language_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Remove a language from a link
    pub async fn remove_language(
        pool: &PgPool,
        link_id: Uuid,
        language_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        let _ = Self::get_by_id(pool, link_id, user_id).await?;

        sqlx::query("DELETE FROM link_languages WHERE link_id = $1 AND language_id = $2")
            .bind(link_id)
            .bind(language_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Get languages for a link
    pub async fn get_languages(
        pool: &PgPool,
        link_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Language>, AppError> {
        let _ = Self::get_by_id(pool, link_id, user_id).await?;

        let languages = sqlx::query_as::<_, Language>(
            r#"
            SELECT l.* FROM languages l
            JOIN link_languages ll ON ll.language_id = l.id
            WHERE ll.link_id = $1
            ORDER BY l.name
            "#,
        )
        .bind(link_id)
        .fetch_all(pool)
        .await?;

        Ok(languages)
    }

    /// Add a license to a link
    pub async fn add_license(
        pool: &PgPool,
        link_id: Uuid,
        license_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        let _ = Self::get_by_id(pool, link_id, user_id).await?;

        sqlx::query(
            r#"
            INSERT INTO link_licenses (link_id, license_id)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(link_id)
        .bind(license_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Remove a license from a link
    pub async fn remove_license(
        pool: &PgPool,
        link_id: Uuid,
        license_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        let _ = Self::get_by_id(pool, link_id, user_id).await?;

        sqlx::query("DELETE FROM link_licenses WHERE link_id = $1 AND license_id = $2")
            .bind(link_id)
            .bind(license_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Get licenses for a link
    pub async fn get_licenses(
        pool: &PgPool,
        link_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<License>, AppError> {
        let _ = Self::get_by_id(pool, link_id, user_id).await?;

        let licenses = sqlx::query_as::<_, License>(
            r#"
            SELECT l.* FROM licenses l
            JOIN link_licenses ll ON ll.license_id = l.id
            WHERE ll.link_id = $1
            ORDER BY l.name
            "#,
        )
        .bind(link_id)
        .fetch_all(pool)
        .await?;

        Ok(licenses)
    }

    /// Update scraped metadata for a link
    ///
    /// Updates title, description, and logo from web scraping results.
    /// Does not update refreshed_at - use mark_refreshed() for that.
    pub async fn update_scraped_metadata(
        pool: &PgPool,
        link_id: Uuid,
        user_id: Uuid,
        metadata: crate::scraper::ScrapedMetadata,
    ) -> Result<(), AppError> {
        // Verify link belongs to user
        let _ = Self::get_by_id(pool, link_id, user_id).await?;

        tracing::info!(
            link_id = %link_id,
            has_title = metadata.title.is_some(),
            has_description = metadata.description.is_some(),
            has_favicon = metadata.favicon.is_some(),
            "Updating scraped metadata for link"
        );

        sqlx::query(
            r#"
            UPDATE links
            SET
                title = COALESCE($2, title),
                description = COALESCE($3, description),
                logo = COALESCE($4, logo),
                updated_at = NOW()
            WHERE id = $1 AND user_id = $5
            "#,
        )
        .bind(link_id)
        .bind(&metadata.title)
        .bind(&metadata.description)
        .bind(&metadata.favicon)
        .bind(user_id)
        .execute(pool)
        .await?;

        tracing::info!(link_id = %link_id, "Scraped metadata updated successfully");

        Ok(())
    }

    /// Mark a link as refreshed
    ///
    /// Updates the refreshed_at timestamp to the current time.
    pub async fn mark_refreshed(
        pool: &PgPool,
        link_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        // Verify link belongs to user
        let _ = Self::get_by_id(pool, link_id, user_id).await?;

        tracing::debug!(link_id = %link_id, "Marking link as refreshed");

        sqlx::query(
            r#"
            UPDATE links
            SET refreshed_at = NOW()
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(link_id)
        .bind(user_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Update link status
    ///
    /// Updates the status field of a link. Valid statuses are:
    /// - "active": Link is working normally
    /// - "archived": User has archived the link
    /// - "inaccessible": Link returned an error or non-success status
    /// - "repo_unavailable": GitHub repository is unavailable (404, etc.)
    pub async fn update_status(
        pool: &PgPool,
        id: Uuid,
        status: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE links SET status = $1, updated_at = NOW() WHERE id = $2"
        )
        .bind(status)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Increment failure count and potentially mark as inaccessible
    ///
    /// Increments the consecutive_failures counter. After 3 consecutive failures,
    /// automatically marks the link as inaccessible.
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `id` - Link ID
    pub async fn record_failure(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE links
            SET consecutive_failures = consecutive_failures + 1,
                status = CASE
                    WHEN consecutive_failures >= 2 THEN 'inaccessible'
                    ELSE status
                END,
                updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Reset failure count on successful access
    ///
    /// Resets the consecutive_failures counter to 0. Should be called when
    /// a link is successfully accessed during health checks.
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `id` - Link ID
    pub async fn reset_failures(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE links SET consecutive_failures = 0, updated_at = NOW() WHERE id = $1"
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update GitHub metadata for a link
    ///
    /// Updates GitHub-specific fields and sets refreshed_at timestamp.
    /// This function should only be called for links where is_github_repo = true.
    pub async fn update_github_metadata(
        pool: &PgPool,
        link_id: Uuid,
        user_id: Uuid,
        metadata: crate::github::GitHubRepoMetadata,
    ) -> Result<(), AppError> {
        // Verify link belongs to user and is a GitHub repo
        let link = Self::get_by_id(pool, link_id, user_id).await?;

        if !link.is_github_repo {
            return Err(AppError::validation(
                "link",
                "This link is not a GitHub repository",
            ));
        }

        tracing::info!(
            link_id = %link_id,
            stars = metadata.stars,
            archived = metadata.archived,
            "Updating GitHub metadata for link"
        );

        sqlx::query(
            r#"
            UPDATE links
            SET
                github_stars = $2,
                github_archived = $3,
                github_last_commit = $4,
                refreshed_at = NOW(),
                updated_at = NOW()
            WHERE id = $1 AND user_id = $5
            "#,
        )
        .bind(link_id)
        .bind(metadata.stars)
        .bind(metadata.archived)
        .bind(metadata.last_commit)
        .bind(user_id)
        .execute(pool)
        .await?;

        tracing::info!(link_id = %link_id, "GitHub metadata updated successfully");

        Ok(())
    }

    /// Get all links with their metadata for a user
    pub async fn get_all_with_categories(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Vec<LinkWithCategories>, AppError> {
        let links = Self::get_all_by_user(pool, user_id).await?;

        let mut result = Vec::with_capacity(links.len());
        for link in links {
            let categories = sqlx::query_as::<_, Category>(
                r#"
                SELECT c.* FROM categories c
                JOIN link_categories lc ON lc.category_id = c.id
                WHERE lc.link_id = $1
                ORDER BY c.name
                "#,
            )
            .bind(link.id)
            .fetch_all(pool)
            .await?;

            let tags = sqlx::query_as::<_, Tag>(
                r#"
                SELECT t.* FROM tags t
                JOIN link_tags lt ON lt.tag_id = t.id
                WHERE lt.link_id = $1
                ORDER BY t.name
                "#,
            )
            .bind(link.id)
            .fetch_all(pool)
            .await?;

            let languages = sqlx::query_as::<_, Language>(
                r#"
                SELECT l.* FROM languages l
                JOIN link_languages ll ON ll.language_id = l.id
                WHERE ll.link_id = $1
                ORDER BY l.name
                "#,
            )
            .bind(link.id)
            .fetch_all(pool)
            .await?;

            let licenses = sqlx::query_as::<_, License>(
                r#"
                SELECT l.* FROM licenses l
                JOIN link_licenses ll ON ll.license_id = l.id
                WHERE ll.link_id = $1
                ORDER BY l.name
                "#,
            )
            .bind(link.id)
            .fetch_all(pool)
            .await?;

            result.push(LinkWithCategories { link, categories, tags, languages, licenses });
        }

        Ok(result)
    }

    /// Get links that need checking by the scheduler
    ///
    /// Returns links where:
    /// - last_checked is NULL (never checked), OR
    /// - last_checked is older than the specified interval in hours
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `interval_hours` - Number of hours after which a link needs checking
    /// * `limit` - Maximum number of links to return
    pub async fn get_links_needing_check(
        pool: &PgPool,
        interval_hours: u32,
        limit: i64,
    ) -> Result<Vec<Link>, AppError> {
        let check_threshold = Utc::now() - chrono::Duration::hours(interval_hours as i64);

        let links = sqlx::query_as::<_, Link>(
            r#"
            SELECT * FROM links
            WHERE status IN ('active', 'inaccessible', 'repo_unavailable')
            AND (last_checked IS NULL OR last_checked < $1)
            ORDER BY last_checked ASC NULLS FIRST
            LIMIT $2
            "#,
        )
        .bind(check_threshold)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(links)
    }

    /// Mark a link as checked by the scheduler
    ///
    /// Updates the last_checked timestamp to the current time.
    /// This is separate from mark_refreshed which is used for manual refreshes.
    pub async fn mark_checked(pool: &PgPool, link_id: Uuid) -> Result<(), AppError> {
        tracing::debug!(link_id = %link_id, "Marking link as checked");

        sqlx::query(
            r#"
            UPDATE links
            SET last_checked = NOW()
            WHERE id = $1
            "#,
        )
        .bind(link_id)
        .execute(pool)
        .await?;

        Ok(())
    }
}

/// Link with its associated metadata
#[derive(Debug, Clone, Serialize)]
pub struct LinkWithCategories {
    #[serde(flatten)]
    pub link: Link,
    pub categories: Vec<Category>,
    pub tags: Vec<Tag>,
    pub languages: Vec<Language>,
    pub licenses: Vec<License>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_parsing() {
        let url = Url::parse("https://github.com/rust-lang/rust").unwrap();
        assert_eq!(url.host_str(), Some("github.com"));
        assert_eq!(url.path(), "/rust-lang/rust");
    }

    #[test]
    fn test_github_detection() {
        // Valid GitHub repo URLs
        let github_urls = [
            "https://github.com/rust-lang/rust",
            "https://github.com/owner/repo",
        ];

        for url_str in &github_urls {
            let parsed = Url::parse(url_str).unwrap();
            let domain = parsed.host_str().unwrap();
            let path = parsed.path();
            let parts: Vec<&str> = path.trim_matches('/').split('/').collect();

            assert_eq!(domain, "github.com");
            assert!(parts.len() >= 2, "Should have owner and repo in path");
        }
    }
}
