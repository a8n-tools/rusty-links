# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rusty Links is a full-stack Rust web application using Dioxus 0.7 (fullstack mode) for the frontend. It uses PostgreSQL for data storage with SQLx for database access. Tailwind CSS v4 is used for the styling.
IMPORTANT: Do NOT modify `./assets/tailwind.css`. All CSS should go in `./tailwind.css` and `dx` will automatically run the `tailwindcss` cli to generate `./assets/tailwind.css`.

`./assets/tailwind.css` is gitignored, but `src/main.rs` `include_str!`s it, so a plain `cargo build --features server` fails on a fresh checkout until the file exists. `just pre-commit` and `.forgejo/workflows/check.yml` each create an empty placeholder first; `dx build` overwrites it with the real stylesheet.

## Build & Development Commands

`just pre-commit` is the authoritative check suite: it runs every check `.forgejo/workflows/check.yml` runs, and fails on the first red one. Run it before every commit. It starts with the four static guards that need no compiler (`scripts/check-suite-parity.nu`, `scripts/check-migration-immutability.nu`, `scripts/check-migration-docs.nu`, `scripts/check-build-flags.nu`), which run on the host in well under a second, then the dependency audit, then the cargo legs in the dev container.

`scripts/check-suite-parity.nu` is what keeps that claim true. The recipe and the workflow are two hand-maintained copies of one suite, so it parses the cargo invocations and guard-script calls out of both and fails when either side has a leg the other lacks, in either direction. The `check` recipe is a third copy, of the lint legs only, and is compared one way: every clippy configuration and the fmt leg the workflow runs must appear in `check`, resolved through its dependency recipes, while the build, test, doc-test and database legs it deliberately omits are not demanded of it. Until LINKS-64 `check` was outside the guard entirely, so deleting its `--features server` clippy leg left both this guard and `scripts/check-build-flags.nu` at exit 0. It normalises the spellings each file uses locally (`-D warnings` in the workflow, `--deny warnings` in the recipe; `--features=web` and `--features web`; flag order), so only a real difference is drift. It also fails when a clippy leg carries no `--deny warnings`, and when no clippy leg covers one of the three compilation configurations, which is the drift that survives being applied to every copy at once: `.cargo/config.toml` pins `[build] target = "x86_64-unknown-linux-gnu"`, so a wasm leg that lost `--target` silently re-lints the host build instead of failing (LINKS-39, LINKS-49).

Its last two legs run the test targets that `cargo test --lib` never reaches, each behind a guard script, because `cargo test` exits 0 on an empty run and a suite that runs nothing looks identical to one that passes:

- `scripts/check-doc-tests-ran.nu` runs `cargo test --features server --doc`. Nothing in the repo compiled a doc example before LINKS-48, so all ten had rotted into compile errors on missing `use` lines. The guard fails when the harness collected nothing, when anything is ignored or filtered out, or when passes fall below its floor. An example that must not execute is marked ```` ```no_run ```` (compiled and type-checked, never run) and still counts as a pass; ```` ```ignore ```` is never compiled and fails the leg.
- `scripts/check-db-tests-ran.nu` runs the `tests/` targets against the compose `postgres` service. Before LINKS-44 every `tests/` target was compiled by `--all-targets` and never executed, so no SQL in the repo was covered. New SQL belongs in a `tests/db_*.rs` case; the guard fails when a `db_*` target is missing, ignored, filtered out, or below its floor on passes.

`scripts/check-dependency-audit.nu` is the dependency-audit leg (LINKS-52). It runs `cargo audit` against `Cargo.lock`, so it needs cargo but no build and finishes in seconds, and it fails on a RustSec vulnerability that no dated row in its `EXCEPTIONS` table covers. Warnings (`unmaintained`, `unsound`, `yanked`, `notice`) are printed and do not fail: they are usually unfixable from here and would block unrelated work, and `cargo audit --deny warnings` is the stricter view on demand. An exception row names the crate, the LINKS issue tracking its removal and the date the acceptance stops holding, and the guard rejects a row that is expired, stale, undated or names no issue, so file the issue before adding a row. `.forgejo/workflows/audit.yml` runs the same guard weekly, which is the only run that catches an advisory published against an unchanged `Cargo.lock`.

Nothing measures test coverage and that is deliberate, so do not add a percentage to a doc or a threshold to a job without reopening the decision in docs/TESTING.md (LINKS-51).

IMPORTANT: `default = []`, so a bare `cargo check` / `cargo clippy` / `cargo test` compiles almost none of this crate. Every server module is behind `#[cfg(feature = "server")]`, so any command meant to verify server code must pass `--features server`.

```bash
# Run every check CI runs (the four static guards, the dependency audit, then fmt, clippy under default + server + web/wasm, build, unit/doc/Postgres tests under default + server)
just pre-commit

# Run in development (requires PostgreSQL and .env file)
dx serve

# Check for compilation errors without building (default features: UI only)
cargo check

# Check server feature only
cargo check --features server

# Lint the browser build (the only leg that compiles `#[cfg(target_arch = "wasm32")]` code)
cargo clippy --all-targets --features web --target wasm32-unknown-unknown -- --deny warnings

# Run tests (default features runs only the handful of feature-independent tests)
cargo test

# Run the server-side library suite (`just test` runs exactly this; a bare `cargo test`
# runs 13 of the 220 library tests and prints ok, because `default = []`)
just test
cargo test --features server --lib

# Run a specific test
cargo test --features server <test_name>

# Run the doc examples with the vacuity guard, the same leg pre-commit and CI run
just test-doc

# Run the tests/ targets against the compose postgres, with the skip guard
just test-db

# Audit Cargo.lock against the RustSec advisory database, the same leg pre-commit and CI run
just audit

# Lint on the host: clippy under all three compilation configurations, then fmt.
# `just pre-commit` stays the authoritative suite; `just check` is the fast loop and
# leaves out the build, test, doc-test, database and audit legs, which need the compose stack.
just check

# Or the individual legs (the default leg alone does not lint server code)
cargo fmt
cargo clippy --all-targets -- --deny warnings
cargo clippy --all-targets --features server -- --deny warnings

# Database migrations (auto-run on startup, but manual commands available)
sqlx migrate add <migration_name>
sqlx migrate run
```

## Feature Flags

The project uses Cargo features to separate server and client code:
- `server` - Enables Axum, SQLx, Tokio, and server-side modules
- `web` - Enables WASM/browser-specific dependencies (gloo-net, web-sys)

Server-only modules (`#[cfg(feature = "server")]`): api, auth, config, error, github, models, scheduler, scraper, security

Because `default = []`, any check that omits `--features server` compiles none of those modules. `just pre-commit` and `.forgejo/workflows/check.yml` therefore run build and test twice, once with default features and once with `--features server`, and clippy three times, adding `--features web --target wasm32-unknown-unknown`. That third leg is the only one that compiles `#[cfg(target_arch = "wasm32")]` code, so a warning there is invisible to the other two; it was a bare `cargo check` with no `--deny warnings` until LINKS-39, which is how five of them accumulated. The `tests/` targets are all `#![cfg(feature = "server")]`, so they only ever run on the server leg, and so does the doc-test leg: every module carrying a doc example is behind `#[cfg(feature = "server")]`, so `cargo test --doc` without it collects zero tests and passes vacuously.

There are no `standalone`/`saas` build features. A single binary and OCI image serves both deployment modes; the mode is resolved at runtime (see Configuration below).

## Architecture

- **Entry point**: `src/main.rs` - Initializes database pool, starts scheduler, creates Axum router with Dioxus frontend and API routes. The LINKS-31 trusted-proxy gate is the OUTERMOST layer and must stay there: it rewrites the forwarded headers, so anything layered outside it would read forgeable values.
- **API layer**: `src/api/` - REST endpoints nested under `/api`, with auth routes at `/api/auth/*`
- **Server functions**: `src/server_functions/` - Dioxus server functions bridging client/server communication (available on both sides)
- **Auth**: `src/auth/` - Session-based authentication using cookies, Argon2id password hashing. `trusted_proxy.rs` gates `X-Forwarded-For` / `X-Real-Ip` / `X-IPCountry` on the socket peer; every client-IP and country reader goes through `middleware.rs` so the gate covers them all. `location_alert.rs` holds the single definition of a suspicious sign-in in two halves: `is_new_country` (LINKS-27 alert and LINKS-35 gate both call it) and `is_new_device` (LINKS-45, the gate only). The approval gate (`login_approval.rs`) ORs the two, so both triggers share one kill switch and one hold. `known_device.rs` owns the device identity: a random id the browser mints into `localStorage` and sends with the sign-in, stored only as its SHA-256 in `known_devices`, never a User-Agent. Devices are recorded ONLY from `api/auth.rs::establish_jwt_session` (the one place a session is minted) and from a claimed approval, which is what makes "recorded only when a sign-in completes" structural.
- **UI**: `src/ui/` - Dioxus components with pages (`pages/`) and reusable components (`components/`)
- **Models**: `src/models/` - Database models (User, Link, Category, Tag, etc.)
- **Scheduler**: `src/scheduler/` - Background task runner for periodic metadata updates
- **Scraper**: `src/scraper/` - HTML metadata extraction (titles, descriptions, logos)
- **GitHub**: `src/github/` - GitHub API integration for repo metadata (stars, languages, licenses)
- **Config**: `src/config.rs` - Environment-based configuration
- **Errors**: `src/error.rs` - Centralized error handling with `AppError` type
- **Security**: `src/security.rs` - SSRF guard every scrape target passes through (`validate_url_for_ssrf`), plus the password policy, the login-attempt lockout, and the scheduler's login-attempt and refresh-token cleanups

## Docker Directory Structure

The production container follows a standard three-directory layout:
- `/app` — Application binary and empty directories required by dioxus-server (`assets/`, `public/`). Read-only at runtime.
- `/data` — Persistent application data, mounted as a Docker volume. Currently unused but available for future needs.
- `/config` — Application configuration, mountable as a Docker volume. Currently unused; env vars are passed via `environment:` in `compose.yml`.

Migrations are embedded at compile time by `sqlx::migrate!()` and are not copied to the runtime image.

## Database

- PostgreSQL with SQLx (compile-time checked queries)
- Migrations in `migrations/` directory, run automatically on startup
- A committed migration is immutable; fixes go in a NEW migration (`scripts/check-migration-immutability.nu`)
- A new migration also needs a row in the Migration History table in `docs/DATABASE.md`, or `scripts/check-migration-docs.nu` fails CI. The table had drifted to six of fourteen rows before that guard existed (LINKS-46)
- A migration that adds or drops a table or column also needs the Tables Reference in `docs/DATABASE.md` updated, or `database_doc_matches_the_migrated_schema` in `tests/db_schema.rs` fails: it compares every documented table and column against a migrated database in both directions. That guard is a test rather than a static script because the applied shape is not readable from the SQL text (`20250101000007` drops and re-adds `links.logo` inside a PL/pgSQL block). The reference had drifted through eleven migrations before it existed (LINKS-53)
- Connection pool: 5 max connections, 30s timeout, 10min idle timeout
- Tests get their own database: `tests/common/mod.rs::test_pool` derives `<database>_test` from `DATABASE_URL`, creates it, and migrates it, so a `just pre-commit` run never writes to dev data

## Configuration

The deployment mode is selected at runtime, not at compile time. The same binary/image serves both modes; `OIDC_ISSUER` is the switch:
- `OIDC_ISSUER` unset → standalone mode (local JWT auth, setup flow, user admin).
- `OIDC_ISSUER` set → hosted mode (OIDC login against a8n Tools, active-membership gate). `Config::hosted()` (`src/config.rs`) returns `!oidc.issuer.is_empty()`.

`TRUSTED_PROXY_CIDRS` lists the CIDRs whose socket peers may set `X-Forwarded-For`, `X-Real-Ip`, and `X-IPCountry` (LINKS-31). Empty (the default) trusts no peer and ignores all three, which is correct for local dev; a deployment behind a reverse proxy must set the private ingress ranges or every client collapses to the proxy address and the sign-in location alert stops firing.

`LOGIN_APPROVAL_ENABLED` (LINKS-35, LINKS-45) is opt-in and exact-match `true`, OFF by default: it withholds a session on a sign-in from a country (LINKS-35) or a device (LINKS-45) the account has not used before, until the owner approves it from an emailed link. One switch for both triggers; there is deliberately no second flag. Standalone mode only, since it gates this app's own credential login, and the device trigger does not change that: the OIDC-RP callback stays ungated because the OP owns that credential and its recovery. `known_devices` is created empty and is NOT backfilled, and zero known devices is read as an account's baseline rather than as "new device", so the deploy holds nobody. Recovery from a lost approval email is in `docs/SECURITY.md` and does not depend on email.

`/api/health` reports the resolved mode in its `auth_mode` field (`"standalone"` or `"hosted"`); the WASM client reads it once to render the correct login experience.

Two example env files document the variables for each mode (both are deployment templates for the same binary):
- `.env.standalone.example` - standalone mode (JWT auth variables; leave `OIDC_ISSUER` unset).
- `.env.saas.example` - hosted mode (the `OIDC_*` variables; set `OIDC_ISSUER`).

Copy the appropriate file to `.env` before running:
```bash
cp .env.standalone.example .env   # or .env.saas.example
```

IMPORTANT: When updating code, ALWAYS check if `.env.standalone.example` and `.env.saas.example` need to be updated. This applies when adding, removing, or renaming environment variables in `src/config.rs`, `compose.dev.yml`, `compose.yml`, or any `std::env::var` call. Both files must stay in sync.
