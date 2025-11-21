use dioxus::prelude::*;
use crate::ui::components::navbar::Navbar;

#[component]
pub fn Links() -> Element {
    rsx! {
        div { class: "page-container",
            Navbar {}
            div { class: "links-page",
                div { class: "links-header",
                    h1 { class: "links-title", "My Links" }
                }
                div { class: "placeholder-content",
                    p { class: "placeholder-text", "Links Page - Coming Soon" }
                    p { style: "color: var(--rust-text-secondary); margin-top: 0.5rem;",
                        "The full links management interface will be implemented in the next steps."
                    }
                }
            }
        }
    }
}
