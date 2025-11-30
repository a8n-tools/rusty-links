use dioxus::prelude::*;

#[component]
pub fn ModalSection(title: String, children: Element) -> Element {
    rsx! {
        div { class: "modal-section",
            h3 { class: "section-title", "{title}" }
            div { class: "section-content",
                {children}
            }
        }
    }
}
