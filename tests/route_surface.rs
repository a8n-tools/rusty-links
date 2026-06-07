//! Route-surface integration tests for runtime mode selection (LINKS-13).
//!
//! A single binary serves both deployment modes; the mode is resolved at
//! runtime from `OIDC_ISSUER`. These tests assert that the API router mounts
//! the correct endpoints per mode, so endpoints unavailable in a mode return
//! 404 (a routing miss) rather than running and returning a runtime error.
//!
//! They build the router with a *lazy* Postgres pool that never connects: the
//! assertions only exercise unmounted routes (404 happens at the routing layer,
//! before any handler runs) and `/api/health` (which reads config only), so no
//! database is required.
//!
//! Note: the `/oauth2/*` BFF routes and the page-guard / maintenance middleware
//! are assembled in `main.rs` (not `api::create_router`) and are gated by the
//! same `Config::hosted()` switch validated here; see `src/main.rs`.

#![cfg(feature = "server")]

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use rusty_links::api;
use rusty_links::auth::oidc_rs::OidcVerifier;
use rusty_links::config::{Config, OidcConfig};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt; // for `oneshot`

/// Build a baseline config. When `issuer` is non-empty the instance is in
/// hosted mode; empty means standalone mode.
fn config_with_issuer(issuer: &str) -> Config {
    Config {
        database_url: "postgres://user:pass@localhost/rusty_links_test".to_string(),
        app_port: 4002,
        update_interval_days: 30,
        log_level: "info".to_string(),
        update_interval_hours: 24,
        batch_size: 50,
        jitter_percent: 20,
        host_url: "http://localhost:4002".to_string(),
        webhook_secret: "test-webhook-secret".to_string(),
        oidc: OidcConfig {
            issuer: issuer.to_string(),
            audience: "http://localhost:4002/api".to_string(),
            jwks_url: if issuer.is_empty() {
                String::new()
            } else {
                format!("{}/.well-known/jwks.json", issuer)
            },
            jwks_cache_ttl: 300,
            client_id: if issuer.is_empty() {
                String::new()
            } else {
                "test-client".to_string()
            },
            client_secret: if issuer.is_empty() {
                String::new()
            } else {
                "test-secret".to_string()
            },
            redirect_uri: "http://localhost:4002/oauth2/callback".to_string(),
            post_logout_redirect_uri: "http://localhost:4002/".to_string(),
            leeway_seconds: 30,
            lifecycle_jti_cache_ttl: 300,
            session_ttl_seconds: 1_209_600,
        },
        jwt_secret: "test_secret".to_string(),
        jwt_expiry_hours: 1,
        refresh_token_expiry_days: 7,
        account_lockout_attempts: 5,
        account_lockout_duration_minutes: 30,
        allow_registration: true,
    }
}

/// Build the `/api` router for a given config, backed by a lazy pool that never
/// connects (unmounted routes 404 before any handler touches the database).
fn api_router(config: Config) -> axum::Router {
    let pool = PgPoolOptions::new()
        .connect_lazy(&config.database_url)
        .expect("lazy pool");
    let verifier = Arc::new(OidcVerifier::new(config.oidc.clone()));
    api::create_router(
        pool,
        config,
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
        Arc::new(RwLock::new(None)),
        verifier,
    )
}

async fn status_of(router: axum::Router, method: &str, uri: &str) -> StatusCode {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    router.oneshot(req).await.unwrap().status()
}

#[tokio::test]
async fn health_reports_auth_mode_standalone() {
    let router = api_router(config_with_issuer(""));
    let req = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap();
    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["auth_mode"], "standalone");
}

#[tokio::test]
async fn health_reports_auth_mode_hosted() {
    let router = api_router(config_with_issuer("https://issuer.example"));
    let req = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap();
    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["auth_mode"], "hosted");
}

#[tokio::test]
async fn hosted_does_not_serve_local_auth_or_admin() {
    // Hosted mode: local JWT auth + setup + admin endpoints are not mounted.
    let mk = || api_router(config_with_issuer("https://issuer.example"));
    assert_eq!(
        status_of(mk(), "POST", "/auth/login").await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        status_of(mk(), "POST", "/auth/register").await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        status_of(mk(), "POST", "/auth/setup").await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        status_of(mk(), "POST", "/auth/refresh").await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        status_of(mk(), "GET", "/auth/check-setup").await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        status_of(mk(), "GET", "/admin/users").await,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn hosted_serves_maintenance_webhook() {
    // Hosted-only endpoint is mounted (not a 404 routing miss).
    let status = status_of(
        api_router(config_with_issuer("https://issuer.example")),
        "POST",
        "/webhooks/maintenance",
    )
    .await;
    assert_ne!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn standalone_does_not_serve_maintenance_webhook() {
    // Standalone mode: the hosted maintenance webhook is not mounted.
    let status = status_of(
        api_router(config_with_issuer("")),
        "POST",
        "/webhooks/maintenance",
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn standalone_serves_local_auth_and_admin() {
    // Standalone mode: local auth + admin endpoints are mounted (not 404).
    // They route to their handlers (which may then fail without a DB), so the
    // assertion is simply "not a routing miss".
    assert_ne!(
        status_of(
            api_router(config_with_issuer("")),
            "GET",
            "/auth/check-setup"
        )
        .await,
        StatusCode::NOT_FOUND
    );
    assert_ne!(
        status_of(api_router(config_with_issuer("")), "GET", "/admin/users").await,
        StatusCode::NOT_FOUND
    );
}
