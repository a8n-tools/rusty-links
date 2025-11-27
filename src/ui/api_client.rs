use serde::Serialize;
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
