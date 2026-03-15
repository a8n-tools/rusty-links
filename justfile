# Rusty Links - Task Runner

# List available recipes
default:
    @just --list

# Ensure .env exists (mode: standalone or saas)
[private]
ensure-env mode="standalone":
    @test -f .env || cp .env.{{ mode }} .env

# Install JS dependencies
[private]
ensure-npm:
    @test -d node_modules || bun install

# Build Tailwind CSS once
css-build: ensure-npm
    bun x @tailwindcss/cli --input tailwind.css --output assets/tailwind.css

# Watch and rebuild Tailwind CSS on changes
css-watch: ensure-npm
    bun x @tailwindcss/cli --input tailwind.css --output assets/tailwind.css --watch

# Start development server in Docker (mode: standalone or saas)
dev mode="standalone": (ensure-env mode) css-build
    docker compose up --build --remove-orphans app

# Start PostgreSQL container
db-up:
    docker compose up --detach postgres

# Stop PostgreSQL container
db-down:
    docker compose down postgres

# Stop all containers
down:
    docker compose down

# Remove all containers, volumes, and networks
clean:
    #!/usr/bin/env nu
    docker compose down --remove-orphans
    let suffix = $env.USER
    let vols = [
        $"rusty-links-cargo-($suffix)"
        $"rusty-links-target-($suffix)"
        $"rusty-links-postgres-($suffix)"
        $"rusty-links-app-data-($suffix)"
        $"rusty-links-db-data-($suffix)"
    ]
    let existing = docker volume ls --quiet | lines
    for vol in $vols {
        if $vol in $existing {
            docker volume rm $vol
        }
    }

# Run pending database migrations
migrate-run:
    sqlx migrate run

# Create a new database migration
migrate name:
    sqlx migrate add {{ name }}

# Prepare SQLx offline query data
db-prepare:
    cargo sqlx prepare

# Run all checks (web, clippy, fmt)
check: check-web check-clippy check-fmt

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
    docker buildx build --build-arg BUILD_MODE={{ mode }} --tag rusty-links:check -f oci-build/Dockerfile .

# Build release binary
build:
    cargo build --release

# Build Docker image (mode: standalone or saas)
build-docker mode="standalone":
    docker buildx build --build-arg BUILD_MODE={{ mode }} --tag rusty-links:local -f oci-build/Dockerfile .

# Run tests
test:
    cargo test

# Run integration tests against a running instance
test-integration url="http://localhost:4002":
    bash scripts/integration-test.sh {{ url }}

# Format code
fmt:
    cargo fmt
