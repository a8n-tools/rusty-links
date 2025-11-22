# Part 8: Search & Filtering (Steps 36-40)

## Context
Part 7 is complete. We have:
- Background scheduler running periodic tasks
- Automatic link refresh based on UPDATE_INTERVAL_DAYS
- Link health checking with status updates
- Consecutive failure tracking

---

## Step 36: Basic Search API

### Goal
Search links by title, description, URL.

### Prompt

````text
Add search functionality to the links API for the Rusty Links application.

**Part 1: Add search query to Link model**

In `src/models/link.rs`:

```rust
/// Search parameters for filtering links
#[derive(Debug, Deserialize, Default)]
pub struct LinkSearchParams {
    pub query: Option<String>,        // Text search
    pub status: Option<String>,       // Filter by status
    pub is_github: Option<bool>,      // Filter GitHub repos
}

/// Search links with text query
pub async fn search(
    pool: &PgPool,
    user_id: Uuid,
    params: &LinkSearchParams,
) -> Result<Vec<Link>, AppError> {
    let query_pattern = params.query
        .as_ref()
        .map(|q| format!("%{}%", q.to_lowercase()));

    sqlx::query_as!(
        Link,
        r#"
        SELECT * FROM links
        WHERE user_id = $1
        AND ($2::text IS NULL OR
            LOWER(title) LIKE $2 OR
            LOWER(description) LIKE $2 OR
            LOWER(url) LIKE $2 OR
            LOWER(domain) LIKE $2)
        AND ($3::text IS NULL OR status = $3)
        AND ($4::bool IS NULL OR is_github_repo = $4)
        ORDER BY created_at DESC
        "#,
        user_id,
        query_pattern,
        params.status,
        params.is_github,
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}
```

**Part 2: Update GET /api/links endpoint**

In `src/api/links.rs`:

```rust
use axum::extract::Query;

/// Handler for GET /api/links with search params
async fn list_links(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Query(params): Query<LinkSearchParams>,
) -> Result<Json<Vec<LinkWithMetadata>>, AppError> {
    let user = get_user_from_session(&pool, &jar).await?;

    let links = Link::search(&pool, user.id, &params).await?;

    // Enrich with metadata...
    let links_with_metadata = enrich_links_with_metadata(&pool, links, user.id).await?;

    Ok(Json(links_with_metadata))
}
```

**Part 3: API usage**

The endpoint now supports query parameters:
- `GET /api/links` - all links
- `GET /api/links?query=rust` - search for "rust"
- `GET /api/links?status=active` - only active links
- `GET /api/links?is_github=true` - only GitHub repos
- `GET /api/links?query=rust&status=active` - combined
````

### Verification
- `cargo check` passes
- Can search links via API with query parameter

---

## Step 37: Filter by Category/Tag

### Goal
API filtering by category and tag.

### Prompt

````text
Add category and tag filtering to the links API for the Rusty Links application.

**Part 1: Extend LinkSearchParams**

In `src/models/link.rs`:

```rust
#[derive(Debug, Deserialize, Default)]
pub struct LinkSearchParams {
    pub query: Option<String>,
    pub status: Option<String>,
    pub is_github: Option<bool>,
    pub category_id: Option<Uuid>,    // Filter by category
    pub tag_id: Option<Uuid>,         // Filter by tag
}
```

**Part 2: Update search query**

The query needs to join with junction tables when filtering:

```rust
pub async fn search(
    pool: &PgPool,
    user_id: Uuid,
    params: &LinkSearchParams,
) -> Result<Vec<Link>, AppError> {
    let query_pattern = params.query
        .as_ref()
        .map(|q| format!("%{}%", q.to_lowercase()));

    sqlx::query_as!(
        Link,
        r#"
        SELECT DISTINCT l.* FROM links l
        LEFT JOIN link_categories lc ON l.id = lc.link_id
        LEFT JOIN link_tags lt ON l.id = lt.link_id
        WHERE l.user_id = $1
        AND ($2::text IS NULL OR
            LOWER(l.title) LIKE $2 OR
            LOWER(l.description) LIKE $2 OR
            LOWER(l.url) LIKE $2 OR
            LOWER(l.domain) LIKE $2)
        AND ($3::text IS NULL OR l.status = $3)
        AND ($4::bool IS NULL OR l.is_github_repo = $4)
        AND ($5::uuid IS NULL OR lc.category_id = $5)
        AND ($6::uuid IS NULL OR lt.tag_id = $6)
        ORDER BY l.created_at DESC
        "#,
        user_id,
        query_pattern,
        params.status,
        params.is_github,
        params.category_id,
        params.tag_id,
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}
```

**Part 3: API usage**

Additional query parameters:
- `GET /api/links?category_id=uuid` - links in category
- `GET /api/links?tag_id=uuid` - links with tag
- `GET /api/links?query=rust&category_id=uuid` - combined search + filter
````

### Verification
- `cargo check` passes
- Can filter links by category and tag

---

## Step 38: Filter by Language/License

### Goal
API filtering by language and license.

### Prompt

````text
Add language and license filtering to the links API for the Rusty Links application.

**Part 1: Extend LinkSearchParams**

In `src/models/link.rs`:

```rust
#[derive(Debug, Deserialize, Default)]
pub struct LinkSearchParams {
    pub query: Option<String>,
    pub status: Option<String>,
    pub is_github: Option<bool>,
    pub category_id: Option<Uuid>,
    pub tag_id: Option<Uuid>,
    pub language_id: Option<Uuid>,    // Filter by language
    pub license_id: Option<Uuid>,     // Filter by license
}
```

**Part 2: Update search query**

Extend the query to include language and license joins:

```rust
pub async fn search(
    pool: &PgPool,
    user_id: Uuid,
    params: &LinkSearchParams,
) -> Result<Vec<Link>, AppError> {
    let query_pattern = params.query
        .as_ref()
        .map(|q| format!("%{}%", q.to_lowercase()));

    sqlx::query_as!(
        Link,
        r#"
        SELECT DISTINCT l.* FROM links l
        LEFT JOIN link_categories lc ON l.id = lc.link_id
        LEFT JOIN link_tags lt ON l.id = lt.link_id
        LEFT JOIN link_languages ll ON l.id = ll.link_id
        LEFT JOIN link_licenses lli ON l.id = lli.link_id
        WHERE l.user_id = $1
        AND ($2::text IS NULL OR
            LOWER(l.title) LIKE $2 OR
            LOWER(l.description) LIKE $2 OR
            LOWER(l.url) LIKE $2 OR
            LOWER(l.domain) LIKE $2)
        AND ($3::text IS NULL OR l.status = $3)
        AND ($4::bool IS NULL OR l.is_github_repo = $4)
        AND ($5::uuid IS NULL OR lc.category_id = $5)
        AND ($6::uuid IS NULL OR lt.tag_id = $6)
        AND ($7::uuid IS NULL OR ll.language_id = $7)
        AND ($8::uuid IS NULL OR lli.license_id = $8)
        ORDER BY l.created_at DESC
        "#,
        user_id,
        query_pattern,
        params.status,
        params.is_github,
        params.category_id,
        params.tag_id,
        params.language_id,
        params.license_id,
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}
```

**Part 3: API usage**

Full filtering capability:
- `GET /api/links?language_id=uuid` - links using language
- `GET /api/links?license_id=uuid` - links with license
- `GET /api/links?language_id=uuid&license_id=uuid` - combined
- `GET /api/links?query=rust&language_id=uuid&status=active` - full combo
````

### Verification
- `cargo check` passes
- Can filter by all metadata types

---

## Step 39: Search UI

### Goal
Search bar in Links page.

### Prompt

````text
Add a search bar to the Links page in `src/ui/pages/links.rs`.

**Part 1: Add search state**

Add signals for search:
```rust
let mut search_query = use_signal(|| String::new());
let mut search_debounce = use_signal(|| None::<tokio::task::JoinHandle<()>>);
```

**Part 2: Add search input**

At the top of the links container, add:
```rust
div { class: "search-container",
    input {
        class: "search-input",
        r#type: "text",
        placeholder: "Search links...",
        value: "{search_query}",
        oninput: move |evt| {
            search_query.set(evt.value());
            // Debounce search - wait 300ms after typing stops
            trigger_search();
        },
    }
    if !search_query().is_empty() {
        button {
            class: "search-clear",
            onclick: move |_| {
                search_query.set(String::new());
                trigger_search();
            },
            "✕"
        }
    }
}
```

**Part 3: Debounced search**

Implement debounced search to avoid excessive API calls:
```rust
let trigger_search = move || {
    // Cancel previous debounce timer
    if let Some(handle) = search_debounce.take() {
        handle.abort();
    }

    let query = search_query();
    spawn(async move {
        // Wait 300ms
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Fetch with search query
        fetch_links_with_params(&query, &filters).await;
    });
};
```

**Part 4: Update fetch function**

Create a generic fetch function that builds the query string:
```rust
async fn fetch_links(query: &str, filters: &Filters) {
    let mut url = "/api/links".to_string();
    let mut params = vec![];

    if !query.is_empty() {
        params.push(format!("query={}", urlencoding::encode(query)));
    }
    // Add other filter params...

    if !params.is_empty() {
        url = format!("{}?{}", url, params.join("&"));
    }

    // Make request and update links state...
}
```

**Part 5: Show search results feedback**

- Show "X results for 'query'" when searching
- Show "No results found" when search returns empty
- Highlight matching text in results (optional enhancement)

Add CSS:
- `.search-container` - container for search input
- `.search-input` - the search input field
- `.search-clear` - clear button
- `.search-results-info` - "X results" text
````

### Verification
- `cargo check` passes
- Search input works with debouncing
- Results update as user types

---

## Step 40: Filter UI

### Goal
Filter dropdowns/checkboxes for all metadata types.

### Prompt

````text
Add comprehensive filtering UI to the Links page in `src/ui/pages/links.rs`.

**Part 1: Create filter state**

```rust
#[derive(Default, Clone)]
struct Filters {
    status: Option<String>,
    category_id: Option<Uuid>,
    tag_id: Option<Uuid>,
    language_id: Option<Uuid>,
    license_id: Option<Uuid>,
    is_github: Option<bool>,
}

let mut filters = use_signal(Filters::default);
let mut show_filters = use_signal(|| false);  // Toggle filter panel
```

**Part 2: Create filter panel component**

Create `src/ui/components/filter_panel.rs`:

```rust
#[component]
pub fn FilterPanel(
    filters: Signal<Filters>,
    categories: Vec<Category>,
    tags: Vec<Tag>,
    languages: Vec<Language>,
    licenses: Vec<License>,
    on_change: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "filter-panel",
            // Status filter
            div { class: "filter-group",
                label { "Status" }
                select {
                    value: filters().status.unwrap_or_default(),
                    onchange: move |evt| {
                        let mut f = filters();
                        f.status = if evt.value().is_empty() { None } else { Some(evt.value()) };
                        filters.set(f);
                        on_change.call(());
                    },
                    option { value: "", "All statuses" }
                    option { value: "active", "Active" }
                    option { value: "archived", "Archived" }
                    option { value: "inaccessible", "Inaccessible" }
                }
            }

            // Category filter
            div { class: "filter-group",
                label { "Category" }
                select {
                    // Similar pattern...
                    option { value: "", "All categories" }
                    for cat in categories {
                        option { value: "{cat.id}", "{cat.name}" }
                    }
                }
            }

            // Tag filter - same pattern
            // Language filter - same pattern
            // License filter - same pattern

            // GitHub only checkbox
            div { class: "filter-group",
                label {
                    input {
                        r#type: "checkbox",
                        checked: filters().is_github.unwrap_or(false),
                        onchange: move |evt| {
                            let mut f = filters();
                            f.is_github = if evt.checked() { Some(true) } else { None };
                            filters.set(f);
                            on_change.call(());
                        },
                    }
                    " GitHub repos only"
                }
            }

            // Clear filters button
            button {
                class: "btn btn-secondary",
                onclick: move |_| {
                    filters.set(Filters::default());
                    on_change.call(());
                },
                "Clear filters"
            }
        }
    }
}
```

**Part 3: Integrate filter panel into Links page**

```rust
// Toggle button
button {
    class: "btn btn-secondary",
    onclick: move |_| show_filters.set(!show_filters()),
    if show_filters() { "Hide Filters" } else { "Show Filters" }
}

// Filter panel
if show_filters() {
    FilterPanel {
        filters: filters,
        categories: categories(),
        tags: tags(),
        languages: languages(),
        licenses: licenses(),
        on_change: move |_| fetch_links(),
    }
}

// Active filter badges
div { class: "active-filters",
    for badge in get_active_filter_badges(&filters()) {
        span { class: "filter-badge",
            "{badge.label}"
            button {
                onclick: move |_| clear_filter(badge.filter_type),
                "✕"
            }
        }
    }
}
```

**Part 4: Load filter options on mount**

Fetch categories, tags, languages, licenses when page loads:
```rust
use_effect(move || {
    spawn(async move {
        // Fetch in parallel
        let (cats, tags, langs, lics) = tokio::join!(
            fetch_categories(),
            fetch_tags(),
            fetch_languages(),
            fetch_licenses(),
        );
        categories.set(cats);
        tags.set(tags);
        languages.set(langs);
        licenses.set(lics);
    });
});
````

Add CSS:
- `.filter-panel` - collapsible filter container
- `.filter-group` - each filter dropdown/checkbox
- `.active-filters` - row of active filter badges
- `.filter-badge` - individual filter badge with remove button
```

### Verification
- `cargo check` passes
- Can filter by any combination of criteria
- Active filters shown as removable badges
- Clear all filters works
