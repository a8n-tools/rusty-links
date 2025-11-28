use rusty_links::ui::app::App;

#[cfg(feature = "server")]
use rusty_links::{config, error::AppError, scheduler};
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

fn main() {
    #[cfg(feature = "server")]
    {
        // Initialize tracing
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "info".into()),
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

        // Start async runtime for database initialization
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pool = rt.block_on(async {
            initialize_database(&config.database_url)
                .await
                .expect("Failed to initialize database")
        });

        // Initialize global database pool for server functions
        rusty_links::server_functions::auth::set_db_pool(pool.clone());

        // Start background scheduler
        let scheduler = scheduler::Scheduler::new(pool.clone(), config.clone());
        let _scheduler_handle = scheduler.start();

        tracing::info!("Background scheduler started");

        // Launch Dioxus fullstack app
        // Note: Port configuration should be done via DIOXUS_PORT env var or Dioxus.toml
        tracing::info!(port = config.app_port, "Starting Dioxus fullstack server");
        dioxus::launch(App);
    }

    #[cfg(not(feature = "server"))]
    {
        dioxus::launch(App);
    }
}
