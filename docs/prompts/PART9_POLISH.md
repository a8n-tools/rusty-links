# Part 9: Polish & Advanced Features (Steps 41-45)

## Context
Part 8 is complete. We have:
- Full-text search across title, description, URL
- Filtering by category, tag, language, license, status
- Filter UI with dropdowns and active filter badges
- Debounced search input

---

## Step 41: Sorting Options

### Goal
Sort by date, title, stars, status.

### Prompt

```text
Add sorting functionality to the links API and UI for the Rusty Links application.

**Part 1: Extend LinkSearchParams**

In `src/models/link.rs`:

```rust
#[derive(Debug, Deserialize, Default)]
pub struct LinkSearchParams {
    // ... existing fields ...
    pub sort_by: Option<String>,      // created_at, title, github_stars, status
    pub sort_order: Option<String>,   // asc, desc (default: desc)
}
```

**Part 2: Update search query with dynamic sorting**

```rust
pub async fn search(
    pool: &PgPool,
    user_id: Uuid,
    params: &LinkSearchParams,
) -> Result<Vec<Link>, AppError> {
    // Validate sort field to prevent SQL injection
    let sort_field = match params.sort_by.as_deref() {
        Some("title") => "LOWER(l.title)",
        Some("github_stars") => "l.github_stars",
        Some("status") => "l.status",
        Some("updated_at") => "l.updated_at",
        _ => "l.created_at",  // default
    };

    let sort_order = match params.sort_order.as_deref() {
        Some("asc") => "ASC",
        _ => "DESC",  // default
    };

    // Build query with dynamic ORDER BY
    // Note: Since we're building dynamic SQL, we need to use query_as with raw SQL
    // or use a query builder. For safety, use validated sort fields only.

    let order_clause = format!("ORDER BY {} {} NULLS LAST", sort_field, sort_order);

    // Execute query with order clause...
}
```

**Part 3: Add sort UI**

In `src/ui/pages/links.rs`:

```rust
let mut sort_by = use_signal(|| "created_at".to_string());
let mut sort_order = use_signal(|| "desc".to_string());

// Sort dropdown
div { class: "sort-container",
    label { "Sort by: " }
    select {
        value: "{sort_by}",
        onchange: move |evt| {
            sort_by.set(evt.value());
            fetch_links();
        },
        option { value: "created_at", "Date Added" }
        option { value: "updated_at", "Last Updated" }
        option { value: "title", "Title" }
        option { value: "github_stars", "GitHub Stars" }
        option { value: "status", "Status" }
    }

    button {
        class: "sort-order-btn",
        onclick: move |_| {
            let new_order = if sort_order() == "desc" { "asc" } else { "desc" };
            sort_order.set(new_order.to_string());
            fetch_links();
        },
        if sort_order() == "desc" { "↓" } else { "↑" }
    }
}
```

**Part 4: Update fetch to include sort params**

```rust
if sort_by() != "created_at" || sort_order() != "desc" {
    params.push(format!("sort_by={}", sort_by()));
    params.push(format!("sort_order={}", sort_order()));
}
```

Add CSS:
- `.sort-container` - sort controls container
- `.sort-order-btn` - ascending/descending toggle button
```

### Verification
- `cargo check` passes
- Can sort links by different fields
- Sort order toggles correctly

---

## Step 42: Pagination

### Goal
Paginate link results for performance.

### Prompt

```text
Add pagination to the links API and UI for the Rusty Links application.

**Part 1: Extend LinkSearchParams**

In `src/models/link.rs`:

```rust
#[derive(Debug, Deserialize, Default)]
pub struct LinkSearchParams {
    // ... existing fields ...
    pub page: Option<u32>,            // Page number (1-indexed)
    pub per_page: Option<u32>,        // Items per page (default: 20, max: 100)
}

#[derive(Debug, Serialize)]
pub struct PaginatedLinks {
    pub links: Vec<Link>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}
```

**Part 2: Update search with pagination**

```rust
pub async fn search_paginated(
    pool: &PgPool,
    user_id: Uuid,
    params: &LinkSearchParams,
) -> Result<PaginatedLinks, AppError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).min(100);
    let offset = ((page - 1) * per_page) as i64;

    // Get total count
    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(DISTINCT l.id) FROM links l ... WHERE ..."
    )
    .fetch_one(pool)
    .await?;

    // Get page of results
    let links = sqlx::query_as!(
        Link,
        r#"
        SELECT DISTINCT l.* FROM links l
        -- ... joins and filters ...
        ORDER BY ...
        LIMIT $X OFFSET $Y
        "#,
        // ... params ...,
        per_page as i64,
        offset,
    )
    .fetch_all(pool)
    .await?;

    let total_pages = ((total.0 as f64) / (per_page as f64)).ceil() as u32;

    Ok(PaginatedLinks {
        links,
        total: total.0,
        page,
        per_page,
        total_pages,
    })
}
```

**Part 3: Update API response**

In `src/api/links.rs`, change response to include pagination:

```rust
#[derive(Serialize)]
struct PaginatedResponse {
    links: Vec<LinkWithMetadata>,
    total: i64,
    page: u32,
    per_page: u32,
    total_pages: u32,
}

async fn list_links(...) -> Result<Json<PaginatedResponse>, AppError> {
    let result = Link::search_paginated(&pool, user.id, &params).await?;
    // Enrich with metadata...
    Ok(Json(PaginatedResponse { ... }))
}
```

**Part 4: Add pagination UI**

Create `src/ui/components/pagination.rs`:

```rust
#[component]
pub fn Pagination(
    current_page: u32,
    total_pages: u32,
    on_page_change: EventHandler<u32>,
) -> Element {
    rsx! {
        div { class: "pagination",
            // Previous button
            button {
                class: "pagination-btn",
                disabled: current_page <= 1,
                onclick: move |_| on_page_change.call(current_page - 1),
                "← Previous"
            }

            // Page numbers
            for page in get_page_range(current_page, total_pages) {
                if page == 0 {
                    span { class: "pagination-ellipsis", "..." }
                } else {
                    button {
                        class: if page == current_page { "pagination-btn active" } else { "pagination-btn" },
                        onclick: move |_| on_page_change.call(page),
                        "{page}"
                    }
                }
            }

            // Next button
            button {
                class: "pagination-btn",
                disabled: current_page >= total_pages,
                onclick: move |_| on_page_change.call(current_page + 1),
                "Next →"
            }
        }

        // Page info
        span { class: "pagination-info",
            "Page {current_page} of {total_pages}"
        }
    }
}

fn get_page_range(current: u32, total: u32) -> Vec<u32> {
    // Returns page numbers to display, with 0 for ellipsis
    // e.g., [1, 0, 4, 5, 6, 0, 10] for page 5 of 10
    // ...
}
```

**Part 5: Integrate pagination into Links page**

```rust
let mut current_page = use_signal(|| 1u32);
let mut total_pages = use_signal(|| 1u32);

// After links list
if total_pages() > 1 {
    Pagination {
        current_page: current_page(),
        total_pages: total_pages(),
        on_page_change: move |page| {
            current_page.set(page);
            fetch_links();
        },
    }
}
```

Add CSS:
- `.pagination` - pagination container
- `.pagination-btn`, `.pagination-btn.active` - page buttons
- `.pagination-ellipsis` - ellipsis between page ranges
- `.pagination-info` - "Page X of Y" text
```

### Verification
- `cargo check` passes
- Pagination works correctly
- Page changes trigger new data fetch

---

## Step 43: Bulk Operations

### Goal
Select multiple links for bulk delete/categorize.

### Prompt

```text
Add bulk operations to the Links page for the Rusty Links application.

**Part 1: Add bulk delete endpoint**

In `src/api/links.rs`:

```rust
#[derive(Deserialize)]
struct BulkDeleteRequest {
    link_ids: Vec<Uuid>,
}

/// DELETE /api/links/bulk
async fn bulk_delete(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Json(req): Json<BulkDeleteRequest>,
) -> Result<StatusCode, AppError> {
    let user = get_user_from_session(&pool, &jar).await?;

    // Verify all links belong to user and delete
    for link_id in req.link_ids {
        Link::delete(&pool, link_id, user.id).await?;
    }

    Ok(StatusCode::NO_CONTENT)
}
```

**Part 2: Add bulk category assignment endpoint**

```rust
#[derive(Deserialize)]
struct BulkCategoryRequest {
    link_ids: Vec<Uuid>,
    category_id: Uuid,
    action: String,  // "add" or "remove"
}

/// POST /api/links/bulk/categories
async fn bulk_category(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Json(req): Json<BulkCategoryRequest>,
) -> Result<StatusCode, AppError> {
    let user = get_user_from_session(&pool, &jar).await?;

    for link_id in req.link_ids {
        match req.action.as_str() {
            "add" => Link::add_category(&pool, link_id, req.category_id, user.id).await?,
            "remove" => Link::remove_category(&pool, link_id, req.category_id, user.id).await?,
            _ => return Err(AppError::validation("action", "Must be 'add' or 'remove'")),
        }
    }

    Ok(StatusCode::OK)
}
```

Add routes:
- `DELETE /api/links/bulk`
- `POST /api/links/bulk/categories`
- `POST /api/links/bulk/tags` (same pattern)

**Part 3: Add selection UI**

In `src/ui/pages/links.rs`:

```rust
let mut selected_ids = use_signal(|| HashSet::<Uuid>::new());
let mut selection_mode = use_signal(|| false);

// Selection toggle
button {
    class: "btn btn-secondary",
    onclick: move |_| {
        selection_mode.set(!selection_mode());
        if !selection_mode() {
            selected_ids.set(HashSet::new());
        }
    },
    if selection_mode() { "Cancel Selection" } else { "Select Multiple" }
}

// Select all checkbox (when in selection mode)
if selection_mode() {
    label {
        input {
            r#type: "checkbox",
            checked: selected_ids().len() == links().len(),
            onchange: move |evt| {
                if evt.checked() {
                    selected_ids.set(links().iter().map(|l| l.id).collect());
                } else {
                    selected_ids.set(HashSet::new());
                }
            },
        }
        " Select All"
    }
}

// In link card
if selection_mode() {
    input {
        r#type: "checkbox",
        checked: selected_ids().contains(&link.id),
        onchange: move |evt| {
            let mut ids = selected_ids();
            if evt.checked() {
                ids.insert(link.id);
            } else {
                ids.remove(&link.id);
            }
            selected_ids.set(ids);
        },
    }
}
```

**Part 4: Bulk action bar**

```rust
if selection_mode() && !selected_ids().is_empty() {
    div { class: "bulk-action-bar",
        span { "{selected_ids().len()} selected" }

        button {
            class: "btn btn-danger",
            onclick: move |_| bulk_delete_selected(),
            "Delete Selected"
        }

        // Category dropdown
        select {
            onchange: move |evt| {
                if !evt.value().is_empty() {
                    bulk_add_category(evt.value().parse().unwrap());
                }
            },
            option { value: "", "Add to category..." }
            for cat in categories() {
                option { value: "{cat.id}", "{cat.name}" }
            }
        }
    }
}
```

Add CSS:
- `.bulk-action-bar` - floating/sticky action bar
- `.link-card.selected` - highlight selected links
```

### Verification
- `cargo check` passes
- Can select multiple links
- Bulk delete works
- Bulk category assignment works

---

## Step 44: Import/Export

### Goal
Export links to JSON, import from JSON/bookmarks.

### Prompt

```text
Add import/export functionality for the Rusty Links application.

**Part 1: Export endpoint**

In `src/api/links.rs`:

```rust
#[derive(Serialize)]
struct ExportData {
    exported_at: DateTime<Utc>,
    version: String,
    links: Vec<ExportLink>,
    categories: Vec<Category>,
    tags: Vec<Tag>,
}

#[derive(Serialize)]
struct ExportLink {
    url: String,
    title: Option<String>,
    description: Option<String>,
    status: String,
    categories: Vec<String>,  // Names, not IDs
    tags: Vec<String>,
    languages: Vec<String>,
    licenses: Vec<String>,
    created_at: DateTime<Utc>,
}

/// GET /api/export
async fn export_links(
    State(pool): State<PgPool>,
    jar: CookieJar,
) -> Result<Json<ExportData>, AppError> {
    let user = get_user_from_session(&pool, &jar).await?;

    // Fetch all user data
    let links = Link::get_all_by_user(&pool, user.id).await?;
    let categories = Category::get_all_by_user(&pool, user.id).await?;
    let tags = Tag::get_all_by_user(&pool, user.id).await?;

    // Convert to export format with names instead of IDs
    // ...

    Ok(Json(ExportData {
        exported_at: Utc::now(),
        version: "1.0".to_string(),
        links: export_links,
        categories,
        tags,
    }))
}
```

**Part 2: Import endpoint**

```rust
#[derive(Deserialize)]
struct ImportData {
    links: Vec<ImportLink>,
}

#[derive(Deserialize)]
struct ImportLink {
    url: String,
    title: Option<String>,
    description: Option<String>,
    categories: Option<Vec<String>>,
    tags: Option<Vec<String>>,
}

#[derive(Serialize)]
struct ImportResult {
    imported: u32,
    skipped: u32,
    errors: Vec<String>,
}

/// POST /api/import
async fn import_links(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Json(data): Json<ImportData>,
) -> Result<Json<ImportResult>, AppError> {
    let user = get_user_from_session(&pool, &jar).await?;

    let mut imported = 0;
    let mut skipped = 0;
    let mut errors = vec![];

    for link_data in data.links {
        // Check if URL already exists
        if Link::exists_by_url(&pool, user.id, &link_data.url).await? {
            skipped += 1;
            continue;
        }

        // Create link
        match Link::create(&pool, user.id, CreateLink {
            url: link_data.url.clone(),
            title: link_data.title,
            description: link_data.description,
            ..Default::default()
        }).await {
            Ok(link) => {
                // Add categories/tags by name
                if let Some(cats) = link_data.categories {
                    for cat_name in cats {
                        if let Ok(cat) = Category::get_or_create_by_name(&pool, user.id, &cat_name).await {
                            let _ = Link::add_category(&pool, link.id, cat.id, user.id).await;
                        }
                    }
                }
                // Similar for tags...
                imported += 1;
            }
            Err(e) => {
                errors.push(format!("{}: {}", link_data.url, e));
            }
        }
    }

    Ok(Json(ImportResult { imported, skipped, errors }))
}
```

**Part 3: Add UI for export/import**

```rust
// Export button
button {
    class: "btn btn-secondary",
    onclick: move |_| {
        spawn(async move {
            let response = client.get("/api/export").send().await?;
            let data = response.text().await?;

            // Trigger download
            download_json("rusty-links-export.json", &data);
        });
    },
    "Export Links"
}

// Import button (file input)
input {
    r#type: "file",
    accept: ".json",
    onchange: move |evt| {
        // Read file and POST to /api/import
        // Show results modal with imported/skipped/errors
    },
}
```

**Part 4: Helper functions**

In Link model:
```rust
pub async fn exists_by_url(pool: &PgPool, user_id: Uuid, url: &str) -> Result<bool, AppError>
```

In Category model:
```rust
pub async fn get_or_create_by_name(pool: &PgPool, user_id: Uuid, name: &str) -> Result<Category, AppError>
```
```

### Verification
- `cargo check` passes
- Can export all links to JSON
- Can import links from JSON
- Duplicate URLs are skipped

---

## Step 45: Final UI Polish

### Goal
Loading states, error handling, responsive design.

### Prompt

```text
Add final polish to the Rusty Links UI.

**Part 1: Loading states**

Create `src/ui/components/loading.rs`:

```rust
#[component]
pub fn LoadingSpinner(size: Option<String>) -> Element {
    let size_class = size.unwrap_or("medium".to_string());
    rsx! {
        div { class: "spinner spinner-{size_class}" }
    }
}

#[component]
pub fn LoadingOverlay() -> Element {
    rsx! {
        div { class: "loading-overlay",
            LoadingSpinner { size: "large".to_string() }
        }
    }
}

#[component]
pub fn SkeletonCard() -> Element {
    rsx! {
        div { class: "skeleton-card",
            div { class: "skeleton skeleton-title" }
            div { class: "skeleton skeleton-text" }
            div { class: "skeleton skeleton-text short" }
        }
    }
}
```

Add to Links page:
- Show skeleton cards while initial load
- Show spinner overlays during operations
- Disable buttons during loading

**Part 2: Toast notifications**

Create `src/ui/components/toast.rs`:

```rust
#[derive(Clone)]
pub enum ToastType {
    Success,
    Error,
    Info,
}

#[derive(Clone)]
pub struct Toast {
    pub id: u32,
    pub message: String,
    pub toast_type: ToastType,
}

#[component]
pub fn ToastContainer(toasts: Signal<Vec<Toast>>) -> Element {
    rsx! {
        div { class: "toast-container",
            for toast in toasts() {
                div {
                    class: "toast toast-{toast.toast_type:?}",
                    key: "{toast.id}",
                    "{toast.message}"
                    button {
                        class: "toast-close",
                        onclick: move |_| remove_toast(toast.id),
                        "✕"
                    }
                }
            }
        }
    }
}
```

Use toasts for:
- Link created/updated/deleted
- Import results
- Refresh completed
- Error messages

**Part 3: Empty states**

Create meaningful empty states:

```rust
#[component]
pub fn EmptyState(
    icon: String,
    title: String,
    description: String,
    action: Option<Element>,
) -> Element {
    rsx! {
        div { class: "empty-state",
            span { class: "empty-icon", "{icon}" }
            h3 { "{title}" }
            p { "{description}" }
            {action}
        }
    }
}

// Usage
if links().is_empty() {
    EmptyState {
        icon: "🔗".to_string(),
        title: "No links yet".to_string(),
        description: "Add your first link to get started.".to_string(),
        action: rsx! {
            button {
                class: "btn btn-primary",
                onclick: move |_| show_form.set(true),
                "Add Link"
            }
        },
    }
}
```

**Part 4: Responsive design**

Update `assets/style.css` with responsive breakpoints:

```css
/* Mobile-first base styles */
.links-container {
    padding: 1rem;
}

.link-card {
    padding: 1rem;
}

/* Tablet and up */
@media (min-width: 768px) {
    .links-container {
        padding: 2rem;
    }

    .filter-panel {
        display: flex;
        flex-wrap: wrap;
        gap: 1rem;
    }

    .link-card {
        display: grid;
        grid-template-columns: 1fr auto;
    }
}

/* Desktop */
@media (min-width: 1024px) {
    .links-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
        gap: 1rem;
    }
}
```

**Part 5: Keyboard shortcuts**

Add keyboard navigation:
- `/` - Focus search
- `n` - New link
- `Escape` - Close modals/forms
- `Enter` - Submit forms

```rust
use_effect(move || {
    let handler = |evt: KeyboardEvent| {
        match evt.key().as_str() {
            "/" => {
                evt.prevent_default();
                focus_search();
            }
            "n" if !is_typing() => {
                show_form.set(true);
            }
            "Escape" => {
                show_form.set(false);
                editing_link_id.set(None);
            }
            _ => {}
        }
    };
    // Add event listener...
});
```

**Part 6: Final CSS polish**

Add to `assets/style.css`:
- Smooth transitions
- Focus states for accessibility
- Hover effects
- Dark mode support (optional)
- Print styles (optional)

```css
/* Transitions */
.btn, .link-card, .filter-badge {
    transition: all 0.2s ease;
}

/* Focus states */
:focus-visible {
    outline: 2px solid var(--primary-color);
    outline-offset: 2px;
}

/* Hover effects */
.link-card:hover {
    box-shadow: 0 4px 12px rgba(0,0,0,0.1);
    transform: translateY(-2px);
}
```
```

### Verification
- `cargo check` passes
- Loading states show appropriately
- Toast notifications work
- UI is responsive on mobile/tablet/desktop
- Keyboard shortcuts work
