use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use crate::ui::components::navbar::Navbar;

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
    created_at: String,
}

#[derive(Debug, Serialize)]
struct CreateLinkRequest {
    url: String,
    title: Option<String>,
    description: Option<String>,
}

#[component]
pub fn Links() -> Element {
    let mut links = use_signal(|| Vec::<Link>::new());
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| Option::<String>::None);

    // Form state
    let mut show_form = use_signal(|| false);
    let mut new_url = use_signal(|| String::new());
    let mut new_title = use_signal(|| String::new());
    let mut new_description = use_signal(|| String::new());
    let mut form_loading = use_signal(|| false);
    let mut form_error = use_signal(|| Option::<String>::None);

    // Fetch links on mount
    use_effect(move || {
        spawn(async move {
            let client = reqwest::Client::new();
            let response = client
                .get("/api/links")
                .send()
                .await;

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

    let on_submit = move |_| {
        let url_val = new_url();
        let title_val = new_title();
        let desc_val = new_description();

        // Validation
        if url_val.trim().is_empty() {
            form_error.set(Some("URL is required".to_string()));
            return;
        }

        if !url_val.starts_with("http://") && !url_val.starts_with("https://") {
            form_error.set(Some("URL must start with http:// or https://".to_string()));
            return;
        }

        spawn(async move {
            form_loading.set(true);
            form_error.set(None);

            let request = CreateLinkRequest {
                url: url_val,
                title: if title_val.trim().is_empty() { None } else { Some(title_val) },
                description: if desc_val.trim().is_empty() { None } else { Some(desc_val) },
            };

            let client = reqwest::Client::new();
            let response = client
                .post("/api/links")
                .json(&request)
                .send()
                .await;

            form_loading.set(false);

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        match resp.json::<Link>().await {
                            Ok(new_link) => {
                                // Add to list
                                let mut current = links();
                                current.insert(0, new_link);
                                links.set(current);

                                // Clear form and hide
                                new_url.set(String::new());
                                new_title.set(String::new());
                                new_description.set(String::new());
                                show_form.set(false);
                            }
                            Err(e) => {
                                form_error.set(Some(format!("Failed to parse response: {}", e)));
                            }
                        }
                    } else {
                        let error_text = resp.text().await.unwrap_or_else(|_| "Failed to create link".to_string());
                        form_error.set(Some(error_text));
                    }
                }
                Err(e) => {
                    form_error.set(Some(format!("Network error: {}", e)));
                }
            }
        });
    };

    let on_cancel = move |_| {
        new_url.set(String::new());
        new_title.set(String::new());
        new_description.set(String::new());
        form_error.set(None);
        show_form.set(false);
    };

    rsx! {
        div { class: "page-container",
            Navbar {}
            div { class: "links-page",
                div { class: "links-header",
                    h1 { class: "links-title", "My Links" }
                    if !show_form() {
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| show_form.set(true),
                            "+ Add Link"
                        }
                    }
                }

                // Add Link Form
                if show_form() {
                    div { class: "link-form",
                        h2 { class: "form-title", "Add New Link" }

                        if let Some(err) = form_error() {
                            div { class: "message message-error", "{err}" }
                        }

                        div { class: "form-group",
                            label { class: "form-label", r#for: "url", "URL *" }
                            input {
                                class: "form-input",
                                r#type: "url",
                                id: "url",
                                placeholder: "https://example.com",
                                value: "{new_url}",
                                disabled: form_loading(),
                                oninput: move |evt| new_url.set(evt.value()),
                            }
                        }

                        div { class: "form-group",
                            label { class: "form-label", r#for: "title", "Title" }
                            input {
                                class: "form-input",
                                r#type: "text",
                                id: "title",
                                placeholder: "Optional title",
                                value: "{new_title}",
                                disabled: form_loading(),
                                oninput: move |evt| new_title.set(evt.value()),
                            }
                        }

                        div { class: "form-group",
                            label { class: "form-label", r#for: "description", "Description" }
                            textarea {
                                class: "form-input form-textarea",
                                id: "description",
                                placeholder: "Optional description",
                                value: "{new_description}",
                                disabled: form_loading(),
                                oninput: move |evt| new_description.set(evt.value()),
                            }
                        }

                        div { class: "form-actions",
                            button {
                                class: "btn btn-secondary",
                                r#type: "button",
                                disabled: form_loading(),
                                onclick: on_cancel,
                                "Cancel"
                            }
                            button {
                                class: "btn btn-primary",
                                r#type: "button",
                                disabled: form_loading(),
                                onclick: on_submit,
                                if form_loading() {
                                    "Saving..."
                                } else {
                                    "Save Link"
                                }
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
                            p { class: "empty-title", "No links yet" }
                            p { class: "empty-subtitle", "Add your first link to get started!" }
                        }
                    } else {
                        div { class: "links-list",
                            for link in links() {
                                LinkCard { key: "{link.id}", link: link }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn LinkCard(link: Link) -> Element {
    let display_title = link.title.clone().unwrap_or_else(|| link.url.clone());
    let description = link.description.clone().unwrap_or_default();
    let truncated_desc = if description.len() > 150 {
        format!("{}...", &description[..150])
    } else {
        description
    };

    // Format date (simple display)
    let created_date = &link.created_at[..10]; // Just the date part

    rsx! {
        div { class: "link-card",
            div { class: "link-card-header",
                h3 { class: "link-title",
                    a { href: "{link.url}", target: "_blank", rel: "noopener noreferrer",
                        "{display_title}"
                    }
                }
                span { class: "link-status status-{link.status}", "{link.status}" }
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

            div { class: "link-meta",
                span { class: "link-date", "Added {created_date}" }
            }
        }
    }
}
