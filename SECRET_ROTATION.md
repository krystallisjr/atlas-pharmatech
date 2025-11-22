# Secret Rotation Log - Atlas PharmaTech

## 🔐 Latest Rotation: 2025-11-18

### Rotated Secrets:
- ✅ JWT_SECRET (512-bit cryptographically random)
- ✅ ENCRYPTION_KEY (256-bit cryptographically random)
- ✅ DATABASE_PASSWORD (192-bit cryptographically random)
- ✅ RUST_LOG changed from `debug` to `info` (production security)

### NOT Rotated (Requires User Action):
- ⚠️ ANTHROPIC_API_KEY (Z.AI API key - rotate via Z.AI dashboard)

---

## 📋 Secret Rotation Checklist

### When to Rotate:
- **Immediately** if secrets are exposed (git commit, logs, breach)
- **Every 90 days** for encryption keys
- **Every 180 days** for JWT secrets
- **After** employee offboarding with access
- **Before** major production deployments

### How to Rotate Secrets:

#### 1. Generate New Secrets
```bash
# JWT Secret (512-bit)
openssl rand -base64 64

# Encryption Key (256-bit)
openssl rand -base64 32

# Database Password (192-bit, URL-safe)
openssl rand -base64 24 | tr '+/' '-_'
```

#### 2. Update .env File
```bash
# Backup current .env
cp .env .env.backup.$(date +%Y%m%d_%H%M%S)

# Edit .env with new secrets
nano .env
```

#### 3. Update Database Password
```bash
# Connect to PostgreSQL
PGPASSWORD='OLD_PASSWORD' psql -h localhost -U postgres -d postgres

# Inside psql:
ALTER USER postgres WITH PASSWORD 'NEW_PASSWORD';
\q
```

#### 4. Re-encrypt All PII Data (if ENCRYPTION_KEY changed)
```sql
-- This requires a migration script to:
-- 1. Decrypt all PII with old key
-- 2. Re-encrypt with new key
-- 3. Update all encrypted columns
-- CRITICAL: Test on backup database first!
```

#### 5. Invalidate All JWT Tokens (if JWT_SECRET changed)
```sql
-- All users must re-authenticate
-- Sessions become invalid automatically with new JWT secret
```

#### 6. Test Application
```bash
# Start backend
cargo run --release

# Verify:
- Login still works (new JWT tokens issued)
- PII decryption works (if encryption key unchanged)
- Database connection works
- API calls succeed
```

#### 7. Deploy to Production
```bash
# Update production .env
# Restart services
# Monitor logs for errors
```

---

## 🚨 Emergency Rotation (Secret Exposed)

### Immediate Actions (< 1 hour):
1. **STOP ALL SERVICES** - Prevent further exploitation
2. **Generate new secrets** - Use commands above
3. **Update production .env** - Via secure channel (not git!)
4. **Restart all services** - With new secrets
5. **Invalidate all sessions** - Force re-authentication
6. **Audit access logs** - Check for unauthorized access
7. **Notify security team** - Document incident

### Follow-up Actions (< 24 hours):
1. **Rotate ALL secrets** - Even if not exposed
2. **Review git history** - Ensure no secrets committed
3. **Update secret manager** - If using Vault/KMS
4. **Security audit** - Full codebase scan
5. **Incident report** - Document timeline and actions

---

## 🔒 Best Practices

### Secret Management:
- ✅ NEVER commit .env to git
- ✅ USE .env.example for templates
- ✅ STORE production secrets in secret manager (Vault, KMS, Secrets Manager)
- ✅ ROTATE secrets regularly (90-180 days)
- ✅ USE strong, random secrets (openssl rand)
- ✅ LIMIT secret access (need-to-know basis)
- ✅ AUDIT secret access (log all retrievals)
- ✅ ENCRYPT secrets at rest
- ✅ USE different secrets per environment (dev/staging/prod)

### Don'ts:
- ❌ Don't share secrets via email/Slack
- ❌ Don't reuse secrets across environments
- ❌ Don't use weak/predictable secrets
- ❌ Don't store secrets in code
- ❌ Don't log secrets (even encrypted)
- ❌ Don't commit secrets to git (even private repos)

---

## 📅 Rotation Schedule

| Secret | Last Rotated | Next Due | Frequency |
|--------|--------------|----------|-----------|
| JWT_SECRET | 2025-11-18 | 2026-05-17 | 180 days |
| ENCRYPTION_KEY | 2025-11-18 | 2026-02-16 | 90 days |
| DATABASE_PASSWORD | 2025-11-18 | 2026-05-17 | 180 days |
| ANTHROPIC_API_KEY | (user managed) | (user managed) | As needed |
| TLS Certificates | (check expiry) | (check expiry) | Before expiry |

---

## 🔑 Secret Inventory

| Secret | Purpose | Storage | Encryption | Access |
|--------|---------|---------|------------|--------|
| JWT_SECRET | JWT token signing | .env | At rest | Backend only |
| ENCRYPTION_KEY | PII encryption | .env | At rest | Backend only |
| DATABASE_PASSWORD | PostgreSQL auth | .env | At rest | Backend + DB |
| ANTHROPIC_API_KEY | AI API access | .env | At rest | Backend only |
| TLS Private Key | HTTPS encryption | File system | At rest | Backend only |

---

## 📞 Emergency Contacts

**Security Team:** [security@atlaspharmatech.com](mailto:security@atlaspharmatech.com)
**On-Call Engineer:** [oncall@atlaspharmatech.com](mailto:oncall@atlaspharmatech.com)
**Database Admin:** [dba@atlaspharmatech.com](mailto:dba@atlaspharmatech.com)

---

**Last Updated:** 2025-11-18
**Next Review:** 2026-02-16
**Document Version:** 1.0
