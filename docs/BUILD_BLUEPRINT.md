# Rusty Links Build Blueprint

## Project Overview
Rusty Links is a full-stack Rust web application for managing and organizing bookmarks/links with metadata enrichment. It uses Dioxus (fullstack) for the frontend, Axum for the backend API, and PostgreSQL for data storage.

---

## Part 1: Foundation (COMPLETE - Steps 1-3)
- Step 1: Project scaffolding, Cargo.toml, basic structure
- Step 2: Configuration management (.env, config.rs)
- Step 3: Error handling framework (AppError, conversions, HTTP responses)

## Part 2: Authentication (COMPLETE - Steps 4-9)
- Step 4: Database setup, migrations, connection pool
- Step 5: User model with Argon2 password hashing
- Step 6: Session management (create, validate, delete sessions)
- Step 7: Auth API endpoints (setup, login, logout, me, check-setup)
- Step 8: Auth middleware for protected routes
- Step 9: Auth UI pages (Setup, Login, basic routing)

---

## Part 3: Link Management Core (Steps 10-15)

### Step 10: Link Model
Create the Link model with database operations for CRUD.

### Step 11: Link API - Create & Read
Implement POST /api/links and GET /api/links endpoints.

### Step 12: Link API - Update & Delete
Implement PUT /api/links/:id and DELETE /api/links/:id endpoints.

### Step 13: Links UI - Display List
Build the Links page to display all user links in a list/grid.

### Step 14: Links UI - Create Form
Add a form to create new links with basic fields.

### Step 15: Links UI - Edit & Delete
Add edit and delete functionality to the Links UI.

---

## Part 4: Categorization System (Steps 16-21)

### Step 16: Category Model
Create Category model with hierarchical support (3-level depth).

### Step 17: Category API
CRUD endpoints for categories with parent relationship support.

### Step 18: Tag Model & API
Create Tag model and CRUD endpoints.

### Step 19: Link-Category Association
API endpoints to assign/remove categories from links.

### Step 20: Link-Tag Association
API endpoints to assign/remove tags from links.

### Step 21: Categories & Tags UI
UI components for managing and assigning categories/tags.

---

## Part 5: Metadata & Languages/Licenses (Steps 22-26)

### Step 22: Language & License Models
Models for programming languages and software licenses.

### Step 23: Language & License API
Endpoints for listing and managing languages/licenses.

### Step 24: Link-Language & Link-License Association
API endpoints to assign languages/licenses to links.

### Step 25: Metadata Display UI
Show categories, tags, languages, licenses on link cards.

### Step 26: Metadata Assignment UI
UI for assigning metadata to links (dropdowns, multi-select).

---

## Part 6: External Integrations (Steps 27-31)

### Step 27: Web Scraper - Basic
Scrape page title, description, and favicon from URLs.

### Step 28: Web Scraper - Integration
Auto-populate link metadata on creation.

### Step 29: GitHub API - Basic
Fetch repository metadata (stars, description, archived status).

### Step 30: GitHub Integration
Auto-detect GitHub URLs and enrich with repo data.

### Step 31: Link Refresh
Manual refresh button to update link metadata.

---

## Part 7: Background Processing (Steps 32-35)

### Step 32: Scheduler Setup
Background task runner using tokio.

### Step 33: Periodic Link Refresh
Scheduled job to refresh link metadata based on UPDATE_INTERVAL_DAYS.

### Step 34: Link Health Checking
Check if links are still accessible, update status.

### Step 35: Stale Link Detection
Mark links as stale when unreachable.

---

## Part 8: Search & Filtering (Steps 36-40)

### Step 36: Basic Search API
Search links by title, description, URL.

### Step 37: Filter by Category/Tag
API filtering by category and tag.

### Step 38: Filter by Language/License
API filtering by language and license.

### Step 39: Search UI
Search bar in Links page.

### Step 40: Filter UI
Filter dropdowns/checkboxes for all metadata types.

---

## Part 9: Polish & Advanced Features (Steps 41-45)

### Step 41: Sorting Options
Sort by date, title, stars, status.

### Step 42: Pagination
Paginate link results for performance.

### Step 43: Bulk Operations
Select multiple links for bulk delete/categorize.

### Step 44: Import/Export
Export links to JSON, import from JSON/bookmarks.

### Step 45: Final UI Polish
Loading states, error handling, responsive design.

---

## Implementation Principles

1. **Incremental Progress**: Each step builds on previous steps
2. **No Orphaned Code**: Every step ends with integration
3. **Test as You Go**: Verify each step works before proceeding
4. **Security First**: Validate inputs, sanitize outputs, check permissions
5. **Error Handling**: Use AppError consistently throughout
