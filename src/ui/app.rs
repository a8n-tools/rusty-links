use dioxus::prelude::*;
use dioxus_router::RouterConfig;

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
enum Route {
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
    rsx! {
        div {
            h1 { "Hello, Rusty Links!" }
            p { "Router is working!" }
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
