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
    ]
    let existing = docker volume ls --quiet | lines
    for vol in $vols {
        if $vol in $existing {
            # Force-remove any containers still holding this volume
            let holders = docker ps -aq --filter $"volume=($vol)" | lines
            for c in $holders {
                docker rm -f $c
            }
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

# ── Release ──────────────────────────────────────────────────────────────────

# Create a release: bump major (vx.0.0), minor (v0.x.0), or hotfix (v0.0.x), push branch, and print PR link
# After the PR is merged, the create-release workflow creates the tag and release automatically
create-release bump:
    #!/usr/bin/env nu
    let bump = "{{ bump }}"

    # Abort if there are uncommitted changes
    let status = git status --porcelain | str trim
    if ($status | is-not-empty) {
        print $"(ansi red)Working tree is dirty. Please stash or commit your changes first.(ansi reset)"
        exit 1
    }

    # Switch to main if not already there
    let branch = git branch --show-current | str trim
    if $branch != "main" {
        print $"Switching from ($branch) to main..."
        git checkout main
    }

    # Pull latest changes
    git pull --rebase origin main

	# Calculate next version
    let current = (open Cargo.toml | get package.version | split row "." | each { into int })
    let next = match $bump {
        "major" => [$"($current.0 + 1)" "0" "0"],
        "minor" => [$"($current.0)" $"($current.1 + 1)" "0"],
        "hotfix" => [$"($current.0)" $"($current.1)" $"($current.2 + 1)"],
        _ => { print $"(ansi red)Usage: just create-release <major|minor|hotfix>(ansi reset)"; exit 1 }
    }
    let bare = ($next | str join ".")
    let tag = $"v($bare)"
    let release_branch = $"release/($tag)"

    # Create release branch, bump version, and commit
    git checkout -b $release_branch
    open Cargo.toml | update package.version $bare | to toml | collect | save --force Cargo.toml
    git add Cargo.toml
    git commit --signoff --message $"Release ($tag)"

    # Push release branch
    git push --set-upstream origin $release_branch

    # Print PR and release links
    let remote = git remote get-url origin
    let base_url = if ($remote | str starts-with "ssh://") {
        $remote | str replace "ssh://git@" "https://" | str replace "git.a8n.run" "dev.a8n.run" | str replace ".git" ""
    } else {
        $remote | str replace --regex "git@([^:]+):" "https://$1/" | str replace "git.a8n.run" "dev.a8n.run" | str replace ".git" ""
    }
    print $"(ansi green)Pushed ($release_branch)(ansi reset)"
    print $"Create PR: ($base_url)/compare/main...($release_branch)"
    print $"After merging, the create-release workflow will tag and release ($tag) automatically."

