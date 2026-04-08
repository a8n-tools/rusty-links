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
                        let classes = body.class_name();
                        if !classes.contains("high-contrast") {
                            let new_classes = format!("{classes} high-contrast");
                            body.set_class_name(new_classes.trim());
                        }
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
                    let classes = body.class_name();
                    if new_state {
                        if !classes.contains("high-contrast") {
                            let new_classes = format!("{classes} high-contrast");
                            body.set_class_name(new_classes.trim());
                        }
                    } else {
                        let new_classes = classes.replace("high-contrast", "");
                        body.set_class_name(new_classes.trim());
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
        "inline-flex items-center justify-center w-7 h-7 rounded text-text-muted hover:text-text-primary hover:bg-surface-200 transition-colors"
    };

    let icon_on = if active() { "" } else { "hidden" };
    let icon_off = if active() { "hidden" } else { "" };

    rsx! {
        button {
            class: btn_class,
            onclick: toggle,
            title: "Toggle high contrast",
            "aria-label": "Toggle high contrast",
            "aria-pressed": if active() { "true" } else { "false" },
            role: "menuitem",
            // Filled circle-half icon (high contrast ON)
            svg {
                class: "w-4 h-4 {icon_on}",
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
            // Outline circle-half icon (high contrast OFF)
            svg {
                class: "w-4 h-4 {icon_off}",
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
            if mobile {
                "High Contrast"
            }
        }
    }
}
