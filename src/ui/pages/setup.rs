use crate::server_functions::auth::SetupRequest;
use crate::ui::app::Route;
use crate::ui::http;
use dioxus::prelude::*;

#[component]
pub fn Setup() -> Element {
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut name = use_signal(String::new);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    let nav = navigator();

    let on_submit = move |evt: FormEvent| {
        evt.prevent_default();

        let email_val = email();
        let password_val = password();
        let name_val = name();

        // Basic validation
        if email_val.is_empty() || password_val.is_empty() || name_val.is_empty() {
            error.set(Some("Please fill in all fields".to_string()));
            return;
        }

        if !email_val.contains('@') {
            error.set(Some("Please enter a valid email address".to_string()));
            return;
        }

        if password_val.len() < 8 {
            error.set(Some("Password must be at least 8 characters".to_string()));
            return;
        }

        spawn(async move {
            loading.set(true);
            error.set(None);

            let request = SetupRequest {
                email: email_val.clone(),
                password: password_val.clone(),
                name: name_val.clone(),
            };

            let response = http::post_response("/api/auth/setup", &request).await;

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
                        // Setup successful, redirect to links page
                        nav.push(Route::LinksPage {});
                    } else {
                        error.set(Some(format!("Setup failed: {}", resp.body)));
                    }
                }
                Err(e) => {
                    error.set(Some(format!("Setup failed: {}", e)));
                }
            }

            loading.set(false);
        });
    };

    rsx! {
        div { class: "auth-container",
            div { class: "auth-card",
                h1 { class: "auth-title", "Rusty Links - Setup" }
                p { class: "auth-subtitle", "Create your administrator account" }

                if let Some(err) = error() {
                    div { class: "message message-error", "{err}" }
                }

                form { class: "form", onsubmit: on_submit,
                    div { class: "form-group",
                        label { class: "form-label", r#for: "name", "Name" }
                        input {
                            class: "form-input",
                            r#type: "text",
                            id: "name",
                            placeholder: "Your name",
                            value: "{name}",
                            disabled: loading(),
                            oninput: move |evt| name.set(evt.value()),
                        }
                    }

                    div { class: "form-group",
                        label { class: "form-label", r#for: "email", "Email Address" }
                        input {
                            class: "form-input",
                            r#type: "email",
                            id: "email",
                            placeholder: "admin@example.com",
                            value: "{email}",
                            disabled: loading(),
                            oninput: move |evt| email.set(evt.value()),
                        }
                        span { class: "form-hint", "This will be your login email" }
                    }

                    div { class: "form-group",
                        label { class: "form-label", r#for: "password", "Password" }
                        input {
                            class: "form-input",
                            r#type: "password",
                            id: "password",
                            placeholder: "Enter a secure password (min. 8 characters)",
                            value: "{password}",
                            disabled: loading(),
                            oninput: move |evt| password.set(evt.value()),
                        }
                    }

                    button {
                        class: "btn btn-primary btn-full",
                        r#type: "submit",
                        disabled: loading(),
                        if loading() {
                            span { class: "loading" }
                            "Creating Account..."
                        } else {
                            "Create Account"
                        }
                    }
                }
            }
        }
    }
}
