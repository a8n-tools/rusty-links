use dioxus::prelude::*;

#[component]
pub fn Navbar() -> Element {
    let mut loading = use_signal(|| false);
    let nav = navigator();

    let on_logout = move |_| {
        spawn(async move {
            loading.set(true);

            let client = reqwest::Client::new();
            let response = client
                .post("/api/auth/logout")
                .send()
                .await;

            loading.set(false);

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        // Logout successful, redirect to login
                        nav.push("/login");
                    }
                }
                Err(_) => {
                    // Even if logout fails, redirect to login
                    nav.push("/login");
                }
            }
        });
    };

    rsx! {
        nav { class: "navbar",
            div { class: "navbar-content",
                a { class: "navbar-brand", href: "/links",
                    span { "\u{1F517}" } // Link emoji
                    " Rusty Links"
                }
                div { class: "navbar-menu",
                    a { class: "btn btn-secondary", href: "/links-table",
                        "Links"
                    }
                    a { class: "btn btn-secondary", href: "/categories",
                        "Categories"
                    }
                    button {
                        class: "btn btn-secondary",
                        disabled: loading(),
                        onclick: on_logout,
                        if loading() {
                            span { class: "loading" }
                            "Logging out..."
                        } else {
                            "Logout"
                        }
                    }
                }
            }
        }
    }
}
