use dioxus::prelude::*;
use std::fmt;

/// Toast notification type
#[derive(Clone, Debug, PartialEq)]
pub enum ToastType {
    Success,
    Error,
    Warning,
    Info,
}

impl fmt::Display for ToastType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToastType::Success => write!(f, "success"),
            ToastType::Error => write!(f, "error"),
            ToastType::Warning => write!(f, "warning"),
            ToastType::Info => write!(f, "info"),
        }
    }
}

/// Toast notification
#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    pub id: String,
    pub message: String,
    pub toast_type: ToastType,
    pub duration_ms: u64,
}

impl Toast {
    pub fn success(message: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            message,
            toast_type: ToastType::Success,
            duration_ms: 3000,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            message,
            toast_type: ToastType::Error,
            duration_ms: 5000,
        }
    }

    pub fn warning(message: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            message,
            toast_type: ToastType::Warning,
            duration_ms: 4000,
        }
    }

    pub fn info(message: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            message,
            toast_type: ToastType::Info,
            duration_ms: 3000,
        }
    }
}

/// Toast container component
#[component]
pub fn ToastContainer(mut toasts: Signal<Vec<Toast>>) -> Element {
    rsx! {
        div { class: "toast-container",
            for toast in toasts() {
                ToastItem {
                    key: "{toast.id}",
                    toast: toast.clone(),
                    toasts: toasts,
                }
            }
        }
    }
}

#[component]
fn ToastItem(toast: Toast, mut toasts: Signal<Vec<Toast>>) -> Element {
    let toast_id = toast.id.clone();
    let duration = toast.duration_ms;

    // Auto-dismiss after duration
    use_effect(move || {
        let id = toast_id.clone();
        spawn(async move {
            #[cfg(target_arch = "wasm32")]
            {
                gloo_timers::future::TimeoutFuture::new(duration as u32).await;
            }

            #[cfg(all(not(target_arch = "wasm32"), feature = "server"))]
            {
                tokio::time::sleep(std::time::Duration::from_millis(duration)).await;
            }

            #[cfg(not(any(target_arch = "wasm32", feature = "server")))]
            let _ = duration;

            let mut current_toasts = toasts();
            current_toasts.retain(|t| t.id != id);
            toasts.set(current_toasts);
        });
    });

    let (icon, icon_class) = match toast.toast_type {
        ToastType::Success => ("✓", "toast-icon-success"),
        ToastType::Error => ("✗", "toast-icon-error"),
        ToastType::Warning => ("⚠", "toast-icon-warning"),
        ToastType::Info => ("ℹ", "toast-icon-info"),
    };

    let toast_id_for_dismiss = toast.id.clone();

    rsx! {
        div { class: "toast toast-{toast.toast_type}",
            div { class: "toast-content",
                span { class: "toast-icon {icon_class}", "{icon}" }
                span { class: "toast-message", "{toast.message}" }
            }
            button {
                class: "toast-close",
                onclick: move |_| {
                    let mut current_toasts = toasts();
                    current_toasts.retain(|t| t.id != toast_id_for_dismiss);
                    toasts.set(current_toasts);
                },
                "×"
            }
        }
    }
}

/// Helper to add a toast
pub fn add_toast(toasts: &mut Signal<Vec<Toast>>, message: String, toast_type: ToastType) {
    let mut current = toasts();
    let toast = match toast_type {
        ToastType::Success => Toast::success(message),
        ToastType::Error => Toast::error(message),
        ToastType::Warning => Toast::warning(message),
        ToastType::Info => Toast::info(message),
    };
    current.push(toast);
    toasts.set(current);
}
