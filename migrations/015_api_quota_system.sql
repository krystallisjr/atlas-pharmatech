-- ============================================================================
-- API Quota System Migration
-- ============================================================================
--
-- 🔒 SECURITY: Issue #18 - Anthropic API Usage Quotas
--
-- Creates comprehensive API quota tracking and enforcement system
--
-- Tables:
-- 1. user_api_quotas - User quota tier assignments
-- 2. api_usage_log - Detailed API call tracking
-- 3. api_usage_monthly - Monthly usage summaries (materialized view)
--
-- Features:
-- - 4-tier quota system (Free, Basic, Pro, Enterprise)
-- - Per-request token/cost tracking
-- - Anomaly detection support
-- - Monthly usage aggregation
-- - Performance indexes
--
-- ============================================================================

-- Quota tier enum
CREATE TYPE quota_tier AS ENUM ('Free', 'Basic', 'Pro', 'Enterprise');

-- ============================================================================
-- TABLE: user_api_quotas
-- ============================================================================
-- Stores user quota tier assignments

CREATE TABLE IF NOT EXISTS user_api_quotas (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    quota_tier quota_tier NOT NULL DEFAULT 'Free',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for efficient tier lookups
CREATE INDEX IF NOT EXISTS idx_user_api_quotas_tier ON user_api_quotas(quota_tier);
CREATE INDEX IF NOT EXISTS idx_user_api_quotas_updated ON user_api_quotas(updated_at);

COMMENT ON TABLE user_api_quotas IS 'User API quota tier assignments';
COMMENT ON COLUMN user_api_quotas.quota_tier IS 'Free: 100/month, Basic: 1000/month, Pro: 10000/month, Enterprise: unlimited';

-- ============================================================================
-- TABLE: api_usage_log
-- ============================================================================
-- Detailed log of every API call for tracking and billing

CREATE TABLE IF NOT EXISTS api_usage_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Request details
    endpoint TEXT NOT NULL,
    request_type TEXT,

    -- Token usage (for cost estimation)
    tokens_input INTEGER,
    tokens_output INTEGER,

    -- Cost tracking (in cents for precision)
    cost_cents DECIMAL(10, 4),

    -- Performance tracking
    latency_ms BIGINT,

    -- Success/failure
    success BOOLEAN NOT NULL DEFAULT true,
    error_message TEXT,

    -- Timestamp
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_api_usage_log_user_id ON api_usage_log(user_id);
CREATE INDEX IF NOT EXISTS idx_api_usage_log_created_at ON api_usage_log(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_api_usage_log_user_created ON api_usage_log(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_api_usage_log_endpoint ON api_usage_log(endpoint);

-- Note: Monthly quota checks use api_usage_monthly materialized view (indexed below)
-- Cannot create functional index with EXTRACT as it's not marked IMMUTABLE

COMMENT ON TABLE api_usage_log IS 'Detailed API usage tracking for quota enforcement and billing';
COMMENT ON COLUMN api_usage_log.cost_cents IS 'Cost in cents (e.g., 150 = $1.50)';
COMMENT ON COLUMN api_usage_log.latency_ms IS 'API call latency in milliseconds';

-- ============================================================================
-- MATERIALIZED VIEW: api_usage_monthly
-- ============================================================================
-- Pre-aggregated monthly usage for fast quota checks

CREATE MATERIALIZED VIEW IF NOT EXISTS api_usage_monthly AS
SELECT
    user_id,
    EXTRACT(YEAR FROM created_at)::INTEGER as year,
    EXTRACT(MONTH FROM created_at)::INTEGER as month,
    COUNT(*)::INTEGER as total_requests,
    SUM(tokens_input)::BIGINT as total_tokens_input,
    SUM(tokens_output)::BIGINT as total_tokens_output,
    SUM(cost_cents)::DECIMAL(12, 4) as total_cost_cents,
    AVG(latency_ms)::BIGINT as avg_latency_ms,
    SUM(CASE WHEN success THEN 1 ELSE 0 END)::INTEGER as successful_requests,
    SUM(CASE WHEN NOT success THEN 1 ELSE 0 END)::INTEGER as failed_requests
FROM api_usage_log
GROUP BY user_id, year, month;

-- Index for fast lookups
CREATE UNIQUE INDEX IF NOT EXISTS idx_api_usage_monthly_unique
    ON api_usage_monthly(user_id, year, month);

COMMENT ON MATERIALIZED VIEW api_usage_monthly IS 'Pre-aggregated monthly API usage statistics for performance';

-- ============================================================================
-- FUNCTION: Refresh materialized view
-- ============================================================================
-- Call this function periodically (e.g., hourly) to update monthly stats

CREATE OR REPLACE FUNCTION refresh_api_usage_monthly()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY api_usage_monthly;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION refresh_api_usage_monthly IS 'Refreshes monthly API usage aggregates. Run hourly via cron or scheduler.';

-- ============================================================================
-- TRIGGER: Auto-update updated_at timestamp
-- ============================================================================

CREATE OR REPLACE FUNCTION update_user_api_quotas_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_update_user_api_quotas_updated_at ON user_api_quotas;
CREATE TRIGGER trigger_update_user_api_quotas_updated_at
    BEFORE UPDATE ON user_api_quotas
    FOR EACH ROW
    EXECUTE FUNCTION update_user_api_quotas_updated_at();

-- ============================================================================
-- HELPER FUNCTIONS
-- ============================================================================

-- Get monthly request count for a user
CREATE OR REPLACE FUNCTION get_monthly_request_count(
    p_user_id UUID,
    p_year INTEGER,
    p_month INTEGER
)
RETURNS INTEGER AS $$
DECLARE
    v_count INTEGER;
BEGIN
    SELECT total_requests INTO v_count
    FROM api_usage_monthly
    WHERE user_id = p_user_id
      AND year = p_year
      AND month = p_month;

    RETURN COALESCE(v_count, 0);
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION get_monthly_request_count IS 'Fast lookup of monthly request count from materialized view';

-- ============================================================================
-- INITIAL DATA: Set existing users to Free tier
-- ============================================================================

DO $$
BEGIN
    -- Only insert if table is empty
    IF NOT EXISTS (SELECT 1 FROM user_api_quotas LIMIT 1) THEN
        INSERT INTO user_api_quotas (user_id, quota_tier, created_at, updated_at)
        SELECT id, 'Free', NOW(), NOW()
        FROM users
        ON CONFLICT (user_id) DO NOTHING;

        RAISE NOTICE 'Initialized % users with Free tier quotas',
            (SELECT COUNT(*) FROM user_api_quotas);
    END IF;
END $$;

-- ============================================================================
-- SECURITY: Row-level security (optional, if RLS is enabled)
-- ============================================================================

-- Users can only see their own quota and usage
-- Uncomment if RLS is enabled on the database

-- ALTER TABLE user_api_quotas ENABLE ROW LEVEL SECURITY;
-- CREATE POLICY user_api_quotas_policy ON user_api_quotas
--     FOR ALL TO authenticated
--     USING (user_id = current_user_id());

-- ALTER TABLE api_usage_log ENABLE ROW LEVEL SECURITY;
-- CREATE POLICY api_usage_log_policy ON api_usage_log
--     FOR ALL TO authenticated
--     USING (user_id = current_user_id());

-- ============================================================================
-- MONITORING QUERIES
-- ============================================================================

COMMENT ON SCHEMA public IS '
-- Check quota status for a user:
SELECT u.email, q.quota_tier,
       COALESCE(m.total_requests, 0) as requests_this_month
FROM users u
LEFT JOIN user_api_quotas q ON u.id = q.user_id
LEFT JOIN api_usage_monthly m ON u.id = m.user_id
    AND m.year = EXTRACT(YEAR FROM NOW())
    AND m.month = EXTRACT(MONTH FROM NOW())
WHERE u.id = ''<user-uuid>'';

-- Top API consumers this month:
SELECT u.email, q.quota_tier, m.total_requests, m.total_cost_cents / 100.0 as cost_dollars
FROM api_usage_monthly m
JOIN users u ON m.user_id = u.id
JOIN user_api_quotas q ON u.id = q.user_id
WHERE m.year = EXTRACT(YEAR FROM NOW())
  AND m.month = EXTRACT(MONTH FROM NOW())
ORDER BY m.total_requests DESC
LIMIT 10;

-- Detect anomalies (>100 requests in 24h):
SELECT user_id, COUNT(*) as requests_24h
FROM api_usage_log
WHERE created_at > NOW() - INTERVAL ''24 hours''
GROUP BY user_id
HAVING COUNT(*) > 100
ORDER BY requests_24h DESC;
';

-- ============================================================================
-- MIGRATION COMPLETE
-- ============================================================================

DO $$
BEGIN
    RAISE NOTICE '✅ API Quota System migration completed successfully';
    RAISE NOTICE '   - Tables: user_api_quotas, api_usage_log';
    RAISE NOTICE '   - Materialized view: api_usage_monthly';
    RAISE NOTICE '   - Quota tiers: Free (100/mo), Basic (1K/mo), Pro (10K/mo), Enterprise (unlimited)';
    RAISE NOTICE '   - All existing users assigned Free tier';
END $$;
