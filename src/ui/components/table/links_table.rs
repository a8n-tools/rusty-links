use crate::ui::components::metadata_badges::{CategoryInfo, LanguageInfo, LicenseInfo, TagInfo};
use crate::ui::components::table::TableHeader;
use dioxus::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Link {
    pub id: Uuid,
    pub url: String,
    pub domain: String,
    pub path: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub logo: Option<String>,
    pub status: String,
    pub github_stars: Option<i32>,
    pub github_archived: Option<bool>,
    pub github_last_commit: Option<String>,
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

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len])
    } else {
        s.to_string()
    }
}

fn format_stars(stars: Option<i32>) -> String {
    match stars {
        Some(s) if s >= 1000 => format!("{:.1}k", s as f32 / 1000.0),
        Some(s) => s.to_string(),
        None => "-".to_string(),
    }
}

fn format_date(date_str: &str) -> String {
    // Extract YYYY-MM-DD from ISO date string
    date_str.split('T').next().unwrap_or(date_str).to_string()
}

fn get_status_class(status: &str) -> &'static str {
    match status {
        "active" => "status-active",
        "archived" => "status-archived",
        "inaccessible" => "status-inaccessible",
        "repo_unavailable" => "status-repo-unavailable",
        _ => "status-active",
    }
}

#[component]
pub fn LinksTable(
    links: Vec<Link>,
    sort_by: String,
    sort_order: String,
    on_sort: EventHandler<String>,
    on_row_click: EventHandler<Uuid>,
) -> Element {
    rsx! {
        div { class: "table-container",
            table { class: "links-table",
                thead {
                    tr {
                        TableHeader {
                            column: "logo".to_string(),
                            label: "Logo".to_string(),
                            sortable: false,
                            current_sort_by: sort_by.clone(),
                            current_sort_order: sort_order.clone(),
                            on_sort: on_sort
                        }
                        TableHeader {
                            column: "title".to_string(),
                            label: "Title".to_string(),
                            sortable: true,
                            current_sort_by: sort_by.clone(),
                            current_sort_order: sort_order.clone(),
                            on_sort: on_sort
                        }
                        TableHeader {
                            column: "domain".to_string(),
                            label: "Domain".to_string(),
                            sortable: true,
                            current_sort_by: sort_by.clone(),
                            current_sort_order: sort_order.clone(),
                            on_sort: on_sort
                        }
                        TableHeader {
                            column: "description".to_string(),
                            label: "Description".to_string(),
                            sortable: false,
                            current_sort_by: sort_by.clone(),
                            current_sort_order: sort_order.clone(),
                            on_sort: on_sort
                        }
                        TableHeader {
                            column: "tags".to_string(),
                            label: "Tags".to_string(),
                            sortable: false,
                            current_sort_by: sort_by.clone(),
                            current_sort_order: sort_order.clone(),
                            on_sort: on_sort
                        }
                        TableHeader {
                            column: "categories".to_string(),
                            label: "Categories".to_string(),
                            sortable: false,
                            current_sort_by: sort_by.clone(),
                            current_sort_order: sort_order.clone(),
                            on_sort: on_sort
                        }
                        TableHeader {
                            column: "languages".to_string(),
                            label: "Languages".to_string(),
                            sortable: false,
                            current_sort_by: sort_by.clone(),
                            current_sort_order: sort_order.clone(),
                            on_sort: on_sort
                        }
                        TableHeader {
                            column: "licenses".to_string(),
                            label: "Licenses".to_string(),
                            sortable: false,
                            current_sort_by: sort_by.clone(),
                            current_sort_order: sort_order.clone(),
                            on_sort: on_sort
                        }
                        TableHeader {
                            column: "status".to_string(),
                            label: "Status".to_string(),
                            sortable: true,
                            current_sort_by: sort_by.clone(),
                            current_sort_order: sort_order.clone(),
                            on_sort: on_sort
                        }
                        TableHeader {
                            column: "github_stars".to_string(),
                            label: "Stars".to_string(),
                            sortable: true,
                            current_sort_by: sort_by.clone(),
                            current_sort_order: sort_order.clone(),
                            on_sort: on_sort
                        }
                        TableHeader {
                            column: "created_at".to_string(),
                            label: "Created".to_string(),
                            sortable: true,
                            current_sort_by: sort_by.clone(),
                            current_sort_order: sort_order.clone(),
                            on_sort: on_sort
                        }
                        TableHeader {
                            column: "updated_at".to_string(),
                            label: "Updated".to_string(),
                            sortable: true,
                            current_sort_by: sort_by.clone(),
                            current_sort_order: sort_order.clone(),
                            on_sort: on_sort
                        }
                        TableHeader {
                            column: "refreshed_at".to_string(),
                            label: "Refreshed".to_string(),
                            sortable: true,
                            current_sort_by: sort_by.clone(),
                            current_sort_order: sort_order.clone(),
                            on_sort: on_sort
                        }
                    }
                }
                tbody {
                    for link in links {
                        tr {
                            key: "{link.id}",
                            class: "link-row",
                            onclick: move |_| {
                                on_row_click.call(link.id);
                            },

                            // Link icon/logo
                            td { class: "cell-logo",
                                if let Some(logo) = &link.logo {
                                    img {
                                        src: "{logo}",
                                        alt: "🔗",
                                        class: "link-logo"
                                    }
                                } else {
                                    div { class: "link-logo-placeholder", "🔗" }
                                }
                            }

                            // Title
                            td { class: "cell-title",
                                {link.title.as_ref().map(|t| truncate(t, 40)).unwrap_or_else(|| "-".to_string())}
                            }

                            // Domain
                            td { class: "cell-domain",
                                {
                                    if let Some(path) = &link.path {
                                        format!("{}{}", link.domain, path)
                                    } else {
                                        link.domain.clone()
                                    }
                                }
                            }

                            // Description
                            td { class: "cell-description",
                                {link.description.as_ref().map(|d| truncate(d, 60)).unwrap_or_else(|| "-".to_string())}
                            }

                            // Tags
                            td { class: "cell-tags",
                                {
                                    if link.tags.is_empty() {
                                        "-".to_string()
                                    } else if link.tags.len() <= 2 {
                                        link.tags.iter().map(|t| t.name.clone()).collect::<Vec<_>>().join(", ")
                                    } else {
                                        format!("{}, {} ...", link.tags[0].name, link.tags[1].name)
                                    }
                                }
                            }

                            // Categories
                            td { class: "cell-categories",
                                {
                                    if link.categories.is_empty() {
                                        "-".to_string()
                                    } else if link.categories.len() <= 2 {
                                        link.categories.iter()
                                            .map(|c| c.name.clone())
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    } else {
                                        format!("{}, {} ...",
                                            link.categories[0].name,
                                            link.categories[1].name
                                        )
                                    }
                                }
                            }

                            // Languages
                            td { class: "cell-languages",
                                {
                                    if link.languages.is_empty() {
                                        "-".to_string()
                                    } else if link.languages.len() <= 2 {
                                        link.languages.iter().map(|l| l.name.clone()).collect::<Vec<_>>().join(", ")
                                    } else {
                                        format!("{}, {} ...", link.languages[0].name, link.languages[1].name)
                                    }
                                }
                            }

                            // Licenses
                            td { class: "cell-licenses",
                                {
                                    if link.licenses.is_empty() {
                                        "-".to_string()
                                    } else if link.licenses.len() <= 2 {
                                        link.licenses.iter().map(|l| l.name.clone()).collect::<Vec<_>>().join(", ")
                                    } else {
                                        format!("{}, {} ...", link.licenses[0].name, link.licenses[1].name)
                                    }
                                }
                            }

                            // Status
                            td { class: "cell-status",
                                span {
                                    class: "status-badge {get_status_class(&link.status)}",
                                    {link.status.clone()}
                                }
                            }

                            // Stars
                            td { class: "cell-stars",
                                {format_stars(link.github_stars)}
                            }

                            // Created At
                            td { class: "cell-date",
                                {format_date(&link.created_at)}
                            }

                            // Updated At
                            td { class: "cell-date",
                                {format_date(&link.updated_at)}
                            }

                            // Refreshed At
                            td { class: "cell-date",
                                {link.refreshed_at.as_ref().map(|d| format_date(d)).unwrap_or_else(|| "-".to_string())}
                            }
                        }
                    }
                }
            }
        }
    }
}
