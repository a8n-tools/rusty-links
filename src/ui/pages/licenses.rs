use crate::ui::api_client::{
    create_license, delete_license, fetch_license, fetch_licenses, update_license, LicenseItem,
};
use crate::ui::components::loading::{LoadingSpinner, SpinnerSize};
use crate::ui::components::management::FlatListItem;
use crate::ui::components::modal::ConfirmDialog;
use crate::ui::components::navbar::Navbar;
use dioxus::prelude::*;

#[component]
pub fn LicensesPage() -> Element {
    let mut licenses = use_signal(|| Vec::<LicenseItem>::new());
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
    let mut delete_license_info = use_signal(|| Option::<LicenseItem>::None);

    // Fetch licenses function
    let fetch = move || {
        spawn(async move {
            loading.set(true);
            error.set(None);

            match fetch_licenses().await {
                Ok(lics) => {
                    licenses.set(lics);
                    loading.set(false);
                }
                Err(err) => {
                    error.set(Some(err));
                    loading.set(false);
                }
            }
        });
    };

    // Fetch licenses on mount
    use_effect(move || {
        fetch();
    });

    // Handle add license
    let handle_add = move |(name, acronym): (String, Option<String>)| {
        spawn(async move {
            adding.set(true);
            error.set(None);

            match create_license(&name, acronym).await {
                Ok(_) => {
                    show_add_input.set(false);
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

            // For now, we only update the name, keep existing acronym
            // TODO: Support editing acronym in inline edit
            match update_license(&id, &name, None).await {
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
        let license_id = id.clone();
        spawn(async move {
            // Fetch license info for confirmation dialog
            match fetch_license(&license_id).await {
                Ok(lic) => {
                    delete_license_info.set(Some(lic));
                    delete_confirm_id.set(Some(license_id));
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

                match delete_license(&id).await {
                    Ok(_) => {
                        delete_confirm_id.set(None);
                        delete_license_info.set(None);
                        fetch();
                    }
                    Err(err) => {
                        error.set(Some(err));
                        delete_confirm_id.set(None);
                        delete_license_info.set(None);
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
                    h1 { "Licenses" }
                    button {
                        class: "btn-primary",
                        onclick: move |_| show_add_input.set(true),
                        disabled: show_add_input(),
                        "Add License"
                    }
                }

                // Info message
                div { class: "info-box",
                    "ℹ️ Manage software licenses. Click a license name to edit it."
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
                        message: "Loading licenses...".to_string()
                    }
                }
                else {
                    // Add input (if shown)
                    if show_add_input() {
                        AddLicenseInput {
                            on_add: handle_add,
                            on_cancel: move |_| show_add_input.set(false)
                        }
                    }

                    // Licenses list
                    if licenses().is_empty() {
                        div { class: "empty-state",
                            div { class: "empty-icon", "📜" }
                            div { class: "empty-title", "No licenses yet" }
                            div { class: "empty-description",
                                "Click \"Add License\" to create your first license."
                            }
                        }
                    } else {
                        div { class: "flat-list",
                            for lic in licenses() {
                                FlatListItem {
                                    id: lic.id.clone(),
                                    name: lic.name.clone(),
                                    secondary_text: lic.acronym.clone(),
                                    usage_count: lic.link_count,
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
                if let Some(lic) = delete_license_info() {
                    ConfirmDialog {
                        title: "Delete License".to_string(),
                        message: if lic.link_count > 0 {
                            format!(
                                "Are you sure you want to delete '{}'? It is assigned to {} link{}. The license will be removed from all links.",
                                lic.name,
                                lic.link_count,
                                if lic.link_count == 1 { "" } else { "s" }
                            )
                        } else {
                            format!("Are you sure you want to delete '{}'?", lic.name)
                        },
                        confirm_text: "Delete".to_string(),
                        cancel_text: "Cancel".to_string(),
                        dangerous: true,
                        on_confirm: handle_delete_confirm,
                        on_cancel: move |_| {
                            delete_confirm_id.set(None);
                            delete_license_info.set(None);
                        }
                    }
                }
            }
        }
    }
}

// Custom add license input component (supports both name and acronym)
#[component]
fn AddLicenseInput(
    on_add: EventHandler<(String, Option<String>)>,
    on_cancel: EventHandler<()>,
) -> Element {
    let mut name = use_signal(|| String::new());
    let mut acronym = use_signal(|| String::new());
    let mut error = use_signal(|| Option::<String>::None);

    let mut handle_submit = move || {
        let trimmed_name = name().trim().to_string();
        let trimmed_acronym = acronym().trim().to_string();

        if trimmed_name.is_empty() {
            error.set(Some("License name cannot be empty".to_string()));
            return;
        }

        let acronym_opt = if trimmed_acronym.is_empty() {
            None
        } else {
            Some(trimmed_acronym)
        };

        on_add.call((trimmed_name, acronym_opt));
    };

    rsx! {
        div { class: "add-item-input add-license-input-special",
            input {
                r#type: "text",
                class: "form-input",
                placeholder: "License name (e.g., MIT License)",
                value: "{name()}",
                autofocus: true,
                oninput: move |evt| {
                    name.set(evt.value());
                    error.set(None);
                },
                onkeydown: move |evt| {
                    if evt.key() == Key::Enter {
                        handle_submit();
                    } else if evt.key() == Key::Escape {
                        on_cancel.call(());
                    }
                }
            }

            input {
                r#type: "text",
                class: "form-input",
                placeholder: "Acronym (optional, e.g., MIT)",
                value: "{acronym()}",
                oninput: move |evt| {
                    acronym.set(evt.value());
                },
                onkeydown: move |evt| {
                    if evt.key() == Key::Enter {
                        handle_submit();
                    } else if evt.key() == Key::Escape {
                        on_cancel.call(());
                    }
                }
            }

            button {
                class: "btn-primary btn-sm",
                onclick: move |_| handle_submit(),
                "Add"
            }
            button {
                class: "btn-secondary btn-sm",
                onclick: move |_| on_cancel.call(()),
                "Cancel"
            }

            if let Some(err) = error() {
                div { class: "error-message", "{err}" }
            }
        }
    }
}
