use dioxus::prelude::*;
use serde::Deserialize;
use uuid::Uuid;
use crate::ui::components::navbar::Navbar;
use crate::ui::components::table::LinksTable;
use crate::ui::components::table::links_table::Link;
use crate::ui::components::pagination::Pagination;
use crate::ui::components::loading::LoadingSpinner;
use crate::ui::components::empty_state::EmptyState;

#[derive(Debug, Clone, Deserialize)]
struct PaginatedLinksResponse {
    links: Vec<Link>,
    total: i64,
    page: u32,
    per_page: u32,
    total_pages: u32,
}

async fn fetch_links(
    page: u32,
    per_page: u32,
    sort_by: String,
    sort_order: String,
) -> Result<PaginatedLinksResponse, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "/api/links?page={}&per_page={}&sort_by={}&sort_order={}",
        page, per_page, sort_by, sort_order
    );

    let response = client.get(&url).send().await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        response.json::<PaginatedLinksResponse>().await
            .map_err(|e| format!("Parse error: {}", e))
    } else {
        Err(format!("Server error: {}", response.status()))
    }
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

    // Fetch on mount and when dependencies change
    use_effect(move || {
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

    rsx! {
        div { class: "page-container",
            Navbar {}

            div { class: "content-container",
                div { class: "page-header",
                    h1 { "Links" }
                }

                // Placeholder for search and filters (Step 34)
                div { class: "search-filters-placeholder",
                    p { style: "color: var(--rust-text-secondary); font-style: italic;",
                        "Search and filters coming in Step 34"
                    }
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
                        description: "Add your first link to get started".to_string(),
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
                            // Placeholder for Step 35 - will open modal
                            tracing::info!("Clicked link: {}", link_id);
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
        }
    }
}
