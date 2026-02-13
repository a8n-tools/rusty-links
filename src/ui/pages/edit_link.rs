use crate::ui::components::category_select::CategorySelect;
use crate::ui::components::language_select::LanguageSelect;
use crate::ui::components::license_select::LicenseSelect;
use crate::ui::components::metadata_badges::{CategoryInfo, LanguageInfo, LicenseInfo, TagInfo};
use crate::ui::components::modal::ConfirmDialog;
use crate::ui::components::navbar::Navbar;
use crate::ui::components::tag_select::TagSelect;
use crate::ui::http;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct LinkDetails {
    pub id: Uuid,
    pub url: String,
    pub domain: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub logo: Option<String>,
    pub status: String,
    pub source_code_url: Option<String>,
    pub documentation_url: Option<String>,
    pub notes: Option<String>,
    pub github_stars: Option<i32>,
    pub github_archived: Option<bool>,
    pub github_last_commit: Option<String>,
    pub is_github_repo: bool,
    pub created_at: String,
    pub updated_at: String,
    pub refreshed_at: Option<String>,
    #[serde(default)]
    pub categories: Vec<CategoryInfo>,
    #[serde(default)]
    pub tags: Vec<TagInfo>,
    #[serde(default)]
    pub languages: Vec<LanguageInfo>,
    #[serde(default)]
    pub licenses: Vec<LicenseInfo>,
}

#[derive(Debug, Clone, Serialize)]
struct UpdateLinkRequest {
    url: Option<String>,
    source_code_url: Option<String>,
    documentation_url: Option<String>,
    notes: Option<String>,
    category_ids: Option<Vec<Uuid>>,
    tag_ids: Option<Vec<Uuid>>,
    language_ids: Option<Vec<Uuid>>,
    license_ids: Option<Vec<Uuid>>,
}

async fn fetch_link(link_id: Uuid) -> Result<LinkDetails, String> {
    let url = format!("/api/links/{}", link_id);
    http::get(&url).await
}

async fn save_link(link_id: Uuid, form_data: UpdateLinkRequest) -> Result<LinkDetails, String> {
    let url = format!("/api/links/{}", link_id);
    http::put(&url, &form_data).await
}

async fn delete_link(link_id: Uuid) -> Result<(), String> {
    let url = format!("/api/links/{}", link_id);
    http::delete(&url).await
}

async fn refresh_metadata(link_id: Uuid) -> Result<LinkDetails, String> {
    let url = format!("/api/links/{}/refresh", link_id);
    let response = http::post_empty(&url).await?;
    response.json()
}

fn format_stars(stars: Option<i32>) -> String {
    match stars {
        Some(s) if s >= 1000 => format!("{:.1}k", s as f32 / 1000.0),
        Some(s) => s.to_string(),
        None => "-".to_string(),
    }
}

/// Format a datetime string to YYYY-MM-DD format
fn format_date(datetime: &str) -> String {
    // Handle ISO 8601 format like "2024-01-15T10:30:00Z" or "2024-01-15 10:30:00"
    if let Some(date_part) = datetime.split('T').next() {
        if date_part.len() >= 10 {
            return date_part[..10].to_string();
        }
    }
    // Try splitting by space for "2024-01-15 10:30:00" format
    if let Some(date_part) = datetime.split(' ').next() {
        if date_part.len() >= 10 {
            return date_part[..10].to_string();
        }
    }
    // Return as-is if we can't parse it
    datetime.to_string()
}

/// Get emoji and CSS class for a status
fn status_display(status: &str) -> (&'static str, &'static str) {
    match status.to_lowercase().as_str() {
        "active" => ("✅", "status-badge-active"),
        "pending" => ("⏳", "status-badge-pending"),
        "error" | "failed" => ("❌", "status-badge-error"),
        "archived" => ("📦", "status-badge-archived"),
        _ => ("📋", "status-badge-active"),
    }
}

#[component]
fn Section(title: String, children: Element) -> Element {
    rsx! {
        div { class: "edit-section",
            h3 { class: "edit-section-title", "{title}" }
            div { class: "edit-section-content",
                {children}
            }
        }
    }
}

#[component]
pub fn EditLinkPage(link_id: Uuid) -> Element {
    let nav = navigator();

    // Link data
    let mut link = use_signal(|| Option::<LinkDetails>::None);
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| Option::<String>::None);

    // Form state (editable fields)
    let mut form_url = use_signal(String::new);
    let mut form_source_code_url = use_signal(String::new);
    let mut form_documentation_url = use_signal(String::new);
    let mut form_notes = use_signal(String::new);
    let mut form_categories = use_signal(Vec::<Uuid>::new);
    let mut form_tags = use_signal(Vec::<Uuid>::new);
    let mut form_languages = use_signal(Vec::<Uuid>::new);
    let mut form_licenses = use_signal(Vec::<Uuid>::new);

    // Edit tracking
    let mut has_changes = use_signal(|| false);
    let mut saving = use_signal(|| false);
    let mut save_error = use_signal(|| Option::<String>::None);

    // Delete state
    let mut show_delete_confirm = use_signal(|| false);
    let mut deleting = use_signal(|| false);

    // Refresh state
    let mut refreshing = use_signal(|| false);

    // Unsaved changes warning
    let mut show_unsaved_warning = use_signal(|| false);
    let mut pending_navigation = use_signal(|| Option::<String>::None);

    // Fetch link on mount or when link_id changes
    use_effect(move || {
        spawn(async move {
            loading.set(true);
            error.set(None);

            match fetch_link(link_id).await {
                Ok(link_data) => {
                    // Initialize form with current values
                    form_url.set(link_data.url.clone());
                    form_source_code_url.set(link_data.source_code_url.clone().unwrap_or_default());
                    form_documentation_url
                        .set(link_data.documentation_url.clone().unwrap_or_default());
                    form_notes.set(link_data.notes.clone().unwrap_or_default());
                    form_categories.set(link_data.categories.iter().map(|c| c.id).collect());
                    form_tags.set(link_data.tags.iter().map(|t| t.id).collect());
                    form_languages.set(link_data.languages.iter().map(|l| l.id).collect());
                    form_licenses.set(link_data.licenses.iter().map(|l| l.id).collect());

                    link.set(Some(link_data));
                    has_changes.set(false);
                    loading.set(false);
                }
                Err(err) => {
                    error.set(Some(err));
                    loading.set(false);
                }
            }
        });
    });

    // Handle save
    let do_save = move || {
        let nav = nav;
        spawn(async move {
            saving.set(true);
            save_error.set(None);

            let form_data = UpdateLinkRequest {
                url: Some(form_url()),
                source_code_url: if form_source_code_url().is_empty() {
                    None
                } else {
                    Some(form_source_code_url())
                },
                documentation_url: if form_documentation_url().is_empty() {
                    None
                } else {
                    Some(form_documentation_url())
                },
                notes: if form_notes().is_empty() {
                    None
                } else {
                    Some(form_notes())
                },
                category_ids: Some(form_categories()),
                tag_ids: Some(form_tags()),
                language_ids: Some(form_languages()),
                license_ids: Some(form_licenses()),
            };

            match save_link(link_id, form_data).await {
                Ok(updated_link) => {
                    link.set(Some(updated_link));
                    has_changes.set(false);
                    saving.set(false);
                    // Navigate back to links list
                    nav.push("/links");
                }
                Err(err) => {
                    save_error.set(Some(err));
                    saving.set(false);
                }
            }
        });
    };

    // Handle delete
    let handle_delete = move |_| {
        spawn(async move {
            deleting.set(true);

            match delete_link(link_id).await {
                Ok(_) => {
                    show_delete_confirm.set(false);
                    // Navigate back to links list
                    nav.push("/links");
                }
                Err(err) => {
                    save_error.set(Some(err));
                    deleting.set(false);
                    show_delete_confirm.set(false);
                }
            }
        });
    };

    // Handle refresh
    let handle_refresh = move |_| {
        spawn(async move {
            refreshing.set(true);

            match refresh_metadata(link_id).await {
                Ok(updated_link) => {
                    link.set(Some(updated_link));
                    has_changes.set(true);
                    refreshing.set(false);
                }
                Err(err) => {
                    save_error.set(Some(err));
                    refreshing.set(false);
                }
            }
        });
    };

    // Handle back navigation
    let handle_back = move |_| {
        if has_changes() {
            pending_navigation.set(Some("/links".to_string()));
            show_unsaved_warning.set(true);
        } else {
            nav.push("/links");
        }
    };

    // Handle keyboard shortcuts
    let handle_keydown = move |evt: KeyboardEvent| {
        match evt.key() {
            Key::Escape => {
                if has_changes() {
                    pending_navigation.set(Some("/links".to_string()));
                    show_unsaved_warning.set(true);
                } else {
                    nav.push("/links");
                }
            }
            Key::Enter if evt.modifiers().ctrl() || evt.modifiers().meta() => {
                // Ctrl/Cmd + Enter: Save
                if has_changes() && !saving() {
                    do_save();
                }
            }
            _ => {}
        }
    };

    rsx! {
        div {
            class: "page-container",
            tabindex: "0",
            onkeydown: handle_keydown,

            Navbar {}

            div { class: "content-container",
                div { class: "page-header",
                    h1 { "Edit Link" }
                }

                if loading() {
                    div { class: "loading-container",
                        div { class: "spinner spinner-medium" }
                        p { "Loading link..." }
                    }
                } else if let Some(err) = error() {
                    div { class: "error-container",
                        p { class: "error-message", "Error: {err}" }
                        button {
                            class: "btn-secondary",
                            onclick: handle_back,
                            "Back to Links"
                        }
                    }
                } else if let Some(link_data) = link() {
                    div { class: "edit-link-page-content",
                        // Section 1: Basic Information
                        Section { title: "Basic Information".to_string(),
                            if let Some(logo_url) = link_data.logo.clone() {
                                div { class: "link-logo",
                                    img { src: "{logo_url}", alt: "Logo" }
                                }
                            }

                            div { class: "readonly-field",
                                label { "Title" }
                                div { class: "field-value", "{link_data.title.clone().unwrap_or(\"-\".to_string())}" }
                            }

                            div { class: "editable-field",
                                label { "URL" }
                                input {
                                    r#type: "url",
                                    value: "{form_url()}",
                                    oninput: move |evt| {
                                        form_url.set(evt.value());
                                        has_changes.set(true);
                                    }
                                }
                            }

                            div { class: "readonly-field",
                                label { "Description" }
                                div { class: "field-value", "{link_data.description.clone().unwrap_or(\"-\".to_string())}" }
                            }
                        }

                        // Section 2: Links
                        Section { title: "Related Links".to_string(),
                            div { class: "editable-field",
                                label { "Source Code URL" }
                                input {
                                    r#type: "url",
                                    placeholder: "https://github.com/...",
                                    value: "{form_source_code_url()}",
                                    oninput: move |evt| {
                                        form_source_code_url.set(evt.value());
                                        has_changes.set(true);
                                    }
                                }
                            }

                            div { class: "editable-field",
                                label { "Documentation URL" }
                                input {
                                    r#type: "url",
                                    placeholder: "https://docs.example.com/...",
                                    value: "{form_documentation_url()}",
                                    oninput: move |evt| {
                                        form_documentation_url.set(evt.value());
                                        has_changes.set(true);
                                    }
                                }
                            }
                        }

                        // Section 3: Categorization
                        Section { title: "Categorization".to_string(),
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
                        }

                        // Section 4: GitHub Information (conditional)
                        {
                            let has_github_source = link_data.source_code_url
                                .as_ref()
                                .map(|url| url.contains("github.com"))
                                .unwrap_or(false);

                            if link_data.is_github_repo || has_github_source {
                                rsx! {
                                    Section { title: "GitHub Information".to_string(),
                                        div { class: "github-stats",
                                            span { class: "github-stat",
                                                "⭐ {format_stars(link_data.github_stars)}"
                                            }
                                            span { class: "github-stat",
                                                if link_data.github_archived.unwrap_or(false) {
                                                    "📦 Archived"
                                                } else {
                                                    "✅ Active"
                                                }
                                            }
                                        }

                                        div { class: "readonly-field",
                                            label { "Last Commit" }
                                            div { class: "field-value", "{link_data.github_last_commit.clone().unwrap_or(\"-\".to_string())}" }
                                        }

                                        button {
                                            class: "btn-refresh",
                                            disabled: refreshing(),
                                            onclick: handle_refresh,
                                            if refreshing() {
                                                span { class: "refresh-icon spinning", "↻" }
                                                " Refreshing..."
                                            } else {
                                                span { class: "refresh-icon", "↻" }
                                                " Refresh Metadata"
                                            }
                                        }
                                    }
                                }
                            } else {
                                rsx! {}
                            }
                        }

                        // Section 5: Notes
                        Section { title: "Notes".to_string(),
                            textarea {
                                class: "notes-textarea",
                                value: "{form_notes()}",
                                placeholder: "Add your notes here (Markdown supported)...",
                                oninput: move |evt| {
                                    form_notes.set(evt.value());
                                    has_changes.set(true);
                                }
                            }
                        }

                        // Section 6: Metadata
                        Section { title: "Metadata".to_string(),
                            {
                                let (status_emoji, status_class) = status_display(&link_data.status);
                                rsx! {
                                    div { class: "metadata-field",
                                        label { "Status" }
                                        span {
                                            class: "status-badge {status_class}",
                                            "{status_emoji} {link_data.status}"
                                        }
                                    }
                                }
                            }

                            div { class: "metadata-field",
                                label { "Created" }
                                span { class: "field-value", "{format_date(&link_data.created_at)}" }
                            }

                            div { class: "metadata-field",
                                label { "Updated" }
                                span { class: "field-value", "{format_date(&link_data.updated_at)}" }
                            }

                            div { class: "metadata-field",
                                label { "Refreshed" }
                                span { class: "field-value",
                                    {
                                        match &link_data.refreshed_at {
                                            Some(date) => format_date(date),
                                            None => "-".to_string(),
                                        }
                                    }
                                }
                            }
                        }

                        // Error display
                        if let Some(err) = save_error() {
                            div { class: "error-message", "Error: {err}" }
                        }

                        // Footer with actions
                        div { class: "page-footer",
                            div { class: "keyboard-hints",
                                span { class: "hint",
                                    kbd { "Ctrl" }
                                    "+"
                                    kbd { "Enter" }
                                    " Save"
                                }
                                span { class: "hint",
                                    kbd { "Esc" }
                                    " Back"
                                }
                            }

                            div { class: "page-actions",
                                div { class: "action-group",
                                    button {
                                        class: "btn-delete",
                                        onclick: move |_| show_delete_confirm.set(true),
                                        "Delete"
                                    }
                                    button {
                                        class: "btn-secondary",
                                        onclick: handle_back,
                                        "Cancel"
                                    }
                                    button {
                                        class: "btn-primary",
                                        disabled: !has_changes() || saving(),
                                        onclick: move |_| do_save(),
                                        if saving() {
                                            "Saving..."
                                        } else {
                                            "Save"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Delete confirmation dialog
            if show_delete_confirm() {
                ConfirmDialog {
                    title: "Delete Link".to_string(),
                    message: "Are you sure you want to delete this link? This action cannot be undone.".to_string(),
                    confirm_text: "Delete".to_string(),
                    cancel_text: "Cancel".to_string(),
                    dangerous: true,
                    on_confirm: handle_delete,
                    on_cancel: move |_| show_delete_confirm.set(false)
                }
            }

            // Unsaved changes warning
            if show_unsaved_warning() {
                ConfirmDialog {
                    title: "Unsaved Changes".to_string(),
                    message: "You have unsaved changes. Are you sure you want to leave without saving?".to_string(),
                    confirm_text: "Discard Changes".to_string(),
                    cancel_text: "Keep Editing".to_string(),
                    dangerous: true,
                    on_confirm: move |_| {
                        show_unsaved_warning.set(false);
                        if let Some(target) = pending_navigation() {
                            nav.push(target);
                        }
                    },
                    on_cancel: move |_| {
                        show_unsaved_warning.set(false);
                        pending_navigation.set(None);
                    }
                }
            }
        }
    }
}
