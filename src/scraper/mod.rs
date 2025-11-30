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

    // Parse HTML
    let document = Html::parse_document(&html);

    // Extract metadata
    let mut metadata = ScrapedMetadata::default();

    // Extract title
    metadata.title = extract_title(&document);

    // Extract description
    metadata.description = extract_description(&document);

    // Extract favicon
    metadata.favicon = extract_favicon(&document, &base_url);

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
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    match client.head(url).send().await {
        Ok(response) => {
            let status = response.status();
            Ok(status.is_success() || status.is_redirection())
        }
        Err(e) => {
            tracing::debug!(url = %url, error = %e, "Health check failed");
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

/// Extract favicon URL from HTML document
fn extract_favicon(document: &Html, base_url: &Url) -> Option<String> {
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
                        return Some(favicon_url.to_string());
                    }
                }
            }
        }
    }

    // Fallback to default /favicon.ico
    if let Ok(default_favicon) = base_url.join("/favicon.ico") {
        Some(default_favicon.to_string())
    } else {
        None
    }
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
        // og:title should be preferred
        assert_eq!(title, Some("OG Title".to_string()));
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
    fn test_absolute_favicon_url() {
        let base = Url::parse("https://example.com/page").unwrap();
        let html = r#"
            <html>
                <head>
                    <link rel="icon" href="/favicon.ico" />
                </head>
            </html>
        "#;
        let document = Html::parse_document(html);
        let favicon = extract_favicon(&document, &base);
        assert_eq!(favicon, Some("https://example.com/favicon.ico".to_string()));
    }
}
