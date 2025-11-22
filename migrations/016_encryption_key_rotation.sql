-- ============================================================================
-- Encryption Key Rotation Migration
-- ============================================================================
--
-- 🔒 SECURITY: Issue #16 - Encryption Key Rotation
--
-- Implements envelope encryption with automatic key rotation
--
-- Architecture:
-- - Master Key (KEK) - Stored in environment variable
-- - Data Encryption Keys (DEK) - Stored encrypted in database, rotated every 90 days
-- - Actual Data - Encrypted with current DEK
--
-- Features:
-- - 90-day rotation lifecycle
-- - Version management (incremental key versions)
-- - Active/deprecated status tracking
-- - Automatic rotation triggers
-- - Audit trail of rotations
--
-- ============================================================================

-- Key status enum
CREATE TYPE encryption_key_status AS ENUM ('active', 'deprecated', 'rotated');

-- ============================================================================
-- TABLE: data_encryption_keys
-- ============================================================================
-- Stores encrypted Data Encryption Keys (DEKs) for envelope encryption

CREATE TABLE IF NOT EXISTS data_encryption_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),

    -- Version management
    key_version INTEGER NOT NULL UNIQUE,

    -- Encrypted DEK (encrypted with Master Key)
    encrypted_key TEXT NOT NULL,

    -- Key lifecycle
    status encryption_key_status NOT NULL DEFAULT 'active',
    is_active BOOLEAN NOT NULL DEFAULT true,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    valid_until TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '90 days',
    deprecated_at TIMESTAMPTZ,
    rotated_at TIMESTAMPTZ,

    -- Metadata
    rotated_by UUID REFERENCES users(id),
    rotation_reason TEXT

    -- Note: Single active key constraint enforced by trigger (see below)
    -- CHECK constraints cannot use subqueries in PostgreSQL
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_data_encryption_keys_status ON data_encryption_keys(status);
CREATE INDEX IF NOT EXISTS idx_data_encryption_keys_active ON data_encryption_keys(is_active) WHERE is_active = true;
CREATE INDEX IF NOT EXISTS idx_data_encryption_keys_version ON data_encryption_keys(key_version DESC);
CREATE INDEX IF NOT EXISTS idx_data_encryption_keys_valid_until ON data_encryption_keys(valid_until);

COMMENT ON TABLE data_encryption_keys IS 'Encrypted Data Encryption Keys for envelope encryption pattern';
COMMENT ON COLUMN data_encryption_keys.encrypted_key IS 'DEK encrypted with Master Key (KEK) from environment';
COMMENT ON COLUMN data_encryption_keys.valid_until IS 'Key expires 90 days after creation';
COMMENT ON COLUMN data_encryption_keys.is_active IS 'Only one key can be active at a time';

-- ============================================================================
-- TABLE: key_rotation_log
-- ============================================================================
-- Audit trail of all key rotation events

CREATE TABLE IF NOT EXISTS key_rotation_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),

    -- Rotation details
    old_key_id UUID REFERENCES data_encryption_keys(id),
    old_key_version INTEGER,
    new_key_id UUID REFERENCES data_encryption_keys(id),
    new_key_version INTEGER,

    -- Metadata
    rotated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    rotated_by UUID REFERENCES users(id),
    rotation_reason TEXT,
    rotation_type TEXT DEFAULT 'scheduled', -- 'scheduled', 'manual', 'emergency'

    -- Success/failure
    success BOOLEAN NOT NULL DEFAULT true,
    error_message TEXT
);

CREATE INDEX IF NOT EXISTS idx_key_rotation_log_rotated_at ON key_rotation_log(rotated_at DESC);
CREATE INDEX IF NOT EXISTS idx_key_rotation_log_new_key ON key_rotation_log(new_key_id);

COMMENT ON TABLE key_rotation_log IS 'Audit trail of encryption key rotations';
COMMENT ON COLUMN key_rotation_log.rotation_type IS 'scheduled (90-day), manual (admin), emergency (compromise)';

-- ============================================================================
-- FUNCTION: Get next key version
-- ============================================================================

CREATE OR REPLACE FUNCTION get_next_key_version()
RETURNS INTEGER AS $$
DECLARE
    v_max_version INTEGER;
BEGIN
    SELECT COALESCE(MAX(key_version), 0) INTO v_max_version
    FROM data_encryption_keys;

    RETURN v_max_version + 1;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION get_next_key_version IS 'Returns next available key version number';

-- ============================================================================
-- FUNCTION: Check if key rotation is needed
-- ============================================================================

CREATE OR REPLACE FUNCTION is_key_rotation_needed()
RETURNS TABLE (
    needs_rotation BOOLEAN,
    days_until_expiry INTEGER,
    current_key_id UUID,
    current_key_version INTEGER
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        (valid_until < NOW() + INTERVAL '7 days') as needs_rotation,
        EXTRACT(DAY FROM (valid_until - NOW()))::INTEGER as days_until_expiry,
        id as current_key_id,
        key_version as current_key_version
    FROM data_encryption_keys
    WHERE is_active = true
    ORDER BY key_version DESC
    LIMIT 1;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION is_key_rotation_needed IS 'Checks if rotation is needed (within 7 days of expiry)';

-- ============================================================================
-- FUNCTION: Get active key
-- ============================================================================

CREATE OR REPLACE FUNCTION get_active_encryption_key()
RETURNS TABLE (
    id UUID,
    key_version INTEGER,
    encrypted_key TEXT,
    created_at TIMESTAMPTZ,
    valid_until TIMESTAMPTZ
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        dek.id,
        dek.key_version,
        dek.encrypted_key,
        dek.created_at,
        dek.valid_until
    FROM data_encryption_keys dek
    WHERE dek.is_active = true
    LIMIT 1;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION get_active_encryption_key IS 'Returns currently active encryption key';

-- ============================================================================
-- TRIGGER: Prevent multiple active keys
-- ============================================================================

CREATE OR REPLACE FUNCTION enforce_single_active_key()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.is_active = true THEN
        -- Deactivate all other keys
        UPDATE data_encryption_keys
        SET is_active = false
        WHERE id != NEW.id AND is_active = true;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_enforce_single_active_key ON data_encryption_keys;
CREATE TRIGGER trigger_enforce_single_active_key
    BEFORE INSERT OR UPDATE ON data_encryption_keys
    FOR EACH ROW
    WHEN (NEW.is_active = true)
    EXECUTE FUNCTION enforce_single_active_key();

COMMENT ON FUNCTION enforce_single_active_key IS 'Ensures only one key is active at a time';

-- ============================================================================
-- TRIGGER: Log key rotation
-- ============================================================================

CREATE OR REPLACE FUNCTION log_key_rotation()
RETURNS TRIGGER AS $$
BEGIN
    -- When a key is marked as deprecated, log it
    IF OLD.status != 'deprecated' AND NEW.status = 'deprecated' THEN
        INSERT INTO key_rotation_log (
            old_key_id,
            old_key_version,
            rotation_reason,
            rotation_type
        ) VALUES (
            NEW.id,
            NEW.key_version,
            NEW.rotation_reason,
            'scheduled'
        );
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_log_key_rotation ON data_encryption_keys;
CREATE TRIGGER trigger_log_key_rotation
    AFTER UPDATE ON data_encryption_keys
    FOR EACH ROW
    EXECUTE FUNCTION log_key_rotation();

-- ============================================================================
-- VIEW: Key rotation status
-- ============================================================================

CREATE OR REPLACE VIEW key_rotation_status AS
SELECT
    dek.id,
    dek.key_version,
    dek.status,
    dek.is_active,
    dek.created_at,
    dek.valid_until,
    EXTRACT(DAY FROM (dek.valid_until - NOW()))::INTEGER as days_until_expiry,
    CASE
        WHEN dek.valid_until < NOW() THEN 'EXPIRED'
        WHEN dek.valid_until < NOW() + INTERVAL '7 days' THEN 'ROTATION_RECOMMENDED'
        WHEN dek.valid_until < NOW() + INTERVAL '30 days' THEN 'ROTATION_SOON'
        ELSE 'OK'
    END as rotation_status,
    dek.deprecated_at,
    dek.rotated_at
FROM data_encryption_keys dek
ORDER BY dek.key_version DESC;

COMMENT ON VIEW key_rotation_status IS 'Quick overview of encryption key status and rotation needs';

-- ============================================================================
-- SECURITY: Permissions
-- ============================================================================

-- Restrict access to encryption keys (application role only)
-- Uncomment and modify based on your database role setup

-- REVOKE ALL ON data_encryption_keys FROM PUBLIC;
-- GRANT SELECT, INSERT, UPDATE ON data_encryption_keys TO atlas_app;
-- GRANT SELECT ON key_rotation_log TO atlas_app;

-- ============================================================================
-- MONITORING QUERIES
-- ============================================================================

COMMENT ON SCHEMA public IS '
-- Check if rotation is needed:
SELECT * FROM is_key_rotation_needed();

-- View all key rotation history:
SELECT
    kr.rotated_at,
    kr.old_key_version,
    kr.new_key_version,
    kr.rotation_type,
    kr.rotation_reason,
    u.email as rotated_by_email
FROM key_rotation_log kr
LEFT JOIN users u ON kr.rotated_by = u.id
ORDER BY kr.rotated_at DESC;

-- Current key status:
SELECT * FROM key_rotation_status WHERE is_active = true;

-- All keys by version:
SELECT key_version, status, created_at, valid_until, days_until_expiry, rotation_status
FROM key_rotation_status
ORDER BY key_version DESC;
';

-- ============================================================================
-- INITIAL SETUP NOTES
-- ============================================================================

DO $$
BEGIN
    RAISE NOTICE '✅ Encryption Key Rotation migration completed successfully';
    RAISE NOTICE '   ';
    RAISE NOTICE '   📋 IMPORTANT: Initial Setup Required';
    RAISE NOTICE '   ';
    RAISE NOTICE '   The application will automatically create the first encryption key';
    RAISE NOTICE '   on first startup using the ENCRYPTION_KEY environment variable.';
    RAISE NOTICE '   ';
    RAISE NOTICE '   To check key status:';
    RAISE NOTICE '   SELECT * FROM key_rotation_status;';
    RAISE NOTICE '   ';
    RAISE NOTICE '   To check if rotation is needed:';
    RAISE NOTICE '   SELECT * FROM is_key_rotation_needed();';
    RAISE NOTICE '   ';
    RAISE NOTICE '   🔐 Key rotation will be automatic every 90 days';
END $$;
