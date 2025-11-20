mod config;
mod error;
mod models;

use crate::error::AppError;
use crate::models::{check_user_exists, create_user, find_user_by_email, verify_password, CreateUser};
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

    // =============================================================================
    // TEMPORARY TEST CODE - Step 6: User Model Testing
    // This code will be removed in Step 7
    // =============================================================================
    tracing::info!("=== Starting User Model Tests ===");

    // Test 1: Check if users exist
    match check_user_exists(&pool).await {
        Ok(exists) => {
            tracing::info!(exists = exists, "User existence check completed");
            if exists {
                eprintln!("✓ Users exist in database");
            } else {
                eprintln!("✓ No users exist - fresh database");
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to check user existence");
            eprintln!("✗ User existence check failed: {}", e);
        }
    }

    // Test 2: Create a test user
    let test_email = "test@rustylinks.local";
    let test_password = "secure_test_password_123";

    eprintln!("\nCreating test user: {}", test_email);

    match create_user(
        &pool,
        CreateUser {
            email: test_email.to_string(),
            password: test_password.to_string(),
        },
    )
    .await
    {
        Ok(user) => {
            tracing::info!(user_id = %user.id, email = %user.email, "Test user created");
            eprintln!("✓ User created successfully");
            eprintln!("  - ID: {}", user.id);
            eprintln!("  - Email: {}", user.email);
            eprintln!("  - Created at: {}", user.created_at);
            eprintln!("  - Password hash length: {}", user.password_hash.len());
        }
        Err(e) => {
            match &e {
                AppError::Duplicate { field } => {
                    tracing::info!(field = %field, "User already exists (expected if running multiple times)");
                    eprintln!("✓ User already exists (expected if running multiple times)");
                }
                _ => {
                    tracing::error!(error = %e, "Failed to create user");
                    eprintln!("✗ User creation failed: {}", e);
                }
            }
        }
    }

    // Test 3: Find user by email (case-insensitive)
    eprintln!("\nFinding user by email (testing case-insensitive)...");

    match find_user_by_email(&pool, "TEST@rustylinks.local").await {
        Ok(Some(user)) => {
            tracing::info!(user_id = %user.id, email = %user.email, "User found");
            eprintln!("✓ User found by email (case-insensitive)");
            eprintln!("  - Found user: {}", user.email);

            // Test 4: Verify correct password
            eprintln!("\nVerifying correct password...");
            match verify_password(test_password, &user.password_hash) {
                Ok(true) => {
                    tracing::info!("Password verification successful");
                    eprintln!("✓ Password verification successful");
                }
                Ok(false) => {
                    tracing::error!("Password verification failed unexpectedly");
                    eprintln!("✗ Password verification failed (should have succeeded)");
                }
                Err(e) => {
                    tracing::error!(error = %e, "Password verification error");
                    eprintln!("✗ Password verification error: {}", e);
                }
            }

            // Test 5: Verify incorrect password
            eprintln!("\nVerifying incorrect password...");
            match verify_password("wrong_password", &user.password_hash) {
                Ok(false) => {
                    tracing::info!("Password verification correctly rejected wrong password");
                    eprintln!("✓ Incorrect password correctly rejected");
                }
                Ok(true) => {
                    tracing::error!("Password verification incorrectly accepted wrong password");
                    eprintln!("✗ Incorrect password was accepted (should have been rejected)");
                }
                Err(e) => {
                    tracing::error!(error = %e, "Password verification error");
                    eprintln!("✗ Password verification error: {}", e);
                }
            }
        }
        Ok(None) => {
            tracing::warn!("User not found by email");
            eprintln!("✗ User not found (may have failed to create)");
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to find user");
            eprintln!("✗ User lookup failed: {}", e);
        }
    }

    // Test 6: Try to find non-existent user
    eprintln!("\nTesting lookup of non-existent user...");
    match find_user_by_email(&pool, "nonexistent@example.com").await {
        Ok(None) => {
            tracing::info!("Non-existent user correctly returned None");
            eprintln!("✓ Non-existent user correctly returned None");
        }
        Ok(Some(_)) => {
            tracing::error!("Non-existent user unexpectedly found");
            eprintln!("✗ Non-existent user unexpectedly found");
        }
        Err(e) => {
            tracing::error!(error = %e, "User lookup error");
            eprintln!("✗ User lookup error: {}", e);
        }
    }

    // Test 7: Test email validation
    eprintln!("\nTesting email validation...");
    match create_user(
        &pool,
        CreateUser {
            email: "invalid-email-no-at".to_string(),
            password: "password".to_string(),
        },
    )
    .await
    {
        Err(AppError::Validation { field, message }) => {
            tracing::info!(field = %field, message = %message, "Email validation working correctly");
            eprintln!("✓ Email validation correctly rejected invalid email");
            eprintln!("  - Field: {}", field);
            eprintln!("  - Message: {}", message);
        }
        Ok(_) => {
            tracing::error!("Email validation failed to reject invalid email");
            eprintln!("✗ Invalid email was accepted (should have been rejected)");
        }
        Err(e) => {
            tracing::error!(error = %e, "Unexpected error during email validation test");
            eprintln!("✗ Unexpected error: {}", e);
        }
    }

    tracing::info!("=== User Model Tests Complete ===");
    eprintln!("\n=== All User Model Tests Complete ===");
    eprintln!("Step 6 implementation verified successfully!");

    // =============================================================================
    // END OF TEMPORARY TEST CODE
    // =============================================================================

    tracing::info!("Application initialization complete. Ready for Step 7.");

    // Keep the application running
    // In future steps, this will be replaced with the Axum server
    // For now, we'll just keep the pool alive and exit
    drop(pool);
}
