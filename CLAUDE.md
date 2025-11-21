# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rusty Links is a full-stack Rust web application using Dioxus (fullstack mode) for the frontend and Axum for the backend API. It uses PostgreSQL for data storage with SQLx for database access.

## Build & Development Commands

```bash
# Build the project
cargo build

# Run in development (requires PostgreSQL and .env file)
dx serve

# Check for compilation errors without building
cargo check

# Run tests
cargo test

# Run a specific test
cargo test <test_name>

# Database migrations (auto-run on startup, but manual commands available)
cargo install sqlx-cli --no-default-features --features postgres
sqlx migrate add <migration_name>
sqlx migrate run
```

## Architecture

- **Entry point**: `src/main.rs` - Initializes database pool, creates Axum router with Dioxus frontend and API routes
- **API layer**: `src/api/` - REST endpoints nested under `/api`, with auth routes at `/api/auth/*`
- **Auth**: `src/auth/` - Session-based authentication using cookies, Argon2 password hashing
- **UI**: `src/ui/` - Dioxus components with pages (`pages/`) and reusable components (`components/`)
- **Models**: `src/models/` - Database models (User, etc.)
- **Config**: `src/config.rs` - Environment-based configuration (DATABASE_URL, APP_PORT, etc.)
- **Errors**: `src/error.rs` - Centralized error handling with `AppError` type

## Database

- PostgreSQL with SQLx (compile-time checked queries)
- Migrations in `migrations/` directory, run automatically on startup
- Connection pool: 5 max connections, 30s timeout, 10min idle timeout

## Configuration

Requires `.env` file with:
- `DATABASE_URL` - PostgreSQL connection string
- `APP_PORT` - Server port (default varies)
- `UPDATE_INTERVAL_DAYS` - Update scheduling interval
- `LOG_LEVEL` - Tracing log level
