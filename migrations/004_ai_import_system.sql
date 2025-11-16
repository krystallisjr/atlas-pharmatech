-- AI Import Sessions: Track every import attempt with full context
CREATE TABLE IF NOT EXISTS ai_import_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    
    -- File metadata
    original_filename VARCHAR(500) NOT NULL,
    file_size_bytes BIGINT NOT NULL,
    file_type VARCHAR(50) NOT NULL, -- csv, xlsx, json, xml, pdf
    file_hash VARCHAR(64) NOT NULL, -- SHA256 for deduplication
    file_path VARCHAR(1000), -- Path to stored file on disk

    -- Import status
    status VARCHAR(50) NOT NULL DEFAULT 'analyzing', 
    -- analyzing, mapping_review, importing, completed, failed, cancelled
    
    -- AI Analysis results
    detected_format VARCHAR(50), -- csv, excel, json
    detected_columns JSONB, -- Array of detected column names
    ai_mapping JSONB, -- Claude's column mapping suggestions
    ai_confidence_scores JSONB, -- Confidence per field mapping
    ai_warnings JSONB[], -- Array of warnings Claude identified
    user_mapping_overrides JSONB, -- User's manual adjustments
    
    -- Import statistics
    total_rows INTEGER,
    rows_processed INTEGER DEFAULT 0,
    rows_imported INTEGER DEFAULT 0,
    rows_failed INTEGER DEFAULT 0,
    rows_flagged_for_review INTEGER DEFAULT 0,
    
    -- OpenFDA validation stats
    ndc_validated_count INTEGER DEFAULT 0,
    ndc_not_found_count INTEGER DEFAULT 0,
    auto_enriched_count INTEGER DEFAULT 0,
    
    -- Cost tracking
    ai_api_cost_usd DECIMAL(10, 4) DEFAULT 0.00,
    ai_tokens_used INTEGER DEFAULT 0,
    
    -- Error tracking
    error_message TEXT,
    error_details JSONB,
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    analysis_completed_at TIMESTAMPTZ,
    mapping_approved_at TIMESTAMPTZ,
    import_started_at TIMESTAMPTZ,
    import_completed_at TIMESTAMPTZ,
    
    -- Metadata
    import_source VARCHAR(100), -- web_upload, api, scheduled
    metadata JSONB -- Extensible metadata
);

CREATE INDEX idx_ai_import_sessions_user ON ai_import_sessions(user_id);
CREATE INDEX idx_ai_import_sessions_status ON ai_import_sessions(status);
CREATE INDEX idx_ai_import_sessions_created ON ai_import_sessions(created_at DESC);
CREATE INDEX idx_ai_import_sessions_file_hash ON ai_import_sessions(file_hash);

-- Import Row Results: Individual row processing details
CREATE TABLE IF NOT EXISTS ai_import_row_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES ai_import_sessions(id) ON DELETE CASCADE,
    
    -- Row identification
    row_number INTEGER NOT NULL,
    source_data JSONB NOT NULL, -- Original row data as JSON
    
    -- Processing status
    status VARCHAR(50) NOT NULL, 
    -- pending, processing, imported, failed, flagged_for_review
    
    -- Mapping results
    mapped_data JSONB, -- Data after AI mapping applied
    validated_data JSONB, -- Data after validation/enrichment
    
    -- Validation results
    validation_errors JSONB[], -- Array of validation errors
    validation_warnings JSONB[], -- Array of warnings
    
    -- OpenFDA enrichment
    matched_ndc VARCHAR(50),
    openfda_match_confidence DECIMAL(5, 4),
    openfda_enriched_fields JSONB,
    
    -- If imported successfully
    created_inventory_id UUID REFERENCES inventory(id),
    created_pharmaceutical_id UUID REFERENCES pharmaceuticals(id),
    
    -- Error tracking
    error_message TEXT,
    error_type VARCHAR(100),
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ,
    imported_at TIMESTAMPTZ,
    
    UNIQUE(session_id, row_number)
);

CREATE INDEX idx_import_row_session ON ai_import_row_results(session_id);
CREATE INDEX idx_import_row_status ON ai_import_row_results(status);
CREATE INDEX idx_import_row_inventory ON ai_import_row_results(created_inventory_id);

-- Import Audit Log: Immutable audit trail
CREATE TABLE IF NOT EXISTS ai_import_audit_log (
    id BIGSERIAL PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES ai_import_sessions(id),
    
    event_type VARCHAR(100) NOT NULL,
    -- file_uploaded, analysis_started, analysis_completed, mapping_reviewed,
    -- import_started, row_processed, import_completed, import_failed, etc.
    
    event_data JSONB NOT NULL,
    user_id UUID REFERENCES users(id),
    
    -- Metadata
    ip_address INET,
    user_agent TEXT,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_import_audit_session ON ai_import_audit_log(session_id);
CREATE INDEX idx_import_audit_event ON ai_import_audit_log(event_type);
CREATE INDEX idx_import_audit_created ON ai_import_audit_log(created_at DESC);

-- AI API Usage Tracking: Monitor costs and rate limits
CREATE TABLE IF NOT EXISTS ai_api_usage (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    session_id UUID REFERENCES ai_import_sessions(id),
    
    -- API call details
    api_provider VARCHAR(50) NOT NULL DEFAULT 'anthropic',
    api_model VARCHAR(100) NOT NULL, -- claude-3-5-sonnet-20241022
    api_endpoint VARCHAR(200) NOT NULL,
    
    -- Usage metrics
    input_tokens INTEGER NOT NULL,
    output_tokens INTEGER NOT NULL,
    total_tokens INTEGER NOT NULL,
    
    -- Cost calculation
    input_cost_usd DECIMAL(10, 6) NOT NULL,
    output_cost_usd DECIMAL(10, 6) NOT NULL,
    total_cost_usd DECIMAL(10, 6) NOT NULL,
    
    -- Response metadata
    latency_ms INTEGER,
    status_code INTEGER,
    error_message TEXT,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ai_usage_user ON ai_api_usage(user_id);
CREATE INDEX idx_ai_usage_session ON ai_api_usage(session_id);
CREATE INDEX idx_ai_usage_created ON ai_api_usage(created_at DESC);

-- User AI Usage Limits: Track quotas per plan tier
CREATE TABLE IF NOT EXISTS user_ai_usage_limits (
    user_id UUID PRIMARY KEY REFERENCES users(id),
    
    -- Monthly limits
    monthly_import_limit INTEGER NOT NULL DEFAULT 50,
    monthly_imports_used INTEGER NOT NULL DEFAULT 0,
    monthly_ai_cost_limit_usd DECIMAL(10, 2) NOT NULL DEFAULT 10.00,
    monthly_ai_cost_used_usd DECIMAL(10, 4) NOT NULL DEFAULT 0.00,
    
    -- Reset tracking
    limit_period_start DATE NOT NULL DEFAULT CURRENT_DATE,
    limit_period_end DATE NOT NULL DEFAULT (CURRENT_DATE + INTERVAL '1 month'),
    
    -- Rate limiting
    hourly_import_limit INTEGER NOT NULL DEFAULT 10,
    last_import_hour TIMESTAMPTZ,
    imports_this_hour INTEGER NOT NULL DEFAULT 0,
    
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Function to reset monthly limits
CREATE OR REPLACE FUNCTION reset_monthly_ai_limits()
RETURNS void AS $$
BEGIN
    UPDATE user_ai_usage_limits
    SET 
        monthly_imports_used = 0,
        monthly_ai_cost_used_usd = 0.00,
        limit_period_start = CURRENT_DATE,
        limit_period_end = CURRENT_DATE + INTERVAL '1 month',
        updated_at = NOW()
    WHERE limit_period_end < CURRENT_DATE;
END;
$$ LANGUAGE plpgsql;

COMMENT ON TABLE ai_import_sessions IS 'Tracks every AI-powered import session with full analysis and audit trail';
COMMENT ON TABLE ai_import_row_results IS 'Individual row processing results for each import session';
COMMENT ON TABLE ai_import_audit_log IS 'Immutable audit log of all import-related events';
COMMENT ON TABLE ai_api_usage IS 'Tracks AI API usage for cost monitoring and billing';
COMMENT ON TABLE user_ai_usage_limits IS 'Per-user AI usage limits and quotas';
