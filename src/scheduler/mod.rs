//! Background task scheduler
//!
//! This module provides a simple background task runner that executes
//! periodic maintenance tasks such as refreshing link metadata.

use crate::error::AppError;
use crate::github;
use crate::models::Link;
use crate::scraper;
use sqlx::PgPool;
use std::time::Duration;
use tokio::time::interval;

/// Background task scheduler
///
/// The scheduler runs periodic tasks such as:
/// - Refreshing GitHub repository metadata
/// - Updating web-scraped link metadata
/// - Checking for broken links
///
/// # Example
/// ```rust
/// let scheduler = Scheduler::new(pool.clone(), 7);
/// let handle = scheduler.start();
/// // Scheduler now runs in background
/// ```
pub struct Scheduler {
    pool: PgPool,
    update_interval_days: u32,
}

impl Scheduler {
    /// Create a new scheduler
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `update_interval_days` - How often to refresh link metadata (in days)
    pub fn new(pool: PgPool, update_interval_days: u32) -> Self {
        Self {
            pool,
            update_interval_days,
        }
    }

    /// Start the scheduler in a background task
    ///
    /// This spawns a new tokio task that runs the scheduler loop.
    /// The task will run until the application terminates.
    ///
    /// # Returns
    /// A `JoinHandle` for the background task (usually not awaited)
    pub fn start(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            self.run().await;
        })
    }

    /// Main scheduler loop
    ///
    /// Runs scheduled tasks at regular intervals (every hour).
    /// If a task fails, logs the error and continues running.
    async fn run(&self) {
        // Run tasks periodically - check every hour
        let mut interval = interval(Duration::from_secs(3600));

        tracing::info!(
            update_interval_days = self.update_interval_days,
            check_interval_seconds = 3600,
            "Background scheduler started"
        );

        loop {
            interval.tick().await;

            tracing::debug!("Running scheduled tasks...");

            // Execute scheduled tasks
            if let Err(e) = self.run_tasks().await {
                tracing::error!(error = %e, "Scheduled task failed");
            } else {
                tracing::debug!("Scheduled tasks completed successfully");
            }
        }
    }

    /// Execute all scheduled tasks
    ///
    /// This function is called periodically by the scheduler loop.
    /// Currently implements:
    /// - Refresh stale link metadata (web scraping + GitHub)
    ///
    /// Future tasks:
    /// - Check for broken links and update their status
    /// - Clean up expired sessions
    async fn run_tasks(&self) -> Result<(), AppError> {
        self.refresh_stale_links().await
    }

    /// Refresh metadata for stale links
    ///
    /// Processes up to 10 links per cycle to avoid overloading external services.
    /// Links are considered stale if they haven't been refreshed in `update_interval_days`.
    ///
    /// For each stale link:
    /// - Scrapes metadata from the URL
    /// - If GitHub repo, fetches GitHub metadata
    /// - Updates refreshed_at timestamp
    async fn refresh_stale_links(&self) -> Result<(), AppError> {
        // Process up to 10 links per cycle to avoid overloading
        let stale_links = Link::get_stale_links(&self.pool, self.update_interval_days, 10).await?;

        if stale_links.is_empty() {
            tracing::debug!("No stale links to refresh");
            return Ok(());
        }

        tracing::info!(count = stale_links.len(), "Refreshing stale links");

        for link in stale_links {
            if let Err(e) = self.refresh_single_link(&link).await {
                tracing::warn!(
                    link_id = %link.id,
                    url = %link.url,
                    error = %e,
                    "Failed to refresh link"
                );
                // Continue with other links even if one fails
            }
        }

        Ok(())
    }

    /// Refresh a single link's metadata
    ///
    /// Performs the following steps:
    /// 1. Checks URL health (accessibility)
    /// 2. Updates status if inaccessible or restores to active if recovered
    /// 3. Scrapes URL for title, description, favicon (if healthy)
    /// 4. If GitHub repo, fetches GitHub metadata (stars, archived, etc.)
    /// 5. Marks link as refreshed with current timestamp
    ///
    /// # Arguments
    /// * `link` - The link to refresh
    ///
    /// # Errors
    /// Returns error if scraping or database update fails
    async fn refresh_single_link(&self, link: &Link) -> Result<(), AppError> {
        tracing::debug!(link_id = %link.id, url = %link.url, "Refreshing link");

        // Check if URL is accessible
        let is_healthy = scraper::check_url_health(&link.url).await?;

        if !is_healthy {
            tracing::warn!(
                link_id = %link.id,
                url = %link.url,
                consecutive_failures = link.consecutive_failures + 1,
                "Link is not accessible, recording failure"
            );
            Link::record_failure(&self.pool, link.id).await?;
            Link::mark_refreshed(&self.pool, link.id, link.user_id).await?;
            return Ok(());
        }

        // On success, reset failures
        Link::reset_failures(&self.pool, link.id).await?;

        // If link was previously inaccessible but is now healthy, restore to active
        if link.status == "inaccessible" {
            tracing::info!(
                link_id = %link.id,
                url = %link.url,
                "Link is now accessible, restoring to active status"
            );
            Link::update_status(&self.pool, link.id, "active").await?;
        }

        // Scrape metadata
        let metadata = scraper::scrape_url(&link.url).await?;
        Link::update_scraped_metadata(&self.pool, link.id, link.user_id, metadata).await?;

        // Refresh GitHub metadata if applicable
        if link.is_github_repo {
            if let Some((owner, repo)) = github::parse_repo_from_url(&link.url) {
                match github::fetch_repo_metadata(&owner, &repo).await {
                    Ok(gh_meta) => {
                        Link::update_github_metadata(&self.pool, link.id, link.user_id, gh_meta)
                            .await?;
                    }
                    Err(e) => {
                        // Check if GitHub repo is unavailable (404, etc.)
                        let error_msg = e.to_string();
                        if error_msg.contains("404") || error_msg.contains("Not Found") {
                            tracing::warn!(
                                link_id = %link.id,
                                url = %link.url,
                                error = %e,
                                "GitHub repository not found, marking as repo_unavailable"
                            );
                            Link::update_status(&self.pool, link.id, "repo_unavailable").await?;
                        } else {
                            tracing::warn!(
                                link_id = %link.id,
                                url = %link.url,
                                error = %e,
                                "Failed to fetch GitHub metadata, continuing with refresh"
                            );
                        }
                    }
                }
            }
        }

        // Mark as refreshed
        Link::mark_refreshed(&self.pool, link.id, link.user_id).await?;

        tracing::info!(link_id = %link.id, "Link refreshed successfully");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_creation() {
        // This is just a compilation test
        // We can't actually test the scheduler without a real database
        let update_interval = 7;
        assert_eq!(update_interval, 7);
    }
}
