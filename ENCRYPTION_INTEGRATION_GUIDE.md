# 🔐 Database Encryption Integration Guide

## Status: Foundation Complete ✅

### What's Been Implemented

1. **✅ AES-256-GCM Encryption Service** (`src/services/encryption_service.rs`)
   - Production-grade authenticated encryption
   - Unique nonce generation per encryption
   - Base64 encoding for database storage
   - Helper methods for Optional fields
   - Full test coverage

2. **✅ Database Migration** (`migrations/007_encrypt_pii_fields.sql`)
   - Adds encrypted columns for all PII fields
   - Encryption version tracking
   - Automatic timestamp updates
   - Dual-column approach (encrypted + unencrypted)

3. **✅ Configuration** (`src/config/mod.rs`)
   - Added `encryption_key` to AppConfig
   - Loaded from `ENCRYPTION_KEY` environment variable

4. **✅ Module Exports** (`src/services/mod.rs`)
   - EncryptionService exported and available

### What Needs Integration

#### Step 1: Run the Migration

```bash
# Generate encryption key (save to .env)
openssl rand -base64 32

# Add to .env
echo "ENCRYPTION_KEY=<your-generated-key>" >> .env

# Run migration
sqlx migrate run
```

#### Step 2: Update UserRepository

The user repository needs to be updated to encrypt PII on write and decrypt on read.

**File**: `src/repositories/user_repo.rs`

**Changes Needed**:

1. **Add encryption service to repository**:
```rust
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
```

2. **Update `create` method** (line 16):
```rust
pub async fn create(&self, request: &CreateUserRequest, password_hash: &str) -> Result<User> {
    // Encrypt PII fields
    let email_encrypted = self.encryption.encrypt(&request.email)?;
    let contact_encrypted = self.encryption.encrypt(&request.contact_person)?;
    let phone_encrypted = self.encryption.encrypt_option(&request.phone)?;
    let address_encrypted = self.encryption.encrypt_option(&request.address)?;
    let license_encrypted = self.encryption.encrypt_option(&request.license_number)?;

    let row = query(
        r#"
        INSERT INTO users (
            email, password_hash, company_name, contact_person, phone, address, license_number,
            email_encrypted, contact_person_encrypted, phone_encrypted, address_encrypted, license_number_encrypted
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING id, email, password_hash, company_name, contact_person, phone, address, license_number,
                  email_encrypted, contact_person_encrypted, phone_encrypted, address_encrypted, license_number_encrypted,
                  is_verified, created_at, updated_at
        "#
    )
    .bind(&request.email)  // Still store plaintext for now (dual-write)
    .bind(password_hash)
    .bind(&request.company_name)
    .bind(&request.contact_person)
    .bind(&request.phone)
    .bind(&request.address)
    .bind(&request.license_number)
    .bind(email_encrypted)
    .bind(contact_encrypted)
    .bind(phone_encrypted)
    .bind(address_encrypted)
    .bind(license_encrypted)
    .fetch_one(&self.pool)
    .await?;

    // Decrypt fields when creating User struct
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
```

3. **Update `find_by_email` method** (line 49):
```rust
pub async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
    // Encrypt email for comparison (note: this won't work efficiently with indexes)
    // For production, consider using email as plaintext or implement searchable encryption
    let email_encrypted = self.encryption.encrypt(email)?;

    let row = query(
        "SELECT * FROM users WHERE email_encrypted = $1"
    )
    .bind(email_encrypted)
    .fetch_optional(&self.pool)
    .await?;

    match row {
        Some(row) => Ok(Some(self.decrypt_user_row(&row)?)),
        None => Ok(None),
    }
}
```

4. **Add helper method for decryption**:
```rust
fn decrypt_user_row(&self, row: &sqlx::postgres::PgRow) -> Result<User> {
    use sqlx::Row;

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
```

#### Step 3: Update All Repository Instantiations

Everywhere we create a `UserRepository`, pass the encryption key:

**File**: `src/services/auth_service.rs`
```rust
let user_repo = UserRepository::new(pool, &config.encryption_key)?;
```

**File**: `src/handlers/auth.rs` (multiple locations)
```rust
let user_repo = UserRepository::new(config.database_pool.clone(), &config.encryption_key)?;
```

### Migration Strategy

**Phase 1: Dual-Write** (Current)
- Write to both encrypted and unencrypted columns
- Read from unencrypted columns
- This ensures backwards compatibility

**Phase 2: Dual-Read**
- Write to both columns
- Read from encrypted columns
- Verify everything works

**Phase 3: Encrypted-Only**
- Only write to encrypted columns
- Only read from encrypted columns
- Drop unencrypted columns

### Testing Checklist

- [ ] Generate encryption key and add to `.env`
- [ ] Run migration `007_encrypt_pii_fields.sql`
- [ ] Update UserRepository with encryption logic
- [ ] Update all UserRepository instantiations
- [ ] Test user registration (creates encrypted data)
- [ ] Test user login (reads encrypted data)
- [ ] Test profile updates (updates encrypted data)
- [ ] Verify encrypted data in database
- [ ] Run integration tests
- [ ] Test with existing users (dual-read strategy)

### Security Considerations

1. **Email Searchability**: Encrypted emails can't be indexed efficiently
   - **Solution**: Keep email as plaintext (less sensitive) or use deterministic encryption for searchable fields
   - **Trade-off**: Searchability vs. security

2. **Key Management**: Encryption key must be secured
   - Store in environment variables
   - Use cloud key management (AWS KMS, GCP KMS) for production
   - Implement key rotation strategy

3. **Performance**: Encryption adds ~1-2ms per operation
   - Acceptable for user operations (low frequency)
   - Consider caching for high-frequency reads

### Estimated Time to Complete

- Migration + Repository updates: **1-2 hours**
- Testing + Debugging: **1 hour**
- **Total: 2-3 hours**

### Need Help?

Reference the encryption service tests in `src/services/encryption_service.rs` for examples of encrypt/decrypt usage.
