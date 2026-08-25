# Testing Documentation

Comprehensive testing guide for Rusty Links, covering unit tests, integration tests, and end-to-end testing strategies.

## Table of Contents

- [Overview](#overview)
- [Running Tests](#running-tests)
- [Test Organization](#test-organization)
- [Writing Tests](#writing-tests)
- [Test Coverage](#test-coverage)
- [CI/CD Testing](#cicd-testing)
- [Manual Testing](#manual-testing)
- [Performance Testing](#performance-testing)
- [Troubleshooting](#troubleshooting)

---

## Overview

Rusty Links uses a multi-layered testing approach to ensure code quality and reliability:

### Testing Layers

1. **Unit Tests** - Test individual functions and modules in isolation
   - Located in source files with `#[cfg(test)]` modules
   - Fast execution, no external dependencies
   - Test pure logic and algorithms

2. **Doc Tests** - Compile and run the examples in `///` comments
   - Located in the doc comments of `src/`, so they only exist for the library target
   - Run by `cargo test --features server --doc`, which no `--lib` or `--all-targets` command builds
   - Keep the public API documentation honest: an example that stops compiling fails the build (LINKS-48)

3. **Integration Tests** - Test API endpoints and component interactions
   - Located in `tests/` directory
   - Use test database
   - Test complete workflows

4. **Smoke Tests** - Exercise a running instance over HTTP
   - `scripts/integration-test.sh` (`just test-integration [url]`), curl and jq against a deployed URL
   - Run by hand against a dev or staging instance; no check suite runs it

### Testing Philosophy

- **Test Behavior, Not Implementation** - Focus on what code does, not how
- **Fast Feedback** - Tests should run quickly
- **Isolated Tests** - Each test should be independent
- **Readable Tests** - Tests serve as documentation
- **Cover What You Change** - no coverage percentage is measured; see [Test Coverage](#test-coverage)

---

## Running Tests

### Basic Commands

Server code lives behind `#[cfg(feature = "server")]` and `default = []`, so every command below needs `--features server` to compile and run the server-side tests.

```bash
# Run all tests (unit + integration)
cargo test --features server

# Run with output visible
cargo test --features server -- --nocapture

# Run specific test by name
cargo test --features server test_create_link

# Run tests matching pattern
cargo test --features server auth

# Run tests in specific file (the target name is the tests/<name>.rs stem)
cargo test --features server --test db_users
```

### Advanced Options

```bash
# Run unit tests only (no integration tests)
cargo test --features server --lib

# Run integration tests only (needs DATABASE_URL; see "Database Setup for Tests")
cargo test --features server --test '*'

# Run them the way `just pre-commit` and CI do, with the skip guard
nu scripts/check-db-tests-ran.nu

# Run the doc examples only (the library target's examples; needs --features
# server, since every documented module is cfg'd out without it)
cargo test --features server --doc

# Run them the way `just pre-commit` and CI do, with the vacuity guard
nu scripts/check-doc-tests-ran.nu

# Run tests for specific package
cargo test --features server -p rusty-links

# Run every feature at once (server + web; web needs the wasm target, so this is rarely useful here)
cargo test --all-features

# Run tests in parallel (default)
cargo test --features server

# Run tests serially (useful for database tests)
cargo test --features server -- --test-threads=1

# Show test output even for passing tests
cargo test --features server -- --nocapture --show-output
```

### Environment Setup

The `tests/db_*.rs` targets need `DATABASE_URL`, and they never skip without it: an unset or unreachable value is a panic, because a suite that skips still exits 0 and looks green (LINKS-44).

```bash
# The compose `app` container already has it, which is what `just pre-commit` uses
just test-db

# Standalone Postgres for a bare `cargo test`
just db-up
export DATABASE_URL="postgresql://rustylinks:changeme_secure_password_here@localhost/rustylinks"
```

### Database Setup for Tests

Nothing to create by hand. `tests/common/mod.rs::test_pool` derives a sibling `<database>_test` from `DATABASE_URL`, creates it if it is missing, and applies `migrations/` to it with `sqlx::migrate!`. The database named in `DATABASE_URL` is only ever connected to in order to issue the `CREATE DATABASE`, so running the suites against a dev database does not touch dev data.

In CI the same code path runs against the `postgres:17-alpine` service declared in `.forgejo/workflows/check.yml`.

---

## Test Organization

### Directory Structure

```
rusty-links/
├── src/
│   ├── main.rs
│   ├── auth/
│   │   └── session.rs       # Contains #[cfg(test)] mod tests
│   ├── models/
│   │   ├── user.rs          # Contains unit tests
│   │   └── link.rs          # Contains unit tests
│   └── api/
│       └── auth.rs          # API logic (tested via integration tests)
├── tests/
│   ├── common/
│   │   └── mod.rs             # Pool, fixtures, router and session helpers
│   ├── db_account_settings.rs # PATCH/GET /api/auth/me through the column
│   ├── db_known_devices.rs    # LINKS-45 device recognition, through the login route
│   ├── db_login_approval.rs   # LINKS-35 / LINKS-45 approval-gate lifecycle
│   ├── db_refresh_tokens.rs   # LINKS-59 refresh tokens stored and resolved by hash
│   ├── db_schema.rs           # Migrations apply; shapes; DATABASE.md against the schema
│   ├── db_users.rs            # User creation round trips
│   └── route_surface.rs       # Per-mode route mounting (no database)
└── Cargo.toml
```

### Unit Tests

Unit tests are co-located with the code they test:

```rust
// src/models/user.rs

pub fn validate_email(email: &str) -> bool {
    email.contains('@')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_email() {
        assert!(validate_email("user@example.com"));
        assert!(!validate_email("invalid-email"));
    }
}
```

### Integration Tests

Integration tests live in `tests/` directory:

```rust
// tests/integration_tests.rs

use rusty_links::models::User;
use sqlx::PgPool;

#[tokio::test]
async fn test_create_user() {
    let pool = setup_test_db().await;
    let user = User::create(&pool, "test@example.com", "password").await;
    assert!(user.is_ok());
}
```

### Test Utilities

Common test helpers in `tests/common/mod.rs`:

```rust
// tests/common/mod.rs

// Pool for the `<database>_test` sibling, created and migrated on first use.
// Panics when DATABASE_URL is unset or unreachable; it never skips.
pub async fn test_pool() -> PgPool;

// Fixtures are per test rather than a global truncate, because the targets
// share one database.
pub fn unique_email() -> String;
pub async fn new_user(pool: &PgPool) -> User;
pub async fn delete_user(pool: &PgPool, id: Uuid);

// Standalone-mode config, the `/api` router over a real pool, and a session
// for a fixture user.
pub fn test_config() -> Config;
pub fn api_router(pool: PgPool, config: Config) -> axum::Router;
pub fn bearer(config: &Config, user: &User) -> String;
```

---

## Writing Tests

### Unit Test Examples

#### Testing Pure Functions

```rust
// src/utils.rs

pub fn parse_domain(url: &str) -> Result<String, Error> {
    let parsed = Url::parse(url)?;
    Ok(parsed.host_str().unwrap_or("").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_domain() {
        assert_eq!(
            parse_domain("https://example.com/path").unwrap(),
            "example.com"
        );
    }

    #[test]
    fn test_parse_domain_invalid_url() {
        assert!(parse_domain("not-a-url").is_err());
    }
}
```

#### Testing with Mock Data

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_masked_database_url() {
        let config = Config {
            database_url: "postgresql://user:password@localhost/db".to_string(),
            app_port: 8080,
            update_interval_days: 30,
            log_level: "info".to_string(),
            update_interval_hours: 24,
            batch_size: 50,
            jitter_percent: 20,
        };

        let masked = config.masked_database_url();
        assert!(!masked.contains("password"));
        assert!(masked.contains("****"));
    }
}
```

#### Testing Async Functions

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_session() {
        let pool = setup_test_pool().await;
        let user_id = create_test_user(&pool).await;

        let session = create_session(&pool, user_id).await.unwrap();
        assert_eq!(session.user_id, user_id);
        assert_eq!(session.id.len(), 64); // Session token length
    }
}
```

### Integration Test Examples

#### Testing API Endpoints

```rust
// tests/api_tests.rs

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

#[tokio::test]
async fn test_health_endpoint() {
    let app = create_test_app().await;

    let response = app
        .oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_create_link_unauthorized() {
    let app = create_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/links")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"url":"https://example.com"}"#))
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
```

#### Testing Database Operations

```rust
// tests/database_tests.rs

use rusty_links::models::{Link, CreateLink};

#[tokio::test]
async fn test_create_and_retrieve_link() {
    let pool = setup_test_db().await;
    let user = create_test_user(&pool).await;

    // Create link
    let create_link = CreateLink {
        url: "https://example.com".to_string(),
        title: Some("Example".to_string()),
        description: None,
        logo: None,
        category_ids: vec![],
        tag_ids: vec![],
        language_ids: vec![],
        license_ids: vec![],
    };

    let link = Link::create(&pool, user.id, create_link).await.unwrap();

    // Retrieve link
    let retrieved = Link::get_by_id(&pool, link.id, user.id).await.unwrap();
    assert_eq!(retrieved.url, "https://example.com");
    assert_eq!(retrieved.title, Some("Example".to_string()));

    cleanup_test_db(&pool).await;
}
```

#### Testing Authentication Flow

```rust
// tests/auth_tests.rs

#[tokio::test]
async fn test_full_auth_flow() {
    let app = create_test_app().await;
    let pool = get_test_pool(&app);

    // 1. Check setup is required
    let response = get(&app, "/api/auth/check-setup").await;
    assert_eq!(response.status(), StatusCode::OK);

    // 2. Create first user (setup)
    let response = post_json(
        &app,
        "/api/auth/setup",
        json!({
            "email": "admin@example.com",
            "password": "SecurePass123!"
        })
    ).await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // 3. Login
    let response = post_json(
        &app,
        "/api/auth/login",
        json!({
            "email": "admin@example.com",
            "password": "SecurePass123!"
        })
    ).await;
    assert_eq!(response.status(), StatusCode::OK);
    let cookies = extract_cookies(&response);

    // 4. Access protected endpoint
    let response = get_with_cookies(&app, "/api/auth/me", &cookies).await;
    assert_eq!(response.status(), StatusCode::OK);

    // 5. Logout
    let response = post_with_cookies(&app, "/api/auth/logout", &cookies).await;
    assert_eq!(response.status(), StatusCode::OK);

    cleanup_test_db(&pool).await;
}
```

### Best Practices

#### 1. Test Naming

```rust
// Good: Descriptive test names
#[test]
fn test_create_link_with_valid_url_succeeds() { }

#[test]
fn test_create_link_with_invalid_url_returns_error() { }

#[test]
fn test_create_link_with_duplicate_url_returns_conflict() { }

// Avoid: Vague names
#[test]
fn test_link() { }

#[test]
fn test_error() { }
```

#### 2. Arrange-Act-Assert Pattern

```rust
#[test]
fn test_user_creation() {
    // Arrange - Set up test data
    let email = "test@example.com";
    let password = "password123";

    // Act - Perform the action
    let result = create_user(email, password);

    // Assert - Verify the outcome
    assert!(result.is_ok());
    let user = result.unwrap();
    assert_eq!(user.email, email);
}
```

#### 3. Test One Thing

```rust
// Good: Focused tests
#[test]
fn test_valid_email_accepted() {
    assert!(validate_email("user@example.com"));
}

#[test]
fn test_invalid_email_rejected() {
    assert!(!validate_email("invalid"));
}

// Avoid: Testing multiple things
#[test]
fn test_email_validation() {
    assert!(validate_email("user@example.com"));
    assert!(!validate_email("invalid"));
    assert!(validate_email("another@domain.org"));
    // Too many assertions
}
```

#### 4. Use Helper Functions

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_link() -> CreateLink {
        CreateLink {
            url: "https://example.com".to_string(),
            title: Some("Test Link".to_string()),
            description: None,
            logo: None,
            category_ids: vec![],
            tag_ids: vec![],
            language_ids: vec![],
            license_ids: vec![],
        }
    }

    #[tokio::test]
    async fn test_create_link() {
        let link_data = create_test_link();
        // Use link_data in test
    }
}
```

#### 5. Clean Up After Tests

```rust
#[tokio::test]
async fn test_database_operation() {
    let pool = setup_test_db().await;

    // Test code here

    // Always clean up
    cleanup_test_db(&pool).await;
}
```

---

## Test Coverage

Nothing measures coverage, and that is a decision rather than an omission (LINKS-51). No recipe in `justfile` and no step in `.forgejo/workflows/check.yml` runs `cargo llvm-cov`, `cargo tarpaulin` or any other coverage tool, no report is written, and there is no coverage service configured for this environment. There is therefore no percentage to look up and no per-module target to hit.

The reasoning, so it does not get relitigated from scratch:

- A number with no gate behind it is decoration. Nobody acts on a printed percentage, and a threshold invented without evidence either fails every pull request or is set so low it proves nothing.
- `cargo llvm-cov` needs `llvm-tools-preview` on both the runner image and the dev image, neither of which this repository builds, and an instrumented rebuild roughly doubles a suite that already compiles the crate in three configurations.
- What actually goes wrong here is not thin coverage, it is a leg that runs and proves nothing: `cargo test` exits 0 on an empty run, which is how the `tests/` targets sat compiled but unexecuted (LINKS-44) and how all ten doc examples rotted into compile errors (LINKS-48). The guard scripts enforce execution floors against that directly, which is a stronger property than a percentage and is checked on every push.

If you want a local number for your own purposes, `cargo llvm-cov` and `cargo tarpaulin` both work against this crate with `--features server`; nothing in the repository depends on the result.

### What to prioritise when adding tests

1. **Critical paths** - Authentication, data persistence
2. **Error handling** - All error branches
3. **Edge cases** - Empty inputs, boundary values
4. **Business logic** - Core functionality
5. **API contracts** - Request/response validation

---

## CI/CD Testing

### Forgejo Actions Workflow

Tests run automatically on every push to `main` and every pull request. See `.forgejo/workflows/check.yml` for the complete configuration; the `check` job declares a `postgres:17-alpine` service (matching `compose.dev.yml`, so dev and CI agree) and exports `DATABASE_URL` for the database-backed legs. The `Doc tests (server)` step needs no database and runs before them.

There is one job, on one runner label, against one Postgres version, with the toolchain the runner image ships (nothing in the repo pins one). Before the cargo legs it runs the four static guards, which need no compiler: `scripts/check-suite-parity.nu`, `scripts/check-migration-immutability.nu`, `scripts/check-migration-docs.nu` and `scripts/check-build-flags.nu`. The `Dependency audit` step follows them: it needs cargo but no build, since `scripts/check-dependency-audit.nu` reads `Cargo.lock` (LINKS-52).

A second workflow, `.forgejo/workflows/audit.yml`, runs that audit guard on a weekly schedule and nothing else. A push-triggered job only ever audits a lockfile somebody just touched, so the schedule is the only run that sees an advisory published against dependencies that have not changed. [docs/SECURITY.md](SECURITY.md#security-audit) has the failure policy and the exception rules.

### Local Pre-commit Testing

```bash
# Run all checks before committing (every check .forgejo/workflows/check.yml runs, in the same order)
just pre-commit

# Or manually:
nu scripts/check-suite-parity.nu --self-test
nu scripts/check-suite-parity.nu
nu scripts/check-migration-immutability.nu
nu scripts/check-migration-docs.nu --self-test
nu scripts/check-migration-docs.nu
nu scripts/check-build-flags.nu
nu scripts/check-dependency-audit.nu --self-test
nu scripts/check-dependency-audit.nu
cargo fmt --check
cargo clippy --all-targets -- --deny warnings
cargo clippy --all-targets --features server -- --deny warnings
cargo clippy --all-targets --features web --target wasm32-unknown-unknown -- --deny warnings
cargo build --all-targets
cargo build --all-targets --features server
cargo test --lib
cargo test --features server --lib
nu scripts/check-doc-tests-ran.nu --self-test
nu scripts/check-doc-tests-ran.nu
cargo test --features server --test db_schema
nu scripts/check-db-tests-ran.nu --self-test
nu scripts/check-db-tests-ran.nu
```

`just pre-commit` runs that whole list, static guards included, so there is nothing left to run by hand before pushing.

### Why the copies of the suite are guarded against each other

The `pre-commit` recipe in `justfile` and the steps in `.forgejo/workflows/check.yml` are two hand-maintained copies of one check suite, and until LINKS-49 nothing compared them. The `check` recipe is a third copy of the lint legs, which LINKS-42 grew into the same three clippy configurations CI runs, and which was outside the guard until LINKS-64. `scripts/check-suite-parity.nu` covers all three, as a static guard needing no compiler:

- It reads the cargo invocations out of the `pre-commit` recipe body and out of every `run:` step in the workflow. The justfile's `docker compose ... app` prefix is stripped at the `cargo` token, and lines that are neither a cargo invocation nor a `nu scripts/*.nu` call (`print`, the `CARGO_BUILD_JOBS` export, the stylesheet placeholder) are ignored. Quoted runs stay one token, so a `print "... cargo clippy ..."` line is not mistaken for a leg.
- It normalises each invocation into a canonical leg: subcommand, a sorted feature set, the target, the remaining flags sorted, and the lint flags behind `--`. `-D warnings` and `--deny warnings` are the same token, as are `--features=web` and `--features web`, so each file keeps its own local style and only a real difference reads as drift. Flag order is not drift either.
- Guard-script calls compare on the script name and whether `--self-test` was passed. The recipe's `--runner` argument is plumbing (locally cargo runs inside the compose container, in CI on the runner), not part of the leg.
- The comparison between `pre-commit` and the workflow runs in both directions: a leg or guard in the recipe but not the workflow fails, and a leg or guard in the workflow but not the recipe fails.
- `check` is compared one way, because it deliberately omits the build, test, doc-test and database legs that need the compose stack: every clippy configuration and the fmt leg the workflow runs must appear in `check`, and nothing demands the reverse. A two-way comparison would demand that `check` grow the legs it exists to leave out. Its body is empty (`check: check-web check-clippy check-fmt`), so the guard resolves a recipe's dependencies the way `just --dry-run check` expands them, and a dependency naming a recipe that no longer exists is reported rather than skipped. Before LINKS-64 the recipe was outside the guard entirely: deleting `cargo clippy --all-targets --features server -- --deny warnings` from `check-clippy` left both this guard and `scripts/check-build-flags.nu` at exit 0.
- It also enforces two per-source invariants, which are the drift that survives being applied to every copy at once, and which `check` is held to as well: every clippy leg must pass `--deny warnings`, and the clippy legs together must cover all three compilation configurations (default, `--features server`, and `--features web --target wasm32-unknown-unknown`). That last one is the LINKS-39 hazard: `.cargo/config.toml` pins `[build] target = "x86_64-unknown-linux-gnu"`, so a wasm leg that lost `--target` still runs, still exits 0, and quietly re-lints the host build two other legs already cover.
- Floors on the parsed counts mean a parser that stopped matching fails rather than comparing an empty set to an empty set, and `--self-test` runs first at both call sites, feeding a synthetic justfile and workflow through the real parser to prove a dropped `--target`, a dropped `--deny warnings`, a one-sided leg, a one-sided guard and an unparseable file are all still rejected. The `check` cases assert not only that the drift is rejected but that the rule aimed at it is the one that fired, so a fixture caught by some other comparison first cannot leave its own invariant unreached.

### Why the test legs are scripts and not a bare `cargo test`

`cargo test` exits 0 when a target has no tests, when every case is filtered out by a name argument, and when every case is `#[ignore]`d. That is how the `tests/` targets sat compiled but unexecuted while every check stayed green (LINKS-44), and how all ten doc examples rotted into compile errors without failing anything (LINKS-48). Each guard runs its leg, parses the harness summary, and fails on a run that proves nothing. `--self-test` runs first in both, so a guard that stopped detecting fails the job instead of passing vacuously.

- `scripts/check-db-tests-ran.nu` runs each `tests/*.rs` target and fails when a `db_*` target is missing, ignored, filtered, or below the floor on database-backed passes.
- `scripts/check-doc-tests-ran.nu` runs `cargo test --features server --doc` and fails when the harness collected nothing, when anything is ignored or filtered out, or when passes fall below the floor. The server leg is the one that runs it: `default = []` cfgs out every module holding an example, so the default-feature leg would collect zero doc tests and still exit 0. An example that must not execute is marked ```` ```no_run ````, which rustdoc compiles and type-checks and reports as a pass; ```` ```ignore ```` is not compiled at all and fails the leg.

The three clippy legs need no such guard, and LINKS-39 checked before deciding rather than adding one for symmetry. `cargo clippy -- --deny warnings` has no green-but-empty state to detect: a feature name Cargo.toml does not define is a cargo error (and `scripts/check-build-flags.nu` rejects it in CI before any cargo runs), and a `--target` the toolchain lacks is a hard `E0463: can't find crate for core`, verified against an uninstalled `wasm32-wasip1` at exit 101. The failure the wasm leg really had was the opposite of vacuity: it ran, it printed its findings, and `cargo check` with no `--deny warnings` exited 0 anyway. The residual risk is one level up, in a later edit that drops `--target` or `--deny warnings` from one of the copies of the suite. `scripts/check-suite-parity.nu` covers that risk instead of a per-leg runtime guard (LINKS-49, LINKS-64); see "Why the copies of the suite are guarded against each other" above.

---

## Manual Testing

### Authentication Flow

- [ ] First-time setup creates user successfully
- [ ] Login with correct credentials succeeds
- [ ] Login with incorrect credentials fails
- [ ] Logout clears session
- [ ] Protected routes require authentication
- [ ] Session persists across page reloads
- [ ] Session expires after timeout (if implemented)

### Link Management

- [ ] Create link with URL only (auto-extracts metadata)
- [ ] Create link with custom title/description
- [ ] Create GitHub repository link (fetches stars, language)
- [ ] Update link details
- [ ] Delete link
- [ ] Refresh metadata
- [ ] Refresh GitHub metadata
- [ ] Add/remove categories
- [ ] Add/remove tags
- [ ] Add/remove languages
- [ ] Add/remove licenses

### Search and Filtering

- [ ] Search by title
- [ ] Search by description
- [ ] Search by URL
- [ ] Filter by category
- [ ] Filter by tag
- [ ] Filter by language
- [ ] Filter by license
- [ ] Sort by creation date
- [ ] Sort by update date
- [ ] Sort by title
- [ ] Pagination works correctly

### Categories

- [ ] Create root category (level 1)
- [ ] Create child category (level 2)
- [ ] Create grandchild category (level 3)
- [ ] Cannot create level 4 (validation)
- [ ] Update category name
- [ ] Delete category (removes from links)
- [ ] View category tree

### Bulk Operations

- [ ] Bulk delete multiple links
- [ ] Bulk assign categories
- [ ] Bulk assign tags
- [ ] Confirm dialogs work
- [ ] Success/error messages display

### Import/Export

- [ ] Export all links to JSON
- [ ] Export preserves metadata
- [ ] Import from JSON
- [ ] Import creates categories/tags
- [ ] Import handles duplicates

### Responsive Design

- [ ] Layout works on mobile (320px+)
- [ ] Layout works on tablet (768px+)
- [ ] Layout works on desktop (1024px+)
- [ ] Touch interactions work
- [ ] Navigation is accessible

### Performance

- [ ] Page loads in < 2 seconds
- [ ] Search responds instantly
- [ ] No memory leaks (long sessions)
- [ ] Database queries are optimized
- [ ] Large datasets render efficiently

---

## Performance Testing

### Load Testing

Use tools like `wrk` or `hey` for load testing:

```bash
# Install wrk
# macOS: brew install wrk
# Linux: apt-get install wrk

# Test health endpoint
wrk -t4 -c100 -d30s http://localhost:8080/api/health

# Test authenticated endpoint (with cookie)
wrk -t4 -c100 -d30s -H "Cookie: session_id=TOKEN" \
  http://localhost:8080/api/links
```

### Benchmarking

```rust
// Use criterion for benchmarks
// benches/link_benchmark.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_parse_url(c: &mut Criterion) {
    c.bench_function("parse_url", |b| {
        b.iter(|| parse_url(black_box("https://example.com/path")))
    });
}

criterion_group!(benches, benchmark_parse_url);
criterion_main!(benches);
```

Run benchmarks:

```bash
cargo bench
```

### Database Performance

```sql
-- Slow query logging
ALTER DATABASE rustylinks SET log_min_duration_statement = 100;

-- Check query performance
EXPLAIN ANALYZE
SELECT * FROM links WHERE user_id = 'uuid' ORDER BY created_at DESC LIMIT 50;

-- Index usage
SELECT indexrelname, idx_scan
FROM pg_stat_user_indexes
WHERE schemaname = 'public'
ORDER BY idx_scan DESC;
```

---

## Troubleshooting

### Tests Failing

#### Database Connection Issues

```bash
# Check database is running
psql -U rustylinks -d rustylinks_test -c "SELECT 1"

# Verify DATABASE_URL
echo $DATABASE_URL

# Run migrations
DATABASE_URL=postgresql://rustylinks:password@localhost/rustylinks_test sqlx migrate run
```

#### Flaky Tests

```bash
# Run test multiple times
for i in {1..10}; do cargo test --features server test_name || break; done

# Run with single thread
cargo test --features server -- --test-threads=1

# Add sleep/retry in test code
tokio::time::sleep(Duration::from_millis(100)).await;
```

#### Out of Memory

```bash
# Limit parallel tests
cargo test --features server -- --test-threads=2

# Increase test timeout
cargo test --features server -- --nocapture --test-threads=1
```

### CI/CD Issues

#### Environment Variables Not Set

`.forgejo/workflows/check.yml` sets `DATABASE_URL` at job level:
```yaml
env:
  DATABASE_URL: postgres://rustylinks:rustylinks@postgres:5432/rustylinks
```

The host is the service alias `postgres`, not `localhost`. The service declares no published port on purpose: the runner puts the job container and the service on one network, so the alias is the address, while a published port lands in the docker daemon's namespace where the job cannot reach it.

#### Service Container Not Ready

The `postgres` service already declares a health check; a job that starts before the database is up means the check changed:
```yaml
services:
  postgres:
    options: >-
      --health-cmd="pg_isready --username=rustylinks --dbname=rustylinks"
      --health-interval=5s
      --health-timeout=3s
      --health-retries=10
```

#### Build Cache Issues

The job caches the Rust build with `Swatinem/rust-cache`, including on failure:

```yaml
- name: Cache Rust build
  uses: https://github.com/Swatinem/rust-cache@v2
  with:
    cache-on-failure: true
```

A stale cache is cleared by bumping the action's cache key or re-running the job with the cache disabled, not by adding a `cargo clean` step, which would defeat the cache on every run.

---

## Best Practices Summary

### Do's ✅

- Write tests before fixing bugs (TDD)
- Test edge cases and error conditions
- Use descriptive test names
- Keep tests simple and focused
- Mock external dependencies
- Clean up test data
- Run tests before committing
- Maintain test documentation
- Cover the critical paths a change touches

### Don'ts ❌

- Don't test implementation details
- Don't write flaky tests
- Don't skip cleanup
- Don't commit broken tests
- Don't test third-party libraries
- Don't write tests without assertions
- Don't ignore failing tests
- Don't test everything (focus on value)

---

## References

- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [tokio Testing](https://tokio.rs/tokio/topics/testing)
- [SQLx Testing](https://docs.rs/sqlx/latest/sqlx/testing/index.html)
- [Axum Testing](https://github.com/tokio-rs/axum/tree/main/examples/testing)

---

## Contributing

When contributing tests:

1. Follow existing test patterns
2. Add tests for new features
3. Update this documentation
4. Ensure CI passes
5. Cover the paths the change touches; see [Test Coverage](#test-coverage) for why no number is enforced

See [CONTRIBUTING.md](../CONTRIBUTING.md) for more details.
