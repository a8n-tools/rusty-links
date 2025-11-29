use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::ui::http;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct License {
    pub id: Uuid,
    pub name: String,
    pub is_global: bool,
}

#[derive(Debug, Serialize)]
struct CreateLicenseRequest {
    name: String,
}

#[component]
pub fn LicenseSelect(
    selected_ids: Vec<Uuid>,
    on_change: EventHandler<Vec<Uuid>>,
) -> Element {
    let mut licenses = use_signal(|| Vec::<License>::new());
    let mut loading = use_signal(|| true);
    let mut expanded = use_signal(|| false);
    let mut search = use_signal(|| String::new());
    let mut creating = use_signal(|| false);

    use_effect(move || {
        spawn(async move {
            if let Ok(data) = http::get::<Vec<License>>("/api/licenses").await {
                licenses.set(data);
            }
            loading.set(false);
        });
    });

    let selected_names: Vec<String> = licenses()
        .iter()
        .filter(|l| selected_ids.contains(&l.id))
        .map(|l| l.name.clone())
        .collect();

    let search_val = search();
    let filtered: Vec<License> = licenses()
        .iter()
        .filter(|l| {
            let s = search_val.to_lowercase();
            s.is_empty() || l.name.to_lowercase().contains(&s)
        })
        .cloned()
        .collect();

    let can_create = !search_val.trim().is_empty()
        && !licenses().iter().any(|l| l.name.to_lowercase() == search_val.to_lowercase());

    rsx! {
        div { class: "multi-select",
            div {
                class: "multi-select-header",
                onclick: move |_| expanded.set(!expanded()),
                if selected_names.is_empty() {
                    span { class: "placeholder", "Select license..." }
                } else {
                    div { class: "chips",
                        for name in selected_names.iter() {
                            span { class: "chip license-chip", "{name}" }
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
                                            let request = CreateLicenseRequest { name: name.clone() };
                                            if let Ok(new_lic) = http::post::<License, _>("/api/licenses", &request).await {
                                                let mut current = licenses();
                                                current.push(new_lic.clone());
                                                current.sort_by(|a, b| a.name.cmp(&b.name));
                                                licenses.set(current);
                                                let mut new_selected = current_selected.clone();
                                                new_selected.push(new_lic.id);
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
                        div { class: "empty-message", "No licenses found" }
                    } else {
                        for lic in filtered.iter() {
                            {
                                let lic_id = lic.id;
                                let lic_name = lic.name.clone();
                                let is_selected = selected_ids.contains(&lic_id);
                                let current_ids = selected_ids.clone();
                                rsx! {
                                    div { class: "select-item",
                                        label {
                                            input {
                                                r#type: "checkbox",
                                                checked: is_selected,
                                                onchange: move |_| {
                                                    let mut new_ids = current_ids.clone();
                                                    if new_ids.contains(&lic_id) {
                                                        new_ids.retain(|&x| x != lic_id);
                                                    } else {
                                                        new_ids.push(lic_id);
                                                    }
                                                    on_change.call(new_ids);
                                                },
                                            }
                                            " {lic_name}"
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
