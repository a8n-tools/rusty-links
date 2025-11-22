use dioxus::prelude::*;

/// Get the range of page numbers to display
///
/// For example, if current_page=5 and total_pages=10, might return [3, 4, 5, 6, 7]
/// Always includes first and last page with ellipsis if needed
fn get_page_range(current_page: u32, total_pages: u32) -> Vec<Option<u32>> {
    if total_pages <= 7 {
        // Show all pages if 7 or fewer
        return (1..=total_pages).map(Some).collect();
    }

    let mut pages = Vec::new();

    // Always show first page
    pages.push(Some(1));

    if current_page > 3 {
        // Add ellipsis if current page is far from start
        pages.push(None);
    }

    // Show pages around current page
    let start = current_page.saturating_sub(1).max(2);
    let end = (current_page + 1).min(total_pages - 1);

    for page in start..=end {
        if page > 1 && page < total_pages {
            pages.push(Some(page));
        }
    }

    if current_page < total_pages - 2 {
        // Add ellipsis if current page is far from end
        pages.push(None);
    }

    // Always show last page
    if total_pages > 1 {
        pages.push(Some(total_pages));
    }

    pages
}

#[component]
pub fn Pagination(
    current_page: u32,
    total_pages: u32,
    on_page_change: EventHandler<u32>,
) -> Element {
    // Don't show pagination if there's only one page
    if total_pages <= 1 {
        return rsx! {};
    }

    let page_range = get_page_range(current_page, total_pages);
    let has_previous = current_page > 1;
    let has_next = current_page < total_pages;

    rsx! {
        div { class: "pagination-container",
            div { class: "pagination",
                // Previous button
                button {
                    class: "pagination-btn pagination-prev",
                    disabled: !has_previous,
                    onclick: move |_| {
                        if has_previous {
                            on_page_change.call(current_page - 1);
                        }
                    },
                    "← Previous"
                }

                // Page number buttons
                div { class: "pagination-pages",
                    for page_option in page_range {
                        if let Some(page) = page_option {
                            button {
                                key: "{page}",
                                class: if page == current_page {
                                    "pagination-btn pagination-page pagination-page-active"
                                } else {
                                    "pagination-btn pagination-page"
                                },
                                onclick: move |_| {
                                    if page != current_page {
                                        on_page_change.call(page);
                                    }
                                },
                                "{page}"
                            }
                        } else {
                            span {
                                key: "ellipsis-{page_option:?}",
                                class: "pagination-ellipsis",
                                "..."
                            }
                        }
                    }
                }

                // Next button
                button {
                    class: "pagination-btn pagination-next",
                    disabled: !has_next,
                    onclick: move |_| {
                        if has_next {
                            on_page_change.call(current_page + 1);
                        }
                    },
                    "Next →"
                }
            }

            // Page info
            div { class: "pagination-info",
                "Page {current_page} of {total_pages}"
            }
        }
    }
}
