mod config;

use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

    // Connect to database
    tracing::info!("Connecting to database...");
    let pool = match PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
    {
        Ok(pool) => {
            tracing::info!("Database connection established");
            pool
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to connect to database");
            eprintln!("Database connection error: {}", e);
            eprintln!("\nPlease ensure:");
            eprintln!("  1. PostgreSQL is running");
            eprintln!("  2. DATABASE_URL is correct in .env");
            eprintln!("  3. Database exists and is accessible");
            std::process::exit(1);
        }
    };

    // Run migrations automatically
    // Migrations are located in the migrations/ directory
    // To create new migrations:
    //   1. Install sqlx-cli: cargo install sqlx-cli --no-default-features --features postgres
    //   2. Create migration: sqlx migrate add <name>
    //   3. Edit the generated SQL file in migrations/
    //   4. Migrations run automatically on application startup
    tracing::info!("Running database migrations...");
    if let Err(e) = sqlx::migrate!("./migrations").run(&pool).await {
        tracing::error!(error = %e, "Failed to run database migrations");
        eprintln!("Migration error: {}", e);
        eprintln!("\nPlease check:");
        eprintln!("  1. Migration files in migrations/ directory");
        eprintln!("  2. Database user has necessary permissions");
        eprintln!("  3. Migration syntax is correct");
        std::process::exit(1);
    }
    tracing::info!("Database migrations completed successfully");

    // Placeholder for application logic
    // Will be implemented in subsequent steps

    tracing::info!("Database schema initialized. Ready for Step 4.");
}
