//! API endpoints for Rusty Links

pub mod admin;
pub mod auth;
pub mod categories;
pub mod health;
pub mod languages;
pub mod licenses;
pub mod links;
pub mod scrape;
pub mod tags;
pub mod webhook;

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
    /// Hosted-mode maintenance flag. Always present; only flipped by the
    /// maintenance webhook (hosted mode), stays `false` in standalone mode.
    pub maintenance_mode: Arc<AtomicBool>,
    pub maintenance_message: Arc<std::sync::RwLock<Option<String>>>,
    pub oidc_verifier: Arc<crate::auth::oidc_rs::OidcVerifier>,
}

impl axum::extract::FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> PgPool {
        state.pool.clone()
    }
}

impl axum::extract::FromRef<AppState> for Config {
    fn from_ref(state: &AppState) -> Config {
        state.config.clone()
    }
}

/// Create the main API router with all endpoints.
pub fn create_router(
    pool: PgPool,
    config: Config,
    scheduler_shutdown: Arc<AtomicBool>,
    maintenance_mode: Arc<AtomicBool>,
    maintenance_message: Arc<std::sync::RwLock<Option<String>>>,
    oidc_verifier: Arc<crate::auth::oidc_rs::OidcVerifier>,
) -> Router {
    let state = AppState {
        pool: pool.clone(),
        config: config.clone(),
        scheduler_shutdown,
        maintenance_mode,
        maintenance_message,
        oidc_verifier: oidc_verifier.clone(),
    };

    let health_router = Router::new()
        .route("/health", get(health::health))
        .route("/health/database", get(health::database_health))
        .route("/health/scheduler", get(health::scheduler_health))
        .with_state(state.clone());

    // Mode-specific route surface. Endpoints unavailable in a mode are simply
    // not mounted, so they return 404 (not a runtime 403) exactly as before.
    let auth_router = if config.hosted() {
        // Hosted mode: OIDC owns login/logout/setup; only /me is served here.
        Router::new().route("/me", get(auth::me_handler))
    } else {
        // Standalone mode: local JWT auth surface.
        Router::new()
            .route("/setup", post(auth::setup_handler))
            .route("/register", post(auth::register_handler))
            .route("/login", post(auth::login_handler))
            .route("/refresh", post(auth::refresh_handler))
            .route("/logout", post(auth::logout_handler))
            .route("/me", get(auth::me_handler))
            .route("/check-setup", get(auth::check_setup_handler))
    };

    let mut router = Router::new()
        .merge(health_router)
        .nest("/auth", auth_router)
        .nest("/links", links::create_router())
        .nest("/categories", categories::create_router())
        .nest("/tags", tags::create_router())
        .nest("/languages", languages::create_router())
        .nest("/licenses", licenses::create_router())
        .nest("/scrape", scrape::create_router());

    if config.hosted() {
        // Hosted-only: maintenance webhook + bearer at+jwt verification.
        router = router
            .route(
                "/webhooks/maintenance",
                post(webhook::handle_maintenance_webhook),
            )
            .layer(axum::Extension(oidc_verifier));
    } else {
        // Standalone-only: local user administration.
        let admin_router = Router::new()
            .route("/users", get(admin::list_users))
            .route("/users/{user_id}", delete(admin::delete_user))
            .route("/users/{user_id}/promote", post(admin::promote_user));
        router = router.nest("/admin", admin_router);
    }

    router.with_state(state)
}
