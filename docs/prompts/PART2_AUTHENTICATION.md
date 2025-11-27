# Rusty Links Implementation Guide
# Part 2: Authentication (Steps 6-9)

This document contains implementation prompts for building the authentication layers of Rusty Links.

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

Parts 2 establishes authentication for Rusty Links:

**Authentication (Steps 6-9):**
- User model with Argon2 password hashing
- Session management with secure cookies
- Authentication API endpoints
- Login/Setup/Logout UI

**Next Steps:**
Continue with Part 3 (Core Data Models) to implement Links, Categories, Languages, Licenses, and Tags models.
