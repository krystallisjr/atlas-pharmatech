-- Migration: Encrypted-Only Columns with Searchable Hashes
-- Purpose: Remove plaintext PII, use only encrypted columns + hashes for queries
-- Security: Production-ready encryption at rest

-- Add hash columns for searchable fields
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS email_hash VARCHAR(64);

-- Create index on email hash for fast lookups
CREATE INDEX idx_users_email_hash ON users(email_hash) WHERE email_hash IS NOT NULL;

-- Populate email_hash for existing users
UPDATE users
SET email_hash = encode(sha256(email::bytea), 'hex')
WHERE email IS NOT NULL;

-- Make email_hash unique (after populating)
ALTER TABLE users ADD CONSTRAINT users_email_hash_unique UNIQUE (email_hash);

-- Add comments for documentation
COMMENT ON COLUMN users.email_hash IS 'SHA-256 hash of email for secure lookups without exposing plaintext';
COMMENT ON COLUMN users.email_encrypted IS 'AES-256-GCM encrypted email - only decrypted in application layer';
COMMENT ON COLUMN users.contact_person_encrypted IS 'AES-256-GCM encrypted contact person name';
COMMENT ON COLUMN users.phone_encrypted IS 'AES-256-GCM encrypted phone number';
COMMENT ON COLUMN users.address_encrypted IS 'AES-256-GCM encrypted address';
COMMENT ON COLUMN users.license_number_encrypted IS 'AES-256-GCM encrypted license number';

-- Note: Plaintext columns (email, contact_person, phone, address, license_number) will be phased out
-- For now they remain for backwards compatibility during migration
-- TODO: Drop plaintext columns after confirming all code uses encrypted columns
