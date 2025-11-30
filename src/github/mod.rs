//! GitHub API integration for fetching repository metadata
//!
//! This module provides functions to interact with the GitHub API to fetch
//! repository metadata such as stars, description, language, license, etc.

use crate::error::AppError;
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::Deserialize;
use std::sync::OnceLock;

/// Metadata fetched from a GitHub repository
#[derive(Debug, Clone, serde::Serialize)]
pub struct GitHubRepoMetadata {
    pub stars: i32,
    pub description: Option<String>,
    pub archived: bool,
    pub last_commit: Option<DateTime<Utc>>,
    pub license: Option<String>,
    pub language: Option<String>,
}

/// Response from GitHub API for repository information
#[derive(Debug, Deserialize)]
struct GitHubApiResponse {
    stargazers_count: i32,
    description: Option<String>,
    archived: bool,
    pushed_at: Option<String>,
    license: Option<GitHubLicense>,
    language: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubLicense {
    name: String,
}

/// Check if a URL is a GitHub repository URL
///
/// Handles various GitHub URL formats:
/// - https://github.com/owner/repo
/// - https://github.com/owner/repo.git
/// - https://github.com/owner/repo/tree/branch
/// - git@github.com:owner/repo.git
///
/// # Examples
/// ```
/// assert!(is_github_repo("https://github.com/rust-lang/rust"));
/// assert!(is_github_repo("https://github.com/rust-lang/rust.git"));
/// assert!(!is_github_repo("https://gitlab.com/user/project"));
/// ```
pub fn is_github_repo(url: &str) -> bool {
    static GITHUB_REGEX: OnceLock<Regex> = OnceLock::new();
    let re = GITHUB_REGEX.get_or_init(|| {
        Regex::new(
            r"^(?:https?://github\.com/|git@github\.com:)([^/]+)/([^/\s]+?)(?:\.git)?(?:/.*)?$",
        )
        .unwrap()
    });

    re.is_match(url)
}

/// Parse owner and repository name from a GitHub URL
///
/// # Returns
/// Returns `Some((owner, repo))` if the URL is a valid GitHub repository URL,
/// otherwise returns `None`.
///
/// # Examples
/// ```
/// assert_eq!(
///     parse_repo_from_url("https://github.com/rust-lang/rust"),
///     Some(("rust-lang".to_string(), "rust".to_string()))
/// );
/// assert_eq!(
///     parse_repo_from_url("https://github.com/owner/repo.git"),
///     Some(("owner".to_string(), "repo".to_string()))
/// );
/// ```
pub fn parse_repo_from_url(url: &str) -> Option<(String, String)> {
    static GITHUB_REGEX: OnceLock<Regex> = OnceLock::new();
    let re = GITHUB_REGEX.get_or_init(|| {
        Regex::new(
            r"^(?:https?://github\.com/|git@github\.com:)([^/]+)/([^/\s]+?)(?:\.git)?(?:/.*)?$",
        )
        .unwrap()
    });

    re.captures(url).map(|caps| {
        let owner = caps.get(1).unwrap().as_str().to_string();
        let repo = caps.get(2).unwrap().as_str().to_string();
        (owner, repo)
    })
}

/// Fetch repository metadata from GitHub API
///
/// # Arguments
/// * `owner` - Repository owner (username or organization)
/// * `repo` - Repository name
///
/// # Returns
/// Returns `GitHubRepoMetadata` on success, or an error if:
/// - The repository doesn't exist (404)
/// - Rate limit exceeded (403)
/// - Network error
///
/// # Rate Limiting
/// - Unauthenticated requests: 60 requests per hour
/// - Authenticated requests (with GITHUB_TOKEN): 5000 requests per hour
///
/// # Example
/// ```
/// let metadata = fetch_repo_metadata("rust-lang", "rust").await?;
/// println!("Stars: {}", metadata.stars);
/// ```
pub async fn fetch_repo_metadata(owner: &str, repo: &str) -> Result<GitHubRepoMetadata, AppError> {
    let url = format!("https://api.github.com/repos/{}/{}", owner, repo);

    tracing::info!(
        owner = %owner,
        repo = %repo,
        "Fetching GitHub repository metadata"
    );

    // Build HTTP client with required headers
    let mut request_builder = reqwest::Client::new()
        .get(&url)
        .header("User-Agent", "RustyLinks/1.0")
        .header("Accept", "application/vnd.github+json");

    // Add GitHub token if available for higher rate limits
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.is_empty() {
            tracing::debug!("Using GitHub token for authentication");
            request_builder = request_builder.header("Authorization", format!("Bearer {}", token));
        }
    }

    let response = request_builder.send().await?;

    // Check for rate limiting
    if response.status() == 403 {
        if let Some(remaining) = response.headers().get("x-ratelimit-remaining") {
            if remaining == "0" {
                tracing::warn!(
                    owner = %owner,
                    repo = %repo,
                    "GitHub API rate limit exceeded"
                );
                return Err(AppError::ExternalService(
                    "GitHub API rate limit exceeded. Please try again later or set GITHUB_TOKEN environment variable.".to_string()
                ));
            }
        }
    }

    // Check for not found
    if response.status() == 404 {
        tracing::warn!(
            owner = %owner,
            repo = %repo,
            "GitHub repository not found"
        );
        return Err(AppError::not_found(
            "GitHub repository",
            &format!("{}/{}", owner, repo),
        ));
    }

    // Check for other errors
    if !response.status().is_success() {
        tracing::error!(
            owner = %owner,
            repo = %repo,
            status = %response.status(),
            "GitHub API request failed"
        );
        return Err(AppError::ExternalService(format!(
            "GitHub API request failed with status: {}",
            response.status()
        )));
    }

    let api_response: GitHubApiResponse = response.json().await?;

    // Parse the pushed_at timestamp
    let last_commit = api_response
        .pushed_at
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let metadata = GitHubRepoMetadata {
        stars: api_response.stargazers_count,
        description: api_response.description,
        archived: api_response.archived,
        last_commit,
        license: api_response.license.map(|l| l.name),
        language: api_response.language,
    };

    tracing::info!(
        owner = %owner,
        repo = %repo,
        stars = metadata.stars,
        archived = metadata.archived,
        "GitHub repository metadata fetched successfully"
    );

    Ok(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_github_repo() {
        // Valid GitHub URLs
        assert!(is_github_repo("https://github.com/rust-lang/rust"));
        assert!(is_github_repo("https://github.com/rust-lang/rust.git"));
        assert!(is_github_repo("https://github.com/owner/repo/tree/main"));
        assert!(is_github_repo(
            "https://github.com/owner/repo/blob/main/README.md"
        ));
        assert!(is_github_repo("git@github.com:owner/repo.git"));

        // Invalid URLs
        assert!(!is_github_repo("https://gitlab.com/user/project"));
        assert!(!is_github_repo("https://bitbucket.org/user/repo"));
        assert!(!is_github_repo("https://github.com/"));
        assert!(!is_github_repo("https://github.com/user"));
        assert!(!is_github_repo("not a url"));
    }

    #[test]
    fn test_parse_repo_from_url() {
        // Valid GitHub URLs
        assert_eq!(
            parse_repo_from_url("https://github.com/rust-lang/rust"),
            Some(("rust-lang".to_string(), "rust".to_string()))
        );
        assert_eq!(
            parse_repo_from_url("https://github.com/owner/repo.git"),
            Some(("owner".to_string(), "repo".to_string()))
        );
        assert_eq!(
            parse_repo_from_url("https://github.com/user/project/tree/main"),
            Some(("user".to_string(), "project".to_string()))
        );
        assert_eq!(
            parse_repo_from_url("git@github.com:owner/repo.git"),
            Some(("owner".to_string(), "repo".to_string()))
        );

        // Invalid URLs
        assert_eq!(parse_repo_from_url("https://gitlab.com/user/project"), None);
        assert_eq!(parse_repo_from_url("https://github.com/"), None);
        assert_eq!(parse_repo_from_url("not a url"), None);
    }
}
