use dioxus::prelude::*;
use serde::Deserialize;
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

#[component]
pub fn Links() -> Element {
    let mut links = use_signal(|| Vec::<Link>::new());
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| Option::<String>::None);

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

    rsx! {
        div { class: "page-container",
            Navbar {}
            div { class: "links-page",
                div { class: "links-header",
                    h1 { class: "links-title", "My Links" }
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
