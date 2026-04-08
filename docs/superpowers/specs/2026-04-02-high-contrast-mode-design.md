# High Contrast Mode Toggle

## Summary

Add a manual high contrast mode toggle button to the navbar, placed next to the logout button. The toggle uses an icon-only button (circle-half contrast icon) with a tooltip. The preference persists across sessions via localStorage. This layers on top of the existing `@media (prefers-contrast: high)` media query — OS-level preferences continue to work automatically, and the button gives all users manual control.

## Components

### 1. CSS: `.high-contrast` class (`tailwind.css`)

Add a `.high-contrast` selector on `body` that duplicates the existing `@media (prefers-contrast: high)` overrides. Both the media query and the class apply the same styles — they layer independently.

Overrides (matching existing media query):
- `--color-primary-500: #c2530a`
- `--color-accent: #8b3410`
- `--color-border: #000`
- `--color-text-primary: #000`
- `--color-text-secondary: #1a1a1a`
- Buttons get `border: 2px solid #000`

### 2. New component: `src/ui/components/high_contrast_toggle.rs`

A Dioxus component that:
- On mount: reads `localStorage.getItem("high-contrast")`. If `"true"`, adds `.high-contrast` to `document.body` and sets internal signal to `true`.
- On click: toggles the class on `document.body`, updates localStorage, flips internal signal.
- Renders an icon-only button with `title="Toggle high contrast"`.
- Icon: inline SVG circle-half (filled half indicates active state). Uses distinct SVGs for on/off states so the button itself communicates its state.
- Styled with existing `btn-icon` class for consistency.

### 3. Navbar integration (`src/ui/components/navbar.rs`)

- Desktop: place `HighContrastToggle {}` immediately before `LogoutButton {}` in the right-side nav area.
- Mobile: place `HighContrastToggle {}` inside the mobile menu, before `LogoutButton {}`.

### 4. Module export (`src/ui/components/mod.rs`)

Add `pub mod high_contrast_toggle;` and re-export the component.

## Behavior

| Scenario | Result |
|----------|--------|
| First visit, no OS preference | Normal theme, toggle off |
| First visit, OS `prefers-contrast: high` | High contrast via media query, toggle off (visual match) |
| User clicks toggle on | `.high-contrast` class added, localStorage set to `"true"` |
| User clicks toggle off | `.high-contrast` class removed, localStorage set to `"false"` |
| Return visit with toggle previously on | Class applied on mount from localStorage |
| OS high contrast + toggle on | Both apply (no conflict, same overrides) |

## localStorage key

`high-contrast` — values: `"true"` or `"false"` (absent treated as false).

## Accessibility

- Button has `title="Toggle high contrast"` for tooltip.
- Button has `aria-label="Toggle high contrast"` for screen readers.
- Button has `aria-pressed` attribute reflecting current state.
- Icon visually changes between on/off states.

## Files changed

| File | Change |
|------|--------|
| `tailwind.css` | Add `body.high-contrast { ... }` overrides |
| `src/ui/components/high_contrast_toggle.rs` | New file — toggle component |
| `src/ui/components/mod.rs` | Add module export |
| `src/ui/components/navbar.rs` | Add toggle next to logout button (desktop + mobile) |
