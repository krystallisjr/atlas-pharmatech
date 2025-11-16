-- ============================================================================
-- PRODUCTION-GRADE COMPREHENSIVE AUDIT LOGGING SYSTEM
-- Compliance: SOC 2, HIPAA, ISO 27001, GDPR
-- ============================================================================

-- Main system-wide audit log table
CREATE TABLE IF NOT EXISTS audit_logs (
    -- Primary key
    id BIGSERIAL PRIMARY KEY,

    -- Event identification
    event_id UUID NOT NULL DEFAULT gen_random_uuid(),
    event_type VARCHAR(100) NOT NULL,
    -- Event categories: auth, data_access, data_modification, security, system, admin
    event_category VARCHAR(50) NOT NULL,

    -- Event classification (for filtering/alerting)
    severity VARCHAR(20) NOT NULL CHECK (severity IN ('info', 'warning', 'error', 'critical')),

    -- Actor information (WHO)
    actor_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    actor_type VARCHAR(50) NOT NULL DEFAULT 'user', -- user, system, admin, api, scheduled_job
    actor_identifier VARCHAR(255), -- Email, API key ID, job name, etc.

    -- Target information (WHAT)
    resource_type VARCHAR(100), -- inventory, pharmaceutical, user, session, file, etc.
    resource_id VARCHAR(255), -- UUID or identifier of the resource
    resource_name VARCHAR(500), -- Human-readable resource name

    -- Action performed
    action VARCHAR(100) NOT NULL, -- create, read, update, delete, login, logout, etc.
    action_result VARCHAR(50) NOT NULL DEFAULT 'success', -- success, failure, partial

    -- Detailed event data
    event_data JSONB NOT NULL DEFAULT '{}',
    -- Stores: before/after values, error messages, additional context

    -- Request metadata
    ip_address INET,
    user_agent TEXT,
    request_id VARCHAR(100), -- For correlation with app logs
    session_id VARCHAR(255), -- Session/token identifier

    -- Changes tracking (for data modifications)
    changes_summary TEXT, -- Human-readable summary of changes
    old_values JSONB, -- Previous state (for updates/deletes)
    new_values JSONB, -- New state (for creates/updates)

    -- Compliance fields
    retention_until TIMESTAMPTZ, -- When this log can be deleted (for retention policies)
    is_pii_access BOOLEAN DEFAULT FALSE, -- Flag for PII data access
    compliance_tags VARCHAR(100)[], -- e.g., ['hipaa', 'gdpr', 'financial']

    -- Timestamps (immutable)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Performance indexes
CREATE INDEX idx_audit_event_type ON audit_logs(event_type);
CREATE INDEX idx_audit_event_category ON audit_logs(event_category);
CREATE INDEX idx_audit_severity ON audit_logs(severity);
CREATE INDEX idx_audit_actor_user ON audit_logs(actor_user_id) WHERE actor_user_id IS NOT NULL;
CREATE INDEX idx_audit_resource ON audit_logs(resource_type, resource_id);
CREATE INDEX idx_audit_action ON audit_logs(action);
CREATE INDEX idx_audit_created ON audit_logs(created_at DESC);
CREATE INDEX idx_audit_ip_address ON audit_logs(ip_address) WHERE ip_address IS NOT NULL;
CREATE INDEX idx_audit_result ON audit_logs(action_result);

-- JSONB GIN indexes for efficient querying of event_data
CREATE INDEX idx_audit_event_data_gin ON audit_logs USING GIN (event_data);
CREATE INDEX idx_audit_changes_gin ON audit_logs USING GIN (old_values, new_values);

-- Composite indexes for common queries
CREATE INDEX idx_audit_user_created ON audit_logs(actor_user_id, created_at DESC) WHERE actor_user_id IS NOT NULL;
CREATE INDEX idx_audit_resource_created ON audit_logs(resource_type, resource_id, created_at DESC);
CREATE INDEX idx_audit_security_events ON audit_logs(created_at DESC) WHERE event_category IN ('security', 'auth');
CREATE INDEX idx_audit_failed_actions ON audit_logs(created_at DESC) WHERE action_result = 'failure';

-- Partial index for PII access tracking (compliance requirement)
CREATE INDEX idx_audit_pii_access ON audit_logs(actor_user_id, created_at DESC) WHERE is_pii_access = TRUE;

-- Security: Prevent updates/deletes on audit logs (immutable)
CREATE OR REPLACE FUNCTION prevent_audit_log_modification()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        -- Only allow deletion of logs past retention period
        IF OLD.retention_until IS NULL OR OLD.retention_until > NOW() THEN
            RAISE EXCEPTION 'Audit logs cannot be deleted before retention period expires';
        END IF;
        RETURN OLD;
    ELSIF TG_OP = 'UPDATE' THEN
        -- Audit logs are immutable - no updates allowed
        RAISE EXCEPTION 'Audit logs are immutable and cannot be updated';
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER audit_log_immutable
    BEFORE UPDATE OR DELETE ON audit_logs
    FOR EACH ROW
    EXECUTE FUNCTION prevent_audit_log_modification();

-- Audit log statistics view for compliance reporting
CREATE OR REPLACE VIEW audit_log_statistics AS
SELECT
    event_category,
    event_type,
    action,
    action_result,
    DATE(created_at) as date,
    COUNT(*) as event_count,
    COUNT(DISTINCT actor_user_id) as unique_users,
    COUNT(*) FILTER (WHERE action_result = 'failure') as failure_count,
    COUNT(*) FILTER (WHERE is_pii_access = TRUE) as pii_access_count
FROM audit_logs
WHERE created_at >= NOW() - INTERVAL '90 days'
GROUP BY event_category, event_type, action, action_result, DATE(created_at);

-- Failed login attempts view (security monitoring)
CREATE OR REPLACE VIEW failed_login_attempts AS
SELECT
    actor_identifier as email,
    ip_address,
    COUNT(*) as attempt_count,
    MAX(created_at) as last_attempt,
    ARRAY_AGG(DISTINCT user_agent) as user_agents
FROM audit_logs
WHERE event_type = 'login_failed'
    AND created_at >= NOW() - INTERVAL '24 hours'
GROUP BY actor_identifier, ip_address
HAVING COUNT(*) >= 3
ORDER BY attempt_count DESC, last_attempt DESC;

-- Recent PII access audit (compliance monitoring)
CREATE OR REPLACE VIEW recent_pii_access AS
SELECT
    al.id,
    al.actor_identifier,
    al.event_type,
    al.action,
    al.resource_type,
    al.resource_name,
    al.ip_address,
    al.created_at,
    al.event_data->>'field_accessed' as pii_field_accessed
FROM audit_logs al
WHERE al.is_pii_access = TRUE
    AND al.created_at >= NOW() - INTERVAL '7 days'
ORDER BY al.created_at DESC;

-- Security events dashboard view
CREATE OR REPLACE VIEW security_events_summary AS
SELECT
    DATE(created_at) as date,
    event_type,
    COUNT(*) as event_count,
    COUNT(DISTINCT actor_user_id) as affected_users,
    COUNT(DISTINCT ip_address) as unique_ips,
    COUNT(*) FILTER (WHERE severity IN ('error', 'critical')) as critical_count
FROM audit_logs
WHERE event_category = 'security'
    AND created_at >= NOW() - INTERVAL '30 days'
GROUP BY DATE(created_at), event_type
ORDER BY date DESC, critical_count DESC;

-- User activity summary (for compliance investigations)
CREATE OR REPLACE VIEW user_activity_summary AS
SELECT
    u.id as user_id,
    u.company_name,
    al.actor_identifier as email,
    COUNT(*) as total_actions,
    COUNT(*) FILTER (WHERE al.action IN ('create', 'update', 'delete')) as modification_count,
    COUNT(*) FILTER (WHERE al.is_pii_access = TRUE) as pii_access_count,
    COUNT(*) FILTER (WHERE al.action_result = 'failure') as failed_actions,
    MAX(al.created_at) as last_activity,
    COUNT(DISTINCT al.ip_address) as unique_ips
FROM users u
LEFT JOIN audit_logs al ON al.actor_user_id = u.id
WHERE al.created_at >= NOW() - INTERVAL '30 days' OR al.created_at IS NULL
GROUP BY u.id, u.company_name, al.actor_identifier;

-- Comments for documentation
COMMENT ON TABLE audit_logs IS 'Comprehensive immutable audit log for security, compliance, and forensic analysis';
COMMENT ON COLUMN audit_logs.event_id IS 'Unique identifier for event correlation across systems';
COMMENT ON COLUMN audit_logs.event_category IS 'High-level categorization: auth, data_access, data_modification, security, system, admin';
COMMENT ON COLUMN audit_logs.severity IS 'Event severity for alerting and filtering';
COMMENT ON COLUMN audit_logs.actor_user_id IS 'User who performed the action (NULL for system events)';
COMMENT ON COLUMN audit_logs.resource_type IS 'Type of resource affected (inventory, user, pharmaceutical, etc.)';
COMMENT ON COLUMN audit_logs.event_data IS 'Detailed event information in JSONB format';
COMMENT ON COLUMN audit_logs.old_values IS 'Previous state before modification (for audit trail)';
COMMENT ON COLUMN audit_logs.new_values IS 'New state after modification (for audit trail)';
COMMENT ON COLUMN audit_logs.is_pii_access IS 'Flag indicating if PII was accessed (for GDPR/HIPAA compliance)';
COMMENT ON COLUMN audit_logs.retention_until IS 'Date when log can be deleted per retention policy';
COMMENT ON COLUMN audit_logs.compliance_tags IS 'Compliance framework tags for reporting';

-- Retention policy: Set default retention to 7 years (common compliance requirement)
CREATE OR REPLACE FUNCTION set_audit_log_retention()
RETURNS TRIGGER AS $$
BEGIN
    -- Default 7-year retention for compliance
    IF NEW.retention_until IS NULL THEN
        NEW.retention_until := NEW.created_at + INTERVAL '7 years';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER set_retention_policy
    BEFORE INSERT ON audit_logs
    FOR EACH ROW
    EXECUTE FUNCTION set_audit_log_retention();

-- Automatic cleanup of expired audit logs (run via scheduled job)
CREATE OR REPLACE FUNCTION cleanup_expired_audit_logs()
RETURNS TABLE(deleted_count BIGINT) AS $$
DECLARE
    rows_deleted BIGINT;
BEGIN
    DELETE FROM audit_logs
    WHERE retention_until IS NOT NULL
        AND retention_until < NOW();

    GET DIAGNOSTICS rows_deleted = ROW_COUNT;
    RETURN QUERY SELECT rows_deleted;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION cleanup_expired_audit_logs IS 'Removes audit logs past their retention period - run via scheduled job';
