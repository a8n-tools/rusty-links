use dioxus::prelude::*;
use crate::ui::http;

#[component]
pub fn Navbar() -> Element {
    let mut loading = use_signal(|| false);
    let nav = navigator();

    let on_logout = move |_| {
        spawn(async move {
            loading.set(true);

            let response = http::post_empty("/api/auth/logout").await;

            loading.set(false);

            match response {
                Ok(resp) => {
                    if resp.is_success() {
                        // Logout successful, redirect to login
                        nav.push("/login");
                    } else {
                        // Even if logout fails, redirect to login
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
        nav {
            class: "navbar",
            role: "navigation",
            "aria-label": "Main navigation",
            div { class: "navbar-content",
                a {
                    class: "navbar-brand",
                    href: "/links",
                    "aria-label": "Rusty Links home",
                    span { "aria-hidden": "true", "\u{1F517}" } // Link emoji
                    " Rusty Links"
                }
                div {
                    class: "navbar-menu",
                    role: "menubar",
                    "aria-label": "Site navigation",
                    a {
                        class: "btn btn-secondary",
                        href: "/links-table",
                        role: "menuitem",
                        "aria-label": "View links",
                        "Links"
                    }
                    a {
                        class: "btn btn-secondary",
                        href: "/categories",
                        role: "menuitem",
                        "aria-label": "Manage categories",
                        "Categories"
                    }
                    a {
                        class: "btn btn-secondary",
                        href: "/languages",
                        role: "menuitem",
                        "aria-label": "Manage programming languages",
                        "Languages"
                    }
                    a {
                        class: "btn btn-secondary",
                        href: "/licenses",
                        role: "menuitem",
                        "aria-label": "Manage licenses",
                        "Licenses"
                    }
                    a {
                        class: "btn btn-secondary",
                        href: "/tags",
                        role: "menuitem",
                        "aria-label": "Manage tags",
                        "Tags"
                    }
                    button {
                        class: "btn btn-secondary",
                        role: "menuitem",
                        disabled: loading(),
                        onclick: on_logout,
                        "aria-label": if loading() { "Logging out, please wait" } else { "Logout from application" },
                        "aria-busy": if loading() { "true" } else { "false" },
                        if loading() {
                            span { class: "loading", "aria-hidden": "true" }
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
