use dioxus::prelude::*;
use dioxus_router::RouterConfig;

use crate::server_functions::auth::check_setup;
use crate::ui::pages::setup::Setup;
use crate::ui::pages::login::Login;
use crate::ui::pages::links::Links;
use crate::ui::pages::links_list::LinksListPage;
use crate::ui::pages::categories::CategoriesPage;
use crate::ui::pages::tags::TagsPage;
use crate::ui::pages::languages::LanguagesPage;
use crate::ui::pages::licenses::LicensesPage;

// Include the CSS at compile time
const STYLES: &str = include_str!("../../assets/style.css");

#[component]
pub fn App() -> Element {
    rsx! {
        style { {STYLES} }
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
    #[route("/links/list")]
    LinksList {},
    #[route("/categories")]
    Categories {},
    #[route("/tags")]
    Tags {},
    #[route("/languages")]
    Languages {},
    #[route("/licenses")]
    Licenses {},
}

#[component]
fn Home() -> Element {
    let nav = navigator();
    let setup_check = use_resource(|| async { check_setup().await });

    // Clone the result to avoid borrow issues
    let result = setup_check.read().clone();

    match result {
        Some(Ok(needs_setup)) => {
            if needs_setup {
                // No users exist, redirect to setup
                nav.push(Route::SetupPage {});
            } else {
                // Users exist, redirect to login
                nav.push(Route::LoginPage {});
            }
            rsx! {
                div { class: "auth-container",
                    div { class: "loading-container",
                        div { class: "spinner spinner-medium" }
                        p { class: "loading-message", "Redirecting..." }
                    }
                }
            }
        }
        Some(Err(e)) => {
            let error_msg = e.to_string();
            rsx! {
                div { class: "auth-container",
                    div { class: "auth-card",
                        h1 { class: "auth-title", "Error" }
                        p { class: "message message-error", "Failed to check setup status: {error_msg}" }
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
    rsx! { Links {} }
}

#[component]
fn LinksList() -> Element {
    rsx! { LinksListPage {} }
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
