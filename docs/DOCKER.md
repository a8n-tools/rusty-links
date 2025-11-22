# Docker Deployment Guide

This guide covers deploying Rusty Links using Docker and Docker Compose for both development and production environments.

## Quick Start

### 1. Copy Environment Template

```bash
cp .env.example .env
```

### 2. Configure Environment Variables

Edit `.env` and set secure passwords and configuration:

```bash
# At minimum, change the database password
DB_PASSWORD=your_secure_password_here

# Adjust other settings as needed
RUST_LOG=info
UPDATE_INTERVAL_DAYS=30
```

### 3. Start Services

```bash
docker compose up -d
```

This will:
- Start PostgreSQL database
- Build the application Docker image
- Run database migrations automatically
- Start the application server

### 4. View Logs

```bash
# Follow all logs
docker compose logs -f

# Follow application logs only
docker compose logs -f app

# Follow database logs only
docker compose logs -f postgres
```

### 5. Verify Services

```bash
# Check service status
docker compose ps

# Test application health
curl http://localhost:8080/api/health
```

### 6. Stop Services

```bash
# Stop services (preserves data)
docker compose down

# Stop and remove volumes (deletes all data)
docker compose down -v
```

## Development Mode

For local development with hot reloading:

### 1. Start Development Services

```bash
docker compose -f compose.yml -f compose.dev.yml up
```

This will:
- Mount source code as volumes
- Use `cargo watch` for auto-reloading
- Enable debug logging
- Preserve build cache in a volume

### 2. Make Code Changes

Any changes to `src/`, `assets/`, or `Cargo.toml` will trigger automatic rebuild and restart.

### 3. Stop Development Services

```bash
docker compose -f compose.yml -f compose.dev.yml down
```

## Production Deployment

### 1. Build Production Image

```bash
docker compose build --no-cache
```

### 2. Set Production Environment Variables

Edit `.env` for production:

```bash
DB_PASSWORD=very_secure_production_password
RUST_LOG=warn
UPDATE_INTERVAL_DAYS=30
UPDATE_INTERVAL_HOURS=24
```

### 3. Start Production Services

```bash
docker compose up -d
```

### 4. Monitor Services

```bash
# Check container health
docker compose ps

# View recent logs
docker compose logs --tail=100 app

# Monitor resource usage
docker stats rusty-links-app rusty-links-db
```

## Database Operations

### Access Database Shell

```bash
docker compose exec postgres psql -U rustylinks rustylinks
```

### Backup Database

```bash
# Create backup
docker compose exec postgres pg_dump -U rustylinks rustylinks > backup_$(date +%Y%m%d_%H%M%S).sql

# Or use pg_dump with compression
docker compose exec postgres pg_dump -U rustylinks -Fc rustylinks > backup_$(date +%Y%m%d_%H%M%S).dump
```

### Restore Database

```bash
# From SQL file
docker compose exec -T postgres psql -U rustylinks rustylinks < backup.sql

# From compressed dump
docker compose exec -T postgres pg_restore -U rustylinks -d rustylinks < backup.dump
```

### View Database Logs

```bash
docker compose logs postgres
```

### Reset Database

```bash
# Stop services
docker compose down

# Remove database volume
docker volume rm rusty-links_postgres_data

# Start services (will create fresh database)
docker compose up -d
```

## Migrations

Database migrations run automatically when the application starts. You can also run them manually:

```bash
# Migrations are run by the application on startup
# Check application logs to verify migration status
docker compose logs app | grep migration
```

## Troubleshooting

### Container Won't Start

```bash
# Check logs for errors
docker compose logs app

# Check if database is healthy
docker compose ps

# Restart services
docker compose restart
```

### Database Connection Issues

```bash
# Verify database is running
docker compose ps postgres

# Check database health
docker compose exec postgres pg_isready -U rustylinks

# Verify connection string in app logs
docker compose logs app | grep DATABASE_URL
```

### Build Failures

```bash
# Clean build without cache
docker compose build --no-cache

# Remove old images
docker image prune -a

# Rebuild from scratch
docker compose down
docker compose build --no-cache
docker compose up -d
```

### Port Already in Use

If port 8080 is already in use:

```bash
# Change host port in .env
echo "HOST_PORT=8081" >> .env

# Restart services
docker compose down
docker compose up -d
```

### Out of Disk Space

```bash
# Clean up unused containers, images, and volumes
docker system prune -a --volumes

# Check disk usage
docker system df
```

## Image Size Optimization

The production image uses multi-stage builds and is optimized for size:

```bash
# Check image size
docker images rusty-links

# Expected size: < 150MB
```

## Security Considerations

### Non-Root User

The application runs as a non-root user (`rustylinks`, UID 1000):

```bash
# Verify user
docker compose exec app id
# Output: uid=1000(rustylinks) gid=1000(rustylinks)
```

### Secure Passwords

Always use strong passwords in production:

```bash
# Generate secure password
openssl rand -base64 32

# Update .env file
DB_PASSWORD=<generated_password>
```

### Network Isolation

Services communicate via a dedicated bridge network (`rusty-links-network`), isolated from other containers.

### Minimal Runtime

The production image uses `debian:bookworm-slim` with only essential runtime dependencies.

## Performance Tuning

### Database Connection Pool

Adjust PostgreSQL settings if needed by creating a custom `postgres.conf`:

```bash
# Create custom config
cat > postgres.conf <<EOF
max_connections = 100
shared_buffers = 256MB
effective_cache_size = 1GB
EOF

# Mount it in compose.yml
# volumes:
#   - ./postgres.conf:/etc/postgresql/postgresql.conf
```

### Application Resources

Limit container resources in `compose.yml`:

```yaml
services:
  app:
    deploy:
      resources:
        limits:
          cpus: '1'
          memory: 512M
        reservations:
          cpus: '0.5'
          memory: 256M
```

## Monitoring

### Health Checks

Both services have health checks configured:

```bash
# View health status
docker compose ps

# Manual health check
curl http://localhost:8080/api/health
```

### Logs Management

```bash
# Limit log size in compose.yml
services:
  app:
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
```

## Advanced Configuration

### Custom Port Mapping

```bash
# In .env file
HOST_PORT=3000

# Application will be available on http://localhost:3000
```

### Environment-Specific Compose Files

```bash
# Production
docker compose -f compose.yml up -d

# Development
docker compose -f compose.yml -f compose.dev.yml up

# Staging (create compose.staging.yml)
docker compose -f compose.yml -f compose.staging.yml up -d
```

## Cleanup

### Remove All Services and Data

```bash
# Stop and remove containers, networks, volumes
docker compose down -v

# Remove images
docker rmi rusty-links:latest

# Clean up build cache
docker builder prune -a
```

## Support

For issues or questions:
- Check application logs: `docker compose logs app`
- Check database logs: `docker compose logs postgres`
- Review this documentation
- Check the main README.md for project details
