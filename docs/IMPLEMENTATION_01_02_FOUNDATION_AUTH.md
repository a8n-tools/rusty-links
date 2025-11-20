# Rusty Links Implementation Guide
# Part 1-2: Foundation & Authentication (Steps 1-9)

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

## PART 2: Authentication Layer (Steps 6-9)

### **Step 6: User Model and Database Operations**

**Context:** With database and error handling ready (Steps 1-5), implement the User model with Argon2 password hashing.

**Prompt:**

````
Implement the User model with secure password handling for Rusty Links. Build on Steps 1-5.

Requirements:

1. Add dependencies to Cargo.toml:
   - argon2 (latest version)
   - uuid (with "serde" feature)
   - chrono (with "serde" feature)

2. Create src/models/user.rs with:

   **User struct:**
   ```rust
   pub struct User {
       pub id: Uuid,
       pub email: String,
       pub password_hash: String,
       pub created_at: DateTime<Utc>,
   }
   ```

   **CreateUser struct for new users:**
   ```rust
   pub struct CreateUser {
       pub email: String,
       pub password: String,
   }
   ```

3. Implement user database operations in src/models/user.rs:

   **create_user(pool: &PgPool, create_user: CreateUser) -> Result<User, AppError>**
   - Validate email format (must contain @ and domain)
   - Hash password using Argon2 with recommended parameters
   - Insert into database
   - Handle unique constraint violation (duplicate email)
   - Return created User

   **find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, AppError>**
   - Query user by email (case-insensitive)
   - Return Option<User>

   **verify_password(password: &str, hash: &str) -> Result<bool, AppError>**
   - Use Argon2 to verify password against hash
   - Return boolean result

   **check_user_exists(pool: &PgPool) -> Result<bool, AppError>**
   - Check if ANY user exists in database
   - Used to determine if setup is needed
   - Return boolean

4. Create src/models/mod.rs and export user module

5. Add comprehensive error handling:
   - Email validation errors
   - Password hashing errors
   - Database errors

6. Add logging for:
   - User creation (log email, not password)
   - Authentication attempts (log success/failure, not passwords)

Do not create API endpoints or UI yet. Test the functions work correctly with direct database calls from main.rs (you can add temporary test code that you'll remove).
````

---

### **Step 7: Session Management**

**Context:** With User model complete (Steps 1-6), implement secure session management with cookies.

**Prompt:**

````
Implement session management for Rusty Links with secure cookie-based sessions. Build on Steps 1-6.

Requirements:

1. Add dependencies to Cargo.toml:
   - axum (with "tokio" feature) - for HTTP server and middleware
   - axum-extra (with "cookie" feature) - for cookie handling
   - tower (with "util" feature) - for middleware
   - tower-http (with "cors", "trace" features)
   - rand (latest version) - for session token generation

2. Create src/auth/session.rs with:

   **Session struct:**
   ```rust
   pub struct Session {
       pub id: String,
       pub user_id: Uuid,
       pub created_at: DateTime<Utc>,
   }
   ```

3. Add to database migration (create a new migration file):
   **sessions table:**
   - id: TEXT primary key (random token)
   - user_id: UUID foreign key to users(id) on delete cascade
   - created_at: TIMESTAMP WITH TIME ZONE default now()
   - Add index on user_id

4. Implement session functions in src/auth/session.rs:

   **create_session(pool: &PgPool, user_id: Uuid) -> Result<Session, AppError>**
   - Generate secure random session ID (32 bytes, hex-encoded)
   - Insert into sessions table
   - Return Session

   **get_session(pool: &PgPool, session_id: &str) -> Result<Option<Session>, AppError>**
   - Query session by ID
   - Return Option<Session>

   **delete_session(pool: &PgPool, session_id: &str) -> Result<(), AppError>**
   - Delete session from database
   - Used for logout

   **delete_all_user_sessions(pool: &PgPool, user_id: Uuid) -> Result<(), AppError>**
   - Delete all sessions for a user
   - For future use

5. Implement cookie helpers:

   **create_session_cookie(session_id: &str) -> Cookie**
   - Name: "session_id"
   - HttpOnly: true
   - Secure: true (for HTTPS)
   - SameSite: Lax
   - Path: "/"
   - Max-Age: None (session cookie, persists indefinitely until logout)

   **get_session_from_cookie(cookies: &Cookies) -> Option<String>**
   - Extract session_id from cookies

6. Create src/auth/mod.rs and export session module

7. Add logging for session operations

Do not create authentication endpoints yet. Test session creation/retrieval/deletion works correctly.
````

---

### **Step 8: Authentication API Endpoints**

**Context:** With User and Session models complete (Steps 1-7), create the authentication REST API endpoints.

**Prompt:**

````
Implement authentication API endpoints for Rusty Links using Axum. Build on Steps 1-7.

Requirements:

1. Create src/api/auth.rs with authentication endpoints:

2. **POST /api/auth/setup**
   - Request body: `{ "email": "user@example.com", "password": "secret" }`
   - Check if any user exists using check_user_exists()
   - If user exists, return 403 Forbidden with error "Setup already completed"
   - If no user exists, create first user with create_user()
   - Create session for new user
   - Set session cookie
   - Return 201 Created with user info (id, email, created_at - NO password)
   - Handle validation errors (invalid email format)
   - Handle duplicate email errors

3. **POST /api/auth/login**
   - Request body: `{ "email": "user@example.com", "password": "secret" }`
   - Find user by email
   - Verify password
   - If invalid: return 401 with generic message "Invalid credentials"
   - If valid: create new session, set cookie, return 200 with user info
   - Log authentication attempts (success/failure, email only)

4. **POST /api/auth/logout**
   - Requires authentication (read session cookie)
   - Delete session from database
   - Clear session cookie
   - Return 200 with success message

5. **GET /api/auth/me**
   - Requires authentication
   - Return current user info
   - If not authenticated, return 401

6. **GET /api/auth/check-setup**
   - Public endpoint
   - Return `{ "setup_required": boolean }`
   - Used by frontend to determine if setup page should be shown

7. Create authentication middleware:
   - Extract session from cookie
   - Verify session exists in database
   - Attach User to request extensions
   - Return 401 if not authenticated

8. In src/api/mod.rs:
   - Create and export auth module
   - Create function to build Axum router with all auth routes

9. In src/main.rs:
   - Set up Axum server on configured port
   - Mount auth routes under /api/auth
   - Add CORS middleware (allow all origins for Phase 1)
   - Add request tracing middleware
   - Start server and log "Server listening on port {}"

10. Add request/response logging with tracing

Test all endpoints with curl or similar tool:
- Setup endpoint creates first user
- Login works with correct credentials
- Login fails with wrong credentials
- Logout clears session
- /me returns user when authenticated
- /me returns 401 when not authenticated
````

---

### **Step 9: Authentication UI Components**

**Context:** With auth API complete (Steps 1-8), create the Dioxus frontend components for authentication.

**Prompt:**

````
Create the authentication UI components for Rusty Links using Dioxus. Build on Steps 1-8.

Requirements:

1. Set up basic Dioxus structure in src/ui/:
   - Create src/ui/mod.rs
   - Create src/ui/app.rs - root component
   - Create src/ui/pages/ directory for page components
   - Create src/ui/components/ directory for reusable components

2. In src/ui/pages/setup.rs, create Setup page:
   - Title: "Rusty Links - Setup"
   - Professional, clean design with muted rust color tones
   - Email input field with validation hint
   - Password input field (type="password")
   - Submit button: "Create Account"
   - Form submission:
     * POST to /api/auth/setup
     * Show loading state during submission
     * On success: redirect to /login
     * On error: display error message below form
   - No password requirements or strength indicator (Phase 1)

3. In src/ui/pages/login.rs, create Login page:
   - Title: "Rusty Links - Login"
   - Same visual style as Setup page
   - Email input field
   - Password input field
   - Submit button: "Log In"
   - Form submission:
     * POST to /api/auth/login
     * Show loading state
     * On success: redirect to /links (main page)
     * On error: show "Invalid credentials" message
   - No "remember me" or "forgot password" options (Phase 1)

4. In src/ui/app.rs, create App component with routing:
   - Use Dioxus Router
   - Routes:
     * / - Check setup status, redirect to /setup if needed, else /login
     * /setup - Setup page (only accessible if no user exists)
     * /login - Login page
     * /links - Main links page (placeholder for now, requires auth)
   - Add routing logic to check setup status on app load
   - Fetch /api/auth/check-setup and route appropriately

5. In src/ui/components/navbar.rs, create basic Navbar component:
   - Show "Rusty Links" branding with rusty chain link icon (use text for now)
   - Logout button that calls POST /api/auth/logout
   - Only shown on authenticated pages
   - Horizontal menu (we'll add more items later)

6. Create placeholder src/ui/pages/links.rs:
   - Just show "Links Page - Coming Soon" message
   - Include Navbar component
   - We'll implement the full links table in next steps

7. Update src/main.rs:
   - Serve Dioxus app at root /
   - Mount API routes under /api
   - Configure static file serving for Dioxus assets

8. Add basic CSS (inline or separate file) for:
   - Professional, clean styling
   - Rust-themed color palette (muted rust/orange tones)
   - Responsive forms
   - Button styles
   - Error message styling

Test the complete authentication flow:
- First visit shows setup page
- After setup, login page appears
- Login works and shows links placeholder page
- Logout returns to login page
- Direct navigation to /links without auth redirects to login
````

---

## Summary

Parts 1-2 establish the foundation and authentication for Rusty Links:

**Foundation (Steps 1-5):**
- Project structure and dependencies
- Configuration management
- Database schema and migrations
- Connection pooling
- Error handling framework

**Authentication (Steps 6-9):**
- User model with Argon2 password hashing
- Session management with secure cookies
- Authentication API endpoints
- Login/Setup/Logout UI

**Next Steps:**
Continue with Part 3 (Core Data Models) to implement Links, Categories, Languages, Licenses, and Tags models.
