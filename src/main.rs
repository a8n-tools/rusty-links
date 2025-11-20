mod config;

use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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
/// Returns an error if:
/// - Database connection fails
/// - Migration execution fails
/// - Health check query fails
async fn initialize_database(database_url: &str) -> Result<PgPool, Box<dyn std::error::Error>> {
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

    // Store pool for application use
    // The pool will be passed to API handlers, background jobs, etc.
    // For now, we keep it in scope to prevent it from being dropped
    tracing::info!(
        "Database connection pool ready for application use"
    );

    // Placeholder for application logic
    // Will be implemented in subsequent steps

    tracing::info!("Application initialization complete. Ready for Step 5.");

    // Keep the application running
    // In future steps, this will be replaced with the Axum server
    // For now, we'll just keep the pool alive and exit
    drop(pool);
}
