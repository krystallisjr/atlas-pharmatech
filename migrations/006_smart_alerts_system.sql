-- Smart Inventory Alerts System
-- Migration: 006
-- Description: Adds comprehensive alert and notification infrastructure
--              including user preferences, notification history, and marketplace watchlist

-- ============================================================================
-- USER ALERT PREFERENCES
-- ============================================================================

CREATE TABLE user_alert_preferences (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,

    -- Expiry Alerts
    expiry_alerts_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    expiry_alert_days INTEGER NOT NULL DEFAULT 30 CHECK (expiry_alert_days > 0 AND expiry_alert_days <= 365),

    -- Low Stock Alerts
    low_stock_alerts_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    low_stock_threshold INTEGER NOT NULL DEFAULT 10 CHECK (low_stock_threshold >= 0),

    -- Watchlist Alerts
    watchlist_alerts_enabled BOOLEAN NOT NULL DEFAULT TRUE,

    -- Notification Channels
    email_notifications_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    in_app_notifications_enabled BOOLEAN NOT NULL DEFAULT TRUE,

    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Trigger to auto-update updated_at
CREATE OR REPLACE FUNCTION update_user_alert_preferences_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_user_alert_preferences_updated_at
    BEFORE UPDATE ON user_alert_preferences
    FOR EACH ROW
    EXECUTE FUNCTION update_user_alert_preferences_updated_at();

-- ============================================================================
-- ALERT NOTIFICATIONS
-- ============================================================================

CREATE TABLE alert_notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Alert Classification
    alert_type VARCHAR(50) NOT NULL CHECK (alert_type IN (
        'expiry_warning',
        'expiry_critical',
        'low_stock',
        'watchlist_match',
        'price_drop',
        'system'
    )),
    severity VARCHAR(20) NOT NULL CHECK (severity IN ('info', 'warning', 'critical')),

    -- Alert Content
    title TEXT NOT NULL CHECK (length(title) > 0),
    message TEXT NOT NULL CHECK (length(message) > 0),

    -- Related Entities
    inventory_id UUID REFERENCES inventory(id) ON DELETE SET NULL,
    related_user_id UUID REFERENCES users(id) ON DELETE SET NULL,

    -- Additional Data
    metadata JSONB DEFAULT '{}'::jsonb,
    action_url TEXT,

    -- Status
    is_read BOOLEAN NOT NULL DEFAULT FALSE,
    is_dismissed BOOLEAN NOT NULL DEFAULT FALSE,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    read_at TIMESTAMPTZ,
    dismissed_at TIMESTAMPTZ,

    -- Constraints
    CHECK (read_at IS NULL OR read_at >= created_at),
    CHECK (dismissed_at IS NULL OR dismissed_at >= created_at)
);

-- Indexes for performance
CREATE INDEX idx_alerts_user_unread ON alert_notifications(user_id, is_read, created_at DESC)
    WHERE is_read = FALSE;
CREATE INDEX idx_alerts_user_all ON alert_notifications(user_id, created_at DESC);
CREATE INDEX idx_alerts_type ON alert_notifications(alert_type, created_at DESC);
CREATE INDEX idx_alerts_inventory ON alert_notifications(inventory_id)
    WHERE inventory_id IS NOT NULL;

-- Trigger to set read_at timestamp
CREATE OR REPLACE FUNCTION set_alert_read_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.is_read = TRUE AND OLD.is_read = FALSE THEN
        NEW.read_at = NOW();
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_set_alert_read_timestamp
    BEFORE UPDATE ON alert_notifications
    FOR EACH ROW
    WHEN (NEW.is_read IS DISTINCT FROM OLD.is_read)
    EXECUTE FUNCTION set_alert_read_timestamp();

-- ============================================================================
-- MARKETPLACE WATCHLIST
-- ============================================================================

CREATE TABLE marketplace_watchlist (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Watchlist Details
    name VARCHAR(255) NOT NULL CHECK (length(name) > 0),
    description TEXT,

    -- Search Criteria (stored as JSONB for flexibility)
    search_criteria JSONB NOT NULL,

    -- Alert Settings
    alert_enabled BOOLEAN NOT NULL DEFAULT TRUE,

    -- Tracking
    last_checked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_match_count INTEGER NOT NULL DEFAULT 0,
    total_matches_found INTEGER NOT NULL DEFAULT 0,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CHECK (jsonb_typeof(search_criteria) = 'object')
);

-- Indexes
CREATE INDEX idx_watchlist_user ON marketplace_watchlist(user_id, created_at DESC);
CREATE INDEX idx_watchlist_alerts_enabled ON marketplace_watchlist(user_id, alert_enabled)
    WHERE alert_enabled = TRUE;

-- Trigger to auto-update updated_at
CREATE OR REPLACE FUNCTION update_watchlist_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_watchlist_updated_at
    BEFORE UPDATE ON marketplace_watchlist
    FOR EACH ROW
    EXECUTE FUNCTION update_watchlist_updated_at();

-- ============================================================================
-- ALERT PROCESSING LOG (for debugging and analytics)
-- ============================================================================

CREATE TABLE alert_processing_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_type VARCHAR(50) NOT NULL CHECK (run_type IN (
        'expiry_check',
        'low_stock_check',
        'watchlist_check',
        'scheduled_run'
    )),
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    status VARCHAR(20) NOT NULL CHECK (status IN ('running', 'completed', 'failed')),
    alerts_generated INTEGER NOT NULL DEFAULT 0,
    errors_encountered INTEGER NOT NULL DEFAULT 0,
    error_details TEXT,
    metadata JSONB DEFAULT '{}'::jsonb,

    CHECK (completed_at IS NULL OR completed_at >= started_at)
);

CREATE INDEX idx_alert_log_status ON alert_processing_log(status, started_at DESC);
CREATE INDEX idx_alert_log_type ON alert_processing_log(run_type, started_at DESC);

-- ============================================================================
-- HELPER FUNCTIONS
-- ============================================================================

-- Function to get unread alert count for a user
CREATE OR REPLACE FUNCTION get_unread_alert_count(p_user_id UUID)
RETURNS INTEGER AS $$
    SELECT COUNT(*)::INTEGER
    FROM alert_notifications
    WHERE user_id = p_user_id
      AND is_read = FALSE
      AND is_dismissed = FALSE;
$$ LANGUAGE SQL STABLE;

-- Function to create default preferences for new users
CREATE OR REPLACE FUNCTION create_default_alert_preferences()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO user_alert_preferences (user_id)
    VALUES (NEW.id)
    ON CONFLICT (user_id) DO NOTHING;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_create_default_alert_preferences
    AFTER INSERT ON users
    FOR EACH ROW
    EXECUTE FUNCTION create_default_alert_preferences();

-- ============================================================================
-- COMMENTS (for documentation)
-- ============================================================================

COMMENT ON TABLE user_alert_preferences IS 'User-specific alert configuration and preferences';
COMMENT ON TABLE alert_notifications IS 'All notifications sent to users, including system alerts and inventory warnings';
COMMENT ON TABLE marketplace_watchlist IS 'Saved marketplace searches that generate alerts when matches are found';
COMMENT ON TABLE alert_processing_log IS 'Audit log of background alert processing runs for monitoring and debugging';

COMMENT ON COLUMN user_alert_preferences.expiry_alert_days IS 'Number of days before expiry to trigger warning (1-365)';
COMMENT ON COLUMN user_alert_preferences.low_stock_threshold IS 'Quantity threshold below which to trigger low stock alert';
COMMENT ON COLUMN alert_notifications.metadata IS 'Additional context data stored as JSON (e.g., quantity, batch number, price)';
COMMENT ON COLUMN marketplace_watchlist.search_criteria IS 'SearchInventoryRequest stored as JSON for flexible matching';

-- ============================================================================
-- INITIAL DATA (create preferences for existing users)
-- ============================================================================

INSERT INTO user_alert_preferences (user_id)
SELECT id FROM users
ON CONFLICT (user_id) DO NOTHING;

-- ============================================================================
-- MIGRATION COMPLETE
-- ============================================================================
