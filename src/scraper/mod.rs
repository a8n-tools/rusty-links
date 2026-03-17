//! Web scraping module for extracting metadata from URLs
//!
//! This module provides functionality to scrape basic metadata from web pages
//! including title, description, and favicon.

use crate::error::AppError;
use scraper::{Html, Selector};
use std::time::Duration;
use url::Url;

/// Scraped metadata from a web page
#[derive(Debug, Clone)]
pub struct ScrapedMetadata {
    /// Page title from <title> tag or og:title meta tag
    pub title: Option<String>,
    /// Page description from meta description or og:description
    pub description: Option<String>,
    /// Favicon URL (absolute)
    pub favicon: Option<String>,
}

impl Default for ScrapedMetadata {
    fn default() -> Self {
        Self {
            title: None,
            description: None,
            favicon: None,
        }
    }
}

/// Scrape metadata from a given URL
///
/// Makes an HTTP request to the URL and extracts title, description, and favicon.
/// Handles errors gracefully - if scraping fails, returns empty metadata instead of error.
///
/// # Arguments
/// * `url` - The URL to scrape
///
/// # Returns
/// * `Ok(ScrapedMetadata)` - Scraped metadata (fields may be None if not found)
/// * `Err(AppError)` - Only returns error if the URL is completely invalid or unreachable
pub async fn scrape_url(url: &str) -> Result<ScrapedMetadata, AppError> {
    // Parse URL to validate and use for absolute URL construction
    let base_url =
        Url::parse(url).map_err(|e| AppError::validation("url", &format!("Invalid URL: {}", e)))?;

    crate::security::validate_url_for_ssrf(url)?;

    // Build HTTP client with timeout and redirects
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::limited(5))
        .user_agent("Mozilla/5.0 (compatible; RustyLinks/1.0; +https://github.com/rusty-links)")
        .build()
        .map_err(|e| AppError::ExternalService(format!("Failed to create HTTP client: {}", e)))?;

    // Fetch the page
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| AppError::ExternalService(format!("Failed to fetch URL: {}", e)))?;

    // Check if response is HTML
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !content_type.contains("text/html") {
        tracing::debug!("Non-HTML response, content-type: {}", content_type);
        return Ok(ScrapedMetadata::default());
    }

    // Get response text
    let html = response
        .text()
        .await
        .map_err(|e| AppError::ExternalService(format!("Failed to read response: {}", e)))?;

    // Parse HTML and extract metadata synchronously
    let (title, description, favicon_candidates) = {
        let document = Html::parse_document(&html);
        (
            extract_title(&document),
            extract_description(&document),
            extract_favicon(&document, &base_url),
        )
    };
    // document is dropped here, before any await

    // Create metadata
    let mut metadata = ScrapedMetadata::default();
    metadata.title = title;
    metadata.description = description;

    // Validate favicon candidates (async, no reference to Html)
    metadata.favicon = validate_favicon_candidates(&client, favicon_candidates).await;

    Ok(metadata)
}

/// Check if a URL is accessible (returns HTTP 2xx or 3xx)
///
/// Makes a HEAD request to the URL to check if it's accessible without
/// downloading the full page content. This is used to detect broken links.
///
/// # Arguments
/// * `url` - The URL to check
///
/// # Returns
/// * `Ok(true)` - URL is accessible (returns 2xx or 3xx status)
/// * `Ok(false)` - URL is not accessible (connection failed or non-success status)
/// * `Err(AppError)` - Only returns error if client creation fails
pub async fn check_url_health(url: &str) -> Result<bool, AppError> {
    let _ = url::Url::parse(url)
        .map_err(|e| AppError::validation("url", &format!("Invalid URL: {}", e)))?;
    crate::security::validate_url_for_ssrf(url)?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Try HEAD first
    match client.head(url).send().await {
        Ok(response) => {
            let status = response.status();
            if status.is_success() || status.is_redirection() {
                return Ok(true);
            }
            // Fall back to GET for methods that reject HEAD
            if status == reqwest::StatusCode::METHOD_NOT_ALLOWED
                || status == reqwest::StatusCode::FORBIDDEN
                || status == reqwest::StatusCode::NOT_IMPLEMENTED
            {
                tracing::debug!(url = %url, status = %status, "HEAD rejected, falling back to GET with Range header");
            } else {
                return Ok(false);
            }
        }
        Err(e) => {
            tracing::debug!(url = %url, error = %e, "HEAD request failed, falling back to GET with Range header");
        }
    }

    // Fallback: GET with Range header to minimize data transfer
    match client
        .get(url)
        .header(reqwest::header::RANGE, "bytes=0-0")
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            Ok(status.is_success()
                || status.is_redirection()
                || status == reqwest::StatusCode::PARTIAL_CONTENT)
        }
        Err(e) => {
            tracing::debug!(url = %url, error = %e, "GET fallback also failed");
            Ok(false)
        }
    }
}

/// Extract title from HTML document
fn extract_title(document: &Html) -> Option<String> {
    // Try og:title first
    if let Ok(selector) = Selector::parse("meta[property='og:title']") {
        if let Some(element) = document.select(&selector).next() {
            if let Some(content) = element.value().attr("content") {
                let title = content.trim();
                if !title.is_empty() {
                    return Some(title.to_string());
                }
            }
        }
    }

    // Try regular title tag
    if let Ok(selector) = Selector::parse("title") {
        if let Some(element) = document.select(&selector).next() {
            let title = element.text().collect::<String>().trim().to_string();
            if !title.is_empty() {
                return Some(title);
            }
        }
    }

    None
}

/// Extract description from HTML document
fn extract_description(document: &Html) -> Option<String> {
    // Try og:description first
    if let Ok(selector) = Selector::parse("meta[property='og:description']") {
        if let Some(element) = document.select(&selector).next() {
            if let Some(content) = element.value().attr("content") {
                let desc = content.trim();
                if !desc.is_empty() {
                    return Some(desc.to_string());
                }
            }
        }
    }

    // Try standard meta description
    if let Ok(selector) = Selector::parse("meta[name='description']") {
        if let Some(element) = document.select(&selector).next() {
            if let Some(content) = element.value().attr("content") {
                let desc = content.trim();
                if !desc.is_empty() {
                    return Some(desc.to_string());
                }
            }
        }
    }

    None
}

/// Valid image MIME types for favicons
const VALID_IMAGE_MIME_TYPES: &[&str] = &[
    "image/png",
    "image/jpeg",
    "image/jpg",
    "image/gif",
    "image/x-icon",
    "image/vnd.microsoft.icon",
    "image/ico",
    "image/icon",
    "image/svg+xml",
    "image/webp",
    "image/avif",
    "image/bmp",
];

/// Valid image file extensions for favicons
const VALID_IMAGE_EXTENSIONS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".gif", ".ico", ".svg", ".webp", ".avif", ".bmp",
];

/// Validate that a favicon/image URL exists and returns a valid image
///
/// Makes a HEAD request to check:
/// 1. The URL returns HTTP 2xx status
/// 2. The Content-Type is a valid image MIME type
///
/// # Arguments
/// * `url` - The favicon/image URL to validate
///
/// # Returns
/// * `Ok(true)` if the URL exists and is a valid image
/// * `Ok(false)` if the URL doesn't exist or isn't a valid image
/// * `Err` if the HTTP client couldn't be created
pub async fn validate_image_url(url: &str) -> Result<bool, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))?;

    Ok(validate_favicon_url_with_client(&client, url).await)
}

/// Internal function to validate favicon URL with a provided client
async fn validate_favicon_url_with_client(client: &reqwest::Client, url: &str) -> bool {
    match client.head(url).send().await {
        Ok(response) => {
            // Check for success status
            if !response.status().is_success() {
                tracing::debug!(url = %url, status = %response.status(), "Favicon URL returned non-success status");
                return false;
            }

            // Check Content-Type header
            if let Some(content_type) = response.headers().get(reqwest::header::CONTENT_TYPE) {
                if let Ok(content_type_str) = content_type.to_str() {
                    let content_type_lower = content_type_str.to_lowercase();
                    let is_valid = VALID_IMAGE_MIME_TYPES
                        .iter()
                        .any(|mime| content_type_lower.contains(mime));

                    if !is_valid {
                        tracing::debug!(
                            url = %url,
                            content_type = %content_type_str,
                            "Favicon URL has invalid Content-Type"
                        );
                        return false;
                    }
                    return true;
                }
            }

            // If no Content-Type header, check file extension as fallback
            let url_lower = url.to_lowercase();
            let has_valid_extension = VALID_IMAGE_EXTENSIONS
                .iter()
                .any(|ext| url_lower.ends_with(ext));

            if has_valid_extension {
                tracing::debug!(url = %url, "Favicon URL has valid extension, accepting without Content-Type");
                return true;
            }

            tracing::debug!(url = %url, "Favicon URL has no Content-Type and no valid extension");
            false
        }
        Err(e) => {
            tracing::debug!(url = %url, error = %e, "Failed to validate favicon URL");
            false
        }
    }
}

/// Extract favicon URL from HTML document
fn extract_favicon(document: &Html, base_url: &Url) -> Vec<String> {
    let mut candidates = Vec::new();

    // Try various link rel attributes
    let rel_attributes = [
        "icon",
        "shortcut icon",
        "apple-touch-icon",
        "apple-touch-icon-precomposed",
    ];

    for rel in &rel_attributes {
        if let Ok(selector) = Selector::parse(&format!("link[rel='{}']", rel)) {
            if let Some(element) = document.select(&selector).next() {
                if let Some(href) = element.value().attr("href") {
                    // Convert relative URLs to absolute
                    if let Ok(favicon_url) = base_url.join(href) {
                        candidates.push(favicon_url.to_string());
                    }
                }
            }
        }
    }

    // Fallback to default /favicon.ico
    if let Ok(default_favicon) = base_url.join("/favicon.ico") {
        candidates.push(default_favicon.to_string());
    }

    candidates
}

/// Validate favicon candidates and return the first valid one
///
/// Tries multiple favicon candidates and returns the first one that:
/// 1. Returns HTTP 2xx status
/// 2. Has a valid image Content-Type
async fn validate_favicon_candidates(
    client: &reqwest::Client,
    candidates: Vec<String>,
) -> Option<String> {
    for candidate in candidates {
        if validate_favicon_url_with_client(client, &candidate).await {
            tracing::debug!(url = %candidate, "Found valid favicon");
            return Some(candidate);
        }
    }

    tracing::debug!("No valid favicon found");
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_title_from_title_tag() {
        let html = r#"
            <html>
                <head><title>Test Page</title></head>
                <body></body>
            </html>
        "#;
        let document = Html::parse_document(html);
        let title = extract_title(&document);
        assert_eq!(title, Some("Test Page".to_string()));
    }

    #[test]
    fn test_extract_title_from_og_title() {
        let html = r#"
            <html>
                <head>
                    <meta property="og:title" content="OG Title" />
                    <title>Regular Title</title>
                </head>
                <body></body>
            </html>
        "#;
        let document = Html::parse_document(html);
        let title = extract_title(&document);
        assert_eq!(title, Some("OG Title".to_string()));
    }

    #[test]
    fn test_extract_title_none_when_missing() {
        let html = r#"<html><head></head><body></body></html>"#;
        let document = Html::parse_document(html);
        assert_eq!(extract_title(&document), None);
    }

    #[test]
    fn test_extract_title_empty_title_tag() {
        let html = r#"<html><head><title>   </title></head></html>"#;
        let document = Html::parse_document(html);
        assert_eq!(extract_title(&document), None);
    }

    #[test]
    fn test_extract_title_empty_og_title_falls_back() {
        let html = r#"
            <html>
                <head>
                    <meta property="og:title" content="   " />
                    <title>Fallback Title</title>
                </head>
            </html>
        "#;
        let document = Html::parse_document(html);
        assert_eq!(extract_title(&document), Some("Fallback Title".to_string()));
    }

    #[test]
    fn test_extract_description() {
        let html = r#"
            <html>
                <head>
                    <meta name="description" content="Test description" />
                </head>
                <body></body>
            </html>
        "#;
        let document = Html::parse_document(html);
        let desc = extract_description(&document);
        assert_eq!(desc, Some("Test description".to_string()));
    }

    #[test]
    fn test_extract_description_og_preferred() {
        let html = r#"
            <html>
                <head>
                    <meta property="og:description" content="OG desc" />
                    <meta name="description" content="Regular desc" />
                </head>
            </html>
        "#;
        let document = Html::parse_document(html);
        assert_eq!(
            extract_description(&document),
            Some("OG desc".to_string())
        );
    }

    #[test]
    fn test_extract_description_none_when_missing() {
        let html = r#"<html><head></head></html>"#;
        let document = Html::parse_document(html);
        assert_eq!(extract_description(&document), None);
    }

    #[test]
    fn test_extract_description_empty_content() {
        let html = r#"<html><head><meta name="description" content="  " /></head></html>"#;
        let document = Html::parse_document(html);
        assert_eq!(extract_description(&document), None);
    }

    #[test]
    fn test_extract_favicon_candidates() {
        let base = Url::parse("https://example.com/page").unwrap();
        let html = r#"
            <html>
                <head>
                    <link rel="icon" href="/favicon.ico" />
                </head>
            </html>
        "#;
        let document = Html::parse_document(html);
        let candidates = extract_favicon(&document, &base);
        assert!(candidates.contains(&"https://example.com/favicon.ico".to_string()));
    }

    #[test]
    fn test_extract_favicon_fallback() {
        let base = Url::parse("https://example.com/page").unwrap();
        let html = r#"
            <html>
                <head></head>
            </html>
        "#;
        let document = Html::parse_document(html);
        let candidates = extract_favicon(&document, &base);
        assert_eq!(
            candidates,
            vec!["https://example.com/favicon.ico".to_string()]
        );
    }

    #[test]
    fn test_extract_favicon_relative_url() {
        let base = Url::parse("https://example.com/blog/post").unwrap();
        let html = r#"<html><head><link rel="icon" href="icon.png" /></head></html>"#;
        let document = Html::parse_document(html);
        let candidates = extract_favicon(&document, &base);
        assert!(candidates.contains(&"https://example.com/blog/icon.png".to_string()));
    }

    #[test]
    fn test_extract_favicon_absolute_url() {
        let base = Url::parse("https://example.com/page").unwrap();
        let html = r#"<html><head><link rel="icon" href="https://cdn.example.com/icon.png" /></head></html>"#;
        let document = Html::parse_document(html);
        let candidates = extract_favicon(&document, &base);
        assert!(candidates.contains(&"https://cdn.example.com/icon.png".to_string()));
    }

    #[test]
    fn test_extract_favicon_apple_touch_icon() {
        let base = Url::parse("https://example.com/").unwrap();
        let html = r#"<html><head><link rel="apple-touch-icon" href="/apple-icon.png" /></head></html>"#;
        let document = Html::parse_document(html);
        let candidates = extract_favicon(&document, &base);
        assert!(candidates.contains(&"https://example.com/apple-icon.png".to_string()));
    }

    #[test]
    fn test_extract_favicon_multiple_candidates() {
        let base = Url::parse("https://example.com/").unwrap();
        let html = r#"
            <html><head>
                <link rel="icon" href="/icon.svg" />
                <link rel="apple-touch-icon" href="/apple.png" />
            </head></html>
        "#;
        let document = Html::parse_document(html);
        let candidates = extract_favicon(&document, &base);
        // Should have declared icons + default fallback
        assert!(candidates.len() >= 3);
    }

    #[test]
    fn test_scraped_metadata_default() {
        let meta = ScrapedMetadata::default();
        assert!(meta.title.is_none());
        assert!(meta.description.is_none());
        assert!(meta.favicon.is_none());
    }

    #[test]
    fn test_valid_image_mime_types_coverage() {
        // Ensure common formats are in the list
        assert!(VALID_IMAGE_MIME_TYPES.contains(&"image/png"));
        assert!(VALID_IMAGE_MIME_TYPES.contains(&"image/svg+xml"));
        assert!(VALID_IMAGE_MIME_TYPES.contains(&"image/x-icon"));
        assert!(VALID_IMAGE_MIME_TYPES.contains(&"image/webp"));
    }

    #[test]
    fn test_valid_image_extensions_coverage() {
        assert!(VALID_IMAGE_EXTENSIONS.contains(&".ico"));
        assert!(VALID_IMAGE_EXTENSIONS.contains(&".svg"));
        assert!(VALID_IMAGE_EXTENSIONS.contains(&".png"));
    }
}
