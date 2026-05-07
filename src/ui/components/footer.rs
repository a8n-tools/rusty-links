use crate::build_info;
use dioxus::prelude::*;

#[component]
pub fn Footer() -> Element {
    rsx! {
        footer {
            class: "mt-auto py-3 px-4 text-center text-xs text-text-muted border-t border-surface-300",
            role: "contentinfo",
            "aria-label": "Build information",
            span { class: "font-mono",
                "Rusty Links {build_info::VERSION} \u{00B7} "
                span { "aria-label": "git commit", "{build_info::GIT_HASH}" }
                " \u{00B7} "
                span { "aria-label": "build date", "{build_info::BUILD_DATE}" }
            }
        }
    }
}
