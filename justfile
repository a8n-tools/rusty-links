# Rusty Links - Task Runner

# List available recipes
default:
    @just --list

# Use the per-user dev compose file
compose := "docker compose -f compose.dev.yml "

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
    {{ compose }}up --build --remove-orphans app

# Start local development server in Docker — no Traefik, localhost ports (mode: standalone or saas)
dev-local mode="standalone": (ensure-env mode) css-build
    docker compose up --build --remove-orphans app

# Start PostgreSQL container
db-up:
    {{ compose }}up --detach postgres

# Stop PostgreSQL container
db-down:
    {{ compose }}down postgres

# Stop all containers
down:
    {{ compose }}down

# Remove all containers, volumes, and networks
clean:
    #!/usr/bin/env nu
    docker compose -f compose.dev.yml down --volumes --remove-orphans
    docker compose down --volumes --remove-orphans
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

# Release
# Create a release: bump major (vx.0.0) or minor version (v0.x.0), commit, tag, and push
create-release bump:
    #!/usr/bin/env nu
    let bump = "{{ bump }}"
    let current = (open Cargo.toml | get package.version | split row "." | each { into int })
    let next = match $bump {
        "major" => [$"($current.0 + 1)" "0" "0"],
        "minor" => [$"($current.0)" $"($current.1 + 1)" "0"],
        _ => { print $"(ansi red)Usage: just create-release <major|minor>(ansi reset)"; exit 1 }
    }
    let bare = ($next | str join ".")
    let tag = $"v($bare)"
    open Cargo.toml | update package.version $bare | to toml | collect | save --force Cargo.toml
    git add Cargo.toml
    git commit --signoff --message $"Release ($tag)"
    git tag --annotate $tag --message $"Release ($tag)"
    git push --follow-tags
    print $"Released ($tag)"

# Test the release flow: create major release, cancel CI, delete tag, and revert commit (requires FORGEJO_TOKEN)
test-release:
    #!/usr/bin/env nu
    let token = ($env | get --ignore-errors FORGEJO_TOKEN | default "")
    if ($token | is-empty) { print $"(ansi red)FORGEJO_TOKEN env var required(ansi reset)"; exit 1 }
    let current = (open Cargo.toml | get package.version | split row "." | each { into int })
    let bare = $"($current.0 + 1).0.0"
    let tag = $"v($bare)"
    just create-release major
    print "Waiting for CI to pick up the tag..."
    sleep 5sec
    let headers = {Authorization: $"token ($token)"}
    let runs = (http get --headers $headers "https://dev.a8n.run/api/v1/repos/a8n-tools/rusty-links/actions/runs")
    let matched = ($runs.workflow_runs | where prettyref == $tag)
    if ($matched | is-empty) {
        print $"(ansi yellow)No workflow run found for ($tag) — skipping cancel(ansi reset)"
    } else {
        let run_id = ($matched | first | get id)
        try {
            http post --headers $headers --content-type "application/json" $"https://dev.a8n.run/api/v1/repos/a8n-tools/rusty-links/actions/runs/($run_id)/cancel" {}
            print $"Cancelled workflow run ($run_id)"
        } catch {
            print $"(ansi yellow)Could not cancel run ($run_id) — may have already completed(ansi reset)"
        }
    }
    ^git tag --delete $tag
    ^git push origin --delete $tag
    ^git revert --no-edit HEAD
    ^git push
    print $"Done — ($tag) cleaned up"
