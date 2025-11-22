# Rusty Links

A full-stack web application for link management built with Rust, featuring Dioxus for the frontend and Axum for the backend API.

## Features

- Full-stack Rust application using Dioxus and Axum
- PostgreSQL database with SQLx
- Session-based authentication with Argon2 password hashing
- Automated link metadata updates
- Docker support for easy deployment
- Multi-platform Docker images (AMD64/ARM64)

## Quick Start with Docker

The fastest way to get started is using Docker Compose:

```bash
# Clone the repository
git clone https://github.com/YOUR-USERNAME/rusty-links.git
cd rusty-links

# Copy environment template
cp .env.example .env

# Edit .env and set a secure DB_PASSWORD
nano .env

# Start services
docker compose up -d

# Access the application
open http://localhost:8080
```

## Installation

### Using Pre-built Docker Images

Pull and run the latest published image from GitHub Container Registry:

```bash
# Pull latest version
docker pull ghcr.io/YOUR-USERNAME/rusty-links:latest

# Run with Docker Compose (recommended)
cat > compose.yml <<EOF
version: '3.8'

services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: rustylinks
      POSTGRES_PASSWORD: changeme
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

  app:
    image: ghcr.io/YOUR-USERNAME/rusty-links:latest
    environment:
      DATABASE_URL: postgres://rustylinks:changeme@postgres:5432/rustylinks
      APP_PORT: 8080
      RUST_LOG: info
    ports:
      - "8080:8080"
    depends_on:
      postgres:
        condition: service_healthy

volumes:
  postgres_data:
EOF

docker compose up -d
```

### Building from Source

#### Prerequisites

- Rust 1.75 or later
- PostgreSQL 14 or later
- Node.js (for Dioxus CLI)

#### Local Development

```bash
# Install Dioxus CLI
cargo install dioxus-cli

# Clone repository
git clone https://github.com/YOUR-USERNAME/rusty-links.git
cd rusty-links

# Set up PostgreSQL database
createdb rusty_links

# Configure environment
cp .env.example .env
# Edit .env with your database credentials

# Run database migrations (automatic on startup)
# Or manually: sqlx migrate run

# Start development server
dx serve
```

#### Building Docker Image Locally

```bash
# Build production image
docker build -t rusty-links:local .

# Or use Docker Compose to build
docker compose build
```

## Configuration

All configuration is done via environment variables. See `.env.example` for available options:

- `DATABASE_URL` - PostgreSQL connection string
- `APP_PORT` - Application server port (default: 8080)
- `UPDATE_INTERVAL_DAYS` - Days between metadata updates (default: 30)
- `UPDATE_INTERVAL_HOURS` - Hours between scheduler runs (default: 24)
- `RUST_LOG` - Log level (trace, debug, info, warn, error)

## Documentation

- [Docker Deployment Guide](docs/DOCKER.md) - Complete Docker deployment instructions
- [Release Process](docs/RELEASE.md) - How to create and publish releases
- [CLAUDE.md](CLAUDE.md) - Project overview and architecture

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
```

## Production Deployment

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

## Architecture

- **Frontend**: Dioxus (fullstack mode)
- **Backend**: Axum REST API
- **Database**: PostgreSQL with SQLx
- **Authentication**: Session-based with Argon2 password hashing
- **Deployment**: Multi-platform Docker images (AMD64/ARM64)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

See LICENSE file for details.

## Prompts

This is the prompt to create vibe coding prompts.

Part 8: Deployment & Documentation (Steps 46-55)

> Read the `IMPLEMENTATION_GUIDE` for Part 8: Deployment & Documentation (Steps 46-55). Draft a detailed, step-by-step blueprint for building Part 8: Deployment & Documentation (Steps 46-55). Then, once you have a solid plan, break it down into small, iterative chunks that build on each other. Look at these chunks and then go another round to break it into small steps. Review the results and make sure that the steps are small enough to be implemented safely, but big enough to move the project forward. Iterate until you feel that the steps are right sized for this project.
>
> From here you should have the foundation to provide a series of prompts for a code-generation LLM that will implement each step. Prioritize best practices, and incremental progress, ensuring no big jumps in complexity at any stage. Make sure that each prompt builds on the previous prompts, and ends with wiring things together. There should be no hanging or orphaned code that isn't integrated into a previous step.
>
> Make sure and separate each prompt section. Use markdown. Each prompt should be tagged as text using code tags using quadruple (4) backticks. The goal is to output prompts, but context, etc is important as well. The inner code tags should use triple (3) backticks. Save the prompts in the `docs/` directory.