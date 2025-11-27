use dioxus::prelude::*;

#[component]
pub fn InlineEditInput(
    value: String,
    on_save: EventHandler<String>,
    on_cancel: EventHandler<()>,
) -> Element {
    let mut input_value = use_signal(|| value.clone());

    rsx! {
        div { class: "inline-edit",
            input {
                r#type: "text",
                class: "inline-edit-input",
                value: "{input_value()}",
                autofocus: true,
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
            button {
                class: "btn-icon btn-save",
                onclick: move |_| on_save.call(input_value()),
                "✓"
            }
            button {
                class: "btn-icon btn-cancel",
                onclick: move |_| on_cancel.call(()),
                "×"
            }
        }
    }
}
