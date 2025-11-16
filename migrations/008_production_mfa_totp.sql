-- ============================================================================
-- PRODUCTION-GRADE MFA/TOTP SYSTEM
-- Multi-Factor Authentication with Time-Based One-Time Passwords
-- Compliance: SOC 2, PCI-DSS, NIST 800-63B
-- ============================================================================

-- Add MFA columns to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS mfa_enabled BOOLEAN DEFAULT FALSE NOT NULL;
ALTER TABLE users ADD COLUMN IF NOT EXISTS mfa_secret_encrypted TEXT; -- Encrypted TOTP secret
ALTER TABLE users ADD COLUMN IF NOT EXISTS mfa_backup_codes_encrypted TEXT[]; -- Encrypted backup codes
ALTER TABLE users ADD COLUMN IF NOT EXISTS mfa_enabled_at TIMESTAMPTZ;

-- MFA enrollment/setup tracking
CREATE TABLE IF NOT EXISTS mfa_enrollment_log (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    action VARCHAR(50) NOT NULL, -- enrolled, disabled, backup_code_used, device_added
    device_name VARCHAR(255), -- "iPhone 13", "Work Laptop", etc.

    ip_address INET,
    user_agent TEXT,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_mfa_enrollment_user ON mfa_enrollment_log(user_id);
CREATE INDEX idx_mfa_enrollment_created ON mfa_enrollment_log(created_at DESC);

-- Trusted devices (for "Remember this device" feature)
CREATE TABLE IF NOT EXISTS mfa_trusted_devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    device_fingerprint VARCHAR(255) NOT NULL, -- Hashed device identifier
    device_name VARCHAR(255), -- User-friendly name
    device_type VARCHAR(50), -- mobile, desktop, tablet

    -- Device metadata
    ip_address INET,
    user_agent TEXT,
    browser VARCHAR(100),
    os VARCHAR(100),

    -- Trust management
    trusted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL, -- Default 30 days
    last_used_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_active BOOLEAN DEFAULT TRUE NOT NULL,

    -- Security
    revoked_at TIMESTAMPTZ,
    revoked_reason TEXT,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_mfa_devices_user ON mfa_trusted_devices(user_id);
CREATE INDEX idx_mfa_devices_fingerprint ON mfa_trusted_devices(device_fingerprint);
CREATE INDEX idx_mfa_devices_active ON mfa_trusted_devices(user_id, is_active) WHERE is_active = TRUE;
CREATE INDEX idx_mfa_devices_expires ON mfa_trusted_devices(expires_at) WHERE is_active = TRUE;

-- MFA verification attempts (for rate limiting and security monitoring)
CREATE TABLE IF NOT EXISTS mfa_verification_log (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    verification_type VARCHAR(50) NOT NULL, -- totp, backup_code, trusted_device
    verification_result VARCHAR(50) NOT NULL, -- success, invalid_code, expired, rate_limited

    -- Attempt details
    code_provided VARCHAR(20), -- Hashed for security
    ip_address INET,
    user_agent TEXT,

    -- Rate limiting tracking
    attempts_count INTEGER DEFAULT 1,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_mfa_verify_user ON mfa_verification_log(user_id);
CREATE INDEX idx_mfa_verify_created ON mfa_verification_log(created_at DESC);
CREATE INDEX idx_mfa_verify_result ON mfa_verification_log(verification_result);
CREATE INDEX idx_mfa_verify_rate_limit ON mfa_verification_log(user_id, created_at DESC)
    WHERE verification_result = 'invalid_code';

-- Function to clean up expired trusted devices
CREATE OR REPLACE FUNCTION cleanup_expired_mfa_devices()
RETURNS TABLE(expired_count BIGINT) AS $$
DECLARE
    rows_updated BIGINT;
BEGIN
    UPDATE mfa_trusted_devices
    SET is_active = FALSE,
        revoked_at = NOW(),
        revoked_reason = 'expired'
    WHERE is_active = TRUE
        AND expires_at < NOW();

    GET DIAGNOSTICS rows_updated = ROW_COUNT;
    RETURN QUERY SELECT rows_updated;
END;
$$ LANGUAGE plpgsql;

-- Function to check MFA verification rate limiting
CREATE OR REPLACE FUNCTION check_mfa_rate_limit(
    p_user_id UUID,
    p_window_minutes INTEGER DEFAULT 5,
    p_max_attempts INTEGER DEFAULT 5
)
RETURNS BOOLEAN AS $$
DECLARE
    attempt_count INTEGER;
BEGIN
    SELECT COUNT(*)
    INTO attempt_count
    FROM mfa_verification_log
    WHERE user_id = p_user_id
        AND verification_result = 'invalid_code'
        AND created_at > NOW() - (p_window_minutes || ' minutes')::INTERVAL;

    RETURN attempt_count < p_max_attempts;
END;
$$ LANGUAGE plpgsql;

-- View: MFA security summary per user
CREATE OR REPLACE VIEW mfa_security_summary AS
SELECT
    u.id as user_id,
    u.company_name,
    u.mfa_enabled,
    u.mfa_enabled_at,

    -- Backup codes status
    CASE
        WHEN u.mfa_backup_codes_encrypted IS NULL THEN 0
        ELSE COALESCE(array_length(u.mfa_backup_codes_encrypted, 1), 0)
    END as backup_codes_remaining,

    -- Trusted devices count
    (SELECT COUNT(*) FROM mfa_trusted_devices
     WHERE user_id = u.id AND is_active = TRUE) as active_trusted_devices,

    -- Failed verification attempts (last 24h)
    (SELECT COUNT(*) FROM mfa_verification_log
     WHERE user_id = u.id
       AND verification_result = 'invalid_code'
       AND created_at > NOW() - INTERVAL '24 hours') as failed_attempts_24h,

    -- Last successful MFA verification
    (SELECT MAX(created_at) FROM mfa_verification_log
     WHERE user_id = u.id
       AND verification_result = 'success') as last_successful_verification,

    u.created_at as account_created_at
FROM users u;

-- View: Recent MFA failures (security monitoring)
CREATE OR REPLACE VIEW mfa_recent_failures AS
SELECT
    v.user_id,
    u.company_name,
    v.verification_type,
    v.verification_result,
    v.ip_address,
    v.created_at,
    COUNT(*) OVER (
        PARTITION BY v.user_id, DATE_TRUNC('hour', v.created_at)
    ) as failures_this_hour
FROM mfa_verification_log v
JOIN users u ON u.id = v.user_id
WHERE v.verification_result IN ('invalid_code', 'rate_limited')
    AND v.created_at > NOW() - INTERVAL '24 hours'
ORDER BY v.created_at DESC;

-- Comments for documentation
COMMENT ON TABLE mfa_enrollment_log IS 'Tracks MFA enrollment, disablement, and usage events';
COMMENT ON TABLE mfa_trusted_devices IS 'Stores trusted devices for "Remember this device" functionality';
COMMENT ON TABLE mfa_verification_log IS 'Logs all MFA verification attempts for security monitoring';
COMMENT ON COLUMN users.mfa_secret_encrypted IS 'Encrypted TOTP secret key (base32 encoded, encrypted with master key)';
COMMENT ON COLUMN users.mfa_backup_codes_encrypted IS 'Array of encrypted backup codes (each code encrypted separately)';
COMMENT ON COLUMN mfa_trusted_devices.device_fingerprint IS 'Hashed device identifier (browser fingerprint + user agent hash)';
COMMENT ON COLUMN mfa_trusted_devices.expires_at IS 'Trusted device token expiration (default 30 days from creation)';
COMMENT ON FUNCTION check_mfa_rate_limit IS 'Returns TRUE if user has not exceeded MFA verification rate limit';

-- Security: Prevent direct updates to MFA secret (must go through service layer)
CREATE OR REPLACE FUNCTION prevent_direct_mfa_secret_update()
RETURNS TRIGGER AS $$
BEGIN
    -- Allow updates from service layer (identified by comment in transaction)
    IF current_setting('app.bypass_mfa_trigger', TRUE) = 'true' THEN
        RETURN NEW;
    END IF;

    -- Block direct updates to MFA secret
    IF OLD.mfa_secret_encrypted IS DISTINCT FROM NEW.mfa_secret_encrypted THEN
        RAISE EXCEPTION 'Direct updates to mfa_secret_encrypted are not allowed. Use MFA service endpoints.';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER mfa_secret_protection
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION prevent_direct_mfa_secret_update();

-- Add admin flag for audit log access (RBAC)
ALTER TABLE users ADD COLUMN IF NOT EXISTS is_admin BOOLEAN DEFAULT FALSE NOT NULL;

CREATE INDEX idx_users_admin ON users(id) WHERE is_admin = TRUE;

COMMENT ON COLUMN users.is_admin IS 'Admin users can access audit logs and perform administrative actions';

-- Audit log: Track MFA enrollment changes
CREATE OR REPLACE FUNCTION audit_mfa_changes()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'UPDATE' THEN
        -- MFA enabled
        IF OLD.mfa_enabled = FALSE AND NEW.mfa_enabled = TRUE THEN
            INSERT INTO mfa_enrollment_log (user_id, action)
            VALUES (NEW.id, 'enrolled');
        END IF;

        -- MFA disabled
        IF OLD.mfa_enabled = TRUE AND NEW.mfa_enabled = FALSE THEN
            INSERT INTO mfa_enrollment_log (user_id, action)
            VALUES (NEW.id, 'disabled');
        END IF;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER audit_mfa_changes_trigger
    AFTER UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION audit_mfa_changes();
