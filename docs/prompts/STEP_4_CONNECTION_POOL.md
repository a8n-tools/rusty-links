# Step 4: Database Connection Pool

This document describes the PostgreSQL connection pool implementation for Rusty Links.

## Implementation Overview

### Connection Pool Configuration

The database connection pool is implemented using SQLx's `PgPoolOptions` with optimized settings for a single-user application:

**Configuration Settings:**
- **Max Connections:** 5 (appropriate for single-user workload)
- **Connection Timeout:** 30 seconds (time to wait for an available connection)
- **Idle Timeout:** 10 minutes (idle connections are closed after 10 minutes of inactivity)

These settings balance resource usage with responsiveness for a self-hosted, single-user application.

### Implementation Details

#### `initialize_database` Function (src/main.rs:32-68)

A dedicated async function that handles all database initialization tasks:

```rust
async fn initialize_database(database_url: &str) -> Result<PgPool, Box<dyn std::error::Error>>
```

**Responsibilities:**
1. **Create Connection Pool** - Establishes PostgreSQL connection with optimized settings
2. **Run Migrations** - Automatically applies pending database migrations
3. **Health Check** - Verifies database connectivity with a simple query
4. **Error Handling** - Returns descriptive errors for troubleshooting

**Process Flow:**
```
1. Initialize connection pool with PgPoolOptions
   ├─ Set max_connections(5)
   ├─ Set acquire_timeout(30 seconds)
   └─ Set idle_timeout(10 minutes)

2. Run database migrations
   ├─ Execute sqlx::migrate!("./migrations")
   └─ Apply any pending SQL migration files

3. Perform health check
   ├─ Execute "SELECT 1" query
   └─ Verify connection is working

4. Return configured pool or error
```

#### Main Function Integration (src/main.rs:104-120)

The `main()` function calls `initialize_database()` and handles the result:

- **Success:** Logs "Database connected successfully" and stores pool for application use
- **Failure:** Logs detailed error, displays helpful troubleshooting steps, and exits with code 1

### Error Handling

Comprehensive error handling for common database issues:

**Connection Errors:**
- PostgreSQL not running
- Incorrect DATABASE_URL
- Database doesn't exist
- Network connectivity issues
- Authentication failures

**Migration Errors:**
- Syntax errors in SQL files
- Permission issues (CREATE, ALTER, etc.)
- Constraint violations in migrations
- Missing migration files

**Health Check Errors:**
- Database became unavailable after connection
- Permission issues for SELECT queries
- Network interruption

All errors are:
- Logged with structured tracing (JSON format)
- Displayed to user with actionable troubleshooting steps
- Result in graceful application shutdown with exit code 1

### Logging

Structured JSON logging at each stage:

```json
{"level":"INFO","message":"Initializing database connection pool..."}
{"level":"INFO","max_connections":5,"acquire_timeout_secs":30,"idle_timeout_secs":600,"message":"Database connection pool created"}
{"level":"INFO","message":"Running database migrations..."}
{"level":"INFO","message":"Database migrations completed successfully"}
{"level":"INFO","message":"Performing database health check..."}
{"level":"INFO","health_check_result":1,"message":"Database health check passed"}
{"level":"INFO","message":"Database initialized successfully"}
{"level":"INFO","message":"Database connected successfully"}
{"level":"INFO","message":"Database connection pool ready for application use"}
```

## Testing Instructions

### Prerequisites
1. PostgreSQL server running
2. Database created: `createdb rusty_links`
3. Valid `.env` file with DATABASE_URL

### Running the Application

```bash
cargo run
```

### Expected Output

Successful initialization should produce these log messages in order:

1. "Rusty Links starting..."
2. "Configuration loaded successfully"
3. "Initializing database connection pool..."
4. "Database connection pool created" (with settings)
5. "Running database migrations..."
6. "Database migrations completed successfully"
7. "Performing database health check..."
8. "Database health check passed" (result: 1)
9. "Database initialized successfully"
10. "Database connected successfully"
11. "Database connection pool ready for application use"
12. "Application initialization complete. Ready for Step 5."

### Testing Error Scenarios

#### Test 1: PostgreSQL Not Running
```bash
# Stop PostgreSQL (method varies by OS)
sudo systemctl stop postgresql  # Linux
brew services stop postgresql   # macOS

cargo run
```

**Expected:** Error message with troubleshooting steps, exit code 1

#### Test 2: Invalid DATABASE_URL
```bash
# Temporarily modify .env
DATABASE_URL=postgresql://invalid:invalid@localhost/nonexistent

cargo run
```

**Expected:** Connection error with actionable guidance

#### Test 3: Database Doesn't Exist
```bash
# Drop the database
dropdb rusty_links

cargo run
```

**Expected:** Error indicating database doesn't exist

#### Test 4: Insufficient Permissions
```sql
-- As superuser, revoke permissions from your user
REVOKE CREATE ON DATABASE rusty_links FROM your_user;
```

**Expected:** Migration error due to insufficient permissions

### Verifying Health Check

The health check executes a simple query to verify the connection works:

```sql
SELECT 1
```

You can verify the pool is working by checking the logs for:
```json
{"level":"INFO","health_check_result":1,"message":"Database health check passed"}
```

If the health check fails, it indicates the connection was established but queries cannot be executed.

## Connection Pool Lifecycle

### Pool Creation
- Occurs once at application startup
- Connections are created lazily (on first use)
- Initial connection validates DATABASE_URL is correct

### Connection Management
- **Acquire:** Requests wait up to 30 seconds for available connection
- **Release:** Connections returned to pool after query completion
- **Idle:** Connections idle for 10+ minutes are automatically closed
- **Max:** Pool never exceeds 5 concurrent connections

### Pool Shutdown
- Currently: Pool dropped at end of main() (application exits immediately)
- Future: Pool will be kept alive for the Axum web server
- Dropping the pool cleanly closes all connections

## Future Enhancements

The connection pool is ready for use in upcoming steps:

**Step 5: Error Handling Framework**
- Pool will be wrapped in shared state (Arc<PgPool>)
- Errors will use custom AppError types

**Step 6: User Model**
- Pool will be passed to database query functions
- User CRUD operations will use connections from pool

**Step 8: Authentication API**
- Pool will be available to Axum handlers via state
- Each request will acquire connection, execute query, release

**Step 11: Background Jobs**
- Pool will be shared with scheduler
- Metadata updates will use connections from same pool

## Connection Pool Best Practices

### Do's
✅ Pass `&PgPool` to functions that need database access
✅ Let SQLx manage connection acquisition/release automatically
✅ Use `?` operator to propagate errors from queries
✅ Share the pool across the application (Arc not needed, PgPool is already Arc internally)
✅ Trust the pool to handle connection reuse and cleanup

### Don'ts
❌ Don't manually manage individual connections unless necessary
❌ Don't create multiple pools (use one pool for the entire application)
❌ Don't hold connections longer than needed (avoid long-running transactions)
❌ Don't panic on query errors (use Result types and proper error handling)

## Troubleshooting

### Issue: "Connection pool timed out"
**Cause:** All 5 connections are in use and request waited > 30 seconds
**Solution:** Indicates a connection leak or slow queries. Review query performance.

### Issue: "Too many connections to database"
**Cause:** Multiple application instances or external connections consuming slots
**Solution:** Check max_connections in postgresql.conf and other clients

### Issue: "Connection refused"
**Cause:** PostgreSQL not running or wrong host/port
**Solution:** Verify PostgreSQL is running and DATABASE_URL is correct

### Issue: "Authentication failed"
**Cause:** Incorrect username/password in DATABASE_URL
**Solution:** Verify credentials and pg_hba.conf settings

## Summary

The database connection pool is now fully configured and ready for use:

✅ Optimized settings for single-user application
✅ Automatic migration execution
✅ Health check verification
✅ Comprehensive error handling
✅ Structured logging
✅ Well-documented configuration

**Next Step:** Proceed to Step 5 (Error Handling Framework) to create custom error types for the application.
