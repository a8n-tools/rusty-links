use crate::ui::http;
use dioxus::prelude::*;

#[component]
pub fn Navbar() -> Element {
    let mut loading = use_signal(|| false);
    let mut menu_open = use_signal(|| false);
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

    let toggle_menu = move |_| {
        menu_open.set(!menu_open());
    };

    let close_menu = move |_| {
        menu_open.set(false);
    };

    rsx! {
        nav {
            class: "bg-white border-b-2 border-gray-200 shadow-sm",
            role: "navigation",
            "aria-label": "Main navigation",
            div { class: "max-w-7xl mx-auto px-4 sm:px-6",
                div { class: "flex justify-between items-center py-4",
                    // Logo
                    a {
                        class: "text-xl sm:text-2xl font-bold text-orange-700 hover:text-orange-600 no-underline flex items-center gap-2",
                        href: "/links",
                        "aria-label": "Rusty Links home",
                        span { "aria-hidden": "true", "\u{1F517}" }
                        " Rusty Links"
                    }

                    // Desktop menu (hidden on mobile)
                    div {
                        class: "hidden md:flex gap-2 lg:gap-3 items-center",
                        role: "menubar",
                        "aria-label": "Site navigation",
                        NavLinks { on_click: close_menu }
                        LogoutButton { loading: loading(), on_logout: on_logout }
                    }

                    // Mobile hamburger button (hidden on desktop)
                    button {
                        class: "md:hidden p-2 rounded-md text-gray-700 hover:bg-gray-100 focus:outline-none focus:ring-2 focus:ring-orange-500",
                        onclick: toggle_menu,
                        "aria-expanded": if menu_open() { "true" } else { "false" },
                        "aria-controls": "mobile-menu",
                        "aria-label": if menu_open() { "Close menu" } else { "Open menu" },
                        if menu_open() {
                            // X icon
                            svg {
                                class: "w-6 h-6",
                                fill: "none",
                                stroke: "currentColor",
                                "stroke-width": "2",
                                "viewBox": "0 0 24 24",
                                path {
                                    "stroke-linecap": "round",
                                    "stroke-linejoin": "round",
                                    d: "M6 18L18 6M6 6l12 12"
                                }
                            }
                        } else {
                            // Hamburger icon
                            svg {
                                class: "w-6 h-6",
                                fill: "none",
                                stroke: "currentColor",
                                "stroke-width": "2",
                                "viewBox": "0 0 24 24",
                                path {
                                    "stroke-linecap": "round",
                                    "stroke-linejoin": "round",
                                    d: "M4 6h16M4 12h16M4 18h16"
                                }
                            }
                        }
                    }
                }

                // Mobile menu (shown when open)
                if menu_open() {
                    div {
                        id: "mobile-menu",
                        class: "md:hidden py-4 border-t border-gray-200",
                        role: "menu",
                        "aria-label": "Mobile navigation",
                        div { class: "flex flex-col gap-2",
                            NavLinks { on_click: close_menu, mobile: true }
                            LogoutButton { loading: loading(), on_logout: on_logout, mobile: true }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn NavLinks(on_click: EventHandler<MouseEvent>, #[props(default = false)] mobile: bool) -> Element {
    let base_class = if mobile {
        "block w-full px-4 py-3 bg-gray-100 text-gray-800 rounded-md font-medium hover:bg-gray-200 transition-colors no-underline text-left"
    } else {
        "px-4 py-2.5 bg-gray-200 text-gray-800 rounded-md font-medium hover:bg-gray-300 transition-colors no-underline whitespace-nowrap"
    };

    rsx! {
        a {
            class: base_class,
            href: "/links",
            role: "menuitem",
            "aria-label": "View links",
            onclick: move |e| on_click.call(e),
            "Links"
        }
        a {
            class: base_class,
            href: "/categories",
            role: "menuitem",
            "aria-label": "Manage categories",
            onclick: move |e| on_click.call(e),
            "Categories"
        }
        a {
            class: base_class,
            href: "/languages",
            role: "menuitem",
            "aria-label": "Manage programming languages",
            onclick: move |e| on_click.call(e),
            "Languages"
        }
        a {
            class: base_class,
            href: "/licenses",
            role: "menuitem",
            "aria-label": "Manage licenses",
            onclick: move |e| on_click.call(e),
            "Licenses"
        }
        a {
            class: base_class,
            href: "/tags",
            role: "menuitem",
            "aria-label": "Manage tags",
            onclick: move |e| on_click.call(e),
            "Tags"
        }
    }
}

#[component]
fn LogoutButton(
    loading: bool,
    on_logout: EventHandler<MouseEvent>,
    #[props(default = false)] mobile: bool,
) -> Element {
    let base_class = if mobile {
        "block w-full px-4 py-3 bg-gray-100 text-gray-800 rounded-md font-medium hover:bg-gray-200 transition-colors disabled:opacity-50 disabled:cursor-not-allowed text-left"
    } else {
        "px-4 py-2.5 bg-gray-200 text-gray-800 rounded-md font-medium hover:bg-gray-300 transition-colors disabled:opacity-50 disabled:cursor-not-allowed whitespace-nowrap"
    };

    rsx! {
        button {
            class: base_class,
            role: "menuitem",
            disabled: loading,
            onclick: move |e| on_logout.call(e),
            "aria-label": if loading { "Logging out, please wait" } else { "Logout from application" },
            "aria-busy": if loading { "true" } else { "false" },
            if loading {
                span { class: "inline-block w-4 h-4 border-2 border-gray-600 border-t-transparent rounded-full animate-spin mr-2" }
                "Logging out..."
            } else {
                "Logout"
            }
        }
    }
}
