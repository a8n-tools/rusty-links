use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::models::link::Link as DbLink;
#[cfg(feature = "server")]
use sqlx::PgPool;

// Import the global DB pool from auth module
#[cfg(feature = "server")]
use crate::server_functions::auth::{DB_POOL};

/// Link data for client
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Link {
    pub id: String,
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Request to create a new link
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateLinkRequest {
    pub url: String,
}

/// Paginated links response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaginatedLinksResponse {
    pub links: Vec<Link>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
}

/// Get all links with pagination
#[server]
pub async fn get_links(
    page: u32,
    per_page: u32,
    search: Option<String>,
    category_id: Option<String>,
    tag_id: Option<String>,
) -> Result<PaginatedLinksResponse, ServerFnError> {
    let pool = extract_pool()?;

    let offset = ((page - 1) * per_page) as i64;
    let limit = per_page as i64;

    // Build query based on filters
    let (links, total) = if search.is_some() || category_id.is_some() || tag_id.is_some() {
        // Filtered query
        let mut query = String::from("SELECT * FROM links WHERE 1=1");
        let mut count_query = String::from("SELECT COUNT(*) FROM links WHERE 1=1");

        if let Some(ref s) = search {
            let search_clause = format!(" AND (title ILIKE '%{}%' OR description ILIKE '%{}%')", s, s);
            query.push_str(&search_clause);
            count_query.push_str(&search_clause);
        }

        if let Some(ref cat) = category_id {
            let cat_clause = format!(" AND category_id = '{}'", cat);
            query.push_str(&cat_clause);
            count_query.push_str(&cat_clause);
        }

        query.push_str(&format!(" ORDER BY created_at DESC LIMIT {} OFFSET {}", limit, offset));

        let links: Vec<DbLink> = sqlx::query_as(&query)
            .fetch_all(pool)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

        let total: (i64,) = sqlx::query_as(&count_query)
            .fetch_one(pool)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

        (links, total.0)
    } else {
        // Get all links
        let links: Vec<DbLink> = sqlx::query_as(
            "SELECT * FROM links ORDER BY created_at DESC LIMIT $1 OFFSET $2"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM links")
            .fetch_one(pool)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

        (links, total.0)
    };

    Ok(PaginatedLinksResponse {
        links: links.into_iter().map(convert_link).collect(),
        total,
        page,
        per_page,
    })
}

/// Create a new link
#[server]
pub async fn create_link(request: CreateLinkRequest) -> Result<Link, ServerFnError> {
    let pool = extract_pool()?;

    // TODO: Get user_id from session context
    // For now, using a placeholder - this needs proper session management
    let user_id = uuid::Uuid::new_v4(); // TEMPORARY

    let create_req = crate::models::link::CreateLink {
        url: request.url,
        title: None,
        description: None,
        logo: None,
    };

    let link = DbLink::create(pool, user_id, create_req)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create link: {}", e)))?;

    Ok(convert_link(link))
}

/// Delete a link
#[server]
pub async fn delete_link(id: String) -> Result<(), ServerFnError> {
    let pool = extract_pool()?;

    let uuid = uuid::Uuid::parse_str(&id)
        .map_err(|e| ServerFnError::new(format!("Invalid ID: {}", e)))?;

    // TODO: Get user_id from session context
    let user_id = uuid::Uuid::new_v4(); // TEMPORARY

    DbLink::delete(pool, uuid, user_id)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to delete link: {}", e)))?;

    Ok(())
}

/// Mark link as active
#[server]
pub async fn mark_link_active(id: String) -> Result<(), ServerFnError> {
    let pool = extract_pool()?;

    let uuid = uuid::Uuid::parse_str(&id)
        .map_err(|e| ServerFnError::new(format!("Invalid ID: {}", e)))?;

    DbLink::mark_as_active(pool, uuid)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to mark link as active: {}", e)))?;

    Ok(())
}

#[cfg(feature = "server")]
fn extract_pool() -> Result<&'static PgPool, ServerFnError> {
    DB_POOL
        .get()
        .ok_or_else(|| ServerFnError::new("Database pool not initialized"))
}

#[cfg(feature = "server")]
fn convert_link(link: DbLink) -> Link {
    Link {
        id: link.id.to_string(),
        url: link.url,
        title: link.title,
        description: link.description,
        status: link.status,
        created_at: link.created_at.to_rfc3339(),
        updated_at: link.updated_at.to_rfc3339(),
    }
}
