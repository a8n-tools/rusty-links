//! Shared setup for the integration-test targets.
//!
//! Every helper is fail-fast on purpose (LINKS-44): a missing or unreachable
//! `DATABASE_URL` panics instead of skipping, because a suite that skips still
//! exits 0 and looks green, which is the blindness this module exists to
//! remove. Nothing here is gated on the database being available.
//!
//! The suites share one Postgres server but never the application database:
//! [`test_pool`] derives a sibling `<db>_test` database from `DATABASE_URL` and
//! runs the migrations there, so `just pre-commit` against the compose
//! `postgres` service cannot touch a developer's dev data.

// Each tests/*.rs target compiles this module and uses only a subset of it.
#![allow(dead_code)]

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use rusty_links::api;
use rusty_links::auth::oidc_rs::OidcVerifier;
use rusty_links::config::{Config, MailConfig, OidcConfig};
use rusty_links::models::{create_user, CreateUser, User};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tokio::sync::OnceCell;
use url::Url;
use uuid::Uuid;

/// How long to keep retrying the first connection. A CI Postgres service can
/// still be starting when the first test target runs.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(60);

const NO_DATABASE_URL: &str = "\
DATABASE_URL is not set, and the database-backed tests never skip.
  - `just pre-commit` runs them inside the dev compose `app` container, where
    compose.dev.yml sets DATABASE_URL to the `postgres` service.
  - `just db-up` starts that Postgres on its own.
  - Otherwise point DATABASE_URL at any reachable Postgres; the suites use a
    sibling `<db>_test` database, never the one named in the URL.
See docs/TESTING.md.";

/// URL of the created and migrated `<db>_test` database. The URL is cached and
/// the pool is not: every `#[tokio::test]` gets its own runtime, and a pool that
/// outlives the runtime its sockets were opened on hands out connections nothing
/// drives, which surfaces as `PoolTimedOut`.
static TEST_DATABASE_URL: OnceCell<String> = OnceCell::const_new();

/// A pool on the migrated test database, owned by the calling test.
///
/// # Panics
///
/// Panics when `DATABASE_URL` is unset, the server is unreachable, or the
/// migrations do not apply. All three are failures, never skips.
pub async fn test_pool() -> PgPool {
    let url = TEST_DATABASE_URL.get_or_init(bootstrap).await;
    PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(10))
        .connect(url)
        .await
        .unwrap_or_else(|error| panic!("connecting to the test database: {error}"))
}

/// Create the test database if it is missing and migrate it. Runs once per test
/// binary and closes both pools before returning, so nothing outlives the
/// runtime that opened them.
async fn bootstrap() -> String {
    let url = std::env::var("DATABASE_URL").expect(NO_DATABASE_URL);
    let (admin_url, test_url, test_db) = derive_test_database(&url);

    // The maintenance database is the only one guaranteed to exist, so the
    // CREATE runs from there.
    let admin = connect_with_retry(&admin_url).await;
    if let Err(error) = sqlx::query(&format!(r#"CREATE DATABASE "{test_db}""#))
        .execute(&admin)
        .await
    {
        // 42P04 is duplicate_database: another target created it first.
        let already_there = error
            .as_database_error()
            .and_then(|db| db.code())
            .is_some_and(|code| code == "42P04");
        assert!(already_there, "creating test database {test_db}: {error}");
    }
    admin.close().await;

    let pool = connect_with_retry(&test_url).await;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .unwrap_or_else(|error| panic!("migrations must apply to {test_db}: {error}"));
    pool.close().await;

    test_url
}

/// Split `DATABASE_URL` into the maintenance URL, the `<db>_test` URL, and the
/// test database name.
fn derive_test_database(url: &str) -> (String, String, String) {
    let parsed =
        Url::parse(url).unwrap_or_else(|error| panic!("DATABASE_URL is not a URL: {error}"));
    let name = parsed.path().trim_start_matches('/').to_string();
    assert!(
        !name.is_empty(),
        "DATABASE_URL must name a database, got {url}"
    );

    let test_db = format!("{name}_test");
    let mut admin = parsed.clone();
    admin.set_path("/postgres");
    let mut test = parsed;
    test.set_path(&format!("/{test_db}"));
    (admin.to_string(), test.to_string(), test_db)
}

async fn connect_with_retry(url: &str) -> PgPool {
    let deadline = Instant::now() + CONNECT_TIMEOUT;
    loop {
        let attempt = PgPoolOptions::new()
            .max_connections(2)
            .acquire_timeout(Duration::from_secs(10))
            .connect(url)
            .await;
        match attempt {
            Ok(pool) => return pool,
            Err(_) if Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            Err(error) => {
                panic!("Postgres at DATABASE_URL is unreachable: {error}\n{NO_DATABASE_URL}")
            }
        }
    }
}

/// Password every fixture user is created with.
pub const TEST_PASSWORD: &str = "TestPassword123!";

/// An address no other test can collide on. The suites share one database, so
/// every fixture is per-test rather than a global truncate.
pub fn unique_email() -> String {
    format!("test-{}@example.com", Uuid::new_v4())
}

/// A user owned by the calling test, cleaned up with [`delete_user`].
pub async fn new_user(pool: &PgPool) -> User {
    create_user(
        pool,
        CreateUser {
            email: unique_email(),
            password: TEST_PASSWORD.to_string(),
        },
    )
    .await
    .expect("test user must be created")
}

/// Drop a fixture user. `ON DELETE CASCADE` takes its dependent rows with it.
pub async fn delete_user(pool: &PgPool, id: Uuid) {
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .expect("fixture user must be removable");
}

/// Standalone-mode config (`OIDC_ISSUER` empty) pointed at the test database,
/// with the LINKS-35 approval gate on and mail unconfigured, so approval mail
/// is logged rather than sent.
pub fn test_config() -> Config {
    Config {
        // Cosmetic here: the router is handed the pool from `test_pool`.
        database_url: std::env::var("DATABASE_URL").unwrap_or_default(),
        app_port: 4002,
        update_interval_days: 30,
        log_level: "info".to_string(),
        update_interval_hours: 24,
        batch_size: 50,
        jitter_percent: 20,
        host_url: "http://localhost:4002".to_string(),
        webhook_secret: "test-webhook-secret".to_string(),
        oidc: OidcConfig {
            issuer: String::new(),
            audience: "http://localhost:4002/api".to_string(),
            jwks_url: String::new(),
            jwks_cache_ttl: 300,
            client_id: String::new(),
            client_secret: String::new(),
            redirect_uri: "http://localhost:4002/oauth2/callback".to_string(),
            post_logout_redirect_uri: "http://localhost:4002/".to_string(),
            leeway_seconds: 30,
            lifecycle_jti_cache_ttl: 300,
            session_ttl_seconds: 1_209_600,
        },
        jwt_secret: "test-jwt-secret".to_string(),
        jwt_expiry_hours: 1,
        refresh_token_expiry_days: 7,
        account_lockout_attempts: 5,
        account_lockout_duration_minutes: 30,
        allow_registration: true,
        mail: MailConfig {
            login_approval_enabled: true,
            ..MailConfig::default()
        },
        trusted_proxy_cidrs: Vec::new(),
    }
}

/// The `/api` router over a real pool, as `main.rs` mounts it.
pub fn api_router(pool: PgPool, config: Config) -> axum::Router {
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

/// A standalone-mode session for `user`, as the login route issues it.
pub fn bearer(config: &Config, user: &User) -> String {
    let token = rusty_links::auth::jwt::create_jwt(
        &user.email,
        user.id,
        user.is_admin,
        &config.jwt_secret,
        config.jwt_expiry_hours,
    )
    .expect("test JWT must encode");
    format!("Bearer {token}")
}
