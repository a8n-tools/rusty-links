# Per-User Dev Instances

**Date:** 2026-04-01
**Status:** Completed
**PR:** #25 (merged into main)

## Goal

Enable developers to spin up their own instance of rusty-links at `{USER}-links.a8n.run` on the shared dev server, following the same pattern used by the SaaS project (`{USER}-app.a8n.run`).

## Approach

Mirror the SaaS project's two-compose-file pattern:

- `compose.dev.yml` — Per-user, Traefik-backed (used on the shared dev server)
- `compose.local.yml` — Localhost ports, no Traefik (for local-only development)

The current `compose.yml` becomes `compose.local.yml` (unchanged). A new `compose.dev.yml` is created with Traefik integration.

## New File: `compose.dev.yml`

Based on the SaaS project's `compose.dev.yml`, adapted for rusty-links' single-service architecture (Dioxus serves both app and API on one port).

Key properties:

- **Project name:** `rusty-links-${USER}`
- **Container names:** `rusty-links-postgres-${USER}`, `rusty-links-app-${USER}`
- **Volume names:** `rusty-links-cargo-${USER}`, `rusty-links-target-${USER}`, `rusty-links-postgres-${USER}`
- **App service:** Uses `expose` (not `ports`). Traefik handles external routing.
- **Postgres service:** Uses `expose` (not `ports`). Only accessible from the app container via Docker networking.
- **Networks:** Joins external `network-traefik-public` for Traefik discovery. Uses `default` network for internal postgres communication.
- **Traefik labels on app service:**
  - `traefik.enable=true`
  - `traefik.docker.network=network-traefik-public`
  - `traefik.http.routers.links-${USER}.rule=Host(\`${USER}-links.a8n.run\`)`
  - `traefik.http.routers.links-${USER}.service=links-${USER}@docker`
  - `traefik.http.routers.links-${USER}.entrypoints=web-secure`
  - `traefik.http.routers.links-${USER}.tls.certresolver=cert-cloudflare`
  - `traefik.http.services.links-${USER}.loadbalancer.server.port=4002`
- **env_file:** `.env` (same as current). `DATABASE_URL` overridden in `environment:` to point at the compose postgres service.

## Local Compose File: `compose.yml`

Initially renamed to `compose.local.yml`, then renamed back to `compose.yml` (commit `df43ae1`) so `docker compose` works without `-f` for local development. Provides localhost-only development for developers not on the shared server.

## Justfile Changes

- Add `compose` variable: `compose := "docker compose -f compose.dev.yml "`
- `just dev` uses `compose.dev.yml` (via the variable)
- Add `just dev-local` recipe using `compose.local.yml`
- All other docker compose recipes (`db-up`, `db-down`, `down`, `clean`) switch to the `compose` variable

## Infrastructure

No changes needed:

- **DNS:** Wildcard `*.a8n.run` already configured in Cloudflare
- **TLS:** `cert-cloudflare` certresolver auto-issues certs via Let's Encrypt + Cloudflare DNS challenge
- **Traefik:** Already running on dev-01, `network-traefik-public` network exists
- **Dockerfiles:** No changes (dev or prod)
- **Application code / config.rs:** No changes
- **CI/CD:** No changes

## Deviations from Original Plan

- `compose.local.yml` was renamed back to `compose.yml` so that `docker compose` (no `-f`) works for local dev. The `just dev-local` recipe uses bare `docker compose` accordingly.
- `Dockerfile` required a fix: `dx serve` needed `--addr 0.0.0.0` to bind to all interfaces, otherwise Traefik couldn't reach the container (commit `372cea4`).

## Commits

1. `a36248b` — refactor: rename compose.yml to compose.local.yml
2. `1ea9af3` — feat: add compose.dev.yml for per-user Traefik-backed dev instances
3. `ac49443` — feat: update justfile to use compose.dev.yml by default
4. `891fe16` — docs: update CLAUDE.md to reference new compose file names
5. `df43ae1` — refactor: rename compose.local.yml back to compose.yml
6. `372cea4` — fix: bind dx serve to 0.0.0.0 and fix clean recipe for Traefik routing

## Not Changed

- `.env` example files
- `config.rs` / environment variable handling
- Production Dockerfile (`oci-build/Dockerfile`)
- Forgejo CI workflow
- `examples/compose.yml`
