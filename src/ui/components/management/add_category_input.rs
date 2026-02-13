use dioxus::prelude::*;

#[component]
pub fn AddCategoryInput(
    parent_id: Option<String>,
    on_add: EventHandler<(String, Option<String>)>,
    on_cancel: EventHandler<()>,
) -> Element {
    let mut name = use_signal(String::new);
    let mut error = use_signal(|| Option::<String>::None);

    // Clone parent_id for use in closures
    let parent_id_for_submit = parent_id.clone();
    let parent_id_for_keydown = parent_id.clone();

    let mut handle_submit = move || {
        let trimmed = name().trim().to_string();

        if trimmed.is_empty() {
            error.set(Some("Category name cannot be empty".to_string()));
            return;
        }

        on_add.call((trimmed, parent_id_for_submit.clone()));
    };

    rsx! {
        div { class: "add-category-input",
            input {
                r#type: "text",
                class: "form-input",
                placeholder: "Category name",
                value: "{name()}",
                autofocus: true,
                oninput: move |evt| {
                    name.set(evt.value());
                    error.set(None);
                },
                onkeydown: move |evt| {
                    if evt.key() == Key::Enter {
                        let trimmed = name().trim().to_string();
                        if !trimmed.is_empty() {
                            on_add.call((trimmed, parent_id_for_keydown.clone()));
                        } else {
                            error.set(Some("Category name cannot be empty".to_string()));
                        }
                    } else if evt.key() == Key::Escape {
                        on_cancel.call(());
                    }
                }
            }

            button {
                class: "btn-primary btn-sm",
                onclick: move |_| handle_submit(),
                "Add"
            }
            button {
                class: "btn-secondary btn-sm",
                onclick: move |_| on_cancel.call(()),
                "Cancel"
            }

            if let Some(err) = error() {
                div { class: "error-message", "{err}" }
            }
        }
    }
}
