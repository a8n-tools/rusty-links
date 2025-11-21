use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct User {
    id: String,
    email: String,
}

#[component]
pub fn Login() -> Element {
    let mut email = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());
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

            let client = reqwest::Client::new();
            let response = client
                .post("/api/auth/login")
                .json(&request)
                .send()
                .await;

            loading.set(false);

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        // Login successful, redirect to links page
                        nav.push("/links");
                    } else {
                        // Show generic error to prevent email enumeration
                        error.set(Some("Invalid credentials".to_string()));
                    }
                }
                Err(e) => {
                    error.set(Some(format!("Network error: {}", e)));
                }
            }
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
