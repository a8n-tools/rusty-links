# Contributing to Rusty Links

Thank you for your interest in contributing to Rusty Links! This document provides guidelines and instructions for contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Submitting Changes](#submitting-changes)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Documentation](#documentation)
- [Community](#community)

## Code of Conduct

This project adheres to a code of conduct that all contributors are expected to follow:

- Be respectful and inclusive
- Welcome newcomers and help them get started
- Accept constructive criticism gracefully
- Focus on what is best for the community
- Show empathy towards other community members

## Getting Started

### Prerequisites

Before you begin, ensure you have:

- Rust (latest stable recommended)
- PostgreSQL 17+ running locally
- Dioxus CLI (`cargo install dioxus-cli` or `cargo binstall dioxus-cli`)
- Git for version control

### Finding Issues to Work On

1. Check the [Issues](https://dev.a8n.run/a8n-tools/rusty-links/issues) page
2. Look for issues labeled `good first issue` for beginner-friendly tasks
3. Check for issues labeled `help wanted` if you're more experienced
4. Comment on the issue to let others know you're working on it

## Development Setup

1. **Fork the repository**
   ```bash
   # Click "Fork" on dev.a8n.run, then clone your fork
   git clone https://dev.a8n.run/a8n-tools/rusty-links.git
   cd rusty-links
   ```

2. **Add upstream remote**
   ```bash
   git remote add upstream https://dev.a8n.run/a8n-tools/rusty-links.git
   ```

3. **Install dependencies**
   ```bash
   # Install Dioxus CLI
   cargo install dioxus-cli

   # Install SQLx CLI
   cargo install sqlx-cli --no-default-features --features postgres
   ```

4. **Set up database**
   ```bash
   # Create database
   createdb rustylinks

   # Copy environment template
   cp .env.standalone.example .env
   # Edit .env with your database credentials

   # Run migrations
   sqlx migrate run
   ```

5. **Run development server**
   ```bash
   dx serve
   ```

## Making Changes

### Creating a Branch

Always create a new branch for your changes:

```bash
# Update your fork
git fetch upstream
git checkout main
git merge upstream/main

# Create a feature branch
git checkout -b feature/your-feature-name
# or
git checkout -b fix/your-bug-fix
```

Branch naming conventions:
- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation changes
- `refactor/` - Code refactoring
- `test/` - Adding or updating tests

### Development Workflow

1. **Make your changes**
   - Write clean, readable code
   - Follow the coding standards (see below)
   - Add tests for new functionality
   - Update documentation as needed

2. **Test your changes**

   `default = []` and every server module is behind `#[cfg(feature = "server")]`, so a bare `cargo test` compiles almost none of the crate. Pass `--features server` to exercise the server-side suite.

   ```bash
   # Run the server-side unit tests (the bulk of the suite)
   cargo test --features server --lib

   # Run the default-feature tests (feature-independent code only)
   cargo test

   # Run the doc examples with the vacuity guard
   just test-doc

   # Run the tests/ targets against the compose postgres, with the skip guard
   just test-db

   # Run specific test
   cargo test --features server test_name

   # Run with output
   cargo test --features server -- --nocapture
   ```

3. **Check code quality**
   ```bash
   # Run every check CI runs, in the dev container, failing on the first red leg
   just pre-commit

   # Or run the individual legs
   cargo fmt
   cargo clippy --all-targets -- --deny warnings
   cargo clippy --all-targets --features server -- --deny warnings
   cargo clippy --all-targets --features web --target wasm32-unknown-unknown -- --deny warnings

   # Guard run by CI: every `--features` name the justfile passes exists in Cargo.toml
   # and every `--build-arg` is declared by a Dockerfile
   nu scripts/check-build-flags.nu
   ```

4. **Commit your changes**
   ```bash
   git add .
   git commit -m "feat: add amazing feature"
   ```

### Commit Message Guidelines

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

Examples:
```
feat(auth): add password reset functionality
fix(api): resolve link deletion bug
docs(readme): update installation instructions
refactor(db): optimize query performance
test(links): add integration tests for link creation
```

## Submitting Changes

### Creating a Pull Request

1. **Push your changes**
   ```bash
   git push origin feature/your-feature-name
   ```

2. **Open a Pull Request**
   - From the terminal: `fj --host dev.a8n.run pr create "<title>" --body-file <file>`
   - Or open your fork on dev.a8n.run and click "New Pull Request"
   - Select your feature branch
   - Describe the change

3. **PR Description Should Include**
   - What changes were made
   - Why these changes are necessary
   - Any related issue numbers (e.g., "Fixes #123")
   - Screenshots (if UI changes)
   - Testing instructions

### Pull Request Checklist

Before submitting, ensure:

- [ ] Code follows project coding standards
- [ ] `just pre-commit` passes (fmt, clippy under default features, `--features server` and `--features web` on wasm, build and tests under both feature sets, the doc examples, and the Postgres-backed `tests/` targets)
- [ ] Server-side tests pass (`cargo test --features server --lib`)
- [ ] Any new SQL is covered by a `tests/db_*.rs` case, since a query with no database-backed test is only compile-checked
- [ ] Any new doc example compiles (`just test-doc`); mark one that must not execute ```` ```no_run ````, never ```` ```ignore ````, which is not compiled at all and fails the leg
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings under any of the three configurations: `cargo clippy --all-targets -- --deny warnings`, the same with `--features server`, and `cargo clippy --all-targets --features web --target wasm32-unknown-unknown -- --deny warnings`
- [ ] Documentation is updated
- [ ] Commit messages follow conventions
- [ ] PR description is clear and complete
- [ ] Branch is up to date with main

### Review Process

1. Maintainers will review your PR
2. Address any requested changes
3. Once approved, your PR will be merged
4. Your contribution will be credited

## Coding Standards

### Rust Style Guide

Follow the official [Rust Style Guide](https://doc.rust-lang.org/stable/style-guide/):

- Use `cargo fmt` to format code
- Run `cargo clippy --all-targets --features server -- --deny warnings` (as well as the default-feature leg and the web/wasm leg) and address warnings
- Browser-only code under `#[cfg(target_arch = "wasm32")]` is compiled by the web/wasm leg alone, so lint it with `just check-web`
- Use meaningful variable and function names
- Add comments for complex logic
- Keep functions focused and small
- Use Result and Option types appropriately

### Code Organization

```rust
// 1. Imports (grouped)
use std::collections::HashMap;

use axum::Router;
use sqlx::PgPool;

use crate::models::User;

// 2. Constants
const MAX_LINKS: usize = 1000;

// 3. Type definitions
type Result<T> = std::result::Result<T, AppError>;

// 4. Structs and implementations
pub struct LinkService {
    pool: PgPool,
}

impl LinkService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// 5. Functions
pub async fn create_link(/* ... */) -> Result<Link> {
    // Implementation
}
```

### Error Handling

Always use proper error handling:

```rust
// Good
let user = db::get_user(id).await?;

// Good - with context
let user = db::get_user(id)
    .await
    .map_err(|e| AppError::NotFound(format!("User {}: {}", id, e)))?;

// Avoid
let user = db::get_user(id).await.unwrap();
```

### Documentation

Add doc comments for public APIs:

```rust
/// Creates a new link with the given URL
///
/// # Arguments
///
/// * `url` - The URL to create a link for
/// * `pool` - Database connection pool
///
/// # Returns
///
/// Returns the created `Link` or an `AppError`
///
/// # Errors
///
/// Returns `AppError::Validation` if URL is invalid
/// Returns `AppError::Database` if database operation fails
pub async fn create_link(url: &str, pool: &PgPool) -> Result<Link> {
    // Implementation
}
```

## Testing

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_validation() {
        let valid_url = "https://example.com";
        assert!(validate_url(valid_url).is_ok());
    }

    #[tokio::test]
    async fn test_create_link() {
        let pool = setup_test_db().await;
        let link = create_link("https://example.com", &pool).await;
        assert!(link.is_ok());
    }
}
```

### Running Tests

Server code needs `--features server`; without it those modules are not compiled and their tests do not run.

```bash
# Run the server-side unit tests
cargo test --features server --lib

# Run the default-feature tests
cargo test

# Run with output
cargo test --features server -- --nocapture

# Run specific test
cargo test --features server test_name

# Run integration tests only. The db_* targets need DATABASE_URL and panic without
# it: a suite that skips still exits 0, which is what left every query in this
# repo unexecuted until LINKS-44. `just test-db` runs them the way CI does.
cargo test --features server --test '*'

# Run the doc examples. `just test-doc` wraps this with the guard that fails on an
# empty collection: nothing in the repo compiled an example until LINKS-48, and
# without --features server every documented module is cfg'd out, so the run
# collects nothing and still exits 0.
cargo test --features server --doc
```

## Documentation

### Types of Documentation

1. **Code Comments**
   - Explain why, not what
   - Document complex algorithms
   - Add TODOs with issue numbers

2. **Doc Comments**
   - Use `///` for public APIs
   - Include examples when helpful
   - Document errors and panics
   - Examples are compiled and run by `just test-doc`, so an example needs its own `use rusty_links::...;` lines. One that must not execute (network or database I/O) is marked ```` ```no_run ````, which still compiles and type-checks it.

3. **README and Guides**
   - Keep README up to date
   - Add guides to `docs/` directory
   - Include examples and screenshots

### Building Documentation

```bash
# Generate and open documentation
cargo doc --open

# Include private items
cargo doc --document-private-items --open
```

## Database Changes

### Creating Migrations

```bash
# Create new migration
sqlx migrate add descriptive_name

# Edit migration files in migrations/
# - XXXXXX_descriptive_name.up.sql (apply migration)
# - XXXXXX_descriptive_name.down.sql (revert migration)

# Run migrations
sqlx migrate run
```

### Migration Guidelines

- Keep migrations atomic and focused
- Migrations here are simple (one `.sql` per version), not reversible: there is no `.down.sql` and `sqlx migrate revert` does not apply
- Never modify, rename or delete a migration that has merged to `main`; `scripts/check-migration-immutability.nu` fails the PR if you do
- Add a row for the new migration to the Migration History table in [docs/DATABASE.md](docs/DATABASE.md); `scripts/check-migration-docs.nu` fails the PR if you do not
- Document complex migrations in the file's leading comment

## Release Process

See [RELEASE.md](docs/RELEASE.md) for the complete release process.

For maintainers:
1. Run `just create-release <major|minor|hotfix>`, which bumps `Cargo.toml`, pushes a `release/vX.Y.Z` branch and opens the release PR
2. Update `CHANGELOG.md` on that branch if the release needs a hand-written entry
3. Merge the PR
4. `.forgejo/workflows/create-release.yml` creates the tag and the release, and `.forgejo/workflows/build-oci-image.yml` builds and pushes the image

## Community

### Getting Help

- **Issues**: Bug reports and feature requests, at <https://dev.a8n.run/a8n-tools/rusty-links/issues>
- **Pull Requests**: Code review and collaboration

### Recognition

Contributors are recognized in:
- The repository's contributor list
- Release notes

Thank you for contributing to Rusty Links!
