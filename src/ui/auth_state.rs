//! Authentication state management for the browser.
//!
//! Stores JWT tokens in localStorage for standalone mode.
//! In saas mode, authentication is handled by the parent app's cookies.

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
