# Part 7: Background Processing (Steps 32-35)

## Context
Part 6 is complete. We have:
- Web scraper for extracting page metadata
- GitHub API integration for repo metadata
- Manual refresh functionality for links
- Auto-population of metadata on link creation

---

## Step 32: Scheduler Setup

### Goal
Background task runner using tokio.

### Prompt

````text
Set up background task scheduling in `src/scheduler/mod.rs` for the Rusty Links application.

**Part 1: Implement in `src/scheduler/mod.rs`**

Create a simple background task runner:

```rust
use sqlx::PgPool;
use std::time::Duration;
use tokio::time::interval;

pub struct Scheduler {
    pool: PgPool,
    update_interval_days: u32,
}

impl Scheduler {
    pub fn new(pool: PgPool, update_interval_days: u32) -> Self {
        Self { pool, update_interval_days }
    }

    /// Start the scheduler in a background task
    pub fn start(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            self.run().await;
        })
    }

    async fn run(&self) {
        // Run tasks periodically
        let mut interval = interval(Duration::from_secs(3600)); // Check every hour

        loop {
            interval.tick().await;

            tracing::info!("Running scheduled tasks...");

            // Task execution will be added in subsequent steps
            if let Err(e) = self.run_tasks().await {
                tracing::error!(error = %e, "Scheduled task failed");
            }
        }
    }

    async fn run_tasks(&self) -> Result<(), crate::error::AppError> {
        // Placeholder - tasks will be added
        Ok(())
    }
}
```

**Part 2: Integrate with main.rs**

In `src/main.rs`:
1. After database initialization, create and start the scheduler
2. Pass the database pool and config.update_interval_days
3. The scheduler runs in the background (don't await its handle)

```rust
// After pool initialization
let scheduler = scheduler::Scheduler::new(pool.clone(), config.update_interval_days);
let _scheduler_handle = scheduler.start();
tracing::info!("Background scheduler started");
```

Export in `src/main.rs`: `mod scheduler;`
````

### Verification
- `cargo check` passes
- Scheduler starts on application launch (check logs)

---

## Step 33: Periodic Link Refresh

### Goal
Scheduled job to refresh link metadata based on UPDATE_INTERVAL_DAYS.

### Prompt

````text
Implement periodic link refresh in the scheduler for the Rusty Links application.

**Part 1: Add query to Link model**

In `src/models/link.rs`, add:

```rust
/// Get links that need refresh (not refreshed in the last N days)
pub async fn get_stale_links(
    pool: &PgPool,
    stale_days: u32,
    limit: i64
) -> Result<Vec<Link>, AppError> {
    let stale_threshold = Utc::now() - chrono::Duration::days(stale_days as i64);

    sqlx::query_as!(
        Link,
        r#"
        SELECT * FROM links
        WHERE status = 'active'
        AND (refreshed_at IS NULL OR refreshed_at < $1)
        ORDER BY refreshed_at ASC NULLS FIRST
        LIMIT $2
        "#,
        stale_threshold,
        limit
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}
```

**Part 2: Update Scheduler**

In `src/scheduler/mod.rs`:

```rust
async fn run_tasks(&self) -> Result<(), AppError> {
    self.refresh_stale_links().await
}

async fn refresh_stale_links(&self) -> Result<(), AppError> {
    // Process up to 10 links per cycle to avoid overloading
    let stale_links = Link::get_stale_links(
        &self.pool,
        self.update_interval_days,
        10
    ).await?;

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

async fn refresh_single_link(&self, link: &Link) -> Result<(), AppError> {
    tracing::debug!(link_id = %link.id, url = %link.url, "Refreshing link");

    // Scrape metadata
    let metadata = crate::scraper::scrape_url(&link.url).await?;
    Link::update_scraped_metadata(&self.pool, link.id, metadata).await?;

    // Refresh GitHub if applicable
    if link.is_github_repo {
        if let Some((owner, repo)) = crate::github::parse_repo_from_url(&link.url) {
            if let Ok(gh_meta) = crate::github::fetch_repo_metadata(&owner, &repo).await {
                Link::update_github_metadata(&self.pool, link.id, gh_meta).await?;
            }
        }
    }

    // Mark as refreshed
    Link::mark_refreshed(&self.pool, link.id).await?;

    Ok(())
}
```

**Part 3: Add imports**

Ensure scheduler has access to:
- `use crate::models::link::Link;`
- `use crate::scraper;`
- `use crate::github;`
````

### Verification
- `cargo check` passes
- Links are refreshed in background (check logs after waiting)

---

## Step 34: Link Health Checking

### Goal
Check if links are still accessible, update status.

### Prompt

````text
Add link health checking to the scheduler for the Rusty Links application.

**Part 1: Add health check function to scraper**

In `src/scraper/mod.rs`:

```rust
/// Check if a URL is accessible (returns HTTP 2xx or 3xx)
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
```

**Part 2: Add status update to Link model**

In `src/models/link.rs`:

```rust
/// Update link status
pub async fn update_status(
    pool: &PgPool,
    id: Uuid,
    status: &str
) -> Result<(), AppError> {
    sqlx::query!(
        "UPDATE links SET status = $1, updated_at = NOW() WHERE id = $2",
        status,
        id
    )
    .execute(pool)
    .await?;
    Ok(())
}
```

**Part 3: Update scheduler refresh**

In `src/scheduler/mod.rs`, update `refresh_single_link`:

```rust
async fn refresh_single_link(&self, link: &Link) -> Result<(), AppError> {
    tracing::debug!(link_id = %link.id, url = %link.url, "Checking link health");

    // First check if URL is accessible
    let is_healthy = crate::scraper::check_url_health(&link.url).await?;

    if !is_healthy {
        tracing::info!(
            link_id = %link.id,
            url = %link.url,
            "Link is inaccessible"
        );
        Link::update_status(&self.pool, link.id, "inaccessible").await?;
        Link::mark_refreshed(&self.pool, link.id).await?;
        return Ok(());
    }

    // If healthy and was inaccessible, mark as active again
    if link.status == "inaccessible" {
        Link::update_status(&self.pool, link.id, "active").await?;
    }

    // Continue with normal refresh...
    let metadata = crate::scraper::scrape_url(&link.url).await?;
    Link::update_scraped_metadata(&self.pool, link.id, metadata).await?;

    // GitHub refresh...
    if link.is_github_repo {
        // Check if repo is unavailable (404)
        if let Some((owner, repo)) = crate::github::parse_repo_from_url(&link.url) {
            match crate::github::fetch_repo_metadata(&owner, &repo).await {
                Ok(gh_meta) => {
                    Link::update_github_metadata(&self.pool, link.id, gh_meta).await?;
                    if link.status == "repo_unavailable" {
                        Link::update_status(&self.pool, link.id, "active").await?;
                    }
                }
                Err(_) => {
                    Link::update_status(&self.pool, link.id, "repo_unavailable").await?;
                }
            }
        }
    }

    Link::mark_refreshed(&self.pool, link.id).await?;
    Ok(())
}
```
````

### Verification
- `cargo check` passes
- Inaccessible links get status updated
- Status shown in UI reflects actual accessibility

---

## Step 35: Stale Link Detection

### Goal
Mark links as stale when unreachable.

### Prompt

````text
Add stale link detection and UI indicators for the Rusty Links application.

**Part 1: Track consecutive failures**

Add migration for failure tracking:

```sql
-- migrations/XXXXXX_add_failure_count.sql
ALTER TABLE links ADD COLUMN consecutive_failures INTEGER NOT NULL DEFAULT 0;
```

Run: `sqlx migrate add add_failure_count`

**Part 2: Update Link model**

In `src/models/link.rs`:

```rust
/// Increment failure count and potentially mark as inaccessible
pub async fn record_failure(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        UPDATE links
        SET consecutive_failures = consecutive_failures + 1,
            status = CASE
                WHEN consecutive_failures >= 2 THEN 'inaccessible'
                ELSE status
            END,
            updated_at = NOW()
        WHERE id = $1
        "#,
        id
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Reset failure count on successful access
pub async fn reset_failures(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    sqlx::query!(
        "UPDATE links SET consecutive_failures = 0, updated_at = NOW() WHERE id = $1",
        id
    )
    .execute(pool)
    .await?;
    Ok(())
}
```

**Part 3: Update scheduler to use failure tracking**

In `src/scheduler/mod.rs`, update health check logic:

```rust
if !is_healthy {
    Link::record_failure(&self.pool, link.id).await?;
    Link::mark_refreshed(&self.pool, link.id).await?;
    return Ok(());
}

// On success, reset failures
Link::reset_failures(&self.pool, link.id).await?;
if link.status == "inaccessible" {
    Link::update_status(&self.pool, link.id, "active").await?;
}
```

**Part 4: UI indicators for link status**

In `src/ui/pages/links.rs`:

1. Add visual indicators for link status:
   - Active: normal appearance (or green dot)
   - Archived: gray/muted appearance
   - Inaccessible: red warning icon, strike-through URL
   - Repo Unavailable: warning icon with "Repository unavailable" text

2. Add filter to show/hide by status:
   - Dropdown or checkboxes to filter by status
   - Default: show active only, or show all except inaccessible

3. Add "Mark as Active" button for inaccessible links:
   - Allows user to manually override status
   - Resets consecutive_failures to 0

Add CSS:
- `.link-status-active`, `.link-status-archived`, `.link-status-inaccessible`
- `.status-indicator` (small colored dot)
````

### Verification
- `cargo check` passes
- Links marked inaccessible after 3 consecutive failures
- UI shows status indicators
- Users can filter by status
