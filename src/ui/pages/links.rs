use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::ui::components::navbar::Navbar;
use crate::ui::components::category_select::CategorySelect;
use crate::ui::components::tag_select::TagSelect;
use crate::ui::components::language_select::LanguageSelect;
use crate::ui::components::license_select::LicenseSelect;
use crate::ui::components::metadata_badges::{MetadataBadges, CategoryInfo, TagInfo, LanguageInfo, LicenseInfo};

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct Link {
    id: String,
    url: String,
    domain: String,
    title: Option<String>,
    description: Option<String>,
    status: String,
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
    let mut form_loading = use_signal(|| false);
    let mut form_scraping = use_signal(|| false);
    let mut form_error = use_signal(|| Option::<String>::None);

    // Delete state
    let mut deleting_id = use_signal(|| Option::<String>::None);

    // Refresh state
    let mut refreshing_id = use_signal(|| Option::<String>::None);

    // Fetch links on mount
    use_effect(move || {
        spawn(async move {
            let client = reqwest::Client::new();
            let response = client.get("/api/links").send().await;

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        match resp.json::<Vec<Link>>().await {
                            Ok(data) => {
                                links.set(data);
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
    });

    let is_editing = editing_link_id().is_some();

    rsx! {
        div { class: "page-container",
            Navbar {}
            div { class: "links-page",
                div { class: "links-header",
                    h1 { class: "links-title", "My Links" }
                    if !show_form() {
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
                            p { class: "empty-title", "No links yet" }
                            p { class: "empty-subtitle", "Add your first link to get started!" }
                        }
                    } else {
                        div { class: "links-list",
                            for link in links() {
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
                                }
                            }
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

    rsx! {
        div { class: "link-card",
            div { class: "link-card-header",
                h3 { class: "link-title",
                    a { href: "{link.url}", target: "_blank", rel: "noopener noreferrer",
                        "{display_title}"
                    }
                }
                div { class: "link-header-right",
                    span { class: "link-status status-{link.status}", "{link.status}" }
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
