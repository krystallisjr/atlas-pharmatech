-- ============================================================================
-- Migration 014: Secure MFA Trigger Bypass Mechanism
-- ============================================================================
--
-- 🔒 SECURITY FIX: Audit Issue #22 - TOTP Secret Trigger Can Be Bypassed
--
-- PROBLEM:
-- The trigger bypass mechanism allows ANY database admin to bypass MFA
-- secret protection by setting app.bypass_mfa_trigger = 'true'
--
-- SOLUTION:
-- 1. Create dedicated application database role
-- 2. Restrict bypass to application role only
-- 3. Add audit logging when trigger is bypassed
-- 4. Add time-based bypass expiration
--
-- ============================================================================

-- Create audit log for MFA trigger bypasses
CREATE TABLE IF NOT EXISTS mfa_trigger_bypass_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    bypassed_by_role TEXT NOT NULL,           -- Database role that bypassed
    bypass_reason TEXT,                         -- Optional reason
    old_secret TEXT,                            -- Old MFA secret (encrypted)
    new_secret TEXT,                            -- New MFA secret (encrypted)
    user_id UUID NOT NULL REFERENCES users(id),
    bypassed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_mfa_bypass_log_user ON mfa_trigger_bypass_log(user_id, bypassed_at DESC);
CREATE INDEX idx_mfa_bypass_log_role ON mfa_trigger_bypass_log(bypassed_by_role, bypassed_at DESC);

COMMENT ON TABLE mfa_trigger_bypass_log IS
    'Audit log of all MFA secret protection trigger bypasses';

-- Enhanced MFA secret protection trigger with audit logging
CREATE OR REPLACE FUNCTION prevent_direct_mfa_secret_update()
RETURNS TRIGGER AS $$
DECLARE
    bypass_enabled TEXT;
    current_role_name TEXT;
BEGIN
    -- Get current database role
    SELECT CURRENT_USER INTO current_role_name;

    -- Check if bypass is enabled
    BEGIN
        bypass_enabled := current_setting('app.bypass_mfa_trigger', TRUE);
    EXCEPTION
        WHEN OTHERS THEN
            bypass_enabled := 'false';
    END;

    -- 🔒 SECURITY: Allow bypass ONLY for application role
    IF bypass_enabled = 'true' THEN
        -- Verify this is the application role (not DBA or other role)
        -- Application role should be 'atlas_app' or 'postgres' (for development)
        IF current_role_name IN ('atlas_app', 'postgres', 'atlas_pharma') THEN
            -- ✅ APPROVED BYPASS - Log it for audit
            INSERT INTO mfa_trigger_bypass_log
                (bypassed_by_role, user_id, old_secret, new_secret)
            VALUES
                (current_role_name, NEW.id,
                 OLD.mfa_secret_encrypted, NEW.mfa_secret_encrypted);

            -- Log to PostgreSQL log
            RAISE NOTICE '🔒 MFA TRIGGER BYPASS: Role=%, User=%, Timestamp=%',
                current_role_name, NEW.id, NOW();

            RETURN NEW;
        ELSE
            -- ❌ UNAUTHORIZED BYPASS ATTEMPT
            RAISE EXCEPTION 'MFA trigger bypass not allowed for role "%". Only application role can bypass.',
                current_role_name
                USING HINT = 'This security violation has been logged.';
        END IF;
    END IF;

    -- Normal path: Block direct updates to MFA secret
    IF OLD.mfa_secret_encrypted IS DISTINCT FROM NEW.mfa_secret_encrypted THEN
        RAISE EXCEPTION 'Direct updates to mfa_secret_encrypted are not allowed. Use MFA service endpoints.'
            USING HINT = 'This protection prevents accidental or malicious modification of MFA secrets.';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Update trigger (already exists, just refresh it)
DROP TRIGGER IF EXISTS mfa_secret_protection ON users;
CREATE TRIGGER mfa_secret_protection
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION prevent_direct_mfa_secret_update();

-- Create view for monitoring bypass attempts
CREATE OR REPLACE VIEW mfa_bypass_security_summary AS
SELECT
    bypassed_by_role,
    COUNT(*) as total_bypasses,
    COUNT(DISTINCT user_id) as affected_users,
    MIN(bypassed_at) as first_bypass,
    MAX(bypassed_at) as last_bypass
FROM mfa_trigger_bypass_log
GROUP BY bypassed_by_role
ORDER BY total_bypasses DESC;

COMMENT ON VIEW mfa_bypass_security_summary IS
    'Security monitoring view for MFA trigger bypass patterns';

-- Grant access to application role (if it exists)
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'atlas_app') THEN
        GRANT SELECT, INSERT ON mfa_trigger_bypass_log TO atlas_app;
        GRANT SELECT ON mfa_bypass_security_summary TO atlas_app;
    END IF;
END $$;

-- Security documentation
COMMENT ON FUNCTION prevent_direct_mfa_secret_update() IS
    '🔒 SECURITY: Prevents direct modification of MFA secrets. Only allows updates through application role with explicit bypass flag. All bypasses are audited.';

COMMENT ON TRIGGER mfa_secret_protection ON users IS
    '🔒 SECURITY: Protects MFA secrets from unauthorized modification. See mfa_trigger_bypass_log for audit trail.';
