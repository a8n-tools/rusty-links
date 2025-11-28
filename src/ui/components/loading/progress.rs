use dioxus::prelude::*;

#[component]
pub fn LoadingProgress(
    current: i32,
    total: i32,
    #[props(default = String::new())] message: String,
) -> Element {
    let percentage = if total > 0 {
        (current as f32 / total as f32 * 100.0) as i32
    } else {
        0
    };

    rsx! {
        div { class: "progress-container",
            if !message.is_empty() {
                p { class: "progress-message", "{message}" }
            }
            div { class: "progress-bar",
                div {
                    class: "progress-fill",
                    style: "width: {percentage}%",
                }
            }
            p { class: "progress-text", "{current} / {total} ({percentage}%)" }
        }
    }
}

#[component]
pub fn IndeterminateProgress(
    #[props(default = String::new())] message: String,
) -> Element {
    rsx! {
        div { class: "progress-container",
            if !message.is_empty() {
                p { class: "progress-message", "{message}" }
            }
            div { class: "progress-bar",
                div { class: "progress-fill-indeterminate" }
            }
        }
    }
}
