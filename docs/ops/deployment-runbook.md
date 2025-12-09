# Deployment Runbook — Heimdall on Linux Hosts

## Purpose

This runbook provides step-by-step instructions for deploying Heimdall in production on Linux hosts, including guidance on Postgres+AGE provisioning, TLS configuration, backups, and operational procedures.

## Prerequisites

### System Requirements

- **Operating System**: Linux (Ubuntu 22.04 LTS, RHEL 8/9, or equivalent recommended)
- **CPU**: Minimum 4 cores (8+ cores recommended for production)
- **Memory**: Minimum 8 GB RAM (16+ GB recommended for production)
- **Storage**: Minimum 100 GB SSD (storage requirements scale with data volume)
- **Network**: TLS 1.3 capable; no self-signed certificates in production

### Software Dependencies

- **Rust toolchain**: Latest stable version (install via [rustup](https://rustup.rs/))
- **Docker** (if using sidecar deployment): Docker Engine 20.10+ and Docker Compose v2+
- **PostgreSQL**: Version 14+ with Apache AGE 1.3+ and pgvector extension
- **TLS certificates**: Valid certificates from a trusted CA (self-signed certificates are rejected)

### Required Credentials and Secrets

- **OAuth/OIDC Provider**: Discovery URL and client credentials for H2M (human-to-machine) and M2M (machine-to-machine) flows
- **TLS Certificates**: Private key and certificate chain
- **Database Credentials**: PostgreSQL connection details
- **Cookie Encryption Secret**: 32+ byte random secret for secure session cookies

## Deployment Options

Heimdall supports two primary deployment models:

1. **Sidecar Deployment**: Run Postgres+AGE in a Docker container alongside Heimdall
2. **Managed Database**: Use a managed PostgreSQL service with AGE and pgvector extensions installed

### Option 1: Sidecar Deployment (Docker Compose)

Recommended for development, staging, and small production deployments.

#### Step 1: Clone Repository

```bash
git clone https://github.com/Vanopticon/Heimdall.git
cd Heimdall
git checkout v1.0.0
```

#### Step 2: Build Heimdall

```bash
cargo build --release
```

The compiled binary will be located at `target/release/vanopticon_heimdall`.

#### Step 3: Start Postgres+AGE Sidecar

```bash
# Build and start the database container
docker compose build db
docker compose up -d db

# Verify the container is running
docker compose ps
```

The database will be available at `localhost:5432` with:

- Database: `heimdall`
- User: `heimdall`
- Password: `heimdall` (default - **change for production**)

**Production Security Note**: Override default database credentials by setting environment variables in your docker-compose override or environment file:

```bash
# In docker-compose.override.yml or as environment variables
POSTGRES_PASSWORD=your-secure-password-here
POSTGRES_USER=heimdall
POSTGRES_DB=heimdall
```

The initialization script (`docker/postgres-age/initdb/01-extensions.sql`) automatically creates the `vector` and `ag` extensions and the `heimdall_graph` AGE graph.

#### Step 4: Configure Environment Variables

Create a `.env` file or export environment variables:

```bash
# Required: TLS Configuration
export HMD_TLS_KEY=/path/to/tls/private.key
export HMD_TLS_CERT=/path/to/tls/certificate.crt

# Required: OAuth/OIDC Configuration (replace all with actual values from your provider)
export HMD_OAUTH_DISCOVERY_URL=https://auth.example.com/.well-known/openid-configuration
export HMD_OAUTH_H2M_ID=REPLACE_WITH_ACTUAL_H2M_CLIENT_ID
export HMD_OAUTH_H2M_SECRET=REPLACE_WITH_ACTUAL_H2M_CLIENT_SECRET
export HMD_OAUTH_M2M_ID=REPLACE_WITH_ACTUAL_M2M_CLIENT_ID
export HMD_OAUTH_M2M_SECRET=REPLACE_WITH_ACTUAL_M2M_CLIENT_SECRET

# Optional: Server Configuration
export HMD_HOST=0.0.0.0
export HMD_PORT=443

# Optional: Database Configuration (defaults work with sidecar)
export HMD_DATABASE_URL=postgres://heimdall:heimdall@localhost:5432/heimdall
export HMD_AGE_GRAPH=heimdall_graph

# Optional: Security (generate cookie secret once and store securely)
# Generate once with: openssl rand -base64 32
# REQUIRED: Replace GENERATE_AND_REPLACE_ME with actual secret before deployment
export HMD_COOKIE_SECRET=GENERATE_AND_REPLACE_ME
export HMD_OIDC_SCOPE="openid profile email"
```

**Security Note**: Replace **all** placeholder values (REPLACE_WITH_*, GENERATE_AND_REPLACE_ME) with actual credentials from your OAuth provider, database, and generated secrets. Never commit secrets to version control. Use a secrets manager (HashiCorp Vault, AWS Secrets Manager, etc.) or environment-specific configuration files with restricted permissions (chmod 600).

#### Step 5: Run Heimdall

```bash
# Run the service
./target/release/vanopticon_heimdall run

# Or use systemd (see systemd service example below)
```

#### Step 6: Verify Deployment

```bash
# Check service health
curl -k https://localhost/health

# Verify database connectivity
docker compose exec db psql -U heimdall -d heimdall -c "SELECT * FROM ag_catalog.ag_graph;"
```

### Option 2: Managed Database Deployment

Recommended for large-scale production deployments.

#### Step 1: Provision Managed PostgreSQL

Choose a managed PostgreSQL provider that supports custom extensions:

- **AWS RDS**: PostgreSQL with custom extensions
- **Azure Database for PostgreSQL**: Flexible Server with extension support
- **Google Cloud SQL**: PostgreSQL with extension installation
- **Self-managed**: PostgreSQL on dedicated VMs with manual AGE/pgvector installation

#### Step 2: Install Required Extensions

Connect to your managed database and install extensions:

```sql
-- Connect as superuser or database owner
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS age;

-- Create AGE schema and graph
CREATE SCHEMA IF NOT EXISTS ag_catalog;
SELECT ag_catalog.create_graph('heimdall_graph');
```

**Note**: Some managed providers require support tickets to install custom extensions like Apache AGE. Plan for additional lead time.

#### Step 3: Configure Database Connection

Set environment variables to point to your managed database:

```bash
export HMD_DATABASE_URL=postgres://username:password@db-host.example.com:5432/heimdall?sslmode=require
export HMD_AGE_GRAPH=heimdall_graph
```

#### Step 4: Deploy Heimdall Binary

Build and deploy the Heimdall binary to your application servers:

```bash
# Build release binary
cargo build --release

# Copy to deployment location
sudo cp target/release/vanopticon_heimdall /usr/local/bin/heimdall
sudo chmod +x /usr/local/bin/heimdall
```

#### Step 5: Configure and Start Service

Follow steps 4-6 from the sidecar deployment, adjusting the database connection details.

## TLS Configuration

### Certificate Requirements

- **Protocol**: TLS 1.3 only (TLS 1.2 and below are rejected)
- **Certificate Authority**: Must be signed by a trusted CA; self-signed certificates are not permitted in production
- **Key Type**: RSA 2048+ or ECDSA P-256+
- **Format**: PEM-encoded certificate chain and private key

### Obtaining Certificates

#### Option 1: Let's Encrypt (Recommended for internet-facing deployments)

```bash
# Install certbot
sudo apt-get update
sudo apt-get install certbot

# Obtain certificate (HTTP-01 challenge)
sudo certbot certonly --standalone -d heimdall.example.com

# Certificates will be placed in:
# /etc/letsencrypt/live/heimdall.example.com/privkey.pem
# /etc/letsencrypt/live/heimdall.example.com/fullchain.pem
```

#### Option 2: Internal CA

For internal deployments, use your organization's internal CA:

```bash
# Generate private key
openssl genrsa -out heimdall.key 2048

# Generate CSR
openssl req -new -key heimdall.key -out heimdall.csr \
  -subj "/CN=heimdall.internal.example.com/O=Your Organization"

# Submit CSR to your internal CA and obtain signed certificate
# Place certificate and key in secure location with restricted permissions
```

### Certificate Permissions

```bash
# Set restrictive permissions on private key
sudo chown heimdall:heimdall /path/to/tls/private.key
sudo chmod 400 /path/to/tls/private.key

# Certificate can be more permissive but still restricted
sudo chown heimdall:heimdall /path/to/tls/certificate.crt
sudo chmod 644 /path/to/tls/certificate.crt
```

### Certificate Rotation

Heimdall requires a restart to load new certificates. Implement a rotation procedure:

1. **Pre-rotation**: Obtain new certificates before current certificates expire (30+ days recommended)
2. **Deploy**: Replace certificate files in place or update paths in environment configuration
3. **Reload**: Restart Heimdall service to load new certificates
4. **Verify**: Test TLS connectivity and certificate validity

#### Automated Rotation with Let's Encrypt

```bash
# Create renewal hook script
sudo cat > /etc/letsencrypt/renewal-hooks/deploy/heimdall-restart.sh <<'EOF'
#!/bin/bash
systemctl restart heimdall
EOF

sudo chmod +x /etc/letsencrypt/renewal-hooks/deploy/heimdall-restart.sh

# Test renewal process
sudo certbot renew --dry-run
```

## Database Operations

### Backup Strategy

#### Logical Backups (pg_dump)

Recommended for daily backups and disaster recovery:

```bash
#!/bin/bash
# backup-heimdall.sh

BACKUP_DIR=/var/backups/heimdall
DATE=$(date +%Y%m%d_%H%M%S)
PGHOST=localhost
PGDATABASE=heimdall
PGUSER=heimdall
# Set POSTGRES_PASSWORD via environment or .pgpass file
# For sidecar: use the password from docker-compose.yml or override
# Example: export POSTGRES_PASSWORD="your-db-password"

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Perform backup (POSTGRES_PASSWORD should be set in environment)
PGPASSWORD="${POSTGRES_PASSWORD}" pg_dump \
  -h "$PGHOST" \
  -U "$PGUSER" \
  -d "$PGDATABASE" \
  -F c \
  -f "$BACKUP_DIR/heimdall_backup_$DATE.dump"

# Compress backup
gzip "$BACKUP_DIR/heimdall_backup_$DATE.dump"

# Retain last 30 days of backups
find "$BACKUP_DIR" -name "heimdall_backup_*.dump.gz" -mtime +30 -delete

echo "Backup completed: $BACKUP_DIR/heimdall_backup_$DATE.dump.gz"
```

Schedule with cron:

```bash
# Run daily at 2 AM
0 2 * * * /usr/local/bin/backup-heimdall.sh >> /var/log/heimdall-backup.log 2>&1
```

#### Physical Backups (pg_basebackup)

For larger databases or faster recovery:

```bash
#!/bin/bash
# basebackup-heimdall.sh

BACKUP_DIR=/var/backups/heimdall/base
DATE=$(date +%Y%m%d_%H%M%S)
PGHOST=localhost
PGUSER=heimdall
# Set POSTGRES_PASSWORD via environment or .pgpass file
# Example: export POSTGRES_PASSWORD="your-db-password"

mkdir -p "$BACKUP_DIR"

# POSTGRES_PASSWORD should be set in environment
PGPASSWORD="${POSTGRES_PASSWORD}" pg_basebackup \
  -h "$PGHOST" \
  -U "$PGUSER" \
  -D "$BACKUP_DIR/base_$DATE" \
  -F t \
  -z \
  -P

echo "Base backup completed: $BACKUP_DIR/base_$DATE"
```

#### Docker Volume Backups (Sidecar Deployment)

```bash
# Backup Docker volume
docker run --rm \
  -v heimdall_pgdata:/data \
  -v /var/backups/heimdall:/backup \
  alpine tar czf /backup/pgdata_$(date +%Y%m%d_%H%M%S).tar.gz /data
```

### Restore Procedures

#### From pg_dump

```bash
# Stop Heimdall service
systemctl stop heimdall

# Restore database
gunzip -c /var/backups/heimdall/heimdall_backup_YYYYMMDD_HHMMSS.dump.gz | \
  PGPASSWORD="${POSTGRES_PASSWORD}" pg_restore \
  -h localhost \
  -U heimdall \
  -d heimdall \
  -c

# Restart Heimdall service
systemctl start heimdall
```

#### From base backup

```bash
# Stop database
docker compose down db

# Remove existing volume
docker volume rm heimdall_pgdata

# Restore from basebackup
# (Extract tar to new volume or restore to data directory)

# Start database
docker compose up -d db
```

### Upgrade Strategies

#### Minor Version Upgrades (PostgreSQL patch releases)

```bash
# Sidecar deployment:
# 1. Update base image in docker/postgres-age/Dockerfile
# 2. Rebuild and restart container
docker compose build db
docker compose down db
docker compose up -d db

# Managed database: Follow provider's upgrade procedure
```

#### Major Version Upgrades (PostgreSQL major releases)

1. **Test in staging**: Always test major upgrades in a non-production environment
2. **Backup**: Perform full backup before upgrade
3. **Upgrade extensions**: Verify Apache AGE and pgvector compatibility with new PostgreSQL version
4. **Perform upgrade**: Use `pg_upgrade` or dump/restore method
5. **Verify**: Test all Heimdall functionality after upgrade

```bash
# Example pg_upgrade process
# 1. Install new PostgreSQL version
# 2. Initialize new cluster
# 3. Run pg_upgrade
sudo -u postgres pg_upgrade \
  -b /usr/lib/postgresql/14/bin \
  -B /usr/lib/postgresql/15/bin \
  -d /var/lib/postgresql/14/main \
  -D /var/lib/postgresql/15/main

# 4. Reinstall extensions in new cluster
# 5. Update heimdall configuration
# 6. Restart services
```

#### Application Upgrades (Heimdall releases)

```bash
# 1. Review release notes and breaking changes
# 2. Backup database
# 3. Build new version
git fetch origin
git checkout v1.1.0
cargo build --release

# 4. Stop service
systemctl stop heimdall

# 5. Replace binary
sudo cp target/release/vanopticon_heimdall /usr/local/bin/heimdall

# 6. Run any migration scripts (if provided)
# 7. Start service
systemctl start heimdall

# 8. Verify functionality
curl -k https://localhost/health
```

## Systemd Service Configuration

Create a systemd service unit for production deployments:

```ini
# /etc/systemd/system/heimdall.service

[Unit]
Description=Heimdall ETL and Data Scrubbing Service
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=heimdall
Group=heimdall
WorkingDirectory=/opt/heimdall

# Environment configuration
EnvironmentFile=/etc/heimdall/environment

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/log/heimdall

# Service execution
ExecStart=/usr/local/bin/heimdall run
Restart=on-failure
RestartSec=10s

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=heimdall

[Install]
WantedBy=multi-user.target
```

Create environment file:

```bash
# /etc/heimdall/environment

HMD_TLS_KEY=/etc/heimdall/tls/private.key
HMD_TLS_CERT=/etc/heimdall/tls/certificate.crt
HMD_OAUTH_DISCOVERY_URL=https://auth.example.com/.well-known/openid-configuration
HMD_OAUTH_H2M_ID=REPLACE_WITH_ACTUAL_H2M_CLIENT_ID
HMD_OAUTH_H2M_SECRET=REPLACE_WITH_ACTUAL_H2M_CLIENT_SECRET
HMD_OAUTH_M2M_ID=REPLACE_WITH_ACTUAL_M2M_CLIENT_ID
HMD_OAUTH_M2M_SECRET=REPLACE_WITH_ACTUAL_M2M_CLIENT_SECRET
HMD_DATABASE_URL=postgres://heimdall:REPLACE_WITH_DB_PASSWORD@localhost:5432/heimdall
HMD_AGE_GRAPH=heimdall_graph
HMD_HOST=0.0.0.0
HMD_PORT=443
# REQUIRED: Generate cookie secret with: openssl rand -base64 32
# Replace GENERATE_AND_REPLACE_ME with the generated secret
HMD_COOKIE_SECRET=GENERATE_AND_REPLACE_ME
```

**Security Note**: Store sensitive values (OAuth secrets, database passwords, cookie secrets) securely. Replace **all** placeholder values (REPLACE_WITH_*, GENERATE_AND_REPLACE_ME, etc.) with actual secrets before deployment. The cookie secret is **required** for secure session management - generate it with `openssl rand -base64 32` and replace the placeholder. Never use placeholder or example values in production.

Set permissions:

```bash
sudo chmod 600 /etc/heimdall/environment
sudo chown heimdall:heimdall /etc/heimdall/environment
```

Enable and start service:

```bash
# Enable service to start on boot
sudo systemctl enable heimdall

# Start service
sudo systemctl start heimdall

# Check status
sudo systemctl status heimdall

# View logs
sudo journalctl -u heimdall -f
```

## Monitoring and Observability

### Health Checks

Heimdall exposes health check endpoints:

```bash
# Basic health check
curl -k https://localhost/health

# Readiness check (includes database connectivity)
curl -k https://localhost/ready
```

Implement health check monitoring:

```bash
# Example monitoring script
#!/bin/bash
# /usr/local/bin/check-heimdall-health.sh

if ! curl -f -k https://localhost/health > /dev/null 2>&1; then
  echo "Heimdall health check failed"
  systemctl restart heimdall
  # Send alert to ops team
fi
```

### Metrics Collection

Heimdall exposes Prometheus-compatible metrics at `/metrics`:

```yaml
# prometheus.yml example
scrape_configs:
  - job_name: 'heimdall'
    scheme: https
    tls_config:
      insecure_skip_verify: false
    static_configs:
      - targets: ['heimdall.example.com:443']
    metrics_path: /metrics
```

### Log Aggregation

Heimdall logs structured JSON to stdout/stderr. Configure log forwarding:

```bash
# Using journald + promtail
# Install promtail and configure to scrape journald

# Or redirect logs to file
sudo journalctl -u heimdall -f -o json > /var/log/heimdall/service.log
```

### Key Metrics to Monitor

- **Ingestion throughput**: Records processed per second
- **Persistence latency**: Time to persist records to database
- **Database connection pool**: Active/idle connections
- **Error rates**: HTTP 4xx/5xx responses
- **TLS certificate expiration**: Days until certificate expires
- **Database disk usage**: Monitor available space

## Troubleshooting

### Common Issues

#### Service fails to start

1. **Check logs**: `sudo journalctl -u heimdall -n 50`
2. **Verify TLS certificates**: Ensure certificates are valid and readable
3. **Test database connectivity**: Verify database is accessible
4. **Check port availability**: Ensure port 443 is not in use

#### Database connection errors

```bash
# Test database connectivity
docker compose exec db psql -U heimdall -d heimdall

# Check AGE extension
docker compose exec db psql -U heimdall -d heimdall -c "SELECT * FROM ag_catalog.ag_graph;"

# Verify graph exists
docker compose exec db psql -U heimdall -d heimdall -c "SELECT * FROM ag_catalog.ag_graph WHERE name = 'heimdall_graph';"
```

#### TLS certificate errors

```bash
# Verify certificate validity
openssl x509 -in /path/to/certificate.crt -text -noout

# Check certificate chain
openssl verify -CAfile /path/to/ca-bundle.crt /path/to/certificate.crt

# Test TLS connection
openssl s_client -connect localhost:443 -tls1_3
```

#### Performance issues

1. **Check database performance**: Monitor slow queries, connection pool saturation
2. **Review resource utilization**: CPU, memory, disk I/O
3. **Analyze persistence batching**: Review batch sizes and flush intervals
4. **Check network latency**: For managed database deployments

## Security Considerations

### Secrets Management

- **Never commit secrets**: Use environment variables or secrets managers
- **Rotate credentials regularly**: OAuth credentials, database passwords, cookie secrets
- **Use least privilege**: Database users should have minimal required permissions
- **Encrypt secrets at rest**: Use encrypted volumes or secrets managers

### Network Security

- **Firewall rules**: Restrict access to Heimdall and database ports
- **TLS 1.3 only**: Disable older TLS versions
- **Private networking**: Use VPC/private networks for database connections
- **Rate limiting**: Implement rate limiting on ingestion endpoints

### Audit Trail

- **Enable database audit logging**: Track all database operations
- **Structured logging**: All operations include actor (OIDC sub), request ID, and timestamp
- **Log retention**: Retain logs according to compliance requirements
- **Secure log storage**: Protect logs from tampering

## Production Checklist

Before deploying to production:

- [ ] TLS certificates obtained from trusted CA
- [ ] OAuth/OIDC provider configured and tested
- [ ] Database provisioned with AGE and pgvector extensions
- [ ] Backup strategy implemented and tested
- [ ] Monitoring and alerting configured
- [ ] Secrets stored securely (not in version control)
- [ ] Firewall rules configured
- [ ] Log aggregation configured
- [ ] Health checks implemented
- [ ] Disaster recovery plan documented
- [ ] Certificate rotation procedure tested
- [ ] Performance baseline established
- [ ] Security review completed
- [ ] Runbook reviewed and approved by repository owners

## References

- [Architecture Documentation](../design/Architecture.md)
- [Postgres+AGE Setup Guide](../POSTGRES_AGE_SETUP.md)
- [Testing Documentation](../TESTING.md)
- [Configuration Module Feature](../design/features/CFG-001-Config-Module.md)
- [Implementation Roadmap](../design/Implementation-Roadmap.md)

## Support and Escalation

For issues not covered in this runbook:

1. Review GitHub Issues: <https://github.com/Vanopticon/Heimdall/issues>
2. Consult design documentation in `docs/design/`
3. Contact repository maintainers

## Change Log

- **2025-12-09**: Initial deployment runbook for Milestone 5, Task 5.2
