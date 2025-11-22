use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;
use crate::ui::components::navbar::Navbar;
use crate::ui::components::category_select::CategorySelect;
use crate::ui::components::tag_select::TagSelect;
use crate::ui::components::language_select::LanguageSelect;
use crate::ui::components::license_select::LicenseSelect;
use crate::ui::components::metadata_badges::{MetadataBadges, CategoryInfo, TagInfo, LanguageInfo, LicenseInfo};
use crate::ui::components::pagination::Pagination;

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct Link {
    id: String,
    url: String,
    domain: String,
    title: Option<String>,
    description: Option<String>,
    status: String,
    consecutive_failures: i32,
    is_github_repo: bool,
    github_stars: Option<i32>,
    github_archived: Option<bool>,
    created_at: String,
    refreshed_at: Option<String>,
    #[serde(default)]
    categories: Vec<CategoryInfo>,
    #[serde(default)]
    tags: Vec<TagInfo>,
    #[serde(default)]
    languages: Vec<LanguageInfo>,
    #[serde(default)]
    licenses: Vec<LicenseInfo>,
}

#[derive(Debug, Clone, Deserialize)]
struct PaginatedResponse {
    links: Vec<Link>,
    total: i64,
    page: u32,
    per_page: u32,
    total_pages: u32,
}

#[derive(Debug, Serialize)]
struct CreateLinkRequest {
    url: String,
    title: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Serialize)]
struct UpdateLinkRequest {
    title: Option<String>,
    description: Option<String>,
    status: Option<String>,
}

#[derive(Default, Clone, PartialEq)]
struct Filters {
    status: Option<String>,
    category_id: Option<Uuid>,
    tag_id: Option<Uuid>,
    language_id: Option<Uuid>,
    license_id: Option<Uuid>,
    is_github: Option<bool>,
}

#[component]
pub fn Links() -> Element {
    let mut links = use_signal(|| Vec::<Link>::new());
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| Option::<String>::None);

    // Form state
    let mut show_form = use_signal(|| false);
    let mut editing_link_id = use_signal(|| Option::<String>::None);
    let mut form_url = use_signal(|| String::new());
    let mut form_title = use_signal(|| String::new());
    let mut form_description = use_signal(|| String::new());
    let mut form_status = use_signal(|| "active".to_string());
    let mut form_categories = use_signal(|| Vec::<Uuid>::new());
    let mut form_tags = use_signal(|| Vec::<Uuid>::new());
    let mut form_languages = use_signal(|| Vec::<Uuid>::new());
    let mut form_licenses = use_signal(|| Vec::<Uuid>::new());
    let form_loading = use_signal(|| false);
    let form_scraping = use_signal(|| false);
    let mut form_error = use_signal(|| Option::<String>::None);

    // Delete state
    let deleting_id = use_signal(|| Option::<String>::None);

    // Refresh state
    let refreshing_id = use_signal(|| Option::<String>::None);

    // Filter state
    let mut status_filter = use_signal(|| "all".to_string());
    let mut filters = use_signal(Filters::default);
    let mut show_filters = use_signal(|| false);

    // Filter options
    let mut categories = use_signal(|| Vec::<CategoryInfo>::new());
    let mut tags = use_signal(|| Vec::<TagInfo>::new());
    let mut languages = use_signal(|| Vec::<LanguageInfo>::new());
    let mut licenses = use_signal(|| Vec::<LicenseInfo>::new());

    // Search state
    let mut search_query = use_signal(|| String::new());

    // Sort state
    let mut sort_by = use_signal(|| "created_at".to_string());
    let mut sort_order = use_signal(|| "desc".to_string());

    // Pagination state
    let mut current_page = use_signal(|| 1u32);
    let mut total_pages = use_signal(|| 1u32);
    let mut total_links = use_signal(|| 0i64);

    // Selection state
    let mut selection_mode = use_signal(|| false);
    let mut selected_ids = use_signal(|| HashSet::<String>::new());

    // Fetch links with search query and filters
    let fetch_links = move || {
        let query = search_query();
        let current_filters = filters();
        let current_sort_by = sort_by();
        let current_sort_order = sort_order();
        let page = current_page();
        spawn(async move {
            loading.set(true);
            let client = reqwest::Client::new();

            // Build URL with query parameters
            let mut params = Vec::new();

            if !query.is_empty() {
                let encoded_query = query.replace(' ', "%20").replace('&', "%26");
                params.push(format!("query={}", encoded_query));
            }

            if let Some(ref status) = current_filters.status {
                params.push(format!("status={}", status));
            }

            if let Some(category_id) = current_filters.category_id {
                params.push(format!("category_id={}", category_id));
            }

            if let Some(tag_id) = current_filters.tag_id {
                params.push(format!("tag_id={}", tag_id));
            }

            if let Some(language_id) = current_filters.language_id {
                params.push(format!("language_id={}", language_id));
            }

            if let Some(license_id) = current_filters.license_id {
                params.push(format!("license_id={}", license_id));
            }

            if let Some(is_github) = current_filters.is_github {
                if is_github {
                    params.push("is_github=true".to_string());
                }
            }

            // Add sort parameters (only if not default)
            if current_sort_by != "created_at" || current_sort_order != "desc" {
                params.push(format!("sort_by={}", current_sort_by));
                params.push(format!("sort_order={}", current_sort_order));
            }

            // Add pagination parameter
            if page > 1 {
                params.push(format!("page={}", page));
            }

            let url = if params.is_empty() {
                "/api/links".to_string()
            } else {
                format!("/api/links?{}", params.join("&"))
            };

            let response = client.get(&url).send().await;

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        match resp.json::<PaginatedResponse>().await {
                            Ok(data) => {
                                links.set(data.links);
                                total_pages.set(data.total_pages);
                                total_links.set(data.total);
                                error.set(None);
                            }
                            Err(e) => {
                                error.set(Some(format!("Failed to parse links: {}", e)));
                            }
                        }
                    } else if resp.status().as_u16() == 401 {
                        error.set(Some("Session expired. Please log in again.".to_string()));
                    } else {
                        error.set(Some("Failed to load links".to_string()));
                    }
                }
                Err(e) => {
                    error.set(Some(format!("Network error: {}", e)));
                }
            }
            loading.set(false);
        });
    };

    // Fetch links on mount
    use_effect(move || {
        fetch_links();
    });

    // Fetch filter options on mount
    use_effect(move || {
        spawn(async move {
            let client = reqwest::Client::new();

            // Fetch categories
            if let Ok(resp) = client.get("/api/categories").send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<Vec<CategoryInfo>>().await {
                        categories.set(data);
                    }
                }
            }

            // Fetch tags
            if let Ok(resp) = client.get("/api/tags").send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<Vec<TagInfo>>().await {
                        tags.set(data);
                    }
                }
            }

            // Fetch languages
            if let Ok(resp) = client.get("/api/languages").send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<Vec<LanguageInfo>>().await {
                        languages.set(data);
                    }
                }
            }

            // Fetch licenses
            if let Ok(resp) = client.get("/api/licenses").send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<Vec<LicenseInfo>>().await {
                        licenses.set(data);
                    }
                }
            }
        });
    });

    let is_editing = editing_link_id().is_some();

    rsx! {
        div { class: "page-container",
            Navbar {}
            div { class: "links-page",
                div { class: "links-header",
                    h1 { class: "links-title", "My Links" }
                    div { class: "links-actions",
                        if !show_form() {
                            select {
                                class: "status-filter",
                                value: "{status_filter()}",
                                onchange: move |evt| {
                                    status_filter.set(evt.value().clone());
                                },
                                option { value: "all", "All Links" }
                                option { value: "active", "Active Only" }
                                option { value: "archived", "Archived" }
                                option { value: "inaccessible", "Inaccessible" }
                                option { value: "repo_unavailable", "Repo Unavailable" }
                            }
                            button {
                                class: if selection_mode() { "btn btn-secondary" } else { "btn btn-secondary" },
                                onclick: move |_| {
                                    selection_mode.set(!selection_mode());
                                    if !selection_mode() {
                                        selected_ids.set(HashSet::new());
                                    }
                                },
                                if selection_mode() { "Cancel Selection" } else { "Select Multiple" }
                            }
                            button {
                                class: "btn btn-secondary",
                                onclick: move |_| {
                                    spawn(async move {
                                        let client = reqwest::Client::new();
                                        match client.get("/api/links/export").send().await {
                                            Ok(resp) => {
                                                if resp.status().is_success() {
                                                    match resp.text().await {
                                                        Ok(data) => {
                                                            // Create download using web_sys
                                                            #[cfg(target_arch = "wasm32")]
                                                            {
                                                                use wasm_bindgen::JsCast;
                                                                let window = web_sys::window().expect("no window");
                                                                let document = window.document().expect("no document");

                                                                let blob_parts = js_sys::Array::new();
                                                                blob_parts.push(&wasm_bindgen::JsValue::from_str(&data));

                                                                let mut blob_options = web_sys::BlobPropertyBag::new();
                                                                blob_options.type_("application/json");

                                                                if let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&blob_parts, &blob_options) {
                                                                    let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();

                                                                    if let Some(link_element) = document.create_element("a").ok() {
                                                                        let link = link_element.dyn_into::<web_sys::HtmlAnchorElement>().unwrap();
                                                                        link.set_href(&url);
                                                                        link.set_download("rusty-links-export.json");
                                                                        link.click();

                                                                        web_sys::Url::revoke_object_url(&url).ok();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            error.set(Some(format!("Failed to export: {}", e)));
                                                        }
                                                    }
                                                } else {
                                                    error.set(Some("Export failed".to_string()));
                                                }
                                            }
                                            Err(e) => {
                                                error.set(Some(format!("Network error: {}", e)));
                                            }
                                        }
                                    });
                                },
                                "Export Links"
                            }
                            label {
                                class: "btn btn-secondary file-upload-btn",
                                r#for: "import-file",
                                "Import Links"
                            }
                            input {
                                id: "import-file",
                                r#type: "file",
                                accept: ".json",
                                style: "display: none;",
                                onchange: move |evt| {
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        use wasm_bindgen::JsCast;
                                        use wasm_bindgen_futures::JsFuture;

                                        spawn(async move {
                                            let window = web_sys::window().expect("no window");
                                            let document = window.document().expect("no document");

                                            if let Some(input) = document.get_element_by_id("import-file") {
                                                let input: web_sys::HtmlInputElement = input.dyn_into().unwrap();
                                                if let Some(files) = input.files() {
                                                    if let Some(file) = files.get(0) {
                                                        match JsFuture::from(file.text()).await {
                                                            Ok(text_value) => {
                                                                let text = text_value.as_string().unwrap();

                                                                // Send to import endpoint
                                                                let client = reqwest::Client::new();
                                                                match client.post("/api/links/import")
                                                                    .header("Content-Type", "application/json")
                                                                    .body(text)
                                                                    .send()
                                                                    .await
                                                                {
                                                                    Ok(resp) => {
                                                                        if resp.status().is_success() {
                                                                            fetch_links();
                                                                            error.set(Some("Import completed successfully!".to_string()));
                                                                        } else {
                                                                            error.set(Some("Import failed".to_string()));
                                                                        }
                                                                    }
                                                                    Err(e) => {
                                                                        error.set(Some(format!("Import error: {}", e)));
                                                                    }
                                                                }

                                                                // Reset file input
                                                                input.set_value("");
                                                            }
                                                            Err(_) => {
                                                                error.set(Some("Failed to read file".to_string()));
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        });
                                    }
                                },
                            }
                            button {
                                class: "btn btn-primary",
                                onclick: move |_| {
                                    form_url.set(String::new());
                                    form_title.set(String::new());
                                    form_description.set(String::new());
                                    form_status.set("active".to_string());
                                    form_categories.set(Vec::new());
                                    form_tags.set(Vec::new());
                                    form_languages.set(Vec::new());
                                    form_licenses.set(Vec::new());
                                    form_error.set(None);
                                    editing_link_id.set(None);
                                    show_form.set(true);
                                },
                                "+ Add Link"
                            }
                        }
                    }
                }

                // Link Form (Create/Edit)
                if show_form() {
                    LinkForm {
                        is_editing: is_editing,
                        form_url: form_url,
                        form_title: form_title,
                        form_description: form_description,
                        form_status: form_status,
                        form_categories: form_categories,
                        form_tags: form_tags,
                        form_languages: form_languages,
                        form_licenses: form_licenses,
                        form_loading: form_loading,
                        form_scraping: form_scraping,
                        form_error: form_error,
                        editing_link_id: editing_link_id,
                        show_form: show_form,
                        links: links,
                    }
                }

                // Search bar
                if !show_form() {
                    div { class: "search-container",
                        input {
                            class: "search-input",
                            r#type: "text",
                            placeholder: "Search links by title, description, URL...",
                            value: "{search_query()}",
                            oninput: move |evt| {
                                let new_value = evt.value();
                                search_query.set(new_value);
                                current_page.set(1);
                                fetch_links();
                            },
                        }
                        if !search_query().is_empty() {
                            button {
                                class: "search-clear",
                                onclick: move |_| {
                                    search_query.set(String::new());
                                    current_page.set(1);
                                    fetch_links();
                                },
                                title: "Clear search",
                                "✕"
                            }
                        }
                    }

                    // Filter toggle and panel
                    div { class: "filter-section",
                        button {
                            class: "btn btn-secondary filter-toggle-btn",
                            onclick: move |_| show_filters.set(!show_filters()),
                            if show_filters() { "▼ Hide Filters" } else { "▶ Show Filters" }
                        }

                        if show_filters() {
                            div { class: "filter-panel",
                                // Status filter
                                div { class: "filter-group",
                                    label { "Status" }
                                    select {
                                        value: "{filters().status.clone().unwrap_or_default()}",
                                        onchange: move |evt| {
                                            let mut f = filters();
                                            let value = evt.value();
                                            f.status = if value.is_empty() { None } else { Some(value) };
                                            filters.set(f);
                                            current_page.set(1);
                                            fetch_links();
                                        },
                                        option { value: "", "All statuses" }
                                        option { value: "active", "Active" }
                                        option { value: "archived", "Archived" }
                                        option { value: "inaccessible", "Inaccessible" }
                                        option { value: "repo_unavailable", "Repo Unavailable" }
                                    }
                                }

                                // Category filter
                                div { class: "filter-group",
                                    label { "Category" }
                                    select {
                                        value: "{filters().category_id.map(|id| id.to_string()).unwrap_or_default()}",
                                        onchange: move |evt| {
                                            let mut f = filters();
                                            let value = evt.value();
                                            f.category_id = if value.is_empty() { None } else { value.parse().ok() };
                                            filters.set(f);
                                            current_page.set(1);
                                            fetch_links();
                                        },
                                        option { value: "", "All categories" }
                                        for cat in categories() {
                                            option { value: "{cat.id}", "{cat.name}" }
                                        }
                                    }
                                }

                                // Tag filter
                                div { class: "filter-group",
                                    label { "Tag" }
                                    select {
                                        value: "{filters().tag_id.map(|id| id.to_string()).unwrap_or_default()}",
                                        onchange: move |evt| {
                                            let mut f = filters();
                                            let value = evt.value();
                                            f.tag_id = if value.is_empty() { None } else { value.parse().ok() };
                                            filters.set(f);
                                            current_page.set(1);
                                            fetch_links();
                                        },
                                        option { value: "", "All tags" }
                                        for tag in tags() {
                                            option { value: "{tag.id}", "{tag.name}" }
                                        }
                                    }
                                }

                                // Language filter
                                div { class: "filter-group",
                                    label { "Language" }
                                    select {
                                        value: "{filters().language_id.map(|id| id.to_string()).unwrap_or_default()}",
                                        onchange: move |evt| {
                                            let mut f = filters();
                                            let value = evt.value();
                                            f.language_id = if value.is_empty() { None } else { value.parse().ok() };
                                            filters.set(f);
                                            current_page.set(1);
                                            fetch_links();
                                        },
                                        option { value: "", "All languages" }
                                        for lang in languages() {
                                            option { value: "{lang.id}", "{lang.name}" }
                                        }
                                    }
                                }

                                // License filter
                                div { class: "filter-group",
                                    label { "License" }
                                    select {
                                        value: "{filters().license_id.map(|id| id.to_string()).unwrap_or_default()}",
                                        onchange: move |evt| {
                                            let mut f = filters();
                                            let value = evt.value();
                                            f.license_id = if value.is_empty() { None } else { value.parse().ok() };
                                            filters.set(f);
                                            current_page.set(1);
                                            fetch_links();
                                        },
                                        option { value: "", "All licenses" }
                                        for lic in licenses() {
                                            option { value: "{lic.id}", "{lic.name}" }
                                        }
                                    }
                                }

                                // GitHub only checkbox
                                div { class: "filter-group filter-group-checkbox",
                                    label {
                                        input {
                                            r#type: "checkbox",
                                            checked: filters().is_github.unwrap_or(false),
                                            onchange: move |evt| {
                                                let mut f = filters();
                                                f.is_github = if evt.checked() { Some(true) } else { None };
                                                filters.set(f);
                                                current_page.set(1);
                                                fetch_links();
                                            },
                                        }
                                        " GitHub repositories only"
                                    }
                                }

                                // Clear filters button
                                button {
                                    class: "btn btn-secondary",
                                    onclick: move |_| {
                                        filters.set(Filters::default());
                                        current_page.set(1);
                                        fetch_links();
                                    },
                                    "Clear All Filters"
                                }
                            }
                        }
                    }

                    // Sort controls
                    div { class: "sort-container",
                        label { class: "sort-label", "Sort by:" }
                        select {
                            class: "sort-select",
                            value: "{sort_by()}",
                            onchange: move |evt| {
                                sort_by.set(evt.value());
                                current_page.set(1);
                                fetch_links();
                            },
                            option { value: "created_at", "Date Added" }
                            option { value: "updated_at", "Last Updated" }
                            option { value: "title", "Title" }
                            option { value: "github_stars", "GitHub Stars" }
                            option { value: "status", "Status" }
                        }
                        button {
                            class: "btn-icon sort-order-btn",
                            title: if sort_order() == "desc" { "Descending" } else { "Ascending" },
                            onclick: move |_| {
                                let new_order = if sort_order() == "desc" { "asc" } else { "desc" };
                                sort_order.set(new_order.to_string());
                                current_page.set(1);
                                fetch_links();
                            },
                            if sort_order() == "desc" { "↓" } else { "↑" }
                        }
                    }

                    // Search results info
                    if !search_query().is_empty() && !loading() {
                        div { class: "search-results-info",
                            "{links().len()} result(s) for \"{search_query()}\""
                        }
                    }

                    // Bulk action bar
                    if selection_mode() && !selected_ids().is_empty() {
                        div { class: "bulk-action-bar",
                            span { class: "bulk-selection-count", "{selected_ids().len()} selected" }

                            button {
                                class: "btn btn-danger btn-sm",
                                onclick: move |_| {
                                    let ids: Vec<String> = selected_ids().iter().cloned().collect();
                                    spawn(async move {
                                        let client = reqwest::Client::new();
                                        let body = serde_json::json!({"link_ids": ids});
                                        let response = client
                                            .post("/api/links/bulk/delete")
                                            .json(&body)
                                            .send()
                                            .await;

                                        match response {
                                            Ok(resp) => {
                                                if resp.status().is_success() || resp.status().as_u16() == 204 {
                                                    // Remove deleted links from list
                                                    let mut current = links();
                                                    let ids_set: HashSet<String> = ids.iter().cloned().collect();
                                                    current.retain(|l| !ids_set.contains(&l.id));
                                                    links.set(current);
                                                    selected_ids.set(HashSet::new());
                                                } else {
                                                    error.set(Some("Failed to delete selected links".to_string()));
                                                }
                                            }
                                            Err(e) => {
                                                error.set(Some(format!("Network error: {}", e)));
                                            }
                                        }
                                    });
                                },
                                "Delete Selected"
                            }

                            select {
                                class: "bulk-select",
                                onchange: move |evt| {
                                    let value = evt.value();
                                    if !value.is_empty() {
                                        if let Ok(category_id) = value.parse::<Uuid>() {
                                            let ids: Vec<String> = selected_ids().iter().cloned().collect();
                                            spawn(async move {
                                                let client = reqwest::Client::new();
                                                let body = serde_json::json!({
                                                    "link_ids": ids,
                                                    "category_id": category_id,
                                                    "action": "add"
                                                });
                                                let response = client
                                                    .post("/api/links/bulk/categories")
                                                    .json(&body)
                                                    .send()
                                                    .await;

                                                match response {
                                                    Ok(resp) => {
                                                        if resp.status().is_success() {
                                                            // Refresh links to show updated categories
                                                            fetch_links();
                                                        } else {
                                                            error.set(Some("Failed to add category to selected links".to_string()));
                                                        }
                                                    }
                                                    Err(e) => {
                                                        error.set(Some(format!("Network error: {}", e)));
                                                    }
                                                }
                                            });
                                        }
                                    }
                                },
                                option { value: "", "Add to category..." }
                                for cat in categories() {
                                    option { value: "{cat.id}", "{cat.name}" }
                                }
                            }

                            select {
                                class: "bulk-select",
                                onchange: move |evt| {
                                    let value = evt.value();
                                    if !value.is_empty() {
                                        if let Ok(tag_id) = value.parse::<Uuid>() {
                                            let ids: Vec<String> = selected_ids().iter().cloned().collect();
                                            spawn(async move {
                                                let client = reqwest::Client::new();
                                                let body = serde_json::json!({
                                                    "link_ids": ids,
                                                    "tag_id": tag_id,
                                                    "action": "add"
                                                });
                                                let response = client
                                                    .post("/api/links/bulk/tags")
                                                    .json(&body)
                                                    .send()
                                                    .await;

                                                match response {
                                                    Ok(resp) => {
                                                        if resp.status().is_success() {
                                                            // Refresh links to show updated tags
                                                            fetch_links();
                                                        } else {
                                                            error.set(Some("Failed to add tag to selected links".to_string()));
                                                        }
                                                    }
                                                    Err(e) => {
                                                        error.set(Some(format!("Network error: {}", e)));
                                                    }
                                                }
                                            });
                                        }
                                    }
                                },
                                option { value: "", "Add to tag..." }
                                for tag in tags() {
                                    option { value: "{tag.id}", "{tag.name}" }
                                }
                            }
                        }
                    }

                    // Selection controls
                    if selection_mode() && !loading() && !links().is_empty() {
                        div { class: "selection-controls",
                            label { class: "select-all-label",
                                input {
                                    r#type: "checkbox",
                                    checked: !links().is_empty() && selected_ids().len() == links().len(),
                                    onchange: move |evt| {
                                        if evt.checked() {
                                            let all_ids: HashSet<String> = links().iter().map(|l| l.id.clone()).collect();
                                            selected_ids.set(all_ids);
                                        } else {
                                            selected_ids.set(HashSet::new());
                                        }
                                    },
                                }
                                " Select All ({selected_ids().len()} selected)"
                            }
                        }
                    }
                }

                div { class: "links-container",
                    if loading() {
                        div { class: "loading-state",
                            p { "Loading..." }
                        }
                    } else if let Some(err) = error() {
                        div { class: "error-state",
                            p { class: "message message-error", "{err}" }
                        }
                    } else if links().is_empty() {
                        div { class: "empty-state",
                            if !search_query().is_empty() {
                                p { class: "empty-title", "No results found for \"{search_query()}\"" }
                                p { class: "empty-subtitle", "Try different search terms or clear the search" }
                            } else {
                                p { class: "empty-title", "No links yet" }
                                p { class: "empty-subtitle", "Add your first link to get started!" }
                            }
                        }
                    } else {
                        {
                            let filtered_links: Vec<Link> = links().into_iter()
                                .filter(|link| {
                                    let filter = status_filter();
                                    filter == "all" || link.status == filter
                                })
                                .collect();

                            if filtered_links.is_empty() {
                                rsx! {
                                    div { class: "empty-state",
                                        p { class: "empty-title", "No links match the selected filter" }
                                        p { class: "empty-subtitle", "Try changing the filter to see more links" }
                                    }
                                }
                            } else {
                                rsx! {
                                    div { class: "links-list",
                                        for link in filtered_links {
                                            LinkCard {
                                                key: "{link.id}",
                                                link: link.clone(),
                                                deleting_id: deleting_id,
                                                refreshing_id: refreshing_id,
                                                links: links,
                                                error: error,
                                                form_url: form_url,
                                                form_title: form_title,
                                                form_description: form_description,
                                                form_status: form_status,
                                                form_categories: form_categories,
                                                form_tags: form_tags,
                                                form_languages: form_languages,
                                                form_licenses: form_licenses,
                                                form_error: form_error,
                                                editing_link_id: editing_link_id,
                                                show_form: show_form,
                                                selection_mode: selection_mode,
                                                selected_ids: selected_ids,
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Pagination
                    if !loading() && !links().is_empty() && total_pages() > 1 {
                        Pagination {
                            current_page: current_page(),
                            total_pages: total_pages(),
                            on_page_change: move |page| {
                                current_page.set(page);
                                fetch_links();
                            },
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct ScrapeResponse {
    title: Option<String>,
    description: Option<String>,
    favicon: Option<String>,
}

#[component]
fn LinkForm(
    is_editing: bool,
    mut form_url: Signal<String>,
    mut form_title: Signal<String>,
    mut form_description: Signal<String>,
    mut form_status: Signal<String>,
    mut form_categories: Signal<Vec<Uuid>>,
    mut form_tags: Signal<Vec<Uuid>>,
    mut form_languages: Signal<Vec<Uuid>>,
    mut form_licenses: Signal<Vec<Uuid>>,
    mut form_loading: Signal<bool>,
    mut form_scraping: Signal<bool>,
    mut form_error: Signal<Option<String>>,
    mut editing_link_id: Signal<Option<String>>,
    mut show_form: Signal<bool>,
    mut links: Signal<Vec<Link>>,
) -> Element {
    // Track if user has manually edited title/description
    let mut title_touched = use_signal(|| false);
    let mut description_touched = use_signal(|| false);

    let scrape_url = move |_| {
        let url = form_url();
        if url.trim().is_empty() || editing_link_id().is_some() || form_scraping() {
            return;
        }

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return;
        }

        spawn(async move {
            form_scraping.set(true);
            let client = reqwest::Client::new();
            let result = client
                .post("/api/scrape")
                .json(&serde_json::json!({ "url": url }))
                .send()
                .await;

            form_scraping.set(false);

            if let Ok(resp) = result {
                if resp.status().is_success() {
                    if let Ok(scraped) = resp.json::<ScrapeResponse>().await {
                        // Only auto-fill if user hasn't manually edited
                        if !title_touched() {
                            if let Some(title) = scraped.title {
                                form_title.set(title);
                            }
                        }
                        if !description_touched() {
                            if let Some(desc) = scraped.description {
                                form_description.set(desc);
                            }
                        }
                    }
                }
            }
        });
    };

    let on_submit = move |_| {
        let url_val = form_url();
        let title_val = form_title();
        let desc_val = form_description();
        let status_val = form_status();
        let categories_val = form_categories();
        let tags_val = form_tags();
        let languages_val = form_languages();
        let licenses_val = form_licenses();
        let edit_id = editing_link_id();

        // Validation for create
        if edit_id.is_none() {
            if url_val.trim().is_empty() {
                form_error.set(Some("URL is required".to_string()));
                return;
            }
            if !url_val.starts_with("http://") && !url_val.starts_with("https://") {
                form_error.set(Some("URL must start with http:// or https://".to_string()));
                return;
            }
        }

        let is_update = edit_id.is_some();
        let edit_id_clone = edit_id.clone();

        spawn(async move {
            form_loading.set(true);
            form_error.set(None);

            let client = reqwest::Client::new();

            let response = if let Some(ref id) = edit_id_clone {
                let request = UpdateLinkRequest {
                    title: if title_val.trim().is_empty() { None } else { Some(title_val) },
                    description: if desc_val.trim().is_empty() { None } else { Some(desc_val) },
                    status: Some(status_val),
                };
                client.put(&format!("/api/links/{}", id)).json(&request).send().await
            } else {
                let request = CreateLinkRequest {
                    url: url_val,
                    title: if title_val.trim().is_empty() { None } else { Some(title_val) },
                    description: if desc_val.trim().is_empty() { None } else { Some(desc_val) },
                };
                client.post("/api/links").json(&request).send().await
            };

            form_loading.set(false);

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        match resp.json::<Link>().await {
                            Ok(mut updated_link) => {
                                let link_id = updated_link.id.clone();

                                // Save categories
                                for cat_id in &categories_val {
                                    let _ = client
                                        .post(&format!("/api/links/{}/categories", link_id))
                                        .json(&serde_json::json!({ "category_id": cat_id }))
                                        .send()
                                        .await;
                                }

                                // Save tags
                                for tag_id in &tags_val {
                                    let _ = client
                                        .post(&format!("/api/links/{}/tags", link_id))
                                        .json(&serde_json::json!({ "tag_id": tag_id }))
                                        .send()
                                        .await;
                                }

                                // Save languages
                                for lang_id in &languages_val {
                                    let _ = client
                                        .post(&format!("/api/links/{}/languages", link_id))
                                        .json(&serde_json::json!({ "language_id": lang_id }))
                                        .send()
                                        .await;
                                }

                                // Save licenses
                                for lic_id in &licenses_val {
                                    let _ = client
                                        .post(&format!("/api/links/{}/licenses", link_id))
                                        .json(&serde_json::json!({ "license_id": lic_id }))
                                        .send()
                                        .await;
                                }

                                // Refetch link to get updated metadata
                                if let Ok(resp) = client.get("/api/links").send().await {
                                    if let Ok(all_links) = resp.json::<Vec<Link>>().await {
                                        if let Some(refreshed) = all_links.iter().find(|l| l.id == link_id) {
                                            updated_link = refreshed.clone();
                                        }
                                    }
                                }

                                let mut current = links();
                                if is_update {
                                    if let Some(pos) = current.iter().position(|l| l.id == updated_link.id) {
                                        current[pos] = updated_link;
                                    }
                                } else {
                                    current.insert(0, updated_link);
                                }
                                links.set(current);
                                form_url.set(String::new());
                                form_title.set(String::new());
                                form_description.set(String::new());
                                form_status.set("active".to_string());
                                form_categories.set(Vec::new());
                                form_tags.set(Vec::new());
                                form_languages.set(Vec::new());
                                form_licenses.set(Vec::new());
                                form_error.set(None);
                                editing_link_id.set(None);
                                show_form.set(false);
                            }
                            Err(e) => {
                                form_error.set(Some(format!("Failed to parse response: {}", e)));
                            }
                        }
                    } else {
                        let error_text = resp.text().await.unwrap_or_else(|_| "Operation failed".to_string());
                        form_error.set(Some(error_text));
                    }
                }
                Err(e) => {
                    form_error.set(Some(format!("Network error: {}", e)));
                }
            }
        });
    };

    rsx! {
        div { class: "link-form",
            h2 { class: "form-title",
                if is_editing { "Edit Link" } else { "Add New Link" }
            }

            if let Some(err) = form_error() {
                div { class: "message message-error", "{err}" }
            }

            if !is_editing {
                div { class: "form-group",
                    label { class: "form-label", r#for: "url", "URL *" }
                    input {
                        class: "form-input",
                        r#type: "url",
                        id: "url",
                        placeholder: "https://example.com",
                        value: "{form_url}",
                        disabled: form_loading(),
                        oninput: move |evt| form_url.set(evt.value()),
                        onblur: scrape_url,
                    }
                    if form_scraping() {
                        span { class: "scraping-indicator", "Fetching metadata..." }
                    }
                }
            }

            div { class: "form-group",
                label { class: "form-label", r#for: "title", "Title" }
                input {
                    class: "form-input",
                    r#type: "text",
                    id: "title",
                    placeholder: "Optional title",
                    value: "{form_title}",
                    disabled: form_loading(),
                    oninput: move |evt| {
                        title_touched.set(true);
                        form_title.set(evt.value());
                    },
                }
            }

            div { class: "form-group",
                label { class: "form-label", r#for: "description", "Description" }
                textarea {
                    class: "form-input form-textarea",
                    id: "description",
                    placeholder: "Optional description",
                    value: "{form_description}",
                    disabled: form_loading(),
                    oninput: move |evt| {
                        description_touched.set(true);
                        form_description.set(evt.value());
                    },
                }
            }

            if is_editing {
                div { class: "form-group",
                    label { class: "form-label", r#for: "status", "Status" }
                    select {
                        class: "form-input",
                        id: "status",
                        disabled: form_loading(),
                        value: "{form_status}",
                        onchange: move |evt| form_status.set(evt.value()),
                        option { value: "active", "Active" }
                        option { value: "archived", "Archived" }
                    }
                }
            }

            div { class: "form-group",
                label { class: "form-label", "Categories" }
                CategorySelect {
                    selected_ids: form_categories(),
                    on_change: move |ids| form_categories.set(ids),
                }
            }

            div { class: "form-group",
                label { class: "form-label", "Tags" }
                TagSelect {
                    selected_ids: form_tags(),
                    on_change: move |ids| form_tags.set(ids),
                }
            }

            div { class: "form-group",
                label { class: "form-label", "Languages" }
                LanguageSelect {
                    selected_ids: form_languages(),
                    on_change: move |ids| form_languages.set(ids),
                }
            }

            div { class: "form-group",
                label { class: "form-label", "License" }
                LicenseSelect {
                    selected_ids: form_licenses(),
                    on_change: move |ids| form_licenses.set(ids),
                }
            }

            div { class: "form-actions",
                button {
                    class: "btn btn-secondary",
                    r#type: "button",
                    disabled: form_loading(),
                    onclick: move |_| {
                        form_url.set(String::new());
                        form_title.set(String::new());
                        form_description.set(String::new());
                        form_status.set("active".to_string());
                        form_categories.set(Vec::new());
                        form_tags.set(Vec::new());
                        form_languages.set(Vec::new());
                        form_licenses.set(Vec::new());
                        form_error.set(None);
                        editing_link_id.set(None);
                        show_form.set(false);
                    },
                    "Cancel"
                }
                button {
                    class: "btn btn-primary",
                    r#type: "button",
                    disabled: form_loading(),
                    onclick: on_submit,
                    if form_loading() { "Saving..." } else { "Save" }
                }
            }
        }
    }
}

#[component]
fn LinkCard(
    link: Link,
    mut deleting_id: Signal<Option<String>>,
    mut refreshing_id: Signal<Option<String>>,
    mut links: Signal<Vec<Link>>,
    mut error: Signal<Option<String>>,
    mut form_url: Signal<String>,
    mut form_title: Signal<String>,
    mut form_description: Signal<String>,
    mut form_status: Signal<String>,
    mut form_categories: Signal<Vec<Uuid>>,
    mut form_tags: Signal<Vec<Uuid>>,
    mut form_languages: Signal<Vec<Uuid>>,
    mut form_licenses: Signal<Vec<Uuid>>,
    mut form_error: Signal<Option<String>>,
    mut editing_link_id: Signal<Option<String>>,
    mut show_form: Signal<bool>,
    selection_mode: Signal<bool>,
    mut selected_ids: Signal<HashSet<String>>,
) -> Element {
    let display_title = link.title.clone().unwrap_or_else(|| link.url.clone());
    let description = link.description.clone().unwrap_or_default();
    let truncated_desc = if description.len() > 150 {
        format!("{}...", &description[..150])
    } else {
        description
    };

    let created_date = &link.created_at[..10];
    let refreshed_date = link.refreshed_at.as_ref().map(|r| &r[..10]);
    let is_deleting = deleting_id() == Some(link.id.clone());
    let is_refreshing = refreshing_id() == Some(link.id.clone());
    let link_for_edit = link.clone();
    let link_id_for_delete = link.id.clone();
    let link_id_for_refresh = link.id.clone();

    // Determine status icon and label
    let status_icon = match link.status.as_str() {
        "active" => "●",
        "archived" => "📦",
        "inaccessible" => "⚠️",
        "repo_unavailable" => "⚠️",
        _ => "●",
    };
    let status_label = match link.status.as_str() {
        "active" => "Active",
        "archived" => "Archived",
        "inaccessible" => "Inaccessible",
        "repo_unavailable" => "Repo Unavailable",
        _ => link.status.as_str(),
    };
    let status_class = match link.status.as_str() {
        "active" => "status-active",
        "archived" => "status-archived",
        "inaccessible" => "status-inaccessible",
        "repo_unavailable" => "status-repo-unavailable",
        _ => "status-unknown",
    };

    let link_id = link.id.clone();
    let is_selected = selected_ids().contains(&link.id);
    let card_class = if is_selected && selection_mode() {
        format!("link-card link-card-{} link-card-selected", link.status)
    } else {
        format!("link-card link-card-{}", link.status)
    };

    rsx! {
        div {
            class: "{card_class}",
            // Selection checkbox
            if selection_mode() {
                div { class: "link-card-checkbox",
                    input {
                        r#type: "checkbox",
                        checked: is_selected,
                        onchange: move |evt| {
                            let mut ids = selected_ids();
                            if evt.checked() {
                                ids.insert(link_id.clone());
                            } else {
                                ids.remove(&link_id);
                            }
                            selected_ids.set(ids);
                        },
                    }
                }
            }
            div { class: "link-card-header",
                h3 {
                    class: if link.status == "inaccessible" || link.status == "repo_unavailable" {
                        "link-title link-title-unavailable"
                    } else {
                        "link-title"
                    },
                    a { href: "{link.url}", target: "_blank", rel: "noopener noreferrer",
                        "{display_title}"
                    }
                }
                div { class: "link-header-right",
                    span {
                        class: "link-status {status_class}",
                        span { class: "status-indicator", "{status_icon}" }
                        span { class: "status-label", "{status_label}" }
                    }
                    div { class: "link-actions",
                        button {
                            class: "btn-icon",
                            title: "Edit",
                            disabled: is_deleting || is_refreshing,
                            onclick: move |_| {
                                let l = link_for_edit.clone();
                                form_url.set(l.url.clone());
                                form_title.set(l.title.unwrap_or_default());
                                form_description.set(l.description.unwrap_or_default());
                                form_status.set(l.status.clone());
                                form_categories.set(l.categories.iter().map(|c| c.id).collect());
                                form_tags.set(l.tags.iter().map(|t| t.id).collect());
                                form_languages.set(l.languages.iter().map(|l| l.id).collect());
                                form_licenses.set(l.licenses.iter().map(|l| l.id).collect());
                                form_error.set(None);
                                editing_link_id.set(Some(l.id));
                                show_form.set(true);
                            },
                            "✏️"
                        }
                        button {
                            class: "btn-icon btn-refresh",
                            title: "Refresh metadata",
                            disabled: is_refreshing,
                            onclick: move |_| {
                                let id = link_id_for_refresh.clone();
                                refreshing_id.set(Some(id.clone()));

                                spawn(async move {
                                    let client = reqwest::Client::new();
                                    let response = client.post(&format!("/api/links/{}/refresh", id)).send().await;

                                    match response {
                                        Ok(resp) => {
                                            if resp.status().is_success() {
                                                match resp.json::<Link>().await {
                                                    Ok(updated_link) => {
                                                        // Update the link in the list
                                                        let mut current = links();
                                                        if let Some(pos) = current.iter().position(|l| l.id == id) {
                                                            current[pos] = updated_link;
                                                            links.set(current);
                                                        }
                                                    }
                                                    Err(e) => {
                                                        error.set(Some(format!("Failed to parse refreshed link: {}", e)));
                                                    }
                                                }
                                            } else {
                                                error.set(Some("Failed to refresh link".to_string()));
                                            }
                                        }
                                        Err(e) => {
                                            error.set(Some(format!("Network error: {}", e)));
                                        }
                                    }

                                    refreshing_id.set(None);
                                });
                            },
                            if is_refreshing { "..." } else { "🔄" }
                        }
                        // Add "Mark as Active" button for inaccessible links
                        if link.status == "inaccessible" || link.status == "repo_unavailable" {
                            button {
                                class: "btn-icon btn-success",
                                title: "Mark as Active",
                                disabled: is_deleting || is_refreshing,
                                onclick: move |_| {
                                    let id = link.id.clone();
                                    spawn(async move {
                                        let client = reqwest::Client::new();
                                        let body = serde_json::json!({"status": "active"});
                                        let response = client
                                            .patch(&format!("/api/links/{}", id))
                                            .header("Content-Type", "application/json")
                                            .body(body.to_string())
                                            .send()
                                            .await;

                                        match response {
                                            Ok(resp) => {
                                                if resp.status().is_success() {
                                                    match resp.json::<Link>().await {
                                                        Ok(updated_link) => {
                                                            let mut current = links();
                                                            if let Some(pos) = current.iter().position(|l| l.id == id) {
                                                                current[pos] = updated_link;
                                                                links.set(current);
                                                            }
                                                        }
                                                        Err(e) => {
                                                            error.set(Some(format!("Failed to parse updated link: {}", e)));
                                                        }
                                                    }
                                                } else {
                                                    error.set(Some("Failed to update link status".to_string()));
                                                }
                                            }
                                            Err(e) => {
                                                error.set(Some(format!("Network error: {}", e)));
                                            }
                                        }
                                    });
                                },
                                "✓"
                            }
                        }
                        button {
                            class: "btn-icon btn-danger",
                            title: "Delete",
                            disabled: is_deleting || is_refreshing,
                            onclick: move |_| {
                                let id = link_id_for_delete.clone();
                                deleting_id.set(Some(id.clone()));

                                spawn(async move {
                                    let client = reqwest::Client::new();
                                    let response = client.delete(&format!("/api/links/{}", id)).send().await;

                                    deleting_id.set(None);

                                    match response {
                                        Ok(resp) => {
                                            if resp.status().is_success() || resp.status().as_u16() == 204 {
                                                let mut current = links();
                                                current.retain(|l| l.id != id);
                                                links.set(current);
                                            } else {
                                                error.set(Some("Failed to delete link".to_string()));
                                            }
                                        }
                                        Err(e) => {
                                            error.set(Some(format!("Network error: {}", e)));
                                        }
                                    }
                                });
                            },
                            if is_deleting { "..." } else { "🗑️" }
                        }
                    }
                }
            }

            div { class: "link-domain",
                "{link.domain}"
                if link.is_github_repo {
                    if let Some(stars) = link.github_stars {
                        span { class: "github-stars", " ⭐ {stars}" }
                    }
                }
            }

            if !truncated_desc.is_empty() {
                p { class: "link-description", "{truncated_desc}" }
            }

            MetadataBadges {
                categories: link.categories.clone(),
                tags: link.tags.clone(),
                languages: link.languages.clone(),
                licenses: link.licenses.clone(),
                is_github_repo: Some(link.is_github_repo),
                github_stars: link.github_stars,
                github_archived: link.github_archived,
            }

            div { class: "link-meta",
                span { class: "link-date", "Added {created_date}" }
                if let Some(ref_date) = refreshed_date {
                    span { class: "link-refreshed", " • Refreshed {ref_date}" }
                }
            }
        }
    }
}
