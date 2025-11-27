#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;

/// Check if a string is a valid HTTP/HTTPS URL
pub fn is_valid_url(text: &str) -> bool {
    let trimmed = text.trim();

    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        return false;
    }

    // Use url crate for validation
    url::Url::parse(trimmed).is_ok()
}

/// Read text from the clipboard (browser only)
#[cfg(target_arch = "wasm32")]
pub async fn read_clipboard() -> Result<String, String> {
    let window = web_sys::window().ok_or("No window available")?;
    let navigator = window.navigator();
    let clipboard = navigator.clipboard();

    if let Some(clipboard) = clipboard {
        let promise = clipboard.read_text();
        let result = JsFuture::from(promise).await
            .map_err(|_| "Failed to read clipboard")?;

        let text = result.as_string()
            .ok_or("Clipboard content is not a string")?;

        Ok(text)
    } else {
        Err("Clipboard API not available".to_string())
    }
}

/// Fallback for non-browser environments
#[cfg(not(target_arch = "wasm32"))]
pub async fn read_clipboard() -> Result<String, String> {
    Err("Clipboard not available in non-browser environment".to_string())
}
