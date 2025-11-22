-- OAuth/OIDC Provider Integration
-- Enables "Login with Google/GitHub/Microsoft" while maintaining existing email/password auth
-- Production-ready with proper indexing, constraints, and audit trails

-- ============================================================================
-- STEP 1: Add OAuth columns to users table
-- ============================================================================

-- OAuth provider information
ALTER TABLE users
ADD COLUMN IF NOT EXISTS oauth_provider VARCHAR(50),
ADD COLUMN IF NOT EXISTS oauth_provider_id VARCHAR(255),
ADD COLUMN IF NOT EXISTS oauth_email VARCHAR(255),
ADD COLUMN IF NOT EXISTS oauth_name VARCHAR(255),
ADD COLUMN IF NOT EXISTS oauth_avatar_url TEXT,
ADD COLUMN IF NOT EXISTS oauth_access_token_encrypted TEXT,
ADD COLUMN IF NOT EXISTS oauth_refresh_token_encrypted TEXT,
ADD COLUMN IF NOT EXISTS oauth_token_expires_at TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS oauth_linked_at TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS oauth_last_login_at TIMESTAMPTZ;

-- Allow password_hash to be NULL for OAuth-only users
-- Users can have both OAuth AND password (hybrid auth)
ALTER TABLE users ALTER COLUMN password_hash DROP NOT NULL;

-- ============================================================================
-- STEP 2: Create indexes for OAuth lookups
-- ============================================================================

-- Unique constraint: one provider account per user
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_oauth_provider_unique
ON users (oauth_provider, oauth_provider_id)
WHERE oauth_provider IS NOT NULL;

-- Index for finding users by OAuth email (for account linking)
CREATE INDEX IF NOT EXISTS idx_users_oauth_email
ON users (oauth_email)
WHERE oauth_email IS NOT NULL;

-- Index for provider-specific queries
CREATE INDEX IF NOT EXISTS idx_users_oauth_provider
ON users (oauth_provider)
WHERE oauth_provider IS NOT NULL;

-- ============================================================================
-- STEP 3: OAuth state management (CSRF protection)
-- ============================================================================

CREATE TABLE IF NOT EXISTS oauth_states (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- State token for CSRF protection
    state VARCHAR(255) UNIQUE NOT NULL,

    -- Nonce for ID token replay protection
    nonce VARCHAR(255) NOT NULL,

    -- PKCE code verifier (for enhanced security)
    pkce_code_verifier VARCHAR(128),

    -- Provider being authenticated against
    provider VARCHAR(50) NOT NULL,

    -- Where to redirect after auth (validated against whitelist)
    redirect_uri TEXT,

    -- Optional: link to existing user (for account linking flow)
    linking_user_id UUID REFERENCES users(id) ON DELETE CASCADE,

    -- IP address for security logging
    ip_address INET,

    -- User agent for security logging
    user_agent TEXT,

    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    expires_at TIMESTAMPTZ DEFAULT NOW() + INTERVAL '10 minutes' NOT NULL,
    used_at TIMESTAMPTZ  -- Set when state is consumed
);

-- Index for state lookup (most common operation)
CREATE INDEX IF NOT EXISTS idx_oauth_states_state ON oauth_states(state);

-- Index for cleanup of expired states
CREATE INDEX IF NOT EXISTS idx_oauth_states_expires ON oauth_states(expires_at) WHERE used_at IS NULL;

-- Index for security auditing by IP
CREATE INDEX IF NOT EXISTS idx_oauth_states_ip ON oauth_states(ip_address, created_at DESC);

-- ============================================================================
-- STEP 4: OAuth audit log for security compliance
-- ============================================================================

CREATE TABLE IF NOT EXISTS oauth_audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- User involved (may be NULL for failed attempts)
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,

    -- OAuth provider
    provider VARCHAR(50) NOT NULL,

    -- Event type
    event_type VARCHAR(50) NOT NULL, -- 'login', 'register', 'link', 'unlink', 'refresh', 'failed'

    -- Event details (JSON for flexibility)
    event_details JSONB DEFAULT '{}'::jsonb,

    -- Provider's user ID (for correlation)
    oauth_provider_id VARCHAR(255),

    -- Security context
    ip_address INET,
    user_agent TEXT,

    -- Success/failure
    success BOOLEAN NOT NULL DEFAULT true,
    error_message TEXT,

    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL
);

-- Index for user's OAuth history
CREATE INDEX IF NOT EXISTS idx_oauth_audit_user ON oauth_audit_log(user_id, created_at DESC) WHERE user_id IS NOT NULL;

-- Index for failed attempts (security monitoring)
CREATE INDEX IF NOT EXISTS idx_oauth_audit_failed ON oauth_audit_log(ip_address, created_at DESC) WHERE success = false;

-- Index for provider-specific auditing
CREATE INDEX IF NOT EXISTS idx_oauth_audit_provider ON oauth_audit_log(provider, created_at DESC);

-- ============================================================================
-- STEP 5: Supported OAuth providers configuration
-- ============================================================================

CREATE TABLE IF NOT EXISTS oauth_providers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Provider identifier (e.g., 'google', 'github', 'microsoft')
    provider_name VARCHAR(50) UNIQUE NOT NULL,

    -- Display name for UI
    display_name VARCHAR(100) NOT NULL,

    -- OIDC discovery URL (for auto-configuration)
    issuer_url TEXT,

    -- Manual configuration (if not using OIDC discovery)
    authorization_endpoint TEXT,
    token_endpoint TEXT,
    userinfo_endpoint TEXT,
    jwks_uri TEXT,

    -- Scopes to request
    scopes TEXT[] DEFAULT ARRAY['openid', 'email', 'profile'],

    -- Client credentials (encrypted)
    client_id_encrypted TEXT NOT NULL,
    client_secret_encrypted TEXT NOT NULL,

    -- Provider status
    is_enabled BOOLEAN DEFAULT true NOT NULL,

    -- Rate limiting
    max_requests_per_minute INTEGER DEFAULT 60,

    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW() NOT NULL
);

-- ============================================================================
-- STEP 6: Insert default provider configurations
-- ============================================================================

-- Note: Client IDs/secrets should be set via UPDATE after initial deployment
-- These are placeholder entries with encrypted empty values

INSERT INTO oauth_providers (provider_name, display_name, issuer_url, scopes) VALUES
    ('google', 'Google', 'https://accounts.google.com', ARRAY['openid', 'email', 'profile']),
    ('github', 'GitHub', NULL, ARRAY['read:user', 'user:email']),
    ('microsoft', 'Microsoft', 'https://login.microsoftonline.com/common/v2.0', ARRAY['openid', 'email', 'profile'])
ON CONFLICT (provider_name) DO NOTHING;

-- ============================================================================
-- STEP 7: Cleanup function for expired states
-- ============================================================================

CREATE OR REPLACE FUNCTION cleanup_expired_oauth_states()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM oauth_states
    WHERE expires_at < NOW()
       OR (used_at IS NOT NULL AND used_at < NOW() - INTERVAL '1 hour');

    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- STEP 8: Trigger for updated_at on oauth_providers
-- ============================================================================

CREATE OR REPLACE FUNCTION update_oauth_providers_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_oauth_providers_updated_at ON oauth_providers;
CREATE TRIGGER trigger_oauth_providers_updated_at
    BEFORE UPDATE ON oauth_providers
    FOR EACH ROW
    EXECUTE FUNCTION update_oauth_providers_updated_at();

-- ============================================================================
-- STEP 9: View for OAuth-enabled users (admin dashboard)
-- ============================================================================

CREATE OR REPLACE VIEW oauth_user_summary AS
SELECT
    u.id,
    u.email,
    u.company_name,
    u.role,
    u.oauth_provider,
    u.oauth_email,
    u.oauth_linked_at,
    u.oauth_last_login_at,
    CASE
        WHEN u.password_hash IS NOT NULL THEN true
        ELSE false
    END as has_password,
    CASE
        WHEN u.oauth_provider IS NOT NULL THEN true
        ELSE false
    END as has_oauth,
    u.created_at,
    u.is_verified
FROM users u;

-- ============================================================================
-- COMMENTS
-- ============================================================================

COMMENT ON TABLE oauth_states IS 'Temporary storage for OAuth CSRF protection tokens';
COMMENT ON TABLE oauth_audit_log IS 'Security audit trail for all OAuth operations';
COMMENT ON TABLE oauth_providers IS 'Configuration for supported OAuth/OIDC providers';

COMMENT ON COLUMN users.oauth_provider IS 'OAuth provider name (google, github, microsoft)';
COMMENT ON COLUMN users.oauth_provider_id IS 'Unique user ID from OAuth provider (sub claim)';
COMMENT ON COLUMN users.oauth_email IS 'Email address from OAuth provider';
COMMENT ON COLUMN users.oauth_access_token_encrypted IS 'Encrypted OAuth access token for API calls';
COMMENT ON COLUMN users.oauth_refresh_token_encrypted IS 'Encrypted OAuth refresh token';
COMMENT ON COLUMN users.oauth_linked_at IS 'When OAuth was first linked to this account';
COMMENT ON COLUMN users.oauth_last_login_at IS 'Last successful OAuth login';
