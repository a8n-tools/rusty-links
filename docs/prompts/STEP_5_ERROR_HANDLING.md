# Step 5: Error Handling Framework

This document describes the comprehensive error handling framework for Rusty Links.

## Overview

The error handling framework provides:
- **Unified Error Type**: Single `AppError` type used throughout the application
- **Automatic Conversions**: From traits for common error types (sqlx, io, serde_json)
- **HTTP Status Mapping**: Each error variant maps to appropriate HTTP status code
- **User-Friendly Messages**: Clear, actionable error messages for frontend
- **Detailed Logging**: Full error context preserved for debugging
- **API Response Format**: Consistent JSON error format for all endpoints

## Architecture

### Core Components

#### 1. `AppError` Enum (src/error.rs)

The main application error type with variants for all error categories:

```rust
pub enum AppError {
    Database(sqlx::Error),           // Database operations
    Configuration(String),           // Configuration loading
    Validation { field, message },   // Input validation
    InvalidCredentials,              // Login failures
    SessionExpired,                  // Expired sessions
    Unauthorized,                    // Access control
    NotFound { resource, id },       // Missing resources
    Duplicate { field },             // Unique constraint violations
    ExternalService(String),         // External API errors
    Io(std::io::Error),             // File/network I/O
    Json(serde_json::Error),        // JSON parsing
    Internal(String),               // Unexpected errors
}
```

#### 2. `ApiErrorResponse` Struct

Standardized JSON error response format:

```json
{
    "error": "User-friendly error message",
    "code": "ERROR_CODE",
    "status": 400
}
```

**Fields:**
- `error` (string): Human-readable message suitable for display
- `code` (string): Machine-readable error code for client-side handling
- `status` (u16): HTTP status code

### Error Code Mapping

| Error Variant | HTTP Status | Error Code | Description |
|--------------|-------------|------------|-------------|
| `Validation` | 400 Bad Request | VALIDATION_ERROR | Invalid input data |
| `InvalidCredentials` | 401 Unauthorized | INVALID_CREDENTIALS | Wrong email/password |
| `SessionExpired` | 401 Unauthorized | SESSION_EXPIRED | Session no longer valid |
| `Unauthorized` | 403 Forbidden | UNAUTHORIZED | Insufficient permissions |
| `NotFound` | 404 Not Found | NOT_FOUND | Resource doesn't exist |
| `Duplicate` | 409 Conflict | DUPLICATE_FIELD | Unique constraint violation |
| `Database` | 500 Internal Server Error | DATABASE_ERROR | Database operation failed |
| `Io` | 500 Internal Server Error | IO_ERROR | File/network I/O error |
| `Json` | 500 Internal Server Error | JSON_ERROR | JSON parse error |
| `Internal` | 500 Internal Server Error | INTERNAL_ERROR | Unexpected error |
| `ExternalService` | 502 Bad Gateway | EXTERNAL_SERVICE_ERROR | External API failed |
| `Configuration` | 503 Service Unavailable | CONFIGURATION_ERROR | Invalid configuration |

## Helper Functions

### Creating Errors

Convenient constructors for common error patterns:

```rust
// Validation error
let error = AppError::validation("email", "Email must contain @ symbol");

// Not found error
let error = AppError::not_found("link", "550e8400-e29b-41d4-a716-446655440000");

// Duplicate error
let error = AppError::duplicate("email");
```

### Converting to Response

```rust
let error = AppError::validation("password", "Password too short");
let response = error.to_response();
// ApiErrorResponse {
//     error: "password: Password too short",
//     code: "VALIDATION_ERROR",
//     status: 400
// }
```

### Getting Status Code

```rust
let error = AppError::not_found("user", "123");
let status = error.status_code(); // 404
let code = error.error_code();     // "NOT_FOUND"
```

## Automatic Conversions

The framework implements `From<T>` traits for automatic error conversion:

### From `sqlx::Error`

```rust
async fn get_user(pool: &PgPool, id: &str) -> Result<User, AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await?; // Automatically converts sqlx::Error to AppError
    Ok(user)
}
```

**Special Handling:**
- `sqlx::Error::RowNotFound` → `AppError::NotFound`
- Unique constraint violations → `AppError::Duplicate` (with field name extraction)
- Other database errors → `AppError::Database`

### From `std::io::Error`

```rust
use std::fs;

fn read_config_file(path: &str) -> Result<String, AppError> {
    let contents = fs::read_to_string(path)?; // Auto-converts to AppError::Io
    Ok(contents)
}
```

### From `serde_json::Error`

```rust
fn parse_json(data: &str) -> Result<Value, AppError> {
    let value: Value = serde_json::from_str(data)?; // Auto-converts to AppError::Json
    Ok(value)
}
```

## Error Messages

### User-Friendly Messages

The `Display` trait provides messages suitable for end users:

```rust
let error = AppError::validation("email", "Invalid format");
println!("{}", error); // "email: Invalid format"

let error = AppError::not_found("link", "123");
println!("{}", error); // "Link not found."

let error = AppError::duplicate("email");
println!("{}", error); // "Email already exists."
```

**Design Principles:**
- Clear and actionable
- No internal implementation details
- No sensitive information (error chains, stack traces)
- Proper capitalization and formatting

### Debug Output

The `Debug` trait provides detailed information for logging:

```rust
let error = AppError::Database(sqlx_error);
tracing::error!("{:?}", error); // Full error chain with source
```

## Integration with Existing Code

### Updated `config.rs`

Configuration loading now uses `AppError`:

```rust
pub fn from_env() -> Result<Self, AppError> {
    let database_url = std::env::var("DATABASE_URL").map_err(|_| {
        AppError::Configuration("Missing required environment variable: DATABASE_URL".to_string())
    })?;
    // ...
}
```

### Updated `main.rs`

Database initialization uses `AppError`:

```rust
async fn initialize_database(database_url: &str) -> Result<PgPool, AppError> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?; // sqlx::Error automatically converted to AppError

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}
```

Error handling in `main()`:

```rust
let config = match config::Config::from_env() {
    Ok(cfg) => cfg,
    Err(e) => {
        tracing::error!(error = %e, "Failed to load configuration");
        eprintln!("Configuration error: {}", e);
        std::process::exit(1);
    }
};
```

## Usage Examples

### Example 1: Validation Error

```rust
fn validate_email(email: &str) -> Result<(), AppError> {
    if !email.contains('@') {
        return Err(AppError::validation("email", "Email must contain @ symbol"));
    }
    Ok(())
}

// Usage
match validate_email("invalid") {
    Ok(_) => println!("Valid"),
    Err(e) => println!("Error: {}", e), // "email: Email must contain @ symbol"
}
```

### Example 2: Database Query with Auto-Conversion

```rust
async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<User, AppError> {
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE email = $1"
    )
    .bind(email)
    .fetch_one(pool)
    .await?; // RowNotFound → AppError::NotFound automatically

    Ok(user)
}
```

### Example 3: Handling Duplicate Constraint

```rust
async fn create_user(pool: &PgPool, email: &str) -> Result<User, AppError> {
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (email, password_hash) VALUES ($1, $2) RETURNING *"
    )
    .bind(email)
    .bind("hash")
    .fetch_one(pool)
    .await?; // Unique violation → AppError::Duplicate { field: "email" }

    Ok(user)
}

// Usage
match create_user(&pool, "existing@example.com").await {
    Ok(user) => println!("Created: {:?}", user),
    Err(AppError::Duplicate { field }) => {
        println!("Duplicate {}: already exists", field)
    }
    Err(e) => println!("Error: {}", e),
}
```

### Example 4: API Error Response

```rust
// Future use in API handlers
async fn login_handler() -> Result<Json<User>, AppError> {
    let user = find_user("email@example.com").await?;

    if !verify_password("password", &user.password_hash)? {
        return Err(AppError::InvalidCredentials);
    }

    Ok(Json(user))
}

// The error will be converted to JSON:
// {
//     "error": "Invalid email or password.",
//     "code": "INVALID_CREDENTIALS",
//     "status": 401
// }
```

## Testing

### Unit Tests

The error module includes unit tests for core functionality:

```bash
cargo test --lib error
```

**Test Coverage:**
- Validation error creation and formatting
- Not found error creation and formatting
- Duplicate error creation and formatting
- Status code mapping
- Error code generation
- API response conversion
- Helper functions (capitalize_first)

### Running Tests

```bash
# Run all tests
cargo test

# Run error module tests only
cargo test --lib error::tests

# Run with output
cargo test -- --nocapture
```

### Example Test

```rust
#[test]
fn test_validation_error() {
    let error = AppError::validation("email", "Invalid email format");
    assert_eq!(error.status_code(), 400);
    assert_eq!(error.error_code(), "VALIDATION_ERROR");
    assert!(error.to_string().contains("email"));
    assert!(error.to_string().contains("Invalid email format"));
}
```

## Future API Integration

When implementing API handlers (Step 8), errors will be automatically converted:

```rust
use axum::{response::{IntoResponse, Response}, Json};
use http::StatusCode;

// Implement IntoResponse for AppError
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status_code()).unwrap();
        let response = self.to_response();
        (status, Json(response)).into_response()
    }
}

// API handler example
async fn create_link(
    State(pool): State<PgPool>,
    Json(data): Json<CreateLinkData>,
) -> Result<Json<Link>, AppError> {
    // Validation errors automatically converted
    validate_url(&data.url)?;

    // Database errors automatically converted
    let link = insert_link(&pool, data).await?;

    Ok(Json(link))
}
```

## Best Practices

### Do's

✅ **Use `?` operator** - Leverage automatic conversions
```rust
let user = query_user(&pool, id).await?;
```

✅ **Use helper functions** - For common patterns
```rust
return Err(AppError::validation("email", "Invalid format"));
```

✅ **Preserve context** - Include relevant information
```rust
AppError::not_found("link", &link_id.to_string())
```

✅ **Log errors** - Use tracing for debugging
```rust
tracing::error!(error = %e, user_id = %user_id, "Failed to create link");
```

### Don'ts

❌ **Don't expose sensitive data** - Keep internal details out of user messages
```rust
// Bad: Exposes database structure
AppError::Internal("Failed to insert into users table: column 'foo' doesn't exist")

// Good: Generic message for users, detailed log for debugging
tracing::error!("Database schema mismatch: {}", db_error);
AppError::Internal("An internal error occurred".to_string())
```

❌ **Don't use panic!** - Return errors instead
```rust
// Bad
if user.is_none() {
    panic!("User not found");
}

// Good
user.ok_or_else(|| AppError::not_found("user", id))?
```

❌ **Don't lose error context** - Chain errors properly
```rust
// Bad: Loses original error
.map_err(|_| AppError::Internal("Parse failed".to_string()))

// Good: Preserves error through automatic conversion
.map_err(|e| AppError::Internal(format!("Parse failed: {}", e)))
```

## Error Handling Strategy

### Layers of Error Handling

1. **Application Layer** (main.rs)
   - Catches startup errors (config, database)
   - Logs errors with tracing
   - Displays user-friendly messages to console
   - Exits with appropriate code

2. **Business Logic Layer** (models, services)
   - Returns `Result<T, AppError>`
   - Uses `?` for automatic conversion
   - Adds context with helper functions
   - Logs important errors

3. **API Layer** (handlers) - Future
   - Converts AppError to HTTP responses
   - Returns appropriate status codes
   - Sends JSON error responses
   - Logs request/error correlation

### Error Flow Example

```
User submits login form
    ↓
API Handler validates input
    ↓
Business logic checks credentials
    ↓
Database query executes
    ↓
sqlx::Error::RowNotFound
    ↓
From<sqlx::Error> converts to AppError::NotFound
    ↓
Returns to business logic as AppError
    ↓
Returns to API handler as AppError
    ↓
IntoResponse converts to HTTP response
    ↓
Frontend receives JSON error
```

## Troubleshooting

### Issue: "Cannot convert error to AppError"

**Cause:** Error type doesn't implement `From<ErrorType> for AppError`

**Solution:** Add a `From` implementation or manually convert:
```rust
.map_err(|e| AppError::Internal(format!("Failed: {}", e)))?
```

### Issue: "Error message exposes sensitive data"

**Cause:** Using `Debug` output or database error messages directly

**Solution:** Use `Internal` variant with generic message for users:
```rust
tracing::error!("Sensitive detail: {:?}", error);
AppError::Internal("An error occurred".to_string())
```

## Summary

The error handling framework provides:

✅ Comprehensive error type covering all application errors
✅ Automatic conversions from common error types
✅ HTTP status code mapping
✅ User-friendly error messages
✅ Detailed logging capabilities
✅ Consistent API response format
✅ Helper functions for common patterns
✅ Integration with config and database layers
✅ Unit tests for core functionality

**Next Steps:**
- Proceed to Step 6 (User Model) to implement database operations with AppError
- AppError will be used throughout all future implementations
