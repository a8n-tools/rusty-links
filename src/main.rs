use rusty_links::ui::app::App;

#[cfg(feature = "server")]
use dioxus::prelude::*;
#[cfg(feature = "server")]
use dioxus::server::{DioxusRouterExt, ServeConfig};
#[cfg(feature = "server")]
use rusty_links::{api, config, error::AppError, scheduler};
#[cfg(feature = "server")]
use sqlx::{postgres::PgPoolOptions, PgPool};
#[cfg(feature = "server")]
use std::sync::atomic::AtomicBool;
#[cfg(feature = "server")]
use std::sync::Arc;
#[cfg(feature = "server")]
use std::time::Duration;
#[cfg(feature = "server")]
use tower_http::services::ServeDir;
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
    let scheduler_shutdown = Arc::new(AtomicBool::new(false));
    let scheduler_instance = scheduler::Scheduler::new(pool.clone(), config.clone());
    let _scheduler_handle = scheduler_instance.start();

    tracing::info!("Background scheduler started");

    // Create API router
    let api_router = api::create_router(pool.clone(), config.clone(), scheduler_shutdown);

    // Get the fullstack address from CLI or use localhost
    let address = dioxus::cli_config::fullstack_address_or_localhost();

    tracing::info!(
        "Starting Dioxus fullstack server with API routes at {}",
        address
    );

    // Build the Axum router:
    // 1. First create Dioxus app router (handles server functions, static assets, and rendering)
    // 2. Then nest our custom API routes (they take precedence due to being more specific)
    let dioxus_router = axum::Router::new().serve_dioxus_application(ServeConfig::new(), App);

    // Merge the API router with the Dioxus router
    // API routes under /api will be handled by our custom router
    // Everything else will be handled by Dioxus
    // Also serve static assets from the assets directory
    let router = axum::Router::new()
        .nest("/api", api_router)
        .nest_service("/assets", ServeDir::new("assets"))
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

    // Launch the server
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("Failed to bind to address");

    tracing::info!("Server listening on {}", address);

    axum::serve(listener, router).await.expect("Server error");
}

#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(App);
}
