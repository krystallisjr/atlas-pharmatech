-- ============================================================================
-- Migration: 013_add_webhook_security.sql
-- Description: Add webhook security (secrets, rate limiting, audit logging)
-- Created: 2025-11-18
-- Author: Atlas PharmaTech Security Team
-- ============================================================================

-- Add webhook secret to ERP connections (encrypted at rest)
ALTER TABLE erp_connections
ADD COLUMN IF NOT EXISTS webhook_secret_encrypted TEXT;

-- Add webhook configuration
ALTER TABLE erp_connections
ADD COLUMN IF NOT EXISTS webhook_enabled BOOLEAN DEFAULT FALSE NOT NULL,
ADD COLUMN IF NOT EXISTS webhook_url TEXT,
ADD COLUMN IF NOT EXISTS webhook_events TEXT[] DEFAULT ARRAY['inventory_update', 'item_created']::TEXT[];

-- Create webhook audit log table
CREATE TABLE IF NOT EXISTS webhook_audit_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    connection_id UUID NOT NULL REFERENCES erp_connections(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    request_id UUID NOT NULL,
    source_ip INET,
    signature_valid BOOLEAN NOT NULL,
    payload_size_bytes INTEGER NOT NULL,
    http_status INTEGER NOT NULL,
    error_message TEXT,
    processing_time_ms INTEGER,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,

    -- Indexes for querying
    CONSTRAINT webhook_audit_log_event_type_check CHECK (event_type IN ('netsuite', 'sap'))
);

-- Indexes for webhook audit log
CREATE INDEX IF NOT EXISTS idx_webhook_audit_connection_id ON webhook_audit_log(connection_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_webhook_audit_signature_invalid ON webhook_audit_log(signature_valid, created_at DESC) WHERE signature_valid = FALSE;
CREATE INDEX IF NOT EXISTS idx_webhook_audit_errors ON webhook_audit_log(created_at DESC) WHERE error_message IS NOT NULL;

-- Create webhook rate limiting table (per connection)
CREATE TABLE IF NOT EXISTS webhook_rate_limits (
    connection_id UUID PRIMARY KEY REFERENCES erp_connections(id) ON DELETE CASCADE,
    requests_count INTEGER DEFAULT 0 NOT NULL,
    window_start TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    last_request_at TIMESTAMP WITH TIME ZONE,
    blocked_until TIMESTAMP WITH TIME ZONE,

    CONSTRAINT webhook_rate_limits_requests_non_negative CHECK (requests_count >= 0)
);

-- Function to check webhook rate limit (100 requests per 15 minutes per connection)
CREATE OR REPLACE FUNCTION check_webhook_rate_limit(p_connection_id UUID)
RETURNS TABLE (
    allowed BOOLEAN,
    requests_remaining INTEGER,
    reset_at TIMESTAMP WITH TIME ZONE,
    blocked BOOLEAN
) AS $$
DECLARE
    v_window_minutes INTEGER := 15;
    v_max_requests INTEGER := 100;
    v_record RECORD;
    v_now TIMESTAMP WITH TIME ZONE := NOW();
BEGIN
    -- Get or create rate limit record
    INSERT INTO webhook_rate_limits (connection_id, window_start, last_request_at)
    VALUES (p_connection_id, v_now, v_now)
    ON CONFLICT (connection_id) DO UPDATE
    SET last_request_at = v_now
    RETURNING * INTO v_record;

    -- Check if blocked
    IF v_record.blocked_until IS NOT NULL AND v_record.blocked_until > v_now THEN
        RETURN QUERY SELECT
            FALSE as allowed,
            0 as requests_remaining,
            v_record.blocked_until as reset_at,
            TRUE as blocked;
        RETURN;
    END IF;

    -- Check if window has expired (reset counter)
    IF v_now > v_record.window_start + (v_window_minutes || ' minutes')::INTERVAL THEN
        UPDATE webhook_rate_limits
        SET requests_count = 1,
            window_start = v_now,
            blocked_until = NULL
        WHERE connection_id = p_connection_id;

        RETURN QUERY SELECT
            TRUE as allowed,
            v_max_requests - 1 as requests_remaining,
            v_now + (v_window_minutes || ' minutes')::INTERVAL as reset_at,
            FALSE as blocked;
        RETURN;
    END IF;

    -- Check if limit exceeded
    IF v_record.requests_count >= v_max_requests THEN
        -- Block for 1 hour
        UPDATE webhook_rate_limits
        SET blocked_until = v_now + INTERVAL '1 hour'
        WHERE connection_id = p_connection_id;

        RETURN QUERY SELECT
            FALSE as allowed,
            0 as requests_remaining,
            v_now + INTERVAL '1 hour' as reset_at,
            TRUE as blocked;
        RETURN;
    END IF;

    -- Increment counter
    UPDATE webhook_rate_limits
    SET requests_count = requests_count + 1
    WHERE connection_id = p_connection_id;

    RETURN QUERY SELECT
        TRUE as allowed,
        v_max_requests - (v_record.requests_count + 1) as requests_remaining,
        v_record.window_start + (v_window_minutes || ' minutes')::INTERVAL as reset_at,
        FALSE as blocked;
END;
$$ LANGUAGE plpgsql;

-- Function to generate webhook secret
CREATE OR REPLACE FUNCTION generate_webhook_secret()
RETURNS TEXT AS $$
BEGIN
    -- Generate 32-byte random secret, base64 encoded
    RETURN encode(gen_random_bytes(32), 'base64');
END;
$$ LANGUAGE plpgsql;

-- Grant execute permission
GRANT EXECUTE ON FUNCTION check_webhook_rate_limit(UUID) TO atlas_user;
GRANT EXECUTE ON FUNCTION generate_webhook_secret() TO atlas_user;

-- Add comment
COMMENT ON TABLE webhook_audit_log IS 'Audit log for all webhook requests (successful and failed)';
COMMENT ON TABLE webhook_rate_limits IS 'Rate limiting for webhook endpoints per ERP connection';
COMMENT ON COLUMN erp_connections.webhook_secret_encrypted IS 'Encrypted webhook secret for HMAC signature verification';
