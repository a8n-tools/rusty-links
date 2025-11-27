use serde::{Deserialize, Serialize};
use crate::ui::components::table::links_table::Link;

#[derive(Serialize)]
pub struct CreateLinkRequest {
    pub url: String,
}

/// Check if a URL already exists in the database
pub async fn check_duplicate_url(url: &str) -> Result<Option<Link>, String> {
    let client = reqwest::Client::new();
    let encoded_url = urlencoding::encode(url);
    let api_url = format!("/api/links/check-duplicate?url={}", encoded_url);

    let response = client.get(&api_url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        let result: Option<Link> = response.json().await
            .map_err(|e| format!("Parse error: {}", e))?;
        Ok(result)
    } else if response.status().as_u16() == 404 {
        // No duplicate found
        Ok(None)
    } else {
        Err(format!("Server error: {}", response.status()))
    }
}

/// Create a new link
pub async fn create_link_request(url: &str) -> Result<Link, String> {
    let client = reqwest::Client::new();
    let request_body = CreateLinkRequest {
        url: url.to_string(),
    };

    let response = client.post("/api/links")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        let link: Link = response.json().await
            .map_err(|e| format!("Parse error: {}", e))?;
        Ok(link)
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("Failed to create link: {}", error_text))
    }
}

/// Fetch link details by ID
pub async fn fetch_link_details(link_id: &str) -> Result<Link, String> {
    let client = reqwest::Client::new();
    let api_url = format!("/api/links/{}", link_id);

    let response = client.get(&api_url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        let link: Link = response.json().await
            .map_err(|e| format!("Parse error: {}", e))?;
        Ok(link)
    } else {
        Err(format!("Server error: {}", response.status()))
    }
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
    let client = reqwest::Client::new();
    let response = client.get("/api/categories")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        let categories: Vec<CategoryNode> = response.json().await
            .map_err(|e| format!("Parse error: {}", e))?;
        Ok(build_category_tree(categories))
    } else {
        Err(format!("Server error: {}", response.status()))
    }
}

/// Fetch a single category by ID
pub async fn fetch_category(id: &str) -> Result<CategoryNode, String> {
    let client = reqwest::Client::new();
    let url = format!("/api/categories/{}", id);

    let response = client.get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        response.json().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        Err(format!("Server error: {}", response.status()))
    }
}

/// Create a new category
pub async fn create_category(name: &str, parent_id: Option<String>) -> Result<CategoryNode, String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "name": name,
        "parent_id": parent_id,
    });

    let response = client.post("/api/categories")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        response.json().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("Failed to create category: {}", error_text))
    }
}

/// Update a category's name
pub async fn update_category(id: &str, name: &str) -> Result<CategoryNode, String> {
    let client = reqwest::Client::new();
    let url = format!("/api/categories/{}", id);
    let body = serde_json::json!({ "name": name });

    let response = client.put(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        response.json().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("Failed to update category: {}", error_text))
    }
}

/// Delete a category
pub async fn delete_category(id: &str) -> Result<(), String> {
    let client = reqwest::Client::new();
    let url = format!("/api/categories/{}", id);

    let response = client.delete(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("Failed to delete category: {}", error_text))
    }
}

/// Move a category to a new parent
pub async fn move_category(id: &str, new_parent_id: Option<String>) -> Result<CategoryNode, String> {
    let client = reqwest::Client::new();
    let url = format!("/api/categories/{}/move", id);
    let body = serde_json::json!({ "parent_id": new_parent_id });

    let response = client.put(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        response.json().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("Failed to move category: {}", error_text))
    }
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
    let client = reqwest::Client::new();
    let response = client.get("/api/languages")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        response.json().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        Err(format!("Server error: {}", response.status()))
    }
}

/// Fetch a single language by ID
pub async fn fetch_language(id: &str) -> Result<LanguageItem, String> {
    let client = reqwest::Client::new();
    let url = format!("/api/languages/{}", id);

    let response = client.get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        response.json().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        Err(format!("Server error: {}", response.status()))
    }
}

/// Create a new language
pub async fn create_language(name: &str) -> Result<LanguageItem, String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({ "name": name });

    let response = client.post("/api/languages")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        response.json().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("Failed to create language: {}", error_text))
    }
}

/// Update a language's name
pub async fn update_language(id: &str, name: &str) -> Result<LanguageItem, String> {
    let client = reqwest::Client::new();
    let url = format!("/api/languages/{}", id);
    let body = serde_json::json!({ "name": name });

    let response = client.put(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        response.json().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("Failed to update language: {}", error_text))
    }
}

/// Delete a language
pub async fn delete_language(id: &str) -> Result<(), String> {
    let client = reqwest::Client::new();
    let url = format!("/api/languages/{}", id);

    let response = client.delete(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("Failed to delete language: {}", error_text))
    }
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
    let client = reqwest::Client::new();
    let response = client.get("/api/licenses")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        response.json().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        Err(format!("Server error: {}", response.status()))
    }
}

/// Fetch a single license by ID
pub async fn fetch_license(id: &str) -> Result<LicenseItem, String> {
    let client = reqwest::Client::new();
    let url = format!("/api/licenses/{}", id);

    let response = client.get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        response.json().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        Err(format!("Server error: {}", response.status()))
    }
}

/// Create a new license
pub async fn create_license(name: &str, acronym: Option<String>) -> Result<LicenseItem, String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "name": name,
        "acronym": acronym,
    });

    let response = client.post("/api/licenses")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        response.json().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("Failed to create license: {}", error_text))
    }
}

/// Update a license
pub async fn update_license(id: &str, name: &str, acronym: Option<String>) -> Result<LicenseItem, String> {
    let client = reqwest::Client::new();
    let url = format!("/api/licenses/{}", id);
    let body = serde_json::json!({
        "name": name,
        "acronym": acronym,
    });

    let response = client.put(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        response.json().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("Failed to update license: {}", error_text))
    }
}

/// Delete a license
pub async fn delete_license(id: &str) -> Result<(), String> {
    let client = reqwest::Client::new();
    let url = format!("/api/licenses/{}", id);

    let response = client.delete(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("Failed to delete license: {}", error_text))
    }
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
    let client = reqwest::Client::new();
    let response = client.get("/api/tags")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        response.json().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        Err(format!("Server error: {}", response.status()))
    }
}

/// Fetch a single tag by ID
pub async fn fetch_tag(id: &str) -> Result<TagItem, String> {
    let client = reqwest::Client::new();
    let url = format!("/api/tags/{}", id);

    let response = client.get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        response.json().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        Err(format!("Server error: {}", response.status()))
    }
}

/// Create a new tag
pub async fn create_tag(name: &str) -> Result<TagItem, String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({ "name": name });

    let response = client.post("/api/tags")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        response.json().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("Failed to create tag: {}", error_text))
    }
}

/// Update a tag's name
pub async fn update_tag(id: &str, name: &str) -> Result<TagItem, String> {
    let client = reqwest::Client::new();
    let url = format!("/api/tags/{}", id);
    let body = serde_json::json!({ "name": name });

    let response = client.put(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        response.json().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("Failed to update tag: {}", error_text))
    }
}

/// Delete a tag
pub async fn delete_tag(id: &str) -> Result<(), String> {
    let client = reqwest::Client::new();
    let url = format!("/api/tags/{}", id);

    let response = client.delete(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("Failed to delete tag: {}", error_text))
    }
}
