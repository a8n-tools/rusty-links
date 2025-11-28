use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub enum SpinnerSize {
    Small,
    Medium,
    Large,
}

#[component]
pub fn LoadingSpinner(
    #[props(default = SpinnerSize::Medium)] size: SpinnerSize,
    #[props(default = String::new())] message: String,
) -> Element {
    let size_class = match size {
        SpinnerSize::Small => "spinner-small",
        SpinnerSize::Medium => "spinner-medium",
        SpinnerSize::Large => "spinner-large",
    };

    rsx! {
        div { class: "loading-container",
            div { class: "spinner {size_class}" }
            if !message.is_empty() {
                p { class: "loading-message", "{message}" }
            }
        }
    }
}

#[component]
pub fn InlineSpinner() -> Element {
    rsx! {
        span { class: "spinner-inline" }
    }
}

#[component]
pub fn ButtonSpinner() -> Element {
    rsx! {
        span { class: "spinner-button" }
    }
}
