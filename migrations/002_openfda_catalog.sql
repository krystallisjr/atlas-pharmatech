-- OpenFDA Pharmaceutical Catalog Cache
-- This table caches pharmaceutical data from the FDA OpenFDA API
-- Updated periodically to provide fast autocomplete and search functionality

CREATE TABLE IF NOT EXISTS openfda_catalog (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Core product identifiers
    product_ndc VARCHAR(20) UNIQUE NOT NULL,
    product_id VARCHAR(100),

    -- Names and descriptions
    brand_name TEXT NOT NULL,
    brand_name_base TEXT,
    generic_name TEXT NOT NULL,

    -- Manufacturer/Labeler info
    labeler_name TEXT NOT NULL,

    -- Product characteristics
    dosage_form TEXT,
    route TEXT[], -- Array of administration routes
    strength TEXT, -- Combined strength from active ingredients
    active_ingredients JSONB, -- Full active ingredient details

    -- Classification
    product_type TEXT,
    marketing_category TEXT,
    pharm_class TEXT[], -- Array of pharmaceutical classes
    dea_schedule VARCHAR(10), -- Drug Enforcement Administration schedule

    -- Packaging info
    packaging JSONB, -- Full packaging details

    -- Status and dates
    finished BOOLEAN DEFAULT true,
    marketing_start_date DATE,
    listing_expiration_date DATE,

    -- OpenFDA metadata
    openfda_data JSONB, -- Full openfda object for additional info

    -- Cache metadata
    last_synced_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,

    -- Search optimization
    search_vector tsvector GENERATED ALWAYS AS (
        setweight(to_tsvector('english', coalesce(brand_name, '')), 'A') ||
        setweight(to_tsvector('english', coalesce(generic_name, '')), 'B') ||
        setweight(to_tsvector('english', coalesce(labeler_name, '')), 'C') ||
        setweight(to_tsvector('english', coalesce(product_ndc, '')), 'D')
    ) STORED
);

-- Indexes for fast searching
CREATE INDEX idx_openfda_brand_name ON openfda_catalog USING gin(to_tsvector('english', brand_name));
CREATE INDEX idx_openfda_generic_name ON openfda_catalog USING gin(to_tsvector('english', generic_name));
CREATE INDEX idx_openfda_labeler_name ON openfda_catalog USING gin(to_tsvector('english', labeler_name));
CREATE INDEX idx_openfda_product_ndc ON openfda_catalog(product_ndc);
CREATE INDEX idx_openfda_search_vector ON openfda_catalog USING gin(search_vector);
CREATE INDEX idx_openfda_marketing_category ON openfda_catalog(marketing_category);
CREATE INDEX idx_openfda_dosage_form ON openfda_catalog(dosage_form);
CREATE INDEX idx_openfda_last_synced ON openfda_catalog(last_synced_at);

-- Trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_openfda_catalog_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_openfda_catalog_updated_at
    BEFORE UPDATE ON openfda_catalog
    FOR EACH ROW
    EXECUTE FUNCTION update_openfda_catalog_updated_at();

-- Sync tracking table to manage incremental updates
CREATE TABLE IF NOT EXISTS openfda_sync_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sync_started_at TIMESTAMP WITH TIME ZONE NOT NULL,
    sync_completed_at TIMESTAMP WITH TIME ZONE,
    records_fetched INTEGER DEFAULT 0,
    records_inserted INTEGER DEFAULT 0,
    records_updated INTEGER DEFAULT 0,
    status VARCHAR(20) DEFAULT 'in_progress', -- 'in_progress', 'completed', 'failed'
    error_message TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_sync_log_status ON openfda_sync_log(status);
CREATE INDEX idx_sync_log_started_at ON openfda_sync_log(sync_started_at DESC);

-- Comments for documentation
COMMENT ON TABLE openfda_catalog IS 'Cached pharmaceutical data from FDA OpenFDA API for fast autocomplete and search';
COMMENT ON COLUMN openfda_catalog.product_ndc IS 'National Drug Code (NDC) - unique product identifier';
COMMENT ON COLUMN openfda_catalog.search_vector IS 'Generated full-text search vector for fast searching';
COMMENT ON COLUMN openfda_catalog.last_synced_at IS 'Timestamp of last sync from OpenFDA API';
