use crate::ui::api_client::CategoryNode;
use crate::ui::components::management::InlineEditInput;
use dioxus::prelude::*;

#[component]
pub fn CategoryTreeNode(
    node: CategoryNode,
    editing_id: Option<String>,
    on_edit_start: EventHandler<(String, String)>,
    on_edit_save: EventHandler<(String, String)>,
    on_edit_cancel: EventHandler<()>,
    on_delete: EventHandler<String>,
    on_add_child: EventHandler<String>,
) -> Element {
    let is_editing = editing_id.as_ref() == Some(&node.id);
    let mut edit_value = use_signal(|| node.name.clone());

    // Clone IDs upfront to avoid move issues
    let node_id_for_save = node.id.clone();
    let node_id_for_edit = node.id.clone();
    let node_id_for_add = node.id.clone();
    let node_id_for_delete = node.id.clone();
    let node_name = node.name.clone();

    rsx! {
        div {
            class: "tree-node",
            "data-depth": "{node.depth}",

            div {
                class: "tree-node-content",
                style: "padding-left: {node.depth * 24}px",

                // Drag handle (visual only for now)
                div { class: "drag-handle", "⋮⋮" }

                // Category name or edit input
                if is_editing {
                    InlineEditInput {
                        value: edit_value(),
                        on_save: move |new_name: String| {
                            on_edit_save.call((node_id_for_save.clone(), new_name));
                        },
                        on_cancel: on_edit_cancel
                    }
                } else {
                    div {
                        class: "category-name",
                        onclick: move |_| {
                            edit_value.set(node_name.clone());
                            on_edit_start.call((node_id_for_edit.clone(), node_name.clone()));
                        },
                        "{node.name}"
                    }
                }

                // Usage count
                div { class: "usage-count",
                    "{node.link_count} "
                    if node.link_count == 1 { "link" } else { "links" }
                }

                // Action buttons
                div { class: "tree-node-actions",
                    // Add child button (only if depth < 2)
                    if node.depth < 2 {
                        button {
                            class: "btn-icon btn-add",
                            title: "Add subcategory",
                            onclick: move |_| on_add_child.call(node_id_for_add.clone()),
                            "+"
                        }
                    }

                    // Delete button
                    button {
                        class: "btn-icon btn-delete",
                        title: "Delete category",
                        onclick: move |_| on_delete.call(node_id_for_delete.clone()),
                        "×"
                    }
                }
            }

            // Children
            if !node.children.is_empty() {
                div { class: "tree-children",
                    for child in &node.children {
                        CategoryTreeNode {
                            node: child.clone(),
                            editing_id: editing_id.clone(),
                            on_edit_start: on_edit_start,
                            on_edit_save: on_edit_save,
                            on_edit_cancel: on_edit_cancel,
                            on_delete: on_delete,
                            on_add_child: on_add_child
                        }
                    }
                }
            }
        }
    }
}
