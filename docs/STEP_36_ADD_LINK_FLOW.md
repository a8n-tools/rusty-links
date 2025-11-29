# Step 36: Add Link Flow - Implementation Prompts

## Overview

This document provides step-by-step implementation prompts for building the complete "Add Link Flow" feature in Rusty Links. The prompts are designed for code-generation LLMs and follow an incremental approach where each step builds on the previous one.

## Current State Analysis

### What Already Exists

The existing implementation includes:

1. **`AddLinkButton` component** (`src/ui/components/add_link_button.rs`)
   - Floating action button with plus icon
   - Clipboard URL detection on click
   - Opens `AddLinkDialog` modal
   - Handles duplicate detection (shows existing link in `LinkDetailsModal`)
   - Shows newly created link in `LinkDetailsModal` after success

2. **`AddLinkDialog` component** (`src/ui/components/modal/add_link_dialog.rs`)
   - URL input field with validation
   - Duplicate check via `/api/links/check-duplicate`
   - Link creation via `POST /api/links`
   - Error handling and loading states

3. **`LinkDetailsModal` component** (`src/ui/components/modal/link_details_modal.rs`)
   - Displays full link details after creation
   - Read-only display of extracted metadata (title, description, logo)
   - Editable fields (URL, source code URL, documentation URL, notes)
   - Categorization display (chips for categories, tags, languages, licenses)
   - **Placeholder text**: "Category editing coming in Step 36"
   - Save, Delete, Cancel buttons with unsaved changes warning
   - GitHub metadata section for GitHub repositories

4. **Selection Components** (already implemented)
   - `CategorySelect` - Multi-select dropdown with hierarchical categories
   - `TagSelect` - Multi-select with search and inline tag creation
   - `LanguageSelect` - Multi-select dropdown
   - `LicenseSelect` - Multi-select dropdown

5. **API Endpoints** (`src/api/links.rs`)
   - `POST /api/links` - Create link with metadata extraction
   - `PUT /api/links/{id}` - Update link (accepts category_ids, tag_ids, language_ids, license_ids)
   - `GET /api/links/{id}` - Get single link with all associations
   - `POST /api/links/{id}/refresh` - Refresh metadata

### What Needs to Be Implemented (Step 36)

The "Add Link Flow" enhancement focuses on:

1. **Replace placeholder chips with actual selection components** in `LinkDetailsModal`
2. **Wire up selection changes** to the form state
3. **Enable saving categorization** (categories, tags, languages, licenses)
4. **Improve the add link UX** with:
   - Better loading states during metadata extraction
   - Progress indication
   - Preview of extracted metadata before saving

---

## Implementation Prompts

Each prompt is designed to be self-contained and executable. Copy the entire prompt to your LLM.

---

### Step 36.1: Wire CategorySelect into LinkDetailsModal

````markdown
## Context

You are working on Rusty Links, a Dioxus 0.7 fullstack application with Tailwind CSS v4. The `LinkDetailsModal` component currently shows categories as read-only chips with the placeholder text "Category editing coming in Step 36".

## Files to Modify

- `src/ui/components/modal/link_details_modal.rs`

## Existing Components

The `CategorySelect` component already exists at `src/ui/components/category_select.rs`:

```rust
#[component]
pub fn CategorySelect(
    selected_ids: Vec<Uuid>,
    on_change: EventHandler<Vec<Uuid>>,
) -> Element
```

## Requirements

1. Import `CategorySelect` from `crate::ui::components::category_select`

2. In the "Categorization" section of `LinkDetailsModal`, replace the static category chips and placeholder text with the `CategorySelect` component

3. Wire the `CategorySelect` to:
   - Use `form_categories` signal as the selected IDs
   - Update `form_categories` and set `has_changes` to true on change

4. Remove the placeholder text "Category editing coming in Step 36" for categories

## Current Code to Replace

```rust
div { class: "form-group",
    label { "Categories" }
    div { class: "category-chips",
        for cat in link_data.categories.clone() {
            span {
                key: "{cat.id}",
                class: "chip",
                "{cat.name}"
            }
        }
    }
    p { class: "form-hint", "Category editing coming in Step 36" }
}
```

## New Code

Replace with:

```rust
div { class: "form-group",
    label { "Categories" }
    CategorySelect {
        selected_ids: form_categories(),
        on_change: move |ids| {
            form_categories.set(ids);
            has_changes.set(true);
        }
    }
}
```

## Test

1. Run `cargo check` to verify compilation
2. Open a link in the modal
3. Click the categories dropdown
4. Select/deselect categories
5. Verify "Save Changes" button becomes enabled
6. Save and verify categories persist
````

---

### Step 36.2: Wire TagSelect into LinkDetailsModal

````markdown
## Context

You are working on Rusty Links, a Dioxus 0.7 fullstack application. Following Step 36.1, you now need to wire up the `TagSelect` component in the `LinkDetailsModal`.

## Files to Modify

- `src/ui/components/modal/link_details_modal.rs`

## Existing Components

The `TagSelect` component exists at `src/ui/components/tag_select.rs`:

```rust
#[component]
pub fn TagSelect(
    selected_ids: Vec<Uuid>,
    on_change: EventHandler<Vec<Uuid>>,
) -> Element
```

It supports:
- Multi-select with checkboxes
- Search/filter functionality
- Inline tag creation (type name, click "+")

## Requirements

1. Import `TagSelect` from `crate::ui::components::tag_select`

2. In the "Categorization" section, replace the static tag chips and placeholder with `TagSelect`

3. Wire the component to use `form_tags` signal

4. Remove the placeholder text "Tag editing coming in Step 36"

## Current Code to Replace

```rust
div { class: "form-group",
    label { "Tags" }
    div { class: "tag-chips",
        for tag in link_data.tags.clone() {
            span {
                key: "{tag.id}",
                class: "chip",
                "{tag.name}"
            }
        }
    }
    p { class: "form-hint", "Tag editing coming in Step 36" }
}
```

## New Code

```rust
div { class: "form-group",
    label { "Tags" }
    TagSelect {
        selected_ids: form_tags(),
        on_change: move |ids| {
            form_tags.set(ids);
            has_changes.set(true);
        }
    }
}
```

## Test

1. Run `cargo check`
2. Open a link modal
3. Test tag selection and inline tag creation
4. Verify changes enable the Save button
5. Save and verify tags persist
````

---

### Step 36.3: Wire LanguageSelect into LinkDetailsModal

````markdown
## Context

You are working on Rusty Links, a Dioxus 0.7 fullstack application. Following Steps 36.1-36.2, you now need to wire up the `LanguageSelect` component.

## Files to Modify

- `src/ui/components/modal/link_details_modal.rs`

## Existing Components

The `LanguageSelect` component exists at `src/ui/components/language_select.rs` with the same interface pattern:

```rust
#[component]
pub fn LanguageSelect(
    selected_ids: Vec<Uuid>,
    on_change: EventHandler<Vec<Uuid>>,
) -> Element
```

## Requirements

1. Import `LanguageSelect` from `crate::ui::components::language_select`

2. Replace the static language chips and placeholder with `LanguageSelect`

3. Wire to `form_languages` signal

4. Remove placeholder text "Language editing coming in Step 36"

## Current Code to Replace

```rust
div { class: "form-group",
    label { "Languages" }
    div { class: "language-chips",
        for lang in link_data.languages.clone() {
            span {
                key: "{lang.id}",
                class: "chip",
                "{lang.name}"
            }
        }
    }
    p { class: "form-hint", "Language editing coming in Step 36" }
}
```

## New Code

```rust
div { class: "form-group",
    label { "Languages" }
    LanguageSelect {
        selected_ids: form_languages(),
        on_change: move |ids| {
            form_languages.set(ids);
            has_changes.set(true);
        }
    }
}
```

## Test

1. Run `cargo check`
2. Open link modal, test language selection
3. Verify Save button enables on change
4. Save and verify languages persist
````

---

### Step 36.4: Wire LicenseSelect into LinkDetailsModal

````markdown
## Context

You are working on Rusty Links, a Dioxus 0.7 fullstack application with Tailwind CSS v4. All CSS should be added to `./tailwind.css` in the root of the repo. DO NOT ADD CSS to `./assets/tailwind.css`.
This is the final categorization component to wire up.

## Files to Modify

- `src/ui/components/modal/link_details_modal.rs`

## Existing Components

The `LicenseSelect` component exists at `src/ui/components/license_select.rs`:

```rust
#[component]
pub fn LicenseSelect(
    selected_ids: Vec<Uuid>,
    on_change: EventHandler<Vec<Uuid>>,
) -> Element
```

## Requirements

1. Import `LicenseSelect` from `crate::ui::components::license_select`

2. Replace static license chips and placeholder with `LicenseSelect`

3. Wire to `form_licenses` signal

4. Remove placeholder text "License editing coming in Step 36"

## Current Code to Replace

```rust
div { class: "form-group",
    label { "Licenses" }
    div { class: "license-chips",
        for lic in link_data.licenses.clone() {
            span {
                key: "{lic.id}",
                class: "chip",
                "{lic.name}"
            }
        }
    }
    p { class: "form-hint", "License editing coming in Step 36" }
}
```

## New Code

```rust
div { class: "form-group",
    label { "Licenses" }
    LicenseSelect {
        selected_ids: form_licenses(),
        on_change: move |ids| {
            form_licenses.set(ids);
            has_changes.set(true);
        }
    }
}
```

## Test

1. Run `cargo check`
2. Open link modal, test license selection
3. Verify Save button enables on change
4. Save and verify licenses persist
5. Test that all four selectors work together
````

---

### Step 36.5: Add Metadata Preview to AddLinkDialog

````markdown
## Context

You are working on Rusty Links, a Dioxus 0.7 fullstack application with Tailwind CSS v4. The `AddLinkDialog` currently only shows a URL input and creates the link immediately. We want to improve the UX by showing a preview of extracted metadata before creating the link.

## Files to Modify

- `src/ui/components/modal/add_link_dialog.rs`
- `src/ui/api_client.rs`

## Current Flow

1. User enters URL
2. User clicks "Add Link"
3. System checks for duplicates
4. If no duplicate, creates link with metadata extraction
5. Shows newly created link in `LinkDetailsModal`

## New Flow

1. User enters URL
2. User clicks "Preview" (renamed from "Add Link")
3. System checks for duplicates
4. If no duplicate, fetches metadata preview (without creating link)
5. Shows preview: title, description, favicon, GitHub info if applicable
6. User clicks "Save Link" to create
7. Shows newly created link in `LinkDetailsModal`

## Requirements

### 1. Add Metadata Preview API Endpoint

In `src/api/links.rs`, add a new endpoint:

```rust
/// POST /api/links/preview
///
/// Preview metadata for a URL without creating the link.
/// Returns extracted title, description, favicon, and GitHub info if applicable.
#[derive(Debug, Deserialize)]
struct PreviewRequest {
    url: String,
}

#[derive(Debug, Serialize)]
struct PreviewResponse {
    url: String,
    domain: String,
    title: Option<String>,
    description: Option<String>,
    favicon: Option<String>,
    is_github_repo: bool,
    github_stars: Option<i32>,
    github_description: Option<String>,
    github_languages: Vec<String>,
    github_license: Option<String>,
}

async fn preview_link_handler(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Json(request): Json<PreviewRequest>,
) -> Result<Json<PreviewResponse>, AppError> {
    // Validate session
    let user = validate_session(&pool, &jar).await?;

    // Extract domain
    let domain = extract_domain(&request.url)?;

    // Check if GitHub repo
    let is_github = crate::github::is_github_repo(&request.url);

    let mut response = PreviewResponse {
        url: request.url.clone(),
        domain,
        title: None,
        description: None,
        favicon: None,
        is_github_repo: is_github,
        github_stars: None,
        github_description: None,
        github_languages: vec![],
        github_license: None,
    };

    if is_github {
        // Fetch GitHub metadata
        if let Some((owner, repo)) = crate::github::parse_repo_from_url(&request.url) {
            if let Ok(metadata) = crate::github::fetch_repo_metadata(&owner, &repo).await {
                response.title = Some(format!("{}/{}", owner, repo));
                response.description = metadata.description.clone();
                response.github_stars = Some(metadata.stars);
                response.github_description = metadata.description;
                response.github_languages = metadata.languages.clone();
                response.github_license = metadata.license.clone();
            }
        }
    } else {
        // Regular web scraping
        if let Ok(metadata) = crate::scraper::scrape_url(&request.url).await {
            response.title = metadata.title;
            response.description = metadata.description;
            response.favicon = metadata.favicon;
        }
    }

    Ok(Json(response))
}
```

Add route: `.route("/preview", post(preview_link_handler))`

### 2. Add API Client Function

In `src/ui/api_client.rs`:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct LinkPreview {
    pub url: String,
    pub domain: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub favicon: Option<String>,
    pub is_github_repo: bool,
    pub github_stars: Option<i32>,
    pub github_description: Option<String>,
    pub github_languages: Vec<String>,
    pub github_license: Option<String>,
}

/// Fetch metadata preview for a URL
pub async fn preview_link(url: &str) -> Result<LinkPreview, String> {
    let body = serde_json::json!({ "url": url });
    http::post("/api/links/preview", &body).await
}
```

### 3. Update AddLinkDialog Component

Modify `src/ui/components/modal/add_link_dialog.rs`:

1. Add state for preview:
```rust
let mut preview = use_signal(|| Option::<LinkPreview>::None);
let mut previewing = use_signal(|| false);
```

2. Add preview step in the flow:
- First button: "Preview" → fetches metadata, shows preview
- After preview: "Save Link" → creates the link
- Add "Back" button to return to URL input

3. Show preview section with:
- Favicon (if available)
- Title
- Description
- GitHub info badge (if GitHub repo)
- GitHub stars
- Detected languages

## CSS Classes (Tailwind v4)

Add these styles to preview section:

```css
.preview-section {
    @apply bg-gray-50 rounded-lg p-4 mt-4;
}

.preview-header {
    @apply flex items-center gap-3 mb-3;
}

.preview-favicon {
    @apply w-8 h-8 rounded;
}

.preview-title {
    @apply text-lg font-semibold text-gray-900;
}

.preview-description {
    @apply text-sm text-gray-600 mb-3;
}

.preview-meta {
    @apply flex flex-wrap gap-2;
}

.preview-badge {
    @apply px-2 py-1 text-xs font-medium rounded-full;
}

.preview-badge-github {
    @apply bg-gray-900 text-white;
}

.preview-badge-language {
    @apply bg-blue-100 text-blue-800;
}
```

## Test

1. Run `cargo check`
2. Click "Add Link" button
3. Enter a URL (try both GitHub and non-GitHub URLs)
4. Click "Preview" - verify metadata appears
5. Click "Save Link" - verify link is created
6. Verify the `LinkDetailsModal` shows with the new link
````

---

### Step 36.6: Add Loading Progress Indicator

````markdown
## Context

You are working on Rusty Links. The metadata extraction can take several seconds, especially for GitHub repositories. We need better loading feedback.

## Files to Modify

- `src/ui/components/modal/add_link_dialog.rs`
- `tailwind.css`

## Requirements

### 1. Add Progress Steps UI

When previewing/creating, show progress steps:

```rust
#[derive(Clone, PartialEq)]
enum ProgressStep {
    ValidatingUrl,
    CheckingDuplicates,
    FetchingMetadata,
    CreatingLink,
    Complete,
}

fn step_label(step: &ProgressStep) -> &'static str {
    match step {
        ProgressStep::ValidatingUrl => "Validating URL...",
        ProgressStep::CheckingDuplicates => "Checking for duplicates...",
        ProgressStep::FetchingMetadata => "Fetching metadata...",
        ProgressStep::CreatingLink => "Creating link...",
        ProgressStep::Complete => "Complete!",
    }
}
```

### 2. Progress Component

```rust
#[component]
fn ProgressIndicator(current_step: ProgressStep) -> Element {
    let steps = [
        ProgressStep::ValidatingUrl,
        ProgressStep::CheckingDuplicates,
        ProgressStep::FetchingMetadata,
        ProgressStep::CreatingLink,
    ];

    rsx! {
        div { class: "progress-steps",
            for (i, step) in steps.iter().enumerate() {
                {
                    let is_current = *step == current_step;
                    let is_complete = steps.iter().position(|s| s == &current_step).unwrap_or(0) > i;
                    rsx! {
                        div {
                            class: "progress-step",
                            class: if is_current { "progress-step-active" },
                            class: if is_complete { "progress-step-complete" },
                            div { class: "progress-step-indicator",
                                if is_complete {
                                    "✓"
                                } else {
                                    "{i + 1}"
                                }
                            }
                            span { class: "progress-step-label", "{step_label(step)}" }
                        }
                    }
                }
            }
        }
    }
}
```

### 3. CSS Styles

```css
.progress-steps {
    @apply flex flex-col gap-2 my-4;
}

.progress-step {
    @apply flex items-center gap-3 text-gray-400;
}

.progress-step-active {
    @apply text-orange-600 font-medium;
}

.progress-step-active .progress-step-indicator {
    @apply bg-orange-600 text-white animate-pulse;
}

.progress-step-complete {
    @apply text-green-600;
}

.progress-step-complete .progress-step-indicator {
    @apply bg-green-600 text-white;
}

.progress-step-indicator {
    @apply w-6 h-6 rounded-full bg-gray-200 flex items-center justify-center text-xs font-bold;
}

.progress-step-label {
    @apply text-sm;
}
```

### 4. Wire Into Flow

Update the async handler to update progress:

```rust
let handle_preview = move |_| {
    // ... validation ...

    spawn(async move {
        progress_step.set(Some(ProgressStep::ValidatingUrl));
        // URL validation...

        progress_step.set(Some(ProgressStep::CheckingDuplicates));
        match check_duplicate_url(&url).await {
            // ...
        }

        progress_step.set(Some(ProgressStep::FetchingMetadata));
        match preview_link(&url).await {
            // ...
        }

        progress_step.set(None);
    });
};
```

## Test

1. Run `cargo check`
2. Add a new link
3. Verify progress steps appear and animate
4. Verify each step updates in sequence
5. Test with slow network (throttle in dev tools)
````

---

### Step 36.7: Add Quick Categorization to AddLinkDialog

````markdown
## Context

You are working on Rusty Links. After showing the metadata preview, users should be able to set initial categories, tags, languages, and licenses before creating the link.

## Files to Modify

- `src/ui/components/modal/add_link_dialog.rs`
- `src/ui/api_client.rs`
- `src/api/links.rs`

## Requirements

### 1. Update CreateLinkRequest

Modify the API to accept initial categorization:

```rust
#[derive(Debug, Deserialize)]
pub struct CreateLinkRequest {
    pub url: String,
    #[serde(default)]
    pub category_ids: Vec<Uuid>,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
    #[serde(default)]
    pub language_ids: Vec<Uuid>,
    #[serde(default)]
    pub license_ids: Vec<Uuid>,
}
```

Update `create_link_handler` to save associations after creating the link:

```rust
// After creating the link...
let link = Link::create(&pool, user.id, request.url, ...).await?;

// Add initial categorization
if !request.category_ids.is_empty() {
    Link::set_categories(&pool, link.id, user.id, &request.category_ids).await?;
}
if !request.tag_ids.is_empty() {
    Link::set_tags(&pool, link.id, user.id, &request.tag_ids).await?;
}
// ... same for languages and licenses
```

### 2. Add Selection State to AddLinkDialog

```rust
// In AddLinkDialog component
let mut selected_categories = use_signal(|| Vec::<Uuid>::new());
let mut selected_tags = use_signal(|| Vec::<Uuid>::new());
let mut selected_languages = use_signal(|| Vec::<Uuid>::new());
let mut selected_licenses = use_signal(|| Vec::<Uuid>::new());
```

### 3. Add Categorization Section to Preview

After the preview section, add collapsible categorization:

```rust
// Inside preview view, after metadata preview
details { class: "categorization-accordion",
    summary { class: "categorization-summary",
        "Add Categories (optional)"
        span { class: "categorization-count",
            if !selected_categories().is_empty() || !selected_tags().is_empty() {
                {
                    let count = selected_categories().len()
                        + selected_tags().len()
                        + selected_languages().len()
                        + selected_licenses().len();
                    rsx! { " ({count} selected)" }
                }
            }
        }
    }

    div { class: "categorization-content",
        div { class: "form-group",
            label { "Categories" }
            CategorySelect {
                selected_ids: selected_categories(),
                on_change: move |ids| selected_categories.set(ids)
            }
        }

        div { class: "form-group",
            label { "Tags" }
            TagSelect {
                selected_ids: selected_tags(),
                on_change: move |ids| selected_tags.set(ids)
            }
        }

        div { class: "form-group",
            label { "Languages" }
            LanguageSelect {
                selected_ids: selected_languages(),
                on_change: move |ids| selected_languages.set(ids)
            }
        }

        div { class: "form-group",
            label { "Licenses" }
            LicenseSelect {
                selected_ids: selected_licenses(),
                on_change: move |ids| selected_licenses.set(ids)
            }
        }
    }
}
```

### 4. Update Create Link Call

```rust
let handle_create = move |_| {
    spawn(async move {
        creating.set(true);

        let request = CreateLinkWithCategories {
            url: url_input(),
            category_ids: selected_categories(),
            tag_ids: selected_tags(),
            language_ids: selected_languages(),
            license_ids: selected_licenses(),
        };

        match create_link_with_categories(&request).await {
            Ok(link) => on_success.call(link),
            Err(e) => error.set(Some(e)),
        }

        creating.set(false);
    });
};
```

### 5. CSS Styles

```css
.categorization-accordion {
    @apply border border-gray-200 rounded-lg mt-4;
}

.categorization-summary {
    @apply px-4 py-3 bg-gray-50 cursor-pointer font-medium text-gray-700 rounded-lg;
    @apply hover:bg-gray-100 transition-colors;
    @apply flex justify-between items-center;
}

.categorization-summary::-webkit-details-marker {
    display: none;
}

.categorization-summary::before {
    content: "▶";
    @apply mr-2 text-xs text-gray-400 transition-transform;
}

details[open] .categorization-summary::before {
    transform: rotate(90deg);
}

.categorization-count {
    @apply text-sm text-orange-600;
}

.categorization-content {
    @apply p-4 space-y-4 border-t border-gray-200;
}
```

## Test

1. Run `cargo check`
2. Add a new link, get to preview
3. Expand "Add Categories" section
4. Select categories, tags, languages, licenses
5. Click "Save Link"
6. Verify link is created with all associations
7. Open link details modal to confirm
````

---

### Step 36.8: Auto-Suggest Categories from GitHub Languages

````markdown
## Context

You are working on Rusty Links. When adding a GitHub repository, the system detects programming languages. We should auto-suggest matching languages from the database.

## Files to Modify

- `src/ui/components/modal/add_link_dialog.rs`

## Requirements

### 1. Match GitHub Languages to Database Languages

When preview shows GitHub languages, automatically select matching languages:

```rust
// After preview is loaded
use_effect(move || {
    if let Some(preview_data) = preview() {
        if !preview_data.github_languages.is_empty() {
            // Fetch available languages and match
            spawn(async move {
                if let Ok(languages) = fetch_languages().await {
                    let matched_ids: Vec<Uuid> = languages
                        .iter()
                        .filter(|lang| {
                            preview_data.github_languages.iter().any(|gh_lang| {
                                lang.name.to_lowercase() == gh_lang.to_lowercase()
                            })
                        })
                        .map(|lang| Uuid::parse_str(&lang.id).unwrap())
                        .collect();

                    if !matched_ids.is_empty() {
                        selected_languages.set(matched_ids);
                    }
                }
            });
        }

        // Also auto-select license if detected
        if let Some(gh_license) = &preview_data.github_license {
            spawn(async move {
                if let Ok(licenses) = fetch_licenses().await {
                    if let Some(matched) = licenses.iter().find(|lic| {
                        lic.name.to_lowercase() == gh_license.to_lowercase()
                            || lic.acronym.as_ref().map(|a| a.to_lowercase()) == Some(gh_license.to_lowercase())
                    }) {
                        if let Ok(id) = Uuid::parse_str(&matched.id) {
                            selected_licenses.set(vec![id]);
                        }
                    }
                }
            });
        }
    }
});
```

### 2. Show Auto-Suggested Badge

When languages/licenses are auto-suggested, show an indicator:

```rust
div { class: "form-group",
    label {
        "Languages"
        if auto_suggested_languages() {
            span { class: "auto-suggested-badge", "Auto-detected" }
        }
    }
    LanguageSelect { /* ... */ }
}
```

### 3. CSS for Auto-Suggested Badge

```css
.auto-suggested-badge {
    @apply ml-2 px-2 py-0.5 text-xs font-medium bg-green-100 text-green-700 rounded-full;
}
```

## Test

1. Run `cargo check`
2. Add a GitHub repository URL (e.g., https://github.com/rust-lang/rust)
3. Verify languages are auto-selected after preview
4. Verify "Auto-detected" badge appears
5. Verify license is auto-selected if detected
6. Verify user can still modify selections
````

---

### Step 36.9: Add Keyboard Navigation

````markdown
## Context

You are working on Rusty Links. The Add Link dialog should support keyboard navigation for accessibility.

## Files to Modify

- `src/ui/components/modal/add_link_dialog.rs`

## Requirements

### 1. Keyboard Shortcuts

- `Enter` in URL field: Trigger preview (if URL is valid)
- `Ctrl/Cmd + Enter`: Save link (when preview is shown)
- `Escape`: Close dialog (with unsaved changes warning if applicable)
- `Tab`: Navigate through form fields

### 2. Add Event Handler

```rust
let handle_keydown = move |evt: KeyboardEvent| {
    match evt.key() {
        Key::Escape => {
            on_close.call(());
        }
        Key::Enter if evt.modifiers().ctrl() || evt.modifiers().meta() => {
            if preview().is_some() && !creating() {
                handle_create(());
            }
        }
        _ => {}
    }
};
```

### 3. Wire to Modal

```rust
ModalBase {
    on_close: on_close,

    div {
        class: "add-link-dialog",
        onkeydown: handle_keydown,
        tabindex: "0",
        // ... rest of dialog
    }
}
```

### 4. Focus Management

Auto-focus URL input on open:

```rust
use_effect(move || {
    // Focus URL input on mount
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(input) = document.get_element_by_id("url-input") {
                    let _ = input.dyn_ref::<web_sys::HtmlElement>().map(|e| e.focus());
                }
            }
        }
    }
});
```

Add ID to URL input:

```rust
input {
    id: "url-input",
    r#type: "url",
    // ...
}
```

### 5. Show Keyboard Hints

```rust
div { class: "keyboard-hints",
    span { class: "hint",
        kbd { "Enter" }
        " Preview"
    }
    if preview().is_some() {
        span { class: "hint",
            kbd { "Ctrl" }
            " + "
            kbd { "Enter" }
            " Save"
        }
    }
    span { class: "hint",
        kbd { "Esc" }
        " Close"
    }
}
```

### 6. CSS

```css
.keyboard-hints {
    @apply flex gap-4 text-xs text-gray-500 mt-2 justify-end;
}

.keyboard-hints kbd {
    @apply px-1.5 py-0.5 bg-gray-100 border border-gray-300 rounded text-gray-700 font-mono;
}

.keyboard-hints .hint {
    @apply flex items-center gap-1;
}
```

## Test

1. Run `cargo check`
2. Open Add Link dialog
3. Verify URL input is auto-focused
4. Enter URL, press Enter - verify preview loads
5. Press Ctrl+Enter - verify link saves
6. Press Escape - verify dialog closes
7. Tab through all form elements
````

---

### Step 36.10: Final Integration and Polish

````markdown
## Context

You are working on Rusty Links, a Dioxus 0.7 fullstack application with Tailwind CSS v4. All CSS should be added to `./tailwind.css` in the root of the repo. DO NOT ADD CSS to `./assets/tailwind.css`.
This is the final step to polish the Add Link Flow and ensure everything works together.

## Files to Review and Test

- `src/ui/components/modal/add_link_dialog.rs`
- `src/ui/components/modal/link_details_modal.rs`
- `src/ui/components/add_link_button.rs`
- `src/api/links.rs`

## Requirements

### 1. Error Handling Improvements

Add specific error messages for common failures:

```rust
fn format_error(error: &str) -> String {
    if error.contains("timeout") {
        "The website took too long to respond. The link will be saved but metadata may be incomplete.".to_string()
    } else if error.contains("403") || error.contains("forbidden") {
        "Access to this website was blocked. The link will be saved but metadata may be incomplete.".to_string()
    } else if error.contains("404") {
        "This URL appears to be broken (404 Not Found).".to_string()
    } else if error.contains("certificate") || error.contains("ssl") {
        "This website has security certificate issues.".to_string()
    } else {
        error.to_string()
    }
}
```

### 2. Retry Option for Failed Metadata

If metadata extraction fails, allow retry:

```rust
if let Some(err) = metadata_error() {
    div { class: "error-with-retry",
        p { class: "error-message", "{format_error(&err)}" }
        button {
            class: "btn-retry",
            onclick: move |_| handle_retry(),
            "Retry"
        }
        p { class: "error-hint",
            "You can still save the link. Metadata can be refreshed later."
        }
    }
}
```

### 3. Success Animation

Add a brief success animation when link is created:

```css
@keyframes success-pulse {
    0% { transform: scale(1); }
    50% { transform: scale(1.05); }
    100% { transform: scale(1); }
}

.success-animation {
    animation: success-pulse 0.3s ease-out;
}
```

### 4. Cleanup Signals on Close

Ensure all state is reset when dialog closes:

```rust
let reset_state = move || {
    url_input.set(String::new());
    preview.set(None);
    error.set(None);
    selected_categories.set(vec![]);
    selected_tags.set(vec![]);
    selected_languages.set(vec![]);
    selected_licenses.set(vec![]);
    progress_step.set(None);
};

// Call in on_close handler
let handle_close = move |_| {
    reset_state();
    on_close.call(());
};
```

### 5. Responsive Design Check

Ensure dialog works on mobile:

```css
@media (max-width: 640px) {
    .add-link-dialog {
        @apply max-h-[90vh] overflow-y-auto;
    }

    .categorization-content {
        @apply space-y-3;
    }

    .keyboard-hints {
        @apply hidden; /* Hide on touch devices */
    }

    .preview-section {
        @apply p-3;
    }

    .dialog-footer {
        @apply flex-col gap-2;
    }

    .dialog-footer button {
        @apply w-full;
    }
}
```

### 6. Accessibility Audit

Ensure all elements have proper ARIA attributes:

```rust
// URL Input
input {
    id: "url-input",
    r#type: "url",
    "aria-label": "URL to add",
    "aria-describedby": "url-hint",
    "aria-invalid": if validation_error().is_some() { "true" } else { "false" },
    // ...
}

// Error messages
if let Some(err) = validation_error() {
    div {
        id: "url-error",
        class: "error-message",
        role: "alert",
        "{err}"
    }
}

// Progress
div {
    class: "progress-steps",
    role: "status",
    "aria-live": "polite",
    "aria-label": "Link creation progress",
    // ...
}
```

## Complete Test Checklist

1. **Happy Path**
   - [ ] Enter valid URL
   - [ ] Preview loads with metadata
   - [ ] Select categories/tags/languages/licenses
   - [ ] Save link successfully
   - [ ] Link appears in list with all associations

2. **Duplicate Detection**
   - [ ] Enter existing URL
   - [ ] Duplicate warning appears
   - [ ] Click to view existing link
   - [ ] Can edit existing link

3. **GitHub Repository**
   - [ ] Enter GitHub URL
   - [ ] GitHub metadata extracted (stars, languages, license)
   - [ ] Languages auto-suggested
   - [ ] License auto-suggested
   - [ ] Can modify suggestions

4. **Error Handling**
   - [ ] Invalid URL shows validation error
   - [ ] Inaccessible URL shows warning but allows save
   - [ ] Network error shows retry option
   - [ ] API error shows meaningful message

5. **Keyboard Navigation**
   - [ ] Tab through all fields
   - [ ] Enter triggers preview
   - [ ] Ctrl+Enter saves
   - [ ] Escape closes

6. **Mobile Responsiveness**
   - [ ] Dialog fits on small screens
   - [ ] Buttons are tap-friendly
   - [ ] Dropdowns work on touch

7. **Accessibility**
   - [ ] Screen reader announces progress
   - [ ] Error messages are announced
   - [ ] Focus management is correct
````

---

## Summary

This implementation guide breaks Step 36 into 10 incremental sub-steps:

| Step  | Description              | Complexity |
|-------|--------------------------|------------|
| 36.1  | Wire CategorySelect      | Low        |
| 36.2  | Wire TagSelect           | Low        |
| 36.3  | Wire LanguageSelect      | Low        |
| 36.4  | Wire LicenseSelect       | Low        |
| 36.5  | Add Metadata Preview     | Medium     |
| 36.6  | Add Progress Indicator   | Low        |
| 36.7  | Add Quick Categorization | Medium     |
| 36.8  | Auto-Suggest from GitHub | Medium     |
| 36.9  | Keyboard Navigation      | Low        |
| 36.10 | Final Polish             | Low        |

Each step is designed to be completed in one session and results in working, tested code before moving to the next step.
