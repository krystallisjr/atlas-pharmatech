-- Migration: Add encryption for PII fields
--
-- SECURITY: This migration adds encrypted versions of sensitive PII fields.
-- After running this migration, you must:
-- 1. Set ENCRYPTION_KEY in .env (generate with: openssl rand -base64 32)
-- 2. Run data migration script to encrypt existing data
-- 3. Update application code to use encrypted fields
-- 4. Drop unencrypted columns after verification (separate migration)

-- Step 1: Add new encrypted columns to users table
ALTER TABLE users
    ADD COLUMN email_encrypted TEXT,
    ADD COLUMN contact_person_encrypted TEXT,
    ADD COLUMN phone_encrypted TEXT,
    ADD COLUMN address_encrypted TEXT,
    ADD COLUMN license_number_encrypted TEXT;

-- Step 2: Create index on encrypted email for lookups (will be slower but necessary)
-- Note: We can't index encrypted data efficiently, so we keep email unencrypted
-- but add a flag to indicate it should be treated as sensitive
CREATE INDEX idx_users_email_encrypted ON users(email_encrypted) WHERE email_encrypted IS NOT NULL;

-- Step 3: Add metadata columns
ALTER TABLE users
    ADD COLUMN encryption_version INTEGER DEFAULT 1,
    ADD COLUMN last_encryption_update TIMESTAMP WITH TIME ZONE;

-- Step 4: Create function to update encryption timestamp
CREATE OR REPLACE FUNCTION update_encryption_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.last_encryption_update = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Step 5: Create trigger for encryption updates
CREATE TRIGGER trigger_update_encryption_timestamp
    BEFORE UPDATE OF email_encrypted, contact_person_encrypted, phone_encrypted,
                      address_encrypted, license_number_encrypted
    ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_encryption_timestamp();

-- Note: For now, we keep both encrypted and unencrypted columns
-- This allows gradual migration:
-- 1. Deploy code that writes to both columns
-- 2. Migrate existing data
-- 3. Deploy code that only reads from encrypted columns
-- 4. Drop unencrypted columns (in future migration)

COMMENT ON COLUMN users.email_encrypted IS 'AES-256-GCM encrypted email address';
COMMENT ON COLUMN users.contact_person_encrypted IS 'AES-256-GCM encrypted contact person name';
COMMENT ON COLUMN users.phone_encrypted IS 'AES-256-GCM encrypted phone number';
COMMENT ON COLUMN users.address_encrypted IS 'AES-256-GCM encrypted address';
COMMENT ON COLUMN users.license_number_encrypted IS 'AES-256-GCM encrypted license number';
COMMENT ON COLUMN users.encryption_version IS 'Version of encryption algorithm used (for key rotation)';
