use dioxus::prelude::*;
use dioxus_router::RouterConfig;
use uuid::Uuid;

use crate::server_functions::auth::check_setup;
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

#[component]
pub fn App() -> Element {
    rsx! {
        // The Stylesheet component inserts a style link into the head of the document
        Stylesheet {
            // Urls are relative to your Cargo.toml file
            href: asset!("/assets/tailwind.css")
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
    #[route("/setup")]
    SetupPage {},
    #[route("/login")]
    LoginPage {},
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
    #[route("/:..route")]
    NotFound { route: Vec<String> },
}

#[derive(Clone, Debug, PartialEq)]
enum HomeState {
    NeedsSetup,
    NeedsLogin,
    LoggedIn,
    Error(String),
}

async fn check_home_state() -> HomeState {
    // First check if setup is needed
    match check_setup().await {
        Ok(true) => return HomeState::NeedsSetup,
        Ok(false) => {}
        Err(e) => return HomeState::Error(e.to_string()),
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
    let home_state = use_resource(|| check_home_state());

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
