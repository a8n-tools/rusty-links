use dioxus::prelude::*;

#[component]
pub fn TableHeader(
    column: String,
    label: String,
    sortable: bool,
    current_sort_by: String,
    current_sort_order: String,
    on_sort: EventHandler<String>,
) -> Element {
    let is_sorted = current_sort_by == column;
    let sort_arrow = if is_sorted {
        if current_sort_order == "asc" {
            " ▲"
        } else {
            " ▼"
        }
    } else {
        ""
    };

    rsx! {
        th {
            class: if sortable { "sortable" } else { "" },
            onclick: move |_| {
                if sortable {
                    on_sort.call(column.clone());
                }
            },
            "{label}{sort_arrow}"
        }
    }
}
