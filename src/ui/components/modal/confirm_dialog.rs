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
    // Keyboard navigation - Escape to cancel
    let handle_keydown = move |evt: KeyboardEvent| {
        if evt.key() == Key::Escape {
            on_cancel.call(());
        }
    };

    rsx! {
        div {
            class: "modal-overlay",
            role: "dialog",
            "aria-modal": "true",
            "aria-labelledby": "confirm-dialog-title",
            "aria-describedby": "confirm-dialog-message",
            tabindex: "-1",
            onkeydown: handle_keydown,
            div {
                class: "confirm-dialog",
                h3 {
                    id: "confirm-dialog-title",
                    "{title}"
                }
                p {
                    id: "confirm-dialog-message",
                    "{message}"
                }
                div {
                    class: "dialog-actions",
                    role: "group",
                    "aria-label": "Dialog actions",
                    button {
                        class: "btn-secondary",
                        onclick: move |_| on_cancel.call(()),
                        "aria-label": "Cancel and close dialog",
                        "{cancel_text}"
                    }
                    button {
                        class: if dangerous { "btn-danger" } else { "btn-primary" },
                        onclick: move |_| on_confirm.call(()),
                        "aria-label": if dangerous {
                            format!("{} (destructive action)", confirm_text)
                        } else {
                            confirm_text.clone()
                        },
                        autofocus: true,
                        "{confirm_text}"
                    }
                }
            }
        }
    }
}
