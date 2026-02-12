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
                        error.set(Some("Invalid credentials".to_string()));
                    }
                }
                Err(e) => {
                    error.set(Some(format!("Login failed: {}", e)));
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

                form { class: "form", onsubmit: on_submit,
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
                        disabled: loading(),
                        if loading() {
                            span { class: "loading" }
                            "Logging In..."
                        } else {
                            "Log In"
                        }
                    }
                }
            }
        }
    }
}
