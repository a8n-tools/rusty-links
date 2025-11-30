use crate::ui::api_client::{
    create_category, delete_category, fetch_categories, fetch_category, update_category,
    CategoryNode,
};
use crate::ui::components::loading::{LoadingSpinner, SpinnerSize};
use crate::ui::components::management::{AddCategoryInput, CategoryTreeNode};
use crate::ui::components::modal::ConfirmDialog;
use crate::ui::components::navbar::Navbar;
use dioxus::prelude::*;

#[component]
pub fn CategoriesPage() -> Element {
    let mut categories = use_signal(|| Vec::<CategoryNode>::new());
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| Option::<String>::None);

    // Add state
    let mut show_add_input = use_signal(|| false);
    let mut new_category_parent = use_signal(|| Option::<String>::None);
    let mut adding = use_signal(|| false);

    // Edit state
    let mut editing_id = use_signal(|| Option::<String>::None);
    let mut saving = use_signal(|| false);

    // Delete state
    let mut deleting_id = use_signal(|| Option::<String>::None);
    let mut delete_confirm_id = use_signal(|| Option::<String>::None);
    let mut delete_category_info = use_signal(|| Option::<CategoryNode>::None);

    // Fetch categories function
    let fetch = move || {
        spawn(async move {
            loading.set(true);
            error.set(None);

            match fetch_categories().await {
                Ok(cats) => {
                    categories.set(cats);
                    loading.set(false);
                }
                Err(err) => {
                    error.set(Some(err));
                    loading.set(false);
                }
            }
        });
    };

    // Fetch categories on mount
    use_effect(move || {
        fetch();
    });

    // Handle add category
    let handle_add = move |(name, parent_id): (String, Option<String>)| {
        spawn(async move {
            adding.set(true);
            error.set(None);

            match create_category(&name, parent_id).await {
                Ok(_) => {
                    show_add_input.set(false);
                    new_category_parent.set(None);
                    fetch();
                }
                Err(err) => {
                    error.set(Some(err));
                }
            }
            adding.set(false);
        });
    };

    // Handle edit save
    let handle_edit_save = move |(id, name): (String, String)| {
        spawn(async move {
            saving.set(true);
            error.set(None);

            match update_category(&id, &name).await {
                Ok(_) => {
                    editing_id.set(None);
                    fetch();
                }
                Err(err) => {
                    error.set(Some(err));
                    editing_id.set(None);
                }
            }
            saving.set(false);
        });
    };

    // Handle delete request
    let handle_delete_request = move |id: String| {
        let category_id = id.clone();
        spawn(async move {
            // Fetch category info for confirmation dialog
            match fetch_category(&category_id).await {
                Ok(cat) => {
                    delete_category_info.set(Some(cat));
                    delete_confirm_id.set(Some(category_id));
                }
                Err(err) => {
                    error.set(Some(err));
                }
            }
        });
    };

    // Handle delete confirm
    let handle_delete_confirm = move |_| {
        if let Some(id) = delete_confirm_id() {
            spawn(async move {
                deleting_id.set(Some(id.clone()));
                error.set(None);

                match delete_category(&id).await {
                    Ok(_) => {
                        delete_confirm_id.set(None);
                        delete_category_info.set(None);
                        fetch();
                    }
                    Err(err) => {
                        error.set(Some(err));
                        delete_confirm_id.set(None);
                        delete_category_info.set(None);
                    }
                }
                deleting_id.set(None);
            });
        }
    };

    rsx! {
        div { class: "page-container",
            Navbar {}

            div { class: "content-container",
                div { class: "page-header",
                    h1 { "Categories" }
                    button {
                        class: "btn-primary",
                        onclick: move |_| {
                            show_add_input.set(true);
                            new_category_parent.set(None);
                        },
                        disabled: show_add_input(),
                        "Add Category"
                    }
                }

                // Info message
                div { class: "info-box",
                    "ℹ️ Categories can be nested up to 3 levels deep. Click a category name to edit it."
                }

                // Error display
                if let Some(err) = error() {
                    div { class: "error-box",
                        "⚠️ {err}"
                    }
                }

                // Loading state
                if loading() {
                    LoadingSpinner {
                        size: SpinnerSize::Medium,
                        message: "Loading categories...".to_string()
                    }
                }
                else {
                    // Add input (if shown)
                    if show_add_input() {
                        AddCategoryInput {
                            parent_id: new_category_parent(),
                            on_add: handle_add,
                            on_cancel: move |_| {
                                show_add_input.set(false);
                                new_category_parent.set(None);
                            }
                        }
                    }

                    // Category tree
                    if categories().is_empty() {
                        div { class: "empty-state",
                            div { class: "empty-icon", "📁" }
                            div { class: "empty-title", "No categories yet" }
                            div { class: "empty-description",
                                "Click \"Add Category\" to create your first category."
                            }
                        }
                    } else {
                        div { class: "category-tree",
                            for node in categories() {
                                CategoryTreeNode {
                                    node: node.clone(),
                                    editing_id: editing_id(),
                                    on_edit_start: move |(id, _name): (String, String)| {
                                        editing_id.set(Some(id));
                                    },
                                    on_edit_save: handle_edit_save,
                                    on_edit_cancel: move |_| {
                                        editing_id.set(None);
                                    },
                                    on_delete: handle_delete_request,
                                    on_add_child: move |parent_id: String| {
                                        new_category_parent.set(Some(parent_id));
                                        show_add_input.set(true);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Delete confirmation dialog
            if delete_confirm_id().is_some() {
                if let Some(cat) = delete_category_info() {
                    ConfirmDialog {
                        title: "Delete Category".to_string(),
                        message: if cat.link_count > 0 {
                            format!(
                                "Are you sure you want to delete '{}'? It is assigned to {} link{}. The category will be removed from all links.",
                                cat.name,
                                cat.link_count,
                                if cat.link_count == 1 { "" } else { "s" }
                            )
                        } else {
                            format!("Are you sure you want to delete '{}'?", cat.name)
                        },
                        confirm_text: "Delete".to_string(),
                        cancel_text: "Cancel".to_string(),
                        dangerous: true,
                        on_confirm: handle_delete_confirm,
                        on_cancel: move |_| {
                            delete_confirm_id.set(None);
                            delete_category_info.set(None);
                        }
                    }
                }
            }
        }
    }
}
