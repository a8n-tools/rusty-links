# Rusty Links Implementation Guide
# Part 1: Foundation (Steps 1-5)

This document contains implementation prompts for building the foundation and authentication layers of Rusty Links.

---

## PART 1: Foundation Layer (Steps 1-5)

### **Step 1: Project Initialization**

**Context:** Starting a new Rust project with Dioxus Fullstack framework for a self-hosted bookmark manager called "Rusty Links".

**Prompt:**

````
Initialize a new Rust project for a bookmark manager application called "Rusty Links". Set up the project structure with the following requirements:

1. Create a new Cargo project with these dependencies:
   - dioxus (latest stable with "fullstack" feature)
   - dioxus-web
   - sqlx (with "postgres", "runtime-tokio-native-tls", "migrate" features)
   - tokio (with "full" feature)
   - serde and serde_json for serialization
   - dotenvy for environment variables
   - tracing and tracing-subscriber for structured logging

2. Set up the project structure:
   - src/main.rs - entry point
   - src/config.rs - configuration module (stub for now)
   - src/models/ - database models directory
   - src/api/ - API endpoints directory
   - src/ui/ - frontend components directory
   - src/auth/ - authentication module directory
   - src/scraper/ - web scraping module directory
   - src/github/ - GitHub integration directory
   - src/scheduler/ - background jobs directory

3. Create a basic main.rs that:
   - Initializes tracing with JSON formatting
   - Logs "Rusty Links starting..." message
   - Has a placeholder async main function

4. Add appropriate .gitignore entries for Rust projects

5. Ensure the project compiles successfully

Keep the code minimal and focused on structure. We'll implement functionality incrementally in subsequent steps.
````

---

### **Step 2: Configuration Management**

**Context:** Building on Step 1, we need to handle environment variables for database connection, port, and update intervals.

**Prompt:**

````
Implement the configuration management system for Rusty Links. Build on the existing project structure from Step 1.

Requirements:

1. In src/config.rs, create a Config struct with these fields:
   - database_url: String (required)
   - app_port: u16 (required)
   - update_interval_days: u32 (optional, default: 30, minimum: 1)
   - log_level: String (optional, from RUST_LOG)

2. Implement Config::from_env() that:
   - Uses dotenvy to load .env file
   - Reads environment variables
   - Validates required variables exist
   - Validates update_interval_days >= 1
   - Returns Result<Config, ConfigError>

3. Create a custom ConfigError enum with variants for:
   - Missing required variables
   - Invalid values
   - Parsing errors
   - Implement Display for user-friendly error messages

4. Update main.rs to:
   - Load configuration at startup
   - Log configuration (mask sensitive data like database_url)
   - Exit gracefully with clear error message if config fails

5. Create an example .env.example file with:
   ```
   DATABASE_URL=postgresql://user:password@localhost/rusty_links
   APP_PORT=8080
   UPDATE_INTERVAL_DAYS=30
   RUST_LOG=info
   ```

Test that the application starts successfully with valid config and fails clearly with missing/invalid config.
````

---

### **Step 3: Database Schema and Initial Migration**

**Context:** With configuration in place (Steps 1-2), set up PostgreSQL database schema with SQLx migrations.

**Prompt:**

````
Create the complete database schema for Rusty Links using SQLx migrations. Build on the existing project from Steps 1-2.

Requirements:

1. Set up SQLx migrations:
   - Create migrations/ directory
   - Add sqlx-cli to dev-dependencies or document its installation

2. Create initial migration (migrations/001_initial_schema.sql) with these tables:

   **users table:**
   - id: UUID primary key with default gen_random_uuid()
   - email: TEXT unique not null
   - password_hash: TEXT not null
   - created_at: TIMESTAMP WITH TIME ZONE default now()
   - Add index on email

   **links table:**
   - id: UUID primary key with default gen_random_uuid()
   - user_id: UUID foreign key to users(id) on delete cascade
   - url: TEXT not null
   - domain: TEXT not null
   - path: TEXT not null
   - title: TEXT nullable
   - description: TEXT nullable
   - logo: BYTEA nullable
   - source_code_url: TEXT nullable
   - documentation_url: TEXT nullable
   - notes: TEXT nullable
   - status: TEXT not null default 'active' check (status in ('active', 'archived', 'inaccessible', 'repo_unavailable'))
   - github_stars: INTEGER nullable
   - github_archived: BOOLEAN nullable
   - github_last_commit: DATE nullable
   - created_at: TIMESTAMP WITH TIME ZONE default now()
   - updated_at: TIMESTAMP WITH TIME ZONE default now()
   - refreshed_at: TIMESTAMP WITH TIME ZONE nullable
   - Unique constraint on (user_id, domain, path)
   - Add indexes on user_id, domain, status, created_at

   **categories table:**
   - id: UUID primary key with default gen_random_uuid()
   - user_id: UUID foreign key to users(id) on delete cascade
   - name: TEXT not null
   - parent_id: UUID nullable foreign key to categories(id) on delete cascade
   - depth: INTEGER not null check (depth >= 0 and depth <= 2)
   - sort_order: INTEGER nullable
   - created_at: TIMESTAMP WITH TIME ZONE default now()
   - Unique constraint on (user_id, lower(name))
   - Add indexes on user_id, parent_id

   **languages table:**
   - id: UUID primary key with default gen_random_uuid()
   - user_id: UUID foreign key to users(id) on delete cascade
   - name: TEXT not null
   - created_at: TIMESTAMP WITH TIME ZONE default now()
   - Unique constraint on (user_id, lower(name))
   - Add index on user_id

   **licenses table:**
   - id: UUID primary key with default gen_random_uuid()
   - user_id: UUID foreign key to users(id) on delete cascade
   - name: TEXT not null (acronym)
   - full_name: TEXT not null
   - created_at: TIMESTAMP WITH TIME ZONE default now()
   - Unique constraint on (user_id, lower(name))
   - Add index on user_id

   **tags table:**
   - id: UUID primary key with default gen_random_uuid()
   - user_id: UUID foreign key to users(id) on delete cascade
   - name: TEXT not null
   - created_at: TIMESTAMP WITH TIME ZONE default now()
   - Unique constraint on (user_id, lower(name))
   - Add index on user_id

   **Junction tables:**
   - link_categories (link_id UUID, category_id UUID, primary key (link_id, category_id))
   - link_languages (link_id UUID, language_id UUID, order_num INTEGER, primary key (link_id, language_id))
   - link_licenses (link_id UUID, license_id UUID, order_num INTEGER, primary key (link_id, license_id))
   - link_tags (link_id UUID, tag_id UUID, order_num INTEGER, primary key (link_id, tag_id))

   All junction tables should have foreign keys with on delete cascade

3. Create seed data migration (migrations/002_seed_data.sql) that inserts:
   - 20 predefined languages (JavaScript, Python, Java, C#, C++, TypeScript, PHP, C, Ruby, Go, Rust, Swift, Kotlin, R, Dart, Scala, Perl, Lua, Haskell, Elixir)
   - 20 predefined licenses with acronyms and full names (MIT, Apache-2.0, GPL-3.0, etc. as listed in spec)
   - Note: These seed with user_id = NULL temporarily (we'll update this later)

4. Update main.rs to run migrations automatically on startup using sqlx::migrate!()

5. Document in comments that migrations run automatically and how to create new migrations

Test that the application starts and creates the database schema successfully.
````

---

### **Step 4: Database Connection Pool**

**Context:** With schema in place (Steps 1-3), establish the database connection pool that will be used throughout the application.

**Prompt:**

````
Set up the PostgreSQL connection pool for Rusty Links. Build on Steps 1-3.

Requirements:

1. In src/main.rs, create a database connection pool:
   - Use sqlx::postgres::PgPoolOptions
   - Configure with:
     * Max connections: 5 (appropriate for single-user)
     * Connection timeout: 30 seconds
     * Idle timeout: 10 minutes
   - Connect using Config::database_url
   - Run pending migrations automatically: sqlx::migrate!().run(&pool).await

2. Add error handling for:
   - Database connection failures
   - Migration failures
   - Log clear, actionable error messages

3. Create a function in src/main.rs:
   ```rust
   async fn initialize_database(database_url: &str) -> Result<PgPool, Box<dyn std::error::Error>> {
       // Implementation here
   }
   ```

4. Update main() to:
   - Call initialize_database()
   - Store the pool for later use
   - Log "Database connected successfully" on success
   - Exit with error message on failure

5. Add a health check query to verify the connection:
   - Run a simple SELECT 1 query after migrations
   - Log the result

Do not create any API endpoints yet - we're just establishing the database connection. Test that the application connects to PostgreSQL successfully and runs migrations.
````

---

### **Step 5: Error Handling Framework**

**Context:** Before building features (Steps 1-4 complete), establish consistent error handling patterns.

**Prompt:**

````
Create a comprehensive error handling framework for Rusty Links that will be used across the application. Build on Steps 1-4.

Requirements:

1. Create src/error.rs with an AppError enum covering:
   - Database errors (sqlx::Error)
   - Configuration errors
   - Validation errors (with field name and message)
   - Authentication errors (invalid credentials, session expired, unauthorized)
   - NotFound errors (with resource type and id)
   - Duplicate errors (with field name)
   - External service errors (GitHub API, web scraping)
   - Internal server errors

2. Implement conversions From<T> for AppError from common error types:
   - sqlx::Error
   - std::io::Error
   - serde_json::Error

3. Create an ApiErrorResponse struct for JSON responses:
   ```rust
   {
       "error": "User-friendly message",
       "code": "ERROR_CODE",
       "status": 400
   }
   ```

4. Implement Display and Debug for AppError with:
   - User-friendly messages for frontend
   - Detailed messages for logs
   - Appropriate HTTP status codes for each variant

5. Create helper functions:
   - AppError::validation(field: &str, message: &str) -> Self
   - AppError::not_found(resource: &str, id: &str) -> Self
   - AppError::duplicate(field: &str) -> Self

6. Add AppError to your main module exports

7. Update existing code in main.rs, config.rs to use AppError where appropriate

Do not implement API error responses yet - we're establishing the error types. Document the error handling strategy with comments. Test that errors compile and display correctly.
````

---

## Summary

Part 1 establishes the foundation for Rusty Links:

**Foundation (Steps 1-5):**
- Project structure and dependencies
- Configuration management
- Database schema and migrations
- Connection pooling
- Error handling framework

**Next Steps:**
Continue with Part 2 (Authentication) to Authentication.
