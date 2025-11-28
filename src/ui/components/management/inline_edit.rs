use dioxus::prelude::*;

#[component]
pub fn InlineEditInput(
    value: String,
    on_save: EventHandler<String>,
    on_cancel: EventHandler<()>,
) -> Element {
    let mut input_value = use_signal(|| value.clone());

    rsx! {
        div {
            class: "inline-edit",
            role: "group",
            "aria-label": "Inline edit mode",
            input {
                r#type: "text",
                class: "inline-edit-input",
                value: "{input_value()}",
                autofocus: true,
                "aria-label": "Edit item name",
                "aria-describedby": "edit-hint",
                oninput: move |evt| input_value.set(evt.value()),
                onkeydown: move |evt| {
                    if evt.key() == Key::Enter {
                        on_save.call(input_value());
                    } else if evt.key() == Key::Escape {
                        on_cancel.call(());
                    }
                },
                onblur: move |_| on_save.call(input_value())
            }
            span {
                id: "edit-hint",
                class: "sr-only",
                "Press Enter to save, Escape to cancel"
            }
            button {
                class: "btn-icon btn-save",
                onclick: move |_| on_save.call(input_value()),
                "aria-label": "Save changes",
                title: "Save (Enter)",
                span { "aria-hidden": "true", "✓" }
            }
            button {
                class: "btn-icon btn-cancel",
                onclick: move |_| on_cancel.call(()),
                "aria-label": "Cancel editing",
                title: "Cancel (Escape)",
                span { "aria-hidden": "true", "×" }
            }
        }
    }
}
