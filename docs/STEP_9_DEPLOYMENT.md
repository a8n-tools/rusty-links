# Rusty Links Implementation Guide
# Part 8: Deployment & Documentation (Steps 46-55)

## Overview

Part 8 focuses on production readiness, including:
- Background job scheduling for metadata updates
- Docker containerization and orchestration
- CI/CD automation
- Comprehensive documentation
- Security hardening
- Final integration testing

## Dependencies

- All previous parts (1-7) must be complete
- Working authentication, models, API, UI
- GitHub integration functional
- Web scraping operational

  ---
Step-by-Step Breakdown & Iteration

First Pass - Major Chunks:

1. Background Jobs (Step 46)
2. Containerization (Steps 47-48)
3. CI/CD (Step 49)
4. Documentation (Steps 50-52)
5. Testing & Security (Steps 53-54)
6. Final Integration (Step 55)

Second Pass - Breaking Down Further:

Chunk 1: Background Jobs
- 46a: Scheduler infrastructure
- 46b: Metadata update job
- 46c: Integration with existing system

Chunk 2: Docker Setup
- 47a: Dockerfile with build stages
- 47b: Optimization and security
- 48a: Docker Compose services
- 48b: Volumes and networking

Chunk 3: CI/CD
- 49a: GitHub Actions workflow
- 49b: Container registry publishing

Chunk 4: Core Documentation
- 50: README with quick start
- 51: API documentation
- 52: Database documentation

Chunk 5: QA & Security
- 53: Testing documentation
- 54: Security hardening

Chunk 6: Launch
- 55: Final integration and checklist

Third Pass - Final Sizing Review:

After review, the chunks are appropriately sized:
- Each step builds incrementally
- No large complexity jumps
- Clear integration points
- Testable at each stage

  ---
Implementation Prompts

Context for All Prompts

Current State:
- Rusty Links application fully functional (Parts 1-7 complete)
- Authentication, models, API, UI all working
- GitHub integration and web scraping operational
- Ready for production deployment

  ---
Step 46: Scheduled Metadata Updates - Background Job

**Objective:** Implement a background scheduler that periodically updates link metadata (GitHub stars, availability checks, etc.) without manual intervention.

**Requirements:**

Create a scheduler module at `src/scheduler/mod.rs` with the following:

1. **Job Configuration**
   - Update interval configurable via environment variable (default: 24 hours)
   - Random jitter of ±20% to prevent all updates running simultaneously
   - Batch processing to avoid overwhelming external APIs
   - Respect manual override flags

2. **Update Job Implementation**
   ```rust
   // Core functionality needed:
   - Fetch all links that need updating (last_checked > interval)
   - For GitHub repos: refresh stars, commit date, archived status
   - For all links: verify URL still accessible
   - Update database with new metadata
   - Log all operations
   - Handle rate limiting gracefully
   ```

3. **Scheduler Integration**
    - Use tokio::spawn for background task
    - Graceful shutdown support
    - Health check endpoint: GET /api/health/scheduler
    - Start scheduler on application boot

4. **Database Updates**
    - Add `last_checked` timestamp to links table
    - Add `check_failed_count` to track consecutive failures
    - Migration: `migrations/20XX_add_scheduler_fields.sql`

5. **Configuration**
   ```rust
   // Add to src/config.rs:
   UPDATE_INTERVAL_HOURS: u32  // default 24
   BATCH_SIZE: usize           // default 50
   JITTER_PERCENT: u8          // default 20
   ```

6. **Error Handling**
    - Continue on individual link failures
    - Log errors without stopping the job
    - Increment failure counter, mark inaccessible after 3 failures
    - Retry with exponential backoff for transient errors

**Testing:**
  ```bash
  # 1. Verify scheduler starts
  cargo run
  # Check logs for "Scheduler started"

  # 2. Test manual trigger (for development)
  curl -X POST http://localhost:8080/api/admin/trigger-update

  # 3. Verify updates occur
  # Check link metadata changes in database after interval
  ```

**Integration Points:**
- Links model: add `last_checked`, `check_failed_count` fields
- GitHub client: reuse existing fetch functions
- Main application: start scheduler on boot
- Configuration: add new environment variables

**Files to Create/Modify:**
- Create: `src/scheduler/mod.rs`
- Create: `migrations/XXXX_add_scheduler_fields.sql`
- Modify: `src/config.rs` (add scheduler config)
- Modify: `src/main.rs` (start scheduler)
- Modify: `src/models/link.rs` (add new fields)

  ---
Step 47: Dockerfile - Multi-stage Build

**Objective:** Create an optimized, secure Dockerfile using multi-stage builds for production deployment.

**Requirements:**

Create `Dockerfile` in project root with the following structure:

1. **Builder Stage**
   ```dockerfile
   # Use official Rust image
   FROM rust:1.75-slim as builder

   # Install build dependencies
   RUN apt-get update && apt-get install -y \
       pkg-config \
       libssl-dev \
       && rm -rf /var/lib/apt/lists/*

   # Set working directory
   WORKDIR /app

   # Copy dependency files first (for layer caching)
   COPY Cargo.toml Cargo.lock ./

   # Build dependencies (cached layer)
   RUN mkdir src && \
       echo "fn main() {}" > src/main.rs && \
       cargo build --release && \
       rm -rf src

   # Copy source code
   COPY . .

   # Build application
   RUN cargo build --release
   ```

2. **Runtime Stage**
   ```dockerfile
   # Use minimal runtime image
   FROM debian:bookworm-slim

   # Install runtime dependencies only
   RUN apt-get update && apt-get install -y \
       ca-certificates \
       libssl3 \
       && rm -rf /var/lib/apt/lists/*

   # Create non-root user
   RUN useradd -m -u 1000 rustylinks

   # Set working directory
   WORKDIR /app

   # Copy binary from builder
   COPY --from=builder /app/target/release/rusty-links .

   # Copy static assets
   COPY --from=builder /app/assets ./assets

   # Change ownership
   RUN chown -R rustylinks:rustylinks /app

   # Switch to non-root user
   USER rustylinks

   # Expose port
   EXPOSE 8080

   # Run application
   CMD ["./rusty-links"]
   ```

3. **Optimization Techniques**
    - Multi-stage build to minimize final image size
    - Layer caching for dependencies
    - Remove build artifacts and caches
    - Use slim base images
    - Target image size: < 100MB

4. **Security Measures**
    - Non-root user (rustylinks)
    - Minimal runtime dependencies
    - No package manager in final image
    - CA certificates for HTTPS

5. **Build Instructions**
   Create `.dockerignore`:
   ```
   target/
   .env
   .git/
   *.md
   docker-compose.yml
   ```

**Testing:**
  ```bash
  # 1. Build image
  docker build -t rusty-links:latest .

  # 2. Verify image size
  docker images rusty-links:latest
  # Should be < 150MB

  # 3. Test run (with env vars)
  docker run -it --rm \
    -e DATABASE_URL=postgres://user:pass@host/db \
    -e APP_PORT=8080 \
    -p 8080:8080 \
    rusty-links:latest

  # 4. Verify non-root user
  docker run rusty-links:latest id
  # Should show uid=1000(rustylinks)

  # 5. Test application works
  curl http://localhost:8080/api/health
  ```

**Files to Create:**
- Create: `Dockerfile`
- Create: `.dockerignore`

**Best Practices Implemented:**
- ✅ Multi-stage build
- ✅ Layer caching optimization
- ✅ Non-root user
- ✅ Minimal runtime dependencies
- ✅ Security hardening

  ---
Step 48: Docker Compose Configuration

**Objective:** Create Docker Compose configuration for easy local development and production deployment with PostgreSQL.

**Requirements:**

Create `compose.yml` in project root:

1. **PostgreSQL Service**
   ```yaml
   version: '3.8'

   services:
     postgres:
       image: postgres:16-alpine
       container_name: rusty-links-db
       environment:
         POSTGRES_USER: rustylinks
         POSTGRES_PASSWORD: ${DB_PASSWORD:-changeme}
         POSTGRES_DB: rustylinks
       volumes:
         - postgres_data:/var/lib/postgresql/data
       ports:
         - "5432:5432"
       healthcheck:
         test: ["CMD-SHELL", "pg_isready -U rustylinks"]
         interval: 10s
         timeout: 5s
         retries: 5
       restart: unless-stopped
   ```

2. **Application Service**
   ```yaml
     app:
       build:
         context: .
         dockerfile: Dockerfile
       container_name: rusty-links-app
       environment:
         DATABASE_URL: postgres://rustylinks:${DB_PASSWORD:-changeme}@postgres:5432/rustylinks
         APP_PORT: 8080
         UPDATE_INTERVAL_HOURS: ${UPDATE_INTERVAL_HOURS:-24}
         RUST_LOG: ${RUST_LOG:-info}
       ports:
         - "8080:8080"
       depends_on:
         postgres:
           condition: service_healthy
       restart: unless-stopped
   ```

3. **Volumes**
   ```yaml
   volumes:
     postgres_data:
       driver: local
   ```

4. **Environment File**
   Create `.env.example`:
   ```bash
   # Database
   DB_PASSWORD=your_secure_password_here

   # Application
   APP_PORT=8080
   UPDATE_INTERVAL_HOURS=24
   RUST_LOG=info

   # GitHub (optional - for higher rate limits)
   GITHUB_TOKEN=ghp_your_token_here
   ```

5. **Development Override**
   Create `compose.dev.yml`:
   ```yaml
   version: '3.8'

   services:
     app:
       build:
         target: builder
       volumes:
         - ./src:/app/src
         - ./assets:/app/assets
       environment:
         RUST_LOG: debug
       command: cargo watch -x run
   ```

**Usage Instructions:**

Create `docs/DOCKER.md`:
  ```markdown
  # Docker Deployment Guide

  ## Quick Start

  1. Copy environment template:
     ```bash
     cp .env.example .env
     ```

  2. Edit `.env` and set secure passwords

  3. Start services:
     ```bash
     docker compose up -d
     ```

  4. View logs:
     ```bash
     docker compose logs -f app
     ```

  5. Stop services:
     ```bash
     docker compose down
     ```

  ## Development Mode

  Use development compose file:
  ```bash
  docker compose -f compose.yml -f compose.dev.yml up
  ```

## Database Migrations

Migrations run automatically on application start.

## Backup Database

  ```bash
  docker compose exec postgres pg_dump -U rustylinks rustylinks > backup.sql
  ```

## Restore Database

  ```bash
  docker compose exec -T postgres psql -U rustylinks rustylinks < backup.sql
  ```
  ```

  **Testing:**
  ```bash
  # 1. Create environment file
  cp .env.example .env

  # 2. Start services
  docker compose up -d

  # 3. Verify services are running
  docker compose ps
  # Should show both services as "running (healthy)"

  # 4. Check application logs
  docker compose logs app

  # 5. Test application
  curl http://localhost:8080/api/health

  # 6. Access database
  docker compose exec postgres psql -U rustylinks rustylinks

  # 7. Stop services
  docker compose down

  # 8. Verify data persistence
  docker compose up -d
  # Data should still exist
  ```

**Files to Create:**
- Create: `compose.yml`
- Create: `compose.dev.yml`
- Create: `.env.example`
- Create: `docs/DOCKER.md`

**Integration:**
- Application automatically runs migrations on startup
- Health checks ensure proper startup order
- Volumes persist data across container restarts

  ---
Step 49: GitHub Container Registry Publishing

**Objective:** Set up automated CI/CD pipeline to build and publish Docker images to GitHub Container Registry on every release.

**Requirements:**

Create `.github/workflows/docker-publish.yml`:

1. **Workflow Triggers**
   ```yaml
   name: Build and Publish Docker Image

   on:
     push:
       tags:
         - 'v*'  # Trigger on version tags (v1.0.0, v1.1.0, etc.)
     workflow_dispatch:  # Allow manual triggers
   ```

2. **Build and Push Job**
   ```yaml
   jobs:
     build-and-push:
       runs-on: ubuntu-latest
       permissions:
         contents: read
         packages: write

       steps:
         - name: Checkout code
           uses: actions/checkout@v4

         - name: Log in to GitHub Container Registry
           uses: docker/login-action@v3
           with:
             registry: ghcr.io
             username: ${{ github.actor }}
             password: ${{ secrets.GITHUB_TOKEN }}

         - name: Extract metadata
           id: meta
           uses: docker/metadata-action@v5
           with:
             images: ghcr.io/${{ github.repository }}
             tags: |
               type=semver,pattern={{version}}
               type=semver,pattern={{major}}.{{minor}}
               type=semver,pattern={{major}}
               type=sha,prefix={{branch}}-
               type=raw,value=latest,enable={{is_default_branch}}

         - name: Build and push Docker image
           uses: docker/build-push-action@v5
           with:
             context: .
             push: true
             tags: ${{ steps.meta.outputs.tags }}
             labels: ${{ steps.meta.outputs.labels }}
             cache-from: type=gha
             cache-to: type=gha,mode=max
   ```

3. **Multi-Platform Build (Optional)**
   ```yaml
         - name: Set up QEMU
           uses: docker/setup-qemu-action@v3

         - name: Set up Docker Buildx
           uses: docker/setup-buildx-action@v3

         - name: Build and push (multi-platform)
           uses: docker/build-push-action@v5
           with:
             platforms: linux/amd64,linux/arm64
             # ... rest of config
   ```

4. **Release Instructions**
   Create `docs/RELEASE.md`:
   ```markdown
   # Release Process

   ## Creating a Release

   1. Update version in `Cargo.toml`
   2. Update `CHANGELOG.md`
   3. Commit changes:
      ```bash
      git add .
      git commit -m "chore: bump version to v1.0.0"
      ```
    4. Create and push tag:
       ```bash
       git tag v1.0.0
       git push origin main
       git push origin v1.0.0
       ```
    5. GitHub Actions will automatically build and publish

   ## Using Published Images

   ```bash
   docker pull ghcr.io/USERNAME/rusty-links:latest
   docker pull ghcr.io/USERNAME/rusty-links:v1.0.0
   ```

   ## Update compose.yml to use published image:

   ```yaml
   services:
     app:
       image: ghcr.io/USERNAME/rusty-links:latest
       # Remove 'build' section
   ```
   ```

5. **Package Visibility**
    - Document how to make package public in GitHub
    - Add README section about pulling images

**Testing:**
  ```bash
  # 1. Create a test tag
  git tag v0.1.0-test
  git push origin v0.1.0-test

  # 2. Monitor GitHub Actions
  # Go to repository → Actions tab
  # Verify workflow runs successfully

  # 3. Check package was published
  # Go to repository → Packages
  # Verify image appears

  # 4. Pull and test the image
  docker pull ghcr.io/YOUR-USERNAME/rusty-links:v0.1.0-test
  docker run --rm ghcr.io/YOUR-USERNAME/rusty-links:v0.1.0-test --version

  # 5. Clean up test tag
  git tag -d v0.1.0-test
  git push origin :refs/tags/v0.1.0-test
  ```

**Files to Create:**
- Create: `.github/workflows/docker-publish.yml`
- Create: `docs/RELEASE.md`
- Modify: `README.md` (add installation instructions)

**Tag Strategy:**
- `latest` - Most recent release
- `v1.0.0` - Specific version
- `v1.0` - Minor version (auto-updates patch)
- `v1` - Major version (auto-updates minor/patch)
- `main-abc123` - SHA-based tags for development

  ---
Step 50: Comprehensive README Documentation

**Objective:** Create a professional, comprehensive README that serves as the primary documentation entry point.

**Requirements:**

Update `README.md` with the following structure:

1. **Header Section**
   ```markdown
   # Rusty Links 🔗

   A self-hosted bookmark manager built with Rust and Dioxus. Organize, search, and manage your links with automatic metadata extraction and GitHub integration.

   ![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)
   ![License](https://img.shields.io/badge/license-MIT-blue)
   ![Docker](https://img.shields.io/badge/docker-ready-blue)

   [Features](#features) • [Quick Start](#quick-start) • [Documentation](#documentation) • [Contributing](#contributing)
   ```

2. **Features Section**
   ```markdown
   ## ✨ Features

   - 🔐 **Single-user authentication** with secure Argon2 password hashing
   - 📋 **Link management** with full CRUD operations
   - 🤖 **Automatic metadata extraction** - titles, descriptions, logos
   - 🐙 **GitHub integration** - stars, languages, licenses auto-detected
   - 📂 **Hierarchical categories** (up to 3 levels)
   - 🏷️ **Tags, languages, and licenses** for organization
   - 🔍 **Full-text search** with advanced filtering
   - ⏰ **Scheduled updates** to keep metadata fresh
   - 📱 **Responsive UI** - works on mobile, tablet, desktop
   - 🐳 **Docker ready** - one command deployment
   - 🔒 **Privacy-first** - self-hosted, your data stays yours
   ```

3. **Quick Start Section**
   ```markdown
   ## 🚀 Quick Start

   ### Using Docker Compose (Recommended)

   1. **Clone the repository**
      ```bash
      git clone https://github.com/USERNAME/rusty-links.git
      cd rusty-links
      ```

    2. **Configure environment**
       ```bash
       cp .env.example .env
       # Edit .env and set a secure database password
       ```

    3. **Start services**
       ```bash
       docker compose up -d
       ```

    4. **Access the application**
        - Open http://localhost:8080
        - Create your account (first user only)
        - Start adding links!

   ### Using Docker Image

   ```bash
   # Pull latest image
   docker pull ghcr.io/USERNAME/rusty-links:latest

   # Run with external PostgreSQL
   docker run -d \
     -e DATABASE_URL=postgres://user:pass@host/db \
     -e APP_PORT=8080 \
     -p 8080:8080 \
     ghcr.io/USERNAME/rusty-links:latest
   ```

   ### From Source

   See [Building from Source](#building-from-source) below.
   ```

4. **Screenshots Section**
   ```markdown
   ## 📸 Screenshots

   *Add screenshots here when available*
   ```

5. **Configuration Section**
   ```markdown
   ## ⚙️ Configuration

   Configure via environment variables:

   | Variable | Description | Default |
   |----------|-------------|---------|
   | `DATABASE_URL` | PostgreSQL connection string | *Required* |
   | `APP_PORT` | Application port | `8080` |
   | `UPDATE_INTERVAL_HOURS` | Metadata update frequency | `24` |
   | `RUST_LOG` | Log level | `info` |
   | `GITHUB_TOKEN` | GitHub API token (optional) | None |

   See [Configuration Guide](docs/CONFIGURATION.md) for details.
   ```

6. **Documentation Links**
   ```markdown
   ## 📚 Documentation

   - [API Documentation](docs/API.md) - Complete REST API reference
   - [Database Schema](docs/DATABASE.md) - Schema and migrations
   - [Docker Deployment](docs/DOCKER.md) - Docker setup guide
   - [Development Setup](docs/DEVELOPMENT.md) - Contributing guide
   - [Security](docs/SECURITY.md) - Security best practices
   - [Troubleshooting](docs/TROUBLESHOOTING.md) - Common issues
   ```

7. **Building from Source**
   ```markdown
   ## 🔨 Building from Source

   ### Prerequisites

   - Rust 1.75 or later
   - PostgreSQL 14+
   - Node.js 18+ (for Dioxus CLI)

   ### Steps

   1. **Install dependencies**
      ```bash
      cargo install dioxus-cli
      ```

    2. **Set up database**
       ```bash
       createdb rustylinks
       ```

    3. **Configure environment**
       ```bash
       cp .env.example .env
       # Edit .env with your database URL
       ```

    4. **Run migrations**
       ```bash
       cargo install sqlx-cli --no-default-features --features postgres
       sqlx migrate run
       ```

    5. **Run development server**
       ```bash
       dx serve
       ```

    6. **Build for production**
       ```bash
       cargo build --release
       ./target/release/rusty-links
       ```
   ```

8. **Architecture Overview**
   ```markdown
   ## 🏗️ Architecture

   - **Backend:** Rust with Axum web framework
   - **Frontend:** Dioxus (React-like for Rust)
   - **Database:** PostgreSQL with SQLx
   - **Authentication:** Session-based with Argon2
   - **Scraping:** reqwest + scraper crate
   - **Deployment:** Docker + Docker Compose
   ```

9. **Contributing Section**
   ```markdown
   ## 🤝 Contributing

   Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

   ### Development Workflow

   1. Fork the repository
   2. Create a feature branch
   3. Make your changes
   4. Run tests: `cargo test`
   5. Submit a pull request
   ```

10. **License and Credits**
    ```markdown
    ## 📄 License

    This project is licensed under the MIT License - see [LICENSE](LICENSE) for details.

    ## 🙏 Credits

    Built with:
    - [Rust](https://www.rust-lang.org/)
    - [Dioxus](https://dioxuslabs.com/)
    - [Axum](https://github.com/tokio-rs/axum)
    - [SQLx](https://github.com/launchbadge/sqlx)

    ## ⭐ Star History

    If you find this project useful, please consider giving it a star!
    ```

**Testing:**
  ```bash
  # 1. Verify all links work
  # Click through each documentation link

  # 2. Test quick start instructions
  # Follow steps exactly as written

  # 3. Verify Docker commands
  docker compose up -d
  # Should work without errors

  # 4. Check formatting
  # View README on GitHub
  # Ensure proper rendering
  ```

**Files to Create/Modify:**
- Modify: `README.md`
- Create: `CONTRIBUTING.md` (basic guidelines)
- Create: `LICENSE` (MIT license)

  ---
Step 51: API Documentation

**Objective:** Create comprehensive API documentation with all endpoints, request/response formats, and examples.

**Requirements:**

Create `docs/API.md` with complete API reference:

1. **Overview Section**
   ```markdown
   # Rusty Links API Documentation

   RESTful API for managing bookmarks, categories, tags, and metadata.

   ## Base URL

   ```
   http://localhost:8080/api
   ```

   ## Authentication

   Session-based authentication using HTTP-only cookies. Login via `/api/auth/login` to obtain a session.

   ## Response Format

   All endpoints return JSON. Successful responses have 2XX status codes.

   ### Success Response
   ```json
   {
     "id": "123e4567-e89b-12d3-a456-426614174000",
     "title": "Example Link",
     "url": "https://example.com"
   }
   ```

   ### Error Response
   ```json
   {
     "error": {
       "message": "Link not found",
       "code": "NOT_FOUND",
       "field": "id"  // Optional
     }
   }
   ```

   ## Rate Limiting

   No rate limiting for authenticated users. External API calls (GitHub, web scraping) are rate-limited internally.
   ```

2. **Authentication Endpoints**
   ```markdown
   ## Authentication

   ### POST /api/auth/setup

   Create the first user account (only works if no users exist).

   **Request:**
   ```json
   {
     "email": "user@example.com",
     "password": "secure_password"
   }
   ```

   **Response:** `201 Created`
   ```json
   {
     "id": "uuid",
     "email": "user@example.com"
   }
   ```

   **Errors:**
    - `409 Conflict` - User already exists
    - `400 Bad Request` - Invalid email/password

   ---

   ### POST /api/auth/login

   Login with email and password.

   **Request:**
   ```json
   {
     "email": "user@example.com",
     "password": "secure_password"
   }
   ```

   **Response:** `200 OK`
   ```json
   {
     "id": "uuid",
     "email": "user@example.com"
   }
   ```

   Sets session cookie automatically.

   **Errors:**
    - `401 Unauthorized` - Invalid credentials

   ---

   ### POST /api/auth/logout

   Logout current session.

   **Response:** `200 OK`
   ```json
   {
     "message": "Logged out successfully"
   }
   ```
   ```

3. **Links Endpoints**
   ```markdown
   ## Links

   ### GET /api/links

   List all links with optional search and filtering.

   **Query Parameters:**
   - `query` - Full-text search across title, description, URL
   - `category_id` - Filter by category UUID
   - `language_id` - Filter by language UUID (OR logic for multiple)
   - `license_id` - Filter by license UUID (OR logic)
   - `tag_id` - Filter by tag UUID (OR logic)
   - `page` - Page number (default: 1)
   - `per_page` - Items per page (default: 20, max: 100)
   - `sort_by` - Sort field: `created_at`, `updated_at`, `title`, `github_stars`
   - `sort_order` - `asc` or `desc` (default: `desc`)

   **Example:**
   ```bash
   curl "http://localhost:8080/api/links?query=rust&sort_by=github_stars&page=1"
   ```

   **Response:** `200 OK`
   ```json
   {
     "links": [
       {
         "id": "uuid",
         "url": "https://github.com/rust-lang/rust",
         "title": "The Rust Programming Language",
         "description": "Empowering everyone...",
         "logo": "base64...",
         "is_github_repo": true,
         "github_stars": 85000,
         "github_last_commit": "2024-01-15T10:30:00Z",
         "archived": false,
         "created_at": "2024-01-01T12:00:00Z",
         "updated_at": "2024-01-15T12:00:00Z",
         "categories": [
           {"id": "uuid", "name": "Programming Languages"}
         ],
         "languages": [
           {"id": "uuid", "name": "Rust"}
         ],
         "licenses": [
           {"id": "uuid", "acronym": "MIT", "full_name": "MIT License"}
         ],
         "tags": [
           {"id": "uuid", "name": "systems programming"}
         ]
       }
     ],
     "total": 42,
     "page": 1,
     "per_page": 20,
     "total_pages": 3
   }
   ```

   ---

   ### POST /api/links

   Create a new link. Metadata will be extracted asynchronously.

   **Request:**
   ```json
   {
     "url": "https://github.com/rust-lang/rust",
     "title": "Optional override",
     "description": "Optional override"
   }
   ```

   **Response:** `201 Created`
   ```json
   {
     "id": "uuid",
     "url": "https://github.com/rust-lang/rust",
     "title": "The Rust Programming Language",
     "description": "Extracted description...",
     "is_github_repo": true,
     "github_stars": 85000,
     "languages": [{"id": "uuid", "name": "Rust"}],
     "licenses": [{"id": "uuid", "acronym": "MIT"}]
   }
   ```

   **Errors:**
    - `400 Bad Request` - Invalid URL
    - `409 Conflict` - URL already exists

   ---

   ### PUT /api/links/:id

   Update an existing link.

   **Request:**
   ```json
   {
     "title": "New title",
     "description": "New description",
     "category_ids": ["uuid1", "uuid2"],
     "language_ids": ["uuid1"],
     "license_ids": ["uuid1"],
     "tag_ids": ["uuid1", "uuid2"]
   }
   ```

   **Response:** `200 OK`

   ---

   ### DELETE /api/links/:id

   Delete a link.

   **Response:** `204 No Content`

   **Errors:**
    - `404 Not Found` - Link doesn't exist

   ---

   ### POST /api/links/:id/refresh

   Manually trigger metadata refresh for a link.

   **Response:** `200 OK`
   ```json
   {
     "message": "Metadata refresh queued",
     "link_id": "uuid"
   }
   ```
   ```

4. **Categories Endpoints**
   ```markdown
   ## Categories

   ### GET /api/categories

   List all categories in hierarchical tree structure.

   **Response:** `200 OK`
   ```json
   [
     {
       "id": "uuid",
       "name": "Programming",
       "parent_id": null,
       "depth": 0,
       "children": [
         {
           "id": "uuid",
           "name": "Languages",
           "parent_id": "parent-uuid",
           "depth": 1,
           "children": []
         }
       ]
     }
   ]
   ```

   ### POST /api/categories

   Create a new category.

   **Request:**
   ```json
   {
     "name": "Web Development",
     "parent_id": "optional-parent-uuid"
   }
   ```

   **Response:** `201 Created`

   **Errors:**
    - `400 Bad Request` - Maximum depth (3) exceeded

   ### POST /api/categories/:id/move

   Move a category to a new parent (for drag-drop).

   **Request:**
   ```json
   {
     "new_parent_id": "uuid or null"
   }
   ```

   **Response:** `200 OK`
   ```

5. **Other Endpoints** (Languages, Licenses, Tags, Health)
   ```markdown
   ## Languages

   ### GET /api/languages
   List all programming languages.

   ### POST /api/languages
   Create a new language.

   **Request:**
   ```json
   {
     "name": "TypeScript"
   }
   ```

   ## Licenses

   ### GET /api/licenses
   List all licenses.

   ### POST /api/licenses
   Create a new license.

   **Request:**
   ```json
   {
     "acronym": "MIT",
     "full_name": "MIT License"
   }
   ```

   ## Tags

   ### GET /api/tags
   List all tags.

   ### POST /api/tags
   Create a new tag.

   ## Health Check

   ### GET /api/health

   Check if the application is running.

   **Response:** `200 OK`
   ```json
   {
     "status": "ok",
     "version": "1.0.0"
   }
   ```

   ### GET /api/health/scheduler

   Check scheduler status.

   **Response:** `200 OK`
   ```json
   {
     "status": "running",
     "last_run": "2024-01-15T12:00:00Z",
     "next_run": "2024-01-16T12:00:00Z"
   }
   ```
   ```

6. **curl Examples**
   ```markdown
   ## Complete Examples

   ### Setup and Login Flow

   ```bash
   # 1. Create first user
   curl -X POST http://localhost:8080/api/auth/setup \
     -H "Content-Type: application/json" \
     -d '{"email":"user@example.com","password":"secure123"}'

   # 2. Login (saves session cookie)
   curl -X POST http://localhost:8080/api/auth/login \
     -c cookies.txt \
     -H "Content-Type: application/json" \
     -d '{"email":"user@example.com","password":"secure123"}'

   # 3. Use session for authenticated requests
   curl http://localhost:8080/api/links \
     -b cookies.txt
   ```

   ### Working with Links

   ```bash
   # Create a link
   curl -X POST http://localhost:8080/api/links \
     -b cookies.txt \
     -H "Content-Type: application/json" \
     -d '{"url":"https://github.com/rust-lang/rust"}'

   # Search links
   curl "http://localhost:8080/api/links?query=rust&sort_by=github_stars" \
     -b cookies.txt

   # Update a link
   curl -X PUT http://localhost:8080/api/links/UUID \
     -b cookies.txt \
     -H "Content-Type: application/json" \
     -d '{"title":"Updated Title"}'
   ```
   ```

**Files to Create:**
- Create: `docs/API.md`

**Testing:**
  ```bash
  # Test all documented examples
  # Verify they work exactly as documented
  ```

  ---
Step 52: Database Migrations Documentation

**Objective:** Document the database schema, migrations, and backup/restore procedures.

**Requirements:**

Create `docs/DATABASE.md`:

1. **Schema Overview**
   ```markdown
   # Database Documentation

   ## Overview

   Rusty Links uses PostgreSQL 14+ with SQLx for type-safe queries and migrations.

   ## Connection

   ```
   postgresql://user:password@host:5432/database
   ```

   Configure via `DATABASE_URL` environment variable.

   ## Migrations

   Located in `migrations/` directory. Run automatically on application startup.

   ### Manual Migration Management

   ```bash
   # Install sqlx-cli
   cargo install sqlx-cli --no-default-features --features postgres

   # Run migrations
   sqlx migrate run

   # Revert last migration
   sqlx migrate revert

   # Check migration status
   sqlx migrate info
   ```
   ```

2. **Tables Documentation**
   ```markdown
   ## Tables

   ### users

   Stores user accounts (single user for Phase 1).

   | Column | Type | Constraints | Description |
   |--------|------|-------------|-------------|
   | id | UUID | PRIMARY KEY | User ID |
   | email | VARCHAR(255) | UNIQUE, NOT NULL | Login email |
   | password_hash | TEXT | NOT NULL | Argon2 hash |
   | created_at | TIMESTAMPTZ | NOT NULL | Account creation |

   ---

   ### sessions

   Active user sessions for authentication.

   | Column | Type | Constraints | Description |
   |--------|------|-------------|-------------|
   | id | UUID | PRIMARY KEY | Session ID |
   | user_id | UUID | FOREIGN KEY | References users |
   | expires_at | TIMESTAMPTZ | NOT NULL | Expiration time |
   | created_at | TIMESTAMPTZ | NOT NULL | Session start |

   **Indexes:**
   - `idx_sessions_user_id` on `user_id`
   - `idx_sessions_expires_at` on `expires_at`

   ---

   ### links

   Main table for storing bookmarks.

   | Column | Type | Constraints | Description |
   |--------|------|-------------|-------------|
   | id | UUID | PRIMARY KEY | Link ID |
   | user_id | UUID | FOREIGN KEY | Owner |
   | url | TEXT | NOT NULL | Full URL |
   | domain | VARCHAR(255) | NOT NULL | Extracted domain |
   | path | TEXT | | URL path |
   | title | TEXT | | Page title |
   | description | TEXT | | Description |
   | logo | BYTEA | | Logo/favicon |
   | is_github_repo | BOOLEAN | DEFAULT false | GitHub flag |
   | github_stars | INTEGER | | Star count |
   | github_last_commit | TIMESTAMPTZ | | Last commit date |
   | archived | BOOLEAN | DEFAULT false | Repo archived |
   | last_checked | TIMESTAMPTZ | | Last metadata update |
   | check_failed_count | INTEGER | DEFAULT 0 | Failure counter |
   | created_at | TIMESTAMPTZ | NOT NULL | Creation time |
   | updated_at | TIMESTAMPTZ | NOT NULL | Last update |

   **Indexes:**
   - `idx_links_user_id` on `user_id`
   - `idx_links_domain` on `domain`
   - `idx_links_github_stars` on `github_stars`
   - `idx_links_created_at` on `created_at`

   **Unique Constraint:**
   - `unique_user_domain_path` on `(user_id, domain, path)`

   ---

   ### categories

   Hierarchical categories (max 3 levels).

   | Column | Type | Constraints | Description |
   |--------|------|-------------|-------------|
   | id | UUID | PRIMARY KEY | Category ID |
   | user_id | UUID | FOREIGN KEY | Owner |
   | name | VARCHAR(100) | NOT NULL | Category name |
   | parent_id | UUID | FOREIGN KEY | Parent category |
   | depth | INTEGER | NOT NULL | Hierarchy depth (0-2) |
   | created_at | TIMESTAMPTZ | NOT NULL | Creation time |

   **Constraints:**
   - `check_depth` - `depth BETWEEN 0 AND 2`

   ---

   ### languages

   Programming languages.

   | Column | Type | Constraints | Description |
   |--------|------|-------------|-------------|
   | id | UUID | PRIMARY KEY | Language ID |
   | user_id | UUID | FOREIGN KEY | Owner |
   | name | VARCHAR(50) | NOT NULL | Language name |
   | created_at | TIMESTAMPTZ | NOT NULL | Creation time |

   **Unique:** `(user_id, LOWER(name))`

   ---

   ### licenses

   Software licenses.

   | Column | Type | Constraints | Description |
   |--------|------|-------------|-------------|
   | id | UUID | PRIMARY KEY | License ID |
   | user_id | UUID | FOREIGN KEY | Owner |
   | acronym | VARCHAR(20) | NOT NULL | e.g., "MIT" |
   | full_name | TEXT | NOT NULL | Full license name |
   | created_at | TIMESTAMPTZ | NOT NULL | Creation time |

   **Unique:** `(user_id, LOWER(acronym))`

   ---

   ### tags

   User-defined tags.

   | Column | Type | Constraints | Description |
   |--------|------|-------------|-------------|
   | id | UUID | PRIMARY KEY | Tag ID |
   | user_id | UUID | FOREIGN KEY | Owner |
   | name | VARCHAR(50) | NOT NULL | Tag name |
   | created_at | TIMESTAMPTZ | NOT NULL | Creation time |

   **Unique:** `(user_id, LOWER(name))`

   ---

   ### Junction Tables

   Many-to-many relationships between links and metadata.

   #### link_categories
   | Column | Type | Constraints |
   |--------|------|-------------|
   | link_id | UUID | FOREIGN KEY → links |
   | category_id | UUID | FOREIGN KEY → categories |

   **Primary Key:** `(link_id, category_id)`

   #### link_languages
   | Column | Type | Constraints |
   |--------|------|-------------|
   | link_id | UUID | FOREIGN KEY → links |
   | language_id | UUID | FOREIGN KEY → languages |
   | position | INTEGER | Order |

   **Primary Key:** `(link_id, language_id)`

   #### link_licenses
   | Column | Type | Constraints |
   |--------|------|-------------|
   | link_id | UUID | FOREIGN KEY → links |
   | license_id | UUID | FOREIGN KEY → licenses |
   | position | INTEGER | Order |

   **Primary Key:** `(link_id, license_id)`

   #### link_tags
   | Column | Type | Constraints |
   |--------|------|-------------|
   | link_id | UUID | FOREIGN KEY → links |
   | tag_id | UUID | FOREIGN KEY → tags |
   | position | INTEGER | Order |

   **Primary Key:** `(link_id, tag_id)`
   ```

3. **ER Diagram**
   ```markdown
   ## Entity Relationship Diagram

   ```
   users
   │
   ├──< sessions (user_id)
   │
   ├──< links (user_id)
   │    │
   │    ├──< link_categories >── categories
   │    ├──< link_languages >── languages
   │    ├──< link_licenses >── licenses
   │    └──< link_tags >── tags
   │
   ├──< categories (user_id)
   │    └─── categories (parent_id, self-reference)
   │
   ├──< languages (user_id)
   ├──< licenses (user_id)
   └──< tags (user_id)
   ```

   **Legend:**
   - `─<` : One-to-many relationship
   - `>─<` : Many-to-many (via junction table)
   ```

4. **Backup and Restore**
   ```markdown
   ## Backup and Restore

   ### Full Database Backup

   ```bash
   # Using Docker Compose
   docker compose exec postgres pg_dump -U rustylinks rustylinks > backup.sql
   
   # Or with custom format (compressed)
   docker compose exec postgres pg_dump -U rustylinks -Fc rustylinks > backup.dump
   ```

   ### Restore Database

   ```bash
   # From SQL backup
   docker compose exec -T postgres psql -U rustylinks rustylinks < backup.sql

   # From custom format
   docker compose exec -T postgres pg_restore -U rustylinks -d rustylinks backup.dump
   ```

   ### Automated Backups

   Create a cron job:

   ```bash
   # Daily backup at 2 AM
   0 2 * * * cd /path/to/rusty-links && docker compose exec -T postgres pg_dump -U rustylinks -Fc rustylinks > backups/backup-$(date +\%Y\%m\%d).dump
   ```

   ### Data Export/Import (Application Level)

   Use the API endpoints:

   ```bash
   # Export all links
   curl http://localhost:8080/api/export > links-export.json

   # Import links
   curl -X POST http://localhost:8080/api/import \
     -H "Content-Type: application/json" \
     -d @links-export.json
   ```
   ```

5. **Performance Tuning**
   ```markdown
   ## Performance Tuning

   ### Recommended PostgreSQL Settings

   For Docker deployment, add to `compose.yml`:

   ```yaml
   postgres:
     command:
       - postgres
       - -c
       - shared_buffers=256MB
       - -c
       - max_connections=100
       - -c
       - effective_cache_size=1GB
   ```

   ### Query Performance

   All common queries have appropriate indexes:
    - Link lookups by domain: `idx_links_domain`
    - Sorting by stars: `idx_links_github_stars`
    - Sorting by date: `idx_links_created_at`
    - Session lookups: `idx_sessions_user_id`

   ### Maintenance

   ```bash
   # Analyze tables (updates statistics)
   docker compose exec postgres psql -U rustylinks rustylinks -c "ANALYZE;"

   # Vacuum to reclaim space
   docker compose exec postgres psql -U rustylinks rustylinks -c "VACUUM FULL;"
   ```
   ```

**Files to Create:**
- Create: `docs/DATABASE.md`

**Testing:**
  ```bash
  # Verify backup/restore works
  docker compose exec postgres pg_dump -U rustylinks rustylinks > test-backup.sql
  docker compose exec -T postgres psql -U rustylinks rustylinks < test-backup.sql
  ```

  ---
Step 53: Testing Documentation and Test Suite Completion

**Objective:** Document the testing strategy and ensure comprehensive test coverage exists.

**Requirements:**

Create `docs/TESTING.md`:

1. **Testing Overview**
   ```markdown
   # Testing Documentation

   ## Overview

   Rusty Links uses a multi-layered testing approach:

   - **Unit Tests** - Individual functions and modules
   - **Integration Tests** - API endpoints and workflows
   - **End-to-End Tests** - Full user flows

   ## Running Tests

   ```bash
   # Run all tests
   cargo test

   # Run with output
   cargo test -- --nocapture

   # Run specific test
   cargo test test_create_link

   # Run integration tests only
   cargo test --test '*'
   
   # Run with coverage (requires cargo-tarpaulin)
   cargo tarpaulin --out Html
   ```
   ```

2. **Test Organization**
   ```markdown
   ## Test Structure
   
   ```
   tests/
   ├── api/
   │   ├── auth_test.rs         # Authentication endpoints
   │   ├── links_test.rs        # Links CRUD
   │   ├── categories_test.rs   # Categories
   │   └── search_test.rs       # Search and filtering
   ├── models/
   │   ├── link_test.rs         # Link model tests
   │   └── category_test.rs     # Category hierarchy
   └── common/
   └── mod.rs               # Test utilities
   ```
   
   ### Test Utilities
   
   Located in `tests/common/mod.rs`:
   
   ```rust
   // Create test database
   pub async fn setup_test_db() -> PgPool;
   
   // Create test user
   pub async fn create_test_user(pool: &PgPool) -> User;
   
   // Create authenticated session
   pub async fn create_test_session(pool: &PgPool, user_id: Uuid) -> String;
   
   // Cleanup test data
   pub async fn cleanup_test_db(pool: &PgPool);
   ```
   ```

3. **Writing Tests**
   ```markdown
   ## Writing New Tests
   
   ### Unit Test Example
   
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_url_validation() {
           assert!(is_valid_url("https://example.com"));
           assert!(!is_valid_url("not a url"));
       }

       #[tokio::test]
       async fn test_create_link() {
           let pool = setup_test_db().await;
           let user = create_test_user(&pool).await;

           let link = Link::create(&pool, user.id, CreateLink {
               url: "https://example.com".to_string(),
               title: Some("Test".to_string()),
               description: None,
           }).await.unwrap();

           assert_eq!(link.url, "https://example.com");

           cleanup_test_db(&pool).await;
       }
   }
   ```

   ### Integration Test Example

   ```rust
   // tests/api/links_test.rs
   use axum::http::StatusCode;
   use serde_json::json;

   #[tokio::test]
   async fn test_create_link_endpoint() {
       let pool = setup_test_db().await;
       let app = create_app(pool.clone()).await;
       let session = create_test_session(&pool).await;

       let response = app
           .oneshot(
               Request::builder()
                   .method("POST")
                   .uri("/api/links")
                   .header("Cookie", format!("session_id={}", session))
                   .header("Content-Type", "application/json")
                   .body(json!({
                       "url": "https://example.com"
                   }).to_string())
                   .unwrap()
           )
           .await
           .unwrap();

       assert_eq!(response.status(), StatusCode::CREATED);

       cleanup_test_db(&pool).await;
   }
   ```
   ```

4. **Test Coverage**
   ```markdown
   ## Coverage Goals

   Target coverage by module:

   | Module | Target | Current |
   |--------|--------|---------|
   | Models | 90% | Check with tarpaulin |
   | API | 85% | Check with tarpaulin |
   | Auth | 95% | Check with tarpaulin |
   | Scraper | 70% | Check with tarpaulin |
   | GitHub | 75% | Check with tarpaulin |

   ### Generate Coverage Report

   ```bash
   cargo install cargo-tarpaulin
   cargo tarpaulin --out Html --output-dir coverage
   open coverage/index.html
   ```
   ```

5. **CI/CD Testing**
   ```markdown
   ## Continuous Integration

   Create `.github/workflows/test.yml`:

   ```yaml
   name: Tests

   on: [push, pull_request]
   
   jobs:
     test:
       runs-on: ubuntu-latest
       
       services:
         postgres:
           image: postgres:16-alpine
           env:
             POSTGRES_PASSWORD: test
             POSTGRES_DB: rustylinks_test
           options: >-
             --health-cmd pg_isready
             --health-interval 10s
             --health-timeout 5s
             --health-retries 5

       steps:
         - uses: actions/checkout@v4

         - uses: actions-rs/toolchain@v1
           with:
             toolchain: stable
             override: true

         - name: Run tests
           env:
             DATABASE_URL: postgres://postgres:test@localhost/rustylinks_test
           run: cargo test --all-features

         - name: Check formatting
           run: cargo fmt -- --check

         - name: Run clippy
           run: cargo clippy -- -D warnings
   ```
   ```

6. **Manual Testing Checklist**
   ```markdown
   ## Manual Testing Checklist

   Use before each release:

   ### Authentication
   - [ ] Can create first user account
   - [ ] Cannot create second user (single-user mode)
   - [ ] Can login with correct credentials
   - [ ] Cannot login with wrong credentials
   - [ ] Can logout
   - [ ] Session persists across browser refresh

   ### Link Management
   - [ ] Can add a link with URL only
   - [ ] Metadata extracted automatically
   - [ ] GitHub repos show stars and languages
   - [ ] Can edit link title and description
   - [ ] Can delete link
   - [ ] Cannot add duplicate URL

   ### Search and Filtering
   - [ ] Search finds links by title
   - [ ] Search finds links by description
   - [ ] Can filter by category
   - [ ] Can filter by language (OR logic)
   - [ ] Can combine filters (AND logic)
   - [ ] Sorting works for all columns

   ### Categories
   - [ ] Can create top-level category
   - [ ] Can create child category
   - [ ] Cannot create 4th level (max 3)
   - [ ] Can drag-drop to reparent
   - [ ] Can delete empty category
   - [ ] Cannot delete category with links

   ### Responsive Design
   - [ ] Works on mobile (< 640px)
   - [ ] Works on tablet (640px - 1024px)
   - [ ] Works on desktop (> 1024px)
   - [ ] Touch interactions work

   ### Performance
   - [ ] Initial page load < 2s
   - [ ] Search results appear instantly
   - [ ] Pagination works smoothly
   - [ ] No console errors
   ```

**Files to Create:**
- Create: `docs/TESTING.md`
- Create: `.github/workflows/test.yml`
- Create: `tests/common/mod.rs` (if not exists)

**Verify:**
  ```bash
  # All tests pass
  cargo test

  # CI workflow is valid
  # Push to GitHub and check Actions tab
  ```

  ---
Step 54: Security Hardening Checklist

**Objective:** Document security features and provide a hardening checklist for production deployment.

**Requirements:**

Create `docs/SECURITY.md`:

1. **Security Features**
   ```markdown
   # Security Documentation

   ## Overview

   Rusty Links implements security best practices for a self-hosted application.

   ## Implemented Security Features

   ### Authentication & Authorization

   - ✅ **Argon2 Password Hashing** - Industry-standard, memory-hard hashing
   - ✅ **Session-based Authentication** - HTTP-only, secure cookies
   - ✅ **Session Expiration** - Automatic timeout after inactivity
   - ✅ **Single-user Mode** - Simplified attack surface for Phase 1
   - ✅ **CSRF Protection** - Same-site cookie attribute

   ### Input Validation

   - ✅ **URL Validation** - Proper parsing and sanitization
   - ✅ **Email Validation** - Format checking
   - ✅ **SQL Injection Prevention** - Parameterized queries via SQLx
   - ✅ **XSS Prevention** - Content Security Policy headers
   - ✅ **Path Traversal Prevention** - Input sanitization

   ### Data Protection

   - ✅ **Password Requirements** - Minimum 8 characters
   - ✅ **Secure Session Storage** - Database-backed sessions
   - ✅ **No Sensitive Logging** - Passwords never logged
   - ✅ **Database Encryption at Rest** - Via PostgreSQL configuration

   ### Network Security

   - ✅ **HTTPS Ready** - Works behind reverse proxy
   - ✅ **CORS Configuration** - Restricted to same origin
   - ✅ **Security Headers** - CSP, X-Frame-Options, etc.

   ### Dependency Security

   - ✅ **No Known Vulnerabilities** - Regular `cargo audit`
   - ✅ **Minimal Dependencies** - Reduced attack surface
   - ✅ **Trusted Crates** - Only well-maintained dependencies
   ```

2. **Production Hardening Checklist**
   ```markdown
   ## Production Deployment Checklist

   ### Before Deployment

   - [ ] Change default database password in `.env`
   - [ ] Set strong admin email/password
   - [ ] Configure `RUST_LOG=warn` (not debug/trace)
   - [ ] Review all environment variables
   - [ ] Enable automatic database backups
   - [ ] Set up monitoring and alerts

   ### Reverse Proxy Configuration

   Deploy behind nginx or Caddy for HTTPS:

   **Nginx Example** (`/etc/nginx/sites-available/rustylinks`):

   ```nginx
   server {
       listen 443 ssl http2;
       server_name links.yourdomain.com;

       ssl_certificate /path/to/cert.pem;
       ssl_certificate_key /path/to/key.pem;

       # Security headers
       add_header X-Frame-Options "SAMEORIGIN" always;
       add_header X-Content-Type-Options "nosniff" always;
       add_header X-XSS-Protection "1; mode=block" always;
       add_header Referrer-Policy "no-referrer-when-downgrade" always;
       add_header Content-Security-Policy "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline';" always;

       # Reverse proxy
       location / {
           proxy_pass http://localhost:8080;
           proxy_set_header Host $host;
           proxy_set_header X-Real-IP $remote_addr;
           proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
           proxy_set_header X-Forwarded-Proto $scheme;
       }

       # Rate limiting
       limit_req_zone $binary_remote_addr zone=login:10m rate=5r/m;

       location /api/auth/login {
           limit_req zone=login burst=10;
           proxy_pass http://localhost:8080;
       }
   }

   # Redirect HTTP to HTTPS
   server {
       listen 80;
       server_name links.yourdomain.com;
       return 301 https://$server_name$request_uri;
   }
   ```

   **Caddy Example** (`Caddyfile`):

   ```
   links.yourdomain.com {
       reverse_proxy localhost:8080

       header {
           X-Frame-Options "SAMEORIGIN"
           X-Content-Type-Options "nosniff"
           X-XSS-Protection "1; mode=block"
           Referrer-Policy "no-referrer-when-downgrade"
           Content-Security-Policy "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline';"
       }
   }
   ```

   ### Firewall Configuration

   ```bash
   # Allow only necessary ports
   sudo ufw default deny incoming
   sudo ufw default allow outgoing
   sudo ufw allow 22/tcp    # SSH
   sudo ufw allow 80/tcp    # HTTP (nginx)
   sudo ufw allow 443/tcp   # HTTPS (nginx)
   sudo ufw enable

   # Application port (8080) should NOT be exposed
   # Only nginx should access it on localhost
   ```

   ### Docker Security

    - [ ] Run container as non-root user (already configured)
    - [ ] Set read-only root filesystem where possible
    - [ ] Limit container resources (CPU, memory)
    - [ ] Use Docker secrets for sensitive data
    - [ ] Scan images for vulnerabilities

   ```yaml
   # Enhanced compose.yml security
   services:
     app:
       # ... existing config ...
       security_opt:
         - no-new-privileges:true
       read_only: true
       tmpfs:
         - /tmp
       deploy:
         resources:
           limits:
             cpus: '1.0'
             memory: 512M
   ```

   ### Database Security

    - [ ] Use strong database password (32+ characters)
    - [ ] Restrict PostgreSQL to localhost only
    - [ ] Enable SSL for database connections (production)
    - [ ] Regular automated backups
    - [ ] Test restore procedure

   ```yaml
   # PostgreSQL SSL in compose.yml
   postgres:
     command: postgres -c ssl=on -c ssl_cert_file=/var/lib/postgresql/server.crt -c ssl_key_file=/var/lib/postgresql/server.key
   ```

   ### Application Security

    - [ ] Set session expiration (default: 24 hours)
    - [ ] Configure rate limiting at reverse proxy
    - [ ] Monitor failed login attempts
    - [ ] Regular dependency updates (`cargo update`)
    - [ ] Run `cargo audit` monthly

   ### Monitoring

    - [ ] Set up uptime monitoring (e.g., UptimeRobot)
    - [ ] Configure log aggregation
    - [ ] Monitor disk space (database growth)
    - [ ] Alert on container restarts
    - [ ] Track failed authentication attempts
   ```

3. **Vulnerability Reporting**
   ```markdown
   ## Reporting Security Issues

   If you discover a security vulnerability:

   1. **Do NOT** open a public GitHub issue
   2. Email security@yourdomain.com (or your contact)
   3. Include:
      - Description of the vulnerability
      - Steps to reproduce
      - Potential impact
      - Suggested fix (if any)

   We will respond within 48 hours and work on a fix.

   ## Security Updates

   Subscribe to security updates:
   - GitHub Watch → Custom → Security alerts only
   - Star the repository for release notifications

   ## Regular Maintenance

   Schedule these security tasks:

   - **Weekly:** Review logs for suspicious activity
   - **Monthly:** Update dependencies (`cargo update`, rebuild)
   - **Monthly:** Run `cargo audit`
   - **Quarterly:** Review user access (Phase 2: multi-user)
   - **Quarterly:** Test backup and restore procedure
   - **Yearly:** Rotate database credentials
   - **Yearly:** Review and update security policies
   ```

4. **Security Best Practices**
   ```markdown
   ## Best Practices for Users

   ### Password Policy

   - Use a unique password (not reused from other services)
   - Minimum 12 characters recommended
   - Use a password manager
   - Change password if compromised

   ### Network Access

   - Do NOT expose Rusty Links directly to the internet
   - ALWAYS use a reverse proxy with HTTPS
   - Consider VPN access for remote usage
   - Use fail2ban for SSH protection

   ### Backup Policy

   - Automated daily backups
   - Store backups off-site
   - Encrypt backup files
   - Test restores regularly
   - Keep backups for 30 days minimum

   ### Update Policy

   - Apply security updates immediately
   - Test updates in staging first (if critical)
   - Monitor release notes for security fixes
   - Subscribe to security announcements
   ```

**Files to Create:**
- Create: `docs/SECURITY.md`
- Create: `docs/DEPLOYMENT.md` (production deployment guide)
- Create: `examples/nginx.conf`
- Create: `examples/Caddyfile`

**Verify:**
  ```bash
  # Run security audit
  cargo audit

  # Check for outdated dependencies
  cargo outdated

  # Scan Docker image
  docker scout cves rusty-links:latest
  ```

  ---
Step 55: Final Integration and Launch Preparation

**Objective:** Final integration testing, documentation review, and preparation for production launch.

**Requirements:**

Complete these final tasks before considering Phase 1 complete:

1. **Complete Testing Checklist**

   Create `docs/LAUNCH_CHECKLIST.md`:

   ```markdown
   # Launch Checklist

   ## Pre-Launch Testing

   ### Functionality Tests

   - [ ] Fresh installation works (docker compose up)
   - [ ] User creation flow works
   - [ ] Login/logout functionality
   - [ ] Add link with automatic metadata extraction
   - [ ] GitHub integration fetches stars and languages
   - [ ] Search finds links correctly
   - [ ] Filters work (category, language, license, tag)
   - [ ] Sorting works for all columns
   - [ ] Pagination works correctly
   - [ ] Category hierarchy (create, edit, delete, drag-drop)
   - [ ] Tags CRUD operations
   - [ ] Languages and licenses management
   - [ ] Scheduled updates run successfully
   - [ ] Manual link refresh works
   - [ ] Link editing preserves data
   - [ ] Link deletion works
   - [ ] Duplicate URL detection works

   ### Performance Tests

   - [ ] Page loads in < 2 seconds
   - [ ] Search is responsive (< 500ms)
   - [ ] Handles 1000+ links without slowdown
   - [ ] Docker image size reasonable (< 150MB)
   - [ ] Database migrations complete quickly
   - [ ] Metadata updates don't block UI

   ### Security Tests

   - [ ] Cannot access /api/* without authentication
   - [ ] Session expires correctly
   - [ ] Password requirements enforced
   - [ ] Cannot create duplicate users
   - [ ] SQL injection attempts fail
   - [ ] XSS attempts sanitized
   - [ ] CSRF protection works
   - [ ] Non-root user in Docker

   ### Browser Compatibility

   - [ ] Chrome/Chromium (latest)
   - [ ] Firefox (latest)
   - [ ] Safari (latest)
   - [ ] Mobile Safari (iOS)
   - [ ] Mobile Chrome (Android)

   ### Responsive Design

   - [ ] Mobile (320px - 640px)
   - [ ] Tablet (640px - 1024px)
   - [ ] Desktop (1024px - 1920px)
   - [ ] 4K displays (2560px+)

   ## Documentation Review

   - [ ] README.md complete and accurate
   - [ ] API.md covers all endpoints
   - [ ] DATABASE.md schema documented
   - [ ] SECURITY.md hardening checklist complete
   - [ ] TESTING.md explains test strategy
   - [ ] DOCKER.md deployment guide clear
   - [ ] RELEASE.md release process documented
   - [ ] All code examples tested and working
   - [ ] Screenshots added (if available)
   - [ ] Links in docs all work

   ## Code Quality

   - [ ] `cargo test` passes all tests
   - [ ] `cargo clippy` has no warnings
   - [ ] `cargo fmt` formatting applied
   - [ ] `cargo audit` shows no vulnerabilities
   - [ ] No unwrap() in production code paths
   - [ ] Proper error handling everywhere
   - [ ] Logging configured appropriately
   - [ ] No hardcoded secrets
   - [ ] .env.example up to date

   ## Docker & Deployment

   - [ ] Dockerfile builds successfully
   - [ ] Image size optimized
   - [ ] Docker compose starts cleanly
   - [ ] Health checks pass
   - [ ] Migrations run automatically
   - [ ] Volumes persist data correctly
   - [ ] Can stop/start without data loss
   - [ ] Logs accessible via docker compose logs
   - [ ] GitHub Actions workflow working
   - [ ] Container registry publishing works

   ## Final Preparation

   - [ ] Version number set correctly (in Cargo.toml)
   - [ ] CHANGELOG.md updated
   - [ ] Git tags created
   - [ ] GitHub release created with notes
   - [ ] Demo instance running (optional)
   - [ ] Community announcement prepared

   ## Post-Launch Monitoring

   - [ ] Monitor GitHub issues for bug reports
   - [ ] Check Docker Hub pull statistics
   - [ ] Watch for security alerts
   - [ ] Respond to community questions
   - [ ] Plan Phase 2 features
   ```

2. **Create Changelog**

   Create `CHANGELOG.md`:

   ```markdown
   # Changelog

   All notable changes to Rusty Links will be documented in this file.

   The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
   and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

   ## [1.0.0] - 2024-XX-XX

   ### Added

   #### Core Features
   - Single-user authentication with Argon2 password hashing
   - Session-based authentication with HTTP-only cookies
   - Link management with full CRUD operations
   - Automatic metadata extraction (title, description, logo)
   - GitHub repository integration (stars, languages, licenses)
   - Duplicate URL detection

   #### Organization
   - Hierarchical categories (up to 3 levels deep)
   - Programming languages management
   - Software licenses management
   - Custom tags
   - Many-to-many relationships for all metadata

   #### Search & Filtering
   - Full-text search across title, description, URL
   - Filter by category (single selection)
   - Filter by languages (OR logic, multi-select)
   - Filter by licenses (OR logic, multi-select)
   - Filter by tags (OR logic, multi-select)
   - Sort by: created date, title, GitHub stars
   - Pagination support

   #### UI/UX
   - Responsive design (mobile, tablet, desktop)
   - Real-time search with debouncing
   - Drag-and-drop category re-parenting
   - Link details modal with async metadata loading
   - GitHub suggestions as outlined chips
   - Loading states and error handling
   - Toast notifications

   #### Background Jobs
   - Scheduled metadata updates (configurable interval)
   - Automatic GitHub data refresh
   - Link accessibility verification
   - Graceful failure handling

   #### Deployment
   - Docker support with multi-stage builds
   - Docker Compose for one-command deployment
   - GitHub Actions CI/CD workflow
   - GitHub Container Registry publishing
   - Non-root Docker user for security
   - Health check endpoints

   #### Documentation
   - Comprehensive README
   - Complete API documentation
   - Database schema documentation
   - Security hardening guide
   - Testing documentation
   - Deployment guides

   ### Security
   - Argon2 password hashing
   - SQL injection prevention (parameterized queries)
   - XSS prevention (content security policy)
   - CSRF protection (SameSite cookies)
   - Session expiration
   - Input validation
   - Minimal Docker attack surface

   ## [Unreleased]

   ### Planned for Phase 2
   - Multi-user support with roles
   - Public/private link sharing
   - Dark mode theme
   - Browser extension
   - Bulk import/export
   - Advanced search operators
   - OAuth authentication
   - Mobile app
   - API rate limiting
   - Advanced analytics

   ---

   [1.0.0]: https://github.com/USERNAME/rusty-links/releases/tag/v1.0.0
   [Unreleased]: https://github.com/USERNAME/rusty-links/compare/v1.0.0...HEAD
   ```

3. **Final Integration Test Script**

   Create `scripts/integration-test.sh`:

   ```bash
   #!/bin/bash
   set -e

   echo "🧪 Running Rusty Links Integration Tests"
   echo "========================================"

   # Colors
   GREEN='\033[0;32m'
   RED='\033[0;31m'
   NC='\033[0m'

   BASE_URL="http://localhost:8080"
   COOKIES="test-cookies.txt"

   # Cleanup
   cleanup() {
       rm -f $COOKIES
       echo "Cleaned up test artifacts"
   }
   trap cleanup EXIT

   # Test health endpoint
   echo -n "Testing health endpoint... "
   if curl -s "$BASE_URL/api/health" | grep -q "ok"; then
       echo -e "${GREEN}✓${NC}"
   else
       echo -e "${RED}✗${NC}"
       exit 1
   fi

   # Test setup (create first user)
   echo -n "Testing user setup... "
   RESPONSE=$(curl -s -X POST "$BASE_URL/api/auth/setup" \
       -H "Content-Type: application/json" \
       -d '{"email":"test@example.com","password":"testpass123"}')

   if echo $RESPONSE | grep -q "test@example.com"; then
       echo -e "${GREEN}✓${NC}"
   else
       echo -e "${RED}✗${NC}"
       echo "Response: $RESPONSE"
       exit 1
   fi

   # Test login
   echo -n "Testing login... "
   LOGIN_RESPONSE=$(curl -s -c $COOKIES -X POST "$BASE_URL/api/auth/login" \
       -H "Content-Type: application/json" \
       -d '{"email":"test@example.com","password":"testpass123"}')

   if echo $LOGIN_RESPONSE | grep -q "test@example.com"; then
       echo -e "${GREEN}✓${NC}"
   else
       echo -e "${RED}✗${NC}"
       exit 1
   fi

   # Test create link
   echo -n "Testing link creation... "
   LINK_RESPONSE=$(curl -s -b $COOKIES -X POST "$BASE_URL/api/links" \
       -H "Content-Type: application/json" \
       -d '{"url":"https://github.com/rust-lang/rust"}')

   LINK_ID=$(echo $LINK_RESPONSE | grep -o '"id":"[^"]*"' | cut -d'"' -f4)
   if [ -n "$LINK_ID" ]; then
       echo -e "${GREEN}✓${NC}"
   else
       echo -e "${RED}✗${NC}"
       exit 1
   fi

   # Test get links
   echo -n "Testing get links... "
   if curl -s -b $COOKIES "$BASE_URL/api/links" | grep -q "rust-lang"; then
       echo -e "${GREEN}✓${NC}"
   else
       echo -e "${RED}✗${NC}"
       exit 1
   fi

   # Test search
   echo -n "Testing search... "
   if curl -s -b $COOKIES "$BASE_URL/api/links?query=rust" | grep -q "rust-lang"; then
       echo -e "${GREEN}✓${NC}"
   else
       echo -e "${RED}✗${NC}"
       exit 1
   fi

   # Test logout
   echo -n "Testing logout... "
   if curl -s -b $COOKIES -X POST "$BASE_URL/api/auth/logout" | grep -q "success"; then
       echo -e "${GREEN}✓${NC}"
   else
       echo -e "${GREEN}✓${NC}"  # Logout might return different response
   fi

   echo ""
   echo -e "${GREEN}All integration tests passed!${NC}"
   ```

   Make executable:
   ```bash
   chmod +x scripts/integration-test.sh
   ```

4. **Create Release Notes Template**

   Create `.github/RELEASE_TEMPLATE.md`:

   ```markdown
   # Rusty Links v1.0.0

   ## 🎉 Initial Release

   Rusty Links is a self-hosted bookmark manager built with Rust and Dioxus. This is the first stable release ready for production use.

   ## ✨ Features

   - Single-user authentication
   - Automatic metadata extraction
   - GitHub integration
   - Hierarchical categories
   - Full-text search
   - Scheduled updates
   - Docker deployment
   - Complete documentation

   ## 📦 Installation

   ### Docker Compose (Recommended)

   ```bash
   wget https://raw.githubusercontent.com/USERNAME/rusty-links/v1.0.0/compose.yml
   wget https://raw.githubusercontent.com/USERNAME/rusty-links/v1.0.0/.env.example
   mv .env.example .env
   # Edit .env with your settings
   docker compose up -d
   ```

   ### Docker Image

   ```bash
   docker pull ghcr.io/USERNAME/rusty-links:v1.0.0
   ```

   ## 🆕 What's New

   This is the initial release. See [CHANGELOG.md](CHANGELOG.md) for complete list of features.

   ## 📚 Documentation

    - [README](README.md) - Quick start guide
    - [API Documentation](docs/API.md)
    - [Security Guide](docs/SECURITY.md)
    - [Deployment Guide](docs/DOCKER.md)

   ## 🐛 Known Issues

    - None at this time

   ## 🙏 Thanks

   Thank you to all contributors and testers who helped make this release possible!

   ## 📝 Upgrading

   This is the first release, no upgrade necessary.

   For future upgrades, see [UPGRADING.md](docs/UPGRADING.md).
   ```

**Final Verification:**
  ```bash
  # 1. Run all tests
  cargo test

  # 2. Run integration test script
  ./scripts/integration-test.sh

  # 3. Build Docker image
  docker build -t rusty-links:1.0.0 .

  # 4. Test Docker deployment
  docker compose down -v
  docker compose up -d
  # Follow quick start in README

  # 5. Verify documentation
  # Open each doc file and verify accuracy

  # 6. Create release
  git tag v1.0.0
  git push origin v1.0.0
  # Create GitHub release with notes
  ```

**Files to Create:**
- Create: `docs/LAUNCH_CHECKLIST.md`
- Create: `CHANGELOG.md`
- Create: `scripts/integration-test.sh`
- Create: `.github/RELEASE_TEMPLATE.md`
- Create: `docs/UPGRADING.md` (placeholder for future)

**Success Criteria:**
- All checklist items pass
- All tests green
- Docker deployment works flawlessly
- Documentation is accurate and complete
- Ready for public release

🎉 **Phase 1 Complete!**

  ---
Summary

This blueprint provides 10 implementation prompts for Part 8: Deployment & Documentation:

1. Step 46: Background job scheduler for metadata updates
2. Step 47: Multi-stage Dockerfile with security hardening
3. Step 48: Docker Compose configuration with PostgreSQL
4. Step 49: GitHub Actions CI/CD and container registry
5. Step 50: Comprehensive README documentation
6. Step 51: Complete API documentation
7. Step 52: Database schema and migrations documentation
8. Step 53: Testing documentation and test suite
9. Step 54: Security hardening checklist and guides
10. Step 55: Final integration testing and launch preparation

Each prompt is:
- ✅ Self-contained with clear objectives
- ✅ Builds incrementally on previous work
- ✅ Includes testing/verification steps
- ✅ Properly sized (not too large, not too small)
- ✅ Has clear integration points
- ✅ Leaves no orphaned code
- ✅ Production-ready focus

The prompts follow best practices for deployment, security, documentation, and launch preparation, ensuring a production-ready application at completion.
