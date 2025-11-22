use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct SetupRequest {
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct User {
    id: String,
    email: String,
}

#[component]
pub fn Setup() -> Element {
    let mut email = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    let nav = navigator();

    let on_submit = move |evt: FormEvent| {

        let email_val = email();
        let password_val = password();

        // Basic validation
        if email_val.is_empty() || password_val.is_empty() {
            error.set(Some("Please fill in all fields".to_string()));
            return;
        }

        if !email_val.contains('@') {
            error.set(Some("Please enter a valid email address".to_string()));
            return;
        }

        spawn(async move {
            loading.set(true);
            error.set(None);

            let request = SetupRequest {
                email: email_val.clone(),
                password: password_val.clone(),
            };

            let client = reqwest::Client::new();
            let response = client
                .post("/api/auth/setup")
                .json(&request)
                .send()
                .await;

            loading.set(false);

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        // Setup successful, redirect to login
                        nav.push("/login");
                    } else {
                        let error_text = resp.text().await.unwrap_or_else(|_| "Setup failed".to_string());
                        error.set(Some(format!("Setup failed: {}", error_text)));
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
                h1 { class: "auth-title", "Rusty Links - Setup" }
                p { class: "auth-subtitle", "Create your administrator account" }

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
                            placeholder: "Enter a secure password",
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
