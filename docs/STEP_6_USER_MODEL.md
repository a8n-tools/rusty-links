# Step 6: User Model and Database Operations

This document describes the User model implementation with secure password handling.

## Overview

The User model provides:
- **Secure Password Storage**: Argon2id hashing with OWASP-recommended parameters
- **Email Validation**: Format validation before user creation
- **Case-Insensitive Lookup**: Email queries ignore case
- **Password Verification**: Constant-time comparison to prevent timing attacks
- **User Existence Check**: Determine if setup is required

## Implementation

### 1. Dependencies Added (Cargo.toml)

```toml
# Password hashing
argon2 = "0.5"

# Already present:
uuid = { version = "1.6", features = ["serde", "v4"] }
chrono = { version = "0.4", features = ["serde"] }
```

### 2. User Model (src/models/user.rs)

#### Data Structures

**User Entity:**
```rust
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,  // Never sent to frontend
    pub created_at: DateTime<Utc>,
}
```

**CreateUser Request:**
```rust
pub struct CreateUser {
    pub email: String,
    pub password: String,  // Plain text, will be hashed
}
```

#### Database Operations

**1. create_user(pool, create_user) -> Result<User, AppError>**

Creates a new user with secure password hashing.

**Process:**
1. Validates email format (contains @, has domain with dot)
2. Hashes password using Argon2id
3. Inserts into database
4. Handles duplicate email (unique constraint violation)
5. Returns created user

**Errors:**
- `AppError::Validation` - Invalid email format
- `AppError::Duplicate` - Email already exists
- `AppError::Internal` - Password hashing failed
- `AppError::Database` - Database operation failed

**Security:**
- Password never logged
- Email logged for audit trail
- Argon2id with OWASP parameters

**Example:**
```rust
let user = create_user(
    &pool,
    CreateUser {
        email: "user@example.com".to_string(),
        password: "secure_password".to_string(),
    }
).await?;

println!("User created: {}", user.id);
```

**2. find_user_by_email(pool, email) -> Result<Option<User>, AppError>**

Finds a user by email address (case-insensitive).

**Process:**
1. Queries database with `LOWER(email) = LOWER($1)`
2. Returns `Some(User)` if found, `None` if not found

**Errors:**
- `AppError::Database` - Database operation failed

**Example:**
```rust
match find_user_by_email(&pool, "USER@EXAMPLE.COM").await? {
    Some(user) => println!("Found: {}", user.email),
    None => println!("Not found"),
}
```

**3. verify_password(password, hash) -> Result<bool, AppError>**

Verifies a plain-text password against an Argon2 hash.

**Process:**
1. Parses the stored hash
2. Uses Argon2 to verify password
3. Returns true if match, false if not

**Errors:**
- `AppError::Internal` - Hash parsing or verification failed

**Security:**
- Constant-time comparison (prevents timing attacks)
- Never logs password or hash
- Uses Argon2's built-in verification

**Example:**
```rust
if verify_password("user_input", &user.password_hash)? {
    println!("Password correct");
} else {
    println!("Password incorrect");
}
```

**4. check_user_exists(pool) -> Result<bool, AppError>**

Checks if any user exists in the database.

**Process:**
1. Runs `SELECT EXISTS(SELECT 1 FROM users LIMIT 1)`
2. Returns boolean result

**Use Case:**
- Determine if initial setup is needed
- Show setup page vs. login page

**Errors:**
- `AppError::Database` - Database operation failed

**Example:**
```rust
if !check_user_exists(&pool).await? {
    println!("Setup required - no users exist");
} else {
    println!("Application already configured");
}
```

### 3. Password Security

#### Argon2id Configuration

**Algorithm:** Argon2id (hybrid of Argon2i and Argon2d)
- Resistant to both side-channel and GPU attacks
- Recommended by OWASP for password storage

**Parameters (Argon2 defaults):**
- Memory cost: 19456 KiB (19 MiB)
- Time cost: 2 iterations
- Parallelism: 1 thread
- Output length: 32 bytes
- Salt: 128-bit random (generated with OsRng)

**Hash Format:**
```
$argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>
```

**Why These Parameters?**
- Based on OWASP recommendations
- Balances security and performance
- Sufficient for single-user application
- Can be increased if needed

#### Security Properties

✅ **Random Salt** - Each hash uses unique salt (prevents rainbow tables)
✅ **Constant-Time Verification** - Prevents timing attacks
✅ **Memory-Hard** - Resistant to GPU/ASIC attacks
✅ **Password Never Logged** - Only email is logged
✅ **No Plain-Text Storage** - Passwords hashed immediately

### 4. Email Validation

**Validation Rules:**
1. Must contain exactly one @ symbol
2. Must have content before @
3. Must have domain after @
4. Domain must contain at least one dot

**Examples:**

Valid emails:
- `user@example.com`
- `test.user@example.co.uk`
- `user+tag@example.com`

Invalid emails:
- `userexample.com` (no @)
- `@example.com` (empty local part)
- `user@` (empty domain)
- `user@example` (domain without dot)

**Error Messages:**
```rust
AppError::validation("email", "Email must contain @ symbol")
AppError::validation("email", "Email must have content before @")
AppError::validation("email", "Email must have a domain after @")
AppError::validation("email", "Email domain must contain a dot")
```

### 5. Module Structure

**src/models/mod.rs:**
```rust
pub mod user;

// Re-export commonly used types
pub use user::{
    check_user_exists,
    create_user,
    find_user_by_email,
    verify_password,
    CreateUser,
    User
};
```

**Usage:**
```rust
use crate::models::{create_user, find_user_by_email, CreateUser};
```

### 6. Error Handling

All operations use `AppError` for consistent error handling:

**Email Validation:**
```rust
AppError::Validation {
    field: "email",
    message: "Email must contain @ symbol"
}
```

**Duplicate Email:**
```rust
AppError::Duplicate {
    field: "email"
}
// Automatically detected from unique constraint violation
```

**Password Hashing:**
```rust
AppError::Internal("Failed to hash password: <details>")
// Password not included in error message
```

**Database Errors:**
```rust
AppError::Database(sqlx::Error)
// Automatic conversion via From trait
```

### 7. Logging

**User Creation:**
```json
{"level":"INFO","email":"user@example.com","message":"Creating new user"}
{"level":"INFO","user_id":"550e8400-...","email":"user@example.com","message":"User created successfully"}
```

**Password Verification:**
```json
{"level":"DEBUG","message":"Verifying password"}
{"level":"DEBUG","message":"Password verification successful"}
```

**User Lookup:**
```json
{"level":"DEBUG","email":"user@example.com","message":"Looking up user by email"}
{"level":"DEBUG","email":"user@example.com","message":"User found"}
```

**Security Notes:**
- Passwords are NEVER logged
- Email addresses are logged (not sensitive)
- User IDs are logged for audit trail
- Success/failure is logged for security monitoring

## Testing

### Unit Tests (src/models/user.rs)

The module includes comprehensive unit tests:

```bash
# Run user model tests
cargo test --lib models::user

# Run all tests
cargo test
```

**Test Coverage:**
- ✅ Valid email validation
- ✅ Invalid email rejection
- ✅ Password hashing
- ✅ Password verification (correct password)
- ✅ Password verification (incorrect password)
- ✅ Hash uniqueness (different salts)

### Integration Tests (main.rs)

Temporary test code added to main.rs verifies:

1. **User Existence Check**
   - Checks if any users exist
   - Returns boolean result

2. **User Creation**
   - Creates test user
   - Validates email
   - Hashes password
   - Stores in database

3. **Duplicate Detection**
   - Attempts to create duplicate user
   - Verifies duplicate error is returned

4. **Case-Insensitive Lookup**
   - Finds user with different case (`TEST@...` finds `test@...`)
   - Verifies user data returned

5. **Password Verification**
   - Verifies correct password returns true
   - Verifies incorrect password returns false

6. **Non-Existent User**
   - Lookup of non-existent email returns None

7. **Email Validation**
   - Invalid email format rejected
   - Validation error returned

### Running Tests

```bash
# Ensure database is running and migrations applied
cargo run
```

**Expected Output:**
```
✓ Users exist in database / No users exist - fresh database
✓ User created successfully
  - ID: 550e8400-e29b-41d4-a716-446655440000
  - Email: test@rustylinks.local
  - Created at: 2025-01-20 10:30:45 UTC
  - Password hash length: 97
✓ User found by email (case-insensitive)
  - Found user: test@rustylinks.local
✓ Password verification successful
✓ Incorrect password correctly rejected
✓ Non-existent user correctly returned None
✓ Email validation correctly rejected invalid email
  - Field: email
  - Message: Email must contain @ symbol

=== All User Model Tests Complete ===
Step 6 implementation verified successfully!
```

## Database Schema

The users table was created in Step 3:

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users(email);
```

**Constraints:**
- Primary key on `id`
- Unique constraint on `email`
- Index on `email` for fast lookups

**Case-Insensitive Query:**
```sql
SELECT * FROM users WHERE LOWER(email) = LOWER($1)
```

This works efficiently with the index since we're comparing the lowercased values.

## Usage Examples

### Example 1: User Registration

```rust
async fn register_user(pool: &PgPool, email: String, password: String) -> Result<User, AppError> {
    // Email validation and password hashing handled automatically
    let user = create_user(
        pool,
        CreateUser { email, password }
    ).await?;

    tracing::info!(user_id = %user.id, "New user registered");
    Ok(user)
}
```

### Example 2: User Login

```rust
async fn authenticate_user(
    pool: &PgPool,
    email: String,
    password: String
) -> Result<User, AppError> {
    // Find user (case-insensitive)
    let user = find_user_by_email(pool, &email)
        .await?
        .ok_or(AppError::InvalidCredentials)?;

    // Verify password
    if !verify_password(&password, &user.password_hash)? {
        return Err(AppError::InvalidCredentials);
    }

    tracing::info!(user_id = %user.id, "User authenticated successfully");
    Ok(user)
}
```

### Example 3: Setup Check

```rust
async fn is_setup_required(pool: &PgPool) -> Result<bool, AppError> {
    // Returns true if no users exist
    Ok(!check_user_exists(pool).await?)
}
```

## Security Best Practices

### Implemented

✅ **Argon2id Hashing** - Industry-standard password hashing
✅ **Random Salts** - Each hash uses unique salt
✅ **OWASP Parameters** - Recommended memory/time costs
✅ **Constant-Time Comparison** - Prevents timing attacks
✅ **No Plain-Text Logging** - Passwords never logged
✅ **Automatic Error Conversion** - Database errors handled consistently
✅ **Email Validation** - Basic format validation
✅ **Case-Insensitive Lookup** - Prevents duplicate emails with different cases

### Future Enhancements (Out of Scope for Phase 1)

- Email verification (send confirmation email)
- Password strength requirements
- Password reset functionality
- Account lockout after failed attempts
- Two-factor authentication
- Session management (Step 7)

## API Integration (Future)

In Step 8, these functions will be used by API endpoints:

**POST /api/auth/setup**
```rust
async fn setup_handler(
    State(pool): State<PgPool>,
    Json(data): Json<CreateUser>,
) -> Result<Json<User>, AppError> {
    // Check if setup already completed
    if check_user_exists(&pool).await? {
        return Err(AppError::Unauthorized);
    }

    // Create first user
    let user = create_user(&pool, data).await?;
    Ok(Json(user))
}
```

**POST /api/auth/login**
```rust
async fn login_handler(
    State(pool): State<PgPool>,
    Json(credentials): Json<LoginRequest>,
) -> Result<Json<User>, AppError> {
    // Find user
    let user = find_user_by_email(&pool, &credentials.email)
        .await?
        .ok_or(AppError::InvalidCredentials)?;

    // Verify password
    if !verify_password(&credentials.password, &user.password_hash)? {
        return Err(AppError::InvalidCredentials);
    }

    Ok(Json(user))
}
```

## Troubleshooting

### Issue: "Failed to hash password"

**Cause:** Argon2 library error (rare)

**Solution:** Check system entropy, ensure OsRng is available

### Issue: "Failed to parse password hash"

**Cause:** Corrupted hash in database or wrong format

**Solution:** Verify hash format starts with `$argon2id$`

### Issue: "Email already exists"

**Cause:** Duplicate email in database

**Solution:** This is expected behavior - handle with appropriate error message

### Issue: "Email validation failed"

**Cause:** Email doesn't match validation rules

**Solution:** Ensure email has format `local@domain.tld`

## Performance Considerations

### Password Hashing

Argon2 is intentionally slow (for security):
- **Hash time:** ~100-300ms on typical hardware
- **Verify time:** ~100-300ms on typical hardware

This is acceptable for:
- User registration (infrequent)
- User login (once per session)

### Database Queries

Optimized with indexes:
- Email lookup: O(log n) with index
- User existence check: O(1) with LIMIT 1

### Memory Usage

Argon2 memory cost:
- 19 MiB per hash operation
- Released after operation completes
- Not an issue for single-user application

## Summary

The User model implementation provides:

✅ Secure password storage with Argon2id
✅ Email validation
✅ Case-insensitive email lookup
✅ Password verification with timing attack prevention
✅ User existence checking
✅ Comprehensive error handling
✅ Detailed logging (security-conscious)
✅ Unit and integration tests
✅ Well-documented API

**Next Steps:**
- Proceed to Step 7 (Session Management)
- Session creation will use `find_user_by_email` and `verify_password`
- Sessions will reference `user.id`

**Test Code Removal:**
The temporary test code in main.rs will be removed in Step 7 when we implement session management.
