# List available recipes
default:
    @just --list

# Run all checks (server, web, clippy, fmt)
check: check-server check-web check-clippy check-fmt

# Check server compilation
check-server:
    cargo check --features server

# Check web/WASM compilation
check-web:
    cargo check --features web --target wasm32-unknown-unknown

# Run clippy lints
check-clippy:
    cargo clippy

# Check formatting
check-fmt:
    cargo fmt --check

# Build Docker image for validation
check-docker:
    docker buildx build --tag rusty-links:check .

# Build release binary
build:
    cargo build --release

# Build Docker image
build-docker:
    docker buildx build --tag rusty-links:local .

# Start development server
dev:
    dx serve

# Run tests
test:
    cargo test

# Format code
fmt:
    cargo fmt

# Create a new database migration
migrate name:
    sqlx migrate add {{ name }}
