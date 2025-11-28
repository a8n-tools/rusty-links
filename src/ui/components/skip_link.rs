use dioxus::prelude::*;

/// Skip link component for keyboard navigation
/// Allows keyboard users to skip navigation and jump directly to main content
#[component]
pub fn SkipLink() -> Element {
    rsx! {
        a {
            href: "#main-content",
            class: "skip-link",
            "Skip to main content"
        }
    }
}
