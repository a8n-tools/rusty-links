//! API endpoints for Rusty Links

#[cfg(feature = "standalone")]
pub mod admin;
pub mod auth;
pub mod categories;
pub mod health;
pub mod languages;
pub mod licenses;
pub mod links;
pub mod scrape;
pub mod tags;

use axum::{
    routing::{delete, get, post},
    Router,
};
use sqlx::PgPool;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::config::Config;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
    pub scheduler_shutdown: Arc<AtomicBool>,
}

// Allow extracting PgPool from AppState
impl axum::extract::FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> PgPool {
        state.pool.clone()
    }
}

// Allow extracting Config from AppState (needed by JWT middleware)
impl axum::extract::FromRef<AppState> for Config {
    fn from_ref(state: &AppState) -> Config {
        state.config.clone()
    }
}

/// Create the main API router with all endpoints
pub fn create_router(pool: PgPool, config: Config, scheduler_shutdown: Arc<AtomicBool>) -> Router {
    let state = AppState {
        pool: pool.clone(),
        config,
        scheduler_shutdown,
    };

    // Create health router with AppState
    let health_router = Router::new()
        .route("/health", get(health::health))
        .route("/health/database", get(health::database_health))
        .route("/health/scheduler", get(health::scheduler_health))
        .with_state(state.clone());

    // Create auth router (feature-gated)
    #[cfg(feature = "standalone")]
    let auth_router = {
        Router::new()
            .route("/setup", post(auth::setup_handler))
            .route("/register", post(auth::register_handler))
            .route("/login", post(auth::login_handler))
            .route("/refresh", post(auth::refresh_handler))
            .route("/me", get(auth::me_handler))
            .route("/check-setup", get(auth::check_setup_handler))
    };

    #[cfg(all(feature = "saas", not(feature = "standalone")))]
    let auth_router = {
        Router::new()
            .route("/me", get(auth::me_handler))
            .route("/check-setup", get(auth::check_setup_handler))
    };

    #[cfg(not(any(feature = "standalone", feature = "saas")))]
    let auth_router = {
        Router::new().route("/check-setup", get(auth::check_setup_handler))
    };

    // Create admin router (standalone only)
    #[cfg(feature = "standalone")]
    let admin_router = Router::new()
        .route("/users", get(admin::list_users))
        .route("/users/{user_id}", delete(admin::delete_user))
        .route("/users/{user_id}/promote", post(admin::promote_user));

    // Create main API router
    let mut router = Router::new()
        .merge(health_router)
        .nest("/auth", auth_router)
        .nest("/links", links::create_router())
        .nest("/categories", categories::create_router())
        .nest("/tags", tags::create_router())
        .nest("/languages", languages::create_router())
        .nest("/licenses", licenses::create_router())
        .nest("/scrape", scrape::create_router());

    #[cfg(feature = "standalone")]
    {
        router = router.nest("/admin", admin_router);
    }

    router.with_state(state)
}
