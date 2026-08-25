# Rusty Links

A self-hosted bookmark manager built with Rust and Dioxus. Organize, search, and manage your links with automatic metadata extraction and GitHub integration.

[Features](#features) | [Quick Start](#quick-start) | [Documentation](#documentation) | [Contributing](#contributing)

---

## Features

- **JWT authentication** with Argon2id password hashing and refresh tokens
- **Link management** with full CRUD operations
- **Automatic metadata extraction** - titles, descriptions, logos
- **GitHub integration** - stars, languages, licenses auto-detected
- **Hierarchical categories** (up to 3 levels)
- **Tags, languages, and licenses** for organization
- **Full-text search** with advanced filtering
- **Scheduled updates** to keep metadata fresh
- **Responsive UI** - works on mobile, tablet, and desktop
- **Docker ready** - one command deployment
- **Two deployment modes** - standalone (self-hosted) and hosted (parent app auth), selected at runtime

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
   cp .env.standalone.example .env
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

### Deployment Modes

Rusty Links resolves its deployment mode at runtime from `OIDC_ISSUER`; a single binary and image serves both. There is no build-time mode argument and no `standalone` / `saas` Cargo feature.

- **standalone** (`OIDC_ISSUER` unset) - Self-hosted with built-in JWT authentication
- **hosted** (`OIDC_ISSUER` set) - Authentication delegated to the parent application's OIDC provider

Environment variable templates are provided for each mode:
```bash
cp .env.standalone.example .env   # Standalone mode
cp .env.saas.example .env         # Hosted mode
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

#### Sign-in Location Alert Settings

On a successful login the country is read from the `X-IPCountry` header injected by the reverse proxy's geoblock plugin (there is no geoip database). When it differs from the country of the account's previous login, the user is emailed a security alert. The header is believed only when the socket peer is a trusted proxy (`TRUSTED_PROXY_CIDRS`, see below); from any other peer, or with no such header, no country resolves and no alert is sent, so the feature degrades cleanly and a forged header can neither fake nor suppress an alert. Applies to both modes.

| Variable                        | Description                                              | Default      |
|---------------------------------|----------------------------------------------------------|--------------|
| `LOGIN_LOCATION_ALERTS_ENABLED` | Global kill switch for new-location alerts               | `true`       |
| `SMTP_HOST`                     | SMTP server hostname (unset means log-only delivery)     | None         |
| `SMTP_TLS`                      | Connection encryption: `starttls`, `tls`, or `none`      | `starttls`   |
| `SMTP_PORT`                     | SMTP server port (overrides the port `SMTP_TLS` implies) | Per `SMTP_TLS` |
| `SMTP_USERNAME`                 | SMTP username (omit for an unauthenticated relay)        | None         |
| `SMTP_PASSWORD`                 | SMTP password                                            | None         |
| `SMTP_FROM_EMAIL`               | Sender address (unset means log-only delivery)           | None         |
| `SMTP_FROM_NAME`                | Sender display name                                      | None         |

Alert mail is sent over an encrypted connection by default: `SMTP_TLS` defaults to `starttls` (STARTTLS required on port 587), and `tls` selects implicit TLS on port 465. `none` is a plaintext escape hatch for a trusted loopback or sidecar MTA; it must be set explicitly and logs a warning naming the host on every send. Parsing is case-insensitive and an unrecognised value falls back to `starttls`.

Alerts are also suppressed per user by the `users.notify_new_location` opt-out column, and are capped at one email per user per country per day. A signed-in user turns their own alerts off and back on from the **Account** page in the navbar (LINKS-43), or with `PATCH /api/auth/me` (`{"notify_new_location": false}`) against `GET /api/auth/me`'s current setting; the write always targets the session's own account (LINKS-33).

#### Sign-in Approval Gate Settings

The gate turns the detection above into a block. With it on, a sign-in that passes the password but does not look familiar issues no session at all: the user is emailed a single-use link, and the sign-in completes only after they open it, approve, and sign in again. An attempt nobody approves never completes, which is what an alert alone cannot do, because an attacker who reads the mailbox can delete an alert but cannot make an unapproved sign-in succeed. It is notify-and-approve, not a lock: nothing about the account is disabled.

Two triggers feed the one gate, and either alone holds a sign-in:

- **a country the account has not used before** (LINKS-35), resolved at the edge from `X-IPCountry`
- **a device the account has not used before** (LINKS-45), identified by a random id the browser mints once and keeps in `localStorage`, of which only the SHA-256 is stored

A sign-in that trips both is held once and approved once. One switch disables both; there is deliberately no second flag.

| Variable                  | Description                                                             | Default |
|---------------------------|-------------------------------------------------------------------------|---------|
| `LOGIN_APPROVAL_ENABLED`  | Hold a sign-in from a new country or a new device until it is approved   | `false` |

It is opt-in, unlike the alert kill switch, because it can stop a real user from signing in. Only the exact value `true` enables it; unset, empty, `TRUE`, `yes`, and `1` all leave behaviour exactly as the alert shipped it. Turn it on per deployment, and only where SMTP is set up, since with no SMTP the approval link is only written to the log. The country trigger additionally needs the geoblock edge and `TRUSTED_PROXY_CIDRS`; the device trigger needs neither and works on any deployment.

These sign-ins are never gated, by construction, because gating any of them would lock a real user out:

- a first-ever sign-in, which has no prior country and no recorded device (this is what `POST /api/auth/setup` creates)
- a sign-in whose country does not resolve, which is any deployment with no geoblock edge or an empty `TRUSTED_PROXY_CIDRS`, on the country trigger
- an account with no recorded devices, on the device trigger, which is **every existing account on the deploy that adds the feature** (see below)
- a sign-in that submits no device id, such as an API client or `curl`, on the device trigger

##### Turning it on for the first time: nobody is held

The `known_devices` table is created **empty and is deliberately not backfilled**. Zero known devices is read as an account's baseline, exactly as a NULL `last_login_country` is, and never as "new device". So upgrading to a build that has the device trigger holds nobody, including you: the first sign-in after the upgrade records the browser it came from, and only a different browser after that is held. Reading it the other way would hold every account on the instance at once, with an emailed link as the only way back in.

The device signal is browser recognition, not machine identification. Clearing site data, a private window, a second browser on the same machine and a genuinely new laptop all look identical to it, and each produces one approval mail. That is the accepted cost; what it never does is let an unfamiliar browser through silently. `docs/SECURITY.md` states the full boundary.

The gate deliberately ignores the per-user `notify_new_location` opt-out. That preference is written from an authenticated session, so honouring it would let anyone holding a session switch the security control off, and an opted-out user would be held with no mail to approve with. It continues to govern the alert alone, and it disables neither trigger.

##### Locked out and cannot receive the email?

Recovery does not depend on email. In order:

1. Set `LOGIN_APPROVAL_ENABLED=false` (or remove it) and restart. The password sign-in works again immediately, with no deploy and no code change.
2. If the environment cannot be changed but the database can, clear the stored baselines so the next sign-in is treated as a first-ever one:

   ```sql
   UPDATE users SET last_login_country = NULL WHERE email = '<address>';
   DELETE FROM known_devices WHERE user_id = (SELECT id FROM users WHERE email = '<address>');
   ```

3. If SMTP is in log mode, the approval mail (link included) is in the application log; the app warns about this at startup when the gate is on with no SMTP configured.

There is deliberately no way to approve a held sign-in from the database: only the SHA-256 of the emailed token is stored, so nobody with database access can mint or replay an approval link.

#### Trusted Proxy Settings

`X-Forwarded-For`, `X-Real-Ip`, and `X-IPCountry` are believed only when the socket peer sits in a configured trusted CIDR. The peer is the one input a client cannot forge, so it gates all three: a client reaching the app directly can spoof neither its IP nor its country.

| Variable               | Description                                                          | Default |
|------------------------|----------------------------------------------------------------------|---------|
| `TRUSTED_PROXY_CIDRS`  | Comma-separated CIDRs or bare IPs whose peers may set those headers   | Empty   |

Empty is the secure default: no peer is trusted, every forwarded header is ignored, and the socket address is used. Unparseable entries log a warning and are skipped.

Setting it is **required behind a reverse proxy**. The peer is then the proxy on a private Docker network, so an empty list collapses every client to the proxy address and the sign-in location alert stops firing. The deployed stacks use the private ingress ranges:

```
TRUSTED_PROXY_CIDRS=10.0.0.0/8,172.16.0.0/12,192.168.0.0/16,fd00::/8
```

See `.env.standalone.example` and `.env.saas.example` for full documentation of all options.

---

## Documentation

- [Docker Deployment Guide](docs/DOCKER.md) - Complete Docker setup and deployment
- [API Reference](docs/API.md) - Complete endpoint reference with examples
- [Database Schema](docs/DATABASE.md) - Schema reference and migration history
- [Security](docs/SECURITY.md) - Security features and hardening guide
- [Testing](docs/TESTING.md) - Testing strategy and instructions
- [Deployment](docs/DEPLOYMENT.md) - Production deployment guide
- [Release Process](docs/RELEASE.md) - Versioning and release workflow

### Historical documents

`docs/history/` holds the pre-build design documents. They are kept for the record and are not maintained, so they still name GitHub Container Registry and describe a plan rather than the system: LINKS-38 corrected those claims across the documents above and deliberately left these alone, because rewriting a design document to match what was later built destroys the record of what was intended. Each opens with a banner saying so and its body is unchanged.

- [Phase 1 Product Specification](docs/history/SPECIFICATION.md)
- [Implementation Guide](docs/history/IMPLEMENTATION_GUIDE.md)
- [Step 36: Add Link Flow](docs/history/STEP_36_ADD_LINK_FLOW.md)
- [UI Implementation Guide](docs/history/UI_IMPLEMENTATION_README.md)
- [Dioxus 0.7 Fullstack Conversion](docs/history/FULLSTACK_CONVERSION.md)

`docs/superpowers/` holds the dated plans and designs the superpowers skill writes. They are unmaintained in the same way, and each opens with the same banner. They stay in place rather than moving under `docs/history/` because that is where the skill writes new ones, so a move would only be undone by the next plan.

- [Per-User Dev Instances (plan)](docs/superpowers/plans/2026-04-01-per-user-dev-instances.md)
- [High Contrast Mode (plan)](docs/superpowers/plans/2026-04-02-high-contrast-mode.md)
- [Getting Started Links and Clean Recipe (plan)](docs/superpowers/plans/2026-04-02-fix-getting-started.md)
- [Per-User Dev Instances (design)](docs/superpowers/specs/2026-04-01-per-user-dev-instances-design.md)
- [High Contrast Mode (design)](docs/superpowers/specs/2026-04-02-high-contrast-mode-design.md)

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
   cp .env.standalone.example .env
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
- **Authentication:** JWT tokens with Argon2id password hashing
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
├── migrations/              # Database migrations (sqlx, applied on startup)
├── assets/                  # Static assets (generated CSS, favicon)
├── examples/                # Reverse proxy configs (nginx, Caddy)
├── docs/                    # Documentation (history/ holds the unmaintained design docs)
├── Dockerfile               # Multi-stage production build
├── compose.yml              # Docker Compose (app + PostgreSQL)
└── compose.dev.yml          # Development override (hot reloading)
```

---

## Development

### Running Tests

`default = []` and every server module is behind `#[cfg(feature = "server")]`, so a bare `cargo test` compiles almost none of the crate.

```bash
# Server-side unit tests (the bulk of the suite). `just test` runs exactly this leg;
# a bare `cargo test` runs 13 of the 220 library tests and prints ok.
just test
cargo test --features server --lib

# Default-feature tests
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

`just pre-commit` runs every check `.forgejo/workflows/check.yml` runs and stops at the first failure. The five static guards run on the host first, in well under a second; the dependency audit and then the cargo legs run in the dev container.

```bash
just pre-commit
```

`just check` is the fast host-side subset: clippy under all three compilation configurations plus `cargo fmt --check`, and none of the build, test, doc-test, database or dependency-audit legs, which need the compose stack.

```bash
just check
```

The individual legs:

```bash
nu scripts/check-suite-parity.nu             # the justfile and the workflow still run the same legs
nu scripts/check-migration-immutability.nu   # no committed migration was edited
nu scripts/check-migration-docs.nu           # migrations/ and the docs/DATABASE.md table agree
nu scripts/check-build-flags.nu              # every justfile --features / --build-arg exists downstream
nu scripts/check-dev-clean-volumes.nu        # dev-clean removes exactly the volumes the compose files declare
nu scripts/check-dependency-audit.nu         # Cargo.lock against the RustSec advisory database
cargo fmt --check
cargo clippy --all-targets -- --deny warnings
cargo clippy --all-targets --features server -- --deny warnings
cargo clippy --all-targets --features web --target wasm32-unknown-unknown -- --deny warnings
cargo build --all-targets
cargo build --all-targets --features server
cargo test --lib
cargo test --features server --lib
nu scripts/check-doc-tests-ran.nu            # the doc examples, with the vacuity guard
cargo test --features server --test db_schema
nu scripts/check-db-tests-ran.nu             # the tests/ targets against Postgres, with the skip guard
```

`.forgejo/workflows/audit.yml` runs the audit leg again weekly, which is the only run that sees an advisory published against a `Cargo.lock` nobody has touched; [docs/SECURITY.md](docs/SECURITY.md#security-audit) has the failure policy and the exception rules.

`.forgejo/workflows/check.yml` runs exactly this list, and `scripts/check-suite-parity.nu` is what holds the two in step: it parses both files and fails when a leg or a guard script is in one and not the other, in either direction, after normalising the flag spellings each file uses locally. Change one copy and CI tells you about the other (LINKS-49). It holds `just check` to the workflow's lint legs too, one way, so a clippy configuration or the fmt leg dropped from that recipe fails as well (LINKS-64).

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
