use rusty_links::ui::app::App;

#[cfg(feature = "server")]
use dioxus::prelude::*;
#[cfg(feature = "server")]
use dioxus::server::{DioxusRouterExt, ServeConfig};
#[cfg(feature = "server")]
use axum::response::IntoResponse;
#[cfg(feature = "server")]
use rusty_links::{api, config, error::AppError, scheduler};
#[cfg(feature = "server")]
use sqlx::{postgres::PgPoolOptions, PgPool};
#[cfg(feature = "server")]
use std::time::Duration;
#[cfg(feature = "server")]
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(feature = "server")]
async fn initialize_database(database_url: &str) -> Result<PgPool, AppError> {
    tracing::info!("Initializing database connection pool...");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(600))
        .connect(database_url)
        .await?;

    tracing::info!("Running database migrations...");
    sqlx::migrate!("./migrations").run(&pool).await?;

    tracing::info!("Database initialized successfully");
    Ok(pool)
}

#[cfg(feature = "server")]
#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Rusty Links (Fullstack) starting...");

    let config = config::Config::from_env().expect("Failed to load configuration");

    tracing::info!(
        database_url = %config.masked_database_url(),
        app_port = config.app_port,
        "Configuration loaded"
    );

    let pool = match initialize_database(&config.database_url).await {
        Ok(pool) => pool,
        Err(e) => {
            tracing::error!("Failed to connect to database");
            tracing::error!("");
            tracing::error!("Error: {}", e);
            tracing::error!("");
            tracing::error!("Please check:");
            tracing::error!("  1. PostgreSQL is running");
            tracing::error!(
                "  2. DATABASE_URL in .env is correct: {}",
                config.masked_database_url()
            );
            tracing::error!("  3. The database exists and is accessible");
            tracing::error!("");
            tracing::error!("To create the database, run:");
            tracing::error!("  createdb rusty_links");
            std::process::exit(1);
        }
    };

    rusty_links::server_functions::auth::set_db_pool(pool.clone());

    // Create default admin from environment variables if no users exist (standalone only)
    #[cfg(feature = "standalone")]
    {
        if let (Ok(email), Ok(password)) = (
            std::env::var("SETUP_DEFAULT_ADMIN_EMAIL"),
            std::env::var("SETUP_DEFAULT_ADMIN_PASSWORD"),
        ) {
            let name = std::env::var("SETUP_DEFAULT_ADMIN_NAME")
                .unwrap_or_else(|_| "Admin".to_string());

            match rusty_links::models::check_user_exists(&pool).await {
                Ok(false) => {
                    tracing::info!(
                        email = %email,
                        "Creating default admin from SETUP_DEFAULT_ADMIN_* environment variables"
                    );
                    match rusty_links::models::User::create(&pool, &email, &password, &name).await {
                        Ok(user) => {
                            tracing::info!(
                                user_id = %user.id,
                                email = %user.email,
                                is_admin = user.is_admin,
                                "Default admin created successfully"
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                error = %e,
                                "Failed to create default admin"
                            );
                        }
                    }
                }
                Ok(true) => {
                    tracing::debug!("Skipping default admin creation: users already exist");
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        "Failed to check user existence for default admin setup"
                    );
                }
            }
        }
    }

    let scheduler_instance = scheduler::Scheduler::new(pool.clone(), config.clone());
    let scheduler_shutdown = scheduler_instance.shutdown_handle();
    let _scheduler_handle = scheduler_instance.start();

    tracing::info!("Background scheduler started");

    // Create maintenance state (SaaS mode only)
    #[cfg(feature = "saas")]
    let maintenance_mode = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    #[cfg(feature = "saas")]
    let maintenance_message: std::sync::Arc<std::sync::RwLock<Option<String>>> =
        std::sync::Arc::new(std::sync::RwLock::new(None));

    // Build OIDC verifier (saas mode only)
    #[cfg(feature = "saas")]
    let oidc_verifier = std::sync::Arc::new(
        rusty_links::auth::oidc_rs::OidcVerifier::new(config.oidc.clone())
    );

    let api_router = api::create_router(
        pool.clone(),
        config.clone(),
        scheduler_shutdown,
        #[cfg(feature = "saas")]
        maintenance_mode.clone(),
        #[cfg(feature = "saas")]
        maintenance_message.clone(),
        #[cfg(feature = "saas")]
        oidc_verifier.clone(),
    );

    let ip = dioxus::cli_config::server_ip()
        .or_else(|| {
            std::env::var("HOST_IP")
                .ok()
                .and_then(|v| v.parse().ok())
        })
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));
    let port = dioxus::cli_config::server_port().unwrap_or(config.app_port);
    let address = std::net::SocketAddr::new(ip, port);

    tracing::info!(
        "Starting Dioxus fullstack server with API routes at {}",
        address
    );

    let dioxus_router = axum::Router::new().serve_dioxus_application(ServeConfig::new(), App);

    // In SaaS mode, protect page routes: check `rl_session` cookie and redirect
    // unauthenticated users to the OIDC login handler.
    #[cfg(feature = "saas")]
    let dioxus_router = {
        let pool = pool.clone();
        dioxus_router.layer(axum::middleware::from_fn(
            move |req: axum::http::Request<axum::body::Body>,
                  next: axum::middleware::Next| {
                let pool = pool.clone();
                async move {
                    let path = req.uri().path();

                    // OIDC flow paths handle their own auth — never gate them.
                    if path.starts_with("/oauth2/") {
                        return next.run(req).await;
                    }

                    let is_protected = matches!(
                        path,
                        "/" | "/links" | "/categories" | "/tags" | "/languages" | "/licenses" | "/login"
                    ) || path.starts_with("/links/");

                    if !is_protected {
                        return next.run(req).await;
                    }

                    // Extract the rl_session cookie.
                    let jar = axum_extra::extract::CookieJar::from_headers(req.headers());
                    let session_valid = if let Some(cookie) = jar.get("rl_session") {
                        rusty_links::auth::oidc_rp::get_user_from_session(&pool, cookie.value())
                            .await
                            .unwrap_or(None)
                            .is_some()
                    } else {
                        false
                    };

                    if session_valid {
                        if path == "/login" {
                            return axum::response::Redirect::to("/links").into_response();
                        }
                        return next.run(req).await;
                    }

                    // Unauthenticated — redirect to the BFF login handler.
                    let return_to = if path == "/login" { "/links" } else { path };
                    let redirect_url = format!(
                        "/oauth2/login?return_to={}",
                        urlencoding::encode(return_to)
                    );
                    axum::response::Redirect::to(&redirect_url).into_response()
                }
            },
        ))
    };

    // OIDC RP router (registered at root, not under /api)
    #[cfg(feature = "saas")]
    let oidc_router = rusty_links::auth::oidc_rp::create_router(
        pool.clone(),
        config.oidc.clone(),
        oidc_verifier.clone(),
    );

    let mut router = axum::Router::new().nest("/api", api_router);

    // Merge the OIDC RP routes at root level (before dioxus router)
    #[cfg(feature = "saas")]
    {
        router = router.merge(oidc_router);
    }

    let mut router = router
        .route_service(
            "/tailwind.css",
            tower::util::service_fn(|_req: axum::http::Request<axum::body::Body>| async {
                let css = include_str!("../assets/tailwind.css");
                Ok::<_, std::convert::Infallible>(
                    axum::response::Response::builder()
                        .header("Content-Type", "text/css")
                        .body(axum::body::Body::from(css.to_string()))
                        .unwrap(),
                )
            }),
        );

    // Landing page — served before the Dioxus router so it takes precedence at "/"
    #[cfg(feature = "standalone")]
    let mut router = router.route_service(
        "/",
        tower::util::service_fn(|_req: axum::http::Request<axum::body::Body>| async {
            let html = include_str!("../static/index.html");
            Ok::<_, std::convert::Infallible>(
                axum::response::Response::builder()
                    .header("Content-Type", "text/html; charset=utf-8")
                    .body(axum::body::Body::from(html))
                    .unwrap(),
            )
        }),
    );

    let mut router = router.merge(dioxus_router);

    // Maintenance mode guard (outermost middleware, saas only).
    #[cfg(feature = "saas")]
    {
        let mm = maintenance_mode.clone();
        let mm_msg = maintenance_message.clone();
        let pool_mm = pool.clone();
        router = router.layer(axum::middleware::from_fn(
            move |req: axum::http::Request<axum::body::Body>,
                  next: axum::middleware::Next| {
                let mm = mm.clone();
                let mm_msg = mm_msg.clone();
                let pool_mm = pool_mm.clone();
                async move {
                    if !mm.load(std::sync::atomic::Ordering::SeqCst) {
                        return next.run(req).await;
                    }

                    let path = req.uri().path();

                    if path.starts_with("/api/health")
                        || path.starts_with("/oauth2/")
                        || path == "/tailwind.css"
                        || path.starts_with("/assets/")
                    {
                        return next.run(req).await;
                    }

                    // Check if the session belongs to an admin — admins bypass maintenance.
                    let jar = axum_extra::extract::CookieJar::from_headers(req.headers());
                    if let Some(cookie) = jar.get("rl_session") {
                        if let Ok(Some((user_id, _))) = rusty_links::auth::oidc_rp::get_user_from_session(
                            &pool_mm,
                            cookie.value(),
                        )
                        .await
                        {
                            let is_admin = sqlx::query_as::<_, (bool,)>(
                                "SELECT is_admin FROM users WHERE id = $1",
                            )
                            .bind(user_id)
                            .fetch_optional(&pool_mm)
                            .await
                            .unwrap_or(None)
                            .map(|(v,)| v)
                            .unwrap_or(false);

                            if is_admin {
                                return next.run(req).await;
                            }
                        }
                    }

                    let message = mm_msg
                        .read()
                        .unwrap()
                        .clone()
                        .unwrap_or_default();

                    if path.starts_with("/api/") {
                        return (
                            axum::http::StatusCode::SERVICE_UNAVAILABLE,
                            axum::Json(serde_json::json!({
                                "error": "Service under maintenance",
                                "maintenance": true,
                                "message": message,
                            })),
                        )
                            .into_response();
                    }

                    let html = include_str!("maintenance.html")
                        .replace("{{MAINTENANCE_MESSAGE}}", &message);
                    (
                        axum::http::StatusCode::SERVICE_UNAVAILABLE,
                        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
                        html,
                    )
                        .into_response()
                }
            },
        ));
    }

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("Failed to bind to address");

    tracing::info!("Server listening on {}", address);

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .expect("Server error");
}

#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(App);
}
