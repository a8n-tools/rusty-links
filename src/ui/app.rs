use dioxus::prelude::*;
use dioxus_router::RouterConfig;
use uuid::Uuid;

#[cfg(feature = "standalone")]
use crate::server_functions::auth::check_setup;
use crate::ui::http;
use crate::ui::pages::add_link::AddLinkPage;
use crate::ui::pages::categories::CategoriesPage;
use crate::ui::pages::edit_link::EditLinkPage;
use crate::ui::pages::languages::LanguagesPage;
use crate::ui::pages::licenses::LicensesPage;
use crate::ui::pages::links_list::LinksListPage;
use crate::ui::pages::login::Login;
#[cfg(feature = "standalone")]
use crate::ui::pages::setup::Setup;
use crate::ui::pages::tags::TagsPage;

#[component]
pub fn App() -> Element {
    rsx! {
        // The Stylesheet component inserts a style link into the head of the document
        Stylesheet {
            // Urls are relative to your Cargo.toml file
            href: "/tailwind.css"
        }
        document::Link {
            rel: "icon",
            href: "/assets/favicon.ico",
        }
        document::Script {
            src: "/high-contrast-init.js",
        }
        if cfg!(feature = "saas") {
            document::Script {
                src: "/saas-refresh.js",
            }
        }
        Router::<Route> {
            config: || RouterConfig::default().on_update(|_| None)
        }
    }
}

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[route("/")]
    Home {},
    #[cfg(feature = "standalone")]
    #[route("/setup")]
    SetupPage {},
    #[route("/login")]
    LoginPage {},
    #[layout(ProtectedLayout)]
        #[route("/links")]
        LinksPage {},
        #[route("/links/add?:initial_url")]
        AddLink { initial_url: Option<String> },
        #[route("/links/:link_id/edit")]
        EditLink { link_id: Uuid },
        #[route("/categories")]
        Categories {},
        #[route("/tags")]
        Tags {},
        #[route("/languages")]
        Languages {},
        #[route("/licenses")]
        Licenses {},
    #[end_layout]
    #[route("/:..route")]
    NotFound { route: Vec<String> },
}

/// Result of verifying a stored JWT against the server.
///
/// Distinguishes "token is bad" from "couldn't verify" so transient backend
/// failures (DB not ready, proxy 502, network blip) don't masquerade as
/// authentication failures and boot the user back to /login.
#[cfg(feature = "standalone")]
#[derive(Clone, Copy, PartialEq)]
enum AuthVerdict {
    /// Server confirmed the token is valid (2xx).
    Valid,
    /// Server explicitly rejected the token (401/403). Clear it and redirect.
    Invalid,
    /// Couldn't verify: network error, 5xx, or any other status. Retain the
    /// token and let individual pages surface their own fetch errors.
    Unknown,
}

#[cfg(feature = "standalone")]
async fn verify_auth() -> AuthVerdict {
    match http::get_response("/api/auth/me").await {
        Ok(resp) if resp.is_success() => AuthVerdict::Valid,
        Ok(resp) if resp.status == 401 || resp.status == 403 => AuthVerdict::Invalid,
        _ => AuthVerdict::Unknown,
    }
}

#[component]
fn ProtectedLayout() -> Element {
    // The auth guard must only run on the WASM target where localStorage exists.
    //
    // On the server (SSR), is_authenticated() always returns false (non-WASM stub),
    // so the original #[cfg(feature = "standalone")] gate caused the server to emit
    // a redirect to /login on every protected-route render. The browser would briefly
    // show the login page before WASM hydrated and corrected the auth state.
    //
    // Returning a *different* element on the server (e.g. a spinner) instead also
    // fails: Dioxus hydrates by reconciling the server-rendered DOM with the WASM
    // virtual DOM. A spinner vs full Outlet content is a structural mismatch that
    // causes "wasm-bindgen: imported JS function ... threw an error" during DOM
    // reconciliation.
    //
    // Correct fix: gate the guard to wasm32 only. The server always renders Outlet
    // (consistent with what WASM will render for an authenticated user), so hydration
    // is a no-op reconciliation. The guard then runs synchronously on the client
    // where localStorage is available, redirecting unauthenticated users without
    // any DOM mismatch.
    #[cfg(all(feature = "standalone", target_arch = "wasm32"))]
    if !crate::ui::auth_state::is_authenticated() {
        spawn(async move {
            let path = web_sys::window()
                .and_then(|w| w.location().pathname().ok())
                .unwrap_or_default();
            let _ = crate::server_functions::auth::log_unauthenticated_access(path).await;
            navigator().push(Route::LoginPage {});
        });
        return rsx! {};
    }

    // Defense-in-depth: verify the token with the server on mount. A stale
    // token (e.g. left over from a previous session with a different
    // JWT_SECRET) will pass the localStorage presence check above but fail
    // here, so we clear it and send the user to /login.
    //
    // IMPORTANT: only redirect on `Invalid` (401/403). Transient backend
    // failures (DB warming, proxy 502, network blip) return `Unknown` — we
    // keep the token and render Outlet so the child page can show its own
    // error instead of silently bouncing the user to /login.
    //
    // `use_resource` must be called on both SSR and WASM to keep hook counts
    // consistent for hydration. On SSR the resource is never consulted (the
    // check below is wasm32-only) so the server still renders Outlet. The
    // `_auth_check` underscore prefix silences the unused-variable warning
    // on the non-wasm32 SSR build.
    #[cfg(feature = "standalone")]
    let _auth_check = use_resource(verify_auth);

    #[cfg(all(feature = "standalone", target_arch = "wasm32"))]
    {
        // Read once into an owned Copy value so the signal guard is dropped
        // before the early return.
        let verdict: Option<AuthVerdict> = *_auth_check.read();
        if verdict == Some(AuthVerdict::Invalid) {
            crate::ui::auth_state::clear_auth();
            navigator().push(Route::LoginPage {});
            return rsx! {};
        }
    }

    rsx! { Outlet::<Route> {} }
}

#[derive(Clone, Debug, PartialEq)]
enum HomeState {
    #[cfg(feature = "standalone")]
    NeedsSetup,
    NeedsLogin,
    LoggedIn,
    Error(String),
}

async fn check_home_state() -> HomeState {
    // First check if setup is needed (standalone only — saas auth is handled externally)
    #[cfg(feature = "standalone")]
    {
        match check_setup().await {
            Ok(true) => return HomeState::NeedsSetup,
            Ok(false) => {}
            Err(e) => return HomeState::Error(e.to_string()),
        }
    }

    // In standalone mode, skip the network call if no token is stored
    #[cfg(feature = "standalone")]
    {
        if !crate::ui::auth_state::is_authenticated() {
            return HomeState::NeedsLogin;
        }
    }

    // Check if user is logged in by calling /api/auth/me
    match http::get_response("/api/auth/me").await {
        Ok(response) if response.is_success() => HomeState::LoggedIn,
        _ => HomeState::NeedsLogin,
    }
}

#[component]
fn Home() -> Element {
    let nav = navigator();
    let home_state = use_resource(check_home_state);

    let result = home_state.read().clone();

    match result {
        #[cfg(feature = "standalone")]
        Some(HomeState::NeedsSetup) => {
            nav.push(Route::SetupPage {});
            rsx! {
                div { class: "auth-container",
                    div { class: "loading-container",
                        div { class: "spinner spinner-medium" }
                        p { class: "loading-message", "Redirecting to setup..." }
                    }
                }
            }
        }
        Some(HomeState::LoggedIn) => {
            nav.push(Route::LinksPage {});
            rsx! {
                div { class: "auth-container",
                    div { class: "loading-container",
                        div { class: "spinner spinner-medium" }
                        p { class: "loading-message", "Redirecting to links..." }
                    }
                }
            }
        }
        Some(HomeState::NeedsLogin) => {
            nav.push(Route::LoginPage {});
            rsx! {
                div { class: "auth-container",
                    div { class: "loading-container",
                        div { class: "spinner spinner-medium" }
                        p { class: "loading-message", "Redirecting to login..." }
                    }
                }
            }
        }
        Some(HomeState::Error(e)) => {
            rsx! {
                div { class: "auth-container",
                    div { class: "auth-card",
                        h1 { class: "auth-title", "Error" }
                        p { class: "message message-error", "Failed to check status: {e}" }
                    }
                }
            }
        }
        None => {
            rsx! {
                div { class: "auth-container",
                    div { class: "loading-container",
                        div { class: "spinner spinner-medium" }
                        p { class: "loading-message", "Loading..." }
                    }
                }
            }
        }
    }
}

#[cfg(feature = "standalone")]
#[component]
fn SetupPage() -> Element {
    rsx! { Setup {} }
}

#[component]
fn LoginPage() -> Element {
    rsx! { Login {} }
}

#[component]
fn LinksPage() -> Element {
    rsx! { LinksListPage {} }
}

#[component]
fn AddLink(initial_url: Option<String>) -> Element {
    rsx! { AddLinkPage { initial_url: initial_url } }
}

#[component]
fn EditLink(link_id: Uuid) -> Element {
    rsx! { EditLinkPage { link_id: link_id } }
}

#[component]
fn Categories() -> Element {
    rsx! { CategoriesPage {} }
}

#[component]
fn Tags() -> Element {
    rsx! { TagsPage {} }
}

#[component]
fn Languages() -> Element {
    rsx! { LanguagesPage {} }
}

#[component]
fn Licenses() -> Element {
    rsx! { LicensesPage {} }
}

#[component]
fn NotFound(route: Vec<String>) -> Element {
    let path = format!("/{}", route.join("/"));

    rsx! {
        div { class: "not-found-container",
            div { class: "not-found-content",
                div { class: "not-found-icon", "404" }
                h1 { class: "not-found-title", "Page Not Found" }
                p { class: "not-found-message",
                    "The page "
                    code { class: "not-found-path", "{path}" }
                    " doesn't exist."
                }
                p { class: "not-found-suggestion",
                    "It might have been moved, deleted, or perhaps the URL was mistyped."
                }
                div { class: "not-found-actions",
                    a {
                        class: "btn-primary",
                        href: "/links",
                        "Go to Links"
                    }
                    a {
                        class: "btn-secondary",
                        href: "/",
                        "Go Home"
                    }
                }
            }
        }
    }
}
