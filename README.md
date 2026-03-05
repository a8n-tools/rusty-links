# Rusty Links

A self-hosted bookmark manager built with Rust and Dioxus. Organize, search, and manage your links with automatic metadata extraction and GitHub integration.

[Features](#features) | [Quick Start](#quick-start) | [Documentation](#documentation) | [Contributing](#contributing)

---

## Features

- **JWT authentication** with bcrypt password hashing and refresh tokens
- **Link management** with full CRUD operations
- **Automatic metadata extraction** - titles, descriptions, logos
- **GitHub integration** - stars, languages, licenses auto-detected
- **Hierarchical categories** (up to 3 levels)
- **Tags, languages, and licenses** for organization
- **Full-text search** with advanced filtering
- **Scheduled updates** to keep metadata fresh
- **Responsive UI** - works on mobile, tablet, and desktop
- **Docker ready** - one command deployment
- **Two build modes** - standalone (self-hosted) and SaaS (parent app auth)

---

## Quick Start

### Using Docker Compose (Recommended)

1. **Clone the repository**
   ```bash
   git clone https://git.a8n.run/a8n-tools/rusty-links.git
   cd rusty-links
   ```

2. **Configure environment**
   ```bash
   cp .env.standalone .env
   # Edit .env and set a secure database password
   ```

3. **Start services**
   ```bash
   docker compose up -d
   ```

4. **Access the application**
   - Open http://localhost:3003
   - Create your account via the setup page
   - Start adding links!

### From Source

See [Building from Source](#building-from-source) below.

---

## Configuration

### Build Modes

Rusty Links supports two build modes via the `BUILD_MODE` build argument:

- **standalone** (default) - Self-hosted with built-in JWT authentication
- **saas** - Authentication handled by a parent application's cookies

Environment variable templates are provided for each mode:
```bash
cp .env.standalone .env   # Standalone mode
cp .env.saas .env         # SaaS mode
```

### Environment Variables

#### Core Settings

| Variable                | Description                                         | Default      |
|-------------------------|-----------------------------------------------------|--------------|
| `DATABASE_URL`          | PostgreSQL connection string                        | *Required*   |
| `APP_PORT`              | Application port                                    | `8080`       |
| `HOST_PORT`             | Docker host port mapping                            | `3003`       |
| `DB_USERNAME`           | PostgreSQL username (Docker Compose)                | `rustylinks` |
| `DB_PASSWORD`           | PostgreSQL password (Docker Compose)                | `changeme`   |
| `DB_NAME`               | PostgreSQL database name (Docker Compose)           | `rustylinks` |
| `RUST_LOG`              | Log level (trace, debug, info, warn, error)         | `info`       |

#### Scheduler Settings

| Variable                | Description                                         | Default    |
|-------------------------|-----------------------------------------------------|------------|
| `UPDATE_INTERVAL_DAYS`  | Days between metadata updates                       | `30`       |
| `UPDATE_INTERVAL_HOURS` | Scheduler run frequency (hours)                     | `24`       |
| `BATCH_SIZE`            | Links processed per batch                           | `50`       |
| `JITTER_PERCENT`        | Update scheduling jitter (0-100)                    | `20`       |
| `GITHUB_TOKEN`          | GitHub API token (optional, for higher rate limits) | None       |

#### Standalone Mode Settings

| Variable                     | Description                                    | Default |
|------------------------------|------------------------------------------------|---------|
| `JWT_SECRET`                 | Secret key for signing JWT tokens              | Random  |
| `JWT_EXPIRY`                 | Access token expiry in hours                   | `1`     |
| `REFRESH_TOKEN_EXPIRY`       | Refresh token expiry in days                   | `7`     |
| `ACCOUNT_LOCKOUT_ATTEMPTS`   | Failed login attempts before lockout           | `5`     |
| `ACCOUNT_LOCKOUT_DURATION`   | Lockout duration in minutes                    | `30`    |
| `ALLOW_REGISTRATION`         | Allow new user registration (`true`/`1`)       | `true`  |

See `.env.standalone` and `.env.saas` for full documentation of all options.

---

## Documentation

- [Docker Deployment Guide](docs/DOCKER.md) - Complete Docker setup and deployment
- [API Reference](docs/API.md) - Complete endpoint reference with examples
- [Database Schema](docs/DATABASE.md) - Schema reference and migration history
- [Security](docs/SECURITY.md) - Security features and hardening guide
- [Testing](docs/TESTING.md) - Testing strategy and instructions
- [Deployment](docs/DEPLOYMENT.md) - Production deployment guide
- [Release Process](docs/RELEASE.md) - Versioning and release workflow

---

## Building from Source

### Prerequisites

- Rust (latest stable recommended)
- PostgreSQL 17+
- Dioxus CLI (`cargo install dioxus-cli` or `cargo binstall dioxus-cli`)

### Steps

1. **Install dependencies**
   ```bash
   cargo install dioxus-cli
   rustup target add wasm32-unknown-unknown
   ```

2. **Set up database**
   ```bash
   createdb rustylinks
   ```

3. **Configure environment**
   ```bash
   cp .env.standalone .env
   # Edit .env with your database URL
   ```

4. **Run development server**
   ```bash
   dx serve
   ```

   Migrations run automatically on startup.

5. **Build for production**
   ```bash
   dx build --release
   ```

---

## Architecture

- **Backend:** Rust with Axum web framework
- **Frontend:** Dioxus 0.7 (fullstack mode with SSR)
- **Database:** PostgreSQL with SQLx (compile-time checked queries)
- **Authentication:** JWT tokens with bcrypt password hashing
- **Scraping:** reqwest + scraper crate
- **Styling:** Tailwind CSS v4
- **Deployment:** Docker + Docker Compose

### Project Structure

```
rusty-links/
├── src/
│   ├── main.rs              # Application entry point
│   ├── lib.rs               # Library root, feature-gated modules
│   ├── config.rs            # Environment-based configuration
│   ├── error.rs             # Centralized error handling
│   ├── security.rs          # Security utilities
│   ├── api/                 # REST API endpoints
│   ├── auth/                # JWT authentication and middleware
│   ├── github/              # GitHub API integration
│   ├── models/              # Database models (User, Link, Category, Tag, etc.)
│   ├── scheduler/           # Background task runner
│   ├── scraper/             # HTML metadata extraction
│   ├── server_functions/    # Dioxus server functions (client/server bridge)
│   └── ui/                  # Dioxus frontend
│       ├── app.rs           # Root component and routing
│       ├── components/      # Reusable UI components
│       └── pages/           # Page components
├── migrations/              # Database migrations (8 files)
├── assets/                  # Static assets (generated CSS, favicon)
├── examples/                # Reverse proxy configs (nginx, Caddy)
├── docs/                    # Documentation
├── Dockerfile               # Multi-stage production build
├── compose.yml              # Docker Compose (app + PostgreSQL)
└── compose.dev.yml          # Development override (hot reloading)
```

---

## Development

### Running Tests

```bash
cargo test
```

### Development with Docker

For development with hot reloading:

```bash
docker compose -f compose.yml -f compose.dev.yml up
```

### Database Migrations

Migrations run automatically on application startup. For manual control:

```bash
cargo install sqlx-cli --no-default-features --features postgres

# Create new migration
sqlx migrate add <migration_name>

# Run migrations
sqlx migrate run
```

### Code Quality

```bash
cargo fmt
cargo clippy
cargo check
cargo check --features server
cargo check --features web --target wasm32-unknown-unknown
```

---

## Production Deployment

See [docs/DOCKER.md](docs/DOCKER.md) and [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md) for complete instructions.

Quick production deployment:

```bash
docker compose up -d

# View logs
docker compose logs -f app

# Check status
docker compose ps
```

### Security Considerations

- Always set a strong `DB_PASSWORD` and `JWT_SECRET`
- Run as non-root user (default in Docker: appuser, UID 1001)
- Use HTTPS in production (reverse proxy recommended, see `examples/`)
- Regularly backup your database

---

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## License

This project is licensed under the MIT License - see [LICENSE](LICENSE.md) for details.

---

## Credits

Built with [Rust](https://www.rust-lang.org/), [Dioxus](https://dioxuslabs.com/), [Axum](https://github.com/tokio-rs/axum), [SQLx](https://github.com/launchbadge/sqlx), [PostgreSQL](https://www.postgresql.org/), and [Docker](https://www.docker.com/).

---

## TODO

- [ ] Delete `oci-build/setup.nu` — orphaned now that the Dockerfile uses the dummy-src pattern
- [ ] Remove or update `.cargo/config.toml` — sets `target = "x86_64-unknown-linux-gnu"` (glibc), which conflicts with Alpine/musl Docker builds
