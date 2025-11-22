use dioxus::prelude::*;

/// Loading spinner component
#[component]
pub fn LoadingSpinner(size: Option<String>) -> Element {
    let size_class = size.unwrap_or_else(|| "medium".to_string());
    rsx! {
        div { class: "spinner spinner-{size_class}" }
    }
}

/// Full-page loading overlay
#[component]
pub fn LoadingOverlay() -> Element {
    rsx! {
        div { class: "loading-overlay",
            LoadingSpinner { size: Some("large".to_string()) }
        }
    }
}

/// Skeleton card for loading state
#[component]
pub fn SkeletonCard() -> Element {
    rsx! {
        div { class: "skeleton-card",
            div { class: "skeleton skeleton-title" }
            div { class: "skeleton skeleton-text" }
            div { class: "skeleton skeleton-text short" }
        }
    }
}
