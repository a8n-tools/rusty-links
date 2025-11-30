use crate::ui::http;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Tag {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize)]
struct CreateTagRequest {
    name: String,
}

#[component]
pub fn TagSelect(selected_ids: Vec<Uuid>, on_change: EventHandler<Vec<Uuid>>) -> Element {
    let mut tags = use_signal(|| Vec::<Tag>::new());
    let mut loading = use_signal(|| true);
    let mut expanded = use_signal(|| false);
    let mut search = use_signal(|| String::new());
    let mut creating = use_signal(|| false);

    use_effect(move || {
        spawn(async move {
            if let Ok(data) = http::get::<Vec<Tag>>("/api/tags").await {
                tags.set(data);
            }
            loading.set(false);
        });
    });

    let selected_names: Vec<String> = tags()
        .iter()
        .filter(|t| selected_ids.contains(&t.id))
        .map(|t| t.name.clone())
        .collect();

    let search_val = search();
    let filtered_tags: Vec<Tag> = tags()
        .iter()
        .filter(|t| {
            let s = search_val.to_lowercase();
            s.is_empty() || t.name.to_lowercase().contains(&s)
        })
        .cloned()
        .collect();

    let can_create = !search_val.trim().is_empty()
        && !tags()
            .iter()
            .any(|t| t.name.to_lowercase() == search_val.to_lowercase());

    rsx! {
        div { class: "multi-select",
            div {
                class: "multi-select-header",
                onclick: move |_| expanded.set(!expanded()),
                if selected_names.is_empty() {
                    span { class: "placeholder", "Select tags..." }
                } else {
                    div { class: "chips",
                        for name in selected_names.iter() {
                            span { class: "chip tag-chip", "{name}" }
                        }
                    }
                }
                span { class: "dropdown-arrow", if expanded() { "▲" } else { "▼" } }
            }

            if expanded() && !loading() {
                div { class: "multi-select-dropdown",
                    div { class: "tag-search",
                        input {
                            r#type: "text",
                            class: "form-input",
                            placeholder: "Search or create tag...",
                            value: "{search}",
                            oninput: move |evt| search.set(evt.value()),
                        }
                        if can_create {
                            button {
                                class: "btn btn-sm btn-primary",
                                disabled: creating(),
                                onclick: {
                                    let name = search_val.trim().to_string();
                                    let current_selected = selected_ids.clone();
                                    move |_| {
                                        let name = name.clone();
                                        let current_selected = current_selected.clone();
                                        if name.is_empty() || creating() {
                                            return;
                                        }
                                        spawn(async move {
                                            creating.set(true);
                                            let request = CreateTagRequest { name: name.clone() };

                                            if let Ok(new_tag) = http::post::<Tag, _>("/api/tags", &request).await {
                                                let mut current_tags = tags();
                                                current_tags.push(new_tag.clone());
                                                current_tags.sort_by(|a, b| a.name.cmp(&b.name));
                                                tags.set(current_tags);

                                                let mut new_selected = current_selected.clone();
                                                new_selected.push(new_tag.id);
                                                on_change.call(new_selected);

                                                search.set(String::new());
                                            }
                                            creating.set(false);
                                        });
                                    }
                                },
                                if creating() { "..." } else { "+" }
                            }
                        }
                    }

                    if filtered_tags.is_empty() {
                        div { class: "empty-message", "No tags found" }
                    } else {
                        for tag in filtered_tags.iter() {
                            {
                                let tag_id = tag.id;
                                let tag_name = tag.name.clone();
                                let is_selected = selected_ids.contains(&tag_id);
                                let current_ids = selected_ids.clone();
                                rsx! {
                                    div { class: "tag-item",
                                        label {
                                            input {
                                                r#type: "checkbox",
                                                checked: is_selected,
                                                onchange: move |_| {
                                                    let mut new_ids = current_ids.clone();
                                                    if new_ids.contains(&tag_id) {
                                                        new_ids.retain(|&x| x != tag_id);
                                                    } else {
                                                        new_ids.push(tag_id);
                                                    }
                                                    on_change.call(new_ids);
                                                },
                                            }
                                            " {tag_name}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
