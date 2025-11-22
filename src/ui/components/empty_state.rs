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
