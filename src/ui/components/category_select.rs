use dioxus::prelude::*;
use serde::Deserialize;
use uuid::Uuid;
use crate::ui::http;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Category {
    pub id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub depth: i32,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct CategoryWithChildren {
    #[serde(flatten)]
    pub category: Category,
    pub children: Vec<CategoryWithChildren>,
}

#[component]
pub fn CategorySelect(
    selected_ids: Vec<Uuid>,
    on_change: EventHandler<Vec<Uuid>>,
) -> Element {
    let mut categories = use_signal(|| Vec::<CategoryWithChildren>::new());
    let mut loading = use_signal(|| true);
    let mut expanded = use_signal(|| false);

    use_effect(move || {
        spawn(async move {
            if let Ok(data) = http::get::<Vec<CategoryWithChildren>>("/api/categories/tree").await {
                categories.set(data);
            }
            loading.set(false);
        });
    });

    let all_cats: Vec<Category> = categories()
        .iter()
        .flat_map(|c| get_all_categories(c))
        .collect();

    let selected_names: Vec<String> = all_cats
        .iter()
        .filter(|c| selected_ids.contains(&c.id))
        .map(|c| c.name.clone())
        .collect();

    rsx! {
        div { class: "multi-select",
            div {
                class: "multi-select-header",
                onclick: move |_| expanded.set(!expanded()),
                if selected_names.is_empty() {
                    span { class: "placeholder", "Select categories..." }
                } else {
                    div { class: "chips",
                        for name in selected_names.iter() {
                            span { class: "chip category-chip", "{name}" }
                        }
                    }
                }
                span { class: "dropdown-arrow", if expanded() { "▲" } else { "▼" } }
            }

            if expanded() && !loading() {
                div { class: "multi-select-dropdown",
                    if categories().is_empty() {
                        div { class: "empty-message", "No categories" }
                    } else {
                        for cat in all_cats.iter() {
                            {
                                let cat_id = cat.id;
                                let cat_name = cat.name.clone();
                                let indent = cat.depth * 16;
                                let is_selected = selected_ids.contains(&cat_id);
                                let current_ids = selected_ids.clone();
                                rsx! {
                                    div {
                                        class: "category-item",
                                        style: "padding-left: {indent}px",
                                        label {
                                            input {
                                                r#type: "checkbox",
                                                checked: is_selected,
                                                onchange: move |_| {
                                                    let mut new_ids = current_ids.clone();
                                                    if new_ids.contains(&cat_id) {
                                                        new_ids.retain(|&x| x != cat_id);
                                                    } else {
                                                        new_ids.push(cat_id);
                                                    }
                                                    on_change.call(new_ids);
                                                },
                                            }
                                            " {cat_name}"
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

fn get_all_categories(cat: &CategoryWithChildren) -> Vec<Category> {
    let mut result = vec![cat.category.clone()];
    for child in &cat.children {
        result.extend(get_all_categories(child));
    }
    result
}
