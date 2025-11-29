use dioxus::prelude::*;
use uuid::Uuid;
use crate::ui::components::modal::ModalBase;
use crate::ui::components::table::links_table::Link;
use crate::ui::components::category_select::CategorySelect;
use crate::ui::components::tag_select::TagSelect;
use crate::ui::components::language_select::LanguageSelect;
use crate::ui::components::license_select::LicenseSelect;
use crate::ui::utils::is_valid_url;
use crate::ui::api_client::{
    check_duplicate_url, create_link_with_categories, preview_link,
    fetch_languages, fetch_licenses,
    LinkPreview, CreateLinkWithCategoriesRequest
};

/// Format error messages to be more user-friendly
fn format_error(error: &str) -> String {
    // Network errors
    if error.contains("Network error") || error.contains("fetch") {
        return "Unable to connect. Please check your internet connection and try again.".to_string();
    }

    // Timeout errors
    if error.contains("timeout") || error.contains("Timeout") {
        return "The request timed out. The server might be busy - please try again.".to_string();
    }

    // Server errors (5xx)
    if error.contains("500") || error.contains("Internal Server Error") {
        return "Something went wrong on our end. Please try again later.".to_string();
    }
    if error.contains("502") || error.contains("503") || error.contains("504") {
        return "The server is temporarily unavailable. Please try again in a moment.".to_string();
    }

    // Client errors (4xx)
    if error.contains("400") || error.contains("Bad Request") {
        return "Invalid request. Please check the URL and try again.".to_string();
    }
    if error.contains("401") || error.contains("Unauthorized") {
        return "You need to log in to perform this action.".to_string();
    }
    if error.contains("403") || error.contains("Forbidden") {
        return "You don't have permission to access this resource.".to_string();
    }
    if error.contains("404") || error.contains("Not Found") {
        return "The requested resource was not found.".to_string();
    }
    if error.contains("429") || error.contains("Too Many Requests") {
        return "Too many requests. Please wait a moment and try again.".to_string();
    }

    // URL-specific errors
    if error.contains("Invalid URL") || error.contains("invalid url") {
        return "The URL format is invalid. Please check and try again.".to_string();
    }
    if error.contains("could not resolve") || error.contains("DNS") {
        return "Could not find the website. Please check the URL is correct.".to_string();
    }

    // Metadata fetch errors
    if error.contains("Failed to fetch preview") {
        return "Could not load preview. The website might be unavailable or blocking requests.".to_string();
    }

    // Default: return original error but clean it up
    error.trim().to_string()
}

#[derive(Clone, PartialEq)]
enum ProgressStep {
    ValidatingUrl,
    CheckingDuplicates,
    FetchingMetadata,
    CreatingLink,
}

fn step_label(step: &ProgressStep) -> &'static str {
    match step {
        ProgressStep::ValidatingUrl => "Validating URL...",
        ProgressStep::CheckingDuplicates => "Checking for duplicates...",
        ProgressStep::FetchingMetadata => "Fetching metadata...",
        ProgressStep::CreatingLink => "Creating link...",
    }
}

#[component]
fn ProgressIndicator(current_step: ProgressStep) -> Element {
    let steps = [
        ProgressStep::ValidatingUrl,
        ProgressStep::CheckingDuplicates,
        ProgressStep::FetchingMetadata,
    ];

    let current_index = steps.iter().position(|s| *s == current_step);

    rsx! {
        div {
            class: "progress-steps",
            role: "status",
            aria_live: "polite",
            aria_label: "Loading progress",
            for (i, step) in steps.iter().enumerate() {
                {
                    let is_current = *step == current_step;
                    let is_complete = current_index.map(|idx| idx > i).unwrap_or(false);
                    rsx! {
                        div {
                            key: "{i}",
                            class: "progress-step",
                            class: if is_current { "progress-step-active" },
                            class: if is_complete { "progress-step-complete" },
                            aria_current: if is_current { "step" } else { "" },
                            div {
                                class: "progress-step-indicator",
                                aria_hidden: "true",
                                if is_complete {
                                    "✓"
                                } else {
                                    "{i + 1}"
                                }
                            }
                            span { class: "progress-step-label", "{step_label(step)}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CreatingIndicator() -> Element {
    rsx! {
        div {
            class: "progress-steps",
            role: "status",
            aria_live: "polite",
            aria_label: "Creating link",
            div { class: "progress-step progress-step-active",
                div { class: "progress-step-indicator", aria_hidden: "true", "1" }
                span { class: "progress-step-label", "{step_label(&ProgressStep::CreatingLink)}" }
            }
        }
    }
}

#[component]
fn KeyboardHints(show_save: bool) -> Element {
    rsx! {
        div { class: "keyboard-hints",
            span { class: "hint",
                kbd { "Enter" }
                " Preview"
            }
            if show_save {
                span { class: "hint",
                    kbd { "Ctrl" }
                    "+"
                    kbd { "Enter" }
                    " Save"
                }
            }
            span { class: "hint",
                kbd { "Esc" }
                " Close"
            }
        }
    }
}

#[component]
pub fn AddLinkDialog(
    initial_url: String,
    on_close: EventHandler<()>,
    on_success: EventHandler<Link>,
    on_duplicate: EventHandler<Link>,
) -> Element {
    let mut url_input = use_signal(|| initial_url.clone());
    let mut error = use_signal(|| Option::<String>::None);
    let mut validation_error = use_signal(|| Option::<String>::None);
    let mut preview = use_signal(|| Option::<LinkPreview>::None);
    let mut progress_step = use_signal(|| Option::<ProgressStep>::None);
    let mut creating = use_signal(|| false);
    let mut show_success = use_signal(|| false);
    let mut can_retry_preview = use_signal(|| false);

    // Categorization state
    let mut selected_categories = use_signal(|| Vec::<Uuid>::new());
    let mut selected_tags = use_signal(|| Vec::<Uuid>::new());
    let mut selected_languages = use_signal(|| Vec::<Uuid>::new());
    let mut selected_licenses = use_signal(|| Vec::<Uuid>::new());

    // Track auto-suggested items
    let mut auto_suggested_languages = use_signal(|| false);
    let mut auto_suggested_license = use_signal(|| false);

    // Check if we're in a loading state
    let is_loading = move || progress_step().is_some();

    // Count of selected categorizations
    let selection_count = move || {
        selected_categories().len()
            + selected_tags().len()
            + selected_languages().len()
            + selected_licenses().len()
    };

    // Focus URL input on mount
    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Some(input) = document.get_element_by_id("add-link-url-input") {
                        let _ = input.dyn_ref::<web_sys::HtmlElement>().map(|e| e.focus());
                    }
                }
            }
        }
    });

    // Auto-suggest languages and licenses when preview loads
    use_effect(move || {
        let preview_data = preview();
        if let Some(data) = preview_data {
            // Auto-suggest languages from GitHub
            if !data.github_languages.is_empty() {
                let github_langs = data.github_languages.clone();
                spawn(async move {
                    if let Ok(languages) = fetch_languages().await {
                        let matched_ids: Vec<Uuid> = languages
                            .iter()
                            .filter(|lang| {
                                github_langs.iter().any(|gh_lang| {
                                    lang.name.to_lowercase() == gh_lang.to_lowercase()
                                })
                            })
                            .filter_map(|lang| Uuid::parse_str(&lang.id).ok())
                            .collect();

                        if !matched_ids.is_empty() {
                            selected_languages.set(matched_ids);
                            auto_suggested_languages.set(true);
                        }
                    }
                });
            }

            // Auto-suggest license from GitHub
            if let Some(gh_license) = data.github_license.clone() {
                spawn(async move {
                    if let Ok(licenses) = fetch_licenses().await {
                        if let Some(matched) = licenses.iter().find(|lic| {
                            lic.name.to_lowercase() == gh_license.to_lowercase()
                                || lic.acronym.as_ref().map(|a| a.to_lowercase()) == Some(gh_license.to_lowercase())
                        }) {
                            if let Ok(id) = Uuid::parse_str(&matched.id) {
                                selected_licenses.set(vec![id]);
                                auto_suggested_license.set(true);
                            }
                        }
                    }
                });
            }
        }
    });

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

    // Handle preview request
    let mut handle_preview = move |_| {
        if !validate_url() {
            return;
        }

        let url = url_input();

        spawn(async move {
            error.set(None);
            can_retry_preview.set(false);

            // Step 1: Validating URL
            progress_step.set(Some(ProgressStep::ValidatingUrl));
            // Small delay to show the step
            #[cfg(target_arch = "wasm32")]
            gloo_timers::future::TimeoutFuture::new(100).await;

            // Step 2: Checking for duplicates
            progress_step.set(Some(ProgressStep::CheckingDuplicates));
            match check_duplicate_url(&url).await {
                Ok(Some(existing_link)) => {
                    // Duplicate found - notify parent
                    progress_step.set(None);
                    on_duplicate.call(existing_link);
                },
                Ok(None) => {
                    // No duplicate - fetch preview
                    // Step 3: Fetching metadata
                    progress_step.set(Some(ProgressStep::FetchingMetadata));

                    match preview_link(&url).await {
                        Ok(preview_data) => {
                            preview.set(Some(preview_data));
                            progress_step.set(None);
                        },
                        Err(err) => {
                            error.set(Some(format_error(&err)));
                            can_retry_preview.set(true);
                            progress_step.set(None);
                        }
                    }
                },
                Err(err) => {
                    error.set(Some(format_error(&err)));
                    can_retry_preview.set(true);
                    progress_step.set(None);
                }
            }
        });
    };

    // Handle save (create link with categories)
    let handle_save = move |_| {
        if creating() {
            return;
        }

        let url = url_input();
        let categories = selected_categories();
        let tags = selected_tags();
        let languages = selected_languages();
        let licenses = selected_licenses();

        spawn(async move {
            creating.set(true);
            error.set(None);

            let request = CreateLinkWithCategoriesRequest {
                url,
                category_ids: categories,
                tag_ids: tags,
                language_ids: languages,
                license_ids: licenses,
            };

            match create_link_with_categories(&request).await {
                Ok(link) => {
                    creating.set(false);
                    show_success.set(true);
                    // Brief delay to show success animation
                    #[cfg(target_arch = "wasm32")]
                    gloo_timers::future::TimeoutFuture::new(500).await;
                    on_success.call(link);
                },
                Err(err) => {
                    error.set(Some(format_error(&err)));
                    creating.set(false);
                }
            }
        });
    };

    // Reset all state to initial values
    let mut reset_state = move || {
        url_input.set(String::new());
        error.set(None);
        validation_error.set(None);
        preview.set(None);
        progress_step.set(None);
        creating.set(false);
        show_success.set(false);
        can_retry_preview.set(false);
        selected_categories.set(Vec::new());
        selected_tags.set(Vec::new());
        selected_languages.set(Vec::new());
        selected_licenses.set(Vec::new());
        auto_suggested_languages.set(false);
        auto_suggested_license.set(false);
    };

    // Handle back to URL input
    let handle_back = move |_| {
        preview.set(None);
        error.set(None);
        can_retry_preview.set(false);
        // Reset categorization
        selected_categories.set(Vec::new());
        selected_tags.set(Vec::new());
        selected_languages.set(Vec::new());
        selected_licenses.set(Vec::new());
        // Reset auto-suggested flags
        auto_suggested_languages.set(false);
        auto_suggested_license.set(false);
    };

    // Handle keyboard shortcuts
    let handle_keydown = move |evt: KeyboardEvent| {
        match evt.key() {
            Key::Escape => {
                reset_state();
                on_close.call(());
            }
            Key::Enter if evt.modifiers().ctrl() || evt.modifiers().meta() => {
                // Ctrl/Cmd + Enter: Save link (when preview is shown)
                if preview().is_some() && !creating() {
                    handle_save(());
                }
            }
            _ => {}
        }
    };

    // Handle close button click
    let handle_close = move |_| {
        reset_state();
        on_close.call(());
    };

    // Format stars for display
    fn format_stars(stars: Option<i32>) -> String {
        match stars {
            Some(s) if s >= 1000 => format!("{:.1}k", s as f32 / 1000.0),
            Some(s) => s.to_string(),
            None => "-".to_string(),
        }
    }

    let has_preview = preview().is_some();

    rsx! {
        ModalBase {
            on_close: on_close,

            div {
                class: "add-link-dialog",
                tabindex: "0",
                role: "dialog",
                aria_modal: "true",
                aria_labelledby: "add-link-dialog-title",
                onkeydown: handle_keydown,

                div { class: "dialog-header",
                    h2 { id: "add-link-dialog-title", "Add Link" }
                    button {
                        class: "close-button",
                        onclick: handle_close,
                        aria_label: "Close dialog",
                        "×"
                    }
                }

                div { class: "dialog-body",
                    // Show success animation
                    if show_success() {
                        div {
                            class: "success-indicator",
                            role: "status",
                            aria_live: "polite",
                            div { class: "success-checkmark", aria_hidden: "true", "✓" }
                            span { "Link created successfully!" }
                        }
                    // Show progress indicator when loading
                    } else if let Some(step) = progress_step() {
                        ProgressIndicator { current_step: step }
                    } else if creating() {
                        CreatingIndicator {}
                    } else if let Some(preview_data) = preview() {
                        // Preview Section
                        div { class: "preview-section",
                            div { class: "preview-header",
                                if let Some(favicon) = preview_data.favicon.clone() {
                                    img {
                                        class: "preview-favicon",
                                        src: "{favicon}",
                                        alt: "Favicon"
                                    }
                                }
                                div { class: "preview-title-group",
                                    h3 { class: "preview-title",
                                        "{preview_data.title.clone().unwrap_or_else(|| preview_data.domain.clone())}"
                                    }
                                    span { class: "preview-domain", "{preview_data.domain}" }
                                }
                            }

                            if let Some(desc) = preview_data.description.clone() {
                                p { class: "preview-description", "{desc}" }
                            }

                            div { class: "preview-meta",
                                if preview_data.is_github_repo {
                                    span { class: "preview-badge preview-badge-github", "GitHub" }
                                    if let Some(stars) = preview_data.github_stars {
                                        span { class: "preview-badge preview-badge-stars",
                                            "★ {format_stars(Some(stars))}"
                                        }
                                    }
                                    if let Some(license) = preview_data.github_license.clone() {
                                        span { class: "preview-badge preview-badge-license", "{license}" }
                                    }
                                }
                                for lang in preview_data.github_languages.iter() {
                                    span { class: "preview-badge preview-badge-language", "{lang}" }
                                }
                            }

                            div { class: "preview-url",
                                a {
                                    href: "{preview_data.url}",
                                    target: "_blank",
                                    rel: "noopener noreferrer",
                                    "{preview_data.url}"
                                }
                            }
                        }

                        // Categorization Accordion
                        details { class: "categorization-accordion",
                            open: selection_count() > 0,
                            summary { class: "categorization-summary",
                                span { class: "categorization-arrow" }
                                "Add Categories (optional)"
                                if selection_count() > 0 {
                                    span { class: "categorization-count",
                                        " ({selection_count()} selected)"
                                    }
                                }
                            }

                            div { class: "categorization-content",
                                div { class: "form-group",
                                    label { "Categories" }
                                    CategorySelect {
                                        selected_ids: selected_categories(),
                                        on_change: move |ids| selected_categories.set(ids)
                                    }
                                }

                                div { class: "form-group",
                                    label { "Tags" }
                                    TagSelect {
                                        selected_ids: selected_tags(),
                                        on_change: move |ids| selected_tags.set(ids)
                                    }
                                }

                                div { class: "form-group",
                                    label {
                                        "Languages"
                                        if auto_suggested_languages() {
                                            span { class: "auto-suggested-badge", "Auto-detected" }
                                        }
                                    }
                                    LanguageSelect {
                                        selected_ids: selected_languages(),
                                        on_change: move |ids| {
                                            selected_languages.set(ids);
                                            // Clear auto-suggested flag when user modifies
                                            auto_suggested_languages.set(false);
                                        }
                                    }
                                }

                                div { class: "form-group",
                                    label {
                                        "Licenses"
                                        if auto_suggested_license() {
                                            span { class: "auto-suggested-badge", "Auto-detected" }
                                        }
                                    }
                                    LicenseSelect {
                                        selected_ids: selected_licenses(),
                                        on_change: move |ids| {
                                            selected_licenses.set(ids);
                                            // Clear auto-suggested flag when user modifies
                                            auto_suggested_license.set(false);
                                        }
                                    }
                                }
                            }
                        }

                        // Error display
                        if let Some(err) = error() {
                            div {
                                class: "error-box",
                                role: "alert",
                                aria_live: "assertive",
                                div { class: "error-content",
                                    span { class: "error-icon", aria_hidden: "true", "⚠" }
                                    span { class: "error-text", "{err}" }
                                }
                            }
                        }
                    } else {
                        // URL Input Section
                        div { class: "form-group",
                            label { r#for: "add-link-url-input", "URL" }
                            input {
                                id: "add-link-url-input",
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
                                        handle_preview(());
                                    }
                                }
                            }

                            // Validation error
                            if let Some(err) = validation_error() {
                                div { class: "error-message", "{err}" }
                            }
                        }

                        // Error display with retry option
                        if let Some(err) = error() {
                            div {
                                class: "error-box",
                                role: "alert",
                                aria_live: "assertive",
                                div { class: "error-content",
                                    span { class: "error-icon", aria_hidden: "true", "⚠" }
                                    span { class: "error-text", "{err}" }
                                }
                                if can_retry_preview() {
                                    button {
                                        class: "btn-retry",
                                        onclick: move |_| handle_preview(()),
                                        "Try Again"
                                    }
                                }
                            }
                        }

                        // Info about inaccessible links
                        div {
                            class: "info-box",
                            role: "note",
                            "If the URL is not accessible from this location (e.g., internal link), you'll see a warning but the link will still be saved."
                        }
                    }
                }

                div { class: "dialog-footer",
                    // Keyboard hints
                    if !is_loading() && !creating() {
                        KeyboardHints { show_save: has_preview }
                    }

                    div {
                        class: "dialog-actions",
                        role: "group",
                        aria_label: "Dialog actions",
                        if preview().is_some() && !creating() && !show_success() {
                            // Preview mode: Back and Save buttons
                            button {
                                class: "btn-secondary",
                                onclick: handle_back,
                                disabled: creating(),
                                "Back"
                            }
                            button {
                                class: "btn-primary",
                                onclick: move |_| handle_save(()),
                                disabled: creating(),
                                aria_busy: if creating() { "true" } else { "false" },
                                "Save Link"
                            }
                        } else if !is_loading() && !creating() && !show_success() {
                            // Input mode: Cancel and Preview buttons
                            button {
                                class: "btn-secondary",
                                onclick: handle_close,
                                "Cancel"
                            }
                            button {
                                class: "btn-primary",
                                onclick: move |_| handle_preview(()),
                                disabled: url_input().is_empty(),
                                aria_disabled: if url_input().is_empty() { "true" } else { "false" },
                                "Preview"
                            }
                        }
                    }
                }
            }
        }
    }
}
