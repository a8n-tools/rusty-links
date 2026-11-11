# Rusty Links - Task Runner

# SPEC-1.5, SPEC-3.1: app set before the import; any other overridden variable would go here too.
app := "rusty-links"

# LINKS-87: the compose `app` service in compose.dev.yml is this repo's dev
# container; common.just's `pre-commit-compose` runs the fmt leg there.
# `pre_commit_mode` defaults to "compose", repeated here only to pin the
# behavior against a future default change.
pre_commit_mode := "compose"
compose_service := "app"

# LINKS-87: the heavy per-leg matrix (static guards, dependency audit, three
# clippy configurations, two build configurations, library tests, doc tests,
# Postgres integration tests) lives in `check-ci` below. It is NOT wired
# into `pre_push_checks`: STD-4.2 and STD-4.3 forbid compiling code in a
# git hook (including through `pre_push_checks`), and
# `.forgejo/workflows/check.yml` is the authoritative run of that matrix.
# Run `just check-ci` on demand before a push when local confidence beats
# waiting for CI.

# LINKS-87: single-crate layout for common's `create-release` dispatcher.
# `release_crate` defaults to `app` ("rusty-links"), so it is not set.
release_layout := "generic"

# SPEC-1.5, SPEC-3.1: import after the variables above it.
set allow-duplicate-recipes := true

import 'common/common.just'

# SPEC-1.6, SPEC-3.2: default MUST be first and MUST be redefined here, not inherited.
# List available recipes
default:
    @just --list

# LINKS-87: `install-hooks`, `pre-commit` and the fmt-only leg of `pre-push` now
# come from common.just. The heavy per-leg matrix (static guards, dependency
# audit, three clippy configurations, two build configurations, library tests,
# doc tests, Postgres integration tests) lives in `check-ci` below. It is
# NOT reachable from `pre-push` or any hook: STD-4.2 and STD-4.3 forbid
# compiling in a hook, and `.forgejo/workflows/check.yml` runs the matrix
# in CI.

# Run every check .forgejo/workflows/check.yml runs: the five static guards on
# the host, then the dependency audit, then the cargo legs inside the dev
# compose `app` container. scripts/check-suite-parity.nu compares this recipe
# against the workflow and fails when either grows, loses or weakens a leg
# (LINKS-49). The dependency audit runs next, before anything compiles: it
# reads Cargo.lock rather than the build, so a RUSTSEC advisory fails in
# seconds instead of after the Rust legs (LINKS-52). Covers all three
# compilation configurations: default features, `server`, and `web` on wasm.
# `default = []` gates every server module behind `#[cfg(feature = "server")]`,
# so the default-feature legs alone compile almost none of the crate
# (LINKS-36). All three configurations lint under `--deny warnings`. The wasm
# leg was a bare `cargo check`, so five warnings in browser-only code stayed
# green (LINKS-39). The last two legs cover the test targets no `--lib` run
# reaches: the doc examples, which nothing in the repo compiled until LINKS-48,
# and the tests/ targets against the compose `postgres` service, which
# `cargo test --lib` never ran (LINKS-44). `cargo fmt --check` is kept here
# too, mirroring CI, so `scripts/check-suite-parity.nu` can enforce the
# workflow-vs-recipe parity in both directions (common's `pre-commit-compose`
# also runs it, which is the cost of the parity guard).
[group: 'check']
check-ci: ensure-env ensure-css
    #!/usr/bin/env nu
    print "\n[check-ci] static guards"
    ^nu scripts/check-suite-parity.nu --self-test
    ^nu scripts/check-suite-parity.nu
    ^nu scripts/check-migration-immutability.nu
    ^nu scripts/check-migration-docs.nu --self-test
    ^nu scripts/check-migration-docs.nu
    ^nu scripts/check-build-flags.nu
    ^nu scripts/check-dev-clean-volumes.nu --self-test
    ^nu scripts/check-dev-clean-volumes.nu
    print "\n[check-ci] dependency audit"
    ^nu scripts/check-dependency-audit.nu --self-test
    ^nu scripts/check-dependency-audit.nu --runner "docker compose --file compose.dev.yml run --rm --no-deps --env RUST_LOG=warn app"
    print "\n[check-ci] cargo fmt --check"
    ^docker compose --file compose.dev.yml run --rm --no-deps app cargo fmt --check
    print "\n[check-ci] cargo clippy --all-targets -- --deny warnings"
    ^docker compose --file compose.dev.yml run --rm --no-deps app cargo clippy --all-targets -- --deny warnings
    print "\n[check-ci] cargo clippy --all-targets --features server -- --deny warnings"
    ^docker compose --file compose.dev.yml run --rm --no-deps app cargo clippy --all-targets --features server -- --deny warnings
    print "\n[check-ci] cargo clippy --all-targets --features web --target wasm32-unknown-unknown -- --deny warnings"
    ^docker compose --file compose.dev.yml run --rm --no-deps app cargo clippy --all-targets --features web --target wasm32-unknown-unknown -- --deny warnings
    print "\n[check-ci] cargo build --all-targets"
    ^docker compose --file compose.dev.yml run --rm --no-deps app cargo build --all-targets
    print "\n[check-ci] cargo build --all-targets --features server"
    ^docker compose --file compose.dev.yml run --rm --no-deps app cargo build --all-targets --features server
    print "\n[check-ci] cargo test --lib"
    ^docker compose --file compose.dev.yml run --rm app cargo test --lib
    print "\n[check-ci] cargo test --features server --lib"
    ^docker compose --file compose.dev.yml run --rm app cargo test --features server --lib
    print "\n[check-ci] doc tests (server)"
    ^nu scripts/check-doc-tests-ran.nu --self-test
    ^nu scripts/check-doc-tests-ran.nu --runner "docker compose --file compose.dev.yml run --rm --no-deps app"
    print "\n[check-ci] apply migrations to the test database"
    ^docker compose --file compose.dev.yml run --rm app cargo test --features server --test db_schema
    print "\n[check-ci] integration tests against the compose postgres"
    ^nu scripts/check-db-tests-ran.nu --self-test
    ^nu scripts/check-db-tests-ran.nu --runner "docker compose --file compose.dev.yml run --rm app"
    print "\n[check-ci] all checks passed"

# Use the per-user dev compose file
compose := "docker compose -f compose.dev.yml "

# Ensure .env exists (mode: standalone or saas)
[private]
ensure-env mode="standalone":
    @test -f .env || cp .env.{{ mode }}.example .env

# Ensure the generated stylesheet exists. It is gitignored and produced by `dx build`, but
# `src/main.rs` include_str!s it, so any `--features server` cargo command needs the file.
[private]
ensure-css:
    @test -f assets/tailwind.css || touch assets/tailwind.css

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

# Start development server in Docker with SSO (saas mode, detached, Traefik-routed)
dev-sso: (ensure-env "saas") css-build
    {{ compose }}up --build --detach --remove-orphans app
    @echo ""
    @echo "  App: https://{{env('USER')}}-links.a8n.run"

# Start local development server in Docker — no Traefik, localhost ports (mode: standalone or saas)
dev-local mode="standalone": (ensure-env mode) css-build
    docker compose up --build --remove-orphans app

# Start PostgreSQL container
db-up:
    {{ compose }}up --detach postgres

# Tail app logs
logs:
    {{ compose }}logs --follow app

# Stop PostgreSQL container
db-down:
    {{ compose }}down postgres

# Stop all containers (also removes the dx_out volume to prevent stale-binary crash loops on next start)
down:
    #!/usr/bin/env nu
    docker compose -f compose.dev.yml down --remove-orphans
    let vol = $"rusty-links-dx-($env.USER)"
    let existing = (docker volume ls --quiet | lines)
    if $vol in $existing {
        docker volume rm $vol
    }

# Open a dev session via the /dev/seed-session endpoint (saas mode, debug builds only)
seed-session:
    @echo "Opening: https://{{env('USER')}}-links.a8n.run/dev/seed-session"
    xdg-open "https://{{env('USER')}}-links.a8n.run/dev/seed-session"

# Clear the dev session via the /dev/logout endpoint (saas mode, debug builds only)
logout:
    @echo "Opening: https://{{env('USER')}}-links.a8n.run/dev/logout"
    xdg-open "https://{{env('USER')}}-links.a8n.run/dev/logout"

# Run pending database migrations
migrate-run:
    sqlx migrate run

# Create a new database migration
migrate name:
    sqlx migrate add {{ name }}

# Prepare SQLx offline query data
db-prepare:
    cargo sqlx prepare

# Mirror the cargo lint legs of .forgejo/workflows/check.yml on the host: clippy under all three
# compilation configurations, then fmt. `just pre-commit` stays the authoritative suite; it adds
# the static guards, the dependency audit, and the build and test legs inside the dev container.
# No build or test leg here on purpose: `cargo test --features server --lib` needs the dev
# database, which is the compose stack's job.
check: check-web check-clippy check-fmt

# Lint the browser build, the only leg that compiles `#[cfg(target_arch = "wasm32")]` code (matches the `Clippy (web/wasm)` step in .forgejo/workflows/check.yml)
check-web: ensure-css
    cargo clippy --all-targets --features web --target wasm32-unknown-unknown -- --deny warnings

# Lint the default and server configurations, matching the `Clippy (default features)` and
# `Clippy (server)` steps in .forgejo/workflows/check.yml. This was a bare `cargo clippy`: no
# `--all-targets`, no `--deny warnings`, and no server leg at all, so it exited 0 on its own
# findings and compiled almost none of the crate, since `default = []` gates every server module
# behind `#[cfg(feature = "server")]` (LINKS-36, LINKS-42). ensure-css because the server leg
# compiles src/main.rs, which include_str!s the generated stylesheet.
check-clippy: ensure-css
    cargo clippy --all-targets -- --deny warnings
    cargo clippy --all-targets --features server -- --deny warnings

# Check formatting
check-fmt:
    cargo fmt --check

# Build Docker image for validation. The image is mode-agnostic: standalone vs hosted is
# resolved at runtime from OIDC_ISSUER, so there is no build-time mode argument.
check-docker:
    docker buildx build --tag rusty-links:check -f oci-build/Dockerfile .

# Build the release binary in the configuration the project ships, matching
# `cargo build --release --features server` in oci-build/Dockerfile. Under default
# features `default = []` cfgs out every server module and src/main.rs links its
# `#[cfg(not(feature = "server"))]` main instead, so the artifact launches a client
# with no server, no database and no API (LINKS-36, LINKS-66). `just pre-commit` and
# CI still build both configurations, so nothing is lost by this one picking the
# useful artifact. ensure-css because src/main.rs include_str!s the stylesheet.
build: ensure-css
    cargo build --release --features server

# Build Docker image. Mode-agnostic, same as check-docker.
build-docker:
    docker buildx build --tag rusty-links:local -f oci-build/Dockerfile .

# Run the library suite in the server configuration, matching the `Unit tests (server)`
# step in .forgejo/workflows/check.yml. A bare `cargo test` ran 13 of the crate's 220
# library tests and printed ok, because `default = []` cfgs out every server module
# (LINKS-36, LINKS-66). `--lib` on purpose: the tests/ targets need the compose Postgres
# and have `just test-db`, the doc examples have `just test-doc`, and `just pre-commit`
# runs every leg under both feature sets.
test:
    cargo test --features server --lib

# Run the doc tests, the same leg `just pre-commit` and CI run. Fails if the
# harness collected nothing, since `cargo test --doc` exits 0 on an empty run.
# No database needed, so the container starts with --no-deps.
test-doc:
    #!/usr/bin/env nu
    ^nu scripts/check-doc-tests-ran.nu --self-test
    ^nu scripts/check-doc-tests-ran.nu --runner "docker compose --file compose.dev.yml run --rm --no-deps app"

# Audit Cargo.lock against the RustSec advisory database, the same leg `just pre-commit`
# and CI run. Fails on a vulnerability no dated exception covers; warnings are printed.
# No database needed, so the container starts with --no-deps.
audit:
    #!/usr/bin/env nu
    ^nu scripts/check-dependency-audit.nu --self-test
    ^nu scripts/check-dependency-audit.nu --runner "docker compose --file compose.dev.yml run --rm --no-deps --env RUST_LOG=warn app"

# Run the tests/ targets against the compose `postgres` service, the same leg
# `just pre-commit` and CI run. Fails if a database-backed target is skipped.
test-db:
    #!/usr/bin/env nu
    ^nu scripts/check-db-tests-ran.nu --self-test
    ^nu scripts/check-db-tests-ran.nu --runner "docker compose --file compose.dev.yml run --rm app"

# Run integration tests against a running instance
test-integration url="http://localhost:4002":
    bash scripts/integration-test.sh {{ url }}

# Format code
fmt:
    cargo fmt

# ── Cleanup ──────────────────────────────────────────────────────────────────

# Tear down this repo's dev footprint: bring down the compose stack (both compose.dev.yml and the plain compose.yml stack, dropping their default networks), remove this repo's named volumes (cargo, dx, target, postgres, all ${USER}-suffixed), and delete the local Rust target/ and node_modules/ build artifacts. Scoped to this repo; safe on a shared host. Replaces the former `clean` recipe.
[group: 'cleanup']
dev-clean:
    #!/usr/bin/env nu
    docker compose -f compose.dev.yml down --remove-orphans
    docker compose down --remove-orphans
    let suffix = $env.USER
    let vols = [
        $"rusty-links-cargo-($suffix)"
        $"rusty-links-dx-($suffix)"
        $"rusty-links-target-($suffix)"
        $"rusty-links-postgres-($suffix)"
    ]
    let existing = docker volume ls --quiet | lines
    for vol in $vols {
        if $vol in $existing {
            docker volume rm $vol
            print $"removed volume ($vol)"
        }
    }
    let paths = [target node_modules]
    for p in $paths {
        if ($p | path exists) {
            rm --recursive $p
            print $"removed ($p)"
        }
    }
    print "dev-clean: done"

# Everything dev-clean does, plus remove the Docker images this repo builds (rusty-links:check from check-docker, rusty-links:local from build-docker) and prune its buildx cache. Run for a from-scratch rebuild.
[group: 'cleanup']
dev-clean-all: dev-clean
    #!/usr/bin/env nu
    let images = [
        "rusty-links:check"
        "rusty-links:local"
    ]
    for img in $images {
        let present = (do { ^docker image inspect $img } | complete).exit_code == 0
        if $present {
            docker image rm $img
            print $"removed image ($img)"
        }
    }
    docker buildx prune --force
    print "dev-clean-all: done"

# ── Release ──────────────────────────────────────────────────────────────────

# LINKS-87: `create-release` now comes from common.just via `release_layout
# := "generic"` (set above). Common bumps `[package] version` in Cargo.toml,
# opens the release PR via `fj --host dev.a8n.run pr create`, and runs the
# same preflight checks the old local recipe did. The automatic tag + image
# publish still runs through .forgejo/workflows/create-release.yml on merge.

