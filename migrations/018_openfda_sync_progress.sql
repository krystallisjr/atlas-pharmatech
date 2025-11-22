-- OpenFDA Sync Progress Tracking Enhancement
-- Adds columns for real-time progress tracking and performance metrics

-- Add progress tracking columns to sync_log
ALTER TABLE openfda_sync_log
ADD COLUMN IF NOT EXISTS total_expected INTEGER DEFAULT 0,
ADD COLUMN IF NOT EXISTS records_processed INTEGER DEFAULT 0,
ADD COLUMN IF NOT EXISTS records_skipped INTEGER DEFAULT 0,
ADD COLUMN IF NOT EXISTS records_failed INTEGER DEFAULT 0,
ADD COLUMN IF NOT EXISTS current_batch INTEGER DEFAULT 0,
ADD COLUMN IF NOT EXISTS total_batches INTEGER DEFAULT 0,
ADD COLUMN IF NOT EXISTS api_response_time_ms INTEGER,
ADD COLUMN IF NOT EXISTS processing_time_ms INTEGER,
ADD COLUMN IF NOT EXISTS sync_type VARCHAR(20) DEFAULT 'full',
ADD COLUMN IF NOT EXISTS cancelled_at TIMESTAMP WITH TIME ZONE,
ADD COLUMN IF NOT EXISTS cancelled_by UUID REFERENCES users(id);

-- Index for finding active syncs
CREATE INDEX IF NOT EXISTS idx_openfda_sync_log_active
ON openfda_sync_log(status)
WHERE status = 'in_progress';

-- Index for sync type filtering
CREATE INDEX IF NOT EXISTS idx_openfda_sync_log_type
ON openfda_sync_log(sync_type);

-- Comments
COMMENT ON COLUMN openfda_sync_log.total_expected IS 'Total records expected from API meta.results.total';
COMMENT ON COLUMN openfda_sync_log.records_processed IS 'Running count of records processed (inserted + updated + skipped)';
COMMENT ON COLUMN openfda_sync_log.records_skipped IS 'Records skipped due to validation errors';
COMMENT ON COLUMN openfda_sync_log.records_failed IS 'Records that failed to insert/update';
COMMENT ON COLUMN openfda_sync_log.current_batch IS 'Current batch number being processed';
COMMENT ON COLUMN openfda_sync_log.total_batches IS 'Estimated total batches based on total_expected';
COMMENT ON COLUMN openfda_sync_log.api_response_time_ms IS 'Cumulative API response time in milliseconds';
COMMENT ON COLUMN openfda_sync_log.processing_time_ms IS 'Total processing time in milliseconds';
COMMENT ON COLUMN openfda_sync_log.sync_type IS 'Type of sync: full, incremental, manual';
COMMENT ON COLUMN openfda_sync_log.cancelled_at IS 'Timestamp when sync was cancelled';
COMMENT ON COLUMN openfda_sync_log.cancelled_by IS 'User who cancelled the sync';
