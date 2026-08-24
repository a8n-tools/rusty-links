//! Authentication state management for the browser.
//!
//! Stores JWT tokens in localStorage for standalone mode.
//! In saas mode, authentication is handled by the parent app's cookies.
//!
//! Also mints and keeps the device id the LINKS-45 approval gate recognises a
//! browser by. It is deliberately NOT cleared by [`clear_auth`]: signing out is
//! not "this is a new machine", and forgetting it on every sign-out would hold
//! the user for approval every single time.

#[cfg(target_arch = "wasm32")]
use web_sys::window;

/// Save authentication tokens to localStorage
#[cfg(target_arch = "wasm32")]
pub fn save_auth(token: &str, refresh_token: &str, email: &str) {
    if let Some(storage) = get_storage() {
        let _ = storage.set_item("auth_token", token);
        let _ = storage.set_item("refresh_token", refresh_token);
        let _ = storage.set_item("auth_email", email);
    }
}

/// Get the JWT access token from localStorage
#[cfg(target_arch = "wasm32")]
pub fn get_token() -> Option<String> {
    get_storage()?.get_item("auth_token").ok()?
}

/// Get the refresh token from localStorage
#[cfg(target_arch = "wasm32")]
pub fn get_refresh_token() -> Option<String> {
    get_storage()?.get_item("refresh_token").ok()?
}

/// Clear all auth data from localStorage
#[cfg(target_arch = "wasm32")]
pub fn clear_auth() {
    if let Some(storage) = get_storage() {
        let _ = storage.remove_item("auth_token");
        let _ = storage.remove_item("refresh_token");
        let _ = storage.remove_item("auth_email");
    }
}

/// Check if user has an auth token stored
#[cfg(target_arch = "wasm32")]
pub fn is_authenticated() -> bool {
    get_token().is_some()
}

/// Key the device id is kept under. Distinct from the `auth_*` keys because it
/// outlives any one session.
#[cfg(target_arch = "wasm32")]
const DEVICE_ID_KEY: &str = "rl_device_id";

/// This browser's device id for the LINKS-45 gate, minted on first use and kept
/// afterwards.
///
/// A random version-4 UUID: it identifies the browser profile to the server and
/// nothing else, and it grants nothing on its own, because a value the account
/// has no row for is exactly as unrecognised as no value at all. Returns `None`
/// when localStorage is unavailable (a locked-down browser, a private window in
/// some configurations), and the sign-in then falls back to country-only
/// behaviour rather than failing.
#[cfg(target_arch = "wasm32")]
pub fn device_id() -> Option<String> {
    let storage = get_storage()?;

    if let Ok(Some(existing)) = storage.get_item(DEVICE_ID_KEY) {
        if !existing.trim().is_empty() {
            return Some(existing);
        }
    }

    let minted = uuid::Uuid::new_v4().simple().to_string();
    storage.set_item(DEVICE_ID_KEY, &minted).ok()?;
    Some(minted)
}

#[cfg(target_arch = "wasm32")]
fn get_storage() -> Option<web_sys::Storage> {
    window()?.local_storage().ok()?
}

// Non-WASM stubs (server-side rendering)
#[cfg(not(target_arch = "wasm32"))]
pub fn save_auth(_token: &str, _refresh_token: &str, _email: &str) {}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_token() -> Option<String> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_refresh_token() -> Option<String> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
pub fn clear_auth() {}

#[cfg(not(target_arch = "wasm32"))]
pub fn is_authenticated() -> bool {
    false
}

/// No localStorage outside the browser, so the server-rendered pass submits no
/// device id and the sign-in stays on country-only behaviour.
#[cfg(not(target_arch = "wasm32"))]
pub fn device_id() -> Option<String> {
    None
}
