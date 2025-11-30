use dioxus::prelude::*;
use uuid::Uuid;

#[derive(Clone, PartialEq, Debug)]
pub struct FilterOption {
    pub id: Uuid,
    pub label: String,
}

#[component]
pub fn FilterDropdown(
    label: String,
    options: Vec<FilterOption>,
    selected: Vec<Uuid>,
    on_change: EventHandler<Vec<Uuid>>,
    #[props(default = true)] searchable: bool,
) -> Element {
    let mut is_open = use_signal(|| false);
    let mut search_query = use_signal(|| String::new());

    // Clone options for use in closures
    let options_for_filter = options.clone();
    let options_for_label = options.clone();

    // Filtered options based on search
    let filtered_options = use_memo(move || {
        if search_query().is_empty() {
            options_for_filter.clone()
        } else {
            options_for_filter
                .iter()
                .filter(|opt| {
                    opt.label
                        .to_lowercase()
                        .contains(&search_query().to_lowercase())
                })
                .cloned()
                .collect()
        }
    });

    rsx! {
        div { class: "filter-dropdown",
            button {
                class: "filter-button",
                onclick: move |_| is_open.set(!is_open()),
                "{label} ({selected.len()})"
            }

            if is_open() {
                div { class: "filter-menu",
                    // Selected chips
                    if !selected.is_empty() {
                        div { class: "filter-chips",
                            for sel_id in selected.clone() {
                                {
                                    let chip_label = options_for_label.iter()
                                        .find(|opt| opt.id == sel_id)
                                        .map(|opt| opt.label.clone())
                                        .unwrap_or_else(|| sel_id.to_string());
                                    let selected_clone = selected.clone();

                                    rsx! {
                                        div {
                                            key: "{sel_id}",
                                            class: "filter-chip",
                                            span { "{chip_label}" }
                                            button {
                                                onclick: move |_| {
                                                    let mut current = selected_clone.clone();
                                                    current.retain(|&x| x != sel_id);
                                                    on_change.call(current);
                                                },
                                                "×"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Search input (if searchable)
                    if searchable {
                        input {
                            class: "filter-search",
                            r#type: "text",
                            placeholder: "Search...",
                            value: "{search_query()}",
                            oninput: move |evt| search_query.set(evt.value())
                        }
                    }

                    // Options list
                    div { class: "filter-options",
                        for option in filtered_options() {
                            {
                                let selected_clone = selected.clone();
                                let option_id = option.id;
                                let option_label = option.label.clone();

                                rsx! {
                                    label {
                                        key: "{option_id}",
                                        class: "filter-option",
                                        input {
                                            r#type: "checkbox",
                                            checked: selected_clone.contains(&option_id),
                                            onchange: move |_| {
                                                let mut current = selected_clone.clone();
                                                if current.contains(&option_id) {
                                                    current.retain(|&x| x != option_id);
                                                } else {
                                                    current.push(option_id);
                                                }
                                                on_change.call(current);
                                            }
                                        }
                                        "{option_label}"
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
