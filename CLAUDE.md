# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rusty Links is a full-stack Rust web application using Dioxus 0.7 (fullstack mode) for the frontend. It uses PostgreSQL for data storage with SQLx for database access. Tailwind CSS v4 is used for the styling.
IMPORTANT: Do NOT modify `./assets/tailwind.css`. All CSS should go in `./tailwind.css` and `dx` will automatically run the `tailwindcss` cli to generate `./assets/tailwind.css`.

## Build & Development Commands

```bash
# Run in development (requires PostgreSQL and .env file)
dx serve

# Check for compilation errors without building
cargo check

# Check server feature only
cargo check --features server

# Check web/WASM feature only
cargo check --features web --target wasm32-unknown-unknown

# Run tests
cargo test

# Run a specific test
cargo test <test_name>

# Code quality
cargo fmt
cargo clippy

# Database migrations (auto-run on startup, but manual commands available)
sqlx migrate add <migration_name>
sqlx migrate run
```

## Feature Flags

The project uses Cargo features to separate server and client code:
- `server` - Enables Axum, SQLx, Tokio, and server-side modules
- `web` - Enables WASM/browser-specific dependencies (gloo-net, web-sys)

Server-only modules (`#[cfg(feature = "server")]`): api, auth, config, error, github, models, scheduler, scraper

## Architecture

- **Entry point**: `src/main.rs` - Initializes database pool, starts scheduler, creates Axum router with Dioxus frontend and API routes
- **API layer**: `src/api/` - REST endpoints nested under `/api`, with auth routes at `/api/auth/*`
- **Server functions**: `src/server_functions/` - Dioxus server functions bridging client/server communication (available on both sides)
- **Auth**: `src/auth/` - Session-based authentication using cookies, Argon2 password hashing
- **UI**: `src/ui/` - Dioxus components with pages (`pages/`) and reusable components (`components/`)
- **Models**: `src/models/` - Database models (User, Link, Category, Tag, etc.)
- **Scheduler**: `src/scheduler/` - Background task runner for periodic metadata updates
- **Scraper**: `src/scraper/` - HTML metadata extraction (titles, descriptions, logos)
- **GitHub**: `src/github/` - GitHub API integration for repo metadata (stars, languages, licenses)
- **Config**: `src/config.rs` - Environment-based configuration
- **Errors**: `src/error.rs` - Centralized error handling with `AppError` type

## Database

- PostgreSQL with SQLx (compile-time checked queries)
- Migrations in `migrations/` directory, run automatically on startup
- Connection pool: 5 max connections, 30s timeout, 10min idle timeout

## Configuration

Environment variables are defined in feature-specific files:
- `.env.standalone` - Standalone mode (includes JWT auth variables)
- `.env.saas` - SaaS mode (auth handled by parent app cookies)

Copy the appropriate file to `.env` before running:
```bash
cp .env.standalone .env   # or .env.saas
```

IMPORTANT: When adding, removing, or renaming environment variables (in `src/config.rs`, `compose.yml`, or any `std::env::var` call), update both `.env.standalone` and `.env.saas` to keep them in sync.
