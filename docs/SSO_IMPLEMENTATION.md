# SSO Implementation — Rusty Links

**Status:** Design / planning
**Branch:** `feature-sso` (do not merge to `main` until plan is complete)
**Protocol:** OpenID Connect Core 1.0 over OAuth 2.0 Authorization Code + PKCE
**Author date:** 2026-04-23

---

## 1. Summary

This document describes how Rusty Links migrates from the legacy SaaS mode (shared HS256 JWT
cookie) to the same OIDC-based SSO architecture used by DMARC Reporter. The full design spec for
the SSO system lives in `saas/docs/SSO Implementation.md` and `dmarc-reporter/docs/SSO
Implementation.md`. This document covers only the **Rusty-Links-specific** implementation details
and the delta from the dmarc-reporter implementation.

After this change:

- `saas` acts as the **OpenID Provider (OP)** — unchanged from the shared SSO plan.
- Rusty Links becomes an **OIDC Relying Party (RP)** for browser login (Authorization Code + PKCE
  via a Backend-for-Frontend) and an **OAuth 2.0 Resource Server (RS)** for its `/api/*` surface.
- The `standalone` feature (local Argon2id + JWT auth) is untouched.
- The old `saas` mode (shared HS256 cookie) is replaced entirely; its env vars and code are
  removed.

---

## 2. Current state

### 2.1 saas mode today

- `src/auth/saas_auth.rs` — reads the `access_token` cookie set by the SaaS on `.a8n.tools` and
  validates it with a **shared HS256 secret** (`SAAS_JWT_SECRET`). Extracts `user_id`, `email`,
  `membership_status`, `is_admin`.
- `src/main.rs` — an Axum middleware wraps every protected page route, reads the cookie, and
  redirects unauthenticated requests to `SAAS_LOGIN_URL`.
- `src/auth/middleware.rs` — `AuthenticatedUser` extractor reads the same cookie, ensures the user
  exists in the local `users` table (JIT INSERT … ON CONFLICT), and checks membership status.
- `src/config.rs` — `#[cfg(feature = "saas")]` fields: `saas_login_url`, `host_url`,
  `saas_jwt_secret`, `saas_logout_url`, `saas_membership_url`, `saas_refresh_url`.

### 2.2 Why this is a problem

The shared HS256 secret means any service that can verify a token can also mint one. That violates
the SSO security model described in `saas/docs/SSO Implementation.md §3.3` and §9.4 — the
algorithm is symmetric and the secret leaves the IdP.

### 2.3 standalone mode today (remains unchanged)

- Local Argon2id password hashing, HS256 JWT (short-lived access token + `refresh_tokens` table).
- `src/auth/jwt.rs`, `src/auth/middleware.rs` (`#[cfg(feature = "standalone")]`).
- `src/api/auth.rs` — `/api/auth/{login, logout, refresh, register, me}`.

---

## 3. Architecture overview

```
Browser                          Rusty Links (BFF + RS)           SaaS OP
  │                                      │                          │
  │  GET /oauth2/login                   │                          │
  │ ────────────────────────────────────▶│                          │
  │                                      │  302 /oauth2/authorize   │
  │ ◀────────────────────────────────────│─────────────────────────▶│
  │                                      │                          │
  │  (user authenticates at SaaS)        │                          │
  │                                      │ ◀────────────────────────│
  │  GET /oauth2/callback?code=…         │                          │
  │ ────────────────────────────────────▶│                          │
  │                                      │  POST /oauth2/token      │
  │                                      │─────────────────────────▶│
  │                                      │ ◀── {access, id, refresh}│
  │                                      │  verify id_token (JWKS)  │
  │                                      │  JIT-provision user row  │
  │  session cookie ◀───────────────────│                          │
  │  302 to /links                       │                          │
  │                                      │                          │
  │  GET /api/links                      │                          │
  │  Authorization: Bearer <at+jwt>      │                          │
  │  (or HttpOnly session cookie)        │                          │
  │ ────────────────────────────────────▶│  verify at+jwt (JWKS)    │
  │ ◀── 200 JSON                         │                          │
```

The BFF and the RS run in the same binary. Browser-facing pages use the HttpOnly session cookie
path; the `/api/*` surface also accepts a Bearer `at+jwt` from the BFF (forwarded internally) or
from native/desktop clients.

---

## 4. Client registration in SaaS

A new `confidential` OIDC client needs to be registered in the SaaS `oauth_clients` table. Add a
row to the seed migration `api/migrations/20260417000040_create_oidc_clients.sql` in the `saas`
repo (on `feature-sso`):

```sql
-- Rusty Links BFF
-- See saas/api/migrations/20260420000044_register_rustylinks_oidc_client.sql
(
    'a8000000-0000-0000-0000-000000000005',  -- client_id UUID
    'confidential',
    'rustylinks-web-bff',
    ARRAY['https://links.a8n.run/oauth2/callback', 'http://localhost:4002/oauth2/callback'],
    ARRAY['https://links.a8n.run/', 'http://localhost:4002/'],
    'https://links.a8n.run/oauth2/backchannel-logout',
    'https://links.a8n.run/oauth2/lifecycle-event',
    ARRAY['openid', 'email', 'offline_access'],
    ARRAY['authorization_code', 'refresh_token'],
    'client_secret_basic', TRUE,
    'https://links.a8n.run/api'
)
```

> The client secret must be set via admin CLI before production use. See `saas/docs/SSO
> Implementation.md §8` for the registration rules.

Unlike DMARC Reporter, Rusty Links does not request `dmarc:read` / `dmarc:write` scopes — only
`openid email offline_access`.

---

## 5. Database changes (rusty-links)

Three new migrations are needed. Rusty Links uses plain SQLx migrations in `migrations/`.

### 5.1 `migrations/20260417000009_add_sso_fields.sql`

```sql
-- Link local users to their SaaS identity (set on JIT provision; NULL for standalone users).
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS saas_user_id UUID,
    ADD COLUMN IF NOT EXISTS suspended_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS session_version INT NOT NULL DEFAULT 0;

-- Unique index so we can look up a user by their SaaS sub without a full-table scan.
CREATE UNIQUE INDEX IF NOT EXISTS users_saas_user_id_unique
    ON users(saas_user_id)
    WHERE saas_user_id IS NOT NULL;
```

### 5.2 `migrations/20260417000010_create_rp_sessions.sql`

```sql
-- Transient PKCE / state storage for the BFF Authorization Code flow.
-- Rows are deleted on successful callback and expire after 10 minutes.
CREATE TABLE IF NOT EXISTS rp_sessions (
    id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    state          TEXT        NOT NULL UNIQUE,
    nonce          TEXT        NOT NULL,
    code_verifier  TEXT        NOT NULL,
    return_to      TEXT,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at     TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS rp_sessions_expires
    ON rp_sessions(expires_at);
```

### 5.3 `migrations/20260417000011_create_user_sessions.sql`

```sql
-- Long-lived BFF sessions.  The rl_session cookie value is SHA-256-hashed
-- before storage so a DB leak cannot be used to forge a cookie.
-- session_version is a snapshot of users.session_version at login time.
-- On back-channel logout or suspension the users.session_version is incremented;
-- any row where user_sessions.session_version < users.session_version is stale.
CREATE TABLE IF NOT EXISTS user_sessions (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    session_token_hash  BYTEA       NOT NULL UNIQUE,   -- SHA-256 of raw cookie value
    user_id             UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_version     INT         NOT NULL,           -- snapshot at login
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at          TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS user_sessions_user_id
    ON user_sessions(user_id);
CREATE INDEX IF NOT EXISTS user_sessions_expires
    ON user_sessions(expires_at);
```

No changes to the `refresh_tokens` or `login_attempts` tables — those belong to the `standalone`
path and are left untouched.

---

## 6. Configuration changes

### 6.1 Env vars removed (saas mode)

| Removed variable       | Replaced by              |
|------------------------|--------------------------|
| `SAAS_JWT_SECRET`      | `OIDC_ISSUER` + JWKS     |
| `SAAS_LOGIN_URL`       | `OIDC_REDIRECT_URI` (implicit) |
| `SAAS_LOGOUT_URL`      | OIDC end_session_endpoint (from discovery) |
| `SAAS_MEMBERSHIP_URL`  | `has_member_access` ID token claim |
| `SAAS_REFRESH_URL`     | handled internally by BFF |

### 6.2 New env vars (saas mode, `AUTH_MODE=oidc`)

| Variable                          | Default / example                              | Purpose |
|-----------------------------------|------------------------------------------------|---------|
| `OIDC_ISSUER`                     | `https://api.a8n.tools`                        | Expected `iss`; setting this enables OIDC mode |
| `OIDC_AUDIENCE`                   | `https://links.a8n.run/api`             | RS audience for `at+jwt` verification |
| `OIDC_JWKS_URL`                   | `{OIDC_ISSUER}/.well-known/jwks.json`          | Override for pinning; derived from discovery if empty |
| `OIDC_JWKS_CACHE_TTL`             | `300`                                          | Seconds to cache JWKS in memory |
| `OIDC_CLIENT_ID`                  | `a8000000-0000-0000-0000-000000000005`          | UUID of `rustylinks-web-bff` |
| `OIDC_CLIENT_SECRET`              | *(mounted as Docker secret)*                   | Confidential client secret |
| `OIDC_REDIRECT_URI`               | `https://links.a8n.run/oauth2/callback` | Must exactly match client registration |
| `OIDC_POST_LOGOUT_REDIRECT_URI`   | `https://links.a8n.run/`                | Must exactly match client registration |
| `OIDC_LEEWAY_SECONDS`             | `30`                                           | Clock-skew tolerance on token timestamps |
| `OIDC_LIFECYCLE_JTI_CACHE_TTL`   | `300`                                          | Idempotency window for lifecycle event `jti` (seconds) |
| `OIDC_SESSION_TTL_SECONDS`        | `1209600` (14 days)                            | Idle lifetime of a BFF `rl_session` cookie |

`HOST_URL` is kept (used to build the BFF callback URL in dev when `OIDC_REDIRECT_URI` is not set).

### 6.3 `.env.saas` template (updated)

```sh
DATABASE_URL=postgresql://rusty_links:password@localhost:5432/rusty_links
APP_PORT=4002
HOST_URL=http://localhost:4002

# OIDC / SSO
OIDC_ISSUER=https://api.a8n.tools
OIDC_AUDIENCE=https://links.a8n.run/api
OIDC_CLIENT_ID=a8000000-0000-0000-0000-000000000005
OIDC_CLIENT_SECRET=<set via Docker secret in production>
OIDC_REDIRECT_URI=http://localhost:4002/oauth2/callback
OIDC_POST_LOGOUT_REDIRECT_URI=http://localhost:4002/
```

---

## 7. Code changes

### 7.1 `src/config.rs`

Replace the `#[cfg(feature = "saas")]` fields with an `OidcConfig` struct (pattern from
`dmarc-reporter/src/config.rs` on `feature-sso`):

```rust
/// OIDC Relying Party + Resource Server configuration.
/// Active when `OIDC_ISSUER` is set.
#[derive(Debug, Clone)]
pub struct OidcConfig {
    /// Issuer URL.  Setting this enables OIDC mode.
    pub issuer: Option<String>,
    /// `aud` value expected in `at+jwt` access tokens.
    pub audience: String,
    /// JWKS endpoint (derived from issuer if empty).
    pub jwks_url: String,
    /// JWKS in-memory cache TTL (seconds).
    pub jwks_cache_ttl: u64,
    /// OAuth2 client_id.
    pub client_id: String,
    /// OAuth2 client_secret (confidential client).
    pub client_secret: String,
    /// Absolute redirect URI for the BFF callback.
    pub redirect_uri: String,
    /// Post-logout redirect URI.
    pub post_logout_redirect_uri: String,
    /// Clock-skew leeway in seconds.
    pub leeway_seconds: u64,
}

impl OidcConfig {
    /// Returns true when OIDC mode is active.
    pub fn enabled(&self) -> bool {
        self.issuer.as_deref().map_or(false, |s| !s.is_empty())
    }
}
```

Remove the old `saas_login_url`, `host_url`, `saas_jwt_secret`, `saas_logout_url`,
`saas_membership_url`, `saas_refresh_url` fields from `Config`.

Add `pub oidc: OidcConfig` to `Config`.

Fail-fast validation: if the `saas` feature is active and `OIDC_ISSUER` is set but
`OIDC_CLIENT_ID` or `OIDC_CLIENT_SECRET` is empty, refuse to start.

### 7.2 `src/auth/saas_auth.rs` — remove

This file implements the HS256 shared-cookie approach and is replaced entirely. Delete it.
Update `src/auth/mod.rs` to remove the `pub mod saas_auth` export.

### 7.3 `src/auth/oidc_rp/` — new module (BFF)

Model directly on `dmarc-reporter/src/oidc_rp/` on `feature-sso`. Because Rusty Links uses SQLx
(not SeaORM) the DB queries use `sqlx::query!` macros.

```
src/auth/oidc_rp/
    mod.rs          — public API, Axum router
    jit.rs          — JIT provisioning
```

#### `src/auth/oidc_rp/mod.rs`

Routes (all under `#[cfg(feature = "saas")]`):

| Method | Path                        | Behavior |
|--------|-----------------------------|----------|
| GET    | `/oauth2/login`             | Generate PKCE materials, store `rp_sessions` row, 302 to SaaS authorize endpoint |
| GET    | `/oauth2/callback`          | Exchange code, verify ID token, JIT-provision user, create session cookie, 302 to `return_to` |
| GET    | `/oauth2/logout`            | Clear session cookie, 302 to SaaS `end_session_endpoint` |
| POST   | `/oauth2/backchannel-logout`| Receive logout token, increment `session_version` for matching user |
| POST   | `/oauth2/lifecycle-event`   | Receive lifecycle event token, apply `user.suspended` / `user.deleted` / etc. |

The session cookie strategy mirrors dmarc-reporter:
- Name: `rl_session` (to avoid conflicts with the SaaS `access_token` cookie)
- Flags: `HttpOnly; Secure; SameSite=Lax; Path=/; Domain=links.a8n.run`
- Value: opaque random 256-bit ID stored server-side

Because Rusty Links does not have a separate tenant concept, back-channel logout and lifecycle
events only need to operate on the `users` table.

**Session store (`user_sessions` table):**

`rp_sessions` (§5.2) stores only the transient PKCE state during the login flow and is deleted on
callback. Long-lived BFF sessions live in `user_sessions` (§5.3):

1. **On successful callback**: generate a 256-bit random `session_token`, compute `SHA-256(token)`,
   insert a `user_sessions` row with `session_version = users.session_version`, set the `rl_session`
   cookie to the raw token.
2. **On every authenticated request**: hash the cookie value, look up the row, check
   `user_sessions.session_version == users.session_version` and `expires_at > now()`. If either
   fails, treat as unauthenticated.
3. **On back-channel logout or `user.suspended` event**: `UPDATE users SET session_version =
   session_version + 1`. All existing `user_sessions` rows are stale instantly — no row-by-row
   delete required.
4. **On explicit logout**: delete the `user_sessions` row for the current session only.

This mirrors how dmarc-reporter uses axum-login's `session_auth_hash()` (which returns
`session_version_bytes`) but without the axum-login dependency.

> `rp_sessions` rows that never reach the callback (user abandoned the login) are not cleaned up
> by the BFF. Add a short periodic SQL purge to the existing `Scheduler` in
> `src/scheduler/mod.rs`:
> ```sql
> DELETE FROM rp_sessions  WHERE expires_at < NOW();
> DELETE FROM user_sessions WHERE expires_at < NOW();
> ```
> This runs on the same interval tick as the existing metadata-update job.

#### `src/auth/oidc_rp/jit.rs`

```rust
pub struct ProvisionedUser {
    pub id: uuid::Uuid,
    pub is_admin: bool,
}

/// Load existing user by saas_user_id or JIT-provision one on first login.
///
/// Errors:
/// - Forbidden  if email is not verified.
/// - Forbidden  if has_member_access is false.
/// - BadRequest if sub is not a valid UUID.
pub async fn load_or_provision(
    pool: &sqlx::PgPool,
    id_claims: &IdTokenClaims,
) -> Result<ProvisionedUser, AppError>;
```

Differences from the dmarc-reporter implementation:

1. No tenant creation — Rusty Links has no multi-tenancy.
2. The `password_hash` column is `NOT NULL` in the current schema; set it to the sentinel value
   `"!sso:no-password"` for OIDC-provisioned users (same pattern as dmarc-reporter).
3. Admin flag: the `is_admin` field stays as-is in the database; on every JIT update, sync
   `is_admin` from the ID token's `role` claim (`"admin"` = true). The SaaS OP sets the `role`
   claim on the ID token based on `users.role`.

```sql
-- JIT provision (first login)
INSERT INTO users (id, email, password_hash, name, is_admin, saas_user_id)
VALUES ($1, $2, '!sso:no-password', $3, $4, $5)
ON CONFLICT (saas_user_id) DO UPDATE
    SET email    = EXCLUDED.email,
        is_admin = EXCLUDED.is_admin
RETURNING id, is_admin;
```

### 7.4 `src/auth/oidc_rs.rs` — new module (Resource Server verifier)

The RS verifier validates `Authorization: Bearer <at+jwt>` on `/api/*` routes. Model on
`dmarc-reporter/src/oidc_rs/mod.rs` on `feature-sso` — copy the Ed25519 SPKI PEM reconstruction
helper (`ed25519_spki_pem_from_x`) verbatim; it has no external dependencies beyond `base64`
(already in `Cargo.toml`).

```rust
/// Claims from a validated RFC 9068 at+jwt access token.
pub struct AtClaims {
    pub sub: uuid::Uuid,   // SaaS user UUID
    pub scope: String,
    pub exp: i64,
    pub iat: i64,
}

/// Shared verifier — one instance, created at startup, stored in AppState.
pub struct OidcVerifier {
    config: OidcConfig,
    http: reqwest::Client,
    cache: Arc<tokio::sync::RwLock<Option<JwksCache>>>,
}

/// Axum extractor: validates the Bearer token, returns AtClaims.
/// Returns 401 if the token is missing, malformed, expired, or fails
/// any of the checks in §5.7 of the SSO spec.
pub struct BearerUser(pub AtClaims);
```

`OidcVerifier` is constructed in `main.rs` and stored as `Arc<OidcVerifier>` in the Axum state
(or `axum::Extension`). The `BearerUser` extractor reads it from the state via
`axum::extract::Extension::<Arc<OidcVerifier>>`.

Validation checklist (per `saas/docs/SSO Implementation.md §5.7`):
1. `Authorization: Bearer` header present.
2. Parse JWT header; reject if `typ != "at+jwt"`.
3. Algorithm must be `EdDSA` — set `validation.algorithms = vec![Algorithm::EdDSA]` before
   decoding; never accept `HS256` or `alg: none`.
4. `kid` present in header; resolve to a key in the JWKS cache. If `kid` is unknown, refresh the
   cache once, then 401.
5. Signature verifies against the resolved key.
6. `iss` equals `OidcConfig::issuer`.
7. `aud` equals `OidcConfig::audience`.
8. `exp > now() - leeway` and `nbf <= now() + leeway`.

JWKS cache: `Arc<tokio::sync::RwLock<Option<JwksCache>>>` where `JwksCache` holds
`HashMap<kid, DecodingKey>` and a `refreshed_at: DateTime<Utc>`. Refresh interval:
`OIDC_JWKS_CACHE_TTL` seconds (default 300). Use `reqwest` (already in non-WASM deps) for the
JWKS fetch.

### 7.5 `src/auth/middleware.rs` — update `AuthenticatedUser`

The `#[cfg(all(feature = "saas", not(feature = "standalone")))]` block currently reads the HS256
cookie. Replace it with a block that:

1. Check for `rl_session` cookie. If present:
   a. Hash the cookie value with SHA-256.
   b. Query `user_sessions` by `session_token_hash`.
   c. If row missing or `expires_at <= now()` → `SessionExpired`.
   d. Query `users` where `id = row.user_id`. Check `users.session_version == row.session_version`
      and `users.suspended_at IS NULL`. If either fails → `SessionExpired`.
   e. Return `AuthenticatedUser { user_id: row.user_id }`.
2. If no session cookie, check `Authorization: Bearer <at+jwt>`:
   a. Call `OidcVerifier::verify_bearer` (§7.4).
   b. Resolve `sub` → `users.id` via `saas_user_id`. Check `suspended_at IS NULL`.
   c. Return `AuthenticatedUser { user_id }`.
3. If neither, return `AppError::SessionExpired`.

Remove membership gating via `membership_status` — enforced at the SaaS authorization endpoint
(`has_member_access` checked during JIT provisioning, §7.3). A user already past JIT has passed
the membership gate.

### 7.6 `src/main.rs` — update the Dioxus middleware (saas feature)

The Axum middleware that protects page routes currently:
1. Reads the `access_token` cookie with `saas_auth::get_user_from_cookie`.
2. Redirects to `SAAS_LOGIN_URL` when unauthenticated.

Replace this with:
1. Check for a valid `rl_session` cookie (see §7.5).
2. When unauthenticated, redirect to `/oauth2/login?return_to=<current_path>` (the BFF login
   handler, not directly to the SaaS).

Remove the `/logout` path handler that currently forwards to `SAAS_LOGOUT_URL`. Logout is now
handled at `GET /oauth2/logout` (§7.3).

Remove the `/saas-refresh.js` route and the `assets/saas-refresh.js` script. Token refresh is
handled server-side by the BFF; the browser never touches a token.

The maintenance mode middleware needs to be updated: instead of reading the HS256 cookie, read the
`rl_session` cookie and resolve `is_admin` from the associated `users` row.

### 7.7 `src/api/mod.rs` — update `create_router`

Register the OIDC RP routes and add the `OidcVerifier` + JTI cache to the router state under
`#[cfg(feature = "saas")]`:

```rust
#[cfg(feature = "saas")]
{
    let oidc_verifier = Arc::new(OidcVerifier::new(config.oidc.clone()));
    let jti_cache: Arc<moka::future::Cache<String, ()>> = Arc::new(
        moka::future::Cache::builder()
            .time_to_live(Duration::from_secs(config.oidc.lifecycle_jti_cache_ttl))
            .build()
    );
    let oidc_router = crate::auth::oidc_rp::create_router(
        pool.clone(),
        config.oidc.clone(),
        oidc_verifier.clone(),
        jti_cache.clone(),
    );
    router = router
        .merge(oidc_router)
        .layer(axum::Extension(oidc_verifier))
        .layer(axum::Extension(jti_cache));
}
```

The `oidc_router` registers `/oauth2/{login,callback,logout,backchannel-logout,lifecycle-event}`.

### 7.8 `src/api/auth.rs` — disable registration endpoints in saas mode

The `/api/auth/check-setup`, `/api/auth/setup`, and `/api/auth/register` endpoints make no sense
in OIDC mode (the SaaS OP owns user creation). Gate them with `#[cfg(feature = "standalone")]` or
return `404` / `405` when called in saas mode.

The `/api/auth/me` endpoint (returns the current user's profile) must remain active in both modes.
In saas mode it reads the `AuthenticatedUser` extractor (§7.5) and queries the local `users` table.

---

## 8. ID token claims expected from SaaS

The SaaS OP must include these claims in the ID token when the `email` scope is granted (as it
always will be for `rustylinks-web-bff`):

| Claim                 | Type    | Description |
|-----------------------|---------|-------------|
| `sub`                 | UUID    | Stable SaaS user identifier. |
| `email`               | string  | User's email address. |
| `email_verified`      | bool    | Must be `true` to allow JIT provisioning. |
| `membership_status`   | string  | `"active"`, `"grace_period"`, etc. |
| `has_member_access`   | bool    | Convenience flag — `true` if subscription allows access. |
| `role`                | string  | `"admin"` or `"subscriber"`. Used to sync `users.is_admin`. |

These match what the SaaS OP already includes for DMARC Reporter; no SaaS-side changes needed.

---

## 9. Lifecycle event handling

Rusty Links handles the same five event types as DMARC Reporter but without tenant logic:

| Event type            | Action in Rusty Links |
|-----------------------|-----------------------|
| `user.suspended`      | Set `users.suspended_at = NOW()` and increment `session_version` |
| `user.unsuspended`    | Set `users.suspended_at = NULL` |
| `user.deleted`        | Delete `users` row (FKs on `links`, `tags`, etc. cascade) |
| `entitlement.revoked` | Increment `session_version` (next request forces re-auth, which will fail at authorize) |
| `entitlement.granted` | No-op (next login will JIT-provision successfully) |

All handlers must be idempotent on `jti` (use a `moka` cache with a 5-minute TTL, same as
dmarc-reporter). Always return HTTP 200 regardless of whether the user was found.

---

## 10. Server functions

Rusty Links uses Dioxus server functions (`src/server_functions/`) in addition to the Axum REST
API. Server functions run on the server and are called from WASM via a special Dioxus endpoint.

Server functions that access the current user (e.g., `auth.rs`) today call into the standalone
JWT path. Under the `saas` feature these need to read from the session store instead.

The recommended approach:
- Add a `get_current_user_from_session(session_id: &str, pool: &PgPool) -> Result<User, AppError>`
  helper in `src/auth/oidc_rp/mod.rs`.
- In server functions under `#[cfg(feature = "saas")]`, extract the `rl_session` cookie from the
  Axum request (available via `extract::ServerFunctionRequest`) and call this helper.

This keeps server functions working without changes to the Dioxus layer.

---

## 11. Dependencies to add

Add to `Cargo.toml` under the `server` feature:

```toml
# OIDC RS / BFF (saas mode only; gated in code with #[cfg(feature = "saas")])
sha2    = { version = "0.10", optional = true }   # already present via webhook sig
moka    = { version = "0.12", optional = true }   # JWKS + JTI cache
```

`jsonwebtoken = "9"` is already in the `server` feature. Ensure EdDSA support is enabled — the
`rust-crypto` feature must be active (it is by default in `jsonwebtoken 9`).

`reqwest` is already a non-WASM dependency used for scraping. The BFF token exchange POSTs to the
SaaS token endpoint using the existing `reqwest` client.

No new async runtime dependencies — `tokio` is already in the `server` feature.

---

## 12. Key rotation

Rusty Links is a consumer of the SaaS JWKS, not a signer. Key rotation on the SaaS side is
transparent to Rusty Links as long as:

1. The old `kid` is kept in the JWKS for `max(access_token_ttl, id_token_ttl) + 1h` (SaaS
   handles this per `saas/docs/SSO Implementation.md §10.3`).
2. The JWKS cache TTL (`OIDC_JWKS_CACHE_TTL`, default 300 s) means Rusty Links will pick up the
   new key within 5 minutes.

No key material is stored in Rusty Links. The JWKS disk cache path is optional (`/var/lib/rusty-links/jwks.json`) for cold-start resilience.

---

## 13. Docker / compose changes

### `compose.yml` (production)

```yaml
services:
  rusty-links:
    environment:
      OIDC_ISSUER: https://api.a8n.tools
      OIDC_AUDIENCE: https://links.a8n.run/api
      OIDC_CLIENT_ID: a8000000-0000-0000-0000-000000000005
      OIDC_REDIRECT_URI: https://links.a8n.run/oauth2/callback
      OIDC_POST_LOGOUT_REDIRECT_URI: https://links.a8n.run/
    secrets:
      - oidc_client_secret
    # Mount the JWKS disk cache volume for cold-start resilience.
    volumes:
      - rusty_links_data:/var/lib/rusty-links

secrets:
  oidc_client_secret:
    file: ./secrets/rustylinks_oidc_secret.txt
```

The `OIDC_CLIENT_SECRET` env var is read from the Docker secret file at startup:

```rust
// In Config::from_env(), saas feature:
let oidc_client_secret = std::env::var("OIDC_CLIENT_SECRET")
    .or_else(|_| std::fs::read_to_string("/run/secrets/oidc_client_secret")
        .map(|s| s.trim().to_string()))
    .unwrap_or_default();
```

### `compose.dev.yml`

Add `OIDC_ISSUER`, `OIDC_CLIENT_ID`, `OIDC_REDIRECT_URI`, `OIDC_POST_LOGOUT_REDIRECT_URI` pointing
to `http://localhost:18080` (the SaaS dev API port) and `http://localhost:4002` respectively.
`OIDC_CLIENT_SECRET` can be set as a plain env var in dev.

---

## 14. Rollout plan

1. **Create the three SQLx migrations** (§5.1, §5.2, §5.3) and verify they apply cleanly to a
   fresh DB and to an existing DB with users. Check that FKs cascade correctly on `users` delete.

2. **Add `rustylinks-web-bff` client to the SaaS seed migration** (§4) on the `saas/feature-sso`
   branch. Set the client secret via admin CLI against staging.

3. **Implement `OidcConfig` and config changes** (§7.1). Update `.env.saas`. Ensure the binary
   refuses to start when `OIDC_ISSUER` is set but `OIDC_CLIENT_ID` / `OIDC_CLIENT_SECRET` are
   empty.

4. **Implement `src/auth/oidc_rs.rs`** — the RS access-token verifier with JWKS cache. Unit-test
   with a local Ed25519 test keypair: valid token passes; HS256, `alg: none`, wrong audience,
   wrong issuer all fail.

5. **Implement `src/auth/oidc_rp/jit.rs`** — load-or-provision. Test: new user creates row with
   sentinel password hash; existing user updates `email` and `is_admin`.

6. **Implement `src/auth/oidc_rp/mod.rs`** — BFF handlers (`/oauth2/login`, `/oauth2/callback`,
   `/oauth2/logout`, `/oauth2/backchannel-logout`, `/oauth2/lifecycle-event`), including the
   `user_sessions` insert on callback and the `rp_sessions` / `user_sessions` cleanup job wired
   into the `Scheduler`. Test the happy-path login flow end-to-end against a dev SaaS instance.

7. **Update `src/auth/middleware.rs`** — replace HS256 cookie extractor with `user_sessions` +
   bearer extractor (§7.5). Run the full test suite.

8. **Update `src/main.rs`** — swap page-level redirect from `SAAS_LOGIN_URL` to `/oauth2/login`;
   remove `/logout` proxy route and `/saas-refresh.js` (§7.6).

9. **Update server functions** (§10) to use the session store.

10. **Staging validation** against `staging.links.a8n.run`:
    - Full happy-path login via SaaS.
    - Back-channel logout from SaaS kills the Rusty Links session.
    - Lifecycle event `user.suspended` prevents next request.
    - Algorithm confusion (`alg: HS256`, `alg: none`) rejected.
    - ID token used as Bearer rejected by RS.
    - `redirect_uri` mismatch rejected.

11. **Production flip**: deploy, point DNS, announce.

12. **Cleanup**: remove `saas_auth.rs`, old `saas_*` config fields, `saas-refresh.js`, and the
    `assets/saas-refresh.js` file after a one-sprint stabilization window.

Rollback at any step: set `OIDC_ISSUER=` (empty) to fall back to the legacy cookie path, which is
kept alive until step 12.

---

## 15. Security checklist (Rusty-Links-specific)

These items are in addition to the shared security model in `saas/docs/SSO Implementation.md §9`.

- [ ] `SAAS_JWT_SECRET` removed from all configs and secrets stores after the cutover.
- [ ] Session cookie scoped to `links.a8n.run` (not `.a8n.tools`).
- [ ] Maintenance mode middleware does not read HS256 cookie after cutover.
- [ ] Server functions do not accept user-supplied `user_id` directly — always resolve from
      session store.
- [ ] `password_hash` for OIDC-provisioned users is the sentinel `"!sso:no-password"` — the
      standalone login endpoint must reject this sentinel and return a generic error.
- [ ] JTI idempotency cache for lifecycle events uses an in-process `moka` cache (not DB) to
      avoid a round-trip per event; TTL matches `OIDC_LIFECYCLE_JTI_CACHE_TTL` (default 300 s).
- [ ] `user_sessions.session_token_hash` stores SHA-256 of the raw cookie — raw token never
      persisted in the database.
- [ ] `rl_session` cookie `Max-Age` matches `OIDC_SESSION_TTL_SECONDS`; `user_sessions.expires_at`
      set to `now() + session_ttl` at login.
- [ ] The `OidcVerifier` JWKS fetch uses `rustls` (via `reqwest`'s default TLS) — no plain-HTTP
      fallback. The JWKS URL must start with `https://` at startup validation.
- [ ] `/api/auth/setup`, `/api/auth/register` routes are unreachable in the saas binary (gated
      with `#[cfg(feature = "standalone")]` or return 404).
- [ ] Back-channel logout handler returns HTTP 200 even when no matching user is found (idempotent,
      per OIDC Back-Channel Logout 1.0 §2.5).
- [ ] Lifecycle event `user.deleted` cascades correctly — FK `ON DELETE CASCADE` on
      `user_sessions(user_id)` and `rp_sessions` has no user FK (no cascade needed).

---

## 16. Open questions

- **Standalone + OIDC dual-mode**: Should the `saas` and `standalone` features remain mutually
  exclusive, or should the binary support both at runtime (e.g., self-hosters who also want SSO)?
  Current plan: keep them mutually exclusive (compile-time). Revisit if a customer requests hybrid.

- **`rustylinks-desktop` client**: If a native Dioxus desktop version ships, it needs a separate
  public (PKCE-only) client registration. Defer until desktop exists.

- **Token storage in server functions**: Dioxus server functions currently share a global
  `DB_POOL`. We may need to plumb the session ID through the server function context to look up
  the access token for any server-function-initiated RS calls against other a8n services. No such
  calls exist today; design when needed.
