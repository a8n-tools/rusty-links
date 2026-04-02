# Fix: Getting Started Links & Clean Recipe

**Date:** 2026-04-02
**Branch:** `fix/getting-started`
**PR:** #26 (merged into main)

## Problem

1. The standalone landing page (`static/index.html`) had hardcoded links pointing to the SaaS registration page (`pugtsurani.net/register`). Clicking "Get Started", "Create Account", or "Sign Up" in standalone mode sent users to an external SaaS page instead of the local auth flow.
2. When no admin account existed, all auth links pointed to `/login` instead of `/setup`, so new standalone users couldn't create their first account from the landing page.
3. The `just clean` recipe failed with "volume is in use" errors when orphan containers still held Docker volumes. It also listed two phantom volumes (`app-data`, `db-data`) that don't exist in either compose file.

## Changes

### `static/index.html`

- Replaced all 4 `https://pugtsurani.net/register` hrefs with `/login`
- Added `data-auth-link` attribute to all auth-related links (Log In, Get Started, Create Account, Sign Up)
- Added a `DOMContentLoaded` script that calls `GET /api/auth/check-setup` and rewrites all `[data-auth-link]` hrefs to `/setup` when `setup_required` is true

### `justfile`

- Removed phantom `app-data` and `db-data` volumes from the `clean` recipe (these only exist in `examples/compose.yml`, not in the dev compose files)
- Added force-removal of orphan containers holding a volume before calling `docker volume rm`

## Commits

1. `2ce30fe` — fix: point standalone landing page to /login and harden clean recipe
2. `3814590` — fix: redirect landing page to /setup when no admin account exists

## Notes

- `static/index.html` is only served in standalone mode (`#[cfg(feature = "standalone")]` in `src/main.rs:261`), so SaaS mode is unaffected
- The remaining `pugtsurani.net` links in the landing page are informational/branding links to the parent project, not auth flows
