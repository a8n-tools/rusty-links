# Database Documentation

Complete database schema reference for Rusty Links PostgreSQL database.

## Table of Contents

- [Overview](#overview)
- [Connection Details](#connection-details)
- [Schema Summary](#schema-summary)
- [Entity Relationship Diagram](#entity-relationship-diagram)
- [Tables Reference](#tables-reference)
- [Test Database](#test-database)
- [Migrations](#migrations)
- [Indexes](#indexes)
- [Backup and Restore](#backup-and-restore)
- [Performance Tuning](#performance-tuning)
- [Maintenance](#maintenance)

---

## Overview

Rusty Links uses PostgreSQL 17+ as its database backend. The schema is designed with:

- **UUID primary keys** for all entities
- **Cascade deletes** to maintain referential integrity
- **Timestamps** for audit trails
- **Indexes** for query performance
- **Unique constraints** to prevent duplicates
- **Check constraints** for data validation
- **Case-insensitive uniqueness** on names (using `lower()`)

### Database Features Used

- **pgcrypto extension** - UUID generation with `gen_random_uuid()`
- **TIMESTAMP WITH TIME ZONE** - Timezone-aware timestamps
- **Foreign key constraints** - Referential integrity
- **Partial indexes** - Optimized queries on filtered data
- **Self-referencing foreign keys** - Category hierarchy

---

## Connection Details

### Environment Variables

```bash
DATABASE_URL=postgresql://rustylinks:password@localhost:5432/rustylinks
```

### Connection Pool Settings

Default configuration (from SQLx):
- **Max connections**: 5
- **Connection timeout**: 30 seconds
- **Idle timeout**: 10 minutes

### Docker Compose

```yaml
postgres:
  image: postgres:16-alpine
  environment:
    POSTGRES_USER: rustylinks
    POSTGRES_PASSWORD: changeme
    POSTGRES_DB: rustylinks
```

---

## Schema Summary

### Core Tables

| Table        | Purpose                 | Rows (typical)  |
|--------------|-------------------------|-----------------|
| `users`      | User accounts           | 1-10            |
| `pending_login_approvals` | Sign-ins held by the LINKS-35 / LINKS-45 approval gate | 0 (empty unless the gate is on) |
| `known_devices` | Devices an account has completed a sign-in from (LINKS-45) | 1-5 per user |
| `user_sessions` | BFF sessions, keyed by the hashed `rl_session` cookie value | 1-5 per user |
| `refresh_tokens` | Refresh tokens issued alongside the access JWT | 1-5 per user |
| `login_attempts` | Sign-in attempts feeding the account lockout | 0-100 (swept at 30 days) |
| `rp_sessions` | PKCE state for one in-flight OIDC login | 0 (seconds each) |
| `links`      | Bookmarked links        | 100-10,000+     |
| `categories` | Link categorization     | 10-100          |
| `tags`       | Link tags               | 20-200          |
| `languages`  | Programming languages   | 20-50           |
| `licenses`   | Software licenses       | 20-40           |

### Junction Tables

| Table             | Purpose                                     |
|-------------------|---------------------------------------------|
| `link_categories` | Links ↔ Categories (many-to-many)           |
| `link_tags`       | Links ↔ Tags (many-to-many with order)      |
| `link_languages`  | Links ↔ Languages (many-to-many with order) |
| `link_licenses`   | Links ↔ Licenses (many-to-many with order)  |

---

## Entity Relationship Diagram

```
users
  │
  ├──< user_sessions (user_id)
  │     └─ session_token_hash (SHA-256 of the cookie value)
  │
  ├──< refresh_tokens (user_id)
  ├──< pending_login_approvals (user_id)
  ├──< known_devices (user_id)
  │
  ├──< links (user_id)
  │     │
  │     ├──< link_categories >── categories
  │     ├──< link_languages >── languages
  │     ├──< link_licenses >── licenses
  │     └──< link_tags >── tags
  │
  ├──< categories (user_id)
  │     └─── categories (parent_id, self-reference)
  │
  ├──< languages (user_id, nullable for global)
  ├──< licenses (user_id, nullable for global)
  └──< tags (user_id)

rp_sessions     (no user_id: PKCE state for a login that has resolved no account yet)
login_attempts  (no user_id: keyed by the email typed at the sign-in form)

Legend:
  ├──<  One-to-many relationship
  >──   Many-to-one relationship
  └───  Self-referencing relationship
```

### Relationship Details

- **User → Links**: One user can have many links (CASCADE DELETE)
- **User → Categories**: One user can have many categories (CASCADE DELETE)
- **User → Tags**: One user can have many tags (CASCADE DELETE)
- **User → Sessions**: One user can have many `user_sessions` and `refresh_tokens` (CASCADE DELETE)
- **User → Holds and devices**: One user can have many `pending_login_approvals` and `known_devices` (CASCADE DELETE)
- **rp_sessions / login_attempts**: No foreign key. `rp_sessions` exists before a login resolves to an account, and `login_attempts` is keyed by the email typed at the form, which need not match one
- **Link → Categories**: Many-to-many via `link_categories`
- **Link → Tags**: Many-to-many via `link_tags` (with ordering)
- **Link → Languages**: Many-to-many via `link_languages` (with ordering)
- **Link → Licenses**: Many-to-many via `link_licenses` (with ordering)
- **Category → Category**: Self-referencing for hierarchy (CASCADE DELETE)
- **Languages/Licenses**: Can be global (user_id = NULL) or user-specific

---

## Tables Reference

Every table and column below is compared against a freshly migrated database by `database_doc_matches_the_migrated_schema` in `tests/db_schema.rs`, in both directions, so a section describing a table that no longer exists (as the `sessions` section did until LINKS-53) or a column added without a section fails the `cargo test --features server --test db_schema` leg. The comparison needs an applied schema rather than the migration text, because `20250101000007_fix_links_schema.sql` drops and re-adds `links.logo` from inside a PL/pgSQL block, which is why this guard is a test and the file-level [Migration History](#migration-history) guard is `scripts/check-migration-docs.nu`.

### users

User accounts table (single-user application, but supports multiple users).

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    name TEXT NOT NULL DEFAULT '',
    is_admin BOOLEAN NOT NULL DEFAULT FALSE,
    saas_user_id UUID,
    suspended_at TIMESTAMP WITH TIME ZONE,
    session_version INT NOT NULL DEFAULT 0,
    last_login_country VARCHAR(2),
    notify_new_location BOOLEAN NOT NULL DEFAULT TRUE
);
```

**Columns:**

| Column          | Type        | Constraints             | Description                |
|-----------------|-------------|-------------------------|----------------------------|
| `id`            | UUID        | PRIMARY KEY             | User identifier            |
| `email`         | TEXT        | NOT NULL, UNIQUE        | User email address         |
| `password_hash` | TEXT        | NOT NULL                | Argon2id password hash       |
| `created_at`    | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | Account creation timestamp |
| `name`          | TEXT        | NOT NULL, DEFAULT `''`  | Display name, empty until the user sets one through `PATCH /api/auth/me` |
| `is_admin`      | BOOLEAN     | NOT NULL, DEFAULT FALSE | Admin role; `20250101000008_jwt_auth.sql` promoted the oldest existing account when it added the column |
| `saas_user_id`  | UUID        | NULL, UNIQUE where NOT NULL | a8n Tools account this local account is linked to in hosted mode; NULL in standalone mode |
| `suspended_at`  | TIMESTAMPTZ | NULL                    | When the hosted-mode membership check suspended the account; NULL while active |
| `session_version` | INT       | NOT NULL, DEFAULT 0     | Bumped to invalidate every live `user_sessions` row for the account in one write |
| `last_login_country` | VARCHAR(2) | NULL | ISO-3166-1 alpha-2 country of the last login (LINKS-27) |
| `notify_new_location` | BOOLEAN | NOT NULL, DEFAULT TRUE | Per-user opt-out for new-location alerts (LINKS-27), set by the user through `PATCH /api/auth/me` (LINKS-33) |

**Indexes:**
- `idx_users_email` - Fast email lookups for authentication

**Notes:**
- Passwords are hashed using Argon2id
- Email must be unique (case-sensitive)
- Deleting a user cascades to all their data, including `user_sessions`, `refresh_tokens`, `pending_login_approvals` and `known_devices`
- `last_login_country` is written when a sign-in completes (or when a held sign-in is approved, LINKS-35) and the edge resolved a country, and is what the next sign-in is compared against. A sign-in held for approval and never approved writes nothing, so it cannot make its country look familiar next time

---

### pending_login_approvals

A sign-in held by the LINKS-35 / LINKS-45 approval gate, waiting for the account owner to approve it from a single-use emailed link.

```sql
CREATE TABLE pending_login_approvals (
    id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id        UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash     BYTEA       NOT NULL UNIQUE,
    country        VARCHAR(2),
    ip             TEXT        NOT NULL,
    device         TEXT,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at     TIMESTAMPTZ NOT NULL,
    consumed_at    TIMESTAMPTZ,
    reason         TEXT        NOT NULL DEFAULT 'new_country',
    device_id_hash BYTEA
);
```

**Columns:**

| Column        | Type        | Constraints                | Description                                             |
|---------------|-------------|----------------------------|---------------------------------------------------------|
| `id`          | UUID        | PRIMARY KEY                | Row identifier                                          |
| `user_id`     | UUID        | NOT NULL, FK -> users(id)  | Account whose sign-in is held                           |
| `token_hash`  | BYTEA       | NOT NULL, UNIQUE           | SHA-256 of the emailed token; the token is never stored |
| `country`     | VARCHAR(2)  | NULL                       | ISO-3166-1 alpha-2 country the sign-in came from; NULL on a device-only hold where nothing resolved one |
| `ip`          | TEXT        | NOT NULL                   | Client IP shown on the approval page                    |
| `device`      | TEXT        | NULL                       | User-Agent shown on the approval page; display only, never the device identity |
| `created_at`  | TIMESTAMPTZ | NOT NULL, DEFAULT NOW()    | When the sign-in was held                               |
| `expires_at`  | TIMESTAMPTZ | NOT NULL                   | 15 minutes after `created_at`                           |
| `consumed_at` | TIMESTAMPTZ | NULL                       | When the link was claimed; NULL means still claimable   |
| `reason`      | TEXT        | NOT NULL, DEFAULT `'new_country'` | Which trigger held the sign-in: `new_country`, `new_device`, or `new_country_and_device` |
| `device_id_hash` | BYTEA    | NULL                       | SHA-256 of the device id the held sign-in submitted, promoted into `known_devices` when the link is claimed; NULL when the client sent none |

**Indexes:**
- `pending_login_approvals_user_id` - rows for one account
- `pending_login_approvals_expires` - expiry sweep

**Notes:**
- Only the SHA-256 of the token is stored, so a database dump can neither mint nor replay an approval link, and there is no way to approve a held sign-in from the database
- Single use is enforced by a conditional `UPDATE ... SET consumed_at = NOW() WHERE token_hash = $1 AND consumed_at IS NULL AND expires_at > NOW()`; the affected-row count decides a race between two concurrent clicks
- Expired rows are swept opportunistically when the next sign-in is held; a leftover row is inert because every read is guarded on `expires_at` anyway
- Rows exist only where `LOGIN_APPROVAL_ENABLED=true`; with the gate off the table stays empty
- `country` became nullable in `20260824000015_create_known_devices.sql`: a device-only hold in the default deployment (no geoblock edge, no `TRUSTED_PROXY_CIDRS`) resolves no country, and a NOT NULL column would fail that insert and turn the hold into a 500
- At most three live links per account at a time. The dedup collapses a retried sign-in from the same country on the same device into the live link it already has; the cap bounds a client that varies its device id on every attempt. Reaching the cap still holds the sign-in, it only stops another mail

---

### known_devices

A device an account has completed a sign-in from (LINKS-45), which is what the approval gate's second trigger recognises.

```sql
CREATE TABLE known_devices (
    id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id        UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id_hash BYTEA       NOT NULL,
    first_seen_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, device_id_hash)
);
```

**Columns:**

| Column           | Type        | Constraints                | Description                                             |
|------------------|-------------|----------------------------|---------------------------------------------------------|
| `id`             | UUID        | PRIMARY KEY                | Row identifier                                          |
| `user_id`        | UUID        | NOT NULL, FK -> users(id)  | Account the device belongs to                           |
| `device_id_hash` | BYTEA       | NOT NULL, UNIQUE with `user_id` | SHA-256 of the browser's device id; the id is never stored |
| `first_seen_at`  | TIMESTAMPTZ | NOT NULL, DEFAULT NOW()    | When the account first completed a sign-in from it       |
| `last_seen_at`   | TIMESTAMPTZ | NOT NULL, DEFAULT NOW()    | Touched by every later sign-in from it                   |

**Indexes:**
- `known_devices_user_id` - rows for one account
- The `UNIQUE (user_id, device_id_hash)` constraint's implicit index serves the recognition lookup

**Notes:**
- **No backfill, deliberately.** The table is created empty, so every account that exists on the deploy that adds it has zero known devices. Zero devices is read as the account's baseline and is never treated as "new device", exactly as a NULL `users.last_login_country` is, so nobody is held on deploy day. See [SECURITY.md](SECURITY.md)
- The device id is minted by the browser and kept in `localStorage`, then sent with the sign-in request as `device_id`. Only its SHA-256 is stored, so a database dump yields nothing that can be replayed as a known device
- A row is written only when a sign-in COMPLETES or an approval is claimed, never from a held-and-unapproved attempt, so an attempt nobody approves cannot make its device look familiar
- Recognition is scoped to `user_id`, so a shared browser one account has made known does not make a second account's first sign-in from it look familiar
- The write is `INSERT ... ON CONFLICT (user_id, device_id_hash) DO UPDATE SET last_seen_at = NOW()`, so repeat sign-ins touch one row rather than growing the table
- Rows accumulate for the life of the account and are removed with it by `ON DELETE CASCADE`. Letting a user list and revoke their own devices is tracked in LINKS-55

---

### user_sessions

Long-lived BFF sessions, keyed by the SHA-256 of the `rl_session` cookie value.

```sql
CREATE TABLE user_sessions (
    id                 UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    session_token_hash BYTEA       NOT NULL UNIQUE,
    user_id            UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_version    INT         NOT NULL,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at         TIMESTAMPTZ NOT NULL,
    auth_via_oidc      BOOLEAN     NOT NULL DEFAULT FALSE
);
```

**Columns:**

| Column                | Type        | Constraints               | Description                                             |
|-----------------------|-------------|---------------------------|---------------------------------------------------------|
| `id`                  | UUID        | PRIMARY KEY               | Row identifier                                          |
| `session_token_hash`  | BYTEA       | NOT NULL, UNIQUE          | SHA-256 of the `rl_session` cookie value; the value itself is never stored |
| `user_id`             | UUID        | NOT NULL, FK → users(id)  | Account the session belongs to                          |
| `session_version`     | INT         | NOT NULL                  | Snapshot of `users.session_version` when the session was minted |
| `created_at`          | TIMESTAMPTZ | NOT NULL, DEFAULT NOW()   | When the session was minted                             |
| `expires_at`          | TIMESTAMPTZ | NOT NULL                  | When the session stops being accepted                   |
| `auth_via_oidc`       | BOOLEAN     | NOT NULL, DEFAULT FALSE   | Whether an OIDC login minted it, so a sign-out can end the provider session too (`20260427000012`) |

**Indexes:**
- `user_sessions_user_id` - sessions for one account
- `user_sessions_expires` - expiry sweep

**Notes:**
- Only the hash of the cookie value is stored, so a database dump yields nothing that can be replayed as a session cookie
- `session_version` is compared against `users.session_version` every time the session is looked up, so incrementing the user's value invalidates every live session for that account in one write
- Expired rows are swept by the scheduler (`DELETE FROM user_sessions WHERE expires_at < NOW()`), and a sign-out deletes its own row by hash

---

### refresh_tokens

Refresh tokens issued alongside the access JWT, added by `20250101000008_jwt_auth.sql` in place of the `sessions` table this reference used to document.

```sql
CREATE TABLE refresh_tokens (
    id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token      TEXT        NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Columns:**

| Column       | Type        | Constraints               | Description                             |
|--------------|-------------|---------------------------|-----------------------------------------|
| `id`         | UUID        | PRIMARY KEY               | Row identifier                          |
| `user_id`    | UUID        | NOT NULL, FK → users(id)  | Account the token belongs to            |
| `token`      | TEXT        | NOT NULL, UNIQUE          | The refresh token as issued             |
| `expires_at` | TIMESTAMPTZ | NOT NULL                  | When the token stops being accepted     |
| `created_at` | TIMESTAMPTZ | NOT NULL, DEFAULT NOW()   | When the token was issued               |

**Indexes:**
- `idx_refresh_tokens_user_id` - tokens for one account
- `idx_refresh_tokens_token` - the lookup a refresh performs

**Notes:**
- A refresh rotates: the presented row is deleted and a new one inserted, so a token cannot be replayed
- Signing out deletes every row for the account
- Expired rows are swept by the scheduler through `security::cleanup_expired_refresh_tokens`
- `token` is stored as issued, unlike `user_sessions.session_token_hash` and `pending_login_approvals.token_hash`, which store only a SHA-256. Hashing it the same way is tracked in LINKS-59

---

### login_attempts

Sign-in attempts, which is what the account lockout in `src/security.rs` counts.

```sql
CREATE TABLE login_attempts (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    email        TEXT        NOT NULL,
    attempted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    success      BOOLEAN     NOT NULL DEFAULT FALSE
);
```

**Columns:**

| Column         | Type        | Constraints              | Description                                    |
|----------------|-------------|--------------------------|------------------------------------------------|
| `id`           | UUID        | PRIMARY KEY              | Row identifier                                 |
| `email`        | TEXT        | NOT NULL                 | Email typed at the sign-in form                |
| `attempted_at` | TIMESTAMPTZ | NOT NULL, DEFAULT NOW()  | When the attempt was made                      |
| `success`      | BOOLEAN     | NOT NULL, DEFAULT FALSE  | Whether the attempt authenticated              |

**Indexes:**
- `idx_login_attempts_email` - the lockout's count of recent failures for one email

**Notes:**
- No foreign key to `users`. The row is keyed by the email typed at the form, so an attempt against an address with no account is recorded the same way, and the lockout cannot be used to probe which addresses exist
- `security::cleanup_old_login_attempts` drops rows older than 30 days on the scheduler

---

### rp_sessions

PKCE state for one in-flight OIDC authorization code flow.

```sql
CREATE TABLE rp_sessions (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    state         TEXT        NOT NULL UNIQUE,
    nonce         TEXT        NOT NULL,
    code_verifier TEXT        NOT NULL,
    return_to     TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at    TIMESTAMPTZ NOT NULL
);
```

**Columns:**

| Column          | Type        | Constraints              | Description                                             |
|-----------------|-------------|--------------------------|---------------------------------------------------------|
| `id`            | UUID        | PRIMARY KEY              | Row identifier                                          |
| `state`         | TEXT        | NOT NULL, UNIQUE         | OAuth `state`, matched on callback to bind the response to this request |
| `nonce`         | TEXT        | NOT NULL                 | OIDC `nonce`, matched against the ID token claim        |
| `code_verifier` | TEXT        | NOT NULL                 | PKCE verifier whose challenge went out with the authorization request |
| `return_to`     | TEXT        | NULL                     | Where to send the browser after the callback; NULL means the default landing page |
| `created_at`    | TIMESTAMPTZ | NOT NULL, DEFAULT NOW()  | When the flow started                                   |
| `expires_at`    | TIMESTAMPTZ | NOT NULL                 | When the in-flight flow stops being claimable           |

**Indexes:**
- `rp_sessions_expires` - expiry sweep

**Notes:**
- No `user_id`: the row is written before the login has resolved to an account, which is why it is not owned by one
- A successful callback deletes its own row, so a `state` cannot be replayed; abandoned rows are swept by the scheduler on `expires_at`
- Rows live for seconds in normal use, so the table is effectively always empty

---

### links

Bookmarked links with metadata.

```sql
CREATE TABLE links (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    domain TEXT NOT NULL,
    path TEXT,
    title TEXT,
    description TEXT,
    logo TEXT,
    source_code_url TEXT,
    documentation_url TEXT,
    notes TEXT,
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'archived', 'inaccessible', 'repo_unavailable')),
    github_stars INTEGER,
    github_archived BOOLEAN,
    github_last_commit DATE,
    is_github_repo BOOLEAN NOT NULL DEFAULT FALSE,
    consecutive_failures INTEGER NOT NULL DEFAULT 0,
    last_checked TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    refreshed_at TIMESTAMP WITH TIME ZONE,
    CONSTRAINT uq_links_user_domain_path UNIQUE (user_id, domain, path)
);
```

**Columns:**

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | UUID | PRIMARY KEY | Link identifier |
| `user_id` | UUID | NOT NULL, FK → users(id) | Owner of link |
| `url` | TEXT | NOT NULL | Full URL |
| `domain` | TEXT | NOT NULL | Extracted domain (e.g., "github.com") |
| `path` | TEXT | NULL | URL path; nullable since `20250101000007_fix_links_schema.sql` |
| `title` | TEXT | NULL | Page title (auto-extracted) |
| `description` | TEXT | NULL | Page description (auto-extracted) |
| `logo` | TEXT | NULL | Site logo/favicon as a URL or base64 data; widened from BYTEA by `20250101000007_fix_links_schema.sql` |
| `source_code_url` | TEXT | NULL | Link to source code |
| `documentation_url` | TEXT | NULL | Link to documentation |
| `notes` | TEXT | NULL | User notes |
| `status` | TEXT | NOT NULL, DEFAULT 'active' | Link status (see values below) |
| `github_stars` | INTEGER | NULL | GitHub stars count |
| `github_archived` | BOOLEAN | NULL | GitHub archived status |
| `github_last_commit` | DATE | NULL | Last GitHub commit date |
| `is_github_repo` | BOOLEAN | NOT NULL, DEFAULT FALSE | Whether the URL is a GitHub repository, which is what decides if the GitHub fields are refreshed |
| `consecutive_failures` | INTEGER | NOT NULL, DEFAULT 0 | Health check failure count |
| `last_checked` | TIMESTAMPTZ | NULL | Last health check time |
| `created_at` | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | Link creation time |
| `updated_at` | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | Last update time |
| `refreshed_at` | TIMESTAMPTZ | NULL | Last metadata refresh |

**Status Values:**
- `active` - Link is active and accessible
- `archived` - User archived the link
- `inaccessible` - Link is not accessible (404, etc.)
- `repo_unavailable` - GitHub repository is unavailable

**Indexes:**
- `idx_links_user_id` - Find links by user
- `idx_links_domain` - Filter by domain
- `idx_links_status` - Filter by status
- `idx_links_created_at` - Sort by creation date
- `idx_links_last_checked` - Find links needing health checks (partial, WHERE NOT NULL)
- `idx_links_unchecked` - Find never-checked links (partial, WHERE NULL)
- `idx_links_is_github` - Find GitHub repositories (partial, WHERE `is_github_repo = true`)

**Unique Constraints:**
- `uq_links_user_domain_path` - Prevent duplicate URLs per user

**Notes:**
- Logo is stored as TEXT, either a URL or base64 data
- Domain and path are extracted for deduplication. `path` is nullable, and Postgres treats NULLs as distinct in a UNIQUE constraint, so two root-URL links on one domain do not collide
- GitHub fields are populated for GitHub repository URLs
- Health check fields track link availability

---

### categories

Hierarchical categories (3 levels maximum).

```sql
CREATE TABLE categories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    parent_id UUID REFERENCES categories(id) ON DELETE CASCADE,
    depth INTEGER NOT NULL CHECK (depth >= 0 AND depth <= 2),
    sort_order INTEGER,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_categories_user_name UNIQUE (user_id, lower(name))
);
```

**Columns:**

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | UUID | PRIMARY KEY | Category identifier |
| `user_id` | UUID | NOT NULL, FK → users(id) | Owner of category |
| `name` | TEXT | NOT NULL | Category name |
| `parent_id` | UUID | NULL, FK → categories(id) | Parent category (NULL for root) |
| `depth` | INTEGER | NOT NULL, CHECK (0-2) | Hierarchy depth (0=root, 1=child, 2=grandchild) |
| `sort_order` | INTEGER | NULL | Manual sort order |
| `created_at` | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | Creation time |

**Hierarchy Levels:**
- **Depth 0** (Root): Top-level categories (e.g., "Development")
- **Depth 1** (Child): Second-level categories (e.g., "Rust")
- **Depth 2** (Grandchild): Third-level categories (e.g., "Web Frameworks")

**Indexes:**
- `idx_categories_user_id` - Find categories by user
- `idx_categories_parent_id` - Find children of a category

**Unique Constraints:**
- `uq_categories_user_name` - Category names are unique per user (case-insensitive)

**Notes:**
- Self-referencing via `parent_id`
- Deleting a parent cascades to children
- Maximum 3 levels enforced by CHECK constraint

---

### languages

Programming languages (global + user-specific).

```sql
CREATE TABLE languages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_languages_user_name UNIQUE (user_id, lower(name))
);
```

**Columns:**

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | UUID | PRIMARY KEY | Language identifier |
| `user_id` | UUID | NULL, FK → users(id) | Owner (NULL for global) |
| `name` | TEXT | NOT NULL | Language name |
| `created_at` | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | Creation time |

**Indexes:**
- `idx_languages_user_id` - Find languages by user

**Unique Constraints:**
- `uq_languages_user_name` - Unique per user (case-insensitive)

**Global Languages** (user_id = NULL):
JavaScript, Python, Java, C#, C++, TypeScript, PHP, C, Ruby, Go, Rust, Swift, Kotlin, R, Dart, Scala, Perl, Lua, Haskell, Elixir

**Notes:**
- Global languages (user_id = NULL) are seeded on migration
- Users can add custom languages
- Cannot delete global languages

---

### licenses

Software licenses (global + user-specific).

```sql
CREATE TABLE licenses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    full_name TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_licenses_user_name UNIQUE (user_id, lower(name))
);
```

**Columns:**

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | UUID | PRIMARY KEY | License identifier |
| `user_id` | UUID | NULL, FK → users(id) | Owner (NULL for global) |
| `name` | TEXT | NOT NULL | Short name/acronym (e.g., "MIT") |
| `full_name` | TEXT | NOT NULL | Full license name |
| `created_at` | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | Creation time |

**Indexes:**
- `idx_licenses_user_id` - Find licenses by user

**Unique Constraints:**
- `uq_licenses_user_name` - Unique per user (case-insensitive)

**Global Licenses** (user_id = NULL):
MIT, Apache-2.0, GPL-3.0, GPL-2.0, BSD-3-Clause, BSD-2-Clause, LGPL-3.0, LGPL-2.1, MPL-2.0, AGPL-3.0, ISC, CDDL-1.0, EPL-2.0, EPL-1.0, CC0-1.0, CC-BY-4.0, CC-BY-SA-4.0, Unlicense, Zlib, Artistic-2.0

**Notes:**
- Global licenses seeded on migration
- Users can add custom licenses
- `name` is typically SPDX identifier

---

### tags

User-defined tags for links.

```sql
CREATE TABLE tags (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_tags_user_name UNIQUE (user_id, lower(name))
);
```

**Columns:**

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | UUID | PRIMARY KEY | Tag identifier |
| `user_id` | UUID | NULL, FK → users(id) | Owner of tag |
| `name` | TEXT | NOT NULL | Tag name |
| `created_at` | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | Creation time |

**Indexes:**
- `idx_tags_user_id` - Find tags by user

**Unique Constraints:**
- `uq_tags_user_name` - Tag names unique per user (case-insensitive)

**Notes:**
- Tags are user-specific (no global tags)
- Case-insensitive uniqueness prevents duplicates

---

### link_categories

Junction table linking links to categories (many-to-many).

```sql
CREATE TABLE link_categories (
    link_id UUID NOT NULL REFERENCES links(id) ON DELETE CASCADE,
    category_id UUID NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    PRIMARY KEY (link_id, category_id)
);
```

**Columns:**

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `link_id` | UUID | NOT NULL, FK → links(id) | Link reference |
| `category_id` | UUID | NOT NULL, FK → categories(id) | Category reference |

**Notes:**
- Composite primary key prevents duplicates
- Cascade deletes when link or category is deleted
- A link can have multiple categories

---

### link_languages

Junction table linking links to programming languages (many-to-many with ordering).

```sql
CREATE TABLE link_languages (
    link_id UUID NOT NULL REFERENCES links(id) ON DELETE CASCADE,
    language_id UUID NOT NULL REFERENCES languages(id) ON DELETE CASCADE,
    order_num INTEGER NOT NULL,
    PRIMARY KEY (link_id, language_id)
);
```

**Columns:**

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `link_id` | UUID | NOT NULL, FK → links(id) | Link reference |
| `language_id` | UUID | NOT NULL, FK → languages(id) | Language reference |
| `order_num` | INTEGER | NOT NULL | Display order (1, 2, 3...) |

**Notes:**
- `order_num` allows sorting languages by importance
- Primary language typically has `order_num = 1`

---

### link_licenses

Junction table linking links to licenses (many-to-many with ordering).

```sql
CREATE TABLE link_licenses (
    link_id UUID NOT NULL REFERENCES links(id) ON DELETE CASCADE,
    license_id UUID NOT NULL REFERENCES licenses(id) ON DELETE CASCADE,
    order_num INTEGER NOT NULL,
    PRIMARY KEY (link_id, license_id)
);
```

**Columns:**

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `link_id` | UUID | NOT NULL, FK → links(id) | Link reference |
| `license_id` | UUID | NOT NULL, FK → licenses(id) | License reference |
| `order_num` | INTEGER | NOT NULL | Display order |

**Notes:**
- Projects can have multiple licenses (dual-licensing)
- `order_num` specifies primary vs secondary licenses

---

### link_tags

Junction table linking links to tags (many-to-many with ordering).

```sql
CREATE TABLE link_tags (
    link_id UUID NOT NULL REFERENCES links(id) ON DELETE CASCADE,
    tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    order_num INTEGER NOT NULL,
    PRIMARY KEY (link_id, tag_id)
);
```

**Columns:**

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `link_id` | UUID | NOT NULL, FK → links(id) | Link reference |
| `tag_id` | UUID | NOT NULL, FK → tags(id) | Tag reference |
| `order_num` | INTEGER | NOT NULL | Display order |

**Notes:**
- `order_num` allows custom tag ordering per link

---

## Test Database

The integration suites never run against the database named in `DATABASE_URL`. `tests/common/mod.rs::test_pool` derives a sibling `<database>_test` from it, creates it if missing, and applies `migrations/` to it with `sqlx::migrate!`, so running them against a dev database does not touch dev data. Nothing to create by hand, and nothing skips: an unset or unreachable `DATABASE_URL` fails the suite rather than passing vacuously (LINKS-44). See [TESTING.md](TESTING.md).

---

## Migrations

### Migration System

Rusty Links uses SQLx migrations for schema management.

**Migration Files Location:** `migrations/`, one file per migration, named:

```
YYYYMMDDHHMMSS_description.sql
```

The files themselves are the list; [Migration History](#migration-history) below describes each one, and `scripts/check-migration-docs.nu` fails CI when the two disagree.

### Running Migrations

Migrations run **automatically** on application startup.

**Manual Migration Commands:**

```bash
# Install SQLx CLI
cargo install sqlx-cli --no-default-features --features postgres

# Run all pending migrations
sqlx migrate run

# Create new migration
sqlx migrate add <migration_name>

# Get migration status
sqlx migrate info
```

These migrations are simple (a single `.sql` per version), not reversible, so there is no `.down.sql` and `sqlx migrate revert` does not apply. Roll a change back with a new migration.

### Migration History

| Version | Description | Date |
|---------|-------------|------|
| 20250101000001 | Initial schema - users, links, categories, tags, languages, licenses | 2025-01-01 |
| 20250101000002 | Seed global languages and licenses | 2025-01-01 |
| 20250101000003 | Add sessions table for authentication | 2025-01-01 |
| 20250101000004 | Add consecutive_failures to links | 2025-01-01 |
| 20250101000005 | Add scheduler fields (last_checked) to links | 2025-01-01 |
| 20250101000006 | Add name to users | 2025-01-01 |
| 20250101000007 | Add is_github_repo to links, widen logo from BYTEA to TEXT, make path nullable | 2025-01-01 |
| 20250101000008 | Add is_admin to users, add refresh_tokens and login_attempts, drop sessions | 2025-01-01 |
| 20260417000009 | Add saas_user_id, suspended_at and session_version to users for SSO | 2026-04-17 |
| 20260417000010 | Add rp_sessions, the transient PKCE state for the BFF authorization code flow | 2026-04-17 |
| 20260417000011 | Add user_sessions, the long-lived BFF sessions keyed by a hashed cookie value | 2026-04-17 |
| 20260427000012 | Add auth_via_oidc to user_sessions | 2026-04-27 |
| 20260821000013 | Add last_login_country and notify_new_location to users for the new-location alert (LINKS-27) | 2026-08-21 |
| 20260821000014 | Add pending_login_approvals for the sign-in approval gate (LINKS-35) | 2026-08-21 |
| 20260824000015 | Add known_devices for the approval gate's new-device trigger (LINKS-45) | 2026-08-24 |

Every file in `migrations/` has a row here and every row names a file: `scripts/check-migration-docs.nu` compares the two in both directions and runs in `.forgejo/workflows/check.yml`. It fails a migration with no row, a row with no migration, a Date that contradicts the version prefix, and a table it can no longer parse, so a shape change is a red job rather than a silent pass. Before the guard existed the table listed six of the fourteen migrations, the first five plus the most recent, and read as if the schema had stopped changing in January 2025 (LINKS-46). To run it locally:

```bash
nu scripts/check-migration-docs.nu --self-test
nu scripts/check-migration-docs.nu
```

### Creating Custom Migrations

```bash
# Create migration
sqlx migrate add add_custom_field

# Edit migration file
# migrations/TIMESTAMP_add_custom_field.sql

# Run migration
sqlx migrate run
```

**Migration Best Practices:**
- Always test migrations on backup first
- Never modify existing migrations (create new ones)
- Keep migrations small and focused
- Add comments explaining complex changes; the file's leading comment is what the Migration History row above summarises

### Migrations Are Immutable Once Committed

A committed migration file is immutable: once it has merged to `main`, never modify, rename, or delete it. Fixes always go in a NEW migration.

SQLx records a SHA-384 checksum of each migration in the `_sqlx_migrations` table when it applies it, and re-verifies that checksum on every startup. If you edit a migration that any database has already applied, that database refuses to boot with `migration N was previously applied but has been modified`, and recovery requires reconciling the recorded checksum by hand against production. (This is exactly how the mokosh-server v0.4.0 deploy broke on nc-01 when an already-applied seed migration was edited during a cleanup.)

This rule is enforced mechanically in CI: `scripts/check-migration-immutability.nu` runs in `.forgejo/workflows/check.yml` and fails any PR (or push to `main`) that modifies, renames, or deletes a `migrations/*.sql` file already present on `main`. Adding a new migration file passes. To run the same check locally:

```bash
nu scripts/check-migration-immutability.nu
```

---

## Indexes

### Index Strategy

Indexes are created for:
- **Foreign keys** - Efficient JOIN operations
- **Search fields** - WHERE clause optimization
- **Sort fields** - ORDER BY optimization
- **Unique constraints** - Enforce uniqueness
- **Partial indexes** - Filtered queries

### Index List

| Table | Index Name | Columns | Type | Purpose |
|-------|------------|---------|------|---------|
| users | idx_users_email | email | B-tree | Fast email lookup for auth |
| users | users_saas_user_id_unique | saas_user_id | B-tree (unique, partial) | One local account per a8n Tools account (WHERE NOT NULL) |
| user_sessions | user_sessions_user_id | user_id | B-tree | Sessions for one account |
| user_sessions | user_sessions_expires | expires_at | B-tree | Expiry sweep |
| refresh_tokens | idx_refresh_tokens_user_id | user_id | B-tree | Tokens for one account |
| refresh_tokens | idx_refresh_tokens_token | token | B-tree | The lookup a refresh performs |
| login_attempts | idx_login_attempts_email | email | B-tree | Recent failures for one email |
| rp_sessions | rp_sessions_expires | expires_at | B-tree | Expiry sweep |
| pending_login_approvals | pending_login_approvals_user_id | user_id | B-tree | Holds for one account |
| pending_login_approvals | pending_login_approvals_expires | expires_at | B-tree | Expiry sweep |
| links | idx_links_user_id | user_id | B-tree | Find user's links |
| links | idx_links_domain | domain | B-tree | Filter by domain |
| links | idx_links_status | status | B-tree | Filter by status |
| links | idx_links_created_at | created_at | B-tree | Sort by date |
| links | idx_links_last_checked | last_checked | B-tree (partial) | Scheduler queries (WHERE NOT NULL) |
| links | idx_links_unchecked | last_checked | B-tree (partial) | Never-checked links (WHERE NULL) |
| links | idx_links_is_github | is_github_repo | B-tree (partial) | GitHub repositories (WHERE is_github_repo = true) |
| categories | idx_categories_user_id | user_id | B-tree | Find user's categories |
| categories | idx_categories_parent_id | parent_id | B-tree | Find child categories |
| categories | uq_categories_user_name | user_id, lower(name) | B-tree (unique) | Case-insensitive name uniqueness per user |
| languages | idx_languages_user_id | user_id | B-tree | Find user's languages |
| languages | uq_languages_user_name | user_id, lower(name) | B-tree (unique) | Case-insensitive name uniqueness per user |
| licenses | idx_licenses_user_id | user_id | B-tree | Find user's licenses |
| licenses | uq_licenses_user_name | user_id, lower(name) | B-tree (unique) | Case-insensitive name uniqueness per user |
| tags | idx_tags_user_id | user_id | B-tree | Find user's tags |
| tags | uq_tags_user_name | user_id, lower(name) | B-tree (unique) | Case-insensitive name uniqueness per user |
| known_devices | known_devices_user_id | user_id | B-tree | Devices for one account |

Primary keys and the indexes Postgres creates for a UNIQUE constraint are not listed here; each table's own section names its constraints.

### Partial Indexes

**Partial indexes** only index rows matching a condition, saving space and improving performance.

```sql
-- Only index links that have been checked (excludes NULLs)
CREATE INDEX idx_links_last_checked ON links(last_checked)
WHERE last_checked IS NOT NULL;

-- Only index links that have never been checked
CREATE INDEX idx_links_unchecked ON links(last_checked)
WHERE last_checked IS NULL;
```

### Index Maintenance

```sql
-- Rebuild indexes (rarely needed with PostgreSQL)
REINDEX TABLE links;

-- Analyze table for query planner
ANALYZE links;

-- Show index usage statistics
SELECT schemaname, tablename, indexname, idx_scan, idx_tup_read, idx_tup_fetch
FROM pg_stat_user_indexes
WHERE schemaname = 'public'
ORDER BY idx_scan DESC;
```

---

## Backup and Restore

### Full Database Backup

**Using Docker Compose:**

```bash
# SQL format (human-readable)
docker compose exec postgres pg_dump -U rustylinks rustylinks > backup.sql

# Custom format (compressed, faster restore)
docker compose exec postgres pg_dump -U rustylinks -Fc rustylinks > backup.dump

# With timestamp
docker compose exec postgres pg_dump -U rustylinks rustylinks > \
  backup_$(date +%Y%m%d_%H%M%S).sql
```

**Direct PostgreSQL:**

```bash
# Local PostgreSQL
pg_dump -U rustylinks rustylinks > backup.sql

# Remote PostgreSQL
pg_dump -h hostname -U rustylinks rustylinks > backup.sql
```

### Restore Database

**From SQL Backup:**

```bash
# Using Docker Compose
docker compose exec -T postgres psql -U rustylinks rustylinks < backup.sql

# Direct PostgreSQL
psql -U rustylinks rustylinks < backup.sql
```

**From Custom Format:**

```bash
# Using Docker Compose
docker compose exec -T postgres pg_restore -U rustylinks -d rustylinks backup.dump

# Direct PostgreSQL
pg_restore -U rustylinks -d rustylinks backup.dump
```

### Restore to New Database

```bash
# Create new database
docker compose exec postgres createdb -U rustylinks rustylinks_new

# Restore backup
docker compose exec -T postgres psql -U rustylinks rustylinks_new < backup.sql

# Switch databases (update .env)
DATABASE_URL=postgresql://rustylinks:password@localhost/rustylinks_new
```

### Automated Backups

**Backup Script:**

```bash
#!/bin/bash
# backup.sh

BACKUP_DIR="/backups"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="$BACKUP_DIR/rustylinks_$TIMESTAMP.dump"

# Create backup
docker compose exec -T postgres pg_dump -U rustylinks -Fc rustylinks > "$BACKUP_FILE"

# Keep only last 30 days
find $BACKUP_DIR -name "rustylinks_*.dump" -mtime +30 -delete

echo "Backup completed: $BACKUP_FILE"
```

**Cron Job (daily at 2 AM):**

```bash
0 2 * * * /path/to/backup.sh >> /var/log/rustylinks-backup.log 2>&1
```

### Selective Backup

**Backup Specific Tables:**

```bash
# Backup only links and categories
docker compose exec postgres pg_dump -U rustylinks -t links -t categories rustylinks > links_backup.sql
```

**Exclude Tables:**

```bash
# Backup everything except the transient auth tables, which are worthless in a restore
docker compose exec postgres pg_dump -U rustylinks -T rp_sessions -T login_attempts rustylinks > backup.sql
```

### Data-Only Backup

```bash
# Schema and data
docker compose exec postgres pg_dump -U rustylinks rustylinks > full_backup.sql

# Data only (no CREATE statements)
docker compose exec postgres pg_dump -U rustylinks --data-only rustylinks > data_only.sql

# Schema only (no INSERT statements)
docker compose exec postgres pg_dump -U rustylinks --schema-only rustylinks > schema_only.sql
```

---

## Performance Tuning

### PostgreSQL Configuration

**Recommended Settings** for typical deployment (adjust based on available RAM):

```sql
-- Memory settings (for 4GB RAM server)
shared_buffers = 1GB              -- 25% of RAM
effective_cache_size = 3GB        -- 75% of RAM
work_mem = 20MB                   -- Per-operation memory
maintenance_work_mem = 256MB      -- For VACUUM, CREATE INDEX

-- Connection settings
max_connections = 100             -- Adjust based on load

-- Query planner
random_page_cost = 1.1            -- SSD optimization (default: 4.0)
effective_io_concurrency = 200    -- SSD concurrent I/O

-- WAL settings
wal_buffers = 16MB
checkpoint_completion_target = 0.9
```

**Apply Settings:**

```bash
# Edit postgresql.conf
docker compose exec postgres vi /var/lib/postgresql/data/postgresql.conf

# Or mount custom config
# volumes:
#   - ./postgres.conf:/etc/postgresql/postgresql.conf
```

### Query Optimization

**Use EXPLAIN ANALYZE:**

```sql
-- Analyze query performance
EXPLAIN ANALYZE
SELECT l.*, c.name as category_name
FROM links l
LEFT JOIN link_categories lc ON l.id = lc.link_id
LEFT JOIN categories c ON lc.category_id = c.id
WHERE l.user_id = 'uuid'
ORDER BY l.created_at DESC
LIMIT 50;
```

**Common Optimizations:**

1. **Add indexes** for frequently filtered/sorted columns
2. **Use partial indexes** for queries with WHERE clauses
3. **Avoid SELECT *** - specify needed columns
4. **Use prepared statements** - SQLx does this automatically
5. **Batch inserts** - use transactions for multiple INSERTs

### Connection Pooling

SQLx automatically pools connections. Tune pool settings:

```rust
// In main.rs
let pool = PgPoolOptions::new()
    .max_connections(5)          // Increase for high concurrency
    .connect_timeout(Duration::from_secs(30))
    .idle_timeout(Duration::from_secs(600))
    .connect(&database_url)
    .await?;
```

### Database Statistics

```sql
-- Table sizes
SELECT
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;

-- Index usage
SELECT
    indexrelname as index_name,
    idx_scan as times_used,
    pg_size_pretty(pg_relation_size(indexrelid)) as index_size
FROM pg_stat_user_indexes
WHERE schemaname = 'public'
ORDER BY idx_scan DESC;

-- Most expensive queries (pg_stat_statements required)
SELECT
    query,
    calls,
    total_exec_time,
    mean_exec_time
FROM pg_stat_statements
ORDER BY mean_exec_time DESC
LIMIT 10;
```

---

## Maintenance

### Routine Maintenance Tasks

**VACUUM** - Reclaim space from dead tuples:

```sql
-- Vacuum all tables
VACUUM;

-- Vacuum specific table
VACUUM links;

-- Full vacuum (locks table, use sparingly)
VACUUM FULL links;

-- Vacuum with analyze
VACUUM ANALYZE;
```

**ANALYZE** - Update statistics for query planner:

```sql
-- Analyze all tables
ANALYZE;

-- Analyze specific table
ANALYZE links;
```

**REINDEX** - Rebuild corrupted indexes:

```sql
-- Reindex table
REINDEX TABLE links;

-- Reindex database
REINDEX DATABASE rustylinks;
```

### Automated Maintenance

PostgreSQL's **autovacuum** runs automatically (enabled by default).

**Check autovacuum status:**

```sql
SELECT * FROM pg_stat_user_tables
WHERE schemaname = 'public'
ORDER BY n_dead_tup DESC;
```

### Monitoring

**Active Connections:**

```sql
SELECT count(*) FROM pg_stat_activity WHERE datname = 'rustylinks';
```

**Long-Running Queries:**

```sql
SELECT
    pid,
    now() - query_start as duration,
    state,
    query
FROM pg_stat_activity
WHERE state = 'active'
ORDER BY duration DESC;
```

**Database Size:**

```sql
SELECT pg_size_pretty(pg_database_size('rustylinks'));
```

### Troubleshooting

**Too Many Connections:**

```sql
-- See current connections
SELECT count(*) FROM pg_stat_activity;

-- Kill idle connections
SELECT pg_terminate_backend(pid)
FROM pg_stat_activity
WHERE state = 'idle'
AND datname = 'rustylinks';
```

**Slow Queries:**

```sql
-- Enable slow query logging
ALTER DATABASE rustylinks SET log_min_duration_statement = 1000; -- 1 second

-- Check logs
docker compose logs postgres | grep "duration"
```

**Deadlocks:**

```sql
-- View deadlocks
SELECT * FROM pg_stat_database WHERE datname = 'rustylinks';
```

---

## Security Best Practices

### Database Security

1. **Use strong passwords** - Generate with `openssl rand -base64 32`
2. **Limit connections** - Only allow from application server
3. **Use SSL/TLS** - Encrypt database connections in production
4. **Regular backups** - Automated daily backups with off-site storage
5. **Principle of least privilege** - Application user only needs DML permissions
6. **Monitor access** - Review `pg_stat_activity` regularly

### User Permissions

```sql
-- Create restricted user for application
CREATE USER rustylinks_app WITH PASSWORD 'secure_password';

-- Grant only necessary permissions
GRANT CONNECT ON DATABASE rustylinks TO rustylinks_app;
GRANT USAGE ON SCHEMA public TO rustylinks_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO rustylinks_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO rustylinks_app;

-- Revoke superuser access
ALTER USER rustylinks_app WITH NOSUPERUSER;
```

### Backup Security

- **Encrypt backups** - Use `gpg` or similar
- **Secure storage** - Store backups in encrypted location
- **Access control** - Limit who can access backups
- **Test restores** - Verify backups work regularly

---

## References

- [PostgreSQL Documentation](https://www.postgresql.org/docs/)
- [SQLx Documentation](https://docs.rs/sqlx/)
- [PostgreSQL Performance Tuning](https://wiki.postgresql.org/wiki/Performance_Optimization)
- [PostgreSQL Security](https://www.postgresql.org/docs/current/security.html)

---

## Support

For database-related issues:
- Check migration files in `migrations/` directory
- Review SQLx query logs (set `RUST_LOG=sqlx=debug`)
- Consult PostgreSQL logs: `docker compose logs postgres`
- See [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for common issues
