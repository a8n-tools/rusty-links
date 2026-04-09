//! HTTP client abstraction for cross-platform compatibility
//!
//! Uses `gloo-net` for WASM (browser) and `reqwest` for native (server).
//! In standalone mode, adds Authorization: Bearer header from localStorage.
//! In saas mode, uses RequestCredentials::Include for cookie-based auth.

use serde::{de::DeserializeOwned, Serialize};

/// Extract a clean error message from an HTTP error response body.
/// Parses JSON `{"error": "..."}` and returns the error field;
/// falls back to a generic message based on status code.
fn clean_error(status: u16, body: &str) -> String {
    // Try to extract "error" field from JSON response
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(error) = json.get("error").and_then(|v| v.as_str()) {
            return error.to_string();
        }
    }

    // If body is short plain text (not JSON, not HTML), return it as-is.
    // HTML bodies (reverse-proxy error pages, dx's dev index, etc.) and
    // oversized blobs fall through to the status-code fallback so the user
    // sees a useful message instead of raw markup.
    let trimmed = body.trim();
    if !trimmed.is_empty()
        && !trimmed.starts_with('{')
        && !trimmed.starts_with('<')
        && trimmed.len() <= 200
    {
        return trimmed.to_string();
    }

    // Fall back to a generic message based on status code
    match status {
        400 => "Invalid request. Please check your input and try again.".to_string(),
        401 => "You need to log in to perform this action.".to_string(),
        403 => "You don't have permission to perform this action.".to_string(),
        404 => "The requested resource was not found.".to_string(),
        409 => "This item already exists.".to_string(),
        422 => "The submitted data is invalid. Please check your input.".to_string(),
        429 => "Too many requests. Please wait a moment and try again.".to_string(),
        500..=599 => "Something went wrong on the server. Please try again later.".to_string(),
        _ => format!("Request failed (status {}).", status),
    }
}

#[cfg(test)]
mod tests {
    use super::clean_error;

    #[test]
    fn extracts_error_field_from_json() {
        let body = r#"{"error":"A database error occurred. Please try again later.","code":"DATABASE_ERROR","status":500}"#;
        assert_eq!(
            clean_error(500, body),
            "A database error occurred. Please try again later."
        );
    }

    #[test]
    fn short_plain_text_passes_through() {
        assert_eq!(clean_error(400, "bad request syntax"), "bad request syntax");
    }

    #[test]
    fn html_proxy_error_page_falls_through_to_generic() {
        let body = "<html><head><title>502 Bad Gateway</title></head><body><h1>Bad Gateway</h1></body></html>";
        assert_eq!(
            clean_error(502, body),
            "Something went wrong on the server. Please try again later."
        );
    }

    #[test]
    fn empty_body_falls_through_to_generic() {
        assert_eq!(
            clean_error(500, ""),
            "Something went wrong on the server. Please try again later."
        );
    }

    #[test]
    fn json_without_error_field_falls_through() {
        assert_eq!(
            clean_error(500, r#"{"something":"else"}"#),
            "Something went wrong on the server. Please try again later."
        );
    }

    #[test]
    fn oversized_plain_text_falls_through() {
        let body = "x".repeat(500);
        assert_eq!(
            clean_error(500, &body),
            "Something went wrong on the server. Please try again later."
        );
    }
}

/// Redirect to /login when an API call returns 401 Unauthorized.
/// Skips auth endpoints so login/setup pages can handle their own 401s.
#[cfg(target_arch = "wasm32")]
fn redirect_if_unauthorized(status: u16, url: &str) {
    if status == 401 && !url.starts_with("/api/auth/") {
        #[cfg(feature = "standalone")]
        crate::ui::auth_state::clear_auth();

        if let Some(window) = web_sys::window() {
            let _ = window.location().set_href("/login");
        }
    }
}

/// Redirect to the membership page when an API call returns 403 with a redirect URL.
/// This handles the SaaS non-member lockout: the user is authenticated but lacks
/// an active membership.
#[cfg(target_arch = "wasm32")]
fn redirect_if_membership_required(status: u16, body: &str) {
    if status == 403 {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
            if let Some(redirect) = json.get("redirect").and_then(|v| v.as_str()) {
                if let Some(window) = web_sys::window() {
                    let _ = window.location().set_href(redirect);
                }
            }
        }
    }
}

/// Make a GET request and deserialize the JSON response
pub async fn get<T: DeserializeOwned>(url: &str) -> Result<T, String> {
    #[cfg(target_arch = "wasm32")]
    {
        use gloo_net::http::Request;

        let mut request = Request::get(url);

        #[cfg(feature = "standalone")]
        {
            if let Some(token) = crate::ui::auth_state::get_token() {
                request = request.header("Authorization", &format!("Bearer {}", token));
            }
        }

        #[cfg(not(feature = "standalone"))]
        {
            use web_sys::RequestCredentials;
            request = request.credentials(RequestCredentials::Include);
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        redirect_if_unauthorized(response.status(), url);

        if !response.ok() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            redirect_if_membership_required(status, &error_text);
            return Err(clean_error(status, &error_text));
        }

        response
            .json::<T>()
            .await
            .map_err(|e| format!("Parse error: {}", e))
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(clean_error(status, &error_text));
        }

        response
            .json::<T>()
            .await
            .map_err(|e| format!("Parse error: {}", e))
    }
}

/// Make a GET request and return the response with status info
pub async fn get_response(url: &str) -> Result<HttpResponse, String> {
    #[cfg(target_arch = "wasm32")]
    {
        use gloo_net::http::Request;

        let mut request = Request::get(url);

        #[cfg(feature = "standalone")]
        {
            if let Some(token) = crate::ui::auth_state::get_token() {
                request = request.header("Authorization", &format!("Bearer {}", token));
            }
        }

        #[cfg(not(feature = "standalone"))]
        {
            use web_sys::RequestCredentials;
            request = request.credentials(RequestCredentials::Include);
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| format!("Read error: {}", e))?;

        redirect_if_unauthorized(status, url);
        redirect_if_membership_required(status, &text);

        Ok(HttpResponse { status, body: text })
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        let status = response.status().as_u16();
        let text = response
            .text()
            .await
            .map_err(|e| format!("Read error: {}", e))?;

        Ok(HttpResponse { status, body: text })
    }
}

/// Make a POST request with JSON body and deserialize the JSON response
pub async fn post<T: DeserializeOwned, B: Serialize>(url: &str, body: &B) -> Result<T, String> {
    #[cfg(target_arch = "wasm32")]
    {
        use gloo_net::http::Request;

        let mut request = Request::post(url);

        #[cfg(feature = "standalone")]
        {
            if let Some(token) = crate::ui::auth_state::get_token() {
                request = request.header("Authorization", &format!("Bearer {}", token));
            }
        }

        #[cfg(not(feature = "standalone"))]
        {
            use web_sys::RequestCredentials;
            request = request.credentials(RequestCredentials::Include);
        }

        let response = request
            .json(body)
            .map_err(|e| format!("Serialize error: {}", e))?
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        redirect_if_unauthorized(response.status(), url);

        if !response.ok() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            redirect_if_membership_required(status, &error_text);
            return Err(clean_error(status, &error_text));
        }

        response
            .json::<T>()
            .await
            .map_err(|e| format!("Parse error: {}", e))
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let client = reqwest::Client::new();
        let response = client
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(clean_error(status, &error_text));
        }

        response
            .json::<T>()
            .await
            .map_err(|e| format!("Parse error: {}", e))
    }
}

/// Make a POST request and return the response with status info
pub async fn post_response<B: Serialize>(url: &str, body: &B) -> Result<HttpResponse, String> {
    #[cfg(target_arch = "wasm32")]
    {
        use gloo_net::http::Request;

        let mut request = Request::post(url);

        #[cfg(feature = "standalone")]
        {
            if let Some(token) = crate::ui::auth_state::get_token() {
                request = request.header("Authorization", &format!("Bearer {}", token));
            }
        }

        #[cfg(not(feature = "standalone"))]
        {
            use web_sys::RequestCredentials;
            request = request.credentials(RequestCredentials::Include);
        }

        let response = request
            .json(body)
            .map_err(|e| format!("Serialize error: {}", e))?
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| format!("Read error: {}", e))?;

        redirect_if_unauthorized(status, url);
        redirect_if_membership_required(status, &text);

        Ok(HttpResponse { status, body: text })
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let client = reqwest::Client::new();
        let response = client
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        let status = response.status().as_u16();
        let text = response
            .text()
            .await
            .map_err(|e| format!("Read error: {}", e))?;

        Ok(HttpResponse { status, body: text })
    }
}

/// Make a POST request without a body
pub async fn post_empty(url: &str) -> Result<HttpResponse, String> {
    #[cfg(target_arch = "wasm32")]
    {
        use gloo_net::http::Request;

        let mut request = Request::post(url);

        #[cfg(feature = "standalone")]
        {
            if let Some(token) = crate::ui::auth_state::get_token() {
                request = request.header("Authorization", &format!("Bearer {}", token));
            }
        }

        #[cfg(not(feature = "standalone"))]
        {
            use web_sys::RequestCredentials;
            request = request.credentials(RequestCredentials::Include);
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| format!("Read error: {}", e))?;

        redirect_if_unauthorized(status, url);
        redirect_if_membership_required(status, &text);

        Ok(HttpResponse { status, body: text })
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let client = reqwest::Client::new();
        let response = client
            .post(url)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        let status = response.status().as_u16();
        let text = response
            .text()
            .await
            .map_err(|e| format!("Read error: {}", e))?;

        Ok(HttpResponse { status, body: text })
    }
}

/// Make a PUT request with JSON body and deserialize the JSON response
pub async fn put<T: DeserializeOwned, B: Serialize>(url: &str, body: &B) -> Result<T, String> {
    #[cfg(target_arch = "wasm32")]
    {
        use gloo_net::http::Request;

        let mut request = Request::put(url);

        #[cfg(feature = "standalone")]
        {
            if let Some(token) = crate::ui::auth_state::get_token() {
                request = request.header("Authorization", &format!("Bearer {}", token));
            }
        }

        #[cfg(not(feature = "standalone"))]
        {
            use web_sys::RequestCredentials;
            request = request.credentials(RequestCredentials::Include);
        }

        let response = request
            .json(body)
            .map_err(|e| format!("Serialize error: {}", e))?
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        redirect_if_unauthorized(response.status(), url);

        if !response.ok() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            redirect_if_membership_required(status, &error_text);
            return Err(clean_error(status, &error_text));
        }

        response
            .json::<T>()
            .await
            .map_err(|e| format!("Parse error: {}", e))
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let client = reqwest::Client::new();
        let response = client
            .put(url)
            .json(body)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(clean_error(status, &error_text));
        }

        response
            .json::<T>()
            .await
            .map_err(|e| format!("Parse error: {}", e))
    }
}

/// Make a PATCH request with JSON body and deserialize the JSON response
pub async fn patch<T: DeserializeOwned, B: Serialize>(url: &str, body: &B) -> Result<T, String> {
    #[cfg(target_arch = "wasm32")]
    {
        use gloo_net::http::Request;

        let mut request = Request::patch(url);

        #[cfg(feature = "standalone")]
        {
            if let Some(token) = crate::ui::auth_state::get_token() {
                request = request.header("Authorization", &format!("Bearer {}", token));
            }
        }

        #[cfg(not(feature = "standalone"))]
        {
            use web_sys::RequestCredentials;
            request = request.credentials(RequestCredentials::Include);
        }

        let response = request
            .json(body)
            .map_err(|e| format!("Serialize error: {}", e))?
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        redirect_if_unauthorized(response.status(), url);

        if !response.ok() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            redirect_if_membership_required(status, &error_text);
            return Err(clean_error(status, &error_text));
        }

        response
            .json::<T>()
            .await
            .map_err(|e| format!("Parse error: {}", e))
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let client = reqwest::Client::new();
        let response = client
            .patch(url)
            .json(body)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(clean_error(status, &error_text));
        }

        response
            .json::<T>()
            .await
            .map_err(|e| format!("Parse error: {}", e))
    }
}

/// Make a DELETE request
pub async fn delete(url: &str) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        use gloo_net::http::Request;

        let mut request = Request::delete(url);

        #[cfg(feature = "standalone")]
        {
            if let Some(token) = crate::ui::auth_state::get_token() {
                request = request.header("Authorization", &format!("Bearer {}", token));
            }
        }

        #[cfg(not(feature = "standalone"))]
        {
            use web_sys::RequestCredentials;
            request = request.credentials(RequestCredentials::Include);
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        redirect_if_unauthorized(response.status(), url);

        if !response.ok() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            redirect_if_membership_required(status, &error_text);
            return Err(clean_error(status, &error_text));
        }

        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let client = reqwest::Client::new();
        let response = client
            .delete(url)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(clean_error(status, &error_text));
        }

        Ok(())
    }
}

/// HTTP response with status and body
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

impl HttpResponse {
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    pub fn json<T: DeserializeOwned>(&self) -> Result<T, String> {
        serde_json::from_str(&self.body).map_err(|e| format!("Parse error: {}", e))
    }

    /// Extract a human-readable error message from the response body.
    /// Tries to parse as JSON `{"error": "..."}` and returns the error field;
    /// falls back to a friendly message based on status code.
    pub fn error_message(&self) -> String {
        clean_error(self.status, &self.body)
    }
}
