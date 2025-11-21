# Part 6: External Integrations (Steps 27-31)

## Context
Part 5 is complete. We have:
- Language and license models with global/custom support
- Full metadata assignment (categories, tags, languages, licenses)
- UI for displaying and assigning all metadata types

---

## Step 27: Web Scraper - Basic

### Goal
Scrape page title, description, and favicon from URLs.

### Prompt

```text
Implement basic web scraping in `src/scraper/mod.rs` for the Rusty Links application.

Add `reqwest` with features for the scraper:
In Cargo.toml, update reqwest: `reqwest = { version = "0.11", features = ["json", "cookies"] }`
Add `scraper = "0.18"` for HTML parsing.

**Implement in `src/scraper/mod.rs`:**

1. `ScrapedMetadata` struct:
   ```rust
   pub struct ScrapedMetadata {
       pub title: Option<String>,
       pub description: Option<String>,
       pub favicon: Option<String>,
   }
   ```

2. `scrape_url(url: &str) -> Result<ScrapedMetadata, AppError>` function:
   - Make HTTP GET request to the URL
   - Set a reasonable User-Agent header
   - Set timeout (10 seconds)
   - Parse HTML response
   - Extract:
     - Title from `<title>` tag or `<meta property="og:title">`
     - Description from `<meta name="description">` or `<meta property="og:description">`
     - Favicon from `<link rel="icon">` or `<link rel="shortcut icon">` or default /favicon.ico
   - Handle errors gracefully (network errors, parsing errors)
   - Return ScrapedMetadata (fields are Option, so partial success is OK)

3. Handle edge cases:
   - Relative favicon URLs (convert to absolute)
   - Missing meta tags (return None for that field)
   - Non-HTML responses (return empty metadata)
   - Redirects (follow up to 5 redirects)

Export the module in `src/main.rs`: `mod scraper;`
````

### Verification
- `cargo check` passes
- Scraper module compiles

---

## Step 28: Web Scraper - Integration

### Goal
Auto-populate link metadata on creation.

### Prompt

```text
Integrate the web scraper with link creation in the Rusty Links application.

**Part 1: Add scrape endpoint**

Create `src/api/scrape.rs`:

1. `POST /api/scrape` - Scrape a URL for metadata
   - Body: `{ "url": "https://..." }`
   - Returns: `{ "title": "...", "description": "...", "favicon": "..." }`
   - Requires authentication
   - Rate limit consideration: add comment for future rate limiting

Wire into `src/api/mod.rs`: `.nest("/scrape", scrape::create_router(pool.clone()))`

**Part 2: Update link creation flow**

In `src/api/links.rs`, modify the `create_link` handler:
1. After validating the URL, call `scraper::scrape_url(&url)`
2. Use scraped data as defaults (don't overwrite user-provided values)
   - If user provided title, use it; otherwise use scraped title
   - Same for description
   - Always try to get favicon if not provided
3. Store favicon URL in the link's `logo` field

**Part 3: Update Link model**

In `src/models/link.rs`:
- Ensure `CreateLink` accepts optional `logo` field
- Update `create()` to store the logo

**Part 4: Update UI**

In `src/ui/pages/links.rs`:
1. When user enters URL and moves focus away (onblur), call `/api/scrape`
2. Auto-fill title and description if user hasn't modified them
3. Show loading indicator while scraping
4. Show scraped favicon preview if available
```

### Verification
- `cargo check` passes
- Creating a link auto-populates title/description from URL
- Favicon is stored and can be displayed

---

## Step 29: GitHub API - Basic

### Goal
Fetch repository metadata (stars, description, archived status).

### Prompt

```text
Implement GitHub API integration in `src/github/mod.rs` for the Rusty Links application.

**Part 1: Implement in `src/github/mod.rs`**

1. `GitHubRepoMetadata` struct:
   ```rust
   pub struct GitHubRepoMetadata {
       pub stars: i32,
       pub description: Option<String>,
       pub archived: bool,
       pub last_commit: Option<DateTime<Utc>>,
       pub license: Option<String>,
       pub language: Option<String>,
   }
   ```

2. `is_github_repo(url: &str) -> bool` function:
   - Check if URL matches github.com/{owner}/{repo} pattern
   - Handle various GitHub URL formats (with/without .git, with paths)

3. `parse_repo_from_url(url: &str) -> Option<(String, String)>` function:
   - Extract owner and repo name from GitHub URL
   - Return None if not a valid GitHub repo URL

4. `fetch_repo_metadata(owner: &str, repo: &str) -> Result<GitHubRepoMetadata, AppError>`:
   - Call GitHub API: `https://api.github.com/repos/{owner}/{repo}`
   - Parse JSON response
   - Extract stars (stargazers_count), description, archived, pushed_at
   - Handle rate limiting (return appropriate error)
   - Set User-Agent header (required by GitHub)

5. Optional: Support for GitHub token via environment variable
   - `GITHUB_TOKEN` env var for authenticated requests (higher rate limit)

Export in `src/main.rs`: `mod github;`
```

### Verification
- `cargo check` passes
- Can parse GitHub URLs correctly
- Can fetch repo metadata (test manually if needed)

---

## Step 30: GitHub Integration

### Goal
Auto-detect GitHub URLs and enrich with repo data.

### Prompt

```text
Integrate GitHub metadata with links in the Rusty Links application.

**Part 1: Update Link model**

In `src/models/link.rs`:
- Add function `update_github_metadata(pool, link_id, metadata: GitHubRepoMetadata) -> Result<(), AppError>`
  - Updates: is_github_repo, github_stars, github_archived, github_last_commit

**Part 2: Update link creation**

In `src/api/links.rs`, modify `create_link`:
1. After URL validation, check `github::is_github_repo(&url)`
2. If true:
   - Parse owner/repo from URL
   - Fetch GitHub metadata
   - Set `is_github_repo = true`
   - Store stars, archived, last_commit in the link
   - Use GitHub description if no description provided
3. If GitHub fetch fails, continue with link creation (non-blocking)

**Part 3: Create GitHub refresh endpoint**

Add to `src/api/links.rs`:
- `POST /api/links/:id/refresh-github` - Refresh GitHub metadata for a link
  - Only works for links where `is_github_repo = true`
  - Fetches latest metadata and updates the link
  - Returns updated link

**Part 4: Update UI display**

In link cards, for GitHub repos show:
- Star count with icon (⭐ 1,234)
- Archived badge if archived
- "GitHub" indicator badge
- Last commit date (optional, could be in details view)

Update `src/ui/pages/links.rs` to display GitHub-specific info.
```

### Verification
- `cargo check` passes
- Adding a GitHub URL populates stars/archived/etc
- GitHub repos show special metadata in UI

---

## Step 31: Link Refresh

### Goal
Manual refresh button to update link metadata.

### Prompt

```text
Add manual refresh functionality for links in the Rusty Links application.

**Part 1: Add refresh endpoint**

In `src/api/links.rs`:
- `POST /api/links/:id/refresh` - Refresh all metadata for a link
  1. Verify link belongs to user
  2. Re-scrape the URL for title/description/favicon
  3. If GitHub repo, also refresh GitHub metadata
  4. Update `refreshed_at` timestamp
  5. Return updated link with all metadata

**Part 2: Update Link model**

In `src/models/link.rs`:
- Add `update_scraped_metadata(pool, link_id, metadata: ScrapedMetadata) -> Result<(), AppError>`
- Add `mark_refreshed(pool, link_id) -> Result<(), AppError>` - Updates refreshed_at

**Part 3: Add refresh button to UI**

In `src/ui/pages/links.rs`:
1. Add a refresh button (🔄 icon) to each link card
2. On click:
   - Show loading spinner on the button
   - Call `POST /api/links/:id/refresh`
   - Update the link in the list with new data
   - Show success toast/notification (optional)
3. Handle errors (show error message if refresh fails)

**Part 4: Show last refresh time**

In link cards or detail view, show:
- "Last refreshed: 2 days ago" or similar
- Format using relative time (today, yesterday, X days ago)

Add CSS for refresh button: `.btn-refresh`
```

### Verification
- `cargo check` passes
- Can manually refresh any link
- Refreshed data appears in UI
- Last refresh time is displayed
