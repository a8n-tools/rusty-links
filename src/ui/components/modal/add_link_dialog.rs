use dioxus::prelude::*;
use crate::ui::components::modal::ModalBase;
use crate::ui::components::table::links_table::Link;
use crate::ui::utils::is_valid_url;
use crate::ui::api_client::{check_duplicate_url, create_link_request};

#[component]
pub fn AddLinkDialog(
    initial_url: String,
    on_close: EventHandler<()>,
    on_success: EventHandler<Link>,
    on_duplicate: EventHandler<Link>,
) -> Element {
    let mut url_input = use_signal(|| initial_url.clone());
    let mut validating = use_signal(|| false);
    let mut creating = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    let mut validation_error = use_signal(|| Option::<String>::None);

    // Validation
    let mut validate_url = move || {
        let url = url_input();

        // Basic format check
        if url.is_empty() {
            validation_error.set(Some("URL is required".to_string()));
            return false;
        }

        if !url.starts_with("http://") && !url.starts_with("https://") {
            validation_error.set(Some("URL must start with http:// or https://".to_string()));
            return false;
        }

        if !is_valid_url(&url) {
            validation_error.set(Some("Invalid URL format".to_string()));
            return false;
        }

        validation_error.set(None);
        true
    };

    // Handle submit
    let mut handle_submit = move |_| {
        if !validate_url() {
            return;
        }

        let url = url_input();

        spawn(async move {
            validating.set(true);
            error.set(None);

            // Check for duplicates
            match check_duplicate_url(&url).await {
                Ok(Some(existing_link)) => {
                    // Duplicate found - notify parent
                    validating.set(false);
                    on_duplicate.call(existing_link);
                },
                Ok(None) => {
                    // No duplicate - proceed to create
                    creating.set(true);
                    validating.set(false);

                    match create_link_request(&url).await {
                        Ok(link) => {
                            creating.set(false);
                            on_success.call(link);
                        },
                        Err(err) => {
                            error.set(Some(err));
                            creating.set(false);
                        }
                    }
                },
                Err(err) => {
                    error.set(Some(format!("Error checking for duplicates: {}", err)));
                    validating.set(false);
                }
            }
        });
    };

    rsx! {
        ModalBase {
            on_close: on_close,

            div { class: "add-link-dialog",
                div { class: "dialog-header",
                    h2 { "Add Link" }
                    button {
                        class: "close-button",
                        onclick: move |_| on_close.call(()),
                        "×"
                    }
                }

                div { class: "dialog-body",
                    // URL Input
                    div { class: "form-group",
                        label { "URL" }
                        input {
                            r#type: "url",
                            class: "url-input",
                            value: "{url_input()}",
                            placeholder: "https://example.com",
                            autofocus: true,
                            oninput: move |evt| {
                                url_input.set(evt.value());
                                error.set(None);
                                validation_error.set(None);
                            },
                            onkeypress: move |evt| {
                                if evt.key() == Key::Enter {
                                    handle_submit(());
                                }
                            }
                        }

                        // Validation error
                        if let Some(err) = validation_error() {
                            div { class: "error-message", "{err}" }
                        }
                    }

                    // Error display
                    if let Some(err) = error() {
                        div { class: "error-box",
                            "⚠️ {err}"
                        }
                    }

                    // Info about inaccessible links
                    div { class: "info-box",
                        "ℹ️ If the URL is not accessible from this location (e.g., internal link), you'll see a warning but the link will still be saved."
                    }
                }

                div { class: "dialog-footer",
                    button {
                        class: "btn-secondary",
                        onclick: move |_| on_close.call(()),
                        disabled: creating() || validating(),
                        "Cancel"
                    }
                    button {
                        class: "btn-primary",
                        onclick: move |_| handle_submit(()),
                        disabled: creating() || validating() || url_input().is_empty(),
                        if creating() {
                            "Creating..."
                        } else if validating() {
                            "Checking..."
                        } else {
                            "Add Link"
                        }
                    }
                }
            }
        }
    }
}
