use dioxus::prelude::*;

#[component]
pub fn ConfirmDialog(
    title: String,
    message: String,
    confirm_text: String,
    cancel_text: String,
    #[props(default = false)] dangerous: bool,
    on_confirm: EventHandler<()>,
    on_cancel: EventHandler<()>,
) -> Element {
    rsx! {
        div {
            class: "modal-overlay",
            div {
                class: "confirm-dialog",
                h3 { "{title}" }
                p { "{message}" }
                div { class: "dialog-actions",
                    button {
                        class: "btn-secondary",
                        onclick: move |_| on_cancel.call(()),
                        "{cancel_text}"
                    }
                    button {
                        class: if dangerous { "btn-danger" } else { "btn-primary" },
                        onclick: move |_| on_confirm.call(()),
                        "{confirm_text}"
                    }
                }
            }
        }
    }
}
