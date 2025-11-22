use sqlx::{PgPool, query, query_as, Row};
use uuid::Uuid;
use chrono::Utc;
use crate::models::openfda::{OpenFdaCatalogEntry, OpenFdaSyncLog, OpenFdaSearchRequest};
use crate::middleware::error_handling::{Result, AppError};

pub struct OpenFdaRepository {
    pool: PgPool,
}

impl OpenFdaRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert or update a catalog entry
    pub async fn upsert_entry(&self, entry: &OpenFdaCatalogEntry) -> Result<OpenFdaCatalogEntry> {
        let row = query_as::<_, OpenFdaCatalogEntry>(
            r#"
            INSERT INTO openfda_catalog (
                product_ndc, product_id, brand_name, brand_name_base, generic_name,
                labeler_name, dosage_form, route, strength, active_ingredients,
                product_type, marketing_category, pharm_class, dea_schedule,
                packaging, finished, marketing_start_date, listing_expiration_date,
                openfda_data, last_synced_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
            ON CONFLICT (product_ndc) DO UPDATE SET
                product_id = EXCLUDED.product_id,
                brand_name = EXCLUDED.brand_name,
                brand_name_base = EXCLUDED.brand_name_base,
                generic_name = EXCLUDED.generic_name,
                labeler_name = EXCLUDED.labeler_name,
                dosage_form = EXCLUDED.dosage_form,
                route = EXCLUDED.route,
                strength = EXCLUDED.strength,
                active_ingredients = EXCLUDED.active_ingredients,
                product_type = EXCLUDED.product_type,
                marketing_category = EXCLUDED.marketing_category,
                pharm_class = EXCLUDED.pharm_class,
                dea_schedule = EXCLUDED.dea_schedule,
                packaging = EXCLUDED.packaging,
                finished = EXCLUDED.finished,
                marketing_start_date = EXCLUDED.marketing_start_date,
                listing_expiration_date = EXCLUDED.listing_expiration_date,
                openfda_data = EXCLUDED.openfda_data,
                last_synced_at = EXCLUDED.last_synced_at,
                updated_at = CURRENT_TIMESTAMP
            RETURNING *
            "#
        )
        .bind(&entry.product_ndc)
        .bind(&entry.product_id)
        .bind(&entry.brand_name)
        .bind(&entry.brand_name_base)
        .bind(&entry.generic_name)
        .bind(&entry.labeler_name)
        .bind(&entry.dosage_form)
        .bind(&entry.route)
        .bind(&entry.strength)
        .bind(&entry.active_ingredients)
        .bind(&entry.product_type)
        .bind(&entry.marketing_category)
        .bind(&entry.pharm_class)
        .bind(&entry.dea_schedule)
        .bind(&entry.packaging)
        .bind(&entry.finished)
        .bind(&entry.marketing_start_date)
        .bind(&entry.listing_expiration_date)
        .bind(&entry.openfda_data)
        .bind(&entry.last_synced_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    /// Batch upsert multiple entries
    pub async fn batch_upsert(&self, entries: Vec<OpenFdaCatalogEntry>) -> Result<(i32, i32)> {
        let mut inserted = 0;
        let mut updated = 0;

        for entry in entries {
            let result = query(
                r#"
                INSERT INTO openfda_catalog (
                    product_ndc, product_id, brand_name, brand_name_base, generic_name,
                    labeler_name, dosage_form, route, strength, active_ingredients,
                    product_type, marketing_category, pharm_class, dea_schedule,
                    packaging, finished, marketing_start_date, listing_expiration_date,
                    openfda_data, last_synced_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
                ON CONFLICT (product_ndc) DO UPDATE SET
                    product_id = EXCLUDED.product_id,
                    brand_name = EXCLUDED.brand_name,
                    brand_name_base = EXCLUDED.brand_name_base,
                    generic_name = EXCLUDED.generic_name,
                    labeler_name = EXCLUDED.labeler_name,
                    dosage_form = EXCLUDED.dosage_form,
                    route = EXCLUDED.route,
                    strength = EXCLUDED.strength,
                    active_ingredients = EXCLUDED.active_ingredients,
                    product_type = EXCLUDED.product_type,
                    marketing_category = EXCLUDED.marketing_category,
                    pharm_class = EXCLUDED.pharm_class,
                    dea_schedule = EXCLUDED.dea_schedule,
                    packaging = EXCLUDED.packaging,
                    finished = EXCLUDED.finished,
                    marketing_start_date = EXCLUDED.marketing_start_date,
                    listing_expiration_date = EXCLUDED.listing_expiration_date,
                    openfda_data = EXCLUDED.openfda_data,
                    last_synced_at = EXCLUDED.last_synced_at,
                    updated_at = CURRENT_TIMESTAMP
                RETURNING (xmax = 0) AS was_inserted
                "#
            )
            .bind(&entry.product_ndc)
            .bind(&entry.product_id)
            .bind(&entry.brand_name)
            .bind(&entry.brand_name_base)
            .bind(&entry.generic_name)
            .bind(&entry.labeler_name)
            .bind(&entry.dosage_form)
            .bind(&entry.route)
            .bind(&entry.strength)
            .bind(&entry.active_ingredients)
            .bind(&entry.product_type)
            .bind(&entry.marketing_category)
            .bind(&entry.pharm_class)
            .bind(&entry.dea_schedule)
            .bind(&entry.packaging)
            .bind(&entry.finished)
            .bind(&entry.marketing_start_date)
            .bind(&entry.listing_expiration_date)
            .bind(&entry.openfda_data)
            .bind(&entry.last_synced_at)
            .fetch_one(&self.pool)
            .await?;

            let was_inserted: bool = result.try_get("was_inserted")?;
            if was_inserted {
                inserted += 1;
            } else {
                updated += 1;
            }
        }

        Ok((inserted, updated))
    }

    /// Search catalog with full-text search
    pub async fn search(&self, request: &OpenFdaSearchRequest) -> Result<Vec<OpenFdaCatalogEntry>> {
        let limit = request.limit.unwrap_or(20).min(100);
        let offset = request.offset.unwrap_or(0);

        let results = if let Some(ref query_text) = request.query {
            // Full-text search
            query_as::<_, OpenFdaCatalogEntry>(
                r#"
                SELECT * FROM openfda_catalog
                WHERE search_vector @@ plainto_tsquery('english', $1)
                   OR brand_name ILIKE $2
                   OR generic_name ILIKE $2
                   OR product_ndc ILIKE $2
                ORDER BY
                    ts_rank(search_vector, plainto_tsquery('english', $1)) DESC,
                    brand_name ASC
                LIMIT $3 OFFSET $4
                "#
            )
            .bind(query_text)
            .bind(format!("%{}%", query_text))
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            // Return recent entries if no query
            query_as::<_, OpenFdaCatalogEntry>(
                r#"
                SELECT * FROM openfda_catalog
                ORDER BY brand_name ASC
                LIMIT $1 OFFSET $2
                "#
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(results)
    }

    /// Find by NDC code
    pub async fn find_by_ndc(&self, ndc: &str) -> Result<Option<OpenFdaCatalogEntry>> {
        let entry = query_as::<_, OpenFdaCatalogEntry>(
            "SELECT * FROM openfda_catalog WHERE product_ndc = $1"
        )
        .bind(ndc)
        .fetch_optional(&self.pool)
        .await?;

        Ok(entry)
    }

    /// Get total count
    pub async fn get_total_count(&self) -> Result<i64> {
        let row = query("SELECT COUNT(*) as count FROM openfda_catalog")
            .fetch_one(&self.pool)
            .await?;

        let count: i64 = row.try_get("count")?;
        Ok(count)
    }

    /// Start a new sync log
    pub async fn start_sync_log(&self) -> Result<Uuid> {
        let row = query(
            r#"
            INSERT INTO openfda_sync_log (sync_started_at, status)
            VALUES ($1, 'in_progress')
            RETURNING id
            "#
        )
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await?;

        let id: Uuid = row.try_get("id")?;
        Ok(id)
    }

    /// Complete sync log
    pub async fn complete_sync_log(
        &self,
        log_id: Uuid,
        records_fetched: i32,
        records_inserted: i32,
        records_updated: i32,
    ) -> Result<()> {
        query(
            r#"
            UPDATE openfda_sync_log
            SET sync_completed_at = $1,
                records_fetched = $2,
                records_inserted = $3,
                records_updated = $4,
                status = 'completed'
            WHERE id = $5
            "#
        )
        .bind(Utc::now())
        .bind(records_fetched)
        .bind(records_inserted)
        .bind(records_updated)
        .bind(log_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Fail sync log
    pub async fn fail_sync_log(&self, log_id: Uuid, error_message: &str) -> Result<()> {
        query(
            r#"
            UPDATE openfda_sync_log
            SET sync_completed_at = $1,
                status = 'failed',
                error_message = $2
            WHERE id = $3
            "#
        )
        .bind(Utc::now())
        .bind(error_message)
        .bind(log_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get last successful sync
    pub async fn get_last_successful_sync(&self) -> Result<Option<OpenFdaSyncLog>> {
        let log = query_as::<_, OpenFdaSyncLog>(
            r#"
            SELECT * FROM openfda_sync_log
            WHERE status = 'completed'
            ORDER BY sync_completed_at DESC
            LIMIT 1
            "#
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(log)
    }

    /// Start a new sync log with sync type and expected total
    pub async fn start_sync_log_with_type(&self, sync_type: &str, total_expected: Option<i32>) -> Result<Uuid> {
        let total_batches = total_expected.map(|t| (t + 99) / 100); // Batch size of 100
        let row = query(
            r#"
            INSERT INTO openfda_sync_log (
                sync_started_at, status, sync_type, total_expected, total_batches,
                records_processed, records_inserted, records_updated, records_skipped, records_failed,
                current_batch
            )
            VALUES ($1, 'in_progress', $2, $3, $4, 0, 0, 0, 0, 0, 0)
            RETURNING id
            "#
        )
        .bind(Utc::now())
        .bind(sync_type)
        .bind(total_expected)
        .bind(total_batches)
        .fetch_one(&self.pool)
        .await?;

        let id: Uuid = row.try_get("id")?;
        Ok(id)
    }

    /// Update sync progress
    pub async fn update_sync_progress(
        &self,
        log_id: Uuid,
        records_processed: i32,
        records_inserted: i32,
        records_updated: i32,
        records_skipped: i32,
        records_failed: i32,
        current_batch: i32,
        api_response_time_ms: i32,
    ) -> Result<()> {
        query(
            r#"
            UPDATE openfda_sync_log
            SET records_processed = $2,
                records_inserted = $3,
                records_updated = $4,
                records_skipped = $5,
                records_failed = $6,
                current_batch = $7,
                api_response_time_ms = COALESCE(api_response_time_ms, 0) + $8
            WHERE id = $1
            "#
        )
        .bind(log_id)
        .bind(records_processed)
        .bind(records_inserted)
        .bind(records_updated)
        .bind(records_skipped)
        .bind(records_failed)
        .bind(current_batch)
        .bind(api_response_time_ms)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Set total expected records (after first API call)
    pub async fn set_total_expected(&self, log_id: Uuid, total_expected: i32) -> Result<()> {
        let total_batches = (total_expected + 99) / 100;
        query(
            r#"
            UPDATE openfda_sync_log
            SET total_expected = $2, total_batches = $3
            WHERE id = $1
            "#
        )
        .bind(log_id)
        .bind(total_expected)
        .bind(total_batches)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Complete sync log with final stats
    pub async fn complete_sync_log_full(
        &self,
        log_id: Uuid,
        records_fetched: i32,
        records_inserted: i32,
        records_updated: i32,
        records_skipped: i32,
        records_failed: i32,
        processing_time_ms: i32,
    ) -> Result<()> {
        query(
            r#"
            UPDATE openfda_sync_log
            SET sync_completed_at = $1,
                records_fetched = $2,
                records_inserted = $3,
                records_updated = $4,
                records_skipped = $5,
                records_failed = $6,
                records_processed = $2,
                processing_time_ms = $7,
                status = 'completed'
            WHERE id = $8
            "#
        )
        .bind(Utc::now())
        .bind(records_fetched)
        .bind(records_inserted)
        .bind(records_updated)
        .bind(records_skipped)
        .bind(records_failed)
        .bind(processing_time_ms)
        .bind(log_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get sync log by ID
    pub async fn get_sync_log(&self, log_id: Uuid) -> Result<Option<OpenFdaSyncLog>> {
        let log = query_as::<_, OpenFdaSyncLog>(
            "SELECT * FROM openfda_sync_log WHERE id = $1"
        )
        .bind(log_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(log)
    }

    /// Get active sync (if any)
    pub async fn get_active_sync(&self) -> Result<Option<OpenFdaSyncLog>> {
        let log = query_as::<_, OpenFdaSyncLog>(
            r#"
            SELECT * FROM openfda_sync_log
            WHERE status = 'in_progress'
            ORDER BY sync_started_at DESC
            LIMIT 1
            "#
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(log)
    }

    /// Get sync logs with pagination
    pub async fn get_sync_logs(&self, limit: i64, offset: i64) -> Result<Vec<OpenFdaSyncLog>> {
        let logs = query_as::<_, OpenFdaSyncLog>(
            r#"
            SELECT * FROM openfda_sync_log
            ORDER BY sync_started_at DESC
            LIMIT $1 OFFSET $2
            "#
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(logs)
    }

    /// Cancel a running sync
    pub async fn cancel_sync(&self, log_id: Uuid, cancelled_by: Uuid) -> Result<bool> {
        let result = query(
            r#"
            UPDATE openfda_sync_log
            SET status = 'cancelled',
                cancelled_at = $1,
                cancelled_by = $2,
                sync_completed_at = $1
            WHERE id = $3 AND status = 'in_progress'
            "#
        )
        .bind(Utc::now())
        .bind(cancelled_by)
        .bind(log_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Check if there's an active sync running
    pub async fn is_sync_running(&self) -> Result<bool> {
        let row = query(
            "SELECT EXISTS(SELECT 1 FROM openfda_sync_log WHERE status = 'in_progress') as running"
        )
        .fetch_one(&self.pool)
        .await?;

        let running: bool = row.try_get("running")?;
        Ok(running)
    }

    /// Clean up old sync logs (keep last N days)
    pub async fn cleanup_old_logs(&self, days_to_keep: i32) -> Result<i64> {
        let result = query(
            r#"
            DELETE FROM openfda_sync_log
            WHERE sync_started_at < NOW() - INTERVAL '1 day' * $1
              AND status != 'in_progress'
            "#
        )
        .bind(days_to_keep)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }
}
