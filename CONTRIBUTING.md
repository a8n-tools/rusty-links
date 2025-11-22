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

- Rust 1.75 or later installed
- PostgreSQL 14+ running locally
- Node.js 18+ (for Dioxus CLI)
- Git for version control
- A GitHub account

### Finding Issues to Work On

1. Check the [Issues](https://github.com/YOUR-USERNAME/rusty-links/issues) page
2. Look for issues labeled `good first issue` for beginner-friendly tasks
3. Check for issues labeled `help wanted` if you're more experienced
4. Comment on the issue to let others know you're working on it

## Development Setup

1. **Fork the repository**
   ```bash
   # Click "Fork" on GitHub, then clone your fork
   git clone https://github.com/YOUR-USERNAME/rusty-links.git
   cd rusty-links
   ```

2. **Add upstream remote**
   ```bash
   git remote add upstream https://github.com/ORIGINAL-OWNER/rusty-links.git
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
   cp .env.example .env
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
   ```bash
   # Run all tests
   cargo test

   # Run specific test
   cargo test test_name

   # Run with output
   cargo test -- --nocapture
   ```

3. **Check code quality**
   ```bash
   # Format code
   cargo fmt

   # Run linter
   cargo clippy

   # Check for errors without building
   cargo check
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
   - Go to your fork on GitHub
   - Click "New Pull Request"
   - Select your feature branch
   - Fill out the PR template

3. **PR Description Should Include**
   - What changes were made
   - Why these changes are necessary
   - Any related issue numbers (e.g., "Fixes #123")
   - Screenshots (if UI changes)
   - Testing instructions

### Pull Request Checklist

Before submitting, ensure:

- [ ] Code follows project coding standards
- [ ] All tests pass (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
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
- Run `cargo clippy` and address warnings
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

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run integration tests only
cargo test --test '*'

# Run doc tests
cargo test --doc
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

# Revert last migration
sqlx migrate revert
```

### Migration Guidelines

- Keep migrations atomic and reversible
- Test both up and down migrations
- Document complex migrations
- Never modify existing migrations
- Include sample data in comments

## Release Process

See [RELEASE.md](docs/RELEASE.md) for the complete release process.

For maintainers:
1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create and push tag
4. GitHub Actions will build and publish

## Community

### Getting Help

- **GitHub Issues**: Bug reports and feature requests
- **Discussions**: Questions and general discussion
- **Pull Requests**: Code review and collaboration

### Recognition

Contributors are recognized in:
- GitHub contributors list
- Release notes
- Project credits

Thank you for contributing to Rusty Links!
