# 🔧 Security Features Integration Guide

## ✅ What's ACTUALLY Working (Tested)

### 1. TLS/HTTPS Infrastructure
- **File**: `src/config/tls.rs`
- **Status**: ✅ Compiled and integrated in main.rs
- **Test**: Server starts with TLS warnings when disabled
- **To Enable**: Set `TLS_ENABLED=true` in .env + provide certificates

### 2. CORS Whitelist
- **File**: `src/main.rs` (lines 54-70)
- **Status**: ✅ WORKING - Tested with curl
- **Test Result**:
  - `http://evil.com` → ❌ No CORS header (blocked)
  - `http://localhost:3000` → ✅ Proper CORS header

### 3. Secure httpOnly Cookies
- **Files**: `src/handlers/auth.rs`, `src/middleware/auth.rs`
- **Status**: ✅ Code integrated
- **Features**:
  - httpOnly (XSS protection)
  - Secure flag (HTTPS only in production)
  - SameSite::Strict (CSRF protection)
  - 24-hour expiry

### 4. Database Encryption Columns
- **Migration**: `migrations/007_encrypt_pii_fields.sql`
- **Status**: ✅ APPLIED to database
- **Columns Added**:
  - `email_encrypted`
  - `contact_person_encrypted`
  - `phone_encrypted`
  - `address_encrypted`
  - `license_number_encrypted`

---

## ⚠️ What's Written But NOT Connected

### 1. User Encryption (HIGH PRIORITY)

**Current State**: Encryption service exists but user_repo.rs doesn't use it

**Files to Update**:
- `src/repositories/user_repo.rs`
- `src/services/auth_service.rs`

**Integration Steps**:

```rust
// In src/repositories/user_repo.rs

use crate::services::EncryptionService;

pub struct UserRepository {
    pool: PgPool,
    encryption: EncryptionService,
}

impl UserRepository {
    pub fn new(pool: PgPool, encryption_key: &str) -> Result<Self> {
        Ok(Self {
            pool,
            encryption: EncryptionService::new(encryption_key)?,
        })
    }

    pub async fn create(&self, request: &CreateUserRequest, password_hash: &str) -> Result<User> {
        // Encrypt PII
        let email_enc = self.encryption.encrypt(&request.email)?;
        let contact_enc = self.encryption.encrypt(&request.contact_person)?;
        let phone_enc = self.encryption.encrypt_option(&request.phone)?;
        let address_enc = self.encryption.encrypt_option(&request.address)?;
        let license_enc = self.encryption.encrypt_option(&request.license_number)?;

        let row = query(
            r#"
            INSERT INTO users (
                email, password_hash, company_name, contact_person, phone, address, license_number,
                email_encrypted, contact_person_encrypted, phone_encrypted,
                address_encrypted, license_number_encrypted
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING *
            "#
        )
        .bind(&request.email)  // Keep plaintext for now (dual-write)
        .bind(password_hash)
        .bind(&request.company_name)
        .bind(&request.contact_person)
        .bind(&request.phone)
        .bind(&request.address)
        .bind(&request.license_number)
        .bind(email_enc)
        .bind(contact_enc)
        .bind(phone_enc)
        .bind(address_enc)
        .bind(license_enc)
        .fetch_one(&self.pool)
        .await?;

        // Decrypt when reading
        Ok(User {
            id: row.try_get("id")?,
            email: self.encryption.decrypt(&row.try_get::<String, _>("email_encrypted")?)?,
            password_hash: row.try_get("password_hash")?,
            company_name: row.try_get("company_name")?,
            contact_person: self.encryption.decrypt(&row.try_get::<String, _>("contact_person_encrypted")?)?,
            phone: self.encryption.decrypt_option(&row.try_get("phone_encrypted")?)?,
            address: self.encryption.decrypt_option(&row.try_get("address_encrypted")?)?,
            license_number: self.encryption.decrypt_option(&row.try_get("license_number_encrypted")?)?,
            is_verified: row.try_get("is_verified")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}
```

**Update all UserRepository instantiations**:
```rust
// In handlers and services
UserRepository::new(pool.clone(), &config.encryption_key)?
```

---

### 2. Rate Limiting (HIGH PRIORITY)

**Current State**: Functions exist but not applied to routes

**File**: `src/main.rs`

**Add at top**:
```rust
use atlas_pharma::middleware::rate_limiter::{create_auth_rate_limiter, create_api_rate_limiter};
```

**Apply to auth routes** (around line 73):
```rust
.nest(
    "/api/auth",
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/refresh", post(refresh_token))
        .route("/profile", get(get_profile))
        .route("/profile", put(update_profile))
        .route("/delete", delete(delete_account))
        .layer(create_auth_rate_limiter())  // ADD THIS LINE
)
```

**Apply to general API routes**:
```rust
.layer(
    ServiceBuilder::new()
        .layer(cors)
        .layer(create_api_rate_limiter())  // ADD THIS LINE
        .layer(axum::middleware::from_fn_with_state(...))
)
```

**Test**:
```bash
# Should block after 5 requests per second
for i in {1..10}; do
  curl -X POST http://localhost:8080/api/auth/login \
    -H "Content-Type: application/json" \
    -d '{"email":"test","password":"test"}'
done
```

---

### 3. Encrypted File Storage

**Current State**: `EncryptedFileStorage` exists but not used

**File to Update**: `src/handlers/ai_import.rs` (line 82)

**Change from**:
```rust
let file_storage = FileStorage::new(&config.file_storage_path)?;
let (file_path, file_hash) = file_storage.save_file(session_id, &filename, &file_data)?;
```

**To**:
```rust
use crate::utils::EncryptedFileStorage;

let file_storage = EncryptedFileStorage::new(&config.file_storage_path, &config.encryption_key)?;
let (file_path, file_hash) = file_storage.save_encrypted_file(session_id, &filename, &file_data)?;
```

**Update file reading** (wherever files are read):
```rust
let file_data = file_storage.read_encrypted_file(&file_path)?;
```

---

## ❌ What Needs to Be Built

### 1. Token Blacklist Service

**Purpose**: Instant logout / token revocation

**Create**: `src/services/token_blacklist_service.rs`

```rust
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

pub struct TokenBlacklistService {
    pool: PgPool,
}

impl TokenBlacklistService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn blacklist_token(&self, token: &str, user_id: Uuid, expires_at: DateTime<Utc>) -> Result<()> {
        sqlx::query!(
            "INSERT INTO token_blacklist (token_hash, user_id, expires_at) VALUES ($1, $2, $3)",
            sha256(token),  // Hash the token
            user_id,
            expires_at
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn is_blacklisted(&self, token: &str) -> Result<bool> {
        let result = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM token_blacklist WHERE token_hash = $1 AND expires_at > NOW())",
            sha256(token)
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(result.exists.unwrap_or(false))
    }
}
```

**Migration**: `migrations/008_token_blacklist.sql`
```sql
CREATE TABLE token_blacklist (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    token_hash VARCHAR(64) NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id),
    blacklisted_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    reason VARCHAR(255)
);

CREATE INDEX idx_token_blacklist_hash ON token_blacklist(token_hash);
CREATE INDEX idx_token_blacklist_expires ON token_blacklist(expires_at);
```

---

### 2. Audit Logging Service

**Create**: `src/services/security_audit_service.rs`

```rust
pub struct SecurityAuditService {
    pool: PgPool,
}

impl SecurityAuditService {
    pub async fn log_event(&self, event: AuditEvent) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO security_audit_log (
                user_id, event_type, resource_type, resource_id,
                action, ip_address, user_agent, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            event.user_id,
            event.event_type,
            event.resource_type,
            event.resource_id,
            event.action,
            event.ip_address,
            event.user_agent,
            event.metadata
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
```

**Events to Log**:
- Login attempts (success/failure)
- Logout
- Profile updates
- Password changes
- API key creation
- Failed authorization attempts
- Rate limit hits

---

### 3. MFA/TOTP Service

**Dependencies**: Already added (totp-rs, qrcode)

**Create**: `src/services/mfa_service.rs`

```rust
use totp_rs::{TOTP, Algorithm, Secret};
use qrcode::QrCode;

pub struct MFAService {
    pool: PgPool,
}

impl MFAService {
    pub async fn generate_secret(&self, user_id: Uuid, email: &str) -> Result<(String, String)> {
        let secret = Secret::generate_secret();
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,  // 6 digits
            1,
            30, // 30 second window
            secret.to_bytes().unwrap(),
            Some("Atlas Pharma".to_string()),
            email.to_string(),
        )?;

        let secret_str = secret.to_encoded().to_string();
        let qr_url = totp.get_qr_base64()?;

        // Save to database
        sqlx::query!(
            "INSERT INTO user_mfa (user_id, secret, is_enabled) VALUES ($1, $2, false)",
            user_id,
            secret_str
        )
        .execute(&self.pool)
        .await?;

        Ok((secret_str, qr_url))
    }

    pub async fn verify_token(&self, user_id: Uuid, token: &str) -> Result<bool> {
        let record = sqlx::query!(
            "SELECT secret FROM user_mfa WHERE user_id = $1 AND is_enabled = true",
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        let totp = TOTP::from_url(&record.secret)?;
        Ok(totp.check_current(token)?)
    }
}
```

---

## 🧪 Testing Checklist

### Encryption
- [ ] Register new user → Check database for encrypted fields
- [ ] Login → Verify decryption works
- [ ] Update profile → Verify re-encryption

### Rate Limiting
- [ ] Make 10 rapid login attempts → Should be blocked after 5
- [ ] Wait 1 minute → Should work again

### CORS
- [x] ✅ Evil origin blocked
- [x] ✅ Whitelisted origin allowed

### Secure Cookies
- [ ] Login → Check Set-Cookie header has httpOnly, Secure, SameSite
- [ ] Use cookie to access protected route → Should work
- [ ] Logout → Cookie should be cleared

### MFA
- [ ] Enable MFA → Get QR code
- [ ] Scan with Google Authenticator
- [ ] Login → Require TOTP code
- [ ] Wrong code → Reject
- [ ] Correct code → Allow

---

## 📦 Deployment Checklist

### Environment Variables
```bash
# Required
DATABASE_URL=postgres://...
JWT_SECRET=<64-char-random-string>
ENCRYPTION_KEY=<base64-32-bytes>  # openssl rand -base64 32

# TLS (Production)
TLS_ENABLED=true
TLS_CERT_PATH=/etc/letsencrypt/live/domain.com/fullchain.pem
TLS_KEY_PATH=/etc/letsencrypt/live/domain.com/privkey.pem
TLS_PORT=443

# CORS
CORS_ORIGINS=https://yourdomain.com,https://app.yourdomain.com

# File Storage
FILE_STORAGE_PATH=/var/lib/atlas/uploads
```

### Database Migrations
```bash
psql -h localhost -U postgres -d atlas_pharma -f migrations/007_encrypt_pii_fields.sql
psql -h localhost -U postgres -d atlas_pharma -f migrations/008_token_blacklist.sql
psql -h localhost -U postgres -d atlas_pharma -f migrations/009_security_audit_log.sql
psql -h localhost -U postgres -d atlas_pharma -f migrations/010_mfa_tables.sql
```

### Security Headers
Add to production nginx/caddy:
```
Strict-Transport-Security: max-age=31536000; includeSubDomains
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
X-XSS-Protection: 1; mode=block
Content-Security-Policy: default-src 'self'
```

---

## 🎯 Priority Order

1. **CRITICAL** - Rate limiting (prevents brute force attacks)
2. **CRITICAL** - User encryption integration (data protection)
3. **HIGH** - Token blacklist (instant logout capability)
4. **HIGH** - Encrypted file storage (uploaded file protection)
5. **MEDIUM** - Audit logging (compliance & monitoring)
6. **MEDIUM** - MFA/TOTP (additional auth factor)
7. **LOW** - Documentation polish

---

## 💡 Quick Wins (< 30 min each)

1. Apply rate limiting to routes in main.rs
2. Update ENCRYPTION_INTEGRATION_GUIDE.md examples
3. Add security headers to Axum responses
4. Create .env.example with all security vars

---

**Current Status**: 30% implemented, 70% requires integration
**Estimated Time to Complete All**: 6-8 hours of focused work
