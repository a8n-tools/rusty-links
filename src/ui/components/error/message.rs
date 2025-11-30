use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
}

#[component]
pub fn ErrorMessage(
    message: String,
    #[props(default = ErrorSeverity::Error)] severity: ErrorSeverity,
    #[props(default = None)] on_retry: Option<EventHandler<()>>,
    #[props(default = None)] on_dismiss: Option<EventHandler<()>>,
) -> Element {
    let (class_name, icon) = match severity {
        ErrorSeverity::Info => ("error-message-info", "ℹ️"),
        ErrorSeverity::Warning => ("error-message-warning", "⚠️"),
        ErrorSeverity::Error => ("error-message-error", "❌"),
    };

    rsx! {
        div { class: "error-message {class_name}",
            div { class: "error-content",
                span { class: "error-icon", "{icon}" }
                span { class: "error-text", "{message}" }
            }
            div { class: "error-actions",
                if let Some(retry) = on_retry {
                    button {
                        class: "btn-retry",
                        onclick: move |_| retry.call(()),
                        "Retry"
                    }
                }
                if let Some(dismiss) = on_dismiss {
                    button {
                        class: "btn-dismiss",
                        onclick: move |_| dismiss.call(()),
                        "×"
                    }
                }
            }
        }
    }
}

#[component]
pub fn InlineError(message: String) -> Element {
    rsx! {
        span { class: "inline-error",
            span { class: "inline-error-icon", "⚠️" }
            span { class: "inline-error-text", "{message}" }
        }
    }
}

#[component]
pub fn FieldError(message: String) -> Element {
    rsx! {
        div { class: "field-error",
            "{message}"
        }
    }
}
