mod config;

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

    // Placeholder for application logic
    // Will be implemented in subsequent steps

    tracing::info!("Configuration validation complete. Ready for Step 3.");
}
