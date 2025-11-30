use crate::ui::http;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Language {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub link_count: i64,
}

#[derive(Debug, Serialize)]
struct CreateLanguageRequest {
    name: String,
}

#[component]
pub fn LanguageSelect(selected_ids: Vec<Uuid>, on_change: EventHandler<Vec<Uuid>>) -> Element {
    let mut languages = use_signal(|| Vec::<Language>::new());
    let mut loading = use_signal(|| true);
    let mut expanded = use_signal(|| false);
    let mut search = use_signal(|| String::new());
    let mut creating = use_signal(|| false);

    use_effect(move || {
        spawn(async move {
            if let Ok(data) = http::get::<Vec<Language>>("/api/languages").await {
                languages.set(data);
            }
            loading.set(false);
        });
    });

    let selected_names: Vec<String> = languages()
        .iter()
        .filter(|l| selected_ids.contains(&l.id))
        .map(|l| l.name.clone())
        .collect();

    let search_val = search();
    let filtered: Vec<Language> = languages()
        .iter()
        .filter(|l| {
            let s = search_val.to_lowercase();
            s.is_empty() || l.name.to_lowercase().contains(&s)
        })
        .cloned()
        .collect();

    let can_create = !search_val.trim().is_empty()
        && !languages()
            .iter()
            .any(|l| l.name.to_lowercase() == search_val.to_lowercase());

    rsx! {
        div { class: "multi-select",
            div {
                class: "multi-select-header",
                onclick: move |_| expanded.set(!expanded()),
                if selected_names.is_empty() {
                    span { class: "placeholder", "Select languages..." }
                } else {
                    div { class: "chips",
                        for name in selected_names.iter() {
                            span { class: "chip language-chip", "{name}" }
                        }
                    }
                }
                span { class: "dropdown-arrow", if expanded() { "▲" } else { "▼" } }
            }

            if expanded() && !loading() {
                div { class: "multi-select-dropdown",
                    div { class: "select-search",
                        input {
                            r#type: "text",
                            class: "form-input",
                            placeholder: "Search or create...",
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
                                            let request = CreateLanguageRequest { name: name.clone() };
                                            if let Ok(new_lang) = http::post::<Language, _>("/api/languages", &request).await {
                                                let mut current = languages();
                                                current.push(new_lang.clone());
                                                current.sort_by(|a, b| a.name.cmp(&b.name));
                                                languages.set(current);
                                                let mut new_selected = current_selected.clone();
                                                new_selected.push(new_lang.id);
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

                    if filtered.is_empty() {
                        div { class: "empty-message", "No languages found" }
                    } else {
                        for lang in filtered.iter() {
                            {
                                let lang_id = lang.id;
                                let lang_name = lang.name.clone();
                                let is_selected = selected_ids.contains(&lang_id);
                                let current_ids = selected_ids.clone();
                                rsx! {
                                    div { class: "select-item",
                                        label {
                                            input {
                                                r#type: "checkbox",
                                                checked: is_selected,
                                                onchange: move |_| {
                                                    let mut new_ids = current_ids.clone();
                                                    if new_ids.contains(&lang_id) {
                                                        new_ids.retain(|&x| x != lang_id);
                                                    } else {
                                                        new_ids.push(lang_id);
                                                    }
                                                    on_change.call(new_ids);
                                                },
                                            }
                                            " {lang_name}"
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
