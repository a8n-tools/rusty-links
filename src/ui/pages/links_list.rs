use dioxus::prelude::*;
use serde::Deserialize;
use uuid::Uuid;
use std::time::Duration;
use crate::ui::components::navbar::Navbar;
use crate::ui::components::table::LinksTable;
use crate::ui::components::table::links_table::Link;
use crate::ui::components::pagination::Pagination;
use crate::ui::components::loading::LoadingSpinner;
use crate::ui::components::empty_state::EmptyState;
use crate::ui::components::search_filter::{SearchBar, FiltersContainer, FilterOption};
use crate::ui::components::modal::LinkDetailsModal;

#[derive(Debug, Clone, Deserialize)]
struct PaginatedLinksResponse {
    links: Vec<Link>,
    total: i64,
    page: u32,
    per_page: u32,
    total_pages: u32,
}

fn build_links_query(
    page: u32,
    per_page: u32,
    sort_by: String,
    sort_order: String,
    search: String,
    languages: Vec<Uuid>,
    licenses: Vec<Uuid>,
    categories: Vec<Uuid>,
    tags: Vec<Uuid>,
) -> String {
    let mut params = vec![
        format!("page={}", page),
        format!("per_page={}", per_page),
        format!("sort_by={}", sort_by),
        format!("sort_order={}", sort_order),
    ];

    if !search.is_empty() {
        let encoded_query = urlencoding::encode(&search);
        params.push(format!("query={}", encoded_query));
    }

    for lang_id in languages {
        params.push(format!("language_id={}", lang_id));
    }

    for lic_id in licenses {
        params.push(format!("license_id={}", lic_id));
    }

    for cat_id in categories {
        params.push(format!("category_id={}", cat_id));
    }

    for tag_id in tags {
        params.push(format!("tag_id={}", tag_id));
    }

    format!("/api/links?{}", params.join("&"))
}

async fn fetch_links(
    page: u32,
    per_page: u32,
    sort_by: String,
    sort_order: String,
    search: String,
    languages: Vec<Uuid>,
    licenses: Vec<Uuid>,
    categories: Vec<Uuid>,
    tags: Vec<Uuid>,
) -> Result<PaginatedLinksResponse, String> {
    let client = reqwest::Client::new();
    let url = build_links_query(page, per_page, sort_by, sort_order, search, languages, licenses, categories, tags);

    let response = client.get(&url).send().await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        response.json::<PaginatedLinksResponse>().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        Err(format!("Server error: {}", response.status()))
    }
}

async fn fetch_filter_options() -> Result<
    (Vec<FilterOption>, Vec<FilterOption>, Vec<FilterOption>, Vec<FilterOption>),
    String
> {
    let client = reqwest::Client::new();

    // Fetch all filter options in parallel
    let (languages_res, licenses_res, categories_res, tags_res) = tokio::join!(
        client.get("/api/languages").send(),
        client.get("/api/licenses").send(),
        client.get("/api/categories").send(),
        client.get("/api/tags").send(),
    );

    let languages: Vec<FilterOption> = languages_res
        .map_err(|e| format!("Error fetching languages: {}", e))?
        .json::<Vec<serde_json::Value>>()
        .await
        .map_err(|e| format!("Error parsing languages: {}", e))?
        .into_iter()
        .filter_map(|v| {
            Some(FilterOption {
                id: Uuid::parse_str(v.get("id")?.as_str()?).ok()?,
                label: v.get("name")?.as_str()?.to_string(),
            })
        })
        .collect();

    let licenses: Vec<FilterOption> = licenses_res
        .map_err(|e| format!("Error fetching licenses: {}", e))?
        .json::<Vec<serde_json::Value>>()
        .await
        .map_err(|e| format!("Error parsing licenses: {}", e))?
        .into_iter()
        .filter_map(|v| {
            Some(FilterOption {
                id: Uuid::parse_str(v.get("id")?.as_str()?).ok()?,
                label: v.get("name")?.as_str()?.to_string(),
            })
        })
        .collect();

    let categories: Vec<FilterOption> = categories_res
        .map_err(|e| format!("Error fetching categories: {}", e))?
        .json::<Vec<serde_json::Value>>()
        .await
        .map_err(|e| format!("Error parsing categories: {}", e))?
        .into_iter()
        .filter_map(|v| {
            Some(FilterOption {
                id: Uuid::parse_str(v.get("id")?.as_str()?).ok()?,
                label: v.get("name")?.as_str()?.to_string(),
            })
        })
        .collect();

    let tags: Vec<FilterOption> = tags_res
        .map_err(|e| format!("Error fetching tags: {}", e))?
        .json::<Vec<serde_json::Value>>()
        .await
        .map_err(|e| format!("Error parsing tags: {}", e))?
        .into_iter()
        .filter_map(|v| {
            Some(FilterOption {
                id: Uuid::parse_str(v.get("id")?.as_str()?).ok()?,
                label: v.get("name")?.as_str()?.to_string(),
            })
        })
        .collect();

    Ok((languages, licenses, categories, tags))
}

#[component]
pub fn LinksListPage() -> Element {
    // State for links data
    let mut links = use_signal(|| Vec::<Link>::new());
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| Option::<String>::None);

    // Pagination state
    let mut current_page = use_signal(|| 1u32);
    let mut total_pages = use_signal(|| 1u32);
    let mut total_links = use_signal(|| 0i64);
    let per_page = use_signal(|| 20u32);

    // Sorting state
    let mut sort_by = use_signal(|| "created_at".to_string());
    let mut sort_order = use_signal(|| "desc".to_string());

    // Search state
    let mut search_query = use_signal(|| String::new());
    let mut debounced_search = use_signal(|| String::new());

    // Filter state
    let mut selected_languages = use_signal(|| Vec::<Uuid>::new());
    let mut selected_licenses = use_signal(|| Vec::<Uuid>::new());
    let mut selected_categories = use_signal(|| Vec::<Uuid>::new());
    let mut selected_tags = use_signal(|| Vec::<Uuid>::new());

    // Filter options
    let mut languages = use_signal(|| Vec::<FilterOption>::new());
    let mut licenses = use_signal(|| Vec::<FilterOption>::new());
    let mut categories = use_signal(|| Vec::<FilterOption>::new());
    let mut tags = use_signal(|| Vec::<FilterOption>::new());

    // Modal state
    let mut show_modal = use_signal(|| false);
    let mut selected_link_id = use_signal(|| Option::<Uuid>::None);

    // Fetch filter options on mount
    use_effect(move || {
        spawn(async move {
            match fetch_filter_options().await {
                Ok((langs, lics, cats, tgs)) => {
                    languages.set(langs);
                    licenses.set(lics);
                    categories.set(cats);
                    tags.set(tgs);
                },
                Err(err) => {
                    tracing::error!("Error fetching filter options: {}", err);
                }
            }
        });
    });

    // Debounce search input
    use_effect(move || {
        let query = search_query();
        spawn(async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            debounced_search.set(query);
        });
    });

    // Fetch links function
    let fetch = move || {
        spawn(async move {
            loading.set(true);
            error.set(None);

            match fetch_links(
                current_page(),
                per_page(),
                sort_by(),
                sort_order(),
                debounced_search(),
                selected_languages(),
                selected_licenses(),
                selected_categories(),
                selected_tags(),
            ).await {
                Ok(response) => {
                    links.set(response.links);
                    total_pages.set(response.total_pages);
                    total_links.set(response.total);
                    loading.set(false);
                },
                Err(err) => {
                    error.set(Some(err));
                    loading.set(false);
                }
            }
        });
    };

    // Re-fetch when dependencies change
    use_effect(move || {
        // Trigger on any change to filters, search, sort, or page
        let _ = (
            debounced_search(),
            selected_languages(),
            selected_licenses(),
            selected_categories(),
            selected_tags(),
            sort_by(),
            sort_order(),
            current_page(),
        );
        fetch();
    });

    // Handle sort
    let handle_sort = move |column: String| {
        if sort_by() == column {
            // Toggle order
            let new_order = if sort_order() == "asc" { "desc".to_string() } else { "asc".to_string() };
            sort_order.set(new_order);
        } else {
            // New column, default to desc
            sort_by.set(column);
            sort_order.set("desc".to_string());
        }
        current_page.set(1); // Reset to first page
    };

    // Handle page change
    let handle_page_change = move |page: u32| {
        current_page.set(page);
    };

    // Handle filter changes
    let handle_languages_change = move |langs: Vec<Uuid>| {
        selected_languages.set(langs);
        current_page.set(1);
    };

    let handle_licenses_change = move |lics: Vec<Uuid>| {
        selected_licenses.set(lics);
        current_page.set(1);
    };

    let handle_categories_change = move |cats: Vec<Uuid>| {
        selected_categories.set(cats);
        current_page.set(1);
    };

    let handle_tags_change = move |tgs: Vec<Uuid>| {
        selected_tags.set(tgs);
        current_page.set(1);
    };

    // Handle search change
    let handle_search_change = move |query: String| {
        search_query.set(query);
        current_page.set(1);
    };

    // Reset all filters
    let handle_reset = move |_| {
        search_query.set(String::new());
        debounced_search.set(String::new());
        selected_languages.set(Vec::new());
        selected_licenses.set(Vec::new());
        selected_categories.set(Vec::new());
        selected_tags.set(Vec::new());
        current_page.set(1);
    };

    rsx! {
        div { class: "page-container",
            Navbar {}

            div { class: "content-container",
                div { class: "page-header",
                    h1 { "Links" }
                }

                // Search bar
                SearchBar {
                    value: search_query(),
                    on_change: handle_search_change,
                    placeholder: "Search links...".to_string()
                }

                // Filters
                FiltersContainer {
                    languages: languages(),
                    selected_languages: selected_languages(),
                    on_languages_change: handle_languages_change,

                    licenses: licenses(),
                    selected_licenses: selected_licenses(),
                    on_licenses_change: handle_licenses_change,

                    categories: categories(),
                    selected_categories: selected_categories(),
                    on_categories_change: handle_categories_change,

                    tags: tags(),
                    selected_tags: selected_tags(),
                    on_tags_change: handle_tags_change,

                    on_reset: handle_reset
                }

                // Loading, error, empty, or table
                if loading() {
                    LoadingSpinner {}
                } else if let Some(err) = error() {
                    div { class: "error-message",
                        "Error loading links: {err}"
                    }
                } else if links().is_empty() {
                    EmptyState {
                        icon: "🔗".to_string(),
                        title: "No links found".to_string(),
                        description: "Try adjusting your search or filters".to_string(),
                        action: None
                    }
                } else {
                    // Links table
                    LinksTable {
                        links: links(),
                        sort_by: sort_by(),
                        sort_order: sort_order(),
                        on_sort: handle_sort,
                        on_row_click: move |link_id: Uuid| {
                            selected_link_id.set(Some(link_id));
                            show_modal.set(true);
                        }
                    }

                    // Pagination
                    Pagination {
                        current_page: current_page(),
                        total_pages: total_pages(),
                        on_page_change: handle_page_change
                    }

                    // Showing info
                    div { class: "results-info",
                        {
                            let start = ((current_page() - 1) * per_page()) + 1;
                            let end = (current_page() * per_page()).min(total_links() as u32);
                            format!("Showing {} - {} of {} links", start, end, total_links())
                        }
                    }
                }
            }

            // Link Details Modal
            if let Some(link_id) = selected_link_id() {
                LinkDetailsModal {
                    link_id: link_id,
                    is_open: show_modal(),
                    on_close: move |_| {
                        show_modal.set(false);
                        selected_link_id.set(None);
                    },
                    on_save: move |_| {
                        // Re-fetch links after save
                        fetch();
                    }
                }
            }
        }
    }
}
