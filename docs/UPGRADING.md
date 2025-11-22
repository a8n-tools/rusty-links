# Upgrading Guide

This document provides instructions for upgrading Rusty Links between versions.

---

## Table of Contents

- [General Upgrade Process](#general-upgrade-process)
- [Version-Specific Notes](#version-specific-notes)
- [Rollback Procedure](#rollback-procedure)
- [Troubleshooting](#troubleshooting)

---

## General Upgrade Process

### Docker Deployment (Recommended)

The easiest way to upgrade is using Docker with the pre-built images:

```bash
cd /path/to/rustylinks

# 1. Backup your data first (IMPORTANT!)
docker-compose -f compose.prod.yml exec postgres \
  pg_dump -U rustylinks -Fc rustylinks > backup_$(date +%Y%m%d).dump

# 2. Pull the latest image
docker-compose -f compose.prod.yml pull app

# 3. Stop the current application
docker-compose -f compose.prod.yml down app

# 4. Start with new version
docker-compose -f compose.prod.yml up -d app

# 5. Check logs to verify successful start
docker-compose -f compose.prod.yml logs -f app

# 6. Verify application is working
curl http://localhost:8080/api/health
```

### Upgrading to a Specific Version

```bash
# Edit compose.prod.yml to specify version
nano compose.prod.yml

# Change:
# image: ghcr.io/YOUR-USERNAME/rusty-links:latest
# To:
# image: ghcr.io/YOUR-USERNAME/rusty-links:1.1.0

# Then follow the general upgrade process above
```

### Building from Source

If you're building from source:

```bash
cd /path/to/rustylinks

# 1. Backup your data
# (See backup section in DEPLOYMENT.md)

# 2. Pull latest code
git fetch --all
git checkout v1.1.0  # Or desired version tag

# 3. Rebuild
cargo build --release

# 4. Stop current application
# (Stop however you're running it - systemd, docker, etc.)

# 5. Run database migrations (automatic on startup)
# Migrations run automatically when the application starts

# 6. Start new version
./target/release/rusty-links
```

---

## Version-Specific Notes

### Upgrading to v1.0.0

This is the initial release. No upgrade path from previous versions.

**First-time Installation:**

1. Follow the [Quick Start Guide](../README.md#quick-start)
2. Create your first user via `/api/auth/setup`
3. Login and start adding links

---

<!--
Future version upgrade notes will be added here.

Example template for future versions:

### Upgrading to v1.1.0

**Release Date:** 2025-XX-XX

**Breaking Changes:**
- None

**New Features:**
- Feature 1
- Feature 2

**Database Migrations:**
- Migration 006: Description of migration

**Configuration Changes:**
- New environment variable: `NEW_VARIABLE` (optional, default: value)

**Upgrade Steps:**
1. Backup your database
2. Pull new image or update code
3. Update .env file with new variables (if needed)
4. Restart application
5. Verify with health check

**Post-Upgrade:**
- Check logs for any warnings
- Verify new features are working
- Test critical functionality

---

### Upgrading to v2.0.0

**Release Date:** 2025-XX-XX

**Breaking Changes:**
- Breaking change 1 (how to adapt)
- Breaking change 2 (how to adapt)

**Database Migrations:**
- Migration 010: Major schema changes

**Upgrade Steps:**
1. **IMPORTANT:** Read breaking changes carefully
2. Backup your database
3. Test upgrade in development environment first
4. Plan for potential downtime
5. Follow general upgrade process
6. Run post-upgrade verification

**Post-Upgrade Verification:**
- [ ] Application starts successfully
- [ ] Database migrations completed
- [ ] Existing links are accessible
- [ ] Search functionality works
- [ ] Authentication works
- [ ] Categories and tags preserved

-->

---

## Rollback Procedure

If you encounter issues after upgrading, you can rollback to the previous version.

### Docker Rollback

```bash
# 1. Stop the application
docker-compose -f compose.prod.yml down app

# 2. Edit compose.prod.yml to use previous version
nano compose.prod.yml

# Change image tag to previous version:
# image: ghcr.io/YOUR-USERNAME/rusty-links:1.0.0

# 3. Restore database backup (if needed)
docker-compose -f compose.prod.yml exec -T postgres \
  pg_restore -U rustylinks -d rustylinks -c < backup_YYYYMMDD.dump

# 4. Start with previous version
docker-compose -f compose.prod.yml up -d app

# 5. Verify rollback successful
curl http://localhost:8080/api/health
```

### Source Build Rollback

```bash
# 1. Checkout previous version
git checkout v1.0.0

# 2. Rebuild
cargo build --release

# 3. Restore database if needed
# (See DATABASE.md for restore procedures)

# 4. Start previous version
./target/release/rusty-links
```

---

## Database Migrations

Rusty Links uses SQLx for database migrations. Migrations run automatically on application startup.

### How Migrations Work

1. Application starts
2. Checks `_sqlx_migrations` table for applied migrations
3. Runs any new migrations in order
4. Logs migration results

### Manual Migration Management

If you need to manage migrations manually:

```bash
# Install SQLx CLI
cargo install sqlx-cli --no-default-features --features postgres

# Check migration status
sqlx migrate info --database-url "$DATABASE_URL"

# Run migrations manually
sqlx migrate run --database-url "$DATABASE_URL"

# Revert last migration (USE WITH CAUTION!)
sqlx migrate revert --database-url "$DATABASE_URL"
```

**Warning:** Never revert migrations in production unless you know exactly what you're doing!

---

## Backup Best Practices

**Always backup before upgrading!**

### Quick Backup

```bash
# Database backup
docker-compose -f compose.prod.yml exec postgres \
  pg_dump -U rustylinks -Fc rustylinks > backup_$(date +%Y%m%d_%H%M%S).dump

# Configuration backup
tar czf config_backup_$(date +%Y%m%d).tar.gz \
  .env compose.prod.yml /etc/nginx/sites-available/rustylinks
```

### Automated Backups

Set up automated backups with cron:

```bash
# Add to crontab
0 2 * * * /path/to/rustylinks/backup.sh >> /path/to/logs/backup.log 2>&1
```

See [DEPLOYMENT.md](DEPLOYMENT.md#backup-strategy) for complete backup procedures.

---

## Troubleshooting

### Common Upgrade Issues

#### Application Won't Start

```bash
# Check logs
docker-compose -f compose.prod.yml logs app

# Common causes:
# 1. Database migration failed
# 2. Configuration error
# 3. Port conflict
# 4. Database not ready
```

#### Database Migration Failed

```bash
# Check migration status
sqlx migrate info --database-url "$DATABASE_URL"

# View migration history
docker-compose -f compose.prod.yml exec postgres \
  psql -U rustylinks -d rustylinks -c "SELECT * FROM _sqlx_migrations;"

# If migration is stuck, check logs for specific error
docker-compose -f compose.prod.yml logs app | grep -i migration
```

#### Configuration Issues

```bash
# Verify environment variables
docker-compose -f compose.prod.yml exec app env | grep -E "DATABASE_URL|APP_PORT|RUST_LOG"

# Compare with .env.example for new variables
diff .env .env.example
```

#### Performance Degradation After Upgrade

```bash
# Check resource usage
docker stats

# Check database indexes
docker-compose -f compose.prod.yml exec postgres \
  psql -U rustylinks -d rustylinks -c "\di"

# Analyze query performance
# (See DATABASE.md for performance tuning)
```

### Getting Help

If you encounter issues during upgrade:

1. Check the logs: `docker-compose logs -f app`
2. Review [TROUBLESHOOTING.md](TROUBLESHOOTING.md) (if available)
3. Check [GitHub Issues](https://github.com/YOUR-USERNAME/rusty-links/issues)
4. Create a new issue with:
   - Version upgrading from/to
   - Error logs
   - Steps to reproduce
   - Configuration (sanitized, no passwords!)

---

## Compatibility Matrix

| Rusty Links | PostgreSQL | Rust | Docker |
|-------------|------------|------|--------|
| 1.0.0       | 16+        | 1.75+ | 24.0+ |

<!-- Future versions will be added here -->

---

## Upgrade Checklist

Before upgrading, ensure you have:

- [ ] Read the version-specific upgrade notes
- [ ] Reviewed breaking changes (if any)
- [ ] Created a database backup
- [ ] Backed up configuration files
- [ ] Tested upgrade in development environment
- [ ] Planned for potential downtime
- [ ] Notified users (if multi-user in future)

After upgrading, verify:

- [ ] Application starts without errors
- [ ] Health endpoint returns 200 OK
- [ ] Database migrations completed successfully
- [ ] Can login with existing credentials
- [ ] Existing links are accessible
- [ ] Search functionality works
- [ ] New features are working (if any)

---

## Additional Resources

- [CHANGELOG.md](../CHANGELOG.md) - Version history and release notes
- [RELEASE.md](RELEASE.md) - Release process documentation
- [DEPLOYMENT.md](DEPLOYMENT.md) - Deployment guide
- [DATABASE.md](DATABASE.md) - Database documentation
- [SECURITY.md](SECURITY.md) - Security considerations

---

**Last Updated:** 2025-01-XX
**Current Version:** 1.0.0

For the latest upgrade information, always refer to the [GitHub Releases](https://github.com/YOUR-USERNAME/rusty-links/releases) page.
