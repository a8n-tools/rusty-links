use dioxus::prelude::*;
use std::fmt;

/// Toast notification type
#[derive(Clone, Debug, PartialEq)]
pub enum ToastType {
    Success,
    Error,
    Info,
}

impl fmt::Display for ToastType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToastType::Success => write!(f, "success"),
            ToastType::Error => write!(f, "error"),
            ToastType::Info => write!(f, "info"),
        }
    }
}

/// Toast notification
#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    pub id: u32,
    pub message: String,
    pub toast_type: ToastType,
}

/// Toast container component
#[component]
pub fn ToastContainer(mut toasts: Signal<Vec<Toast>>) -> Element {
    rsx! {
        div { class: "toast-container",
            for toast in toasts() {
                div {
                    class: "toast toast-{toast.toast_type}",
                    key: "{toast.id}",
                    span { class: "toast-message", "{toast.message}" }
                    button {
                        class: "toast-close",
                        onclick: move |_| {
                            let mut current_toasts = toasts();
                            current_toasts.retain(|t| t.id != toast.id);
                            toasts.set(current_toasts);
                        },
                        "✕"
                    }
                }
            }
        }
    }
}

/// Helper to add a toast
pub fn add_toast(toasts: &mut Signal<Vec<Toast>>, message: String, toast_type: ToastType) {
    let mut current = toasts();
    let id = current.len() as u32 + 1;
    current.push(Toast {
        id,
        message,
        toast_type,
    });
    toasts.set(current);
}
