use dioxus::prelude::*;
use dioxus_router::Navigator;
use serde::Deserialize;
use crate::ui::pages::{
    setup::Setup, login::Login, links::Links, links_list::LinksListPage,
    categories::CategoriesPage, languages::LanguagesPage, licenses::LicensesPage,
    tags::TagsPage
};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct CheckSetupResponse {
    setup_required: bool,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct User {
    id: String,
    email: String,
}

#[component]
pub fn App() -> Element {
    rsx! {
        link { rel: "stylesheet", href: "/assets/style.css" }
        Router::<Route> {}
    }
}

#[derive(Clone, Routable, Debug, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[route("/")]
    Home {},
    #[route("/setup")]
    Setup {},
    #[route("/login")]
    Login {},
    #[route("/links")]
    Links {},
    #[route("/links-table")]
    LinksTable {},
    #[route("/categories")]
    Categories {},
    #[route("/languages")]
    Languages {},
    #[route("/licenses")]
    Licenses {},
    #[route("/tags")]
    Tags {},
}

#[component]
fn Home() -> Element {
    let mut setup_status = use_signal(|| Option::<bool>::None);
    let nav = navigator();

    // Check setup status on mount
    use_effect(move || {
        spawn(async move {
            let client = reqwest::Client::new();
            let response = client
                .get("/api/auth/check-setup")
                .send()
                .await;

            match response {
                Ok(resp) => {
                    if let Ok(data) = resp.json::<CheckSetupResponse>().await {
                        setup_status.set(Some(data.setup_required));

                        // Redirect based on setup status
                        if data.setup_required {
                            nav.push("/setup");
                        } else {
                            // Check if user is already logged in
                            check_auth_and_redirect(&nav).await;
                        }
                    }
                }
                Err(_) => {
                    // On error, assume we need to check login
                    nav.push("/login");
                }
            }
        });
    });

    rsx! {
        div { class: "auth-container",
            div { class: "auth-card",
                h1 { class: "auth-title", "Rusty Links" }
                p { class: "auth-subtitle", "Loading..." }
            }
        }
    }
}

async fn check_auth_and_redirect(nav: &Navigator) {
    let client = reqwest::Client::new();
    let response = client
        .get("/api/auth/me")
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                // User is logged in, go to links page
                nav.push("/links");
            } else {
                // Not logged in, go to login page
                nav.push("/login");
            }
        }
        Err(_) => {
            // Error checking auth, go to login
            nav.push("/login");
        }
    }
}

#[component]
fn LinksTable() -> Element {
    rsx! { LinksListPage {} }
}

#[component]
fn Categories() -> Element {
    rsx! { CategoriesPage {} }
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
fn Tags() -> Element {
    rsx! { TagsPage {} }
}
