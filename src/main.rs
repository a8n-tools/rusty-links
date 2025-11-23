// Import modules from the library
#[cfg(feature = "server")]
use rusty_links::{api, config, error, scheduler};

#[cfg(feature = "server")]
use error::AppError;
#[cfg(feature = "server")]
use axum::{response::IntoResponse, routing::get, Router};
#[cfg(feature = "server")]
use sqlx::{postgres::PgPoolOptions, PgPool};
#[cfg(feature = "server")]
use std::time::Duration;
#[cfg(feature = "server")]
use tower_http::{cors::CorsLayer, trace::TraceLayer};
#[cfg(feature = "server")]
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(feature = "server")]
/// Serve the index.html page
async fn serve_index() -> impl IntoResponse {
    axum::response::Html(include_str!("../assets/index.html"))
}

#[cfg(feature = "server")]
/// Serve the CSS file
async fn serve_css() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "text/css")],
        include_str!("../assets/style.css"),
    )
}

#[cfg(feature = "server")]
/// Initialize the database connection pool and run migrations.
///
/// This function:
/// - Creates a PostgreSQL connection pool with optimized settings for single-user applications
/// - Runs all pending database migrations automatically
/// - Performs a health check to verify the connection is working
///
/// # Configuration
/// - Max connections: 5 (appropriate for single-user application)
/// - Connection timeout: 30 seconds
/// - Idle timeout: 10 minutes (connections closed after 10 minutes of inactivity)
///
/// # Migrations
/// Migrations are located in the migrations/ directory and run automatically on startup.
/// To create new migrations:
/// 1. Install sqlx-cli: `cargo install sqlx-cli --no-default-features --features postgres`
/// 2. Create migration: `sqlx migrate add <migration_name>`
/// 3. Edit the generated SQL file in migrations/
/// 4. Migrations will run automatically on next application startup
///
/// # Errors
/// Returns `AppError` if:
/// - Database connection fails (AppError::Database)
/// - Migration execution fails (AppError::Database)
/// - Health check query fails (AppError::Database)
async fn initialize_database(database_url: &str) -> Result<PgPool, AppError> {
    tracing::info!("Initializing database connection pool...");

    // Create connection pool with optimized settings
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(600)) // 10 minutes
        .connect(database_url)
        .await?;

    tracing::info!(
        max_connections = 5,
        acquire_timeout_secs = 30,
        idle_timeout_secs = 600,
        "Database connection pool created"
    );

    // Run pending migrations
    tracing::info!("Running database migrations...");
    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("Database migrations completed successfully");

    // Health check: verify database connection is working
    tracing::info!("Performing database health check...");
    let result: (i32,) = sqlx::query_as("SELECT 1")
        .fetch_one(&pool)
        .await?;

    tracing::info!(
        health_check_result = result.0,
        "Database health check passed"
    );

    tracing::info!("Database initialized successfully");
    Ok(pool)
}

#[cfg(feature = "server")]
#[tokio::main]
async fn main() {
    // Initialize tracing with JSON formatting
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    tracing::info!("Rusty Links starting...");

    // Load configuration
    let config = match config::Config::from_env() {
        Ok(cfg) => cfg,
        Err(e) => {
            tracing::error!(error = %e, "Failed to load configuration");
            eprintln!("Configuration error: {}", e);
            eprintln!("\nPlease ensure all required environment variables are set.");
            eprintln!("See .env.example for reference.");
            std::process::exit(1);
        }
    };

    // Log configuration (with sensitive data masked)
    tracing::info!(
        database_url = %config.masked_database_url(),
        app_port = config.app_port,
        update_interval_days = config.update_interval_days,
        update_interval_hours = config.update_interval_hours,
        batch_size = config.batch_size,
        jitter_percent = config.jitter_percent,
        log_level = %config.log_level,
        "Configuration loaded successfully"
    );

    // Initialize database connection pool
    let pool = match initialize_database(&config.database_url).await {
        Ok(pool) => {
            tracing::info!("Database connected successfully");
            pool
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to initialize database");
            eprintln!("Database initialization error: {}", e);
            eprintln!("\nPlease ensure:");
            eprintln!("  1. PostgreSQL is running");
            eprintln!("  2. DATABASE_URL is correct in .env");
            eprintln!("  3. Database exists and is accessible");
            eprintln!("  4. Database user has necessary permissions (CREATE, INSERT, etc.)");
            std::process::exit(1);
        }
    };

    // Start background scheduler
    tracing::info!("Starting background scheduler...");
    let scheduler = scheduler::Scheduler::new(pool.clone(), config.clone());
    let scheduler_shutdown = scheduler.shutdown_handle();
    let _scheduler_handle = scheduler.start();
    tracing::info!(
        update_interval_hours = config.update_interval_hours,
        batch_size = config.batch_size,
        jitter_percent = config.jitter_percent,
        "Background scheduler started successfully"
    );

    // Create API router
    tracing::info!("Creating API router...");
    let api_router = api::create_router(pool.clone(), scheduler_shutdown);

    // Build main application with API routes and frontend
    tracing::info!("Configuring application...");
    let app = Router::new()
        .route("/", get(serve_index))
        .route("/assets/style.css", get(serve_css))
        .nest("/api", api_router)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    tracing::info!("Application configured with frontend, API routes, CORS and tracing middleware");

    // Bind to configured port
    let addr = format!("0.0.0.0:{}", config.app_port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| {
            tracing::error!(error = %e, port = config.app_port, "Failed to bind to port");
            eprintln!("Failed to bind to port {}: {}", config.app_port, e);
            eprintln!("\nPlease ensure:");
            eprintln!("  1. Port {} is not already in use", config.app_port);
            eprintln!("  2. You have permission to bind to this port");
            std::process::exit(1);
        });

    tracing::info!(
        port = config.app_port,
        address = %addr,
        "Server listening on port {}",
        config.app_port
    );
    eprintln!("🚀 Server listening on http://{}", addr);
    eprintln!("\nFrontend:");
    eprintln!("  GET    /                     - Main application");
    eprintln!("\nAPI Endpoints:");
    eprintln!("  POST   /api/auth/setup       - Create first user");
    eprintln!("  POST   /api/auth/login       - Login with email/password");
    eprintln!("  POST   /api/auth/logout      - Logout and clear session");
    eprintln!("  GET    /api/auth/me          - Get current user info");
    eprintln!("  GET    /api/auth/check-setup - Check if setup is required");
    eprintln!();

    // Start server
    axum::serve(listener, app)
        .await
        .unwrap_or_else(|e| {
            tracing::error!(error = %e, "Server error");
            eprintln!("Server error: {}", e);
            std::process::exit(1);
        });
}

// Web client entry point
#[cfg(feature = "web")]
fn main() {
    use rusty_links::ui::app::App;
    dioxus::launch(App);
}
