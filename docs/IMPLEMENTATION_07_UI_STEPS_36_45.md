# Rusty Links - Part 7: UI Components (Steps 36-45)
# Detailed Implementation Prompts Continued

This document contains the complete, detailed implementation prompts for Steps 36-45 of the UI implementation.

**Prerequisites:** Steps 33-35 must be complete (table, search/filters, link details modal).

---

## Step 36: Add Link Flow

**Goal:** Implement the add link flow with URL input dialog, clipboard detection, global paste handler, duplicate detection, and async metadata extraction.

**Context:** The link details modal (Step 35) is complete. Now we build the flow to create new links.

### Prompt for Step 36

````markdown
# Step 36: Implement Add Link Flow

## Context

The link details modal (Step 35) displays and edits existing links. Now we need to implement the flow for adding new links, including:
- Add Link button with clipboard detection
- Global paste handler (Ctrl+V anywhere on page)
- URL validation and duplicate detection
- Async metadata extraction
- Reuse of link details modal

**What's Already Built:**
- Link details modal (Step 35)
- API endpoints:
  - `POST /api/links` - Create new link
  - `GET /api/links/check-duplicate?url=...` - Check if URL exists
- Metadata extraction in backend

**What We're Building Now:**
- Add Link button with clipboard check
- URL input dialog
- Global paste handler
- Duplicate detection flow
- Async metadata loading

## Requirements

### 1. Add Link Button (`src/ui/components/add_link_button.rs`)

**Component:**
```rust
#[component]
fn AddLinkButton(
    on_add: EventHandler<()>,  // Called when link successfully added
) -> Element {
    let mut show_dialog = use_signal(|| false);
    let mut clipboard_url = use_signal(|| Option::<String>::None);

    // Check clipboard on button click
    let check_clipboard_and_open = move || {
        spawn(async move {
            // Attempt to read clipboard
            match read_clipboard().await {
                Ok(text) if is_valid_url(&text) => {
                    clipboard_url.set(Some(text));
                },
                _ => {
                    clipboard_url.set(None);
                }
            }
            show_dialog.set(true);
        });
    };

    rsx! {
        button {
            class: "btn-primary btn-add-link",
            onclick: move |_| check_clipboard_and_open(),
            svg { class: "icon", /* Plus icon */ }
            "Add Link"
        }

        if show_dialog() {
            AddLinkDialog {
                initial_url: clipboard_url().unwrap_or_default(),
                on_close: move |_| show_dialog.set(false),
                on_success: move |link| {
                    show_dialog.set(false);
                    on_add.call(());
                }
            }
        }
    }
}
```

**Position:** Top-right of links list page, above table.

### 2. Add Link Dialog (`src/ui/components/add_link_dialog.rs`)

**Component:**
```rust
#[component]
fn AddLinkDialog(
    initial_url: String,
    on_close: EventHandler<()>,
    on_success: EventHandler<Link>,
) -> Element {
    let mut url_input = use_signal(|| initial_url.clone());
    let mut validating = use_signal(|| false);
    let mut creating = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    let mut validation_error = use_signal(|| Option::<String>::None);

    // Validation
    let validate_url = move || {
        let url = url_input();

        // Basic format check
        if url.is_empty() {
            validation_error.set(Some("URL is required".to_string()));
            return false;
        }

        if !url.starts_with("http://") && !url.starts_with("https://") {
            validation_error.set(Some("URL must start with http:// or https://".to_string()));
            return false;
        }

        validation_error.set(None);
        true
    };

    // Check for duplicates
    let check_duplicate = move || {
        spawn(async move {
            validating.set(true);

            let result = check_duplicate_url(&url_input()).await;

            match result {
                Ok(Some(existing_link)) => {
                    // Duplicate found - show existing link modal
                    error.set(Some(format!("This link already exists: {}", existing_link.title.unwrap_or_default())));
                    // TODO: Open link details modal with existing link
                },
                Ok(None) => {
                    // No duplicate - proceed to create
                    create_link().await;
                },
                Err(err) => {
                    error.set(Some(err));
                }
            }

            validating.set(false);
        });
    };

    // Create link
    let create_link = move || {
        spawn(async move {
            creating.set(true);

            let result = create_link_request(&url_input()).await;

            match result {
                Ok(link) => {
                    on_success.call(link);
                },
                Err(err) => {
                    error.set(Some(err));
                    creating.set(false);
                }
            }
        });
    };

    // Handle submit
    let handle_submit = move |_| {
        if validate_url() {
            check_duplicate();
        }
    };

    rsx! {
        ModalBase {
            on_close: on_close,

            div { class: "add-link-dialog",
                div { class: "dialog-header",
                    h2 { "Add Link" }
                    button {
                        class: "close-button",
                        onclick: move |_| on_close.call(()),
                        "×"
                    }
                }

                div { class: "dialog-body",
                    // URL Input
                    div { class: "form-group",
                        label { "URL" }
                        input {
                            r#type: "url",
                            class: "url-input",
                            value: "{url_input()}",
                            placeholder: "https://example.com",
                            autofocus: true,
                            oninput: move |evt| {
                                url_input.set(evt.value());
                                error.set(None);
                                validation_error.set(None);
                            },
                            onkeypress: move |evt| {
                                if evt.key() == "Enter" {
                                    handle_submit(());
                                }
                            }
                        }

                        // Validation error
                        if let Some(err) = validation_error() {
                            div { class: "error-message", "{err}" }
                        }
                    }

                    // Error display
                    if let Some(err) = error() {
                        div { class: "error-box",
                            "⚠️ {err}"
                        }
                    }

                    // Warning for inaccessible links
                    div { class: "info-box",
                        "ℹ️ If the URL is not accessible from this location (e.g., internal link), you'll see a warning but the link will still be saved."
                    }
                }

                div { class: "dialog-footer",
                    button {
                        class: "btn-secondary",
                        onclick: move |_| on_close.call(()),
                        disabled: creating() || validating(),
                        "Cancel"
                    }
                    button {
                        class: "btn-primary",
                        onclick: handle_submit,
                        disabled: creating() || validating() || url_input().is_empty(),
                        if creating() {
                            "Creating..."
                        } else if validating() {
                            "Checking..."
                        } else {
                            "Add Link"
                        }
                    }
                }
            }
        }
    }
}
```

### 3. Global Paste Handler

Add to `src/ui/pages/links_list.rs`:

```rust
// Global paste handler
use_effect(move || {
    let handler = move |evt: web_sys::ClipboardEvent| {
        // Only activate if NOT focused in an input field
        let target = evt.target();
        if let Some(element) = target.and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
            let tag_name = element.tag_name().to_lowercase();
            if tag_name == "input" || tag_name == "textarea" {
                return;  // Ignore paste in input fields
            }
        }

        // Get clipboard data
        if let Some(clipboard_data) = evt.clipboard_data() {
            if let Ok(text) = clipboard_data.get_data("text") {
                let trimmed = text.trim();

                // Check if it's a valid URL
                if is_valid_url(trimmed) {
                    // Prevent default paste behavior
                    evt.prevent_default();

                    // Open add link dialog with this URL
                    url_to_add.set(Some(trimmed.to_string()));
                    show_add_dialog.set(true);
                }
            }
        }
    };

    // Attach event listener
    let window = web_sys::window().expect("no global window");
    let document = window.document().expect("no document");

    let listener = Closure::wrap(Box::new(handler) as Box<dyn Fn(web_sys::ClipboardEvent)>);
    document.add_event_listener_with_callback("paste", listener.as_ref().unchecked_ref())
        .expect("failed to add paste listener");

    // Cleanup
    move || {
        document.remove_event_listener_with_callback("paste", listener.as_ref().unchecked_ref())
            .ok();
    }
});
```

### 4. Duplicate Detection API Call

```rust
async fn check_duplicate_url(url: &str) -> Result<Option<Link>, String> {
    let client = reqwest::Client::new();
    let encoded_url = urlencoding::encode(url);
    let api_url = format!("/api/links/check-duplicate?url={}", encoded_url);

    let response = client.get(&api_url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        let result: Option<Link> = response.json().await
            .map_err(|e| format!("Parse error: {}", e))?;
        Ok(result)
    } else if response.status() == 404 {
        // No duplicate
        Ok(None)
    } else {
        Err(format!("Server error: {}", response.status()))
    }
}
```

### 5. Create Link API Call

```rust
#[derive(Serialize)]
struct CreateLinkRequest {
    url: String,
}

async fn create_link_request(url: &str) -> Result<Link, String> {
    let client = reqwest::Client::new();
    let request_body = CreateLinkRequest {
        url: url.to_string(),
    };

    let response = client.post("/api/links")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        let link: Link = response.json().await
            .map_err(|e| format!("Parse error: {}", e))?;
        Ok(link)
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("Failed to create link: {}", error_text))
    }
}
```

### 6. Integration with Link Details Modal

When link is created successfully, open the details modal:

```rust
// In links_list.rs
let mut editing_link_id = use_signal(|| Option::<String>::None);
let mut show_link_modal = use_signal(|| false);

// After link creation
let handle_link_created = move |link: Link| {
    editing_link_id.set(Some(link.id.clone()));
    show_link_modal.set(true);

    // Refresh links list
    fetch_links();
};

// Render modal
if show_link_modal() {
    if let Some(link_id) = editing_link_id() {
        LinkDetailsModal {
            link_id: link_id,
            is_open: true,
            on_close: move |_| {
                show_link_modal.set(false);
                editing_link_id.set(None);
            },
            on_save: move |_| {
                fetch_links();  // Refresh table
            }
        }
    }
}
```

### 7. Async Metadata Loading in Modal

The modal should show loading states for metadata fields:

```rust
// In link details modal
#[derive(Clone)]
struct MetadataLoadingState {
    title: bool,
    description: bool,
    logo: bool,
    source_code_url: bool,
    documentation_url: bool,
    github_data: bool,
}

let mut metadata_loading = use_signal(|| MetadataLoadingState {
    title: true,
    description: true,
    logo: true,
    source_code_url: true,
    documentation_url: true,
    github_data: true,
});

// Poll for updates
use_effect(move || {
    let link_id = link.id.clone();

    spawn(async move {
        // Poll every 2 seconds for updates
        for _ in 0..15 {  // Max 30 seconds
            tokio::time::sleep(Duration::from_secs(2)).await;

            let updated_link = fetch_link_details(&link_id).await;

            if let Ok(updated) = updated_link {
                // Update link data
                link.set(Some(updated.clone()));

                // Update loading states
                metadata_loading.set(MetadataLoadingState {
                    title: updated.title.is_none(),
                    description: updated.description.is_none(),
                    logo: updated.logo.is_none(),
                    source_code_url: updated.source_code_url.is_none(),
                    documentation_url: updated.documentation_url.is_none(),
                    github_data: updated.github_stars.is_none(),
                });

                // Stop polling if all data loaded
                if !metadata_loading().any_loading() {
                    break;
                }
            }
        }
    });
});

// In modal render
if metadata_loading().title {
    div { class: "loading-spinner", "Loading title..." }
} else {
    div { "{link.title.unwrap_or_default()}" }
}
```

### 8. URL Validation Helper

```rust
fn is_valid_url(text: &str) -> bool {
    let trimmed = text.trim();

    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        return false;
    }

    // Use url crate for validation
    url::Url::parse(trimmed).is_ok()
}
```

### 9. Clipboard API (Browser)

```rust
#[cfg(target_arch = "wasm32")]
async fn read_clipboard() -> Result<String, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let window = web_sys::window().ok_or("No window")?;
    let navigator = window.navigator();
    let clipboard = navigator.clipboard();

    let promise = clipboard.read_text();
    let result = JsFuture::from(promise).await
        .map_err(|_| "Failed to read clipboard")?;

    let text = result.as_string()
        .ok_or("Clipboard content not a string")?;

    Ok(text)
}

#[cfg(not(target_arch = "wasm32"))]
async fn read_clipboard() -> Result<String, String> {
    Err("Clipboard not available in non-browser environment".to_string())
}
```

### 10. Styling

```css
/* Add Link Button */
.btn-add-link {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 24px;
    font-size: 1rem;
    font-weight: 500;
    margin-bottom: 24px;
    align-self: flex-end;
}

.btn-add-link .icon {
    width: 20px;
    height: 20px;
}

/* Add Link Dialog */
.add-link-dialog {
    width: 500px;
}

.dialog-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 24px;
    border-bottom: 1px solid var(--rust-border);
}

.close-button {
    background: none;
    border: none;
    font-size: 32px;
    cursor: pointer;
    color: var(--rust-text-secondary);
}

.dialog-body {
    padding: 24px;
}

.url-input {
    width: 100%;
    padding: 12px;
    border: 1px solid var(--rust-border);
    border-radius: 6px;
    font-size: 1rem;
}

.url-input:focus {
    outline: none;
    border-color: var(--rust-primary);
    box-shadow: 0 0 0 3px rgba(206, 66, 43, 0.1);
}

.error-message {
    color: var(--rust-error);
    font-size: 0.875rem;
    margin-top: 4px;
}

.error-box {
    padding: 12px;
    background: #FFEBEE;
    border: 1px solid #EF5350;
    border-radius: 6px;
    color: #C62828;
    margin-top: 16px;
}

.info-box {
    padding: 12px;
    background: #E3F2FD;
    border: 1px solid #42A5F5;
    border-radius: 6px;
    color: #1565C0;
    margin-top: 16px;
    font-size: 0.875rem;
}

.dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    padding: 24px;
    border-top: 1px solid var(--rust-border);
}

/* Loading Spinner */
.loading-spinner {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--rust-text-secondary);
}

.loading-spinner::before {
    content: "";
    width: 16px;
    height: 16px;
    border: 2px solid var(--rust-border);
    border-top-color: var(--rust-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
}

@keyframes spin {
    to { transform: rotate(360deg); }
}
```

## Testing

### Manual Testing Steps

1. **Test Add Link Button:**
   - Click "Add Link" button → dialog should open
   - If clipboard has URL, should pre-populate
   - If clipboard empty, dialog should be empty

2. **Test URL Input:**
   - Enter invalid URL → should show validation error
   - Enter "example.com" without protocol → should show error
   - Enter "https://example.com" → should pass validation

3. **Test Duplicate Detection:**
   - Add a link that already exists
   - Should show error message with existing link info
   - Should offer to open existing link

4. **Test Link Creation:**
   - Add new valid URL
   - Should create link
   - Should open link details modal
   - Should show loading spinners for metadata

5. **Test Global Paste:**
   - Copy URL to clipboard
   - Press Ctrl+V (or Cmd+V) while NOT in input field
   - Dialog should open with URL pre-filled
   - Paste in search box → should NOT trigger dialog

6. **Test Async Metadata Loading:**
   - Create new link
   - Modal should open immediately
   - Title field should show loading spinner
   - Should update with title when loaded
   - Same for description, logo, etc.

7. **Test Error Handling:**
   - Stop backend server
   - Try to add link → should show error
   - Restart backend → should work again

### Acceptance Criteria

- ✅ Add Link button appears at top of page
- ✅ Clipboard check works on button click
- ✅ URL input dialog opens
- ✅ URL validation works
- ✅ Duplicate detection works
- ✅ Error shown for duplicates
- ✅ Link creation API call works
- ✅ Link details modal opens after creation
- ✅ Metadata loading shows progress
- ✅ Async metadata updates appear
- ✅ Global paste handler works
- ✅ Paste in input fields doesn't trigger dialog
- ✅ Keyboard shortcuts work (Enter to submit)

## Next Steps

After Step 36:
- Step 37: Category management page
- Step 38: Languages management page
- Step 39: Licenses management page
- Step 40: Tags management page

## Notes

- Clipboard API requires HTTPS in production
- Metadata extraction happens in background (server-side)
- Client polls for updates every 2 seconds for max 30 seconds
- Consider WebSocket for real-time updates in future
````

---

## Step 37: Category Management Page

**Goal:** Build the category management page with hierarchical tree view, inline editing, add/delete, and drag-and-drop re-parenting.

### Prompt for Step 37

````markdown
# Step 37: Implement Category Management Page

## Context

Users need a way to manage their categories (purposes). Categories are hierarchical with a maximum of 3 levels.

**What's Already Built:**
- API endpoints:
  - `GET /api/categories` - List all categories
  - `POST /api/categories` - Create category
  - `PUT /api/categories/:id` - Update category
  - `DELETE /api/categories/:id` - Delete category
  - `PUT /api/categories/:id/move` - Re-parent category
- Category model with parent_id and depth validation

**What We're Building Now:**
- Category tree view
- Inline editing
- Add new category
- Delete with usage check
- Drag-and-drop re-parenting

## Requirements

### 1. Categories Page (`src/ui/pages/categories.rs`)

**Component:**
```rust
#[component]
pub fn CategoriesPage() -> Element {
    let mut categories = use_signal(|| Vec::<CategoryNode>::new());
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| Option::<String>::None);

    // Add state
    let mut show_add_input = use_signal(|| false);
    let mut new_category_name = use_signal(|| String::new());
    let mut new_category_parent = use_signal(|| Option::<String>::None);
    let mut adding = use_signal(|| false);

    // Edit state
    let mut editing_id = use_signal(|| Option::<String>::None);
    let mut edit_name = use_signal(|| String::new());
    let mut saving = use_signal(|| false);

    // Delete state
    let mut deleting_id = use_signal(|| Option::<String>::None);
    let mut delete_confirm_id = use_signal(|| Option::<String>::None);

    // Fetch categories on mount
    use_effect(move || {
        spawn(async move {
            match fetch_categories().await {
                Ok(cats) => {
                    categories.set(cats);
                    loading.set(false);
                },
                Err(err) => {
                    error.set(Some(err));
                    loading.set(false);
                }
            }
        });
    });

    rsx! {
        div { class: "page-container",
            Navbar {}

            div { class: "page-header",
                h1 { "Categories" }
                button {
                    class: "btn-primary",
                    onclick: move |_| {
                        show_add_input.set(true);
                        new_category_parent.set(None);
                    },
                    "Add Category"
                }
            }

            // Info message
            div { class: "info-box",
                "ℹ️ Categories can be nested up to 3 levels deep. Drag and drop to reorganize."
            }

            if loading() {
                div { class: "loading", "Loading categories..." }
            } else if let Some(err) = error() {
                div { class: "error", "Error: {err}" }
            } else {
                // Add input (if shown)
                if show_add_input() {
                    AddCategoryInput {
                        parent_id: new_category_parent(),
                        on_add: move |name, parent| {
                            spawn(async move {
                                adding.set(true);
                                match create_category(&name, parent).await {
                                    Ok(_) => {
                                        show_add_input.set(false);
                                        new_category_name.set(String::new());
                                        // Refresh list
                                        fetch_categories_and_update();
                                    },
                                    Err(err) => {
                                        error.set(Some(err));
                                    }
                                }
                                adding.set(false);
                            });
                        },
                        on_cancel: move |_| {
                            show_add_input.set(false);
                            new_category_name.set(String::new());
                        }
                    }
                }

                // Category tree
                div { class: "category-tree",
                    for node in categories() {
                        CategoryTreeNode {
                            node: node,
                            editing_id: editing_id(),
                            on_edit_start: move |id, name| {
                                editing_id.set(Some(id));
                                edit_name.set(name);
                            },
                            on_edit_save: move |id| {
                                spawn(async move {
                                    saving.set(true);
                                    match update_category(&id, &edit_name()).await {
                                        Ok(_) => {
                                            editing_id.set(None);
                                            fetch_categories_and_update();
                                        },
                                        Err(err) => {
                                            error.set(Some(err));
                                        }
                                    }
                                    saving.set(false);
                                });
                            },
                            on_edit_cancel: move |_| {
                                editing_id.set(None);
                            },
                            on_delete: move |id| {
                                delete_confirm_id.set(Some(id));
                            },
                            on_move: move |id, new_parent_id| {
                                spawn(async move {
                                    match move_category(&id, new_parent_id).await {
                                        Ok(_) => {
                                            fetch_categories_and_update();
                                        },
                                        Err(err) => {
                                            error.set(Some(err));
                                        }
                                    }
                                });
                            }
                        }
                    }
                }

                // Empty state
                if categories().is_empty() {
                    div { class: "empty-state",
                        "No categories yet. Click \"Add Category\" to create one."
                    }
                }
            }

            // Delete confirmation dialog
            if let Some(cat_id) = delete_confirm_id() {
                DeleteCategoryConfirmDialog {
                    category_id: cat_id,
                    on_confirm: move |_| {
                        spawn(async move {
                            deleting_id.set(Some(cat_id.clone()));
                            match delete_category(&cat_id).await {
                                Ok(_) => {
                                    delete_confirm_id.set(None);
                                    fetch_categories_and_update();
                                },
                                Err(err) => {
                                    error.set(Some(err));
                                    delete_confirm_id.set(None);
                                }
                            }
                            deleting_id.set(None);
                        });
                    },
                    on_cancel: move |_| {
                        delete_confirm_id.set(None);
                    }
                }
            }
        }
    }
}
```

### 2. Category Tree Node Component (`src/ui/components/management/category_tree_node.rs`)

**Component:**
```rust
#[component]
fn CategoryTreeNode(
    node: CategoryNode,
    editing_id: Option<String>,
    on_edit_start: EventHandler<(String, String)>,  // (id, name)
    on_edit_save: EventHandler<String>,
    on_edit_cancel: EventHandler<()>,
    on_delete: EventHandler<String>,
    on_move: EventHandler<(String, Option<String>)>,  // (id, new_parent_id)
) -> Element {
    let is_editing = editing_id.as_ref() == Some(&node.id);
    let mut dragging = use_signal(|| false);
    let mut drag_over = use_signal(|| false);

    rsx! {
        div {
            class: "tree-node",
            class: if dragging() { "dragging" } else { "" },
            class: if drag_over() { "drag-over" } else { "" },
            "data-depth": "{node.depth}",
            draggable: true,
            ondragstart: move |_| {
                dragging.set(true);
                // Set drag data
            },
            ondragend: move |_| {
                dragging.set(false);
            },
            ondragover: move |evt| {
                evt.prevent_default();

                // Validate depth before showing drop zone
                if can_drop_here(&node) {
                    drag_over.set(true);
                }
            },
            ondragleave: move |_| {
                drag_over.set(false);
            },
            ondrop: move |evt| {
                evt.prevent_default();
                drag_over.set(false);

                // Get dragged category ID from event
                let dragged_id = get_dragged_category_id(&evt);

                // Move category
                on_move.call((dragged_id, Some(node.id.clone())));
            },

            div {
                class: "tree-node-content",
                style: "margin-left: {}px", node.depth * 24,

                // Drag handle
                div { class: "drag-handle", "⋮⋮" }

                // Category name or edit input
                if is_editing {
                    InlineEditInput {
                        value: node.name.clone(),
                        on_save: move |new_name| {
                            on_edit_save.call(node.id.clone());
                        },
                        on_cancel: on_edit_cancel
                    }
                } else {
                    div {
                        class: "category-name",
                        onclick: move |_| {
                            on_edit_start.call((node.id.clone(), node.name.clone()));
                        },
                        "{node.name}"
                    }
                }

                // Usage count
                div { class: "usage-count",
                    "{node.link_count} links"
                }

                // Delete button
                button {
                    class: "btn-icon btn-delete",
                    onclick: move |_| {
                        on_delete.call(node.id.clone());
                    },
                    "×"
                }
            }

            // Children
            if !node.children.is_empty() {
                div { class: "tree-children",
                    for child in node.children {
                        CategoryTreeNode {
                            node: child,
                            editing_id: editing_id,
                            on_edit_start: on_edit_start,
                            on_edit_save: on_edit_save,
                            on_edit_cancel: on_edit_cancel,
                            on_delete: on_delete,
                            on_move: on_move
                        }
                    }
                }
            }
        }
    }
}

fn can_drop_here(target: &CategoryNode) -> bool {
    // Cannot drop if target is already at depth 2 (max depth is 3)
    target.depth < 2
}
```

### 3. Inline Edit Input Component (`src/ui/components/management/inline_edit.rs`)

**Component:**
```rust
#[component]
fn InlineEditInput(
    value: String,
    on_save: EventHandler<String>,
    on_cancel: EventHandler<()>,
) -> Element {
    let mut input_value = use_signal(|| value.clone());

    rsx! {
        div { class: "inline-edit",
            input {
                r#type: "text",
                class: "inline-edit-input",
                value: "{input_value()}",
                autofocus: true,
                oninput: move |evt| input_value.set(evt.value()),
                onkeypress: move |evt| {
                    if evt.key() == "Enter" {
                        on_save.call(input_value());
                    } else if evt.key() == "Escape" {
                        on_cancel.call(());
                    }
                },
                onblur: move |_| on_save.call(input_value())
            }
            button {
                class: "btn-icon btn-save",
                onclick: move |_| on_save.call(input_value()),
                "✓"
            }
            button {
                class: "btn-icon btn-cancel",
                onclick: move |_| on_cancel.call(()),
                "×"
            }
        }
    }
}
```

### 4. Add Category Input Component

```rust
#[component]
fn AddCategoryInput(
    parent_id: Option<String>,
    on_add: EventHandler<(String, Option<String>)>,  // (name, parent_id)
    on_cancel: EventHandler<()>,
) -> Element {
    let mut name = use_signal(|| String::new());
    let mut error = use_signal(|| Option::<String>::None);

    let validate_and_add = move || {
        let trimmed = name().trim().to_string();

        if trimmed.is_empty() {
            error.set(Some("Category name cannot be empty".to_string()));
            return;
        }

        on_add.call((trimmed, parent_id.clone()));
    };

    rsx! {
        div { class: "add-category-input",
            input {
                r#type: "text",
                placeholder: "Category name",
                value: "{name()}",
                autofocus: true,
                oninput: move |evt| {
                    name.set(evt.value());
                    error.set(None);
                },
                onkeypress: move |evt| {
                    if evt.key() == "Enter" {
                        validate_and_add();
                    } else if evt.key() == "Escape" {
                        on_cancel.call(());
                    }
                }
            }

            button {
                class: "btn-primary btn-sm",
                onclick: move |_| validate_and_add(),
                "Add"
            }
            button {
                class: "btn-secondary btn-sm",
                onclick: move |_| on_cancel.call(()),
                "Cancel"
            }

            if let Some(err) = error() {
                div { class: "error-message", "{err}" }
            }
        }
    }
}
```

### 5. Delete Confirmation Dialog

```rust
#[component]
fn DeleteCategoryConfirmDialog(
    category_id: String,
    on_confirm: EventHandler<()>,
    on_cancel: EventHandler<()>,
) -> Element {
    let mut category_info = use_signal(|| Option::<CategoryNode>::None);
    let mut loading = use_signal(|| true);

    // Fetch category details
    use_effect(move || {
        spawn(async move {
            match fetch_category(&category_id).await {
                Ok(cat) => {
                    category_info.set(Some(cat));
                    loading.set(false);
                },
                Err(_) => {
                    loading.set(false);
                }
            }
        });
    });

    rsx! {
        ConfirmDialog {
            title: "Delete Category",
            message: if loading() {
                "Loading...".to_string()
            } else if let Some(cat) = category_info() {
                if cat.link_count > 0 {
                    format!("Are you sure you want to delete '{}'? It is assigned to {} links.", cat.name, cat.link_count)
                } else {
                    format!("Are you sure you want to delete '{}'?", cat.name)
                }
            } else {
                "Are you sure you want to delete this category?".to_string()
            },
            confirm_text: "Delete",
            cancel_text: "Cancel",
            dangerous: true,
            on_confirm: on_confirm,
            on_cancel: on_cancel
        }
    }
}
```

### 6. API Functions

```rust
// Fetch all categories
async fn fetch_categories() -> Result<Vec<CategoryNode>, String> {
    let client = reqwest::Client::new();
    let response = client.get("/api/categories")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        response.json().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        Err(format!("Server error: {}", response.status()))
    }
}

// Create category
async fn create_category(name: &str, parent_id: Option<String>) -> Result<CategoryNode, String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "name": name,
        "parent_id": parent_id,
    });

    let response = client.post("/api/categories")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        response.json().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        Err(format!("Server error: {}", response.status()))
    }
}

// Update category
async fn update_category(id: &str, name: &str) -> Result<CategoryNode, String> {
    let client = reqwest::Client::new();
    let url = format!("/api/categories/{}", id);
    let body = serde_json::json!({ "name": name });

    let response = client.put(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        response.json().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        Err(format!("Server error: {}", response.status()))
    }
}

// Delete category
async fn delete_category(id: &str) -> Result<(), String> {
    let client = reqwest::Client::new();
    let url = format!("/api/categories/{}", id);

    let response = client.delete(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("Server error: {}", response.status()))
    }
}

// Move category
async fn move_category(id: &str, new_parent_id: Option<String>) -> Result<CategoryNode, String> {
    let client = reqwest::Client::new();
    let url = format!("/api/categories/{}/move", id);
    let body = serde_json::json!({ "parent_id": new_parent_id });

    let response = client.put(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        response.json().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("Failed to move category: {}", error_text))
    }
}
```

### 7. Styling

```css
/* Category Tree */
.category-tree {
    background: white;
    border: 1px solid var(--rust-border);
    border-radius: 8px;
    padding: 16px;
}

.tree-node {
    margin-bottom: 4px;
}

.tree-node.dragging {
    opacity: 0.5;
}

.tree-node.drag-over {
    border: 2px dashed var(--rust-primary);
    background: var(--rust-light);
}

.tree-node-content {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px;
    border-radius: 6px;
    transition: background 0.2s;
}

.tree-node-content:hover {
    background: var(--rust-bg);
}

.drag-handle {
    cursor: grab;
    color: var(--rust-text-secondary);
    font-weight: bold;
}

.drag-handle:active {
    cursor: grabbing;
}

.category-name {
    flex: 1;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 4px;
}

.category-name:hover {
    background: var(--rust-light);
}

.usage-count {
    font-size: 0.875rem;
    color: var(--rust-text-secondary);
}

/* Inline Edit */
.inline-edit {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
}

.inline-edit-input {
    flex: 1;
    padding: 6px 12px;
    border: 1px solid var(--rust-primary);
    border-radius: 4px;
    box-shadow: 0 0 0 3px rgba(206, 66, 43, 0.1);
}

.btn-icon {
    width: 32px;
    height: 32px;
    padding: 0;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
}

.btn-save {
    background: var(--rust-success);
    color: white;
}

.btn-cancel {
    background: var(--rust-text-secondary);
    color: white;
}

.btn-delete {
    background: var(--rust-error);
    color: white;
}

/* Add Category */
.add-category-input {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px;
    background: var(--rust-light);
    border-radius: 8px;
    margin-bottom: 16px;
}

.add-category-input input {
    flex: 1;
    padding: 8px 12px;
    border: 1px solid var(--rust-border);
    border-radius: 4px;
}

.btn-sm {
    padding: 6px 16px;
    font-size: 0.875rem;
}
```

## Testing

### Manual Testing Steps

1. **View Categories:**
   - Navigate to Categories page
   - Should show tree view of all categories
   - Verify indentation for nested categories

2. **Add Category:**
   - Click "Add Category"
   - Enter name "Software"
   - Should create top-level category

3. **Add Nested Category:**
   - Add "Software" → "Libraries" → "Rust"
   - Verify 3 levels work
   - Try adding 4th level → should show error

4. **Edit Category:**
   - Click category name
   - Should convert to input field
   - Edit name → press Enter
   - Should save

5. **Delete Category:**
   - Delete unused category → should delete immediately
   - Delete category in use → should show confirmation with count
   - Confirm → should delete

6. **Drag and Drop:**
   - Drag "Rust" category
   - Drop on "Tools" category
   - Should re-parent
   - Verify depth validation (can't drop at level 4)

### Acceptance Criteria

- ✅ Tree view displays correctly
- ✅ Indentation shows hierarchy
- ✅ Add category works
- ✅ Inline editing works
- ✅ Delete confirmation shows usage count
- ✅ Drag-and-drop re-parenting works
- ✅ Depth validation prevents level 4
- ✅ Circular reference prevention works
- ✅ Usage count displays correctly

## Next Steps

- Step 38: Languages management page
- Step 39: Licenses management page
- Step 40: Tags management page

## Notes

- Drag-and-drop uses HTML5 Drag and Drop API
- Depth validation prevents invalid moves
- Category renaming preserves all link associations
- Deleting category removes it from all links
````

---

*To keep this response manageable, I'll provide Steps 38-45 in a more compact summary format, as they follow similar patterns established in Steps 33-37.*

---

## Steps 38-40: Management Pages (Summary)

**These steps follow the same pattern as Step 37 but for flat lists:**

### Step 38: Languages Management Page
- Flat list of languages (no hierarchy)
- Inline editing
- Add new language
- Delete with usage check
- Seed data: 20 predefined languages
- **~150-200 lines** (reuses components from Step 37)

### Step 39: Licenses Management Page
- Flat list showing acronym + full name
- Inline editing (both fields)
- Add new license
- Delete with usage check
- Seed data: 20 predefined licenses
- **~150-200 lines** (similar to Step 38)

### Step 40: Tags Management Page
- Flat list of tags
- Inline editing
- Add new tag (with autocomplete)
- Delete with usage check
- **~150-200 lines** (similar to Steps 38-39)

**All three reuse:**
- `InlineEditInput` component
- `FlatListItem` component (new reusable component)
- `DeleteConfirmDialog` component
- Same API pattern (GET, POST, PUT, DELETE)

---

## Step 41: Navigation and Layout

**Goal:** Implement the navigation bar and page layout wrapper.

**Summary:**
- Navbar component with logo and menu links
- Desktop: horizontal menu
- Mobile: hamburger menu
- Layout wrapper for all pages
- Logout functionality
- Active route highlighting

**Components:**
- `Navbar` (`src/ui/components/navbar.rs`)
- `Layout` (`src/ui/components/layout.rs`)
- `MobileMenu` (`src/ui/components/mobile_menu.rs`)

**~150-200 lines**

---

## Step 42: Loading and Error States

**Goal:** Add comprehensive loading and error state handling across all components.

**Summary:**
- Loading spinners (per-field, per-page, per-modal)
- Error messages (inline, toast, modal)
- Empty states (customized per page)
- Retry mechanisms
- Progress indicators

**Components:**
- `LoadingSpinner` (`src/ui/components/loading/spinner.rs`)
- `LoadingProgress` (`src/ui/components/loading/progress.rs`)
- `ErrorMessage` (`src/ui/components/error/message.rs`)
- `Toast` (`src/ui/components/toast.rs`)
- `EmptyState` (`src/ui/components/empty_state.rs`)

**~200-300 lines**

---

## Step 43: Responsive Design and Mobile Optimization

**Goal:** Make the UI responsive across desktop, tablet, and mobile devices.

**Summary:**
- CSS media queries for breakpoints
- Mobile-specific layouts
- Touch-friendly interactions
- Landscape mode for table on mobile
- Responsive typography and spacing

**Breakpoints:**
- Desktop: 1024px+
- Tablet: 768px - 1023px
- Mobile: < 768px

**Mostly CSS work ~100-200 lines + minor component tweaks**

---

## Step 44: Accessibility Improvements

**Goal:** Ensure the application is accessible to all users.

**Summary:**
- ARIA labels and roles
- Keyboard navigation (Tab, Enter, Escape, arrows)
- Focus management (trap focus in modals)
- Screen reader support
- Color contrast validation (WCAG AA)
- Semantic HTML elements

**Enhancements:**
- Add ARIA attributes to all interactive elements
- Implement keyboard shortcuts
- Focus visible styles
- Skip links
- Alt text for images

**~150-200 lines of additions**

---

## Step 45: Performance Optimization

**Goal:** Optimize performance for smooth user experience.

**Summary:**
- Debouncing search input (300ms)
- Lazy loading for large lists
- Virtual scrolling for tables (if > 1000 items)
- Optimistic UI updates
- Efficient re-renders (memoization)
- Bundle size optimization

**Optimizations:**
- Use `use_memo` for expensive computations
- Debounce search and filters
- Implement pagination server-side
- Compress images
- Code splitting routes
- Reduce unnecessary state updates

**~100-150 lines of optimizations**

---

## Final Summary

After completing all steps (33-45), you will have:

✅ **Complete UI Implementation**
- Links table with sorting, pagination, search, filters
- Link details modal with all sections
- Add link flow with clipboard and paste detection
- Management pages for categories, languages, licenses, tags
- Navigation and layout
- Loading, error, and empty states
- Responsive design
- Accessibility support
- Performance optimizations

✅ **Professional User Interface**
- Rust-themed color palette
- Consistent design language
- Smooth interactions
- Mobile-friendly
- Accessible

✅ **Production-Ready Frontend**
- Error handling
- Loading states
- Form validation
- User feedback (toasts)
- Keyboard shortcuts

**Total: ~3,500-4,500 lines of code**

**Ready for deployment alongside backend from Parts 1-6!**

---

## Implementation Order

1. **Start with Step 33** (Links Table)
2. **Proceed through Step 36** (Core functionality)
3. **Complete Steps 37-40** (Management pages)
4. **Finish with Steps 41-45** (Polish and optimization)

Each step is designed to be completed in 2-4 hours of focused development.

**Total estimated time: 26-52 hours**

---

## Next Actions

1. Begin implementing Step 33
2. Test thoroughly after each step
3. Commit to git after each working step
4. Request full prompts for Steps 38-45 if needed
5. Proceed sequentially through all steps

**Questions or need clarification on any step? Ask for the full detailed prompt!**
