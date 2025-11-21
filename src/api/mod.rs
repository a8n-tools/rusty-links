//! API endpoints for Rusty Links
//!
//! This module provides REST API endpoints for the application.
//!
//! # Modules
//!
//! - `auth` - Authentication endpoints (login, logout, setup, etc.)
//! - `links` - Link management endpoints
//!
//! Future modules will include:
//! - `categories` - Category management endpoints
//! - `tags` - Tag management endpoints

pub mod auth;
pub mod categories;
pub mod languages;
pub mod licenses;
pub mod links;
pub mod tags;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;

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
/// # State
///
/// The router requires a `PgPool` as shared state for database access.
pub fn create_router(pool: PgPool) -> Router {
    // Create auth router
    let auth_router = Router::new()
        .route("/setup", post(auth::setup_handler))
        .route("/login", post(auth::login_handler))
        .route("/logout", post(auth::logout_handler))
        .route("/me", get(auth::me_handler))
        .route("/check-setup", get(auth::check_setup_handler));

    // Create main API router
    Router::new()
        .nest("/auth", auth_router)
        .nest("/links", links::create_router())
        .nest("/categories", categories::create_router())
        .nest("/tags", tags::create_router())
        .nest("/languages", languages::create_router())
        .nest("/licenses", licenses::create_router())
        .with_state(pool)
}
