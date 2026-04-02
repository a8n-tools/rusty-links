# Per-User Dev Instances Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable developers to spin up isolated rusty-links instances at `{USER}-links.a8n.run` on the shared dev server via Traefik.

**Architecture:** Two compose files — `compose.dev.yml` (Traefik-backed, per-user on shared server) and `compose.local.yml` (localhost ports, no Traefik). Justfile defaults to the dev compose. Follows the pattern established in the SaaS project (`/home/nate/saas/compose.dev.yml`).

**Tech Stack:** Docker Compose, Traefik v3 (labels-based routing), Cloudflare DNS (wildcard `*.a8n.run`), Let's Encrypt (via `cert-cloudflare` certresolver)

---

### Task 1: Rename `compose.yml` to `compose.local.yml`

**Files:**
- Rename: `compose.yml` → `compose.local.yml`

- [ ] **Step 1: Rename the file**

```bash
git mv compose.yml compose.local.yml
```

- [ ] **Step 2: Commit**

```bash
git add compose.local.yml
git commit -m "refactor: rename compose.yml to compose.local.yml

Prepares for adding compose.dev.yml (Traefik-backed per-user instances).
The local compose file is unchanged — just renamed."
```

---

### Task 2: Create `compose.dev.yml`

**Files:**
- Create: `compose.dev.yml`
- Reference: `/home/nate/saas/compose.dev.yml` (SaaS project pattern)

- [ ] **Step 1: Create `compose.dev.yml`**

Write the following to `compose.dev.yml`:

```yaml
---
name: rusty-links-${USER}
services:
  postgres:
    image: postgres:17-alpine
    container_name: rusty-links-postgres-${USER}
    restart: unless-stopped
    environment:
      POSTGRES_USER: ${DB_USERNAME:-rustylinks}
      POSTGRES_PASSWORD: ${DB_PASSWORD:-changeme_secure_password_here}
      POSTGRES_DB: ${DB_NAME:-rustylinks}
    volumes:
      - postgres_data:/var/lib/postgresql/data
    expose:
      - "5432"
    healthcheck:
      test: ["CMD-SHELL", "pg_isready --username=${DB_USERNAME:-rustylinks}"]
      interval: 5s
      timeout: 5s
      retries: 5

  app:
    build:
      context: .
    container_name: rusty-links-app-${USER}
    restart: unless-stopped
    volumes:
      - ./src:/app/src
      - ./assets:/app/assets
      - ./migrations:/app/migrations
      - ./Cargo.toml:/app/Cargo.toml
      - ./Cargo.lock:/app/Cargo.lock
      - ./Dioxus.toml:/app/Dioxus.toml
      - ./static:/app/static
      - cargo_home:/root/.cargo
      - cargo_target:/app/target
    env_file: .env
    environment:
      RUST_LOG: debug
      RUST_BACKTRACE: 1
      DATABASE_URL: postgresql://${DB_USERNAME:-rustylinks}:${DB_PASSWORD:-changeme_secure_password_here}@postgres/${DB_NAME:-rustylinks}
    expose:
      - "4002"
    networks:
      - default
      - network-traefik-public
    depends_on:
      postgres:
        condition: service_healthy
    labels:
      - "traefik.enable=true"
      - "traefik.docker.network=network-traefik-public"
      - "traefik.http.routers.links-${USER}.rule=Host(`${USER}-links.a8n.run`)"
      - "traefik.http.routers.links-${USER}.service=links-${USER}@docker"
      - "traefik.http.routers.links-${USER}.entrypoints=web-secure"
      - "traefik.http.routers.links-${USER}.tls.certresolver=cert-cloudflare"
      - "traefik.http.services.links-${USER}.loadbalancer.server.port=4002"

volumes:
  cargo_home:
    name: rusty-links-cargo-${USER}
  cargo_target:
    name: rusty-links-target-${USER}
  postgres_data:
    name: rusty-links-postgres-${USER}

networks:
  network-traefik-public:
    external: true
```

- [ ] **Step 2: Verify YAML syntax**

```bash
docker compose -f compose.dev.yml config --quiet
```

Expected: No output (valid YAML). If there are errors, fix them.

- [ ] **Step 3: Commit**

```bash
git add compose.dev.yml
git commit -m "feat: add compose.dev.yml for per-user Traefik-backed dev instances

Each developer gets their own instance at {USER}-links.a8n.run.
Uses the same pattern as the SaaS project's compose.dev.yml:
- Traefik labels for automatic routing and TLS
- network-traefik-public for Traefik discovery
- expose (not ports) since Traefik handles external access
- All resources namespaced with \${USER} for isolation"
```

---

### Task 3: Update `justfile`

**Files:**
- Modify: `justfile:1-58`

- [ ] **Step 1: Add compose variable and update `dev` recipe**

At the top of the justfile, after the `default` recipe (after line 5), add the compose variable:

```just
# Use the per-user dev compose file
compose := "docker compose -f compose.dev.yml "
```

Then change the `dev` recipe (line 26-27) from:

```just
# Start development server in Docker (mode: standalone or saas)
dev mode="standalone": (ensure-env mode) css-build
    docker compose up --build --remove-orphans app
```

to:

```just
# Start development server in Docker (mode: standalone or saas)
dev mode="standalone": (ensure-env mode) css-build
    {{ compose }}up --build --remove-orphans app
```

- [ ] **Step 2: Add `dev-local` recipe**

After the `dev` recipe, add:

```just
# Start local development server in Docker — no Traefik, localhost ports (mode: standalone or saas)
dev-local mode="standalone": (ensure-env mode) css-build
    docker compose -f compose.local.yml up --build --remove-orphans app
```

- [ ] **Step 3: Update `db-up`, `db-down`, `down` recipes**

Change `db-up` (line 29-30) from:

```just
# Start PostgreSQL container
db-up:
    docker compose up --detach postgres
```

to:

```just
# Start PostgreSQL container
db-up:
    {{ compose }}up --detach postgres
```

Change `db-down` (line 33-34) from:

```just
# Stop PostgreSQL container
db-down:
    docker compose down postgres
```

to:

```just
# Stop PostgreSQL container
db-down:
    {{ compose }}down postgres
```

Change `down` (line 37-38) from:

```just
# Stop all containers
down:
    docker compose down
```

to:

```just
# Stop all containers
down:
    {{ compose }}down
```

- [ ] **Step 4: Update `clean` recipe**

Change the `clean` recipe (line 42-57). Replace `docker compose down --remove-orphans` with the compose variable. Since this is a nushell script block, the `{{ compose }}` interpolation works inside the `#!/usr/bin/env nu` block. Change line 44 from:

```
    docker compose down --remove-orphans
```

to:

```
    docker compose -f compose.dev.yml down --remove-orphans
```

Note: Just variables (`{{ compose }}`) are interpolated before the script runs, so this works inside nushell blocks. However, the trailing space in the `compose` variable could cause issues in the nu script. Use the explicit command instead for clarity in the nushell block.

- [ ] **Step 5: Verify justfile syntax**

```bash
just --list
```

Expected: All recipes listed without errors. Verify `dev`, `dev-local`, `db-up`, `db-down`, `down`, `clean` all appear.

- [ ] **Step 6: Commit**

```bash
git add justfile
git commit -m "feat: update justfile to use compose.dev.yml by default

- just dev now uses compose.dev.yml (Traefik-backed per-user instance)
- Added just dev-local for localhost-only development
- All docker compose recipes (db-up, db-down, down, clean) use dev compose"
```

---

### Task 4: Update `CLAUDE.md`

**Files:**
- Modify: `CLAUDE.md:88` (the reference to `compose.yml`)

- [ ] **Step 1: Update the compose.yml reference in the Configuration section**

In `CLAUDE.md`, line 88 references `compose.yml`. Update it to mention both compose files. Change:

```
IMPORTANT: When updating code, ALWAYS check if `.env.standalone` and `.env.saas` need to be updated. This applies when adding, removing, or renaming environment variables in `src/config.rs`, `compose.yml`, or any `std::env::var` call. Both files must stay in sync.
```

to:

```
IMPORTANT: When updating code, ALWAYS check if `.env.standalone` and `.env.saas` need to be updated. This applies when adding, removing, or renaming environment variables in `src/config.rs`, `compose.dev.yml`, `compose.local.yml`, or any `std::env::var` call. Both files must stay in sync.
```

- [ ] **Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: update CLAUDE.md to reference new compose file names"
```

---

### Task 5: Smoke test

- [ ] **Step 1: Validate compose.dev.yml renders correctly with USER substitution**

```bash
USER=$(whoami) docker compose -f compose.dev.yml config
```

Expected: Full rendered YAML with your username substituted into container names, volume names, Traefik labels, and the project name. Verify:
- `container_name: rusty-links-app-<your-username>`
- Traefik router rule contains `<your-username>-links.a8n.run`
- Network `network-traefik-public` is listed as external

- [ ] **Step 2: Validate compose.local.yml still works**

```bash
docker compose -f compose.local.yml config
```

Expected: Full rendered YAML matching the original `compose.yml` content. Verify ports are mapped to localhost (`127.0.0.1:5433:5432` for postgres, `0.0.0.0:4002:4002` for app).

- [ ] **Step 3: Verify just recipes parse**

```bash
just --list
```

Expected: All recipes listed. Confirm `dev`, `dev-local`, `db-up`, `db-down`, `down`, `clean` all present.
