use dioxus::prelude::*;
use crate::ui::components::navbar::Navbar;
use crate::ui::components::loading::{LoadingSpinner, SpinnerSize};
use crate::ui::components::management::{FlatListItem, AddItemInput};
use crate::ui::components::modal::ConfirmDialog;
use crate::ui::api_client::{
    LanguageItem, fetch_languages, fetch_language, create_language,
    update_language, delete_language
};

#[component]
pub fn LanguagesPage() -> Element {
    let mut languages = use_signal(|| Vec::<LanguageItem>::new());
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
    let mut delete_language_info = use_signal(|| Option::<LanguageItem>::None);

    // Fetch languages function
    let fetch = move || {
        spawn(async move {
            loading.set(true);
            error.set(None);

            match fetch_languages().await {
                Ok(langs) => {
                    languages.set(langs);
                    loading.set(false);
                },
                Err(err) => {
                    error.set(Some(err));
                    loading.set(false);
                }
            }
        });
    };

    // Fetch languages on mount
    use_effect(move || {
        fetch();
    });

    // Handle add language
    let handle_add = move |name: String| {
        spawn(async move {
            adding.set(true);
            error.set(None);

            match create_language(&name).await {
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

            match update_language(&id, &name).await {
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
        let language_id = id.clone();
        spawn(async move {
            // Fetch language info for confirmation dialog
            match fetch_language(&language_id).await {
                Ok(lang) => {
                    delete_language_info.set(Some(lang));
                    delete_confirm_id.set(Some(language_id));
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

                match delete_language(&id).await {
                    Ok(_) => {
                        delete_confirm_id.set(None);
                        delete_language_info.set(None);
                        fetch();
                    },
                    Err(err) => {
                        error.set(Some(err));
                        delete_confirm_id.set(None);
                        delete_language_info.set(None);
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
                    h1 { "Languages" }
                    button {
                        class: "btn-primary",
                        onclick: move |_| show_add_input.set(true),
                        disabled: show_add_input(),
                        "Add Language"
                    }
                }

                // Info message
                div { class: "info-box",
                    "ℹ️ Manage programming languages. Click a language name to edit it."
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
                        message: "Loading languages...".to_string()
                    }
                }
                else {
                    // Add input (if shown)
                    if show_add_input() {
                        AddItemInput {
                            placeholder: "Language name (e.g., Rust, Python)".to_string(),
                            on_add: handle_add,
                            on_cancel: move |_| show_add_input.set(false)
                        }
                    }

                    // Languages list
                    if languages().is_empty() {
                        div { class: "empty-state",
                            div { class: "empty-icon", "💻" }
                            div { class: "empty-title", "No languages yet" }
                            div { class: "empty-description",
                                "Click \"Add Language\" to create your first language."
                            }
                        }
                    } else {
                        div { class: "flat-list",
                            for lang in languages() {
                                FlatListItem {
                                    id: lang.id.clone(),
                                    name: lang.name.clone(),
                                    secondary_text: None,
                                    usage_count: lang.link_count,
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
                if let Some(lang) = delete_language_info() {
                    ConfirmDialog {
                        title: "Delete Language".to_string(),
                        message: if lang.link_count > 0 {
                            format!(
                                "Are you sure you want to delete '{}'? It is assigned to {} link{}. The language will be removed from all links.",
                                lang.name,
                                lang.link_count,
                                if lang.link_count == 1 { "" } else { "s" }
                            )
                        } else {
                            format!("Are you sure you want to delete '{}'?", lang.name)
                        },
                        confirm_text: "Delete".to_string(),
                        cancel_text: "Cancel".to_string(),
                        dangerous: true,
                        on_confirm: handle_delete_confirm,
                        on_cancel: move |_| {
                            delete_confirm_id.set(None);
                            delete_language_info.set(None);
                        }
                    }
                }
            }
        }
    }
}
