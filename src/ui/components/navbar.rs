use crate::ui::http;
use dioxus::prelude::*;

#[component]
pub fn Navbar() -> Element {
    let mut loading = use_signal(|| false);
    let mut menu_open = use_signal(|| false);
    let mut show_maintenance = use_signal(|| false);
    let nav = navigator();

    // Fetch user info to check if admin + maintenance mode is active
    use_future(move || async move {
        if let Ok(info) = http::get::<crate::server_functions::auth::UserInfo>("/api/auth/me").await
        {
            show_maintenance.set(info.is_admin && info.maintenance_mode);
        }
    });

    let on_logout = move |_| {
        // In SaaS mode, navigate directly to /logout — the server middleware
        // redirects to the SaaS platform's logout endpoint to clear cookies.
        #[cfg(feature = "saas")]
        {
            #[cfg(target_arch = "wasm32")]
            if let Some(window) = web_sys::window() {
                let _ = window.location().set_href("/logout");
                return;
            }
        }

        spawn(async move {
            loading.set(true);

            // Use REST API for logout (invalidates refresh tokens server-side)
            let result = http::post_empty("/api/auth/logout").await;

            // Always clear client-side tokens, even if server call fails
            crate::ui::auth_state::clear_auth();

            loading.set(false);

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
        if show_maintenance() {
            MaintenanceBanner {}
        }
        nav {
            class: "bg-surface-50 border-b border-surface-300",
            role: "navigation",
            "aria-label": "Main navigation",
            div { class: "max-w-7xl mx-auto px-4 sm:px-6",
                div { class: "flex justify-between items-center py-4",
                    // Logo
                    a {
                        class: "text-xl sm:text-2xl font-bold text-primary-500 hover:text-accent-500 no-underline flex items-center gap-2",
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
                        class: "md:hidden p-2 rounded-md text-text-secondary hover:bg-surface-200 focus:outline-none focus:ring-2 focus:ring-primary-500",
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
                        class: "md:hidden py-4 border-t border-surface-300",
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
        "block w-full px-4 py-3 text-text-muted rounded-md font-medium hover:bg-surface-200 hover:text-text-primary transition-colors no-underline text-left text-sm"
    } else {
        "px-3 py-2 text-text-muted font-medium hover:text-text-primary transition-colors no-underline whitespace-nowrap text-sm"
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
        "block w-full px-4 py-3 bg-transparent border border-surface-300 text-text-muted rounded-md font-medium hover:bg-surface-200 hover:text-text-primary transition-colors disabled:opacity-50 disabled:cursor-not-allowed text-left text-sm"
    } else {
        "px-3 py-2 bg-transparent border border-surface-300 text-text-muted rounded-md font-medium hover:bg-surface-200 hover:text-text-primary transition-colors disabled:opacity-50 disabled:cursor-not-allowed whitespace-nowrap text-sm"
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
                span { class: "inline-block w-4 h-4 border-2 border-text-muted border-t-transparent rounded-full animate-spin mr-2" }
                "Logging out..."
            } else {
                "Logout"
            }
        }
    }
}

#[component]
fn MaintenanceBanner() -> Element {
    rsx! {
        div {
            class: "bg-amber-500 text-white text-center py-2 px-4 text-sm font-semibold",
            role: "status",
            "aria-live": "polite",
            div { class: "flex items-center justify-center gap-2",
                span { class: "w-2 h-2 bg-white rounded-full animate-pulse" }
                "Maintenance mode is active"
            }
        }
    }
}
