# rusty-links Specification

## 1. Purpose and scope

rusty-links is a personal link manager: users save links, organize them with categories, tags, languages and licenses,
and the server keeps link metadata fresh in the background (page title, description, favicon, and GitHub repository
details). It is a full-stack Rust web application with a browser UI and a JSON API.

It does:

- Store each user's links with notes, status, and optional GitHub metadata, and let the user search, filter, sort,
  paginate, bulk edit, import and export them.
- Check links periodically and mark ones that stay unreachable as `inaccessible`.
- Authenticate users locally (standalone mode) or through an OpenID Connect provider (hosted mode).

It does not:

- Share links between users; every link belongs to one user.
- Fetch URLs that resolve to private, loopback or link-local addresses (section 7).

## 2. Architecture

A single Cargo package (edition 2021), a Dioxus 0.8 full-stack application with an axum server:

- `server` feature: the axum server, PostgreSQL access (sqlx), authentication, scheduler, scraper, GitHub client and
  mailer. `web` feature: the Dioxus browser build (WebAssembly).
- `src/main.rs`: connects to PostgreSQL, runs the embedded migrations from `migrations/`, optionally seeds a default
  admin, starts the scheduler, builds the router and serves.
- `src/api`: JSON API under `/api`. `src/server_functions`: Dioxus server functions for the UI's auth calls.
- `src/auth`: JWT and refresh tokens (standalone), OIDC relying party and resource server (hosted), login approval,
  known devices, location alert, trusted-proxy header normalization, mailer.
- `src/models`, `src/scraper`, `src/github`, `src/scheduler`, `src/security`, `src/error`.
- `src/ui`: Dioxus pages and components. Styling is Tailwind, built from `tailwind.css`.

The mode is decided at runtime: if `OIDC_ISSUER` is set the instance is hosted, otherwise it is standalone. Routes that do
not exist in a mode are not mounted and answer `404`.

Tech stack: Rust, Dioxus, axum, tower-http, sqlx (PostgreSQL), reqwest, scraper, argon2/bcrypt, jsonwebtoken, lettre,
moka, tracing.

## 3. Functional requirements

### 3.1 Links

- A user creates a link from a URL, optionally with category, tag, language and license ids. The URL is validated against
  the SSRF rules (section 7). The response is `201` with the created link.
- A link is unique per user by `(domain, path)`.
- A link has a status of `active`, `archived`, `inaccessible` or `repo_unavailable`; any other value is rejected.
- Listing supports a text query (title, description, URL, domain), filters (status, GitHub only, category, tag, language,
  license), sorting (`created_at`, `title`, `github_stars`, `status`, `updated_at`; ascending or descending, default
  descending) and pagination (1-indexed pages, default 20 per page, maximum 100).
- A link can be fetched, updated, deleted (`204`), refreshed (metadata scrape) and refreshed from GitHub.
- Categories, tags, languages and licenses can be attached to and detached from a link.
- Bulk operations: delete links, assign categories, assign tags.
- Export returns all of a user's data as JSON; import accepts JSON containing links.
- `check-duplicate` reports whether a URL is already saved; `preview` and `scrape` return scraped metadata for a URL
  without saving it.

### 3.2 Taxonomies

- Categories form a tree with a depth from 0 to 2 and an optional sort order; names are unique per user,
  case-insensitively. `GET /api/categories/tree` returns the tree.
- Tags, languages and licenses are unique per user by case-insensitive name. Languages and licenses also have global
  entries (no owner) seeded by the migrations. A license has an acronym `name` and a `full_name`.

### 3.3 Background scheduler

- Every `UPDATE_INTERVAL_HOURS` (default 24) with random jitter of up to `JITTER_PERCENT` (default 20) percent (never less
  than 60 seconds), the scheduler selects links not checked within `UPDATE_INTERVAL_DAYS` (default 30) in batches of
  `BATCH_SIZE` (default 50). For each: it checks accessibility, scrapes metadata, fetches GitHub metadata for repository
  links, and updates `last_checked`.
- A failed check increments `consecutive_failures`; after 3 consecutive failures the link becomes `inaccessible`. A
  successful check resets the counter and restores an `inaccessible` link to `active`.
- The same pass deletes old login attempts and expired refresh tokens, and in hosted mode expired OIDC flow and browser
  sessions.
- A task failure is logged and the loop continues.

### 3.4 Standalone accounts

- `POST /api/auth/setup` creates the first account (an administrator) and answers `401` once any user exists.
- `POST /api/auth/register` creates further accounts when `ALLOW_REGISTRATION` is true, otherwise `403`.
- Passwords need at least 8 characters, an uppercase letter, a digit and a special character.
- Login returns an access token (JWT) and a refresh token (stored as a SHA-256 hash). Failed logins are counted; an account
  with `ACCOUNT_LOCKOUT_ATTEMPTS` recent failures is locked for `ACCOUNT_LOCKOUT_DURATION` minutes.
- `GET /api/auth/check-setup` reports whether setup is required.
- Administrators list users, delete users and promote users.

### 3.5 Hosted accounts

- Sign-in is the OIDC authorization code flow with PKCE (`/oauth2/login`, `/oauth2/callback`, `/oauth2/logout`). A
  browser session is an opaque token in the `rl_session` cookie, stored hashed in `user_sessions`.
- The API also accepts a bearer `at+jwt` access token validated against the provider's keys (issuer, audience, signature).
- The server accepts back-channel logout and lifecycle event tokens from the provider, de-duplicated by token id, and
  provisions a local user on first sign-in.
- A signed webhook toggles maintenance mode; while on, requests are refused except health checks, OIDC paths, static assets and sessions that belong to an administrator.
- Page routes redirect unauthenticated users to the OIDC login.

### 3.6 Sign-in protection (standalone)

- A sign-in from a new country (read from `X-IPCountry`) or from a device the account has not used before (a random
  browser-held id, stored server-side only as its SHA-256) can be held until the owner opens a single-use emailed link
  (15 minutes). The gate is on only when `LOGIN_APPROVAL_ENABLED` is exactly `true`. A first sign-in is never held.
- A new-country sign-in sends an alert email when SMTP is configured, otherwise it is logged; an account can opt out.
- Account devices can be listed and revoked (`/api/auth/devices`).

## 4. Interfaces

All API routes are under `/api`. Authentication: standalone, a bearer JWT; hosted, the `rl_session` cookie or a bearer
`at+jwt`. Unauthenticated calls to protected routes answer `401`.

| Route                                                        | Methods                 | Mode        | Auth |
| ------------------------------------------------------------ | ----------------------- | ----------- | ---- |
| `/health`, `/health/database`, `/health/scheduler`           | GET                     | both        | none |
| `/auth/setup`, `/auth/register`, `/auth/login`, `/auth/refresh`, `/auth/logout`, `/auth/check-setup` | POST (GET for check-setup) | standalone | none / refresh token |
| `/auth/me`                                                   | GET, PATCH              | both        | yes  |
| `/auth/devices`, `/auth/devices/{id}`                        | GET, DELETE             | both        | yes  |
| `/links`                                                     | GET, POST               | both        | yes  |
| `/links/{id}`                                                | GET, PUT, DELETE        | both        | yes  |
| `/links/check-duplicate`, `/links/export`                    | GET                     | both        | yes  |
| `/links/preview`, `/links/import`, `/links/bulk/delete`, `/links/bulk/categories`, `/links/bulk/tags` | POST | both | yes |
| `/links/{id}/refresh`, `/links/{id}/refresh-github`          | POST                    | both        | yes  |
| `/links/{id}/categories`, `/tags`, `/languages`, `/licenses` | GET, POST               | both        | yes  |
| `/links/{id}/categories/{category_id}`, `/tags/{tag_id}`, `/languages/{language_id}`, `/licenses/{license_id}` | DELETE | both | yes |
| `/categories`, `/categories/tree`, `/categories/{id}`        | GET, POST; GET; GET, PUT, DELETE | both | yes |
| `/tags`, `/languages`, `/licenses`                           | GET, POST               | both        | yes  |
| `/tags/{id}`, `/languages/{id}`, `/licenses/{id}`            | DELETE                  | both        | yes  |
| `/scrape`                                                    | POST                    | both        | yes  |
| `/admin/users`, `/admin/users/{user_id}`, `/admin/users/{user_id}/promote` | GET; DELETE; POST | standalone | admin |
| `/webhooks/maintenance`                                      | POST                    | hosted      | HMAC signature header `X-Webhook-Signature` |

Outside `/api`: `/oauth2/login`, `/oauth2/callback`, `/oauth2/logout` (GET), `/oauth2/backchannel-logout`,
`/oauth2/lifecycle-event` (POST) in hosted mode; `/auth/approve-login` (GET, POST) in standalone mode; `/dev/seed-session`
and `/dev/logout` in hosted debug builds; the Dioxus UI pages for everything else, plus a landing page at `/` in
standalone mode.

`GET /api/health` returns status, version, build information and the auth mode (`standalone` or `hosted`).

## 5. Data model

PostgreSQL. Migrations are in `migrations/` and are applied at startup with `sqlx::migrate!`.

| Table                     | Key fields                                                                                           |
| ------------------------- | ---------------------------------------------------------------------------------------------------- |
| `users`                   | `id` UUID, `email` (unique), `password_hash`, `name`, `is_admin`, `saas_user_id` (unique when set), `suspended_at`, `session_version`, `last_login_country`, `notify_new_location`. |
| `links`                   | `id`, `user_id` (cascade), `url`, `domain`, `path`, `title`, `description`, `logo`, `source_code_url`, `documentation_url`, `notes`, `status`, `github_stars`, `github_archived`, `github_last_commit`, `is_github_repo`, `consecutive_failures`, `last_checked`, `refreshed_at`; unique `(user_id, domain, path)`. |
| `categories`              | `user_id`, `name`, `parent_id`, `depth` (0 to 2), `sort_order`.                                      |
| `languages`, `licenses`, `tags` | `user_id` (nullable for global entries), `name` (licenses also `full_name`).                   |
| `link_categories`, `link_languages`, `link_licenses`, `link_tags` | Join tables (cascade); the language, license and tag joins carry `order_num`. |
| `refresh_tokens`          | `user_id`, `token_hash` (unique), `expires_at`.                                                      |
| `login_attempts`          | `email`, `attempted_at`, `success`.                                                                  |
| `rp_sessions`             | `state` (unique), `nonce`, `code_verifier`, `return_to`, `expires_at`.                               |
| `user_sessions`           | `session_token_hash` (unique), `user_id`, `session_version`, `auth_via_oidc`, `expires_at`.          |
| `pending_login_approvals` | `user_id`, `token_hash` (unique), `country`, `ip`, `device`, `device_id_hash`, `reason`, `expires_at`, `consumed_at`. |
| `known_devices`           | `user_id`, `device_id_hash`, `first_seen_at`, `last_seen_at`; unique `(user_id, device_id_hash)`.     |

## 6. Configuration

Environment variables, with a `.env` file loaded if present. `.env.standalone.example` and `.env.saas.example` are the
per-mode templates.

| Variable                                   | Mode       | Default / rule                                              |
| ------------------------------------------ | ---------- | ----------------------------------------------------------- |
| `DATABASE_URL`                             | both       | Required.                                                   |
| `APP_PORT`                                 | both       | Required, must parse as a port.                             |
| `HOST_IP`                                  | both       | Optional bind address.                                      |
| `HOST_URL`                                 | both       | `http://localhost:<APP_PORT>`.                              |
| `RUST_LOG`                                 | both       | `info`.                                                     |
| `UPDATE_INTERVAL_DAYS`                     | both       | 30; at least 1; unparseable is an error.                    |
| `UPDATE_INTERVAL_HOURS`                    | both       | 24; at least 1; unparseable is an error.                    |
| `BATCH_SIZE`                               | both       | 50; at least 1; unparseable is an error.                    |
| `JITTER_PERCENT`                           | both       | 20; 0 to 100; unparseable is an error.                      |
| `JWT_SECRET`                               | standalone | Random per start when unset (warning); tokens do not survive a restart. |
| `JWT_EXPIRY`                               | standalone | 1 (hours).                                                  |
| `REFRESH_TOKEN_EXPIRY`                     | standalone | 7 (days).                                                   |
| `ACCOUNT_LOCKOUT_ATTEMPTS`                 | standalone | 5.                                                          |
| `ACCOUNT_LOCKOUT_DURATION`                 | standalone | 30 (minutes).                                               |
| `ALLOW_REGISTRATION`                       | standalone | true.                                                       |
| `SETUP_DEFAULT_ADMIN_EMAIL`, `SETUP_DEFAULT_ADMIN_PASSWORD`, `SETUP_DEFAULT_ADMIN_NAME` | standalone | Create an administrator at start when no users exist; name defaults to `Admin`. |
| `OIDC_ISSUER`                              | hosted     | Setting it selects hosted mode.                             |
| `OIDC_CLIENT_ID`, `OIDC_CLIENT_SECRET`     | hosted     | Required when `OIDC_ISSUER` is set (startup fails otherwise). |
| `OIDC_AUDIENCE`                            | hosted     | Expected `aud` of access tokens (has a built-in default).   |
| `OIDC_JWKS_URL`                            | hosted     | Derived from the issuer; must be HTTPS (or localhost).      |
| `OIDC_JWKS_CACHE_TTL`                      | hosted     | 300 seconds.                                                |
| `OIDC_REDIRECT_URI`, `OIDC_POST_LOGOUT_REDIRECT_URI` | hosted | `<HOST_URL>/oauth2/callback`, `<HOST_URL>/`.            |
| `OIDC_LEEWAY_SECONDS`                      | hosted     | 30.                                                         |
| `OIDC_LIFECYCLE_JTI_CACHE_TTL`             | hosted     | 300 seconds.                                                |
| `OIDC_SESSION_TTL_SECONDS`                 | hosted     | 1209600 (14 days).                                          |
| `WEBHOOK_SECRET`                           | hosted     | HMAC secret for the maintenance webhook; empty with a warning. |
| `SMTP_HOST`, `SMTP_PORT`, `SMTP_USERNAME`, `SMTP_PASSWORD`, `SMTP_FROM_EMAIL`, `SMTP_FROM_NAME` | both | Mail is sent only when configured; otherwise alerts are logged. |
| `SMTP_TLS`                                 | both       | `starttls` default; `tls` or `none`; an unrecognized value warns and uses `starttls`. |
| `GITHUB_TOKEN`                           | both       | Optional; when non-empty it authenticates GitHub API requests for a higher rate limit. |
| `LOGIN_LOCATION_ALERTS_ENABLED`            | both       | On unless set to a false value.                             |
| `LOGIN_APPROVAL_ENABLED`                   | standalone | Off unless exactly `true`.                                  |
| `TRUSTED_PROXY_CIDRS`                      | both       | Empty: forwarded address and country headers are ignored.   |
| Build information (`BUILD_VERSION`, `BUILD_GIT_HASH`, `BUILD_DATE`) | build | Baked into the binary; shown by `/api/health`. |

## 7. Security

- URLs submitted for saving, scraping or previewing are checked before any fetch: only `http` and `https` are allowed, a
  hostname is required, the hostname is resolved, and a URL resolving to a loopback, unspecified, private or link-local
  address (including the cloud metadata address) is rejected.
- Passwords are hashed with argon2 (bcrypt hashes are also verified). Refresh tokens and session tokens are stored only as
  hashes.
- Every per-user resource is scoped to the authenticated user.
- Forwarded client address and country headers are read only from sockets inside `TRUSTED_PROXY_CIDRS`; otherwise they are
  dropped before any handler runs.
- Hosted-mode bearer tokens are verified against the provider's JWKS; the maintenance webhook by HMAC-SHA256.
- Account lockout applies to standalone login.

## 8. Integrations and dependencies

- PostgreSQL.
- An OpenID Connect provider (hosted mode).
- The GitHub API (repository metadata) and arbitrary websites (title, description, favicon).
- An SMTP server (optional).
- A reverse proxy supplying forwarded address and country headers (optional).

## 9. Deployment and operations

- `oci-build/Dockerfile` builds the Dioxus web and server bundle in a Rust builder stage and runs it in a Debian slim stage
  as an unprivileged user, exposing 8080 with a health check on `http://localhost:8080/api/health`; the entry point is
  `/app/rusty-links`. `APP_PORT` has no default and must be set to the port the container should listen on.
- `compose.yml` and `compose.dev.yml` define a PostgreSQL service and the app; `compose.dev.yml` mounts the source and
  runs the development server on 4002.
- Migrations run at start. The scheduler runs inside the server process and stops on shutdown.

## 10. Non-functional behavior

- Logging: `tracing` with the `RUST_LOG` filter. A database connection failure at start logs the likely causes and stops
  the server.
- Errors are returned as typed application errors with matching HTTP status codes.
- `GET /api/health/database` and `GET /api/health/scheduler` expose database and scheduler state for monitoring.
