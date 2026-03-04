# Rusty Links 🔗

A self-hosted bookmark manager built with Rust and Dioxus. Organize, search, and manage your links with automatic metadata extraction and GitHub integration.

![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)
![License](https://img.shields.io/badge/license-MIT-blue)
![Docker](https://img.shields.io/badge/docker-ready-blue)

[Features](#-features) • [Quick Start](#-quick-start) • [Documentation](#-documentation) • [Contributing](#-contributing)

---

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

---

## 🚀 Quick Start

### Using Docker Compose (Recommended)

1. **Clone the repository**
   ```bash
   git clone https://github.com/YOUR-USERNAME/rusty-links.git
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
docker pull ghcr.io/NiceGuyIT/rusty-links:latest

# Run with external PostgreSQL
docker run -d \
  -e DATABASE_URL=postgres://user:pass@host/db \
  -e APP_PORT=8080 \
  -p 8080:8080 \
  ghcr.io/YOUR-USERNAME/rusty-links:latest
```

### From Source

See [Building from Source](#-building-from-source) below.

---

## 📸 Screenshots

*Screenshots will be added as the UI is developed*

---

## ⚙️ Configuration

Configure via environment variables:

| Variable                | Description                                         | Default    |
|-------------------------|-----------------------------------------------------|------------|
| `DATABASE_URL`          | PostgreSQL connection string                        | *Required* |
| `APP_PORT`              | Application port                                    | `8080`     |
| `UPDATE_INTERVAL_DAYS`  | Days between metadata updates                       | `30`       |
| `UPDATE_INTERVAL_HOURS` | Metadata update frequency (hours)                   | `24`       |
| `BATCH_SIZE`            | Links processed per batch                           | `50`       |
| `JITTER_PERCENT`        | Update scheduling jitter (0-100)                    | `20`       |
| `RUST_LOG`              | Log level (trace, debug, info, warn, error)         | `info`     |
| `GITHUB_TOKEN`          | GitHub API token (optional, for higher rate limits) | None       |

See `.env.example` for all available options.

---

## 📚 Documentation

- [Docker Deployment Guide](docs/DOCKER.md) - Complete Docker setup and deployment
- [Release Process](docs/RELEASE.md) - How to create and publish releases
- [Project Architecture](CLAUDE.md) - System design and architecture overview

---

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

---

## 🏗️ Architecture

- **Backend:** Rust with Axum web framework
- **Frontend:** Dioxus (React-like for Rust)
- **Database:** PostgreSQL with SQLx
- **Authentication:** Session-based with Argon2
- **Scraping:** reqwest + scraper crate
- **Deployment:** Docker + Docker Compose

### Project Structure

```
rusty-links/
├── src/
│   ├── main.rs           # Application entry point
│   ├── api/              # REST API endpoints
│   ├── auth/             # Authentication logic
│   ├── ui/               # Dioxus frontend components
│   ├── models/           # Database models
│   ├── config.rs         # Configuration management
│   └── error.rs          # Error handling
├── migrations/           # Database migrations
├── assets/               # Static assets
├── docs/                 # Documentation
└── Dockerfile            # Production container
```

---

## 🛠️ Development

### Running Tests

```bash
cargo test
```

### Development with Docker

For development with hot reloading:

```bash
docker compose -f compose.yml -f compose.dev.yml up
```

This will:
- Mount source code as volumes
- Use `cargo watch` for automatic reloads
- Enable debug logging

### Database Migrations

Migrations run automatically on application startup. For manual control:

```bash
# Install SQLx CLI
cargo install sqlx-cli --no-default-features --features postgres

# Create new migration
sqlx migrate add <migration_name>

# Run migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Check without building
cargo check
```

---

## 🚀 Production Deployment

See [docs/DOCKER.md](docs/DOCKER.md) for complete deployment instructions.

Quick production deployment:

```bash
# Pull published image
docker pull ghcr.io/YOUR-USERNAME/rusty-links:latest

# Start with Docker Compose
docker compose up -d

# View logs
docker compose logs -f app

# Check status
docker compose ps
```

### Security Considerations

- Always use strong database passwords
- Run as non-root user (default in Docker)
- Keep dependencies updated
- Use HTTPS in production (reverse proxy recommended)
- Regularly backup your database

---

## 🤝 Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Workflow

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests: `cargo test`
5. Run linter: `cargo clippy`
6. Format code: `cargo fmt`
7. Commit your changes (`git commit -m 'Add amazing feature'`)
8. Push to the branch (`git push origin feature/amazing-feature`)
9. Open a Pull Request

### Reporting Issues

- Use GitHub Issues for bug reports and feature requests
- Check existing issues before creating new ones
- Provide clear reproduction steps for bugs
- Include system information (OS, Rust version, etc.)

---

## 📄 License

This project is licensed under the MIT License - see [LICENSE](LICENSE) for details.

---

## 🙏 Credits

Built with:
- [Rust](https://www.rust-lang.org/) - Systems programming language
- [Dioxus](https://dioxuslabs.com/) - React-like UI framework for Rust
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [SQLx](https://github.com/launchbadge/sqlx) - Async SQL toolkit
- [PostgreSQL](https://www.postgresql.org/) - Database
- [Docker](https://www.docker.com/) - Containerization

---

## ⭐ Star History

If you find this project useful, please consider giving it a star!

---

## Prompts

This is the prompt to create vibe coding prompts.

Part 8: Deployment & Documentation (Steps 46-55)

> Read the `IMPLEMENTATION_GUIDE` for Part 8: Deployment & Documentation (Steps 46-55). Draft a detailed, step-by-step blueprint for building Part 8: Deployment & Documentation (Steps 46-55). Then, once you have a solid plan, break it down into small, iterative chunks that build on each other. Look at these chunks and then go another round to break it into small steps. Review the results and make sure that the steps are small enough to be implemented safely, but big enough to move the project forward. Iterate until you feel that the steps are right sized for this project.
>
> From here you should have the foundation to provide a series of prompts for a code-generation LLM that will implement each step. Prioritize best practices, and incremental progress, ensuring no big jumps in complexity at any stage. Make sure that each prompt builds on the previous prompts, and ends with wiring things together. There should be no hanging or orphaned code that isn't integrated into a previous step.
>
> Make sure and separate each prompt section. Use markdown. Each prompt should be tagged as text using code tags using quadruple (4) backticks. The goal is to output prompts, but context, etc is important as well. The inner code tags should use triple (3) backticks. Save the prompts in the `docs/` directory.

---

> Read the `IMPLEMENTATION_GUIDE` and draft a detailed, step-by-step blueprint for building the Link Flow in step 36. Then, once you have a solid plan, break it down into small, iterative chunks that build on each other. Look at these chunks and then go another round to break it into small steps. Review the results and make sure that the steps are small enough to be implemented safely, but big enough to move the project forward. Iterate until you feel that the steps are right sized for this project.
>
> From here you should have the foundation to provide a series of prompts for a code-generation LLM that will implement each step. Prioritize best practices, and incremental progress, ensuring no big jumps in complexity at any stage. Make sure that each prompt builds on the previous prompts, and ends with wiring things together. There should be no hanging or orphaned code that isn't integrated into a previous step.
>
> Make sure and separate each prompt section. Use markdown. Each prompt should be tagged as text using code tags. The goal is to output prompts, but context, etc. is important as well. Each prompt will use quadruple (4) backtick code tags while the inner code tags will use triple (3) backticks. Save the prompts in the `docs/` directory.

## TODO

- [ ] Delete `oci-build/setup.nu` — it is orphaned now that the Dockerfile uses the dummy-src pattern instead of Nushell + setup.nu for building.
- [ ] Remove or update `.cargo/config.toml` — it sets `target = "x86_64-unknown-linux-gnu"` (glibc), which conflicts with the Alpine/musl Docker build. The Dockerfile works around this by using selective `COPY` instead of `COPY . .`, but the config should be reviewed.

## Notes

- ✅ Server feature compiles successfully (cargo check --features server)
- ✅ Web feature compiles successfully for WASM (cargo check --features web --target wasm32-unknown-unknown)

