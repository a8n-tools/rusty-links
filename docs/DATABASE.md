# Database Documentation

Complete database schema reference for Rusty Links PostgreSQL database.

## Table of Contents

- [Overview](#overview)
- [Connection Details](#connection-details)
- [Schema Summary](#schema-summary)
- [Entity Relationship Diagram](#entity-relationship-diagram)
- [Tables Reference](#tables-reference)
- [Migrations](#migrations)
- [Indexes](#indexes)
- [Backup and Restore](#backup-and-restore)
- [Performance Tuning](#performance-tuning)
- [Maintenance](#maintenance)

---

## Overview

Rusty Links uses PostgreSQL 14+ as its database backend. The schema is designed with:

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
| `sessions`   | Authentication sessions | 1-5 per user    |
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
  ├──< sessions (user_id)
  │     └─ id (session token)
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

Legend:
  ├──<  One-to-many relationship
  >──   Many-to-one relationship
  └───  Self-referencing relationship
```

### Relationship Details

- **User → Links**: One user can have many links (CASCADE DELETE)
- **User → Categories**: One user can have many categories (CASCADE DELETE)
- **User → Tags**: One user can have many tags (CASCADE DELETE)
- **User → Sessions**: One user can have many sessions (CASCADE DELETE)
- **Link → Categories**: Many-to-many via `link_categories`
- **Link → Tags**: Many-to-many via `link_tags` (with ordering)
- **Link → Languages**: Many-to-many via `link_languages` (with ordering)
- **Link → Licenses**: Many-to-many via `link_licenses` (with ordering)
- **Category → Category**: Self-referencing for hierarchy (CASCADE DELETE)
- **Languages/Licenses**: Can be global (user_id = NULL) or user-specific

---

## Tables Reference

### users

User accounts table (single-user application, but supports multiple users).

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);
```

**Columns:**

| Column          | Type        | Constraints             | Description                |
|-----------------|-------------|-------------------------|----------------------------|
| `id`            | UUID        | PRIMARY KEY             | User identifier            |
| `email`         | TEXT        | NOT NULL, UNIQUE        | User email address         |
| `password_hash` | TEXT        | NOT NULL                | Argon2 password hash       |
| `created_at`    | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | Account creation timestamp |

**Indexes:**
- `idx_users_email` - Fast email lookups for authentication

**Notes:**
- Passwords are hashed using Argon2
- Email must be unique (case-sensitive)
- Deleting a user cascades to all their data

---

### sessions

Session tokens for authentication.

```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);
```

**Columns:**

| Column       | Type        | Constraints              | Description                |
|--------------|-------------|--------------------------|----------------------------|
| `id`         | TEXT        | PRIMARY KEY              | Session token (hex string) |
| `user_id`    | UUID        | NOT NULL, FK → users(id) | Owner of session           |
| `created_at` | TIMESTAMPTZ | NOT NULL, DEFAULT NOW()  | Session creation time      |

**Indexes:**
- `idx_sessions_user_id` - Find all sessions for a user

**Notes:**
- Session ID is a secure random token
- No expiration mechanism (manual cleanup required)
- Stored in HTTP-only cookie on client

---

### links

Bookmarked links with metadata.

```sql
CREATE TABLE links (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    domain TEXT NOT NULL,
    path TEXT NOT NULL,
    title TEXT,
    description TEXT,
    logo BYTEA,
    source_code_url TEXT,
    documentation_url TEXT,
    notes TEXT,
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'archived', 'inaccessible', 'repo_unavailable')),
    github_stars INTEGER,
    github_archived BOOLEAN,
    github_last_commit DATE,
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
| `path` | TEXT | NOT NULL | URL path |
| `title` | TEXT | NULL | Page title (auto-extracted) |
| `description` | TEXT | NULL | Page description (auto-extracted) |
| `logo` | BYTEA | NULL | Site logo/favicon (binary) |
| `source_code_url` | TEXT | NULL | Link to source code |
| `documentation_url` | TEXT | NULL | Link to documentation |
| `notes` | TEXT | NULL | User notes |
| `status` | TEXT | NOT NULL, DEFAULT 'active' | Link status (see values below) |
| `github_stars` | INTEGER | NULL | GitHub stars count |
| `github_archived` | BOOLEAN | NULL | GitHub archived status |
| `github_last_commit` | DATE | NULL | Last GitHub commit date |
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

**Unique Constraints:**
- `uq_links_user_domain_path` - Prevent duplicate URLs per user

**Notes:**
- Logo is stored as binary data (BYTEA)
- Domain and path are extracted for deduplication
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

## Migrations

### Migration System

Rusty Links uses SQLx migrations for schema management.

**Migration Files Location:**
```
migrations/
├── 20250101000001_initial_schema.sql
├── 20250101000002_seed_data.sql
├── 20250101000003_sessions_table.sql
├── 20250101000004_add_failure_count.sql
└── 20250101000005_add_scheduler_fields.sql
```

**Naming Convention:**
```
YYYYMMDDHHMMSS_description.sql
```

### Running Migrations

Migrations run **automatically** on application startup.

**Manual Migration Commands:**

```bash
# Install SQLx CLI
cargo install sqlx-cli --no-default-features --features postgres

# Run all pending migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert

# Create new migration
sqlx migrate add <migration_name>

# Get migration status
sqlx migrate info
```

### Migration History

| Version | Description | Date |
|---------|-------------|------|
| 20250101000001 | Initial schema - users, links, categories, tags, languages, licenses | 2025-01-01 |
| 20250101000002 | Seed global languages and licenses | 2025-01-01 |
| 20250101000003 | Add sessions table for authentication | 2025-01-01 |
| 20250101000004 | Add consecutive_failures to links | 2025-01-01 |
| 20250101000005 | Add scheduler fields (last_checked) to links | 2025-01-01 |

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
- Include both UP and DOWN migrations
- Never modify existing migrations (create new ones)
- Keep migrations small and focused
- Add comments explaining complex changes

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
| sessions | idx_sessions_user_id | user_id | B-tree | Find user's sessions |
| links | idx_links_user_id | user_id | B-tree | Find user's links |
| links | idx_links_domain | domain | B-tree | Filter by domain |
| links | idx_links_status | status | B-tree | Filter by status |
| links | idx_links_created_at | created_at | B-tree | Sort by date |
| links | idx_links_last_checked | last_checked | B-tree (partial) | Scheduler queries (WHERE NOT NULL) |
| links | idx_links_unchecked | last_checked | B-tree (partial) | Never-checked links (WHERE NULL) |
| categories | idx_categories_user_id | user_id | B-tree | Find user's categories |
| categories | idx_categories_parent_id | parent_id | B-tree | Find child categories |
| languages | idx_languages_user_id | user_id | B-tree | Find user's languages |
| licenses | idx_licenses_user_id | user_id | B-tree | Find user's licenses |
| tags | idx_tags_user_id | user_id | B-tree | Find user's tags |

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
# Backup everything except sessions
docker compose exec postgres pg_dump -U rustylinks -T sessions rustylinks > backup.sql
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
