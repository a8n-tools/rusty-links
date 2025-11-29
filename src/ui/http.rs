//! HTTP client abstraction for cross-platform compatibility
//!
//! Uses `gloo-net` for WASM (browser) and `reqwest` for native (server).

use serde::{de::DeserializeOwned, Serialize};

/// Make a GET request and deserialize the JSON response
pub async fn get<T: DeserializeOwned>(url: &str) -> Result<T, String> {
    #[cfg(target_arch = "wasm32")]
    {
        use gloo_net::http::Request;

        let response = Request::get(url)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

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

        let response = Request::get(url)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| format!("Read error: {}", e))?;

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

        let response = Request::post(url)
            .json(body)
            .map_err(|e| format!("Serialize error: {}", e))?
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

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

        let response = Request::post(url)
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

        let response = Request::post(url)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| format!("Read error: {}", e))?;

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

        let response = Request::put(url)
            .json(body)
            .map_err(|e| format!("Serialize error: {}", e))?
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

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

        let response = Request::patch(url)
            .json(body)
            .map_err(|e| format!("Serialize error: {}", e))?
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

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

        let response = Request::delete(url)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

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
}
