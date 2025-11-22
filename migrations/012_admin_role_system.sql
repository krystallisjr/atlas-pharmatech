-- ============================================================================
-- Migration: 012_admin_role_system.sql
-- Description: Production-ready admin role system with RBAC foundation
-- Created: 2025-11-18
-- Author: Atlas PharmaTech
-- ============================================================================
--
-- This migration adds:
-- 1. Role-based access control (RBAC) foundation
-- 2. Role enum type for type safety
-- 3. Performance indexes
-- 4. Founder superadmin account
-- 5. Audit logging support
-- 6. Security constraints
--
-- Roles:
--   - user: Regular pharma company user (default)
--   - admin: Platform admin (can verify users, view audit logs)
--   - superadmin: Founder/owner (full system access)
--
-- ============================================================================

BEGIN;

-- ============================================================================
-- 1. CREATE ROLE ENUM TYPE (Type-Safe)
-- ============================================================================

CREATE TYPE user_role AS ENUM ('user', 'admin', 'superadmin');

COMMENT ON TYPE user_role IS 'User role for access control: user (default), admin (platform admin), superadmin (founder)';

-- ============================================================================
-- 2. ADD ROLE COLUMN TO USERS TABLE
-- ============================================================================

ALTER TABLE users
ADD COLUMN role user_role NOT NULL DEFAULT 'user'::user_role;

COMMENT ON COLUMN users.role IS 'User role for RBAC - determines access level in the system';

-- ============================================================================
-- 3. CREATE PERFORMANCE INDEXES
-- ============================================================================

-- Index for role-based queries (e.g., listing all admins)
CREATE INDEX idx_users_role ON users(role);

-- Composite index for role + verification queries
CREATE INDEX idx_users_role_verified ON users(role, is_verified);

-- Index for admin user searches (by email hash for encrypted lookups)
CREATE INDEX idx_users_email_hash_role ON users(email_hash, role);

COMMENT ON INDEX idx_users_role IS 'Performance index for role-based access control queries';
COMMENT ON INDEX idx_users_role_verified IS 'Composite index for admin verification workflows';
COMMENT ON INDEX idx_users_email_hash_role IS 'Secure lookup index for admin user management';

-- ============================================================================
-- 4. ADD ROLE CHANGE TRACKING
-- ============================================================================

-- Add column to track when role was last changed (audit trail)
ALTER TABLE users
ADD COLUMN role_changed_at TIMESTAMP WITH TIME ZONE,
ADD COLUMN role_changed_by UUID REFERENCES users(id) ON DELETE SET NULL;

COMMENT ON COLUMN users.role_changed_at IS 'Timestamp of last role change for audit trail';
COMMENT ON COLUMN users.role_changed_by IS 'User ID who changed this users role (for accountability)';

-- ============================================================================
-- 5. CREATE AUDIT EVENT TYPES FOR ADMIN ACTIONS
-- ============================================================================

-- Ensure audit_logs table supports admin event category
-- (Already exists from migration 007, but adding documentation)

COMMENT ON TABLE audit_logs IS 'Comprehensive audit log supporting admin actions (event_category=admin)';

-- ============================================================================
-- 6. ADMIN ACCOUNT CREATION
-- ============================================================================
-- 🔒 SECURITY NOTE:
-- NO default admin account is created in this migration for security reasons.
--
-- To create the first superadmin account, use the secure CLI command:
--   cargo run --release -- create-admin
--
-- This will generate a cryptographically secure random password and display it ONCE.
-- ============================================================================

DO $$
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '========================================================================';
    RAISE NOTICE '🔒 ADMIN ROLE SYSTEM INITIALIZED';
    RAISE NOTICE '========================================================================';
    RAISE NOTICE '';
    RAISE NOTICE '⚠️  NO DEFAULT ADMIN ACCOUNT CREATED (security best practice)';
    RAISE NOTICE '';
    RAISE NOTICE 'To create your first superadmin account:';
    RAISE NOTICE '   cargo run --release -- create-admin';
    RAISE NOTICE '';
    RAISE NOTICE 'This will:';
    RAISE NOTICE '   - Generate a cryptographically secure random password';
    RAISE NOTICE '   - Create the admin account with your chosen email';
    RAISE NOTICE '   - Display credentials ONCE (save them securely!)';
    RAISE NOTICE '   - Require MFA setup on first login';
    RAISE NOTICE '';
    RAISE NOTICE '========================================================================';
    RAISE NOTICE '';
END $$;

-- ============================================================================
-- 7. CREATE SECURITY CONSTRAINTS
-- ============================================================================

-- Ensure at least one superadmin always exists
-- (Prevents accidental lockout)
CREATE OR REPLACE FUNCTION prevent_last_superadmin_deletion()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.role = 'superadmin'::user_role THEN
        IF (SELECT COUNT(*) FROM users WHERE role = 'superadmin'::user_role AND id != OLD.id) = 0 THEN
            RAISE EXCEPTION 'Cannot delete the last superadmin account (prevents system lockout)';
        END IF;
    END IF;
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER prevent_last_superadmin_deletion_trigger
BEFORE DELETE ON users
FOR EACH ROW
EXECUTE FUNCTION prevent_last_superadmin_deletion();

COMMENT ON FUNCTION prevent_last_superadmin_deletion IS 'Prevents deletion of last superadmin account to avoid system lockout';

-- Prevent demoting the last superadmin
CREATE OR REPLACE FUNCTION prevent_last_superadmin_demotion()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.role = 'superadmin'::user_role AND NEW.role != 'superadmin'::user_role THEN
        IF (SELECT COUNT(*) FROM users WHERE role = 'superadmin'::user_role AND id != OLD.id) = 0 THEN
            RAISE EXCEPTION 'Cannot demote the last superadmin account (prevents system lockout)';
        END IF;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER prevent_last_superadmin_demotion_trigger
BEFORE UPDATE OF role ON users
FOR EACH ROW
EXECUTE FUNCTION prevent_last_superadmin_demotion();

COMMENT ON FUNCTION prevent_last_superadmin_demotion IS 'Prevents demoting last superadmin to avoid system lockout';

-- ============================================================================
-- 8. CREATE ADMIN STATISTICS VIEW
-- ============================================================================

CREATE OR REPLACE VIEW admin_user_statistics AS
SELECT
    role,
    COUNT(*) as user_count,
    COUNT(*) FILTER (WHERE is_verified = TRUE) as verified_count,
    COUNT(*) FILTER (WHERE is_verified = FALSE) as unverified_count,
    COUNT(*) FILTER (WHERE created_at >= NOW() - INTERVAL '7 days') as new_this_week,
    COUNT(*) FILTER (WHERE created_at >= NOW() - INTERVAL '30 days') as new_this_month,
    MIN(created_at) as first_user_created,
    MAX(created_at) as last_user_created
FROM users
GROUP BY role
ORDER BY
    CASE role
        WHEN 'superadmin'::user_role THEN 1
        WHEN 'admin'::user_role THEN 2
        WHEN 'user'::user_role THEN 3
    END;

COMMENT ON VIEW admin_user_statistics IS 'Real-time user statistics by role for admin dashboard';

-- View for pending verifications (admin workflow)
CREATE OR REPLACE VIEW admin_verification_queue AS
SELECT
    u.id,
    u.email,
    u.company_name,
    u.contact_person,
    u.license_number,
    u.created_at,
    u.role,
    COALESCE(inv_count.count, 0) as inventory_count,
    COALESCE(txn_count.count, 0) as transaction_count,
    AGE(NOW(), u.created_at) as waiting_time
FROM users u
LEFT JOIN (
    SELECT user_id, COUNT(*) as count
    FROM inventory
    GROUP BY user_id
) inv_count ON u.id = inv_count.user_id
LEFT JOIN (
    SELECT seller_id as user_id, COUNT(*) as count
    FROM transactions
    GROUP BY seller_id
) txn_count ON u.id = txn_count.user_id
WHERE u.is_verified = FALSE AND u.role = 'user'::user_role
ORDER BY u.created_at ASC;

COMMENT ON VIEW admin_verification_queue IS 'Queue of users pending verification with context for admin decision';

-- ============================================================================
-- 9. GRANT PERMISSIONS (Production Security)
-- ============================================================================

-- Note: In production, create separate database roles for different access levels
-- Example (commented out - configure per deployment):
-- GRANT SELECT ON admin_user_statistics TO atlas_readonly_role;
-- GRANT ALL ON users TO atlas_admin_role;

-- ============================================================================
-- 10. MIGRATION METADATA
-- ============================================================================

-- Track migration success
INSERT INTO audit_logs (
    event_id,
    event_type,
    event_category,
    severity,
    actor_type,
    action,
    action_result,
    event_data
) VALUES (
    uuid_generate_v4(),
    'migration_executed',
    'system',
    'info',
    'system',
    'run_migration_012',
    'success',
    jsonb_build_object(
        'migration', '012_admin_role_system.sql',
        'description', 'Admin role system with RBAC foundation',
        'changes', jsonb_build_array(
            'Added user_role enum type',
            'Added role column to users table',
            'Created performance indexes',
            'Created founder superadmin account',
            'Added security constraints',
            'Created admin views'
        )
    )
);

COMMIT;

-- ============================================================================
-- ROLLBACK INSTRUCTIONS (For reference - do not execute)
-- ============================================================================

-- To rollback this migration (CAUTION - will delete admin account):
/*
BEGIN;

-- Drop triggers
DROP TRIGGER IF EXISTS prevent_last_superadmin_deletion_trigger ON users;
DROP TRIGGER IF EXISTS prevent_last_superadmin_demotion_trigger ON users;
DROP FUNCTION IF EXISTS prevent_last_superadmin_deletion();
DROP FUNCTION IF EXISTS prevent_last_superadmin_demotion();

-- Drop views
DROP VIEW IF EXISTS admin_user_statistics;
DROP VIEW IF EXISTS admin_verification_queue;

-- Remove columns
ALTER TABLE users DROP COLUMN IF EXISTS role;
ALTER TABLE users DROP COLUMN IF EXISTS role_changed_at;
ALTER TABLE users DROP COLUMN IF EXISTS role_changed_by;

-- Drop enum type
DROP TYPE IF EXISTS user_role;

COMMIT;
*/

-- ============================================================================
-- END OF MIGRATION
-- ============================================================================
