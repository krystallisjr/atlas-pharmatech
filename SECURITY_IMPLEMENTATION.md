# Atlas Pharma - Complete Security Implementation

**Status:** IN PROGRESS
**Started:** 2025-11-13
**Target Completion:** 1 week
**Security Level:** Enterprise-Grade / Audit-Ready

## Configuration
- **Timeline:** 1 week with thorough testing
- **Hosting:** DigitalOcean/AWS production
- **Database:** PostgreSQL on same server (secured)
- **Monitoring:** File-based logging
- **MFA:** Full TOTP implementation

## Implementation Checklist

### ✅ COMPLETED

#### Phase 2A: Encryption Service
- [x] Added security dependencies to Cargo.toml
- [x] Created `/src/services/encryption_service.rs`
  - AES-256-GCM implementation
  - Unique nonce generation
  - Authenticated encryption
  - Base64 encoding for database storage
  - Comprehensive test suite
  - Helper methods for Optional fields

### 🚧 IN PROGRESS

#### Phase 1: Transport & Auth Security
- [ ] TLS/HTTPS with rustls
  - [ ] Create `/src/config/tls.rs`
  - [ ] Generate development certificates
  - [ ] Configure Axum for HTTPS
  - [ ] Update server binding in main.rs

- [ ] CORS Configuration
  - [ ] Parse CORS_ORIGINS from environment
  - [ ] Whitelist specific origins
  - [ ] Remove `allow_origin(Any)`

- [ ] Secure Cookies for JWT
  - [ ] Update auth handlers to set httpOnly cookies
  - [ ] Add Secure and SameSite flags
  - [ ] Update auth middleware to read from cookies
  - [ ] Remove localStorage usage in frontend

#### Phase 2: Data Encryption
- [ ] Database Schema Updates
  - [ ] Create migration 007_encrypt_user_pii.sql
  - [ ] Add encrypted columns for PII fields
  - [ ] Data migration script

- [ ] Encrypt User PII
  - [ ] Update user_repo.rs create() method
  - [ ] Update user_repo.rs update() method
  - [ ] Update user_repo.rs find methods (decrypt on read)
  - [ ] Update UserResponse to handle encrypted fields

- [ ] File Upload Encryption
  - [ ] Update file_storage.rs save_file()
  - [ ] Encrypt files before writing to disk
  - [ ] Decrypt files on read
  - [ ] Update file metadata storage

#### Phase 3: Attack Prevention
- [ ] Rate Limiting
  - [ ] Configure tower-governor
  - [ ] Apply to /api/auth/login (5 attempts/min)
  - [ ] Apply to /api/auth/register (3 attempts/hour)
  - [ ] Create rate limit configuration

- [ ] Token Blacklist
  - [ ] Create migration 008_token_blacklist.sql
  - [ ] Create TokenBlacklistService
  - [ ] Update logout endpoint
  - [ ] Update auth middleware to check blacklist

- [ ] Input Sanitization
  - [ ] Whitelist validation for sort_by parameters
  - [ ] Add to inventory repository

#### Phase 4: MFA Implementation
- [ ] TOTP Setup
  - [ ] Create MFA database schema
  - [ ] Create MFAService with totp-rs
  - [ ] QR code generation endpoint
  - [ ] MFA verification endpoint
  - [ ] Recovery codes generation

- [ ] Auth Flow Updates
  - [ ] Add MFA step to login flow
  - [ ] Frontend MFA UI components
  - [ ] MFA enrollment page
  - [ ] MFA settings management

#### Phase 5: Audit & Monitoring
- [ ] Audit Logging
  - [ ] Create audit_logs table
  - [ ] Log authentication events
  - [ ] Log data access
  - [ ] Log sensitive operations
  - [ ] Create audit log query API

- [ ] Security Headers
  - [ ] Add HSTS headers
  - [ ] Add CSP headers
  - [ ] Add X-Frame-Options
  - [ ] Add X-Content-Type-Options

## Files Modified/Created

### New Files Created
1. `/src/services/encryption_service.rs` ✅
2. `/src/config/tls.rs` (pending)
3. `/src/services/mfa_service.rs` (pending)
4. `/src/services/audit_service.rs` (pending)
5. `/src/services/token_blacklist_service.rs` (pending)
6. `/migrations/007_encrypt_user_pii.sql` (pending)
7. `/migrations/008_token_blacklist.sql` (pending)
8. `/migrations/009_mfa_setup.sql` (pending)
9. `/migrations/010_audit_logs.sql` (pending)
10. `/certs/` directory for TLS certificates (pending)

### Files to Modify
1. `/Cargo.toml` ✅ (dependencies added)
2. `/src/main.rs` (TLS, rate limiting)
3. `/src/config/mod.rs` (encryption config)
4. `/src/services/mod.rs` (export new services)
5. `/src/repositories/user_repo.rs` (encryption on CRUD)
6. `/src/handlers/auth.rs` (cookies, MFA)
7. `/src/middleware/auth.rs` (cookie reading, blacklist check)
8. `/src/utils/file_storage.rs` (file encryption)
9. `/.env.example` (new environment variables)

### Frontend Files to Modify
1. `/atlas-frontend/src/lib/api-client.ts` (cookie-based auth)
2. `/atlas-frontend/src/lib/services/auth-service.ts` (remove localStorage)
3. `/atlas-frontend/src/contexts/auth-context.tsx` (cookie handling)

## Environment Variables Needed

```env
# Existing
DATABASE_URL=...
JWT_SECRET=...

# NEW - Add these
ENCRYPTION_KEY=<generate with: cargo run --bin generate-keys>
CORS_ORIGINS=https://yourdomain.com,https://www.yourdomain.com
DATABASE_SSL_MODE=require
TLS_CERT_PATH=./certs/cert.pem
TLS_KEY_PATH=./certs/key.pem
RATE_LIMIT_PER_MINUTE=60
RATE_LIMIT_AUTH_PER_MINUTE=5
```

## Key Generation Commands

```bash
# Generate encryption key
cd /home/user/Atlas
cargo build
# Run key generator (will create after implementing)
./target/debug/generate-keys

# Generate TLS certificates (development)
openssl req -x509 -newkey rsa:4096 -keyout certs/key.pem -out certs/cert.pem -days 365 -nodes

# Generate TLS certificates (production with Let's Encrypt)
# Will provide certbot commands in deployment guide
```

## Security Features Implemented

### Encryption
- ✅ AES-256-GCM for PII data at rest
- ⏳ TLS 1.3 for data in transit
- ⏳ File encryption for uploads
- ⏳ Encrypted database backups (guide)

### Authentication
- ✅ bcrypt password hashing (cost 12)
- ⏳ JWT with httpOnly secure cookies
- ⏳ TOTP-based MFA
- ⏳ Token blacklist for instant logout

### Attack Prevention
- ⏳ Rate limiting (brute force protection)
- ⏳ CORS whitelist
- ⏳ CSRF protection
- ⏳ Input sanitization whitelist
- ✅ SQL injection prevention (SQLx parameterized queries)

### Monitoring & Compliance
- ⏳ Comprehensive audit logging
- ⏳ Security headers (HSTS, CSP, etc.)
- ⏳ Password strength requirements
- ⏳ Failed login monitoring

## Documentation to Create

1. **SECURITY_WHITEPAPER.md** - For investors
   - Architecture overview
   - Encryption specifications
   - Compliance readiness

2. **DEPLOYMENT_GUIDE.md** - Step-by-step deployment
   - Server setup
   - TLS certificate installation
   - Database configuration
   - Environment variables

3. **KEY_MANAGEMENT.md** - Key lifecycle
   - Key generation
   - Key rotation procedure
   - Backup and recovery

4. **INCIDENT_RESPONSE.md** - Security incidents
   - Incident classification
   - Response procedures
   - Communication plan

5. **COMPLIANCE_CHECKLIST.md** - Regulatory compliance
   - HIPAA readiness
   - GDPR compliance
   - SOC 2 prep

## Testing Plan

### Unit Tests
- [x] Encryption service tests
- [ ] MFA service tests
- [ ] Token blacklist tests
- [ ] Rate limiting tests

### Integration Tests
- [ ] TLS connection tests
- [ ] Encrypted data round-trip tests
- [ ] MFA flow tests
- [ ] Rate limit enforcement tests

### Security Tests
- [ ] Penetration testing checklist
- [ ] OWASP Top 10 validation
- [ ] Encryption key rotation test
- [ ] Token blacklist effectiveness

## Next Steps

**Continue in next session:**
1. Implement TLS/HTTPS configuration
2. Create database migrations for encrypted fields
3. Update user repository with encryption
4. Implement rate limiting
5. Create MFA service
6. Generate all documentation

**Estimated remaining time:** 8-10 hours of implementation
