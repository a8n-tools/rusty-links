use dioxus::prelude::*;
use crate::ui::components::management::InlineEditInput;

#[component]
pub fn FlatListItem(
    id: String,
    name: String,
    secondary_text: Option<String>,
    usage_count: i64,
    editing_id: Option<String>,
    on_edit_start: EventHandler<(String, String)>,
    on_edit_save: EventHandler<(String, String)>,
    on_edit_cancel: EventHandler<()>,
    on_delete: EventHandler<String>,
) -> Element {
    let is_editing = editing_id.as_ref() == Some(&id);
    let mut edit_value = use_signal(|| name.clone());

    // Clone IDs upfront
    let id_for_save = id.clone();
    let id_for_edit = id.clone();
    let id_for_delete = id.clone();
    let name_clone = name.clone();

    rsx! {
        div { class: "flat-list-item",
            div { class: "flat-list-item-content",
                // Name or edit input
                if is_editing {
                    InlineEditInput {
                        value: edit_value(),
                        on_save: move |new_name: String| {
                            on_edit_save.call((id_for_save.clone(), new_name));
                        },
                        on_cancel: on_edit_cancel
                    }
                } else {
                    div { class: "item-info",
                        div {
                            class: "item-name",
                            onclick: move |_| {
                                edit_value.set(name_clone.clone());
                                on_edit_start.call((id_for_edit.clone(), name_clone.clone()));
                            },
                            "{name}"
                        }
                        if let Some(secondary) = secondary_text {
                            div { class: "item-secondary", "{secondary}" }
                        }
                    }
                }

                // Usage count
                div { class: "usage-count",
                    "{usage_count} "
                    if usage_count == 1 { "link" } else { "links" }
                }

                // Delete button
                if !is_editing {
                    button {
                        class: "btn-icon btn-delete",
                        title: "Delete",
                        onclick: move |_| on_delete.call(id_for_delete.clone()),
                        "×"
                    }
                }
            }
        }
    }
}
