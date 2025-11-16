-- Migration: ERP Integration System (Oracle NetSuite & SAP S/4HANA)
-- Description: Complete bidirectional sync infrastructure with OAuth authentication
-- Author: Atlas Pharma Engineering Team
-- Date: 2025-01-15

-- ============================================================================
-- TABLE: erp_connections
-- Purpose: Store ERP connection configurations with encrypted credentials
-- ============================================================================
CREATE TABLE erp_connections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- ERP identification
    erp_type VARCHAR(50) NOT NULL CHECK (erp_type IN ('netsuite', 'sap_s4hana')),
    connection_name VARCHAR(100) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'paused', 'error', 'disabled')),

    -- NetSuite specific credentials (encrypted with AES-256-GCM)
    netsuite_account_id VARCHAR(50),
    netsuite_consumer_key TEXT,  -- Encrypted
    netsuite_consumer_secret TEXT,  -- Encrypted
    netsuite_token_id TEXT,  -- Encrypted
    netsuite_token_secret TEXT,  -- Encrypted
    netsuite_realm VARCHAR(50),  -- Account realm for OAuth signature

    -- SAP specific credentials (encrypted with AES-256-GCM)
    sap_base_url VARCHAR(255),
    sap_client_id TEXT,  -- Encrypted
    sap_client_secret TEXT,  -- Encrypted
    sap_token_endpoint VARCHAR(255),
    sap_environment VARCHAR(20) CHECK (sap_environment IN ('cloud', 'on_premise')),
    sap_plant VARCHAR(20),  -- Default plant for inventory operations
    sap_company_code VARCHAR(10),  -- SAP company code

    -- OAuth token cache (encrypted, short-lived)
    cached_access_token TEXT,  -- Encrypted OAuth2 token (SAP only)
    token_expires_at TIMESTAMPTZ,  -- When cached token expires

    -- Sync configuration
    sync_enabled BOOLEAN NOT NULL DEFAULT true,
    sync_frequency_minutes INTEGER NOT NULL DEFAULT 15 CHECK (sync_frequency_minutes >= 5),
    last_sync_at TIMESTAMPTZ,
    last_sync_status VARCHAR(50) CHECK (last_sync_status IN ('success', 'failed', 'partial', 'running')),
    last_sync_error TEXT,
    last_sync_duration_seconds INTEGER,

    -- Feature flags - what to sync
    sync_stock_levels BOOLEAN NOT NULL DEFAULT true,
    sync_product_master BOOLEAN NOT NULL DEFAULT true,
    sync_transactions BOOLEAN NOT NULL DEFAULT true,
    sync_lot_batch BOOLEAN NOT NULL DEFAULT true,

    -- Sync direction preferences
    default_sync_direction VARCHAR(20) NOT NULL DEFAULT 'bidirectional'
        CHECK (default_sync_direction IN ('atlas_to_erp', 'erp_to_atlas', 'bidirectional')),

    -- Conflict resolution strategy
    conflict_resolution VARCHAR(20) NOT NULL DEFAULT 'atlas_wins'
        CHECK (conflict_resolution IN ('atlas_wins', 'erp_wins', 'manual', 'latest_timestamp')),

    -- Custom field mappings (JSONB for flexibility)
    field_mappings JSONB DEFAULT '{}'::jsonb,

    -- Connection metadata
    api_version VARCHAR(20),  -- ERP API version being used
    last_test_at TIMESTAMPTZ,  -- Last successful connection test
    test_result JSONB,  -- Result of last test

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT unique_user_netsuite UNIQUE(user_id, erp_type, netsuite_account_id),
    CONSTRAINT unique_user_sap UNIQUE(user_id, erp_type, sap_base_url),
    CONSTRAINT netsuite_fields_required CHECK (
        (erp_type != 'netsuite') OR
        (netsuite_account_id IS NOT NULL AND
         netsuite_consumer_key IS NOT NULL AND
         netsuite_consumer_secret IS NOT NULL AND
         netsuite_token_id IS NOT NULL AND
         netsuite_token_secret IS NOT NULL)
    ),
    CONSTRAINT sap_fields_required CHECK (
        (erp_type != 'sap_s4hana') OR
        (sap_base_url IS NOT NULL AND
         sap_client_id IS NOT NULL AND
         sap_client_secret IS NOT NULL AND
         sap_token_endpoint IS NOT NULL)
    )
);

CREATE INDEX idx_erp_connections_user ON erp_connections(user_id);
CREATE INDEX idx_erp_connections_status ON erp_connections(status) WHERE sync_enabled = true;
CREATE INDEX idx_erp_connections_type ON erp_connections(erp_type);
CREATE INDEX idx_erp_connections_next_sync ON erp_connections(last_sync_at)
    WHERE sync_enabled = true AND status = 'active';

COMMENT ON TABLE erp_connections IS 'ERP system connections with encrypted OAuth credentials';
COMMENT ON COLUMN erp_connections.netsuite_consumer_key IS 'Encrypted NetSuite OAuth 1.0 consumer key';
COMMENT ON COLUMN erp_connections.cached_access_token IS 'Encrypted cached OAuth2 access token (SAP only)';


-- ============================================================================
-- TABLE: erp_inventory_mappings
-- Purpose: Map Atlas inventory items to ERP items for sync
-- ============================================================================
CREATE TABLE erp_inventory_mappings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    erp_connection_id UUID NOT NULL REFERENCES erp_connections(id) ON DELETE CASCADE,
    atlas_inventory_id UUID NOT NULL REFERENCES inventory(id) ON DELETE CASCADE,

    -- ERP-side identifiers
    erp_item_id VARCHAR(100) NOT NULL,  -- NetSuite internal ID or SAP material number
    erp_item_name VARCHAR(255),
    erp_location_id VARCHAR(100),  -- Warehouse/storage location in ERP
    erp_location_name VARCHAR(255),

    -- Sync control
    sync_enabled BOOLEAN NOT NULL DEFAULT true,
    sync_direction VARCHAR(20) NOT NULL DEFAULT 'bidirectional'
        CHECK (sync_direction IN ('atlas_to_erp', 'erp_to_atlas', 'bidirectional', 'disabled')),

    -- Last sync metadata
    last_synced_at TIMESTAMPTZ,
    last_sync_direction VARCHAR(20),
    last_sync_status VARCHAR(20) CHECK (last_sync_status IN ('success', 'failed', 'skipped', 'conflict')),
    last_sync_error TEXT,

    -- Conflict handling
    conflict_resolution VARCHAR(20) DEFAULT 'inherit'
        CHECK (conflict_resolution IN ('inherit', 'atlas_wins', 'erp_wins', 'manual', 'latest_timestamp')),
    pending_conflict BOOLEAN DEFAULT false,
    conflict_details JSONB,  -- Details of any pending conflict

    -- Field-level custom mappings (if ERP uses non-standard fields)
    quantity_field_path VARCHAR(100),  -- e.g., "quantityOnHand" or "locations.items[0].quantity"
    expiry_field_path VARCHAR(100),
    lot_field_path VARCHAR(100),
    ndc_field_path VARCHAR(100),

    -- Custom field mappings (JSONB for flexibility)
    custom_field_mappings JSONB DEFAULT '{}'::jsonb,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    UNIQUE(erp_connection_id, atlas_inventory_id),
    UNIQUE(erp_connection_id, erp_item_id, erp_location_id)
);

CREATE INDEX idx_erp_mappings_connection ON erp_inventory_mappings(erp_connection_id);
CREATE INDEX idx_erp_mappings_inventory ON erp_inventory_mappings(atlas_inventory_id);
CREATE INDEX idx_erp_mappings_erp_item ON erp_inventory_mappings(erp_connection_id, erp_item_id);
CREATE INDEX idx_erp_mappings_pending_conflicts ON erp_inventory_mappings(erp_connection_id)
    WHERE pending_conflict = true;

COMMENT ON TABLE erp_inventory_mappings IS 'Maps Atlas inventory to ERP items for bidirectional sync';
COMMENT ON COLUMN erp_inventory_mappings.conflict_resolution IS 'inherit = use connection default, or override per item';


-- ============================================================================
-- TABLE: erp_sync_logs
-- Purpose: Audit trail and history of all sync operations
-- ============================================================================
CREATE TABLE erp_sync_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    erp_connection_id UUID NOT NULL REFERENCES erp_connections(id) ON DELETE CASCADE,

    -- Sync metadata
    sync_type VARCHAR(50) NOT NULL CHECK (sync_type IN ('full_sync', 'incremental', 'real_time', 'manual', 'auto_discovery')),
    sync_direction VARCHAR(20) NOT NULL CHECK (sync_direction IN ('atlas_to_erp', 'erp_to_atlas', 'bidirectional')),
    triggered_by VARCHAR(50) NOT NULL CHECK (triggered_by IN ('scheduler', 'webhook', 'user_manual', 'api', 'auto_discovery')),

    -- User context (if manually triggered)
    triggered_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,

    -- Sync execution status
    status VARCHAR(20) NOT NULL CHECK (status IN ('running', 'success', 'failed', 'partial')),

    -- Sync results
    items_synced INTEGER NOT NULL DEFAULT 0,
    items_failed INTEGER NOT NULL DEFAULT 0,
    items_skipped INTEGER NOT NULL DEFAULT 0,
    items_created INTEGER NOT NULL DEFAULT 0,  -- New mappings created
    items_updated INTEGER NOT NULL DEFAULT 0,
    conflicts_detected INTEGER NOT NULL DEFAULT 0,

    -- Error tracking
    error_message TEXT,
    error_details JSONB,  -- Detailed errors per item
    error_stack_trace TEXT,

    -- Performance metrics
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    duration_seconds INTEGER,

    -- API call metrics
    api_calls_made INTEGER DEFAULT 0,
    api_errors INTEGER DEFAULT 0,
    api_retries INTEGER DEFAULT 0,

    -- Data transferred
    bytes_sent BIGINT DEFAULT 0,
    bytes_received BIGINT DEFAULT 0,

    -- Detailed sync results (JSONB for item-level details)
    sync_details JSONB,  -- Array of {item_id, status, error, etc.}

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sync_logs_connection ON erp_sync_logs(erp_connection_id, created_at DESC);
CREATE INDEX idx_sync_logs_status ON erp_sync_logs(status, created_at DESC);
CREATE INDEX idx_sync_logs_user ON erp_sync_logs(triggered_by_user_id) WHERE triggered_by_user_id IS NOT NULL;
CREATE INDEX idx_sync_logs_running ON erp_sync_logs(erp_connection_id, status) WHERE status = 'running';

COMMENT ON TABLE erp_sync_logs IS 'Complete audit trail of all ERP sync operations';


-- ============================================================================
-- TABLE: erp_webhooks
-- Purpose: Manage webhook subscriptions for real-time updates from ERP
-- ============================================================================
CREATE TABLE erp_webhooks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    erp_connection_id UUID NOT NULL REFERENCES erp_connections(id) ON DELETE CASCADE,

    -- Webhook configuration
    webhook_url VARCHAR(500) NOT NULL,  -- Atlas webhook endpoint (this will be our URL)
    webhook_secret VARCHAR(100) NOT NULL,  -- Secret for signature verification (we generate this)

    -- ERP-specific webhook registration
    erp_webhook_id VARCHAR(100),  -- ID returned by ERP when webhook is registered
    erp_subscription_id VARCHAR(100),  -- Subscription ID (for SAP Event Mesh)

    -- Event configuration
    event_types JSONB NOT NULL,  -- ['inventory.updated', 'item.created', 'stock.adjusted', etc.]

    -- Webhook status
    status VARCHAR(20) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'paused', 'failed', 'expired')),

    -- Activity tracking
    last_received_at TIMESTAMPTZ,
    total_events_received INTEGER NOT NULL DEFAULT 0,
    last_event_payload JSONB,  -- Last received event (for debugging)

    -- Error tracking
    consecutive_failures INTEGER NOT NULL DEFAULT 0,
    last_failure_at TIMESTAMPTZ,
    last_failure_reason TEXT,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,  -- Some webhooks expire and need re-registration

    UNIQUE(erp_connection_id, webhook_url)
);

CREATE INDEX idx_webhooks_connection ON erp_webhooks(erp_connection_id);
CREATE INDEX idx_webhooks_status ON erp_webhooks(status) WHERE status = 'active';
CREATE INDEX idx_webhooks_erp_id ON erp_webhooks(erp_webhook_id) WHERE erp_webhook_id IS NOT NULL;

COMMENT ON TABLE erp_webhooks IS 'Webhook subscriptions for real-time ERP updates';
COMMENT ON COLUMN erp_webhooks.webhook_secret IS 'Secret we generate for HMAC signature verification';


-- ============================================================================
-- TABLE: erp_conflict_queue
-- Purpose: Queue of sync conflicts requiring manual resolution
-- ============================================================================
CREATE TABLE erp_conflict_queue (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    erp_connection_id UUID NOT NULL REFERENCES erp_connections(id) ON DELETE CASCADE,
    erp_mapping_id UUID NOT NULL REFERENCES erp_inventory_mappings(id) ON DELETE CASCADE,

    -- Conflict details
    conflict_type VARCHAR(50) NOT NULL CHECK (conflict_type IN ('quantity_mismatch', 'price_mismatch', 'lot_mismatch', 'expiry_mismatch', 'item_deleted', 'field_conflict')),

    -- Current values
    atlas_value JSONB NOT NULL,  -- Current value in Atlas
    erp_value JSONB NOT NULL,  -- Current value in ERP

    -- Conflict metadata
    detected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    atlas_last_modified TIMESTAMPTZ,
    erp_last_modified TIMESTAMPTZ,

    -- Resolution
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'resolved', 'ignored', 'auto_resolved')),
    resolved_at TIMESTAMPTZ,
    resolved_by UUID REFERENCES users(id) ON DELETE SET NULL,
    resolution_action VARCHAR(50) CHECK (resolution_action IN ('use_atlas', 'use_erp', 'merge', 'ignore')),
    resolution_notes TEXT,

    -- Priority
    priority VARCHAR(20) NOT NULL DEFAULT 'medium' CHECK (priority IN ('low', 'medium', 'high', 'critical')),

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_conflict_queue_connection ON erp_conflict_queue(erp_connection_id, status);
CREATE INDEX idx_conflict_queue_mapping ON erp_conflict_queue(erp_mapping_id);
CREATE INDEX idx_conflict_queue_pending ON erp_conflict_queue(erp_connection_id, detected_at DESC)
    WHERE status = 'pending';

COMMENT ON TABLE erp_conflict_queue IS 'Queue of sync conflicts requiring manual user resolution';


-- ============================================================================
-- TABLE: erp_field_mapping_templates
-- Purpose: Predefined field mapping templates for common ERP configurations
-- ============================================================================
CREATE TABLE erp_field_mapping_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Template identification
    erp_type VARCHAR(50) NOT NULL CHECK (erp_type IN ('netsuite', 'sap_s4hana')),
    template_name VARCHAR(100) NOT NULL,
    description TEXT,

    -- Template category
    category VARCHAR(50) CHECK (category IN ('default', 'pharmaceutical', 'custom')),

    -- Field mappings (JSONB)
    field_mappings JSONB NOT NULL,

    -- Usage tracking
    is_default BOOLEAN DEFAULT false,
    usage_count INTEGER DEFAULT 0,

    -- Metadata
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,  -- NULL for system templates
    is_public BOOLEAN DEFAULT false,  -- Public templates visible to all users

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(erp_type, template_name, created_by)
);

CREATE INDEX idx_field_templates_type ON erp_field_mapping_templates(erp_type);
CREATE INDEX idx_field_templates_public ON erp_field_mapping_templates(erp_type, is_public) WHERE is_public = true;

COMMENT ON TABLE erp_field_mapping_templates IS 'Reusable field mapping templates for ERP integration setup';


-- ============================================================================
-- Insert default field mapping templates
-- ============================================================================

-- NetSuite default pharmaceutical template
INSERT INTO erp_field_mapping_templates (erp_type, template_name, description, category, is_default, is_public, field_mappings) VALUES
('netsuite', 'Default Pharmaceutical', 'Standard NetSuite inventory fields for pharmaceutical products', 'pharmaceutical', true, true,
'{
  "quantity": "quantityOnHand",
  "ndc_code": "custitem_ndc_code",
  "lot_number": "custitem_lot_number",
  "expiry_date": "custitem_expiry_date",
  "unit_price": "cost",
  "location": "locations.items[0].location.id",
  "manufacturer": "manufacturer.name"
}'::jsonb);

-- SAP S/4HANA default pharmaceutical template
INSERT INTO erp_field_mapping_templates (erp_type, template_name, description, category, is_default, is_public, field_mappings) VALUES
('sap_s4hana', 'Default Pharmaceutical', 'Standard SAP material master fields for pharmaceutical products', 'pharmaceutical', true, true,
'{
  "material_number": "Material",
  "quantity": "MatlWrhsStkQtyInMatlBaseUnit",
  "plant": "Plant",
  "storage_location": "StorageLocation",
  "batch": "Batch",
  "expiry_date": "YY1_ExpiryDate_MDI",
  "ndc_code": "YY1_NDCCode_MDI",
  "base_unit": "MaterialBaseUnit"
}'::jsonb);


-- ============================================================================
-- Function: Update erp_connections.updated_at on modification
-- ============================================================================
CREATE OR REPLACE FUNCTION update_erp_connections_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_erp_connections_updated_at
    BEFORE UPDATE ON erp_connections
    FOR EACH ROW
    EXECUTE FUNCTION update_erp_connections_updated_at();


-- ============================================================================
-- Function: Update erp_inventory_mappings.updated_at on modification
-- ============================================================================
CREATE TRIGGER trigger_erp_mappings_updated_at
    BEFORE UPDATE ON erp_inventory_mappings
    FOR EACH ROW
    EXECUTE FUNCTION update_erp_connections_updated_at();


-- ============================================================================
-- Function: Automatically mark connection as error if sync fails repeatedly
-- ============================================================================
CREATE OR REPLACE FUNCTION check_consecutive_sync_failures()
RETURNS TRIGGER AS $$
DECLARE
    failure_count INTEGER;
BEGIN
    IF NEW.status = 'failed' THEN
        -- Count consecutive failures in last 24 hours
        SELECT COUNT(*)
        INTO failure_count
        FROM erp_sync_logs
        WHERE erp_connection_id = NEW.erp_connection_id
          AND status = 'failed'
          AND created_at > NOW() - INTERVAL '24 hours'
        ORDER BY created_at DESC
        LIMIT 5;

        -- If 5 consecutive failures, mark connection as error
        IF failure_count >= 5 THEN
            UPDATE erp_connections
            SET status = 'error',
                last_sync_error = 'Automatic pause: 5 consecutive sync failures in 24 hours'
            WHERE id = NEW.erp_connection_id
              AND status = 'active';
        END IF;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_check_sync_failures
    AFTER INSERT ON erp_sync_logs
    FOR EACH ROW
    EXECUTE FUNCTION check_consecutive_sync_failures();


-- ============================================================================
-- Grant permissions
-- ============================================================================
-- These will be executed by application role with appropriate permissions
-- GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO atlas_app_role;


-- ============================================================================
-- Comments for documentation
-- ============================================================================
COMMENT ON COLUMN erp_connections.field_mappings IS 'Custom field mappings in format: {"atlas_field": "erp_field_path"}';
COMMENT ON COLUMN erp_inventory_mappings.custom_field_mappings IS 'Override connection-level mappings per item';
COMMENT ON COLUMN erp_sync_logs.sync_details IS 'Detailed per-item sync results: [{item_id, status, error, before, after}]';
