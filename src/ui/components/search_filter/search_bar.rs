use dioxus::prelude::*;

/// Search bar component with optimized re-renders
/// Uses memoization to prevent unnecessary updates
#[component]
pub fn SearchBar(
    value: String,
    on_change: EventHandler<String>,
    #[props(default = "Search links...".to_string())] placeholder: String,
) -> Element {
    // Memoize the search icon to prevent re-rendering
    let search_icon = use_memo(move || {
        rsx! {
            svg {
                class: "search-icon",
                xmlns: "http://www.w3.org/2000/svg",
                width: "20",
                height: "20",
                view_box: "0 0 24 24",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                circle { cx: "11", cy: "11", r: "8" }
                path { d: "m21 21-4.35-4.35" }
            }
        }
    });

    rsx! {
        div { class: "search-bar",
            // Search icon (memoized for performance)
            {search_icon()}

            input {
                r#type: "text",
                class: "search-input",
                value: "{value}",
                placeholder: "{placeholder}",
                oninput: move |evt| {
                    on_change.call(evt.value());
                }
            }

            if !value.is_empty() {
                button {
                    class: "search-clear",
                    onclick: move |_| on_change.call(String::new()),
                    "×"
                }
            }
        }
    }
}
