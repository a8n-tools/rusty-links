use dioxus::prelude::*;

/// Empty state component with icon, title, description, and optional action
#[component]
pub fn EmptyState(
    icon: String,
    title: String,
    description: String,
    action: Option<Element>,
) -> Element {
    rsx! {
        div { class: "empty-state",
            span { class: "empty-icon", "{icon}" }
            h3 { class: "empty-title", "{title}" }
            p { class: "empty-description", "{description}" }
            if let Some(action_element) = action {
                div { class: "empty-action",
                    {action_element}
                }
            }
        }
    }
}

/// Empty state for no search results
#[component]
pub fn NoResultsState(
    search_query: String,
    #[props(default = None)] on_clear: Option<EventHandler<()>>,
) -> Element {
    rsx! {
        div { class: "empty-state",
            span { class: "empty-icon", "🔍" }
            h3 { class: "empty-title", "No results found" }
            p { class: "empty-description",
                "No items match your search for \"{search_query}\"."
            }
            if let Some(clear) = on_clear {
                div { class: "empty-action",
                    button {
                        class: "btn-secondary",
                        onclick: move |_| clear.call(()),
                        "Clear search"
                    }
                }
            }
        }
    }
}

/// Empty state for errors
#[component]
pub fn ErrorState(
    message: String,
    #[props(default = None)] on_retry: Option<EventHandler<()>>,
) -> Element {
    rsx! {
        div { class: "empty-state error-state",
            span { class: "empty-icon", "⚠️" }
            h3 { class: "empty-title", "Something went wrong" }
            p { class: "empty-description", "{message}" }
            if let Some(retry) = on_retry {
                div { class: "empty-action",
                    button {
                        class: "btn-primary",
                        onclick: move |_| retry.call(()),
                        "Try again"
                    }
                }
            }
        }
    }
}
