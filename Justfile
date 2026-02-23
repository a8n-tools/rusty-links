# List available recipes
default:
    @just --list

# Run all checks (server, web, clippy, fmt)
check: check-server check-web check-clippy check-fmt

# Check server compilation (standalone + saas)
check-server: check-server-standalone check-server-saas

# Check standalone server compilation
check-server-standalone:
    cargo check --features standalone,server

# Check saas server compilation
check-server-saas:
    cargo check --no-default-features --features saas,server

# Check web/WASM compilation (standalone + saas)
check-web: check-web-standalone check-web-saas

# Check standalone web/WASM compilation
check-web-standalone:
    cargo check --features standalone,web --target wasm32-unknown-unknown

# Check saas web/WASM compilation
check-web-saas:
    cargo check --no-default-features --features saas,web --target wasm32-unknown-unknown

# Run clippy lints
check-clippy:
    cargo clippy

# Check formatting
check-fmt:
    cargo fmt --check

# Build Docker image for validation (mode: standalone or saas)
check-docker mode="standalone":
    docker buildx build --build-arg BUILD_MODE={{ mode }} --tag rusty-links:check .

# Build release binary
build:
    cargo build --release

# Build Docker image (mode: standalone or saas)
build-docker mode="standalone":
    docker buildx build --build-arg BUILD_MODE={{ mode }} --tag rusty-links:local .

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
