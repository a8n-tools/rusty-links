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
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Rusty Links (Fullstack) starting...");

    // Load configuration
    let config = config::Config::from_env().expect("Failed to load configuration");

    tracing::info!(
        database_url = %config.masked_database_url(),
        app_port = config.app_port,
        "Configuration loaded"
    );

    // Initialize database
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

    // Initialize global database pool for server functions
    rusty_links::server_functions::auth::set_db_pool(pool.clone());

    // Start background scheduler
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

    // Create API router
    let api_router = api::create_router(
        pool.clone(),
        config.clone(),
        scheduler_shutdown,
        #[cfg(feature = "saas")]
        maintenance_mode.clone(),
        #[cfg(feature = "saas")]
        maintenance_message.clone(),
    );

    // Build bind address from Dioxus CLI config (set by `dx serve`) with fallbacks:
    //   IP   → CLI value, else HOST_IP env, else 0.0.0.0  (not 127.0.0.1, which is unreachable inside Docker)
    //   Port → CLI value, else APP_PORT from config
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

    // Build the Axum router:
    // 1. First create Dioxus app router (handles server functions, static assets, and rendering)
    // 2. Then nest our custom API routes (they take precedence due to being more specific)
    let dioxus_router = axum::Router::new().serve_dioxus_application(ServeConfig::new(), App);

    // In SaaS mode, wrap the Dioxus router with auth-check middleware that redirects
    // unauthenticated users to the parent platform's login page.
    #[cfg(feature = "saas")]
    let dioxus_router = {
        let saas_login_url = config.saas_login_url.clone();
        let host_url = config.host_url.clone();
        let saas_jwt_secret = config.saas_jwt_secret.clone();
        let saas_logout_url = config.saas_logout_url.clone();
        let saas_membership_url = config.saas_membership_url.clone();
        dioxus_router.layer(axum::middleware::from_fn(
            move |jar: axum_extra::extract::CookieJar,
                  req: axum::http::Request<axum::body::Body>,
                  next: axum::middleware::Next| {
                let saas_login_url = saas_login_url.clone();
                let host_url = host_url.clone();
                let saas_jwt_secret = saas_jwt_secret.clone();
                let saas_logout_url = saas_logout_url.clone();
                let saas_membership_url = saas_membership_url.clone();
                async move {
                    let path = req.uri().path();

                    // Handle /logout — redirect to SaaS API logout endpoint
                    if path == "/logout" {
                        let return_to = format!("{}/links", host_url.trim_end_matches('/'));
                        let redirect_url = format!(
                            "{}?url={}",
                            saas_logout_url.trim_end_matches('/'),
                            urlencoding::encode(&return_to)
                        );
                        return axum::response::Redirect::to(&redirect_url).into_response();
                    }

                    // Only protect app pages — skip API, assets, framework routes, and refresh script
                    if path == "/saas-refresh.js" {
                        return next.run(req).await;
                    }

                    let is_protected = matches!(
                        path,
                        "/" | "/links" | "/categories" | "/tags" | "/languages" | "/licenses" | "/login"
                    ) || path.starts_with("/links/");

                    if !is_protected {
                        return next.run(req).await;
                    }

                    // Check access_token cookie
                    match rusty_links::auth::saas_auth::get_user_from_cookie(&jar, &saas_jwt_secret) {
                        Some(claims) => {
                            // Non-admin users must have active or grace_period membership
                            if !claims.is_admin {
                                let has_access = matches!(
                                    claims.membership_status.as_deref(),
                                    Some("active") | Some("grace_period")
                                );
                                if !has_access {
                                    return axum::response::Redirect::to(&saas_membership_url).into_response();
                                }
                            }

                            // Authenticated member hitting /login — send them to links instead
                            if path == "/login" {
                                return axum::response::Redirect::to("/links").into_response();
                            }
                            next.run(req).await
                        }
                        None => {
                            // Not authenticated — redirect to SaaS login
                            // Use /links as the default return page (not /login)
                            let return_path = if path == "/login" { "/links" } else { path };
                            let return_to = format!("{}{}", host_url.trim_end_matches('/'), return_path);
                            let redirect_url = format!(
                                "{}?redirect={}",
                                saas_login_url.trim_end_matches('/'),
                                urlencoding::encode(&return_to)
                            );
                            axum::response::Redirect::to(&redirect_url).into_response()
                        }
                    }
                }
            },
        ))
    };

    // Merge the API router with the Dioxus router
    // API routes under /api will be handled by our custom router
    // Everything else will be handled by Dioxus
    // Also serve static assets from the assets directory
    let mut router = axum::Router::new()
        .nest("/api", api_router);

    // SaaS refresh JS: bake the refresh URL into the script at serve time
    #[cfg(feature = "saas")]
    {
        let js = {
            let refresh_url = config.saas_refresh_url.clone();
            include_str!("../assets/saas-refresh.js").replace("{{SAAS_REFRESH_URL}}", &refresh_url)
        };
        router = router.route_service(
            "/saas-refresh.js",
            tower::util::service_fn(move |_req: axum::http::Request<axum::body::Body>| {
                let js = js.clone();
                async move {
                    Ok::<_, std::convert::Infallible>(
                        axum::response::Response::builder()
                            .header("Content-Type", "application/javascript; charset=utf-8")
                            .body(axum::body::Body::from(js))
                            .unwrap(),
                    )
                }
            }),
        );
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
        )
        .merge(dioxus_router);

    // In SaaS mode, add maintenance guard as the outermost middleware.
    // When maintenance is active, only admins and allowlisted paths pass through.
    #[cfg(feature = "saas")]
    {
        let mm = maintenance_mode.clone();
        let mm_msg = maintenance_message.clone();
        let jwt_secret = config.saas_jwt_secret.clone();
        router = router.layer(axum::middleware::from_fn(
            move |jar: axum_extra::extract::CookieJar,
                  req: axum::http::Request<axum::body::Body>,
                  next: axum::middleware::Next| {
                let mm = mm.clone();
                let mm_msg = mm_msg.clone();
                let jwt_secret = jwt_secret.clone();
                async move {
                    // Fast path: maintenance off
                    if !mm.load(std::sync::atomic::Ordering::SeqCst) {
                        return next.run(req).await;
                    }

                    let path = req.uri().path();

                    // Allowlisted paths always pass through
                    if path.starts_with("/api/webhooks/")
                        || path.starts_with("/api/health")
                        || path == "/tailwind.css"
                        || path == "/saas-refresh.js"
                        || path.starts_with("/assets/")
                    {
                        return next.run(req).await;
                    }

                    // Admins bypass maintenance
                    if let Some(claims) =
                        rusty_links::auth::saas_auth::get_user_from_cookie(&jar, &jwt_secret)
                    {
                        if claims.is_admin {
                            return next.run(req).await;
                        }
                    }

                    let message = mm_msg
                        .read()
                        .unwrap()
                        .clone()
                        .unwrap_or_default();

                    // API routes get JSON 503
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

                    // Page routes get HTML 503
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

    // Launch the server
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
