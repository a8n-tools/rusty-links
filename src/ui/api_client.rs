use crate::ui::components::table::links_table::Link;
use crate::ui::http;
use serde::{Deserialize, Serialize};
use std::future::Future;

#[derive(Serialize)]
pub struct CreateLinkRequest {
    pub url: String,
}

#[derive(Serialize)]
pub struct CreateLinkWithCategoriesRequest {
    pub url: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub category_ids: Vec<uuid::Uuid>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tag_ids: Vec<uuid::Uuid>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub language_ids: Vec<uuid::Uuid>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub license_ids: Vec<uuid::Uuid>,
}

/// Retry configuration
const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 1000;

// Re-export http module functions as the preferred API
pub use crate::ui::http::{
    delete, get, get_response, patch, post, post_empty, post_response, put, HttpResponse,
};

/// Retry a future with exponential backoff
pub async fn retry_with_backoff<F, Fut, T>(mut f: F) -> Result<T, String>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, String>>,
{
    let mut attempts = 0;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                attempts += 1;
                if attempts >= MAX_RETRIES {
                    return Err(format!("Failed after {} attempts: {}", MAX_RETRIES, e));
                }

                // Only retry on network errors, not client/server errors
                if !e.contains("Network error") {
                    return Err(e);
                }

                let delay = RETRY_DELAY_MS * 2u64.pow(attempts - 1);

                #[cfg(target_arch = "wasm32")]
                {
                    gloo_timers::future::TimeoutFuture::new(delay as u32).await;
                }

                #[cfg(all(not(target_arch = "wasm32"), feature = "server"))]
                {
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                }
            }
        }
    }
}

/// Check if a URL already exists in the database
pub async fn check_duplicate_url(url: &str) -> Result<Option<Link>, String> {
    let encoded_url = urlencoding::encode(url);
    let api_url = format!("/api/links/check-duplicate?url={}", encoded_url);

    let response = http::get_response(&api_url).await?;

    if response.is_success() {
        let result: Option<Link> = response.json()?;
        Ok(result)
    } else if response.status == 404 {
        // No duplicate found
        Ok(None)
    } else {
        Err(format!("Server error: {}", response.status))
    }
}

/// Create a new link
pub async fn create_link_request(url: &str) -> Result<Link, String> {
    let request_body = CreateLinkRequest {
        url: url.to_string(),
    };

    http::post("/api/links", &request_body).await
}

/// Create a new link with initial categorization
pub async fn create_link_with_categories(
    request: &CreateLinkWithCategoriesRequest,
) -> Result<Link, String> {
    http::post("/api/links", request).await
}

/// Fetch link details by ID
pub async fn fetch_link_details(link_id: &str) -> Result<Link, String> {
    let api_url = format!("/api/links/{}", link_id);
    http::get(&api_url).await
}

// ==================== Category Management ====================

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CategoryNode {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub depth: i32,
    pub link_count: i64,
    #[serde(default)]
    pub children: Vec<CategoryNode>,
}

/// Fetch all categories as a tree structure
pub async fn fetch_categories() -> Result<Vec<CategoryNode>, String> {
    let categories: Vec<CategoryNode> = http::get("/api/categories").await?;
    Ok(build_category_tree(categories))
}

/// Fetch a single category by ID
pub async fn fetch_category(id: &str) -> Result<CategoryNode, String> {
    let url = format!("/api/categories/{}", id);
    http::get(&url).await
}

/// Create a new category
pub async fn create_category(
    name: &str,
    parent_id: Option<String>,
) -> Result<CategoryNode, String> {
    let body = serde_json::json!({
        "name": name,
        "parent_id": parent_id,
    });
    http::post("/api/categories", &body).await
}

/// Update a category's name
pub async fn update_category(id: &str, name: &str) -> Result<CategoryNode, String> {
    let url = format!("/api/categories/{}", id);
    let body = serde_json::json!({ "name": name });
    http::put(&url, &body).await
}

/// Delete a category
pub async fn delete_category(id: &str) -> Result<(), String> {
    let url = format!("/api/categories/{}", id);
    http::delete(&url).await
}

/// Move a category to a new parent
pub async fn move_category(
    id: &str,
    new_parent_id: Option<String>,
) -> Result<CategoryNode, String> {
    let url = format!("/api/categories/{}/move", id);
    let body = serde_json::json!({ "parent_id": new_parent_id });
    http::put(&url, &body).await
}

/// Build a tree structure from flat list of categories
fn build_category_tree(mut categories: Vec<CategoryNode>) -> Vec<CategoryNode> {
    use std::collections::HashMap;

    // Create a map for quick lookup
    let mut map: HashMap<String, CategoryNode> = HashMap::new();
    for cat in categories.drain(..) {
        map.insert(cat.id.clone(), cat);
    }

    // Build tree
    let mut roots = Vec::new();
    let keys: Vec<String> = map.keys().cloned().collect();

    for id in keys {
        if let Some(cat) = map.remove(&id) {
            if let Some(parent_id) = &cat.parent_id {
                // Add to parent's children
                if let Some(parent) = map.get_mut(parent_id) {
                    parent.children.push(cat);
                } else {
                    // Parent doesn't exist, treat as root
                    roots.push(cat);
                }
            } else {
                // No parent, it's a root
                roots.push(cat);
            }
        }
    }

    // Collect remaining nodes from map (these are parents)
    for (_, cat) in map {
        roots.push(cat);
    }

    roots
}

// ==================== Language Management ====================

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct LanguageItem {
    pub id: String,
    pub name: String,
    pub link_count: i64,
}

/// Fetch all languages
pub async fn fetch_languages() -> Result<Vec<LanguageItem>, String> {
    http::get("/api/languages").await
}

/// Fetch a single language by ID
pub async fn fetch_language(id: &str) -> Result<LanguageItem, String> {
    let url = format!("/api/languages/{}", id);
    http::get(&url).await
}

/// Create a new language
pub async fn create_language(name: &str) -> Result<LanguageItem, String> {
    let body = serde_json::json!({ "name": name });
    http::post("/api/languages", &body).await
}

/// Update a language's name
pub async fn update_language(id: &str, name: &str) -> Result<LanguageItem, String> {
    let url = format!("/api/languages/{}", id);
    let body = serde_json::json!({ "name": name });
    http::put(&url, &body).await
}

/// Delete a language
pub async fn delete_language(id: &str) -> Result<(), String> {
    let url = format!("/api/languages/{}", id);
    http::delete(&url).await
}

// ==================== License Management ====================

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct LicenseItem {
    pub id: String,
    pub name: String,
    pub acronym: Option<String>,
    pub link_count: i64,
}

/// Fetch all licenses
pub async fn fetch_licenses() -> Result<Vec<LicenseItem>, String> {
    http::get("/api/licenses").await
}

/// Fetch a single license by ID
pub async fn fetch_license(id: &str) -> Result<LicenseItem, String> {
    let url = format!("/api/licenses/{}", id);
    http::get(&url).await
}

/// Create a new license
pub async fn create_license(name: &str, acronym: Option<String>) -> Result<LicenseItem, String> {
    let body = serde_json::json!({
        "name": name,
        "acronym": acronym,
    });
    http::post("/api/licenses", &body).await
}

/// Update a license
pub async fn update_license(
    id: &str,
    name: &str,
    acronym: Option<String>,
) -> Result<LicenseItem, String> {
    let url = format!("/api/licenses/{}", id);
    let body = serde_json::json!({
        "name": name,
        "acronym": acronym,
    });
    http::put(&url, &body).await
}

/// Delete a license
pub async fn delete_license(id: &str) -> Result<(), String> {
    let url = format!("/api/licenses/{}", id);
    http::delete(&url).await
}

// ==================== Tag Management ====================

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct TagItem {
    pub id: String,
    pub name: String,
    pub link_count: i64,
}

/// Fetch all tags
pub async fn fetch_tags() -> Result<Vec<TagItem>, String> {
    http::get("/api/tags").await
}

/// Fetch a single tag by ID
pub async fn fetch_tag(id: &str) -> Result<TagItem, String> {
    let url = format!("/api/tags/{}", id);
    http::get(&url).await
}

/// Create a new tag
pub async fn create_tag(name: &str) -> Result<TagItem, String> {
    let body = serde_json::json!({ "name": name });
    http::post("/api/tags", &body).await
}

/// Update a tag's name
pub async fn update_tag(id: &str, name: &str) -> Result<TagItem, String> {
    let url = format!("/api/tags/{}", id);
    let body = serde_json::json!({ "name": name });
    http::put(&url, &body).await
}

/// Delete a tag
pub async fn delete_tag(id: &str) -> Result<(), String> {
    let url = format!("/api/tags/{}", id);
    http::delete(&url).await
}

// ==================== Link Preview ====================

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LinkPreview {
    pub url: String,
    pub domain: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub favicon: Option<String>,
    pub is_github_repo: bool,
    pub github_stars: Option<i32>,
    pub github_description: Option<String>,
    pub github_languages: Vec<String>,
    pub github_license: Option<String>,
}

/// Fetch metadata preview for a URL without creating the link
pub async fn preview_link(url: &str) -> Result<LinkPreview, String> {
    let body = serde_json::json!({ "url": url });
    http::post("/api/links/preview", &body).await
}
