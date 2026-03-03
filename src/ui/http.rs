//! HTTP client abstraction for cross-platform compatibility
//!
//! Uses `gloo-net` for WASM (browser) and `reqwest` for native (server).
//! In standalone mode, adds Authorization: Bearer header from localStorage.
//! In saas mode, uses RequestCredentials::Include for cookie-based auth.

use serde::{de::DeserializeOwned, Serialize};

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
            return Err(format!("HTTP error: {}", response.status()));
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
            return Err(format!("HTTP error: {}", response.status()));
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
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("HTTP error {}: {}", response.status(), error_text));
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
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("HTTP error: {}", error_text));
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
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("HTTP error {}: {}", response.status(), error_text));
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
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("HTTP error: {}", error_text));
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
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("HTTP error {}: {}", response.status(), error_text));
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
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("HTTP error: {}", error_text));
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
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("HTTP error {}: {}", response.status(), error_text));
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
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("HTTP error: {}", error_text));
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
    /// falls back to the raw body text.
    pub fn error_message(&self) -> String {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&self.body) {
            if let Some(error) = json.get("error").and_then(|v| v.as_str()) {
                return error.to_string();
            }
        }
        self.body.clone()
    }
}
