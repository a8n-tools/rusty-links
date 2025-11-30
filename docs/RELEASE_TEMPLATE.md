<!--
GitHub Release Template for Rusty Links

Instructions:
1. Update the version number in the title
2. Fill in the release date
3. Update the highlights section with key features
4. Add/remove sections as needed (Features, Bug Fixes, Breaking Changes)
5. Update installation instructions if changed
6. List any known issues
7. Thank contributors

For major releases (1.0, 2.0), include more comprehensive notes.
For minor releases (1.1, 1.2), focus on new features.
For patch releases (1.0.1, 1.0.2), focus on bug fixes.
-->

# Rusty Links v1.0.0 - Initial Release

**Release Date:** 2025-01-XX

A self-hosted bookmark manager built with Rust, featuring powerful organization tools, GitHub integration, and automated metadata extraction.

---

## Highlights

- **Single-User Authentication** - Secure session-based auth with Argon2 password hashing
- **GitHub Integration** - Automatic metadata fetching for repositories (stars, language, license)
- **Powerful Organization** - Hierarchical categories (3 levels), custom tags, languages, and licenses
- **Smart Search** - Full-text search with advanced filtering and sorting
- **Background Updates** - Automated metadata refresh and link accessibility checks
- **Docker Ready** - One-command deployment with Docker Compose
- **Complete Documentation** - Comprehensive guides for API, deployment, security, and testing

---

## What's New

### Features

#### Core Functionality
- Session-based authentication with Argon2id password hashing
- Full CRUD operations for bookmarks with automatic metadata extraction
- GitHub repository integration (stars, language, license, last commit)
- Duplicate URL detection per user
- Link status tracking (active, archived, inaccessible)
- Bulk operations (delete, assign categories/tags)
- JSON import/export functionality

#### Organization System
- **Hierarchical Categories** - Up to 3 levels deep with cascade delete
- **Programming Languages** - 20+ pre-seeded global languages + user-specific custom languages
- **Software Licenses** - 20+ pre-seeded licenses (MIT, Apache-2.0, GPL, etc.)
- **Custom Tags** - User-defined tags with many-to-many relationships
- **Many-to-Many** - Links can have multiple categories, languages, licenses, and tags

#### Search & Discovery
- Full-text search across titles, descriptions, and URLs (case-insensitive)
- Advanced filtering by category, language, license, and tags
- Multiple sort options (date, title, GitHub stars)
- Pagination with configurable limits (default 50, max 100)

#### Background Jobs
- Automated metadata update scheduler with configurable intervals
- Link accessibility verification
- GitHub metadata refresh for repositories
- Consecutive failure tracking
- Graceful shutdown handling

#### API
- 40+ REST endpoints for complete programmatic control
- Authentication endpoints (`/api/auth/*`)
- Link management (`/api/links/*`)
- Organization endpoints (`/api/categories/*`, `/api/tags/*`, `/api/languages/*`, `/api/licenses/*`)
- Health checks (`/api/health/*`)
- Complete API documentation with curl examples

#### Deployment
- Multi-stage Dockerfile for optimized builds (< 150MB)
- Docker Compose configuration for one-command deployment
- Development mode with hot reloading
- CI/CD with GitHub Actions (testing, coverage, security audits)
- Multi-platform Docker images (AMD64, ARM64)
- Automatic publishing to GitHub Container Registry

#### Documentation
- **API.md** - Complete REST API reference with examples
- **DATABASE.md** - Schema documentation with ER diagram
- **DEPLOYMENT.md** - Production deployment guide
- **SECURITY.md** - Security features and hardening checklist
- **TESTING.md** - Testing strategy and guidelines
- **DOCKER.md** - Docker deployment guide
- **RELEASE.md** - Release process documentation
- **LAUNCH_CHECKLIST.md** - Comprehensive pre-launch checklist

#### Security
- Argon2id password hashing with configurable parameters
- HttpOnly, Secure, SameSite=Lax cookies
- Parameterized SQL queries (injection prevention)
- Input validation and sanitization
- No sensitive data logging
- Non-root Docker container
- Security headers (HSTS, CSP, X-Frame-Options)

### Bug Fixes

N/A - Initial release

### Breaking Changes

N/A - Initial release

---

## Installation

### Quick Start with Docker

```bash
# Pull the image
docker pull ghcr.io/NiceGuyIT/rusty-links:1.0.0

# Create .env file
cat > .env <<EOF
DB_PASSWORD=$(openssl rand -base64 32)
APP_PORT=8080
RUST_LOG=warn
EOF

# Start services
docker compose up -d

# Access at http://localhost:8080
```

### Using Docker Compose

```yaml
version: '3.8'

services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: rustylinks
      POSTGRES_PASSWORD: ${DB_PASSWORD}
      POSTGRES_DB: rustylinks
    volumes:
      - postgres_data:/var/lib/postgresql/data

  app:
    image: ghcr.io/NiceGuyIT/rusty-links:1.0.0
    depends_on:
      - postgres
    environment:
      DATABASE_URL: postgres://rustylinks:${DB_PASSWORD}@postgres:5432/rustylinks
      APP_PORT: 8080
    ports:
      - "8080:8080"

volumes:
  postgres_data:
```

### Building from Source

```bash
# Clone repository
git clone https://github.com/NiceGuyIT/rusty-links.git
cd rusty-links

# Copy environment template
cp .env.example .env

# Edit .env with your configuration
nano .env

# Build and run with Docker Compose
docker compose up -d

# Or build with Cargo (requires Rust 1.75+)
cargo build --release
./target/release/rusty-links
```

---

## Upgrading

This is the initial release. For future upgrades, see [UPGRADING.md](docs/UPGRADING.md).

---

## Configuration

Key environment variables:

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `DATABASE_URL` | PostgreSQL connection string | - | Yes |
| `APP_PORT` | Application port | 8080 | No |
| `UPDATE_INTERVAL_DAYS` | Metadata update interval (days) | 30 | No |
| `UPDATE_INTERVAL_HOURS` | Metadata update interval (hours) | 24 | No |
| `BATCH_SIZE` | Update batch size | 50 | No |
| `JITTER_PERCENT` | Scheduler jitter percentage | 20 | No |
| `RUST_LOG` | Logging level | info | No |
| `GITHUB_TOKEN` | GitHub API token (optional) | - | No |

See [Configuration Guide](README.md#configuration) for complete details.

---

## Documentation

- [README](README.md) - Project overview and quick start
- [API Documentation](docs/API.md) - Complete REST API reference
- [Database Schema](docs/DATABASE.md) - Schema and migration guide
- [Deployment Guide](docs/DEPLOYMENT.md) - Production deployment
- [Security Guide](docs/SECURITY.md) - Security features and hardening
- [Testing Guide](docs/TESTING.md) - Testing strategy
- [Docker Guide](docs/DOCKER.md) - Docker deployment
- [Release Process](docs/RELEASE.md) - How releases are created
- [Contributing Guide](CONTRIBUTING.md) - Contribution guidelines

---

## Known Issues

None at this time. Please report issues at: https://github.com/NiceGuyIT/rusty-links/issues

---

## Performance

- **Container Size:** < 150MB (optimized multi-stage build)
- **Memory Usage:** ~256MB typical, 512MB limit
- **Database:** PostgreSQL 16+ with optimized indexes
- **Response Times:** < 100ms for most operations
- **Scalability:** Tested with 10,000+ links

---

## Security Notes

- All passwords hashed with Argon2id (memory-hard, ASIC-resistant)
- Session tokens generated with cryptographically secure RNG (32 bytes)
- HttpOnly, Secure, SameSite cookies prevent XSS/CSRF attacks
- Parameterized SQL queries prevent injection attacks
- Input validation on all user inputs
- Non-root Docker container for defense in depth

**Security Disclosure:** Report vulnerabilities to [SECURITY.md](docs/SECURITY.md#reporting-a-vulnerability)

---

## Roadmap (Phase 2)

Planned features for future releases:

- Multi-user support with role-based access control
- Public/private link sharing
- Dark mode theme
- Browser extension for quick bookmarking
- Bulk import from browsers (Chrome, Firefox)
- OAuth authentication (GitHub, Google)
- Mobile progressive web app (PWA)
- API rate limiting
- Advanced analytics and statistics
- Link collections/playlists
- Web clipper for saving pages
- Full-text content indexing

See [CHANGELOG.md](CHANGELOG.md) for the complete roadmap.

---

## Contributors

Thank you to everyone who contributed to this release!

- [@NiceGuyIT](https://github.com/NiceGuyIT) - Project creator and maintainer

Want to contribute? See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## Support

- **Documentation:** https://github.com/NiceGuyIT/rusty-links/tree/main/docs
- **Issues:** https://github.com/NiceGuyIT/rusty-links/issues
- **Discussions:** https://github.com/NiceGuyIT/rusty-links/discussions

---

## License

Released under the [MIT License](LICENSE). Copyright 2025.

---

## Acknowledgments

Built with:

- [Rust](https://www.rust-lang.org/) - Systems programming language
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [Dioxus](https://dioxuslabs.com/) - Reactive UI framework
- [PostgreSQL](https://www.postgresql.org/) - Database
- [SQLx](https://github.com/launchbadge/sqlx) - SQL toolkit
- [Tokio](https://tokio.rs/) - Async runtime

Special thanks to the Rust community for their excellent tools and libraries.

---

## Full Changelog

See [CHANGELOG.md](CHANGELOG.md) for complete version history.

**Compare:** [v0.0.0...v1.0.0](https://github.com/NiceGuyIT/rusty-links/compare/v0.0.0...v1.0.0)

---

**Download:**
- Docker: `docker pull ghcr.io/NiceGuyIT/rusty-links:1.0.0`
- Source: [rusty-links-1.0.0.tar.gz](https://github.com/NiceGuyIT/rusty-links/archive/refs/tags/v1.0.0.tar.gz)

**Checksums:** (Add SHA256 checksums if distributing binaries)

---

**Star** this repository if you find it useful! ⭐
