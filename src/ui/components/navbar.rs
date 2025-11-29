use dioxus::prelude::*;
use crate::ui::http;

#[component]
pub fn Navbar() -> Element {
    let mut loading = use_signal(|| false);
    let nav = navigator();

    let on_logout = move |_| {
        spawn(async move {
            loading.set(true);

            // Use REST API for logout (properly clears cookies)
            let result = http::post_empty("/api/auth/logout").await;

            loading.set(false);

            // Always redirect to login, even if logout fails
            if let Err(e) = result {
                tracing::warn!("Logout failed: {:?}", e);
            }
            nav.push("/login");
        });
    };

    rsx! {
        nav {
            class: "bg-white border-b-2 border-gray-200 px-6 py-4 shadow-sm",
            role: "navigation",
            "aria-label": "Main navigation",
            div { class: "max-w-7xl mx-auto flex justify-between items-center",
                a {
                    class: "text-2xl font-bold text-orange-700 hover:text-orange-600 no-underline flex items-center gap-2",
                    href: "/links",
                    "aria-label": "Rusty Links home",
                    span { "aria-hidden": "true", "\u{1F517}" } // Link emoji
                    " Rusty Links"
                }
                div {
                    class: "flex gap-3 items-center",
                    role: "menubar",
                    "aria-label": "Site navigation",
                    a {
                        class: "px-5 py-3 bg-gray-200 text-gray-800 rounded-md font-medium hover:bg-gray-300 transition-colors no-underline",
                        href: "/links-table",
                        role: "menuitem",
                        "aria-label": "View links",
                        "Links"
                    }
                    a {
                        class: "px-5 py-3 bg-gray-200 text-gray-800 rounded-md font-medium hover:bg-gray-300 transition-colors no-underline",
                        href: "/categories",
                        role: "menuitem",
                        "aria-label": "Manage categories",
                        "Categories"
                    }
                    a {
                        class: "px-5 py-3 bg-gray-200 text-gray-800 rounded-md font-medium hover:bg-gray-300 transition-colors no-underline",
                        href: "/languages",
                        role: "menuitem",
                        "aria-label": "Manage programming languages",
                        "Languages"
                    }
                    a {
                        class: "px-5 py-3 bg-gray-200 text-gray-800 rounded-md font-medium hover:bg-gray-300 transition-colors no-underline",
                        href: "/licenses",
                        role: "menuitem",
                        "aria-label": "Manage licenses",
                        "Licenses"
                    }
                    a {
                        class: "px-5 py-3 bg-gray-200 text-gray-800 rounded-md font-medium hover:bg-gray-300 transition-colors no-underline",
                        href: "/tags",
                        role: "menuitem",
                        "aria-label": "Manage tags",
                        "Tags"
                    }
                    button {
                        class: "px-5 py-3 bg-gray-200 text-gray-800 rounded-md font-medium hover:bg-gray-300 transition-colors disabled:opacity-50 disabled:cursor-not-allowed",
                        role: "menuitem",
                        disabled: loading(),
                        onclick: on_logout,
                        "aria-label": if loading() { "Logging out, please wait" } else { "Logout from application" },
                        "aria-busy": if loading() { "true" } else { "false" },
                        if loading() {
                            span { class: "inline-block w-4 h-4 border-2 border-gray-600 border-t-transparent rounded-full animate-spin mr-2" }
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
