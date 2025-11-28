use dioxus::prelude::*;
use crate::ui::components::navbar::Navbar;
use crate::ui::components::skip_link::SkipLink;
use crate::ui::components::loading::{LoadingSpinner, SpinnerSize};
use crate::ui::components::management::{FlatListItem, AddItemInput};
use crate::ui::components::modal::ConfirmDialog;
use crate::ui::api_client::{
    TagItem, fetch_tags, fetch_tag, create_tag,
    update_tag, delete_tag
};

#[component]
pub fn TagsPage() -> Element {
    let mut tags = use_signal(|| Vec::<TagItem>::new());
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| Option::<String>::None);

    // Add state
    let mut show_add_input = use_signal(|| false);
    let mut adding = use_signal(|| false);

    // Edit state
    let mut editing_id = use_signal(|| Option::<String>::None);
    let mut saving = use_signal(|| false);

    // Delete state
    let mut deleting_id = use_signal(|| Option::<String>::None);
    let mut delete_confirm_id = use_signal(|| Option::<String>::None);
    let mut delete_tag_info = use_signal(|| Option::<TagItem>::None);

    // Fetch tags function
    let fetch = move || {
        spawn(async move {
            loading.set(true);
            error.set(None);

            match fetch_tags().await {
                Ok(tgs) => {
                    tags.set(tgs);
                    loading.set(false);
                },
                Err(err) => {
                    error.set(Some(err));
                    loading.set(false);
                }
            }
        });
    };

    // Fetch tags on mount
    use_effect(move || {
        fetch();
    });

    // Handle add tag
    let handle_add = move |name: String| {
        spawn(async move {
            adding.set(true);
            error.set(None);

            match create_tag(&name).await {
                Ok(_) => {
                    show_add_input.set(false);
                    fetch();
                },
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

            match update_tag(&id, &name).await {
                Ok(_) => {
                    editing_id.set(None);
                    fetch();
                },
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
        let tag_id = id.clone();
        spawn(async move {
            // Fetch tag info for confirmation dialog
            match fetch_tag(&tag_id).await {
                Ok(tag) => {
                    delete_tag_info.set(Some(tag));
                    delete_confirm_id.set(Some(tag_id));
                },
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

                match delete_tag(&id).await {
                    Ok(_) => {
                        delete_confirm_id.set(None);
                        delete_tag_info.set(None);
                        fetch();
                    },
                    Err(err) => {
                        error.set(Some(err));
                        delete_confirm_id.set(None);
                        delete_tag_info.set(None);
                    }
                }
                deleting_id.set(None);
            });
        }
    };

    rsx! {
        div { class: "page-container",
            SkipLink {}
            Navbar {}

            main {
                id: "main-content",
                class: "content-container",
                role: "main",
                "aria-label": "Tags management",
                div { class: "page-header",
                    h1 { "Tags" }
                    button {
                        class: "btn-primary",
                        onclick: move |_| show_add_input.set(true),
                        disabled: show_add_input(),
                        "Add Tag"
                    }
                }

                // Info message
                div { class: "info-box",
                    "ℹ️ Manage tags for organizing your links. Click a tag name to edit it."
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
                        message: "Loading tags...".to_string()
                    }
                }
                else {
                    // Add input (if shown)
                    if show_add_input() {
                        AddItemInput {
                            placeholder: "Tag name (e.g., tutorial, cli-tool)".to_string(),
                            on_add: handle_add,
                            on_cancel: move |_| show_add_input.set(false)
                        }
                    }

                    // Tags list
                    if tags().is_empty() {
                        div { class: "empty-state",
                            div { class: "empty-icon", "🏷️" }
                            div { class: "empty-title", "No tags yet" }
                            div { class: "empty-description",
                                "Click \"Add Tag\" to create your first tag."
                            }
                        }
                    } else {
                        div { class: "flat-list",
                            for tag in tags() {
                                FlatListItem {
                                    id: tag.id.clone(),
                                    name: tag.name.clone(),
                                    secondary_text: None,
                                    usage_count: tag.link_count,
                                    editing_id: editing_id(),
                                    on_edit_start: move |(id, _name): (String, String)| {
                                        editing_id.set(Some(id));
                                    },
                                    on_edit_save: handle_edit_save,
                                    on_edit_cancel: move |_| {
                                        editing_id.set(None);
                                    },
                                    on_delete: handle_delete_request
                                }
                            }
                        }
                    }
                }
            }

            // Delete confirmation dialog
            if delete_confirm_id().is_some() {
                if let Some(tag) = delete_tag_info() {
                    ConfirmDialog {
                        title: "Delete Tag".to_string(),
                        message: if tag.link_count > 0 {
                            format!(
                                "Are you sure you want to delete '{}'? It is assigned to {} link{}. The tag will be removed from all links.",
                                tag.name,
                                tag.link_count,
                                if tag.link_count == 1 { "" } else { "s" }
                            )
                        } else {
                            format!("Are you sure you want to delete '{}'?", tag.name)
                        },
                        confirm_text: "Delete".to_string(),
                        cancel_text: "Cancel".to_string(),
                        dangerous: true,
                        on_confirm: handle_delete_confirm,
                        on_cancel: move |_| {
                            delete_confirm_id.set(None);
                            delete_tag_info.set(None);
                        }
                    }
                }
            }
        }
    }
}
