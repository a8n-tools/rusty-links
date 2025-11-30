//! API endpoints for Rusty Links
//!
//! This module provides REST API endpoints for the application.
//!
//! # Modules
//!
//! - `auth` - Authentication endpoints (login, logout, setup, etc.)
//! - `links` - Link management endpoints
//! - `categories` - Category management endpoints
//! - `tags` - Tag management endpoints
//! - `health` - Health check endpoints

pub mod auth;
pub mod categories;
pub mod health;
pub mod languages;
pub mod licenses;
pub mod links;
pub mod scrape;
pub mod tags;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub scheduler_shutdown: Arc<AtomicBool>,
}

// Allow extracting PgPool from AppState for routes that only need the pool
impl axum::extract::FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> PgPool {
        state.pool.clone()
    }
}

/// Create the main API router with all endpoints
///
/// This function creates an Axum router with all API endpoints
/// mounted at their respective paths.
///
/// # Routes
///
/// ## Authentication (`/api/auth`)
/// - POST /api/auth/setup - Create first user
/// - POST /api/auth/login - Login with email/password
/// - POST /api/auth/logout - Logout and clear session
/// - GET /api/auth/me - Get current user
/// - GET /api/auth/check-setup - Check if setup is required
///
/// ## Links (`/api/links`)
/// - POST /api/links - Create a new link
/// - GET /api/links - List all links
///
/// ## Health (`/api/health`)
/// - GET /api/health - General health check
/// - GET /api/health/database - Database connectivity check
/// - GET /api/health/scheduler - Scheduler status check
///
/// # State
///
/// The router requires an `AppState` containing database pool and scheduler shutdown handle.
pub fn create_router(pool: PgPool, scheduler_shutdown: Arc<AtomicBool>) -> Router {
    let state = AppState {
        pool: pool.clone(),
        scheduler_shutdown,
    };

    // Create health router with AppState
    let health_router = Router::new()
        .route("/health", get(health::health))
        .route("/health/database", get(health::database_health))
        .route("/health/scheduler", get(health::scheduler_health))
        .with_state(state);

    // Create auth router
    let auth_router = Router::new()
        .route("/setup", post(auth::setup_handler))
        .route("/login", post(auth::login_handler))
        .route("/logout", post(auth::logout_handler))
        .route("/me", get(auth::me_handler))
        .route("/check-setup", get(auth::check_setup_handler));

    // Create main API router
    // Health routes use AppState, other routes use PgPool
    Router::new()
        .merge(health_router)
        .nest("/auth", auth_router)
        .nest("/links", links::create_router())
        .nest("/categories", categories::create_router())
        .nest("/tags", tags::create_router())
        .nest("/languages", languages::create_router())
        .nest("/licenses", licenses::create_router())
        .nest("/scrape", scrape::create_router())
        .with_state(pool)
}
