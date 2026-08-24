# Rusty Links - Task Runner

# List available recipes
default:
    @just --list

# Install the git pre-commit hook (run once per fresh clone). Writes a stub at .git/hooks/pre-commit that execs `just pre-commit`. Bypass with `git commit --no-verify`.
install-hooks:
    #!/usr/bin/env nu
    let hook = ".git/hooks/pre-commit"
    # Remove first so a leftover symlink from an older install does not get
    # written through to its target file. `try` swallows the not-found case.
    try { rm $hook }
    "#!/usr/bin/env sh\nexec just pre-commit\n" | save $hook
    ^chmod +x $hook
    print $"Wrote ($hook) -> just pre-commit"

# Run the same checks as .forgejo/workflows/check.yml inside the dev compose `app` container.
# Covers all three compilation configurations: default features, `server`, and `web` on wasm.
# `default = []` gates every server module behind `#[cfg(feature = "server")]`, so the
# default-feature legs alone compile almost none of the crate (LINKS-36).
# All three configurations lint under `--deny warnings`. The wasm leg was a bare
# `cargo check`, so five warnings in browser-only code stayed green (LINKS-39).
# The last two legs cover the test targets no `--lib` run reaches: the doc examples,
# which nothing in the repo compiled until LINKS-48, and the tests/ targets against
# the compose `postgres` service, which `cargo test --lib` never ran (LINKS-44).
pre-commit: ensure-env ensure-css
    #!/usr/bin/env nu
    print "\n[pre-commit] cargo fmt --check"
    ^docker compose --file compose.dev.yml run --rm --no-deps app cargo fmt --check
    print "\n[pre-commit] cargo clippy --all-targets -- --deny warnings"
    ^docker compose --file compose.dev.yml run --rm --no-deps app cargo clippy --all-targets -- --deny warnings
    print "\n[pre-commit] cargo clippy --all-targets --features server -- --deny warnings"
    ^docker compose --file compose.dev.yml run --rm --no-deps app cargo clippy --all-targets --features server -- --deny warnings
    print "\n[pre-commit] cargo clippy --all-targets --features web --target wasm32-unknown-unknown -- --deny warnings"
    ^docker compose --file compose.dev.yml run --rm --no-deps app cargo clippy --all-targets --features web --target wasm32-unknown-unknown -- --deny warnings
    print "\n[pre-commit] cargo build --all-targets"
    ^docker compose --file compose.dev.yml run --rm --no-deps app cargo build --all-targets
    print "\n[pre-commit] cargo build --all-targets --features server"
    ^docker compose --file compose.dev.yml run --rm --no-deps app cargo build --all-targets --features server
    print "\n[pre-commit] cargo test --lib"
    ^docker compose --file compose.dev.yml run --rm app cargo test --lib
    print "\n[pre-commit] cargo test --features server --lib"
    ^docker compose --file compose.dev.yml run --rm app cargo test --features server --lib
    print "\n[pre-commit] doc tests (server)"
    ^nu scripts/check-doc-tests-ran.nu --self-test
    ^nu scripts/check-doc-tests-ran.nu --runner "docker compose --file compose.dev.yml run --rm --no-deps app"
    print "\n[pre-commit] integration tests against the compose postgres"
    ^nu scripts/check-db-tests-ran.nu --self-test
    ^nu scripts/check-db-tests-ran.nu --runner "docker compose --file compose.dev.yml run --rm app"
    print "\n[pre-commit] all checks passed"

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

# Run all checks (web, clippy, fmt)
check: check-web check-clippy check-fmt

# Lint the browser build, the only leg that compiles `#[cfg(target_arch = "wasm32")]` code (matches the `Clippy (web/wasm)` step in .forgejo/workflows/check.yml)
check-web: ensure-css
    cargo clippy --all-targets --features web --target wasm32-unknown-unknown -- --deny warnings

# Run clippy lints
check-clippy:
    cargo clippy

# Check formatting
check-fmt:
    cargo fmt --check

# Build Docker image for validation. The image is mode-agnostic: standalone vs hosted is
# resolved at runtime from OIDC_ISSUER, so there is no build-time mode argument.
check-docker:
    docker buildx build --tag rusty-links:check -f oci-build/Dockerfile .

# Build release binary
build:
    cargo build --release

# Build Docker image. Mode-agnostic, same as check-docker.
build-docker:
    docker buildx build --tag rusty-links:local -f oci-build/Dockerfile .

# Run tests
test:
    cargo test

# Run the doc tests, the same leg `just pre-commit` and CI run. Fails if the
# harness collected nothing, since `cargo test --doc` exits 0 on an empty run.
# No database needed, so the container starts with --no-deps.
test-doc:
    #!/usr/bin/env nu
    ^nu scripts/check-doc-tests-ran.nu --self-test
    ^nu scripts/check-doc-tests-ran.nu --runner "docker compose --file compose.dev.yml run --rm --no-deps app"

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

# Tear down this repo's dev footprint: bring down the compose stack (both compose.dev.yml and the plain compose.yml stack, dropping their default networks), remove this repo's named volumes (cargo, dx, target-server, target-wasm, postgres, all ${USER}-suffixed), and delete the local Rust target/ and node_modules/ build artifacts. Scoped to this repo; safe on a shared host. Replaces the former `clean` recipe.
[group: 'cleanup']
dev-clean:
    #!/usr/bin/env nu
    docker compose -f compose.dev.yml down --remove-orphans
    docker compose down --remove-orphans
    let suffix = $env.USER
    let vols = [
        $"rusty-links-cargo-($suffix)"
        $"rusty-links-dx-($suffix)"
        $"rusty-links-target-server-($suffix)"
        $"rusty-links-target-wasm-($suffix)"
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

# Create a release: bump major (vx.0.0), minor (v0.x.0), or hotfix (v0.0.x), push branch, and print PR link
# After the PR is merged, the create-release workflow creates the tag and release automatically
[group: 'release']
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

    # Open the release PR via fj. Body lives in a tempfile so the
    # changelog can grow later without inline escaping pain.
    let body_file = (mktemp --tmpdir --suffix .md)
    [
        $"Automated release PR for ($tag)."
        ""
        $"After merge, `.forgejo/workflows/create-release.yml` tags and publishes ($tag) to the Generic Packages registry."
    ] | str join "\n" | save --force $body_file
    let fj_result = (^fj --host dev.a8n.run pr create $"Release ($tag)" --body-file $body_file | complete)
    rm $body_file
    if $fj_result.exit_code != 0 {
        print $"(ansi red)fj pr create failed(ansi reset)"
        print $fj_result.stderr
        exit 1
    }

    # `fj pr create` prints `created pull request #N: <title>` on success.
    # Parse the number out and build the PR URL from `origin`.
    let pr_num = (
        $fj_result.stdout
        | str trim
        | parse --regex 'created pull request #(?P<num>\d+)'
        | get num.0?
    )
    let remote = (git remote get-url origin | str trim)
    let base_url = if ($remote | str starts-with "ssh://") {
        $remote | str replace "ssh://git@" "https://" | str replace "git.a8n.run" "dev.a8n.run" | str replace ".git" ""
    } else {
        $remote | str replace --regex "git@([^:]+):" "https://$1/" | str replace "git.a8n.run" "dev.a8n.run" | str replace ".git" ""
    }
    print $"(ansi green)Pushed ($release_branch)(ansi reset)"
    if ($pr_num | is-not-empty) {
        print $"PR: ($base_url)/pulls/($pr_num)"
    } else {
        # fj output format drifted; fall back to whatever it said.
        print $"fj output: ($fj_result.stdout | str trim)"
    }
    print $"After merging, the create-release workflow will tag and release ($tag) automatically."

