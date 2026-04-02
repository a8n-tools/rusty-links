use dioxus::prelude::*;

#[component]
pub fn HighContrastToggle(#[props(default = false)] mobile: bool) -> Element {
    let mut active = use_signal(|| false);

    // On mount, read localStorage and apply class if needed
    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                let is_on = window
                    .local_storage()
                    .ok()
                    .flatten()
                    .and_then(|s| s.get_item("high-contrast").ok().flatten())
                    .map(|v| v == "true")
                    .unwrap_or(false);

                if is_on {
                    if let Some(body) = window.document().and_then(|d| d.body()) {
                        let _ = body.class_list().add_1("high-contrast");
                    }
                    active.set(true);
                }
            }
        }
    });

    let toggle = move |_| {
        let new_state = !active();
        active.set(new_state);

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                // Toggle class on body
                if let Some(body) = window.document().and_then(|d| d.body()) {
                    if new_state {
                        let _ = body.class_list().add_1("high-contrast");
                    } else {
                        let _ = body.class_list().remove_1("high-contrast");
                    }
                }

                // Persist to localStorage
                if let Ok(Some(storage)) = window.local_storage() {
                    let _ = storage.set_item(
                        "high-contrast",
                        if new_state { "true" } else { "false" },
                    );
                }
            }
        }
    };

    let btn_class = if mobile {
        "flex items-center justify-center w-full px-4 py-3 bg-transparent border border-surface-300 text-text-muted rounded-md font-medium hover:bg-surface-200 hover:text-text-primary transition-colors text-sm gap-2"
    } else {
        "p-2 rounded-md text-text-muted hover:bg-surface-200 hover:text-text-primary transition-colors"
    };

    rsx! {
        button {
            class: btn_class,
            onclick: toggle,
            title: "Toggle high contrast",
            "aria-label": "Toggle high contrast",
            "aria-pressed": if active() { "true" } else { "false" },
            role: "menuitem",
            if active() {
                // Filled circle-half icon (high contrast ON)
                svg {
                    class: "w-5 h-5",
                    fill: "currentColor",
                    "viewBox": "0 0 24 24",
                    "aria-hidden": "true",
                    circle {
                        cx: "12",
                        cy: "12",
                        r: "10",
                        stroke: "currentColor",
                        "stroke-width": "2",
                        fill: "none",
                    }
                    path {
                        d: "M12 2a10 10 0 0 1 0 20V2z",
                    }
                }
            } else {
                // Outline circle-half icon (high contrast OFF)
                svg {
                    class: "w-5 h-5",
                    fill: "none",
                    "viewBox": "0 0 24 24",
                    "aria-hidden": "true",
                    circle {
                        cx: "12",
                        cy: "12",
                        r: "10",
                        stroke: "currentColor",
                        "stroke-width": "2",
                    }
                    path {
                        d: "M12 2v20",
                        stroke: "currentColor",
                        "stroke-width": "2",
                    }
                }
            }
            if mobile {
                "High Contrast"
            }
        }
    }
}
