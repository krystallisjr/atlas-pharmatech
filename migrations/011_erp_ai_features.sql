-- Migration: ERP AI Features
-- Description: Add tables for AI-powered ERP features (mapping auto-discovery, sync analysis, conflict resolution)
-- Created: 2025-01-16

-- =====================================================
-- 1. AI Mapping Suggestions Table
-- =====================================================
-- Stores AI-suggested mappings between Atlas inventory and ERP items
CREATE TABLE IF NOT EXISTS erp_ai_mapping_suggestions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    erp_connection_id UUID NOT NULL REFERENCES erp_connections(id) ON DELETE CASCADE,
    atlas_inventory_id UUID REFERENCES inventory(id) ON DELETE CASCADE,

    -- ERP item information
    erp_item_id TEXT,
    erp_item_name TEXT,
    erp_item_description TEXT,

    -- Atlas item information (denormalized for performance)
    atlas_product_name TEXT,
    atlas_ndc_code TEXT,
    atlas_batch_number TEXT,

    -- AI analysis
    confidence_score DECIMAL(5,4) CHECK (confidence_score >= 0 AND confidence_score <= 1),
    ai_reasoning TEXT NOT NULL,
    matching_factors JSONB, -- { "ndc_match": true, "name_similarity": 0.95, "manufacturer_match": true }

    -- User decision
    status TEXT NOT NULL DEFAULT 'suggested' CHECK (status IN ('suggested', 'accepted', 'rejected', 'skipped')),
    reviewed_by UUID REFERENCES users(id) ON DELETE SET NULL,
    reviewed_at TIMESTAMPTZ,

    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT unique_mapping_suggestion UNIQUE (erp_connection_id, atlas_inventory_id, erp_item_id)
);

-- Indexes for performance
CREATE INDEX idx_erp_ai_mapping_suggestions_connection ON erp_ai_mapping_suggestions(erp_connection_id);
CREATE INDEX idx_erp_ai_mapping_suggestions_inventory ON erp_ai_mapping_suggestions(atlas_inventory_id);
CREATE INDEX idx_erp_ai_mapping_suggestions_status ON erp_ai_mapping_suggestions(status);
CREATE INDEX idx_erp_ai_mapping_suggestions_confidence ON erp_ai_mapping_suggestions(confidence_score DESC);
CREATE INDEX idx_erp_ai_mapping_suggestions_created ON erp_ai_mapping_suggestions(created_at DESC);

-- =====================================================
-- 2. Conflict Resolution History Table
-- =====================================================
-- Stores AI-suggested conflict resolutions and user decisions
CREATE TABLE IF NOT EXISTS erp_ai_conflict_resolutions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    erp_connection_id UUID NOT NULL REFERENCES erp_connections(id) ON DELETE CASCADE,
    erp_sync_log_id UUID REFERENCES erp_sync_logs(id) ON DELETE SET NULL,

    -- Conflict details
    conflict_type TEXT NOT NULL CHECK (conflict_type IN ('quantity_mismatch', 'price_mismatch', 'data_quality', 'timestamp_conflict', 'other')),
    atlas_inventory_id UUID REFERENCES inventory(id) ON DELETE CASCADE,
    erp_item_id TEXT,

    -- Conflicting data
    conflict_data JSONB NOT NULL, -- { "atlas_quantity": 100, "erp_quantity": 95, "atlas_updated_at": "...", "erp_updated_at": "..." }

    -- AI suggestion
    ai_suggested_resolution TEXT NOT NULL CHECK (ai_suggested_resolution IN ('atlas_wins', 'erp_wins', 'manual_review', 'merge', 'reject_sync')),
    ai_reasoning TEXT NOT NULL,
    confidence_score DECIMAL(5,4) CHECK (confidence_score >= 0 AND confidence_score <= 1),
    risk_level TEXT NOT NULL CHECK (risk_level IN ('low', 'medium', 'high', 'critical')),

    -- User decision
    resolution_taken TEXT CHECK (resolution_taken IN ('atlas_wins', 'erp_wins', 'manual_review', 'merge', 'reject_sync')),
    resolved_by UUID REFERENCES users(id) ON DELETE SET NULL,
    resolved_at TIMESTAMPTZ,
    resolution_notes TEXT,

    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_erp_ai_conflict_connection ON erp_ai_conflict_resolutions(erp_connection_id);
CREATE INDEX idx_erp_ai_conflict_sync_log ON erp_ai_conflict_resolutions(erp_sync_log_id);
CREATE INDEX idx_erp_ai_conflict_type ON erp_ai_conflict_resolutions(conflict_type);
CREATE INDEX idx_erp_ai_conflict_risk ON erp_ai_conflict_resolutions(risk_level);
CREATE INDEX idx_erp_ai_conflict_created ON erp_ai_conflict_resolutions(created_at DESC);

-- =====================================================
-- 3. Sync Insights Table
-- =====================================================
-- Stores AI analysis of sync operations
CREATE TABLE IF NOT EXISTS erp_ai_sync_insights (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    erp_sync_log_id UUID NOT NULL REFERENCES erp_sync_logs(id) ON DELETE CASCADE,
    erp_connection_id UUID NOT NULL REFERENCES erp_connections(id) ON DELETE CASCADE,

    -- Insight details
    insight_type TEXT NOT NULL CHECK (insight_type IN ('error_explanation', 'performance_analysis', 'data_quality', 'recommendation', 'anomaly_detection', 'success_summary')),
    severity TEXT NOT NULL CHECK (severity IN ('info', 'warning', 'error', 'critical')),

    -- AI analysis
    insight_title TEXT NOT NULL,
    insight_text TEXT NOT NULL,
    ai_explanation TEXT,

    -- Recommendations
    recommendations JSONB, -- [{ "action": "...", "priority": "high", "description": "..." }]
    actionable BOOLEAN DEFAULT false,

    -- User interaction
    acknowledged_by UUID REFERENCES users(id) ON DELETE SET NULL,
    acknowledged_at TIMESTAMPTZ,
    action_taken TEXT,

    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_erp_ai_sync_insights_sync_log ON erp_ai_sync_insights(erp_sync_log_id);
CREATE INDEX idx_erp_ai_sync_insights_connection ON erp_ai_sync_insights(erp_connection_id);
CREATE INDEX idx_erp_ai_sync_insights_type ON erp_ai_sync_insights(insight_type);
CREATE INDEX idx_erp_ai_sync_insights_severity ON erp_ai_sync_insights(severity);
CREATE INDEX idx_erp_ai_sync_insights_created ON erp_ai_sync_insights(created_at DESC);

-- =====================================================
-- 4. Update User AI Usage Limits Table
-- =====================================================
-- Add ERP AI quota fields to existing table
ALTER TABLE user_ai_usage_limits
ADD COLUMN IF NOT EXISTS monthly_erp_ai_mapping_limit INTEGER DEFAULT 5,
ADD COLUMN IF NOT EXISTS monthly_erp_ai_mapping_used INTEGER DEFAULT 0,
ADD COLUMN IF NOT EXISTS monthly_erp_ai_analysis_limit INTEGER DEFAULT 50,
ADD COLUMN IF NOT EXISTS monthly_erp_ai_analysis_used INTEGER DEFAULT 0,
ADD COLUMN IF NOT EXISTS monthly_erp_ai_conflict_limit INTEGER DEFAULT 20,
ADD COLUMN IF NOT EXISTS monthly_erp_ai_conflict_used INTEGER DEFAULT 0;

-- =====================================================
-- 5. Create View for High-Confidence Mappings
-- =====================================================
-- Makes it easy to query high-confidence suggested mappings
CREATE OR REPLACE VIEW erp_high_confidence_mappings AS
SELECT
    eams.*,
    i.pharmaceutical_id,
    i.batch_number,
    i.quantity as atlas_quantity,
    ec.connection_name,
    ec.erp_type
FROM erp_ai_mapping_suggestions eams
LEFT JOIN inventory i ON eams.atlas_inventory_id = i.id
LEFT JOIN erp_connections ec ON eams.erp_connection_id = ec.id
WHERE eams.status = 'suggested'
AND eams.confidence_score >= 0.90
ORDER BY eams.confidence_score DESC, eams.created_at DESC;

-- =====================================================
-- 6. Create View for Unresolved Conflicts
-- =====================================================
CREATE OR REPLACE VIEW erp_unresolved_conflicts AS
SELECT
    eacr.*,
    esl.sync_direction,
    esl.status as sync_status,
    ec.connection_name,
    ec.erp_type
FROM erp_ai_conflict_resolutions eacr
LEFT JOIN erp_sync_logs esl ON eacr.erp_sync_log_id = esl.id
LEFT JOIN erp_connections ec ON eacr.erp_connection_id = ec.id
WHERE eacr.resolution_taken IS NULL
ORDER BY
    CASE eacr.risk_level
        WHEN 'critical' THEN 1
        WHEN 'high' THEN 2
        WHEN 'medium' THEN 3
        WHEN 'low' THEN 4
    END,
    eacr.created_at DESC;

-- =====================================================
-- 7. Create View for Recent Sync Insights
-- =====================================================
CREATE OR REPLACE VIEW erp_recent_sync_insights AS
SELECT
    easi.*,
    esl.sync_direction,
    esl.items_synced,
    esl.items_failed,
    esl.status as sync_status,
    ec.connection_name,
    ec.erp_type
FROM erp_ai_sync_insights easi
LEFT JOIN erp_sync_logs esl ON easi.erp_sync_log_id = esl.id
LEFT JOIN erp_connections ec ON easi.erp_connection_id = ec.id
WHERE easi.created_at > NOW() - INTERVAL '7 days'
ORDER BY easi.created_at DESC;

-- =====================================================
-- 8. Add Triggers for Updated_at
-- =====================================================
CREATE OR REPLACE FUNCTION update_erp_ai_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_erp_ai_mapping_suggestions_updated_at
    BEFORE UPDATE ON erp_ai_mapping_suggestions
    FOR EACH ROW
    EXECUTE FUNCTION update_erp_ai_updated_at();

CREATE TRIGGER trigger_erp_ai_conflict_resolutions_updated_at
    BEFORE UPDATE ON erp_ai_conflict_resolutions
    FOR EACH ROW
    EXECUTE FUNCTION update_erp_ai_updated_at();

CREATE TRIGGER trigger_erp_ai_sync_insights_updated_at
    BEFORE UPDATE ON erp_ai_sync_insights
    FOR EACH ROW
    EXECUTE FUNCTION update_erp_ai_updated_at();

-- =====================================================
-- 9. Add Comments for Documentation
-- =====================================================
COMMENT ON TABLE erp_ai_mapping_suggestions IS 'AI-suggested mappings between Atlas inventory and ERP items with confidence scores';
COMMENT ON TABLE erp_ai_conflict_resolutions IS 'AI-assisted conflict resolution suggestions and user decisions';
COMMENT ON TABLE erp_ai_sync_insights IS 'AI analysis and insights from ERP sync operations';

COMMENT ON COLUMN erp_ai_mapping_suggestions.confidence_score IS 'AI confidence in mapping accuracy (0.00 to 1.00)';
COMMENT ON COLUMN erp_ai_mapping_suggestions.matching_factors IS 'JSON object with individual matching factor scores';
COMMENT ON COLUMN erp_ai_conflict_resolutions.conflict_data IS 'JSON object with conflicting data from both systems';
COMMENT ON COLUMN erp_ai_conflict_resolutions.risk_level IS 'Risk assessment: low, medium, high, critical';
COMMENT ON COLUMN erp_ai_sync_insights.recommendations IS 'JSON array of actionable recommendations';

-- =====================================================
-- Migration Complete
-- =====================================================
