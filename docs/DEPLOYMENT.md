# Production Deployment Guide

Complete guide for deploying Rusty Links to production with security hardening and best practices.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Server Setup](#server-setup)
- [Docker Deployment](#docker-deployment)
- [Reverse Proxy Setup](#reverse-proxy-setup)
- [SSL/TLS Configuration](#ssltls-configuration)
- [Database Setup](#database-setup)
- [Application Configuration](#application-configuration)
- [Monitoring](#monitoring)
- [Backup Strategy](#backup-strategy)
- [Post-Deployment](#post-deployment)

---

## Prerequisites

### Minimum Requirements

- **OS**: Ubuntu 22.04 LTS (recommended) or Debian 12
- **RAM**: 1GB minimum, 2GB recommended
- **Disk**: 10GB minimum, 20GB+ recommended
- **CPU**: 1 vCPU minimum, 2+ recommended
- **Network**: Public IP address or domain name

### Required Software

- Docker 24.0+
- Docker Compose 2.20+
- Nginx 1.24+ or Caddy 2.7+
- PostgreSQL 16 (via Docker) or installed locally
- Certbot (for Let's Encrypt with Nginx)

---

## Server Setup

### 1. Update System

```bash
# Update package lists
sudo apt update

# Upgrade installed packages
sudo apt upgrade -y

# Install required tools
sudo apt install -y \
    curl \
    git \
    ufw \
    fail2ban \
    unattended-upgrades
```

### 2. Configure Firewall

```bash
# Reset firewall to default
sudo ufw --force reset

# Set default policies
sudo ufw default deny incoming
sudo ufw default allow outgoing

# Allow SSH (IMPORTANT: Do this before enabling!)
sudo ufw allow 22/tcp

# Allow HTTP and HTTPS
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp

# Enable firewall
sudo ufw enable

# Check status
sudo ufw status verbose
```

### 3. Create Application User

```bash
# Create dedicated user for running application
sudo useradd -r -m -s /bin/bash rustylinks

# Add user to docker group (if using Docker)
sudo usermod -aG docker rustylinks

# Switch to rustylinks user
sudo su - rustylinks
```

### 4. Set Up Directory Structure

```bash
# Create application directory
mkdir -p ~/rustylinks
mkdir -p ~/rustylinks/backups
mkdir -p ~/rustylinks/logs

# Set permissions
chmod 755 ~/rustylinks
chmod 700 ~/rustylinks/backups
```

---

## Docker Deployment

### 1. Install Docker

```bash
# Install Docker
curl -fsSL https://get.docker.com | sh

# Install Docker Compose
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose

# Verify installation
docker --version
docker-compose --version
```

### 2. Clone Repository

```bash
cd ~/rustylinks

# Clone from GitHub
git clone https://github.com/YOUR-USERNAME/rusty-links.git .

# Or pull pre-built image (recommended for production)
# Configuration shown in next steps
```

### 3. Configure Environment

```bash
# Copy environment template
cp .env.example .env

# Generate strong password
openssl rand -base64 32

# Edit .env file
nano .env
```

**Required .env settings:**

```bash
# Database password (use generated password)
DB_PASSWORD=YOUR_STRONG_PASSWORD_HERE

# Application port (internal, don't expose publicly)
APP_PORT=8080

# Update schedule
UPDATE_INTERVAL_DAYS=30
UPDATE_INTERVAL_HOURS=24

# Logging
RUST_LOG=warn  # or error for production

# Optional: GitHub token for higher API rate limits
# GITHUB_TOKEN=ghp_your_token_here
```

**Secure the .env file:**

```bash
chmod 600 .env
```

### 4. Create docker-compose.yml for Production

```bash
nano compose.prod.yml
```

```yaml
version: '3.8'

services:
  postgres:
    image: postgres:16-alpine
    container_name: rustylinks-db
    restart: unless-stopped
    environment:
      POSTGRES_USER: rustylinks
      POSTGRES_PASSWORD: ${DB_PASSWORD}
      POSTGRES_DB: rustylinks
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U rustylinks"]
      interval: 10s
      timeout: 5s
      retries: 5
    networks:
      - rustylinks-network
    # Security: No exposed ports (only accessible via network)

  app:
    image: ghcr.io/YOUR-USERNAME/rusty-links:latest
    container_name: rustylinks-app
    restart: unless-stopped
    depends_on:
      postgres:
        condition: service_healthy
    environment:
      DATABASE_URL: postgres://rustylinks:${DB_PASSWORD}@postgres:5432/rustylinks
      APP_PORT: 8080
      UPDATE_INTERVAL_DAYS: ${UPDATE_INTERVAL_DAYS:-30}
      UPDATE_INTERVAL_HOURS: ${UPDATE_INTERVAL_HOURS:-24}
      BATCH_SIZE: ${BATCH_SIZE:-50}
      JITTER_PERCENT: ${JITTER_PERCENT:-20}
      RUST_LOG: ${RUST_LOG:-warn}
    networks:
      - rustylinks-network
    ports:
      - "127.0.0.1:8080:8080"  # Bind to localhost only
    deploy:
      resources:
        limits:
          cpus: '1'
          memory: 512M
        reservations:
          cpus: '0.5'
          memory: 256M
    security_opt:
      - no-new-privileges:true

volumes:
  postgres_data:
    driver: local

networks:
  rustylinks-network:
    driver: bridge
```

### 5. Start Services

```bash
# Pull latest images
docker-compose -f compose.prod.yml pull

# Start services
docker-compose -f compose.prod.yml up -d

# Check logs
docker-compose -f compose.prod.yml logs -f app

# Verify services are running
docker-compose -f compose.prod.yml ps
```

---

## Reverse Proxy Setup

### Option A: Nginx

#### 1. Install Nginx

```bash
sudo apt install -y nginx
```

#### 2. Copy Configuration

```bash
# Copy example config
sudo cp examples/nginx.conf /etc/nginx/sites-available/rustylinks

# Update domain name in config
sudo sed -i 's/links.yourdomain.com/YOUR_DOMAIN/g' /etc/nginx/sites-available/rustylinks

# Enable site
sudo ln -s /etc/nginx/sites-available/rustylinks /etc/nginx/sites-enabled/

# Remove default site
sudo rm /etc/nginx/sites-enabled/default

# Test configuration
sudo nginx -t

# Reload Nginx
sudo systemctl reload nginx
```

#### 3. Install Certbot for Let's Encrypt

```bash
# Install Certbot
sudo apt install -y certbot python3-certbot-nginx

# Obtain certificate
sudo certbot --nginx -d YOUR_DOMAIN

# Test automatic renewal
sudo certbot renew --dry-run
```

### Option B: Caddy (Easier Setup)

#### 1. Install Caddy

```bash
sudo apt install -y debian-keyring debian-archive-keyring apt-transport-https
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | sudo tee /etc/apt/sources.list.d/caddy-stable.list
sudo apt update
sudo apt install -y caddy
```

#### 2. Configure Caddy

```bash
# Copy example config
sudo cp examples/Caddyfile /etc/caddy/Caddyfile

# Update domain name
sudo sed -i 's/links.yourdomain.com/YOUR_DOMAIN/g' /etc/caddy/Caddyfile

# Test configuration
sudo caddy validate --config /etc/caddy/Caddyfile

# Reload Caddy (automatically obtains SSL certificate)
sudo systemctl reload caddy
```

---

## SSL/TLS Configuration

### Nginx with Let's Encrypt

Let's Encrypt certificates are automatically renewed by Certbot.

**Check certificate expiration:**

```bash
sudo certbot certificates
```

**Manual renewal (if needed):**

```bash
sudo certbot renew
```

### Caddy with Let's Encrypt

Caddy automatically obtains and renews certificates. No manual intervention required!

**Check certificate info:**

```bash
sudo caddy list-certificates
```

### Custom SSL Certificate

If using a custom certificate:

**Nginx:**

```nginx
ssl_certificate /path/to/your/fullchain.pem;
ssl_certificate_key /path/to/your/privkey.pem;
```

**Caddy:**

```caddy
your-domain.com {
    tls /path/to/cert.pem /path/to/key.pem
    reverse_proxy localhost:8080
}
```

---

## Database Setup

### PostgreSQL Configuration

The database runs in Docker with recommended settings.

#### Backup Configuration

```bash
# Create backup script
cat > ~/rustylinks/backup.sh <<'EOF'
#!/bin/bash

BACKUP_DIR="/home/rustylinks/rustylinks/backups"
DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="$BACKUP_DIR/rustylinks_$DATE.dump"

# Create backup
docker-compose -f /home/rustylinks/rustylinks/compose.prod.yml exec -T postgres \
  pg_dump -U rustylinks -Fc rustylinks > "$BACKUP_FILE"

# Compress and encrypt (optional)
gpg --encrypt --recipient your-email@example.com "$BACKUP_FILE"
rm "$BACKUP_FILE"

# Keep only last 30 backups
cd "$BACKUP_DIR"
ls -t rustylinks_*.dump.gpg | tail -n +31 | xargs rm -f

echo "Backup completed: $BACKUP_FILE.gpg"
EOF

chmod +x ~/rustylinks/backup.sh
```

#### Automated Backups

```bash
# Add to crontab
crontab -e

# Add line (daily backup at 2 AM)
0 2 * * * /home/rustylinks/rustylinks/backup.sh >> /home/rustylinks/rustylinks/logs/backup.log 2>&1
```

---

## Application Configuration

### Environment Variables

All configuration in `.env` file:

```bash
# Required
DB_PASSWORD=strong_password_here
APP_PORT=8080

# Optional but recommended
RUST_LOG=warn
UPDATE_INTERVAL_DAYS=30
UPDATE_INTERVAL_HOURS=24

# Optional: GitHub integration
GITHUB_TOKEN=ghp_your_token_here
```

### Updating the Application

```bash
cd ~/rustylinks

# Pull latest image
docker-compose -f compose.prod.yml pull app

# Restart application
docker-compose -f compose.prod.yml up -d app

# Check logs
docker-compose -f compose.prod.yml logs -f app
```

### Rolling Back

```bash
# Pull specific version
docker-compose -f compose.prod.yml pull app:v1.0.0

# Restart with specific version
docker-compose -f compose.prod.yml up -d app
```

---

## Monitoring

### Log Monitoring

```bash
# Application logs
docker-compose -f compose.prod.yml logs -f app

# Database logs
docker-compose -f compose.prod.yml logs -f postgres

# Nginx logs
sudo tail -f /var/log/nginx/rustylinks_access.log
sudo tail -f /var/log/nginx/rustylinks_error.log

# Caddy logs
sudo journalctl -u caddy -f
```

### Resource Monitoring

```bash
# Docker stats
docker stats

# System resources
htop

# Disk usage
df -h
du -sh ~/rustylinks/*
```

### Uptime Monitoring

**Recommended Tools:**

- [Uptime Kuma](https://github.com/louislam/uptime-kuma) - Self-hosted
- [UptimeRobot](https://uptimerobot.com/) - Free tier available
- [Healthchecks.io](https://healthchecks.io/) - Simple ping monitoring

**Setup example (Uptime Kuma):**

```bash
docker run -d --restart=always -p 3001:3001 -v uptime-kuma:/app/data --name uptime-kuma louislam/uptime-kuma:1

# Access at http://localhost:3001
# Add monitor for: https://YOUR_DOMAIN/api/health
```

---

## Backup Strategy

### Automated Backup Setup

1. **Database Backups** (Daily)
   - Automated via cron (see Database Setup section)
   - Stored in `/home/rustylinks/rustylinks/backups`
   - Encrypted with GPG
   - Retained for 30 days

2. **Configuration Backups**

```bash
# Create config backup script
cat > ~/rustylinks/backup-config.sh <<'EOF'
#!/bin/bash

BACKUP_DIR="/home/rustylinks/rustylinks/backups/config"
DATE=$(date +%Y%m%d_%H%M%S)

mkdir -p "$BACKUP_DIR"

# Backup important configs
tar czf "$BACKUP_DIR/config_$DATE.tar.gz" \
    ~/rustylinks/.env \
    ~/rustylinks/compose.prod.yml \
    /etc/nginx/sites-available/rustylinks

# Keep last 10 config backups
cd "$BACKUP_DIR"
ls -t config_*.tar.gz | tail -n +11 | xargs rm -f
EOF

chmod +x ~/rustylinks/backup-config.sh
```

3. **Off-site Backup** (S3/Backblaze B2)

```bash
# Install AWS CLI
sudo apt install -y awscli

# Configure AWS credentials
aws configure

# Add to backup script
aws s3 cp "$BACKUP_FILE.gpg" s3://your-bucket/rustylinks/
```

### Restore Procedure

```bash
# Stop application
docker-compose -f compose.prod.yml down

# Restore database
gpg --decrypt backup.dump.gpg | \
  docker-compose -f compose.prod.yml exec -T postgres \
    pg_restore -U rustylinks -d rustylinks

# Start application
docker-compose -f compose.prod.yml up -d

# Verify
curl https://YOUR_DOMAIN/api/health
```

---

## Post-Deployment

### Verification Checklist

- [ ] Application accessible via HTTPS
- [ ] HTTP redirects to HTTPS
- [ ] SSL certificate valid (A+ rating on [SSL Labs](https://www.ssllabs.com/ssltest/))
- [ ] Security headers present (check with [Security Headers](https://securityheaders.com/))
- [ ] Database backups running
- [ ] Monitoring set up
- [ ] Logs being written
- [ ] /api/health returns 200 OK
- [ ] Can create account and login
- [ ] Can create and manage links

### Security Audit

```bash
# Check for vulnerabilities
docker run --rm -v /var/run/docker.sock:/var/run/docker.sock \
  aquasec/trivy image rusty-links:latest

# Test security headers
curl -I https://YOUR_DOMAIN

# Check firewall rules
sudo ufw status verbose

# Review open ports
sudo netstat -tulpn
```

### Performance Testing

```bash
# Install wrk
sudo apt install -y wrk

# Test application performance
wrk -t4 -c100 -d30s https://YOUR_DOMAIN/api/health

# Results should show:
# - No errors
# - Reasonable latency (<100ms p99)
# - Good throughput
```

### Initial Setup

1. **Access Application**
   ```bash
   # Open in browser
   open https://YOUR_DOMAIN
   ```

2. **Create First User**
   ```bash
   # Via API
   curl -X POST https://YOUR_DOMAIN/api/auth/setup \
     -H "Content-Type: application/json" \
     -d '{"email":"admin@yourdomain.com","password":"StrongPassword123!"}'

   # Or via web interface
   ```

3. **Login and Test**
   - Create a few links
   - Test search functionality
   - Verify metadata extraction works
   - Test GitHub repository links

---

## Troubleshooting

### Application Won't Start

```bash
# Check logs
docker-compose -f compose.prod.yml logs app

# Common issues:
# 1. Database not ready
docker-compose -f compose.prod.yml logs postgres

# 2. Environment variables not set
docker-compose -f compose.prod.yml exec app env

# 3. Port already in use
sudo lsof -i :8080
```

### SSL Certificate Issues

```bash
# Nginx - Check certificate
sudo certbot certificates

# Renew manually
sudo certbot renew --force-renewal

# Caddy - Check logs
sudo journalctl -u caddy -n 50
```

### Database Connection Issues

```bash
# Test database connection
docker-compose -f compose.prod.yml exec postgres \
  psql -U rustylinks -d rustylinks -c "SELECT 1"

# Check connection from app
docker-compose -f compose.prod.yml exec app \
  env | grep DATABASE_URL
```

### High Resource Usage

```bash
# Check resource usage
docker stats

# Adjust limits in compose.prod.yml
# Restart services
docker-compose -f compose.prod.yml restart app
```

---

## Maintenance

### Weekly Tasks

- Check logs for errors
- Verify backups completed
- Review disk space usage
- Check application accessibility

### Monthly Tasks

- Update Docker images
- Run security audit
- Review access logs
- Test backup restoration
- Update system packages

### Quarterly Tasks

- Review security configurations
- Rotate passwords (if needed)
- Update documentation
- Conduct performance testing

---

## Scaling (Future)

### Horizontal Scaling

```yaml
# Multiple app instances
services:
  app:
    deploy:
      replicas: 3

# Load balancer needed (Nginx, HAProxy, etc.)
```

### Database Scaling

- Read replicas for scaling reads
- Connection pooling optimization
- Indexed queries for performance

---

## Support

For deployment issues:
- Review logs: `docker-compose logs`
- Check [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
- Review [SECURITY.md](SECURITY.md)
- Consult [DATABASE.md](DATABASE.md)
- Check GitHub Issues

---

## Additional Resources

- [Docker Documentation](https://docs.docker.com/)
- [Nginx Documentation](https://nginx.org/en/docs/)
- [Caddy Documentation](https://caddyserver.com/docs/)
- [Let's Encrypt Documentation](https://letsencrypt.org/docs/)
- [PostgreSQL Security](https://www.postgresql.org/docs/current/security.html)
