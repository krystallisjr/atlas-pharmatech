# Atlas PharmaTech - Production Deployment Guide

**Last Updated:** 2025-11-19
**Status:** ✅ Production Ready

---

## 🎯 Overview

This guide covers the complete production deployment process for the Atlas PharmaTech B2B Pharmaceutical Platform with all security features fully integrated and operational.

---

## ✅ Prerequisites

### System Requirements
- **OS:** Linux (Ubuntu 20.04+ recommended) or similar
- **Rust:** 1.70+ (latest stable recommended)
- **PostgreSQL:** 14+ with pgvector extension
- **RAM:** 4GB minimum, 8GB+ recommended
- **Disk:** 20GB+ available space
- **CPU:** 2+ cores recommended

### Required Services
- PostgreSQL database with connection pooling configured
- TLS certificates (Let's Encrypt recommended)
- Prometheus server (for metrics scraping)
- Log aggregation service (optional but recommended)

---

## 📦 Step 1: Initial Setup

### 1.1 Clone and Build

```bash
# Clone repository
git clone <repository-url>
cd Atlas

# Build release binary
cargo build --release

# Binary will be at: target/release/atlas-pharma
```

### 1.2 Database Setup

```bash
# Create database
createdb atlas_pharma

# Run all migrations (ensure DATABASE_URL is set)
PGPASSWORD="your_password" psql -h localhost -U postgres -d atlas_pharma -f migrations/001_initial_schema.sql
# ... run all migrations in order, or use:

# Run all migrations at once
for file in migrations/*.sql; do
    echo "Running $file..."
    PGPASSWORD="your_password" psql -h localhost -U postgres -d atlas_pharma -f "$file"
done
```

**Verify all migrations ran successfully:**
```bash
PGPASSWORD="your_password" psql -h localhost -U postgres -d atlas_pharma -c "\dt"
```

You should see tables including:
- `users`
- `user_api_quotas`
- `api_usage_log`
- `data_encryption_keys`
- `mfa_trigger_bypass_log`
- And 30+ other tables

---

## 🔐 Step 2: Security Configuration

### 2.1 Generate Secure Secrets

```bash
# Generate 512-bit JWT secret (production strength)
openssl rand -base64 64

# Generate 256-bit encryption key
openssl rand -base64 32

# Generate webhook secrets
openssl rand -hex 32  # For NetSuite
openssl rand -hex 32  # For SAP
```

### 2.2 Configure Environment Variables

Create `.env` file (use `.env.example` as template):

```bash
# Application
NODE_ENV=production
RUST_LOG=atlas_pharma=info,tower_http=info,sqlx=warn

# Database (with connection pooling configured in code)
DATABASE_URL=postgres://user:password@localhost:5432/atlas_pharma
DATABASE_HOST=localhost
DATABASE_PORT=5432
DATABASE_USER=your_user
DATABASE_PASSWORD=your_secure_password
DATABASE_NAME=atlas_pharma
DATABASE_SSL_MODE=require

# JWT Authentication (512-bit minimum for production)
JWT_SECRET=<your-512-bit-secret-from-step-2.1>
JWT_EXPIRY_HOURS=24

# Encryption (256-bit AES-GCM)
ENCRYPTION_KEY=<your-256-bit-key-from-step-2.1>

# API Keys
ANTHROPIC_API_KEY=<your-anthropic-api-key>

# CORS (whitelist specific domains)
CORS_ORIGINS=https://yourdomain.com,https://app.yourdomain.com

# File Storage
FILE_STORAGE_PATH=/var/lib/atlas-pharma/uploads

# TLS Configuration
TLS_ENABLED=true
TLS_CERT_PATH=/etc/ssl/certs/atlas-pharma.crt
TLS_KEY_PATH=/etc/ssl/private/atlas-pharma.key
TLS_PORT=8443
TLS_CERT_RENEWAL_DAYS_THRESHOLD=30

# Webhook Security
NETSUITE_WEBHOOK_SECRET=<webhook-secret-1>
SAP_WEBHOOK_SECRET=<webhook-secret-2>
```

**🚨 SECURITY WARNING:**
- Never commit `.env` to version control
- Use environment-specific secrets (dev, staging, prod)
- Rotate secrets every 90 days
- Store secrets in a secret manager (AWS Secrets Manager, HashiCorp Vault, etc.)

### 2.3 Set File Permissions

```bash
# Secure environment file
chmod 600 .env

# Create file storage directory
sudo mkdir -p /var/lib/atlas-pharma/uploads
sudo chown atlas-pharma:atlas-pharma /var/lib/atlas-pharma/uploads
chmod 750 /var/lib/atlas-pharma/uploads
```

---

## 🚀 Step 3: Initialize Security Services

### 3.1 First-Time Service Initialization

When you start the application for the first time, the following services will auto-initialize:

**API Quota Service:**
- Automatically creates Free tier quotas for all existing users
- Default: 100 requests/month per user
- Check logs for: `✅ API Quota Service initialized`

**Encryption Key Rotation Service:**
- Generates initial Data Encryption Key (DEK)
- Sets up 90-day rotation schedule
- Check logs for: `✅ Encryption Key Rotation Service initialized`

**Expected Startup Logs:**
```
🔐 Initializing API Quota Service...
✅ API Quota Service initialized (X users configured)
🔐 Initializing Encryption Key Rotation Service...
✅ Encryption Key Rotation Service initialized
✅ Next encryption key rotation in 90 days
```

### 3.2 Verify Initialization

```sql
-- Check API quotas
SELECT COUNT(*) FROM user_api_quotas;

-- Check encryption keys
SELECT id, version, is_active, created_at, valid_until
FROM data_encryption_keys
ORDER BY version DESC;

-- Should see 1 active key with 90-day validity
```

---

## 🔧 Step 4: Run the Application

### 4.1 Development/Testing

```bash
# Run with cargo
cargo run --release

# Or run the binary directly
./target/release/atlas-pharma
```

### 4.2 Production (systemd)

Create systemd service file `/etc/systemd/system/atlas-pharma.service`:

```ini
[Unit]
Description=Atlas PharmaTech B2B Platform
After=network.target postgresql.service

[Service]
Type=simple
User=atlas-pharma
Group=atlas-pharma
WorkingDirectory=/opt/atlas-pharma
EnvironmentFile=/opt/atlas-pharma/.env
ExecStart=/opt/atlas-pharma/target/release/atlas-pharma
Restart=always
RestartSec=10

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/atlas-pharma

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=atlas-pharma

[Install]
WantedBy=multi-user.target
```

Start the service:
```bash
sudo systemctl daemon-reload
sudo systemctl enable atlas-pharma
sudo systemctl start atlas-pharma

# Check status
sudo systemctl status atlas-pharma

# View logs
sudo journalctl -u atlas-pharma -f
```

---

## 📊 Step 5: Monitoring Setup

### 5.1 Prometheus Configuration

Add to `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'atlas-pharma'
    static_configs:
      - targets: ['localhost:8443']
    metrics_path: '/metrics'
    scrape_interval: 15s
    scheme: https
    tls_config:
      insecure_skip_verify: false  # Set to false in production with valid certs
```

### 5.2 Available Metrics

The application exposes the following Prometheus metrics at `/metrics`:

- `atlas_http_request_duration_seconds` - Request latency histogram
- `atlas_http_requests_total` - Total HTTP requests counter
- `atlas_http_connections_active` - Active connections gauge
- `atlas_auth_failures_total` - Authentication failures counter
- `atlas_db_pool_connections` - Database pool state gauge
- `atlas_api_quota_usage_percent` - API quota usage gauge

### 5.3 Grafana Dashboards (Optional)

Import these queries for monitoring:

```promql
# Request rate
rate(atlas_http_requests_total[5m])

# P95 latency
histogram_quantile(0.95, rate(atlas_http_request_duration_seconds_bucket[5m]))

# Error rate
sum(rate(atlas_http_requests_total{status=~"5.."}[5m]))

# Auth failure rate
rate(atlas_auth_failures_total[5m])

# Database pool utilization
atlas_db_pool_connections{state="active"} /
(atlas_db_pool_connections{state="active"} + atlas_db_pool_connections{state="idle"})
```

---

## 🔒 Step 6: Security Validation

### 6.1 Security Checklist

Run through this checklist to verify all security features:

```bash
# 1. Check TLS is enabled
curl -I https://localhost:8443/api/auth/login

# 2. Verify security headers
curl -I https://localhost:8443/api/auth/login | grep -E "X-Frame-Options|X-Content-Type-Options|Strict-Transport-Security"

# 3. Test rate limiting (should get 429 after 5 attempts)
for i in {1..10}; do curl -X POST https://localhost:8443/api/auth/login -d '{}'; done

# 4. Verify CORS is restricted
curl -H "Origin: https://evil.com" https://localhost:8443/api/auth/login -I

# 5. Check metrics endpoint is accessible
curl https://localhost:8443/metrics

# 6. Test Content-Type validation
curl -X POST https://localhost:8443/api/auth/login -H "Content-Type: text/plain" -d '{}'
# Should return 415 Unsupported Media Type
```

### 6.2 Database Security

```sql
-- Verify MFA trigger bypass protection
SELECT * FROM mfa_trigger_bypass_log;

-- Check encryption key rotation status
SELECT version, is_active,
       EXTRACT(DAY FROM (valid_until - NOW())) as days_remaining
FROM data_encryption_keys
WHERE is_active = true;

-- Verify API quotas are enforced
SELECT u.email, q.quota_tier,
       COUNT(l.id) as requests_this_month
FROM users u
JOIN user_api_quotas q ON u.id = q.user_id
LEFT JOIN api_usage_log l ON u.id = l.user_id
    AND l.created_at >= DATE_TRUNC('month', NOW())
GROUP BY u.email, q.quota_tier;
```

---

## 🔄 Step 7: Maintenance Tasks

### 7.1 Encryption Key Rotation (Every 90 Days)

**Automated Check:**
The application logs warnings when rotation is needed:
```
⚠️  Encryption key rotation recommended in 7 days
```

**Manual Rotation:**
```bash
# TODO: Add admin CLI command for key rotation
# For now, rotation happens automatically when keys expire
# Monitor logs for automatic rotation events
```

### 7.2 Secret Rotation (Every 90 Days)

Follow the guide in `SECRET_ROTATION.md`:

1. Generate new JWT secret
2. Update `.env` file
3. Restart application
4. All existing tokens will be invalidated (users must re-login)

### 7.3 Database Maintenance

```bash
# Vacuum database monthly
PGPASSWORD="your_password" psql -h localhost -U postgres -d atlas_pharma -c "VACUUM ANALYZE;"

# Check database size
PGPASSWORD="your_password" psql -h localhost -U postgres -d atlas_pharma -c "SELECT pg_size_pretty(pg_database_size('atlas_pharma'));"

# Archive old audit logs (older than 1 year)
# TODO: Add archiving script
```

### 7.4 Log Management

```bash
# Rotate logs weekly
sudo journalctl --vacuum-time=7d

# Export logs for compliance
sudo journalctl -u atlas-pharma --since "2024-01-01" --until "2024-12-31" > audit-2024.log
```

---

## 🔥 Step 8: Troubleshooting

### Common Issues

**Issue: Application fails to start**
```bash
# Check logs
sudo journalctl -u atlas-pharma -n 50

# Common causes:
# - Database connection failed (check DATABASE_URL)
# - Missing encryption key (check ENCRYPTION_KEY in .env)
# - TLS certificate not found (check TLS_CERT_PATH)
```

**Issue: Metrics endpoint returns 404**
```bash
# Verify route is configured
curl -v https://localhost:8443/metrics

# Check if metrics middleware is loaded (look for log):
# "📊 OBSERVABILITY: Prometheus metrics collection"
```

**Issue: API quota not enforced**
```sql
-- Check if quotas table exists
SELECT COUNT(*) FROM user_api_quotas;

-- Check if user has quota configured
SELECT * FROM user_api_quotas WHERE user_id = '<user-uuid>';

-- Re-initialize if needed (safe to run multiple times)
-- Will be done automatically on next restart
```

**Issue: Encryption key rotation not working**
```sql
-- Check current keys
SELECT * FROM data_encryption_keys ORDER BY version DESC;

-- Verify master key is set
-- Check ENCRYPTION_KEY in .env is present and correct length (44 chars base64)
```

---

## 📋 Step 9: Production Readiness Checklist

Before going live, verify:

### Infrastructure
- [ ] PostgreSQL 14+ with connection pooling (configured in code)
- [ ] TLS certificates installed and valid
- [ ] Firewall rules configured (allow 8443, restrict PostgreSQL)
- [ ] Prometheus monitoring configured
- [ ] Log aggregation service connected
- [ ] Backup system configured (daily database backups)
- [ ] Disaster recovery plan documented

### Security
- [ ] All secrets rotated and secure (512-bit JWT, 256-bit encryption)
- [ ] Environment variables protected (chmod 600 .env)
- [ ] Database migrations applied (014 migrations total)
- [ ] API quota service initialized
- [ ] Encryption key rotation service initialized
- [ ] MFA TOTP system tested
- [ ] Rate limiting verified (5 auth/15min, 100 API/min)
- [ ] Security headers present (8 headers)
- [ ] CSRF protection enabled
- [ ] Content-Type validation active

### Application
- [ ] Application starts successfully
- [ ] Health check responds: `curl https://localhost:8443/api/admin/health`
- [ ] Metrics endpoint accessible: `curl https://localhost:8443/metrics`
- [ ] User registration works
- [ ] User login works
- [ ] API quota enforcement tested
- [ ] File upload encryption tested

### Compliance
- [ ] Audit logging enabled and tested
- [ ] PII encryption verified (5 encrypted fields)
- [ ] Request ID tracking operational
- [ ] GDPR data export capability confirmed
- [ ] HIPAA audit trail verified
- [ ] SOC 2 compliance controls documented

---

## 🎉 Deployment Complete!

Your Atlas PharmaTech platform is now production-ready with enterprise-grade security!

**Next Steps:**
1. Monitor application logs for first 24 hours
2. Set up alerting for critical metrics
3. Schedule first security audit in 30 days
4. Plan encryption key rotation for day 90
5. Begin user onboarding

**Support:**
- Documentation: `docs/`
- Security Issues: See `SECURITY_FIXES_SUMMARY.md`
- Maintenance: See `SECRET_ROTATION.md`

---

**Document Version:** 1.0
**Created:** 2025-11-19
**Author:** Security Engineering Team
