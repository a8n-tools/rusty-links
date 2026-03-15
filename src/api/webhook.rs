use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::sync::atomic::Ordering;

use super::AppState;

type HmacSha256 = Hmac<Sha256>;

#[derive(serde::Deserialize)]
struct MaintenancePayload {
    event: String,
    #[allow(dead_code)]
    slug: Option<String>,
    maintenance_mode: bool,
    maintenance_message: Option<String>,
    #[allow(dead_code)]
    timestamp: Option<String>,
}

/// POST /api/webhooks/maintenance
///
/// Receives maintenance mode toggle from the parent SaaS platform.
/// Authenticated via HMAC-SHA256 signature in X-Webhook-Signature header.
pub async fn handle_maintenance_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // Read signature header
    let signature = match headers.get("X-Webhook-Signature").and_then(|v| v.to_str().ok()) {
        Some(sig) => sig,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Missing X-Webhook-Signature header"})),
            )
                .into_response();
        }
    };

    // Compute HMAC-SHA256 of raw body
    let Ok(mut mac) = HmacSha256::new_from_slice(state.config.saas_jwt_secret.as_bytes()) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "HMAC key error"})),
        )
            .into_response();
    };
    mac.update(&body);

    // Decode the hex signature and verify
    let Ok(sig_bytes) = hex::decode(signature) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Invalid signature format"})),
        )
            .into_response();
    };
    if mac.verify_slice(&sig_bytes).is_err() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Invalid signature"})),
        )
            .into_response();
    }

    // Deserialize payload
    let Ok(payload) = serde_json::from_slice::<MaintenancePayload>(&body) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid payload"})),
        )
            .into_response();
    };

    // Validate event type
    if payload.event != "maintenance_mode_changed" {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("Unsupported event: {}", payload.event)})),
        )
            .into_response();
    }

    // Update state
    state
        .maintenance_mode
        .store(payload.maintenance_mode, Ordering::SeqCst);
    {
        let mut msg = state.maintenance_message.write().unwrap();
        *msg = if payload.maintenance_mode {
            payload.maintenance_message
        } else {
            None
        };
    }

    tracing::info!(
        maintenance_mode = payload.maintenance_mode,
        "Maintenance mode updated via webhook"
    );

    (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))).into_response()
}
