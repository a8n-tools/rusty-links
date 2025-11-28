use dioxus::prelude::*;

/// Live region component for screen reader announcements
/// Use "polite" for non-critical updates, "assertive" for important updates
#[component]
pub fn LiveRegion(
    message: String,
    #[props(default = String::from("polite"))] level: String,
) -> Element {
    if message.is_empty() {
        return rsx! { div { class: "sr-only" } };
    }

    rsx! {
        div {
            class: "sr-only",
            role: "status",
            "aria-live": "{level}",
            "aria-atomic": "true",
            "{message}"
        }
    }
}

/// Component for announcing loading states to screen readers
#[component]
pub fn LoadingAnnouncement(
    loading: bool,
    message: String,
) -> Element {
    if !loading {
        return rsx! { div { class: "sr-only" } };
    }

    rsx! {
        div {
            class: "sr-only",
            role: "status",
            "aria-live": "polite",
            "aria-busy": "true",
            "{message}"
        }
    }
}

/// Component for announcing errors to screen readers
#[component]
pub fn ErrorAnnouncement(
    error: Option<String>,
) -> Element {
    match error {
        Some(msg) => rsx! {
            div {
                class: "sr-only",
                role: "alert",
                "aria-live": "assertive",
                "Error: {msg}"
            }
        },
        None => rsx! { div { class: "sr-only" } }
    }
}
