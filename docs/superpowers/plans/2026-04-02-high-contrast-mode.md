# High Contrast Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a manual high contrast mode toggle button to the navbar that persists via localStorage, layering on top of the existing `prefers-contrast: high` media query.

**Architecture:** A new Dioxus component (`HighContrastToggle`) reads/writes a `high-contrast` localStorage key and toggles a `.high-contrast` CSS class on `document.body`. The CSS class duplicates the existing media query overrides so both manual and OS-level preferences work independently.

**Tech Stack:** Dioxus 0.7, Tailwind CSS v4, web-sys (localStorage + DOM manipulation)

---

### Task 1: Add `.high-contrast` CSS class

**Files:**
- Modify: `tailwind.css:3414-3430` (Accessibility section)

- [ ] **Step 1: Add the `.high-contrast` class after the existing media query**

Add this block immediately after the closing `}` of `@media (prefers-contrast: high)` (after line 3430 in `tailwind.css`):

```css
body.high-contrast {
    --color-primary-500: #c2530a;
    --color-accent-500: #8b3410;
    --color-border: #000;
    --color-text-primary: #000;
}

body.high-contrast button,
body.high-contrast .btn {
    border: 2px solid currentcolor;
}
```

- [ ] **Step 2: Commit**

```bash
git add tailwind.css
git commit -m "feat: add .high-contrast CSS class for manual toggle"
```

---

### Task 2: Create `HighContrastToggle` component

**Files:**
- Create: `src/ui/components/high_contrast_toggle.rs`

- [ ] **Step 1: Create the component file**

Write `src/ui/components/high_contrast_toggle.rs` with the following content:

```rust
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
```

- [ ] **Step 2: Commit**

```bash
git add src/ui/components/high_contrast_toggle.rs
git commit -m "feat: add HighContrastToggle component"
```

---

### Task 3: Export the new component module

**Files:**
- Modify: `src/ui/components/mod.rs`

- [ ] **Step 1: Add the module declaration**

Add `pub mod high_contrast_toggle;` to `src/ui/components/mod.rs`. Insert it alphabetically between `error` and `language_select`:

```rust
pub mod error;
pub mod high_contrast_toggle;
pub mod language_select;
```

- [ ] **Step 2: Commit**

```bash
git add src/ui/components/mod.rs
git commit -m "feat: export high_contrast_toggle module"
```

---

### Task 4: Integrate toggle into navbar

**Files:**
- Modify: `src/ui/components/navbar.rs`

- [ ] **Step 1: Add the import**

Add this import at the top of `src/ui/components/navbar.rs` (after the existing `use` statements on line 2):

```rust
use crate::ui::components::high_contrast_toggle::HighContrastToggle;
```

- [ ] **Step 2: Add toggle to desktop navbar**

In the desktop menu `div` (around line 82-83), add `HighContrastToggle {}` between `NavLinks` and `LogoutButton`:

```rust
                    // Desktop menu (hidden on mobile)
                    div {
                        class: "hidden md:flex gap-2 lg:gap-3 items-center",
                        role: "menubar",
                        "aria-label": "Site navigation",
                        NavLinks { on_click: close_menu }
                        HighContrastToggle {}
                        LogoutButton { loading: loading(), on_logout: on_logout }
                    }
```

- [ ] **Step 3: Add toggle to mobile menu**

In the mobile menu `div` (around line 132-134), add `HighContrastToggle { mobile: true }` between `NavLinks` and `LogoutButton`:

```rust
                    div { class: "flex flex-col gap-2",
                        NavLinks { on_click: close_menu, mobile: true }
                        HighContrastToggle { mobile: true }
                        LogoutButton { loading: loading(), on_logout: on_logout, mobile: true }
                    }
```

- [ ] **Step 4: Commit**

```bash
git add src/ui/components/navbar.rs
git commit -m "feat: add high contrast toggle to navbar (desktop + mobile)"
```
