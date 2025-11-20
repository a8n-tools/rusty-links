# Step 7: Session Management

This document describes the session management implementation with secure cookie-based authentication for Rusty Links.

## Overview

The session management system provides:
- **Secure Token Generation**: 256-bit random session IDs
- **Database-Backed Sessions**: Stateful session storage
- **Secure Cookies**: HttpOnly, Secure, SameSite protection
- **Session Lifecycle**: Create, retrieve, delete operations
- **Multi-Device Support**: Users can have multiple active sessions
- **Logout Functionality**: Individual and bulk session deletion

## Implementation

### 1. Dependencies Added (Cargo.toml)

```toml
# Web framework additions
axum-extra = { version = "0.9", features = ["cookie"] }
tower = { version = "0.4", features = ["util"] }
tower-http = { version = "0.5", features = ["cors", "trace"] }

# Session management
rand = "0.8"
hex = "0.4"
```

**Purpose:**
- `axum-extra` - Cookie handling and extraction
- `tower` - Middleware utilities (for future auth middleware)
- `tower-http` - CORS and tracing middleware
- `rand` - Cryptographically secure random number generation
- `hex` - Hexadecimal encoding for session tokens

### 2. Database Migration (migrations/20250101000003_sessions_table.sql)

Creates the sessions table:

```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sessions_user_id ON sessions(user_id);
```

**Schema Design:**
- `id`: Session token (64-character hex string)
- `user_id`: Foreign key to users table with cascade delete
- `created_at`: Timestamp for audit/analytics
- Index on `user_id` for efficient user session queries

**Cascade Delete:**
When a user is deleted, all their sessions are automatically deleted.

### 3. Session Model (src/auth/session.rs)

#### Data Structure

```rust
pub struct Session {
    pub id: String,              // 64-character hex token
    pub user_id: Uuid,           // User this session belongs to
    pub created_at: DateTime<Utc>, // When session was created
}
```

#### Session Token Generation

**Security Properties:**
- **Token Length**: 32 bytes (256 bits)
- **Encoding**: Hexadecimal (64 characters)
- **Randomness Source**: `rand::thread_rng()` (cryptographically secure)
- **Entropy**: 256 bits (2^256 possible values)
- **Collision Probability**: Negligible (virtually impossible)

**Example Token:**
```
a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456
```

**Why 32 bytes?**
- 128 bits minimum for secure tokens (OWASP)
- 256 bits provides extra margin of safety
- Hex encoding is URL-safe and human-readable (for debugging)

#### Database Operations

**1. create_session(pool, user_id) -> Result<Session, AppError>**

Creates a new session for a user.

**Process:**
1. Generate secure random 32-byte token
2. Hex-encode to 64-character string
3. Insert into database with user_id
4. Return Session object

**Errors:**
- `AppError::Database` - Database operation failed

**Example:**
```rust
let session = create_session(&pool, user.id).await?;
let cookie = create_session_cookie(&session.id);
// Set cookie on HTTP response
```

**2. get_session(pool, session_id) -> Result<Option<Session>, AppError>**

Retrieves a session by its token.

**Process:**
1. Query database for session with matching ID
2. Return Some(Session) if found, None if not

**Errors:**
- `AppError::Database` - Database operation failed

**Use Case:**
- Validate session token from cookie
- Extract user_id for authentication

**Example:**
```rust
match get_session(&pool, session_id).await? {
    Some(session) => {
        // Valid session - user is authenticated
        let user_id = session.user_id;
    }
    None => {
        // Invalid session - require login
    }
}
```

**3. delete_session(pool, session_id) -> Result<(), AppError>**

Deletes a specific session (logout).

**Process:**
1. Delete session from database by ID
2. Return success (even if session didn't exist)

**Errors:**
- `AppError::Database` - Database operation failed

**Use Case:**
- User logout
- Session invalidation

**Example:**
```rust
delete_session(&pool, session_id).await?;
// Clear session cookie on HTTP response
```

**4. delete_all_user_sessions(pool, user_id) -> Result<(), AppError>**

Deletes all sessions for a user ("logout from all devices").

**Process:**
1. Delete all sessions matching user_id
2. Return number of sessions deleted

**Errors:**
- `AppError::Database` - Database operation failed

**Use Case:**
- "Logout from all devices" feature
- Security: User suspects compromise
- Password change (force re-authentication)

**Example:**
```rust
delete_all_user_sessions(&pool, user.id).await?;
// All devices will require re-login
```

### 4. Cookie Helpers (src/auth/session.rs)

#### Cookie Configuration

**create_session_cookie(session_id) -> Cookie**

Creates a secure session cookie.

**Settings:**
- **Name**: `"session_id"`
- **Value**: Session token (64 chars)
- **HttpOnly**: `true` (JavaScript cannot access)
- **Secure**: `true` (HTTPS only)
- **SameSite**: `Lax` (CSRF protection)
- **Path**: `"/"` (available on all routes)
- **Max-Age**: `None` (session cookie)

**Security Properties:**

| Property | Value | Protection Against |
|----------|-------|-------------------|
| HttpOnly | true | XSS attacks (JavaScript cannot steal token) |
| Secure | true | MITM attacks (only sent over HTTPS) |
| SameSite=Lax | true | CSRF attacks (limited cross-site requests) |
| Path="/" | true | N/A (available on all routes) |

**Session Cookie Behavior:**
- No Max-Age = cookie deleted when browser closes
- BUT server-side session persists
- If browser retains cookie (e.g., session restore), session remains valid
- Effectively "remember me" without explicit checkbox

**Example:**
```rust
let cookie = create_session_cookie(&session.id);
// In Axum handler:
// (jar.add(cookie), Json(user))
```

**create_clear_session_cookie() -> Cookie**

Creates a cookie that clears the session.

**Settings:**
- Same as session cookie
- **Value**: `""` (empty)
- **Max-Age**: `0` (immediate expiration)

**Use Case:**
- Logout - clear session cookie on client

**Example:**
```rust
let cookie = create_clear_session_cookie();
// Set on response to logout user
```

**get_session_from_cookies(cookies) -> Option<String>**

Extracts session ID from cookie jar.

**Process:**
1. Look for cookie named "session_id"
2. Return Some(String) with value if found
3. Return None if cookie not present

**Example:**
```rust
use axum_extra::extract::CookieJar;

async fn handler(cookies: CookieJar) {
    if let Some(session_id) = get_session_from_cookies(&cookies) {
        // Validate session with database
    }
}
```

### 5. Module Organization (src/auth/mod.rs)

```rust
pub mod session;

pub use session::{
    create_clear_session_cookie,
    create_session,
    create_session_cookie,
    delete_all_user_sessions,
    delete_session,
    get_session,
    get_session_from_cookies,
    Session,
    SESSION_COOKIE_NAME,
};
```

**Re-exports:**
Commonly used functions and types exported for convenience.

**Usage:**
```rust
use crate::auth::{create_session, get_session};
```

### 6. Logging

All session operations are logged for security monitoring:

**Session Creation:**
```json
{"level":"INFO","user_id":"550e8400-...","session_id":"a1b2c3d4...","message":"Creating new session"}
{"level":"INFO","user_id":"550e8400-...","session_id":"a1b2c3d4...","message":"Session created successfully"}
```

**Session Lookup:**
```json
{"level":"DEBUG","session_id":"a1b2c3d4...","message":"Looking up session"}
{"level":"DEBUG","session_id":"a1b2c3d4...","message":"Session found"}
```

**Session Deletion:**
```json
{"level":"INFO","session_id":"a1b2c3d4...","message":"Deleting session"}
{"level":"INFO","session_id":"a1b2c3d4...","rows_affected":1,"message":"Session deleted"}
```

**Bulk Deletion:**
```json
{"level":"INFO","user_id":"550e8400-...","message":"Deleting all sessions for user"}
{"level":"INFO","user_id":"550e8400-...","sessions_deleted":3,"message":"All user sessions deleted"}
```

## Testing

### Unit Tests (src/auth/session.rs)

The module includes unit tests:

```bash
cargo test --lib auth::session
```

**Test Coverage:**
- ✅ Session token generation (length, uniqueness, hex format)
- ✅ Session cookie settings (HttpOnly, Secure, SameSite)
- ✅ Clear session cookie (empty value, max_age=0)

### Integration Tests (main.rs)

Comprehensive integration tests verify:

1. **Session Creation**
   - Creates session for test user
   - Verifies session ID length (64 chars)
   - Verifies user_id match

2. **Session Retrieval**
   - Looks up created session
   - Verifies ID and user_id match

3. **Cookie Creation**
   - Creates session cookie
   - Verifies all security settings

4. **Multiple Sessions**
   - Creates second session for same user
   - Verifies different session IDs
   - Confirms multiple sessions allowed

5. **Non-Existent Session**
   - Looks up invalid session ID
   - Verifies returns None

6. **Session Deletion**
   - Deletes specific session
   - Verifies session no longer exists

7. **Bulk Deletion**
   - Deletes all user sessions
   - Verifies all sessions removed

### Running Tests

```bash
cargo run
```

**Expected Output:**
```
=== Session Management Tests ===

Using test user: test@rustylinks.local (ID: 550e8400-...)

Test 1: Creating session...
✓ Session created successfully
  - Session ID: a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456
  - Session ID length: 64 chars
  - User ID: 550e8400-e29b-41d4-a716-446655440000
  - Created at: 2025-01-20 10:30:45 UTC

Test 2: Retrieving session by ID...
✓ Session retrieved successfully
  - Session ID matches: true
  - User ID matches: true

Test 3: Creating session cookie...
✓ Session cookie created
  - Cookie name: session_id
  - Cookie value: a1b2c3d4e5f6...
  - HttpOnly: Some(true)
  - Secure: Some(true)
  - SameSite: Some(Lax)
  - Path: Some("/")

Test 4: Creating second session for same user...
✓ Second session created successfully
  - Session ID: f7e8d9c0b1a2...
  - Different from first: true

Test 5: Looking up non-existent session...
✓ Non-existent session correctly returned None

Test 6: Deleting first session...
✓ Session deleted successfully
✓ Session confirmed deleted (not found in database)

Test 7: Deleting all sessions for user...
✓ All user sessions deleted successfully
✓ Second session confirmed deleted

=== All Session Management Tests Complete ===
Step 7 implementation verified successfully!
```

## Security Considerations

### Token Security

**Randomness Quality:**
- Uses `rand::thread_rng()` which is cryptographically secure
- On Linux: Uses `/dev/urandom`
- On Windows: Uses `CryptGenRandom`
- On macOS: Uses `SecRandomCopyBytes`

**Token Length:**
- 256 bits of entropy
- Brute force: 2^256 attempts needed
- At 1 billion attempts/second: Would take 10^60 years

**Token Uniqueness:**
- Collision probability: 1 in 2^128 (negligible)
- No collision detection needed

### Cookie Security

**XSS Protection (HttpOnly):**
- JavaScript cannot access cookie
- Even if XSS vulnerability exists, session token protected
- `document.cookie` does not reveal session_id

**MITM Protection (Secure):**
- Cookie only sent over HTTPS
- No transmission over HTTP
- Prevents network sniffing

**CSRF Protection (SameSite=Lax):**
- Cookie not sent on cross-site POST requests
- Sent on normal navigation (GET requests)
- Balance between security and usability

### Session Fixation Protection

**How It's Prevented:**
- New session token generated at login
- Old sessions can be deleted
- No session reuse across authentication states

**Best Practice:**
When implementing login endpoint:
1. Delete any existing anonymous session
2. Create new authenticated session
3. Set new session cookie

### Session Hijacking Mitigations

**Current:**
- Secure random tokens (hard to guess)
- HTTPS-only cookies (prevents network interception)
- Database-backed (can be invalidated server-side)

**Future Enhancements:**
- IP address validation
- User agent validation
- Session timeout/expiration
- Activity tracking
- Anomaly detection

## Session Lifecycle

### 1. Login Flow

```
User submits credentials
    ↓
Verify email/password (Step 6: User model)
    ↓
create_session(pool, user.id)
    ↓
create_session_cookie(&session.id)
    ↓
Set cookie on response
    ↓
User is authenticated
```

### 2. Authenticated Request Flow

```
Request with cookie
    ↓
get_session_from_cookies(&cookies)
    ↓
get_session(pool, session_id)
    ↓
If Some(session):
    - Extract user.id
    - Load user from database
    - Attach to request
    - Process request
If None:
    - Return 401 Unauthorized
```

### 3. Logout Flow

```
Request with cookie
    ↓
get_session_from_cookies(&cookies)
    ↓
delete_session(pool, session_id)
    ↓
create_clear_session_cookie()
    ↓
Set cookie on response
    ↓
User is logged out
```

### 4. Logout All Devices Flow

```
Authenticated request
    ↓
delete_all_user_sessions(pool, user.id)
    ↓
create_clear_session_cookie()
    ↓
All sessions invalidated
    ↓
User must re-login on all devices
```

## Database Schema

### Sessions Table

```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sessions_user_id ON sessions(user_id);
```

**Performance:**
- Primary key on `id` for O(1) session lookup
- Index on `user_id` for efficient user session queries
- Foreign key with cascade for automatic cleanup

**Queries:**
```sql
-- Create session
INSERT INTO sessions (id, user_id) VALUES ($1, $2);

-- Get session
SELECT * FROM sessions WHERE id = $1;

-- Delete session
DELETE FROM sessions WHERE id = $1;

-- Delete all user sessions
DELETE FROM sessions WHERE user_id = $1;

-- Count user sessions
SELECT COUNT(*) FROM sessions WHERE user_id = $1;
```

## Usage Examples

### Example 1: Login Handler (Future)

```rust
async fn login_handler(
    State(pool): State<PgPool>,
    Json(credentials): Json<LoginRequest>,
) -> Result<(CookieJar, Json<User>), AppError> {
    // Find and verify user
    let user = find_user_by_email(&pool, &credentials.email)
        .await?
        .ok_or(AppError::InvalidCredentials)?;

    if !verify_password(&credentials.password, &user.password_hash)? {
        return Err(AppError::InvalidCredentials);
    }

    // Create session
    let session = create_session(&pool, user.id).await?;
    let cookie = create_session_cookie(&session.id);

    // Return user data with cookie
    let jar = CookieJar::new().add(cookie);
    Ok((jar, Json(user)))
}
```

### Example 2: Logout Handler (Future)

```rust
async fn logout_handler(
    State(pool): State<PgPool>,
    cookies: CookieJar,
) -> Result<CookieJar, AppError> {
    // Get session from cookie
    if let Some(session_id) = get_session_from_cookies(&cookies) {
        // Delete session from database
        delete_session(&pool, &session_id).await?;
    }

    // Clear cookie
    let cookie = create_clear_session_cookie();
    Ok(cookies.add(cookie))
}
```

### Example 3: Authentication Middleware (Future)

```rust
async fn require_auth(
    State(pool): State<PgPool>,
    cookies: CookieJar,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Extract session from cookie
    let session_id = get_session_from_cookies(&cookies)
        .ok_or(AppError::SessionExpired)?;

    // Validate session
    let session = get_session(&pool, &session_id)
        .await?
        .ok_or(AppError::SessionExpired)?;

    // Load user
    let user = find_user_by_id(&pool, session.user_id)
        .await?
        .ok_or(AppError::Internal("User not found".to_string()))?;

    // Attach user to request
    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}
```

## Integration with Step 6 (User Model)

Sessions reference users via `user_id`:

```rust
// Login: Find user, verify password, create session
let user = find_user_by_email(&pool, email).await?.ok_or(...)?;
if verify_password(password, &user.password_hash)? {
    let session = create_session(&pool, user.id).await?;
    // Set cookie
}

// Authentication: Get session, load user
let session = get_session(&pool, session_id).await?.ok_or(...)?;
let user = find_user_by_id(&pool, session.user_id).await?;
```

## Troubleshooting

### Issue: "Session not found"

**Causes:**
- Session was deleted (logout)
- Session token corrupted
- Database connection issue

**Solutions:**
- Check if session exists in database
- Verify cookie value matches database
- Clear cookies and re-login

### Issue: "Cookie not being set"

**Causes:**
- Missing HTTPS in production
- SameSite incompatibility
- Cookie blocked by browser

**Solutions:**
- Ensure HTTPS in production
- Check browser security settings
- Verify cookie settings in DevTools

### Issue: "Session creation fails"

**Causes:**
- Database connection issue
- user_id doesn't exist
- Table not created (migration not run)

**Solutions:**
- Check database connectivity
- Verify user exists
- Run migrations: `sqlx migrate run`

## Performance Considerations

### Session Creation

**Cost:** One database INSERT
- **Time:** ~1-5ms
- **Frequency:** Once per login

**Optimization:** None needed (infrequent operation)

### Session Validation

**Cost:** One database SELECT
- **Time:** ~1-5ms with index
- **Frequency:** Every authenticated request

**Optimization:**
- Primary key lookup is O(1)
- Consider caching for high-traffic (future)

### Session Deletion

**Cost:** One database DELETE
- **Time:** ~1-5ms
- **Frequency:** Once per logout

**Optimization:** None needed (infrequent operation)

### Bulk Session Deletion

**Cost:** DELETE with WHERE clause
- **Time:** ~5-20ms depending on session count
- **Frequency:** Rare (security action)

**Optimization:** Index on user_id makes this efficient

## Future Enhancements (Out of Scope)

- Session expiration (automatic timeout)
- Session refresh (extend lifetime)
- Remember me checkbox (longer-lived sessions)
- Session activity tracking
- IP address validation
- Concurrent session limits
- Session caching (Redis)
- Token rotation

## Summary

Session management implementation provides:

✅ **Secure token generation** - 256-bit random tokens
✅ **Database-backed sessions** - Stateful, server-side storage
✅ **Secure cookies** - HttpOnly, Secure, SameSite protection
✅ **Complete lifecycle** - Create, retrieve, delete operations
✅ **Multi-device support** - Multiple sessions per user
✅ **Bulk operations** - Logout from all devices
✅ **Comprehensive logging** - Security monitoring
✅ **Unit tests** - Token and cookie validation
✅ **Integration tests** - End-to-end verification

**Next Steps:**
- Proceed to Step 8 (Authentication API Endpoints)
- Session functions will be used in login/logout handlers
- Middleware will use `get_session()` for authentication

**Test Code Removal:**
The temporary test code in main.rs will be removed in Step 8 when we implement the authentication API.
