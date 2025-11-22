-- EMA Pharmaceutical Catalog Cache
-- Cached pharmaceutical data from European Medicines Agency for European market
-- Uses EMA ePI API (Electronic Product Information) as primary data source

CREATE TABLE IF NOT EXISTS ema_catalog (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Core EMA identifiers
    eu_number VARCHAR(50) UNIQUE NOT NULL,  -- EU/1/xx/xxx format from EMA
    pms_id VARCHAR(100),                     -- Product Management System ID (if available)
    bundle_id VARCHAR(100),                  -- ePI Bundle identifier
    epi_id VARCHAR(100),                     -- ePI Document identifier

    -- Product names and descriptions
    product_name TEXT NOT NULL,              -- Brand/trade name (primary)
    inn_name TEXT,                           -- International Nonproprietary Name (generic)
    therapeutic_indication TEXT,             -- Therapeutic indication summary

    -- Marketing Authorization
    mah_name TEXT NOT NULL,                  -- Marketing Authorization Holder
    mah_country VARCHAR(10),                 -- MAH country code
    authorization_status VARCHAR(50),        -- Authorized, Suspended, Withdrawn, Refused
    authorization_date DATE,                 -- Initial authorization date
    authorization_country VARCHAR(10),       -- EEA country or "EU" for centralized
    procedure_type VARCHAR(50),              -- Centralized, Decentralized, Mutual Recognition, National

    -- Product characteristics
    pharmaceutical_form TEXT,                -- Dosage form (tablet, solution, etc.)
    route_of_administration TEXT[],          -- Array of administration routes
    strength TEXT,                           -- Strength description
    active_substances JSONB,                 -- Active ingredients with strengths
    excipients JSONB,                       -- Excipients information

    -- Therapeutic classification
    atc_code VARCHAR(20),                    -- Anatomical Therapeutic Chemical code
    atc_classification TEXT,                 -- Full ATC classification
    therapeutic_area TEXT,                   -- Therapeutic area/disease area
    orphan_designation BOOLEAN DEFAULT false, -- Orphan drug designation status

    -- Regulatory and safety
    pharmacovigilance_status VARCHAR(50),    -- Pharmacovigilance requirements
    additional_monitoring BOOLEAN DEFAULT false, -- Additional safety monitoring required
    risk_management_plan BOOLEAN DEFAULT false, -- RMP required

    -- Language and documentation
    language_code VARCHAR(10) DEFAULT 'en',  -- ISO 639-1 language code
    epi_url TEXT,                            -- Link to full ePI document
    smpc_url TEXT,                           -- Link to Summary of Product Characteristics
    pil_url TEXT,                            -- Link to Patient Information Leaflet

    -- Raw API data for reference
    epi_data JSONB,                          -- Full ePI bundle response
    metadata JSONB,                          -- Additional EMA metadata

    -- Cache management
    last_synced_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,

    -- Search optimization (generated full-text search vector)
    search_vector tsvector GENERATED ALWAYS AS (
        setweight(to_tsvector('english', coalesce(product_name, '')), 'A') ||
        setweight(to_tsvector('english', coalesce(inn_name, '')), 'B') ||
        setweight(to_tsvector('english', coalesce(mah_name, '')), 'C') ||
        setweight(to_tsvector('english', coalesce(therapeutic_indication, '')), 'D') ||
        setweight(to_tsvector('english', coalesce(atc_classification, '')), 'D')
    ) STORED
);

-- Add comments for documentation
COMMENT ON TABLE ema_catalog IS 'Cached pharmaceutical data from EMA ePI API for European market';
COMMENT ON COLUMN ema_catalog.eu_number IS 'EMA EU number in format EU/1/XX/XXX/XXX';
COMMENT ON COLUMN ema_catalog.pms_id IS 'Product Management System ID from EMA';
COMMENT ON COLUMN ema_catalog.bundle_id IS 'ePI Bundle identifier for document retrieval';
COMMENT ON COLUMN ema_catalog.product_name IS 'Primary product/trade name';
COMMENT ON COLUMN ema_catalog.inn_name IS 'International Nonproprietary Name (generic name)';
COMMENT ON COLUMN ema_catalog.mah_name IS 'Marketing Authorization Holder company name';
COMMENT ON COLUMN ema_catalog.authorization_status IS 'Current authorization status from EMA';
COMMENT ON COLUMN ema_catalog.atc_code IS 'Anatomical Therapeutic Chemical classification code';
COMMENT ON COLUMN ema_catalog.language_code IS 'ISO 639-1 language code for this record';
COMMENT ON COLUMN ema_catalog.epi_data IS 'Raw ePI bundle data from EMA API';

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_ema_eu_number ON ema_catalog(eu_number);
CREATE INDEX IF NOT EXISTS idx_ema_product_name ON ema_catalog USING gin(to_tsvector('english', product_name));
CREATE INDEX IF NOT EXISTS idx_ema_inn_name ON ema_catalog USING gin(to_tsvector('english', inn_name));
CREATE INDEX IF NOT EXISTS idx_ema_mah_name ON ema_catalog USING gin(to_tsvector('english', mah_name));
CREATE INDEX IF NOT EXISTS idx_ema_search_vector ON ema_catalog USING gin(search_vector);
CREATE INDEX IF NOT EXISTS idx_ema_authorization_status ON ema_catalog(authorization_status);
CREATE INDEX IF NOT EXISTS idx_ema_atc_code ON ema_catalog(atc_code);
CREATE INDEX IF NOT EXISTS idx_ema_language_code ON ema_catalog(language_code);
CREATE INDEX IF NOT EXISTS idx_ema_mah_country ON ema_catalog(mah_country);
CREATE INDEX IF NOT EXISTS idx_ema_orphan_designation ON ema_catalog(orphan_designation);
CREATE INDEX IF NOT EXISTS idx_ema_last_synced ON ema_catalog(last_synced_at);

-- Composite indexes for common query patterns
CREATE INDEX IF NOT EXISTS idx_ema_status_country ON ema_catalog(authorization_status, authorization_country);
CREATE INDEX IF NOT EXISTS idx_ema_atc_status ON ema_catalog(atc_code, authorization_status);

-- Sync tracking table for monitoring data updates
CREATE TABLE IF NOT EXISTS ema_sync_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sync_started_at TIMESTAMP WITH TIME ZONE NOT NULL,
    sync_completed_at TIMESTAMP WITH TIME ZONE,

    -- Sync parameters
    language_code VARCHAR(10),              -- Language synced (en, de, fr, es, etc.)
    sync_type VARCHAR(20) DEFAULT 'full',   -- full, incremental, by_language
    record_limit INTEGER,                   -- Maximum records to sync

    -- Sync results
    records_fetched INTEGER DEFAULT 0,
    records_inserted INTEGER DEFAULT 0,
    records_updated INTEGER DEFAULT 0,
    records_skipped INTEGER DEFAULT 0,
    records_failed INTEGER DEFAULT 0,

    -- Status and error tracking
    status VARCHAR(20) DEFAULT 'in_progress', -- in_progress, completed, failed, cancelled
    error_message TEXT,                      -- Error details if failed
    warning_messages TEXT[],                 -- Array of warnings

    -- Performance metrics
    api_response_time_ms INTEGER,            -- Average API response time
    processing_time_ms INTEGER,              -- Total processing time

    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Add comments to sync log
COMMENT ON TABLE ema_sync_log IS 'Tracks EMA data synchronization operations and metrics';
COMMENT ON COLUMN ema_sync_log.language_code IS 'Language code for this sync operation';
COMMENT ON COLUMN ema_sync_log.sync_type IS 'Type of sync: full, incremental, or by_language';
COMMENT ON COLUMN ema_sync_log.status IS 'Current status: in_progress, completed, failed, cancelled';

-- Indexes for sync log queries
CREATE INDEX IF NOT EXISTS idx_ema_sync_log_status ON ema_sync_log(status);
CREATE INDEX IF NOT EXISTS idx_ema_sync_log_started_at ON ema_sync_log(sync_started_at DESC);
CREATE INDEX IF NOT EXISTS idx_ema_sync_log_completed_at ON ema_sync_log(sync_completed_at DESC);
CREATE INDEX IF NOT EXISTS idx_ema_sync_log_language ON ema_sync_log(language_code);

-- Trigger to automatically update updated_at timestamp
CREATE OR REPLACE FUNCTION update_ema_catalog_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger for automatic updated_at
CREATE TRIGGER trigger_update_ema_catalog_updated_at
    BEFORE UPDATE ON ema_catalog
    FOR EACH ROW
    EXECUTE FUNCTION update_ema_catalog_updated_at();