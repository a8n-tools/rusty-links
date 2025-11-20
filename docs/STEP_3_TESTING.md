# Step 3: Database Schema Testing Guide

This document provides instructions for testing the database schema implementation.

## What Was Implemented

### 1. SQLx Migrations Structure
- Created `migrations/` directory
- Added two migration files:
  - `20250101000001_initial_schema.sql` - Complete database schema
  - `20250101000002_seed_data.sql` - Predefined languages and licenses

### 2. Database Schema

#### Core Tables Created:
1. **users** - User accounts with email and password
2. **links** - Bookmarked links with metadata
3. **categories** - 3-level hierarchical categories
4. **languages** - Programming languages
5. **licenses** - Software licenses
6. **tags** - User-defined tags

#### Junction Tables:
- **link_categories** - Many-to-many: links ↔ categories
- **link_languages** - Many-to-many: links ↔ languages (ordered)
- **link_licenses** - Many-to-many: links ↔ licenses (ordered)
- **link_tags** - Many-to-many: links ↔ tags (ordered)

### 3. Seed Data
- 20 predefined programming languages
- 20 predefined software licenses

### 4. Automatic Migration Execution
- Updated `src/main.rs` to run migrations on startup using `sqlx::migrate!()`
- Added comprehensive error handling and logging

## Testing Instructions

### Prerequisites
1. Ensure PostgreSQL is installed and running
2. Create the database: `createdb rusty_links`
3. Update `.env` file with correct DATABASE_URL

### Running the Application

```bash
# From project root
cargo run
```

### Expected Output

You should see JSON-formatted logs indicating:
```
{"level":"INFO","message":"Rusty Links starting..."}
{"level":"INFO","message":"Configuration loaded successfully",...}
{"level":"INFO","message":"Connecting to database..."}
{"level":"INFO","message":"Database connection established"}
{"level":"INFO","message":"Running database migrations..."}
{"level":"INFO","message":"Database migrations completed successfully"}
{"level":"INFO","message":"Database schema initialized. Ready for Step 4."}
```

### Verifying the Schema

Connect to PostgreSQL and verify the schema:

```bash
psql rusty_links
```

#### Check Tables:
```sql
\dt
```

Expected output: 11 tables (users, links, categories, languages, licenses, tags, and 5 junction tables)

#### Check Users Table:
```sql
\d users
```

Expected columns: id, email, password_hash, created_at

#### Check Links Table:
```sql
\d links
```

Expected columns: id, user_id, url, domain, path, title, description, logo, source_code_url, documentation_url, notes, status, github_stars, github_archived, github_last_commit, created_at, updated_at, refreshed_at

#### Verify Seed Data:
```sql
-- Should return 20 languages
SELECT COUNT(*) FROM languages;

-- Should return 20 licenses
SELECT COUNT(*) FROM licenses;

-- View all predefined languages
SELECT name FROM languages ORDER BY name;

-- View all predefined licenses
SELECT name, full_name FROM licenses ORDER BY name;
```

### Testing Constraints

#### Test Unique Email Constraint:
```sql
-- This should succeed
INSERT INTO users (email, password_hash)
VALUES ('test@example.com', 'hash123');

-- This should fail with unique constraint violation
INSERT INTO users (email, password_hash)
VALUES ('test@example.com', 'hash456');
```

#### Test Link Domain/Path Uniqueness:
```sql
-- Insert a test user first
INSERT INTO users (id, email, password_hash)
VALUES ('550e8400-e29b-41d4-a716-446655440000', 'test2@example.com', 'hash');

-- This should succeed
INSERT INTO links (user_id, url, domain, path)
VALUES ('550e8400-e29b-41d4-a716-446655440000', 'https://example.com/path', 'example.com', '/path');

-- This should fail (duplicate domain+path for same user)
INSERT INTO links (user_id, url, domain, path)
VALUES ('550e8400-e29b-41d4-a716-446655440000', 'https://example.com/path', 'example.com', '/path');
```

#### Test Link Status Check Constraint:
```sql
-- This should succeed
INSERT INTO links (user_id, url, domain, path, status)
VALUES ('550e8400-e29b-41d4-a716-446655440000', 'https://test.com', 'test.com', '/', 'active');

-- This should fail (invalid status)
INSERT INTO links (user_id, url, domain, path, status)
VALUES ('550e8400-e29b-41d4-a716-446655440000', 'https://test2.com', 'test2.com', '/', 'invalid_status');
```

#### Test Category Depth Constraint:
```sql
-- This should succeed (depth 0-2 allowed)
INSERT INTO categories (user_id, name, depth)
VALUES ('550e8400-e29b-41d4-a716-446655440000', 'Test Category', 0);

-- This should fail (depth > 2)
INSERT INTO categories (user_id, name, depth)
VALUES ('550e8400-e29b-41d4-a716-446655440000', 'Invalid Category', 3);
```

### Testing Foreign Key Cascades

```sql
-- Create test user
INSERT INTO users (id, email, password_hash)
VALUES ('550e8400-e29b-41d4-a716-446655440001', 'cascade@test.com', 'hash');

-- Create test link
INSERT INTO links (id, user_id, url, domain, path)
VALUES ('650e8400-e29b-41d4-a716-446655440000', '550e8400-e29b-41d4-a716-446655440001',
        'https://test.com', 'test.com', '/');

-- Delete user - should cascade and delete link
DELETE FROM users WHERE id = '550e8400-e29b-41d4-a716-446655440001';

-- Verify link was deleted
SELECT COUNT(*) FROM links WHERE id = '650e8400-e29b-41d4-a716-446655440000';
-- Should return 0
```

## Creating New Migrations

To create additional migrations in the future:

1. Install sqlx-cli (if not already installed):
```bash
cargo install sqlx-cli --no-default-features --features postgres
```

2. Create a new migration:
```bash
sqlx migrate add <migration_name>
```

3. Edit the generated SQL file in `migrations/`

4. Migrations run automatically on application startup

## Troubleshooting

### Error: "Failed to connect to database"
- Verify PostgreSQL is running
- Check DATABASE_URL in `.env` file
- Ensure database exists: `createdb rusty_links`
- Verify credentials are correct

### Error: "Failed to run database migrations"
- Check migration files for syntax errors
- Verify database user has CREATE TABLE permissions
- Look at detailed error message in logs

### Error: "relation already exists"
- Drop and recreate the database:
  ```bash
  dropdb rusty_links
  createdb rusty_links
  cargo run
  ```

## Next Steps

After verifying the schema works correctly, proceed to:
- **Step 4**: Database Connection Pool (already partially implemented)
- **Step 5**: Error Handling Framework
- **Step 6**: User Model and Database Operations
