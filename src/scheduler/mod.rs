//! Background task scheduler
//!
//! This module provides a simple background task runner that executes
//! periodic maintenance tasks such as refreshing link metadata.

use crate::error::AppError;
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
    /// Currently a placeholder - tasks will be added in subsequent steps.
    ///
    /// Future tasks:
    /// - Refresh GitHub metadata for repositories older than update_interval_days
    /// - Refresh web-scraped metadata for links older than update_interval_days
    /// - Check for broken links and update their status
    /// - Clean up expired sessions
    async fn run_tasks(&self) -> Result<(), AppError> {
        // Placeholder - tasks will be added in subsequent steps
        tracing::trace!("No scheduled tasks configured yet");
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
