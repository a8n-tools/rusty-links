# Security Documentation

Comprehensive security guide for Rusty Links deployment and maintenance.

## Table of Contents

- [Overview](#overview)
- [Implemented Security Features](#implemented-security-features)
- [Production Hardening Checklist](#production-hardening-checklist)
- [Reverse Proxy Configuration](#reverse-proxy-configuration)
- [Vulnerability Reporting](#vulnerability-reporting)
- [Regular Maintenance](#regular-maintenance)
- [Best Practices](#best-practices)
- [Security Audit](#security-audit)

---

## Overview

Rusty Links implements security best practices for a self-hosted bookmark management application. This document outlines implemented security features, hardening procedures, and maintenance tasks.

### Security Philosophy

- **Defense in Depth** - Multiple layers of security
- **Least Privilege** - Minimal permissions required
- **Security by Default** - Secure configurations out of the box
- **Regular Updates** - Timely security patches
- **Transparency** - Clear vulnerability disclosure process

---

## Implemented Security Features

### Authentication & Authorization

✅ **Argon2 Password Hashing**
- Industry-standard, memory-hard hashing algorithm
- Configurable memory cost, time cost, and parallelism
- Resistant to GPU and ASIC attacks
- Salt included automatically

```rust
// Implementation in src/models/user.rs
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
```

✅ **Session-based Authentication**
- Secure random session tokens (32 bytes / 64 hex characters)
- Database-backed sessions (not JWT)
- HttpOnly cookies prevent XSS attacks
- Secure flag ensures HTTPS-only transmission
- SameSite=Lax prevents CSRF attacks

```rust
// Cookie configuration in src/auth/session.rs
Cookie::build((SESSION_COOKIE_NAME, session_id))
    .http_only(true)   // JavaScript cannot access
    .secure(true)      // HTTPS only
    .same_site(SameSite::Lax)  // CSRF protection
    .path("/")
    .into()
```

✅ **Single-user Mode** (Phase 1)
- Simplified attack surface
- First user created via /api/auth/setup
- Setup endpoint disabled after first user
- Reduces multi-tenant security concerns

✅ **CSRF Protection**
- SameSite cookie attribute
- State-changing operations use POST/PUT/DELETE
- Stateful sessions (not vulnerable to token replay)

### Input Validation

✅ **URL Validation**
- Proper URL parsing with `url` crate
- Domain extraction and sanitization
- Path normalization
- Protocol enforcement (HTTP/HTTPS only)

✅ **Email Validation**
- Format checking
- Unique constraint in database
- Case-insensitive comparison

✅ **SQL Injection Prevention**
- Parameterized queries via SQLx
- Compile-time query verification
- No string concatenation in queries
- Prepared statements for all database operations

```rust
// Safe parameterized query
sqlx::query("SELECT * FROM users WHERE email = $1")
    .bind(email)
    .fetch_one(&pool)
    .await
```

✅ **XSS Prevention**
- Content Security Policy headers (via reverse proxy)
- Output encoding in Dioxus components
- No raw HTML injection
- Sanitized user input before storage

✅ **Path Traversal Prevention**
- No file system access from user input
- Assets served from fixed directories
- No dynamic file includes

### Data Protection

✅ **Password Requirements**
- Minimum 8 characters (configurable)
- Hashed with Argon2 before storage
- Never logged or exposed in responses
- Password hash excluded from User serialization

✅ **Secure Session Storage**
- Sessions stored in PostgreSQL database
- Session tokens never in logs
- Automatic cleanup on logout
- No session fixation vulnerabilities

✅ **No Sensitive Data Logging**
- Passwords never logged
- Session tokens never logged
- Masked database URLs in logs
- Structured logging with tracing crate

```rust
// Masked database URL for logging
pub fn masked_database_url(&self) -> String {
    // Masks password in connection string
}
```

✅ **Database Encryption at Rest** (via PostgreSQL configuration)
- Enable with `ssl=on` in postgresql.conf
- TLS for connections in production
- Encrypted backups recommended

### Network Security

✅ **HTTPS Enforcement** (via reverse proxy)
- TLS 1.2+ required
- Modern cipher suites
- HTTP Strict Transport Security (HSTS)
- Redirect HTTP to HTTPS

✅ **CORS Configuration**
- Configured via tower-http
- Restrictive origins in production
- Credentials allowed for same-origin only

### Dependency Security

✅ **Regular Dependency Audits**
- CI/CD security audit with cargo-audit
- Automated vulnerability scanning
- Dependency updates via Dependabot
- Minimal dependency footprint

✅ **Supply Chain Security**
- Cargo.lock committed to repository
- Reproducible builds
- No deprecated dependencies
- Trusted crate sources only

### Container Security

✅ **Non-root User**
- Container runs as user `rustylinks` (UID 1000)
- No privilege escalation
- Minimal capabilities

✅ **Minimal Runtime Image**
- debian:bookworm-slim base
- Only essential runtime dependencies
- No build tools in final image
- Multi-stage builds for size reduction

✅ **Read-only Root Filesystem** (optional)
- Application doesn't write to filesystem
- Temporary storage in /tmp if needed
- Logs to stdout/stderr

---

## Production Hardening Checklist

### Before Deployment

#### Application Configuration

- [ ] Generate strong DATABASE_URL password
- [ ] Use long, random database password (32+ characters)
- [ ] Set APP_PORT appropriately (usually 8080 internally)
- [ ] Configure RUST_LOG for production (warn or error)
- [ ] Disable debug features
- [ ] Review .env file permissions (chmod 600)

#### Database Security

- [ ] Create dedicated database user for application
- [ ] Grant minimum required permissions
- [ ] Enable PostgreSQL SSL/TLS
- [ ] Configure pg_hba.conf for restricted access
- [ ] Set strong postgres user password
- [ ] Enable statement logging for auditing
- [ ] Configure connection limits
- [ ] Enable automatic backups

#### Docker Security

- [ ] Use official base images
- [ ] Scan images for vulnerabilities
- [ ] Set resource limits (CPU, memory)
- [ ] Use Docker secrets for sensitive data
- [ ] Enable Docker content trust
- [ ] Run containers with `--read-only` flag (if possible)
- [ ] Limit container capabilities
- [ ] Use user namespace remapping

#### Reverse Proxy

- [ ] Configure Nginx/Caddy (see examples)
- [ ] Enable HTTPS with valid TLS certificate
- [ ] Set up automatic certificate renewal (Let's Encrypt)
- [ ] Configure HSTS headers
- [ ] Set security headers (CSP, X-Frame-Options, etc.)
- [ ] Enable rate limiting
- [ ] Configure request size limits
- [ ] Set up access logging

#### Firewall

- [ ] Allow only necessary ports (80, 443)
- [ ] Block direct access to database port
- [ ] Block direct access to application port
- [ ] Configure fail2ban for repeated failed attempts
- [ ] Enable UFW or iptables rules

#### Monitoring

- [ ] Set up log aggregation
- [ ] Configure alerts for errors
- [ ] Monitor disk usage
- [ ] Monitor CPU/memory usage
- [ ] Set up uptime monitoring
- [ ] Configure database query monitoring

### Firewall Configuration

#### UFW (Ubuntu)

```bash
# Reset firewall
sudo ufw reset

# Default policies
sudo ufw default deny incoming
sudo ufw default allow outgoing

# Allow SSH (change port if needed)
sudo ufw allow 22/tcp

# Allow HTTP and HTTPS
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp

# Enable firewall
sudo ufw enable

# Check status
sudo ufw status verbose
```

#### iptables

```bash
# Flush existing rules
iptables -F

# Default policies
iptables -P INPUT DROP
iptables -P FORWARD DROP
iptables -P OUTPUT ACCEPT

# Allow loopback
iptables -A INPUT -i lo -j ACCEPT

# Allow established connections
iptables -A INPUT -m conntrack --ctstate ESTABLISHED,RELATED -j ACCEPT

# Allow SSH
iptables -A INPUT -p tcp --dport 22 -j ACCEPT

# Allow HTTP and HTTPS
iptables -A INPUT -p tcp --dport 80 -j ACCEPT
iptables -A INPUT -p tcp --dport 443 -j ACCEPT

# Save rules
iptables-save > /etc/iptables/rules.v4
```

### Docker Security Configuration

#### Resource Limits

```yaml
# compose.yml
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

#### Security Options

```yaml
services:
  app:
    security_opt:
      - no-new-privileges:true
    cap_drop:
      - ALL
    cap_add:
      - NET_BIND_SERVICE  # Only if binding to ports < 1024
```

#### Read-only Filesystem

```yaml
services:
  app:
    read_only: true
    tmpfs:
      - /tmp
      - /var/run
```

---

## Reverse Proxy Configuration

### Why Use a Reverse Proxy?

1. **TLS Termination** - Handle HTTPS certificates
2. **Security Headers** - Add protective HTTP headers
3. **Rate Limiting** - Prevent abuse
4. **Static File Serving** - Offload from application
5. **Load Balancing** - Distribute traffic (future)
6. **DDoS Protection** - Filter malicious traffic

### Nginx Configuration

See `examples/nginx.conf` for complete configuration.

**Quick Setup:**

```bash
# Install Nginx
sudo apt install nginx

# Copy configuration
sudo cp examples/nginx.conf /etc/nginx/sites-available/rustylinks
sudo ln -s /etc/nginx/sites-available/rustylinks /etc/nginx/sites-enabled/

# Test configuration
sudo nginx -t

# Reload Nginx
sudo systemctl reload nginx
```

**Key Features:**
- HTTPS with Let's Encrypt
- Security headers (HSTS, CSP, X-Frame-Options)
- Rate limiting (60 requests/minute)
- Request size limits (10MB)
- Gzip compression
- Access logging

### Caddy Configuration

See `examples/Caddyfile` for complete configuration.

**Quick Setup:**

```bash
# Install Caddy
sudo apt install -y debian-keyring debian-archive-keyring apt-transport-https
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | sudo tee /etc/apt/sources.list.d/caddy-stable.list
sudo apt update
sudo apt install caddy

# Copy configuration
sudo cp examples/Caddyfile /etc/caddy/Caddyfile

# Reload Caddy
sudo systemctl reload caddy
```

**Key Features:**
- Automatic HTTPS with Let's Encrypt
- Automatic certificate renewal
- Security headers
- Simpler configuration than Nginx
- Built-in rate limiting

---

## Vulnerability Reporting

### Responsible Disclosure

If you discover a security vulnerability in Rusty Links:

#### Do NOT:
- ❌ Open a public GitHub issue
- ❌ Discuss publicly on social media
- ❌ Exploit the vulnerability

#### DO:
1. ✅ Email security report to project maintainers
2. ✅ Provide detailed information:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact (CVSS score if applicable)
   - Affected versions
   - Suggested fix (optional)
   - Your contact information

#### Response Timeline

- **Initial Response**: Within 48 hours
- **Assessment**: Within 1 week
- **Fix Development**: 1-4 weeks (depending on severity)
- **Public Disclosure**: After fix is released

#### Severity Levels

| Level | Response Time | Example |
|-------|---------------|---------|
| **Critical** | 24-48 hours | Remote code execution, SQL injection |
| **High** | 1 week | Authentication bypass, XSS |
| **Medium** | 2-4 weeks | CSRF, information disclosure |
| **Low** | As time permits | Minor info leaks, UI issues |

### Security Advisories

Security advisories are published on GitHub Security Advisories page after fixes are released.

---

## Regular Maintenance

### Weekly Tasks

- [ ] Review application logs for errors
- [ ] Check disk space usage
- [ ] Verify backups completed successfully
- [ ] Review failed login attempts

### Monthly Tasks

- [ ] Update dependencies (`cargo update`)
- [ ] Run security audit (`cargo audit`)
- [ ] Review access logs for anomalies
- [ ] Test backup restoration
- [ ] Review and rotate logs
- [ ] Update system packages

### Quarterly Tasks

- [ ] Review and update security configurations
- [ ] Penetration testing (if applicable)
- [ ] Review user access (when multi-user)
- [ ] Update TLS certificates (if not automated)
- [ ] Review firewall rules
- [ ] Conduct security training

### Yearly Tasks

- [ ] Comprehensive security audit
- [ ] Review disaster recovery procedures
- [ ] Update security documentation
- [ ] Review and update password policy
- [ ] Evaluate new security features
- [ ] Review third-party integrations

### Security Update Schedule

#### Application Updates

```bash
# Check for Rust updates
rustup update

# Update dependencies
cargo update

# Check for vulnerabilities
cargo audit

# Review changes
git log --oneline --since="1 week ago"

# Test updates
cargo test

# Deploy
docker compose build
docker compose up -d
```

#### System Updates

```bash
# Ubuntu/Debian
sudo apt update
sudo apt upgrade

# Reboot if kernel updated
sudo reboot

# Check running services
systemctl status rustylinks
```

---

## Best Practices

### Password Policy

**For Administrators:**
- Minimum 16 characters
- Mix of uppercase, lowercase, numbers, symbols
- Unique password (not reused)
- Use password manager
- Rotate every 90 days
- Enable 2FA on GitHub/email

**For Application:**
- Minimum 8 characters enforced
- Consider bcrypt or Argon2 settings adjustment
- No password reset via email (single-user)
- Session timeout after 30 days inactivity (future feature)

### Network Access

**Recommended Setup:**
- Deploy behind VPN or private network
- Use firewall to restrict access by IP
- Do NOT expose directly to internet without reverse proxy
- Consider using Cloudflare Tunnel or Tailscale

**Cloudflare Setup:**
```bash
# Install cloudflared
wget https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb
sudo dpkg -i cloudflared-linux-amd64.deb

# Authenticate
cloudflared tunnel login

# Create tunnel
cloudflared tunnel create rustylinks

# Configure tunnel
cloudflared tunnel route dns rustylinks links.yourdomain.com

# Run tunnel
cloudflared tunnel run rustylinks
```

### Backup Policy

**Backup Strategy:**
- **Frequency**: Daily automated backups
- **Retention**: 30 days rolling
- **Location**: Off-site storage (S3, Backblaze B2)
- **Encryption**: Encrypt backups at rest
- **Testing**: Test restore monthly

**Backup Script:**
```bash
#!/bin/bash
# /usr/local/bin/backup-rustylinks.sh

BACKUP_DIR="/backups/rustylinks"
DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="$BACKUP_DIR/rustylinks_$DATE.dump"

# Create backup
docker compose exec -T postgres pg_dump -U rustylinks -Fc rustylinks > "$BACKUP_FILE"

# Encrypt backup
gpg --encrypt --recipient you@yourdomain.com "$BACKUP_FILE"
rm "$BACKUP_FILE"

# Upload to S3
aws s3 cp "$BACKUP_FILE.gpg" s3://your-bucket/rustylinks/

# Delete local encrypted backup after upload
rm "$BACKUP_FILE.gpg"

# Keep last 30 days
find $BACKUP_DIR -name "rustylinks_*.dump.gpg" -mtime +30 -delete
```

### Update Policy

**Severity-based Updates:**

| Severity | Update Window | Testing |
|----------|---------------|---------|
| Critical | Immediate (< 24 hours) | Minimal testing |
| High | 1 week | Basic testing |
| Medium | Next maintenance window | Full testing |
| Low | Quarterly | Full testing |

**Update Process:**
1. Read changelog and release notes
2. Backup database and configuration
3. Test in staging environment (if available)
4. Apply update during low-traffic period
5. Monitor logs for issues
6. Rollback if problems occur

### Monitoring Setup

**Essential Metrics:**
- Application uptime
- HTTP response codes (4xx, 5xx)
- Database connection pool usage
- Disk space usage
- Memory usage
- CPU usage

**Recommended Tools:**
- **Prometheus + Grafana** - Metrics and dashboards
- **Loki** - Log aggregation
- **Uptime Kuma** - Uptime monitoring
- **Netdata** - Real-time system monitoring

---

## Security Audit

### Running Security Audits

#### Rust Dependencies

```bash
# Install cargo-audit
cargo install cargo-audit

# Run audit
cargo audit

# Check for specific advisory
cargo audit --deny warnings
```

#### Docker Image Scanning

```bash
# Using Docker Scout
docker scout cves rusty-links:latest

# Using Trivy
docker run --rm -v /var/run/docker.sock:/var/run/docker.sock \
  aquasec/trivy image rusty-links:latest

# Using Snyk
snyk container test rusty-links:latest
```

#### Dependency Updates

```bash
# Install cargo-outdated
cargo install cargo-outdated

# Check outdated dependencies
cargo outdated

# Update dependencies
cargo update

# Check for unused dependencies
cargo install cargo-udeps
cargo +nightly udeps
```

### Penetration Testing

**Recommended Tests:**

1. **Authentication Testing**
   - Brute force protection
   - Session fixation
   - Session hijacking
   - Password policy enforcement

2. **Input Validation**
   - SQL injection attempts
   - XSS attempts
   - Command injection
   - Path traversal

3. **Business Logic**
   - Authorization bypass
   - Race conditions
   - Privilege escalation

4. **Infrastructure**
   - TLS configuration
   - HTTP header security
   - Cookie security
   - CORS configuration

**Tools:**
- OWASP ZAP
- Burp Suite
- Nikto
- SQLmap (for testing, never for attack)

---

## Security Headers

The reverse proxy should set these headers:

```nginx
# Security headers (Nginx)
add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
add_header X-Frame-Options "SAMEORIGIN" always;
add_header X-Content-Type-Options "nosniff" always;
add_header X-XSS-Protection "1; mode=block" always;
add_header Referrer-Policy "strict-origin-when-cross-origin" always;
add_header Content-Security-Policy "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'" always;
add_header Permissions-Policy "geolocation=(), microphone=(), camera=()" always;
```

**Header Explanations:**

- **HSTS**: Forces HTTPS for 1 year
- **X-Frame-Options**: Prevents clickjacking
- **X-Content-Type-Options**: Prevents MIME-type sniffing
- **X-XSS-Protection**: Enables browser XSS filter
- **Referrer-Policy**: Controls referrer information
- **CSP**: Restricts resource loading
- **Permissions-Policy**: Disables unnecessary browser features

---

## Incident Response

### Security Incident Procedure

1. **Detection**
   - Monitor logs for suspicious activity
   - Review failed login attempts
   - Check for unusual database queries

2. **Containment**
   - Block malicious IPs
   - Disable compromised accounts
   - Isolate affected systems

3. **Investigation**
   - Collect logs and evidence
   - Determine scope of breach
   - Identify attack vector

4. **Eradication**
   - Remove malicious code
   - Patch vulnerabilities
   - Update credentials

5. **Recovery**
   - Restore from clean backup
   - Verify system integrity
   - Monitor for re-infection

6. **Lessons Learned**
   - Document incident
   - Update security procedures
   - Implement preventive measures

---

## Compliance

### Data Protection

- **GDPR** (if applicable): User data is minimal (email, password hash)
- **Data Retention**: Configure retention policies
- **Data Export**: Supported via export functionality
- **Data Deletion**: Cascade deletes on user deletion

### Logging Compliance

- **PII Handling**: No passwords or sensitive data in logs
- **Log Retention**: Configure based on requirements
- **Access Logs**: Track who accessed what data
- **Audit Trails**: Database audit logging enabled

---

## References

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [OWASP Cheat Sheets](https://cheatsheetseries.owasp.org/)
- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)
- [Docker Security Best Practices](https://docs.docker.com/engine/security/)
- [PostgreSQL Security](https://www.postgresql.org/docs/current/security.html)

---

## Support

For security-related questions:
- Review this documentation
- Check [DEPLOYMENT.md](DEPLOYMENT.md) for deployment guidance
- See [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for common issues
- Contact maintainers for sensitive security issues
