# Changelog

All notable changes to Rusty Links will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned for Phase 2
- Multi-user support with role-based access control
- Public/private link sharing
- Dark mode theme
- Browser extension for quick bookmarking
- Bulk import from browsers (Chrome, Firefox bookmarks)
- Bulk import from other services (Pocket, Raindrop, etc.)
- Advanced search with operators (AND, OR, NOT)
- Saved searches
- OAuth authentication (GitHub, Google)
- Mobile progressive web app (PWA)
- API rate limiting
- Advanced analytics and statistics
- Link collections/playlists
- Collaborative bookmarking
- Comments on links
- Link annotations
- Web clipper for saving pages
- Full-text content indexing
- Duplicate link detection across users
- Link preview generation
- Automatic link categorization with AI
- Browser history import

## [1.0.0] - 2025-01-XX

### Added

#### Core Features
- **Authentication System**
  - Single-user authentication for Phase 1
  - Argon2 password hashing (memory-hard, ASIC-resistant)
  - Session-based authentication with HTTP-only cookies
  - Secure session management with database-backed storage
  - Session token generation using cryptographically secure RNG
  - Initial setup endpoint (`/api/auth/setup`) for first user creation
  - Setup endpoint automatically disabled after first user
  - Login/logout functionality with proper cookie management

- **Link Management**
  - Full CRUD operations for bookmarks (Create, Read, Update, Delete)
  - Automatic metadata extraction from URLs (title, description, favicon)
  - GitHub repository integration with automatic metadata fetching:
    - Repository stars count
    - Primary programming language
    - License detection
    - Last commit date
    - Repository archived status
  - Duplicate URL detection per user
  - Link status tracking (active, archived, inaccessible, repo_unavailable)
  - Manual metadata refresh for individual links
  - Manual GitHub metadata refresh
  - Bulk link operations (delete, assign categories, assign tags)
  - Link export to JSON format
  - Link import from JSON format with duplicate handling

- **Organization System**
  - **Hierarchical Categories** (up to 3 levels deep)
    - Root categories (level 1)
    - Child categories (level 2)
    - Grandchild categories (level 3)
    - Parent-child relationships with cascade delete
    - Category tree view endpoint
    - Move categories by re-parenting
  - **Programming Languages**
    - 20+ pre-seeded global languages (JavaScript, Python, Rust, etc.)
    - User-specific custom languages
    - Cannot delete global languages
  - **Software Licenses**
    - 20+ pre-seeded common licenses (MIT, Apache-2.0, GPL, etc.)
    - User-specific custom licenses
    - SPDX license identifiers
  - **Custom Tags**
    - User-defined tags for flexible organization
    - Case-insensitive uniqueness per user
    - Many-to-many relationship with links
  - **Many-to-Many Relationships**
    - Links can have multiple categories
    - Links can have multiple languages (with ordering)
    - Links can have multiple licenses (with ordering)
    - Links can have multiple tags (with ordering)

- **Search & Filtering**
  - Full-text search across:
    - Link titles
    - Link descriptions
    - URLs
  - Case-insensitive search
  - Advanced filtering:
    - Filter by category (single selection)
    - Filter by language (multi-select with OR logic)
    - Filter by license (multi-select with OR logic)
    - Filter by tag (multi-select with OR logic)
    - Combine multiple filters (AND logic between types)
  - Sorting options:
    - Sort by creation date (ascending/descending)
    - Sort by update date
    - Sort by title (alphabetical)
    - Sort by GitHub stars (for repositories)
  - Pagination:
    - Configurable limit (default 50, max 100)
    - Offset-based pagination
    - Total count in response

- **Background Jobs & Scheduler**
  - Automated metadata update scheduler:
    - Configurable update interval (hours)
    - Configurable batch size for processing
    - Jitter percentage to distribute load
    - Last checked timestamp tracking
    - Consecutive failure tracking
    - Link accessibility verification
    - GitHub metadata refresh for repositories
  - Graceful shutdown handling
  - Health check endpoint for scheduler status
  - Scheduler runs in background without blocking

- **API Endpoints**
  - **Authentication**: `/api/auth/*`
    - `POST /api/auth/setup` - Create first user
    - `POST /api/auth/login` - Authenticate user
    - `POST /api/auth/logout` - End session
    - `GET /api/auth/me` - Get current user
    - `GET /api/auth/check-setup` - Check if setup required
  - **Links**: `/api/links/*`
    - `GET /api/links` - List with search/filter/pagination
    - `POST /api/links` - Create new link
    - `PUT /api/links/:id` - Update link
    - `DELETE /api/links/:id` - Delete link
    - `POST /api/links/:id/refresh` - Refresh metadata
    - `POST /api/links/:id/refresh-github` - Refresh GitHub data
    - `GET /api/links/export` - Export all links
    - `POST /api/links/import` - Import links
    - Bulk operations for delete, categories, tags
    - Link associations (categories, tags, languages, licenses)
  - **Categories**: `/api/categories/*`
    - CRUD operations
    - Tree view endpoint
  - **Tags**: `/api/tags/*`
    - CRUD operations
  - **Languages**: `/api/languages/*`
    - CRUD operations
  - **Licenses**: `/api/licenses/*`
    - CRUD operations
  - **Health**: `/api/health/*`
    - General health check
    - Database health check
    - Scheduler health check
  - **Scraping**: `/api/scrape`
    - Manual URL scraping

#### Technical Implementation

- **Backend Stack**
  - Rust 1.75+ with Axum web framework
  - PostgreSQL 16+ database with SQLx
  - Asynchronous runtime with Tokio
  - Structured logging with tracing crate
  - Session management with secure cookies (axum-extra)
  - Password hashing with argon2 crate

- **Database**
  - PostgreSQL with SQLx for compile-time query verification
  - UUID primary keys for all entities
  - Timestamp tracking (created_at, updated_at)
  - Cascade deletes for referential integrity
  - Indexes for query performance
  - Partial indexes for scheduler queries
  - Unique constraints (case-insensitive for names)
  - Self-referencing foreign keys (category hierarchy)
  - Junction tables for many-to-many relationships
  - Automatic migrations on startup
  - 5 migration files documenting schema evolution

- **Frontend**
  - Dioxus framework for reactive UI (fullstack mode)
  - Component-based architecture
  - Server-side rendering (SSR)
  - Real-time updates

- **External Integrations**
  - GitHub API for repository metadata
  - reqwest for HTTP requests
  - scraper crate for HTML parsing
  - Automatic favicon/logo extraction
  - Configurable GitHub token for higher rate limits

#### Deployment & DevOps

- **Docker Support**
  - Multi-stage Dockerfile for optimized builds
  - Production image < 150MB
  - debian:bookworm-slim base image
  - Non-root user (rustylinks, UID 1000)
  - Security hardening:
    - No new privileges
    - Minimal runtime dependencies
    - Health checks
    - Resource limits
  - Development Dockerfile with cargo-watch

- **Docker Compose**
  - One-command deployment (`docker compose up -d`)
  - PostgreSQL 16-alpine service container
  - Automatic health checks
  - Named volumes for data persistence
  - Isolated bridge network
  - Environment variable configuration
  - Development override (`compose.dev.yml`) with hot reloading

- **CI/CD**
  - GitHub Actions workflows:
    - Automated testing on push/PR
    - Code formatting checks (cargo fmt)
    - Linting with Clippy (cargo clippy)
    - Security audit (cargo audit)
    - Code coverage reporting (cargo-tarpaulin)
    - Dependency checks (outdated, unused)
    - PostgreSQL service container for tests
    - Build caching for faster runs
  - Docker image publishing:
    - Automatic builds on version tags
    - Multi-platform support (AMD64, ARM64)
    - GitHub Container Registry (ghcr.io)
    - Intelligent tagging (latest, semver patterns)
    - Build provenance attestations

- **Configuration**
  - Environment-based configuration
  - `.env.example` template provided
  - Configurable settings:
    - Database connection
    - Application port
    - Update intervals (days and hours)
    - Batch processing size
    - Scheduler jitter percentage
    - Logging level
    - Optional GitHub token

#### Documentation

- **User Documentation**
  - Comprehensive README with:
    - Project overview
    - Feature highlights
    - Quick start guide
    - Installation options (Docker, source)
    - Configuration reference
    - Development setup
    - Architecture overview
  - LICENSE (MIT)
  - CONTRIBUTING.md with guidelines

- **Technical Documentation**
  - **API Documentation** (`docs/API.md`):
    - Complete endpoint reference (40+ endpoints)
    - Request/response formats
    - Authentication flow
    - Query parameters
    - Error responses
    - 150+ curl examples
    - Data models
    - Best practices
  - **Database Documentation** (`docs/DATABASE.md`):
    - Complete schema reference
    - Entity-relationship diagram
    - All 11 tables documented
    - Migration history
    - Index strategies
    - Backup/restore procedures
    - Performance tuning
    - Maintenance tasks
  - **Security Documentation** (`docs/SECURITY.md`):
    - Implemented security features
    - Production hardening checklist
    - Reverse proxy configurations
    - Firewall setup
    - Vulnerability reporting process
    - Regular maintenance schedule
    - Best practices for users
  - **Testing Documentation** (`docs/TESTING.md`):
    - Testing strategy
    - Running tests
    - Writing tests (unit, integration)
    - Test coverage goals
    - Performance testing
    - Manual testing checklist
    - CI/CD testing
  - **Deployment Documentation** (`docs/DEPLOYMENT.md`):
    - Production deployment guide
    - Server setup
    - Reverse proxy configuration
    - SSL/TLS setup
    - Database configuration
    - Monitoring setup
    - Backup strategy
    - Troubleshooting
  - **Docker Documentation** (`docs/DOCKER.md`):
    - Docker deployment guide
    - Development mode
    - Production configuration
    - Database operations
    - Backup/restore
    - Troubleshooting
  - **Release Documentation** (`docs/RELEASE.md`):
    - Release process
    - Versioning strategy
    - Creating releases
    - Publishing packages
    - Package visibility
    - Tag strategy

- **Configuration Examples**
  - Nginx reverse proxy config (`examples/nginx.conf`)
  - Caddy reverse proxy config (`examples/Caddyfile`)
  - Environment template (`.env.example`)
  - Docker Compose production config

#### Security

- **Authentication Security**
  - Argon2id password hashing
  - Configurable Argon2 parameters
  - Constant-time password comparison
  - Session-based authentication (stateful)
  - Secure random session tokens (32 bytes)
  - HttpOnly cookies (XSS prevention)
  - Secure cookie flag (HTTPS only)
  - SameSite=Lax (CSRF protection)

- **Data Protection**
  - Parameterized SQL queries (injection prevention)
  - Input validation and sanitization
  - URL validation with url crate
  - Email format validation
  - No sensitive data logging (passwords, tokens)
  - Masked database URLs in logs
  - Password hash excluded from API responses

- **Container Security**
  - Non-root user execution
  - Minimal attack surface (slim base image)
  - No unnecessary capabilities
  - Security options (no-new-privileges)
  - Resource limits
  - Health checks

- **Dependency Security**
  - Cargo audit integration
  - Automated vulnerability scanning in CI
  - Regular dependency updates
  - Cargo.lock committed for reproducibility
  - Minimal dependency tree

- **Network Security**
  - HTTPS recommended (via reverse proxy)
  - Security headers (HSTS, CSP, X-Frame-Options)
  - Rate limiting (via reverse proxy)
  - CORS configuration

#### Testing

- **Unit Tests**
  - Co-located with source code
  - Test coverage in critical modules:
    - Config (password masking)
    - Auth/Session (token generation, cookies)
    - Models (user creation, validation)
  - Mock data and test utilities

- **Integration Tests**
  - Test utilities in `tests/common/mod.rs`
  - Database setup/cleanup helpers
  - Example tests provided
  - PostgreSQL test database support

- **CI Testing**
  - Automated on every push and PR
  - PostgreSQL service container
  - Migration verification
  - Code formatting checks
  - Linting with Clippy
  - Security audits

### Changed

- N/A (initial release)

### Deprecated

- N/A (initial release)

### Removed

- N/A (initial release)

### Fixed

- N/A (initial release)

### Security

- Argon2 password hashing implementation
- SQL injection prevention via parameterized queries
- XSS prevention via CSP and output encoding
- CSRF protection via SameSite cookies
- Session management security
- Input validation and sanitization
- Secure cookie settings
- Non-root Docker container
- Minimal attack surface

---

## Version History

### [1.0.0] - 2025-01-XX
- Initial public release
- Single-user bookmark management
- GitHub integration
- Docker deployment
- Complete documentation

---

## Links

- [Repository](https://github.com/YOUR-USERNAME/rusty-links)
- [Documentation](https://github.com/YOUR-USERNAME/rusty-links/tree/main/docs)
- [Issues](https://github.com/YOUR-USERNAME/rusty-links/issues)
- [Releases](https://github.com/YOUR-USERNAME/rusty-links/releases)

---

[Unreleased]: https://github.com/YOUR-USERNAME/rusty-links/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/YOUR-USERNAME/rusty-links/releases/tag/v1.0.0
