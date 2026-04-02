// Apply high-contrast class synchronously before first paint to prevent FOUC.
// The Dioxus HighContrastToggle component manages this class at runtime;
// this script just restores the saved preference on page load.
try {
  if (localStorage.getItem("high-contrast") === "true") {
    document.body.classList.add("high-contrast");
  }
} catch (_) {}
