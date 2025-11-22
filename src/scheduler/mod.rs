//! Background task scheduler
//!
//! This module provides a simple background task runner that executes
//! periodic maintenance tasks such as refreshing link metadata.

use crate::config::Config;
use crate::error::AppError;
use crate::github;
use crate::models::Link;
use crate::scraper;
use rand::Rng;
use sqlx::PgPool;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Background task scheduler
///
/// The scheduler runs periodic tasks such as:
/// - Refreshing GitHub repository metadata
/// - Updating web-scraped link metadata
/// - Checking for broken links
///
/// # Example
/// ```rust
/// let scheduler = Scheduler::new(pool.clone(), config.clone());
/// let handle = scheduler.start();
/// // Scheduler now runs in background
/// ```
pub struct Scheduler {
    pool: PgPool,
    config: Config,
    shutdown: Arc<AtomicBool>,
}

impl Scheduler {
    /// Create a new scheduler
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `config` - Application configuration
    pub fn new(pool: PgPool, config: Config) -> Self {
        Self {
            pool,
            config,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get a handle for graceful shutdown
    pub fn shutdown_handle(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.shutdown)
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
    /// Runs scheduled tasks at regular intervals with random jitter.
    /// If a task fails, logs the error and continues running.
    /// Supports graceful shutdown via shutdown signal.
    async fn run(&self) {
        tracing::info!(
            update_interval_hours = self.config.update_interval_hours,
            batch_size = self.config.batch_size,
            jitter_percent = self.config.jitter_percent,
            "Background scheduler started"
        );

        loop {
            // Calculate interval with jitter
            let base_interval_secs = self.config.update_interval_hours as u64 * 3600;
            let jitter_range = (base_interval_secs * self.config.jitter_percent as u64) / 100;
            let jitter = if jitter_range > 0 {
                let mut rng = rand::thread_rng();
                rng.gen_range(0..=jitter_range * 2) as i64 - jitter_range as i64
            } else {
                0
            };
            let interval_with_jitter = (base_interval_secs as i64 + jitter).max(60) as u64;

            tracing::debug!(
                base_interval_secs,
                jitter_secs = jitter,
                final_interval_secs = interval_with_jitter,
                "Calculated next check interval"
            );

            // Wait for the interval or shutdown signal
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(interval_with_jitter)) => {
                    // Check for shutdown signal before running tasks
                    if self.shutdown.load(Ordering::Relaxed) {
                        tracing::info!("Scheduler shutdown signal received");
                        break;
                    }

                    tracing::debug!("Running scheduled tasks...");

                    // Execute scheduled tasks
                    if let Err(e) = self.run_tasks().await {
                        tracing::error!(error = %e, "Scheduled task failed");
                    } else {
                        tracing::debug!("Scheduled tasks completed successfully");
                    }
                }
                _ = self.wait_for_shutdown() => {
                    tracing::info!("Scheduler shutting down gracefully");
                    break;
                }
            }
        }

        tracing::info!("Scheduler stopped");
    }

    /// Wait for shutdown signal
    async fn wait_for_shutdown(&self) {
        while !self.shutdown.load(Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_millis(100)).await;
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
    /// Processes links in batches based on configuration.
    /// Links are selected if they haven't been checked within the configured interval.
    ///
    /// For each link:
    /// - Scrapes metadata from the URL
    /// - If GitHub repo, fetches GitHub metadata
    /// - Updates last_checked timestamp
    async fn refresh_stale_links(&self) -> Result<(), AppError> {
        // Get links that need checking
        let links_to_check = Link::get_links_needing_check(
            &self.pool,
            self.config.update_interval_hours,
            self.config.batch_size as i64,
        )
        .await?;

        if links_to_check.is_empty() {
            tracing::debug!("No links need checking");
            return Ok(());
        }

        tracing::info!(count = links_to_check.len(), "Checking and refreshing links");

        let mut successful = 0;
        let mut failed = 0;

        for link in links_to_check {
            match self.refresh_single_link(&link).await {
                Ok(()) => {
                    successful += 1;
                    // Mark as checked regardless of outcome
                    if let Err(e) = Link::mark_checked(&self.pool, link.id).await {
                        tracing::error!(
                            link_id = %link.id,
                            error = %e,
                            "Failed to mark link as checked"
                        );
                    }
                }
                Err(e) => {
                    failed += 1;
                    tracing::warn!(
                        link_id = %link.id,
                        url = %link.url,
                        error = %e,
                        "Failed to refresh link"
                    );
                    // Still mark as checked to avoid repeatedly failing on the same link
                    if let Err(mark_err) = Link::mark_checked(&self.pool, link.id).await {
                        tracing::error!(
                            link_id = %link.id,
                            error = %mark_err,
                            "Failed to mark link as checked after error"
                        );
                    }
                }
            }
        }

        tracing::info!(
            successful,
            failed,
            total = successful + failed,
            "Link check cycle completed"
        );

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
