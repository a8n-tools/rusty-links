use dioxus::prelude::*;
use uuid::Uuid;
use crate::ui::components::search_filter::{FilterDropdown, FilterOption};

#[component]
pub fn FiltersContainer(
    // Languages
    languages: Vec<FilterOption>,
    selected_languages: Vec<Uuid>,
    on_languages_change: EventHandler<Vec<Uuid>>,

    // Licenses
    licenses: Vec<FilterOption>,
    selected_licenses: Vec<Uuid>,
    on_licenses_change: EventHandler<Vec<Uuid>>,

    // Categories
    categories: Vec<FilterOption>,
    selected_categories: Vec<Uuid>,
    on_categories_change: EventHandler<Vec<Uuid>>,

    // Tags
    tags: Vec<FilterOption>,
    selected_tags: Vec<Uuid>,
    on_tags_change: EventHandler<Vec<Uuid>>,

    // Reset
    on_reset: EventHandler<()>,
) -> Element {
    let has_active_filters =
        !selected_languages.is_empty() ||
        !selected_licenses.is_empty() ||
        !selected_categories.is_empty() ||
        !selected_tags.is_empty();

    rsx! {
        div { class: "filters-container",
            FilterDropdown {
                label: "Languages".to_string(),
                options: languages,
                selected: selected_languages,
                on_change: on_languages_change,
                searchable: true
            }

            FilterDropdown {
                label: "Licenses".to_string(),
                options: licenses,
                selected: selected_licenses,
                on_change: on_licenses_change,
                searchable: true
            }

            FilterDropdown {
                label: "Categories".to_string(),
                options: categories,
                selected: selected_categories,
                on_change: on_categories_change,
                searchable: true
            }

            FilterDropdown {
                label: "Tags".to_string(),
                options: tags,
                selected: selected_tags,
                on_change: on_tags_change,
                searchable: true
            }

            if has_active_filters {
                button {
                    class: "reset-filters",
                    onclick: move |_| on_reset.call(()),
                    "Reset Filters"
                }
            }
        }
    }
}
