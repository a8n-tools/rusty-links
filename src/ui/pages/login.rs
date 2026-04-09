use crate::server_functions::auth::LoginRequest;
use crate::ui::app::Route;
use crate::ui::http;
use dioxus::prelude::*;

#[component]
pub fn Login() -> Element {
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    // `hydrated` stays false during SSR and on the initial WASM render (so
    // hydration reconciles identically), then flips to true in `use_effect`,
    // which only runs on the client after mount. We use this to keep the
    // submit button disabled until the onsubmit handler is actually attached,
    // preventing a native HTML form submission (GET to current URL) if the
    // user clicks "Log In" before WASM has hydrated.
    let mut hydrated = use_signal(|| false);
    use_effect(move || {
        hydrated.set(true);
    });
    let nav = navigator();

    let on_submit = move |evt: FormEvent| {
        evt.prevent_default();

        let email_val = email();
        let password_val = password();

        // Basic validation
        if email_val.is_empty() || password_val.is_empty() {
            error.set(Some("Please fill in all fields".to_string()));
            return;
        }

        spawn(async move {
            loading.set(true);
            error.set(None);

            let request = LoginRequest {
                email: email_val.clone(),
                password: password_val.clone(),
            };

            let response = http::post_response("/api/auth/login", &request).await;

            match response {
                Ok(resp) => {
                    if resp.is_success() {
                        // Parse auth response and store tokens
                        #[cfg(feature = "standalone")]
                        {
                            if let Ok(auth_resp) =
                                resp.json::<crate::server_functions::auth::AuthResponse>()
                            {
                                crate::ui::auth_state::save_auth(
                                    &auth_resp.token,
                                    &auth_resp.refresh_token,
                                    &auth_resp.email,
                                );
                            }
                        }
                        // Login successful, redirect to links page
                        nav.push(Route::LinksPage {});
                    } else {
                        error.set(Some(resp.error_message()));
                    }
                }
                Err(e) => {
                    // Log the raw error for debugging; show the user a
                    // generic retry hint (server starting up, DB not yet
                    // loaded, flaky network, etc.).
                    tracing::warn!("Login request failed: {}", e);
                    error.set(Some(
                        "Can't reach the server right now. Please try again in a moment."
                            .to_string(),
                    ));
                }
            }

            loading.set(false);
        });
    };

    rsx! {
        div { class: "auth-container",
            div { class: "auth-card",
                h1 { class: "auth-title", "Rusty Links - Login" }
                p { class: "auth-subtitle", "Sign in to your account" }

                if let Some(err) = error() {
                    div { class: "message message-error", "{err}" }
                }

                form {
                    class: "form",
                    // Defense against pre-hydration clicks: if the browser
                    // falls back to native form submission (because WASM
                    // hasn't attached onsubmit yet), `javascript:void(0)`
                    // makes the submission a no-op instead of a GET to the
                    // current URL that would wipe the user's typed input.
                    action: "javascript:void(0)",
                    onsubmit: on_submit,
                    div { class: "form-group",
                        label { class: "form-label", r#for: "email", "Email Address" }
                        input {
                            class: "form-input",
                            r#type: "email",
                            id: "email",
                            placeholder: "your@email.com",
                            value: "{email}",
                            disabled: loading(),
                            oninput: move |evt| email.set(evt.value()),
                        }
                    }

                    div { class: "form-group",
                        label { class: "form-label", r#for: "password", "Password" }
                        input {
                            class: "form-input",
                            r#type: "password",
                            id: "password",
                            placeholder: "Enter your password",
                            value: "{password}",
                            disabled: loading(),
                            oninput: move |evt| password.set(evt.value()),
                        }
                    }

                    button {
                        class: "btn btn-primary btn-full",
                        r#type: "submit",
                        disabled: loading() || !hydrated(),
                        if loading() {
                            span { class: "loading" }
                            "Logging In..."
                        } else if !hydrated() {
                            span { class: "loading" }
                            "Initializing..."
                        } else {
                            "Log In"
                        }
                    }
                }
            }
        }
    }
}
