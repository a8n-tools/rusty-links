use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::Serialize;
use std::sync::atomic::Ordering;

use crate::api::AppState;

/// Health check response for the scheduler
#[derive(Serialize)]
pub struct SchedulerHealthResponse {
    pub status: String,
    pub running: bool,
}

/// Scheduler health check endpoint
///
/// Returns the current status of the background scheduler.
/// This endpoint can be used for monitoring and alerting.
///
/// GET /api/health/scheduler
pub async fn scheduler_health(
    State(state): State<AppState>,
) -> Result<Json<SchedulerHealthResponse>, StatusCode> {
    // If shutdown signal is set, scheduler is stopping or stopped
    let running = !state.scheduler_shutdown.load(Ordering::Relaxed);

    let response = SchedulerHealthResponse {
        status: if running { "healthy".to_string() } else { "stopped".to_string() },
        running,
    };

    Ok(Json(response))
}

/// Database health check response
#[derive(Serialize)]
pub struct DatabaseHealthResponse {
    pub status: String,
    pub connected: bool,
}

/// Database health check endpoint
///
/// Verifies database connectivity.
///
/// GET /api/health/database
pub async fn database_health(
    State(state): State<AppState>,
) -> Result<Json<DatabaseHealthResponse>, StatusCode> {
    // Try a simple query to verify database connection
    let connected = sqlx::query("SELECT 1")
        .fetch_one(&state.pool)
        .await
        .is_ok();

    let response = DatabaseHealthResponse {
        status: if connected { "healthy".to_string() } else { "unhealthy".to_string() },
        connected,
    };

    if connected {
        Ok(Json(response))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

/// Overall health check response
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// General health check endpoint
///
/// Returns basic application health status.
///
/// GET /api/health
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}
