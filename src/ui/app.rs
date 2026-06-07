use dioxus::prelude::*;
use dioxus_router::RouterConfig;
use uuid::Uuid;

use crate::server_functions::auth::check_setup;
use crate::ui::components::footer::Footer;
use crate::ui::http;
use crate::ui::pages::add_link::AddLinkPage;
use crate::ui::pages::categories::CategoriesPage;
use crate::ui::pages::edit_link::EditLinkPage;
use crate::ui::pages::languages::LanguagesPage;
use crate::ui::pages::licenses::LicensesPage;
use crate::ui::pages::links_list::LinksListPage;
use crate::ui::pages::login::Login;
use crate::ui::pages::setup::Setup;
use crate::ui::pages::tags::TagsPage;

/// Deployment mode resolved at runtime from `/api/health`.
///
/// A single binary serves both modes; the WASM client fetches the mode once
/// and provides it via context so login/setup/logout render correctly.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum AuthMode {
    /// Local JWT auth (email/password). `OIDC_ISSUER` unset on the server.
    Standalone,
    /// OIDC login against a8n Tools. `OIDC_ISSUER` set on the server.
    Hosted,
}

/// Shape of `/api/health` that the client cares about.
#[derive(serde::Deserialize)]
struct ModeProbe {
    auth_mode: String,
}

/// Fetch the deployment mode from the public `/api/health` endpoint.
///
/// Returns `None` while unresolved (e.g. on SSR, where the relative URL has no
/// host) so callers render a spinner until the client resolves it.
pub async fn fetch_auth_mode() -> Option<AuthMode> {
    match http::get::<ModeProbe>("/api/health").await {
        Ok(probe) if probe.auth_mode == "hosted" => Some(AuthMode::Hosted),
        Ok(_) => Some(AuthMode::Standalone),
        Err(_) => None,
    }
}

/// Context type holding the auth-mode resource. `read().flatten()` yields
/// `None` while pending and `Some(mode)` once resolved.
pub type AuthModeResource = Resource<Option<AuthMode>>;

#[component]
pub fn App() -> Element {
    // Resolve the deployment mode once and share it with all routes via
    // context. The resource is `None` until it resolves on the client; child
    // components render a spinner while pending (hydration-safe: SSR and the
    // initial WASM render both see `None`).
    let auth_mode = use_resource(fetch_auth_mode);
    use_context_provider(|| auth_mode);

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
        // Always included; saas-refresh.js fetches /api/health and no-ops in
        // standalone mode, so it is safe in both deployment modes.
        document::Script {
            src: "/saas-refresh.js",
        }
        Router::<Route> {
            config: || RouterConfig::default().on_update(|_| None)
        }
        Footer {}
    }
}

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[route("/")]
    Home {},
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

async fn verify_auth() -> AuthVerdict {
    match http::get_response("/api/auth/me").await {
        Ok(resp) if resp.is_success() => AuthVerdict::Valid,
        Ok(resp) if resp.status == 401 || resp.status == 403 => AuthVerdict::Invalid,
        _ => AuthVerdict::Unknown,
    }
}

#[component]
fn ProtectedLayout() -> Element {
    // Auth mode from context (`None` while the /api/health probe is pending).
    // Unused on SSR (the guards below are wasm32-only); the underscore silences
    // the unused-variable warning there.
    let _mode_res = use_context::<AuthModeResource>();

    // Defense-in-depth: verify the session with the server on mount. Works in
    // both modes — /api/auth/me succeeds for a valid JWT (standalone) or
    // session cookie / bearer token (hosted), and returns 401/403 otherwise. A
    // stale token (e.g. left over from a previous session with a different
    // JWT_SECRET) fails here so we clear it and send the user to /login.
    //
    // IMPORTANT: only redirect on `Invalid` (401/403). Transient backend
    // failures (DB warming, proxy 502, network blip) return `Unknown` — we
    // keep rendering Outlet so the child page can show its own error instead
    // of silently bouncing the user to /login.
    //
    // `use_resource` runs on both SSR and WASM to keep hook counts consistent
    // for hydration. On SSR the verdict is never consulted (the checks below
    // are wasm32-only) so the server always renders Outlet — a no-op
    // reconciliation against the authenticated WASM render.
    let _auth_check = use_resource(verify_auth);

    // Hosted mode: poll /api/auth/me every 60 seconds so an idle tab is
    // redirected to /login promptly after the user signs out of the IdP. The
    // back-channel logout increments session_version server-side; the next
    // poll returns 401 and we redirect without waiting for a user action.
    // No-op in standalone mode. Gated to wasm32 (browser timer only).
    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        if _mode_res().flatten() != Some(AuthMode::Hosted) {
            return;
        }
        spawn(async move {
            loop {
                gloo_timers::future::sleep(std::time::Duration::from_secs(60)).await;

                if let Ok(None) = http::get_current_user().await {
                    navigator().replace(Route::LoginPage {});
                    break;
                }
            }
        });
    });

    // Client-side guards. SSR (non-wasm32) always renders Outlet, so hydration
    // is a no-op reconciliation; the guards run on the client after mount.
    #[cfg(target_arch = "wasm32")]
    {
        let mode = _mode_res().flatten();

        // Standalone: redirect immediately when no token is stored, before the
        // server round-trip resolves. Hosted sessions are cookie-based and
        // carry no localStorage token, so this check is standalone-only.
        if mode == Some(AuthMode::Standalone) && !crate::ui::auth_state::is_authenticated() {
            spawn(async move {
                let path = web_sys::window()
                    .and_then(|w| w.location().pathname().ok())
                    .unwrap_or_default();
                let _ = crate::server_functions::auth::log_unauthenticated_access(path).await;
                navigator().push(Route::LoginPage {});
            });
            return rsx! {};
        }

        // Both modes: the server explicitly rejected the session (401/403).
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
    NeedsSetup,
    NeedsLogin,
    LoggedIn,
    Error(String),
}

async fn check_home_state() -> HomeState {
    let hosted = matches!(fetch_auth_mode().await, Some(AuthMode::Hosted));

    // Standalone only: check whether first-run setup is needed and short-circuit
    // to login when no token is stored. In hosted mode auth is handled by the
    // OIDC provider, so neither check applies.
    if !hosted {
        match check_setup().await {
            Ok(true) => return HomeState::NeedsSetup,
            Ok(false) => {}
            Err(e) => return HomeState::Error(e.to_string()),
        }

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
