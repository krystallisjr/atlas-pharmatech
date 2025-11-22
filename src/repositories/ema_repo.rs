use sqlx::{PgPool, query, query_as, Row};
use uuid::Uuid;
use chrono::Utc;
use crate::models::ema::{
    EmaCatalogEntry, EmaSyncLog, EmaSearchRequest,
    EmaCatalogStats, LanguageCount, StatusCount, TherapeuticAreaCount
};
use crate::middleware::error_handling::{Result, AppError};

pub struct EmaRepository {
    pub pool: PgPool,
}

impl EmaRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ============================================================================
    // CRUD Operations
    // ============================================================================

    /// Batch upsert multiple catalog entries efficiently
    pub async fn batch_upsert(&self, entries: Vec<EmaCatalogEntry>) -> Result<(i32, i32)> {
        if entries.is_empty() {
            return Ok((0, 0));
        }

        let mut inserted = 0;
        let mut updated = 0;

        // Process in batches to avoid overwhelming the database
        const BATCH_SIZE: usize = 50;
        for batch in entries.chunks(BATCH_SIZE) {
            let (batch_inserted, batch_updated) = self.process_batch(batch).await?;
            inserted += batch_inserted;
            updated += batch_updated;
        }

        Ok((inserted, updated))
    }

    /// Process a single batch of entries
    async fn process_batch(&self, entries: &[EmaCatalogEntry]) -> Result<(i32, i32)> {
        let mut inserted = 0;
        let mut updated = 0;

        for entry in entries {
            let result = query(
                r#"
                INSERT INTO ema_catalog (
                    eu_number, pms_id, bundle_id, epi_id, product_name, inn_name,
                    therapeutic_indication, mah_name, mah_country, authorization_status,
                    authorization_date, authorization_country, procedure_type,
                    pharmaceutical_form, route_of_administration, strength, active_substances,
                    excipients, atc_code, atc_classification, therapeutic_area,
                    orphan_designation, pharmacovigilance_status, additional_monitoring,
                    risk_management_plan, language_code, epi_url, smpc_url, pil_url,
                    epi_data, metadata, last_synced_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32)
                ON CONFLICT (eu_number) DO UPDATE SET
                    pms_id = EXCLUDED.pms_id,
                    bundle_id = EXCLUDED.bundle_id,
                    epi_id = EXCLUDED.epi_id,
                    product_name = EXCLUDED.product_name,
                    inn_name = EXCLUDED.inn_name,
                    therapeutic_indication = EXCLUDED.therapeutic_indication,
                    mah_name = EXCLUDED.mah_name,
                    mah_country = EXCLUDED.mah_country,
                    authorization_status = EXCLUDED.authorization_status,
                    authorization_date = EXCLUDED.authorization_date,
                    authorization_country = EXCLUDED.authorization_country,
                    procedure_type = EXCLUDED.procedure_type,
                    pharmaceutical_form = EXCLUDED.pharmaceutical_form,
                    route_of_administration = EXCLUDED.route_of_administration,
                    strength = EXCLUDED.strength,
                    active_substances = EXCLUDED.active_substances,
                    excipients = EXCLUDED.excipients,
                    atc_code = EXCLUDED.atc_code,
                    atc_classification = EXCLUDED.atc_classification,
                    therapeutic_area = EXCLUDED.therapeutic_area,
                    orphan_designation = EXCLUDED.orphan_designation,
                    pharmacovigilance_status = EXCLUDED.pharmacovigilance_status,
                    additional_monitoring = EXCLUDED.additional_monitoring,
                    risk_management_plan = EXCLUDED.risk_management_plan,
                    language_code = EXCLUDED.language_code,
                    epi_url = EXCLUDED.epi_url,
                    smpc_url = EXCLUDED.smpc_url,
                    pil_url = EXCLUDED.pil_url,
                    epi_data = EXCLUDED.epi_data,
                    metadata = EXCLUDED.metadata,
                    last_synced_at = EXCLUDED.last_synced_at,
                    updated_at = CURRENT_TIMESTAMP
                RETURNING (xmax = 0) AS was_inserted
                "#
            )
            .bind(&entry.eu_number)
            .bind(&entry.pms_id)
            .bind(&entry.bundle_id)
            .bind(&entry.epi_id)
            .bind(&entry.product_name)
            .bind(&entry.inn_name)
            .bind(&entry.therapeutic_indication)
            .bind(&entry.mah_name)
            .bind(&entry.mah_country)
            .bind(&entry.authorization_status)
            .bind(&entry.authorization_date)
            .bind(&entry.authorization_country)
            .bind(&entry.procedure_type)
            .bind(&entry.pharmaceutical_form)
            .bind(&entry.route_of_administration)
            .bind(&entry.strength)
            .bind(&entry.active_substances)
            .bind(&entry.excipients)
            .bind(&entry.atc_code)
            .bind(&entry.atc_classification)
            .bind(&entry.therapeutic_area)
            .bind(&entry.orphan_designation)
            .bind(&entry.pharmacovigilance_status)
            .bind(&entry.additional_monitoring)
            .bind(&entry.risk_management_plan)
            .bind(&entry.language_code)
            .bind(&entry.epi_url)
            .bind(&entry.smpc_url)
            .bind(&entry.pil_url)
            .bind(&entry.epi_data)
            .bind(&entry.metadata)
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

    // ============================================================================
    // Search Operations
    // ============================================================================

    /// Search catalog with full-text search and filters
    pub async fn search(&self, request: &EmaSearchRequest) -> Result<Vec<EmaCatalogEntry>> {
        let limit = request.limit.unwrap_or(20).min(100); // Cap at 100 for performance
        let offset = request.offset.unwrap_or(0);

        // Handle text search first (following OpenFDA pattern)
        if let Some(ref query_text) = request.query {
            if !query_text.trim().is_empty() {
                return self.search_with_text(query_text, request, limit, offset).await;
            }
        }

        // Handle filtered search without text
        self.search_with_filters(request, limit, offset).await
    }

    /// Search with text query (following OpenFDA pattern)
    async fn search_with_text(&self, query_text: &str, request: &EmaSearchRequest, limit: i64, offset: i64) -> Result<Vec<EmaCatalogEntry>> {
        match (request.language.as_ref(), request.authorization_status.as_ref()) {
            (Some(language), Some(status)) => {
                self.search_text_with_language_and_status(query_text, language, status, request, limit, offset).await
            }
            (Some(language), None) => {
                self.search_text_with_language(query_text, language, request, limit, offset).await
            }
            (None, Some(status)) => {
                self.search_text_with_status(query_text, status, request, limit, offset).await
            }
            (None, None) => {
                self.search_text_only(query_text, limit, offset).await
            }
        }
    }

    /// Text search only (base case)
    async fn search_text_only(&self, query_text: &str, limit: i64, offset: i64) -> Result<Vec<EmaCatalogEntry>> {
        query_as::<_, EmaCatalogEntry>(
            r#"
            SELECT * FROM ema_catalog
            WHERE search_vector @@ plainto_tsquery('english', $1)
               OR product_name ILIKE $2
               OR inn_name ILIKE $2
               OR eu_number ILIKE $2
               OR mah_name ILIKE $2
            ORDER BY ts_rank(search_vector, plainto_tsquery('english', $1)) DESC, product_name ASC
            LIMIT $3 OFFSET $4
            "#
        )
        .bind(query_text)
        .bind(format!("%{}%", query_text))
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Text search query failed: {}", e)))
    }

    /// Text search with language filter
    async fn search_text_with_language(
        &self,
        query_text: &str,
        language: &str,
        request: &EmaSearchRequest,
        limit: i64,
        offset: i64
    ) -> Result<Vec<EmaCatalogEntry>> {
        if let Some(ref therapeutic_area) = request.therapeutic_area {
            query_as::<_, EmaCatalogEntry>(
                r#"
                SELECT * FROM ema_catalog
                WHERE (search_vector @@ plainto_tsquery('english', $1)
                       OR product_name ILIKE $2
                       OR inn_name ILIKE $2
                       OR eu_number ILIKE $2
                       OR mah_name ILIKE $2)
                  AND language_code = $3
                  AND therapeutic_area ILIKE $4
                ORDER BY ts_rank(search_vector, plainto_tsquery('english', $1)) DESC, product_name ASC
                LIMIT $5 OFFSET $6
                "#
            )
            .bind(query_text)
            .bind(format!("%{}%", query_text))
            .bind(language)
            .bind(format!("%{}%", therapeutic_area))
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Text search with language and therapeutic area failed: {}", e)))
        } else {
            query_as::<_, EmaCatalogEntry>(
                r#"
                SELECT * FROM ema_catalog
                WHERE (search_vector @@ plainto_tsquery('english', $1)
                       OR product_name ILIKE $2
                       OR inn_name ILIKE $2
                       OR eu_number ILIKE $2
                       OR mah_name ILIKE $2)
                  AND language_code = $3
                ORDER BY ts_rank(search_vector, plainto_tsquery('english', $1)) DESC, product_name ASC
                LIMIT $4 OFFSET $5
                "#
            )
            .bind(query_text)
            .bind(format!("%{}%", query_text))
            .bind(language)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Text search with language failed: {}", e)))
        }
    }

    /// Text search with status filter
    async fn search_text_with_status(
        &self,
        query_text: &str,
        status: &str,
        request: &EmaSearchRequest,
        limit: i64,
        offset: i64
    ) -> Result<Vec<EmaCatalogEntry>> {
        query_as::<_, EmaCatalogEntry>(
            r#"
            SELECT * FROM ema_catalog
            WHERE (search_vector @@ plainto_tsquery('english', $1)
                   OR product_name ILIKE $2
                   OR inn_name ILIKE $2
                   OR eu_number ILIKE $2
                   OR mah_name ILIKE $2)
              AND authorization_status = $3
            ORDER BY ts_rank(search_vector, plainto_tsquery('english', $1)) DESC, product_name ASC
            LIMIT $4 OFFSET $5
            "#
        )
        .bind(query_text)
        .bind(format!("%{}%", query_text))
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Text search with status failed: {}", e)))
    }

    /// Text search with language and status filters
    async fn search_text_with_language_and_status(
        &self,
        query_text: &str,
        language: &str,
        status: &str,
        request: &EmaSearchRequest,
        limit: i64,
        offset: i64
    ) -> Result<Vec<EmaCatalogEntry>> {
        query_as::<_, EmaCatalogEntry>(
            r#"
            SELECT * FROM ema_catalog
            WHERE (search_vector @@ plainto_tsquery('english', $1)
                   OR product_name ILIKE $2
                   OR inn_name ILIKE $2
                   OR eu_number ILIKE $2
                   OR mah_name ILIKE $2)
              AND language_code = $3
              AND authorization_status = $4
            ORDER BY ts_rank(search_vector, plainto_tsquery('english', $1)) DESC, product_name ASC
            LIMIT $5 OFFSET $6
            "#
        )
        .bind(query_text)
        .bind(format!("%{}%", query_text))
        .bind(language)
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Text search with language and status failed: {}", e)))
    }

    /// Search with filters only (no text query)
    async fn search_with_filters(&self, request: &EmaSearchRequest, limit: i64, offset: i64) -> Result<Vec<EmaCatalogEntry>> {
        match (request.language.as_ref(), request.authorization_status.as_ref()) {
            (Some(language), Some(status)) => {
                self.search_by_language_and_status(language, status, request, limit, offset).await
            }
            (Some(language), None) => {
                self.search_by_language(language, request, limit, offset).await
            }
            (None, Some(status)) => {
                self.search_by_status(status, request, limit, offset).await
            }
            (None, None) => {
                self.search_all(request, limit, offset).await
            }
        }
    }

    /// Search by language and status filters
    async fn search_by_language_and_status(
        &self,
        language: &str,
        status: &str,
        request: &EmaSearchRequest,
        limit: i64,
        offset: i64
    ) -> Result<Vec<EmaCatalogEntry>> {
        query_as::<_, EmaCatalogEntry>(
            r#"
            SELECT * FROM ema_catalog
            WHERE language_code = $1 AND authorization_status = $2
            ORDER BY product_name ASC LIMIT $3 OFFSET $4
            "#
        )
        .bind(language)
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Language and status search failed: {}", e)))
    }

    /// Search by language filter only
    async fn search_by_language(&self, language: &str, request: &EmaSearchRequest, limit: i64, offset: i64) -> Result<Vec<EmaCatalogEntry>> {
        query_as::<_, EmaCatalogEntry>(
            r#"
            SELECT * FROM ema_catalog
            WHERE language_code = $1
            ORDER BY product_name ASC LIMIT $2 OFFSET $3
            "#
        )
        .bind(language)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Language search failed: {}", e)))
    }

    /// Search by status filter only
    async fn search_by_status(&self, status: &str, request: &EmaSearchRequest, limit: i64, offset: i64) -> Result<Vec<EmaCatalogEntry>> {
        query_as::<_, EmaCatalogEntry>(
            r#"
            SELECT * FROM ema_catalog
            WHERE authorization_status = $1
            ORDER BY product_name ASC LIMIT $2 OFFSET $3
            "#
        )
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Status search failed: {}", e)))
    }

    /// Search all (no filters)
    async fn search_all(&self, _request: &EmaSearchRequest, limit: i64, offset: i64) -> Result<Vec<EmaCatalogEntry>> {
        query_as::<_, EmaCatalogEntry>(
            r#"
            SELECT * FROM ema_catalog
            ORDER BY product_name ASC LIMIT $1 OFFSET $2
            "#
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("All search failed: {}", e)))
    }

    /// Find catalog entry by EU number
    pub async fn find_by_eu_number(&self, eu_number: &str) -> Result<Option<EmaCatalogEntry>> {
        let entry = query_as::<_, EmaCatalogEntry>(
            "SELECT * FROM ema_catalog WHERE eu_number = $1"
        )
        .bind(eu_number)
        .fetch_optional(&self.pool)
        .await?;

        Ok(entry)
    }

    /// Get total count of catalog entries
    pub async fn get_total_count(&self) -> Result<i64> {
        let row = query("SELECT COUNT(*) as count FROM ema_catalog")
            .fetch_one(&self.pool)
            .await?;

        let count: i64 = row.try_get("count")?;
        Ok(count)
    }

    /// Get count with filters
    pub async fn get_count_with_filters(&self, request: &EmaSearchRequest) -> Result<i64> {
        // Handle text search first
        if let Some(ref query_text) = request.query {
            if !query_text.trim().is_empty() {
                return self.get_count_with_text(query_text, request).await;
            }
        }

        // Handle filtered count without text
        self.get_count_filters_only(request).await
    }

    /// Get count with text search
    async fn get_count_with_text(&self, query_text: &str, request: &EmaSearchRequest) -> Result<i64> {
        match (request.language.as_ref(), request.authorization_status.as_ref()) {
            (Some(language), Some(status)) => {
                self.get_count_text_with_language_and_status(query_text, language, status).await
            }
            (Some(language), None) => {
                self.get_count_text_with_language(query_text, language).await
            }
            (None, Some(status)) => {
                self.get_count_text_with_status(query_text, status).await
            }
            (None, None) => {
                self.get_count_text_only(query_text).await
            }
        }
    }

    /// Get count with text search only
    async fn get_count_text_only(&self, query_text: &str) -> Result<i64> {
        let row = query(
            r#"
            SELECT COUNT(*) as count FROM ema_catalog
            WHERE search_vector @@ plainto_tsquery('english', $1)
               OR product_name ILIKE $2
               OR inn_name ILIKE $2
               OR eu_number ILIKE $2
               OR mah_name ILIKE $2
            "#
        )
        .bind(query_text)
        .bind(format!("%{}%", query_text))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Count text search failed: {}", e)))?;

        let count: i64 = row.try_get("count")?;
        Ok(count)
    }

    /// Get count with text and language
    async fn get_count_text_with_language(&self, query_text: &str, language: &str) -> Result<i64> {
        let row = query(
            r#"
            SELECT COUNT(*) as count FROM ema_catalog
            WHERE (search_vector @@ plainto_tsquery('english', $1)
                   OR product_name ILIKE $2
                   OR inn_name ILIKE $2
                   OR eu_number ILIKE $2
                   OR mah_name ILIKE $2)
              AND language_code = $3
            "#
        )
        .bind(query_text)
        .bind(format!("%{}%", query_text))
        .bind(language)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Count text search with language failed: {}", e)))?;

        let count: i64 = row.try_get("count")?;
        Ok(count)
    }

    /// Get count with text and status
    async fn get_count_text_with_status(&self, query_text: &str, status: &str) -> Result<i64> {
        let row = query(
            r#"
            SELECT COUNT(*) as count FROM ema_catalog
            WHERE (search_vector @@ plainto_tsquery('english', $1)
                   OR product_name ILIKE $2
                   OR inn_name ILIKE $2
                   OR eu_number ILIKE $2
                   OR mah_name ILIKE $2)
              AND authorization_status = $3
            "#
        )
        .bind(query_text)
        .bind(format!("%{}%", query_text))
        .bind(status)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Count text search with status failed: {}", e)))?;

        let count: i64 = row.try_get("count")?;
        Ok(count)
    }

    /// Get count with text, language, and status
    async fn get_count_text_with_language_and_status(&self, query_text: &str, language: &str, status: &str) -> Result<i64> {
        let row = query(
            r#"
            SELECT COUNT(*) as count FROM ema_catalog
            WHERE (search_vector @@ plainto_tsquery('english', $1)
                   OR product_name ILIKE $2
                   OR inn_name ILIKE $2
                   OR eu_number ILIKE $2
                   OR mah_name ILIKE $2)
              AND language_code = $3
              AND authorization_status = $4
            "#
        )
        .bind(query_text)
        .bind(format!("%{}%", query_text))
        .bind(language)
        .bind(status)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Count text search with language and status failed: {}", e)))?;

        let count: i64 = row.try_get("count")?;
        Ok(count)
    }

    /// Get count with filters only (no text)
    async fn get_count_filters_only(&self, request: &EmaSearchRequest) -> Result<i64> {
        match (request.language.as_ref(), request.authorization_status.as_ref()) {
            (Some(language), Some(status)) => {
                self.get_count_by_language_and_status(language, status).await
            }
            (Some(language), None) => {
                self.get_count_by_language(language).await
            }
            (None, Some(status)) => {
                self.get_count_by_status(status).await
            }
            (None, None) => {
                self.get_count_all().await
            }
        }
    }

    /// Get count by language and status
    async fn get_count_by_language_and_status(&self, language: &str, status: &str) -> Result<i64> {
        let row = query(
            "SELECT COUNT(*) as count FROM ema_catalog WHERE language_code = $1 AND authorization_status = $2"
        )
        .bind(language)
        .bind(status)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Count by language and status failed: {}", e)))?;

        let count: i64 = row.try_get("count")?;
        Ok(count)
    }

    /// Get count by language only
    async fn get_count_by_language(&self, language: &str) -> Result<i64> {
        let row = query(
            "SELECT COUNT(*) as count FROM ema_catalog WHERE language_code = $1"
        )
        .bind(language)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Count by language failed: {}", e)))?;

        let count: i64 = row.try_get("count")?;
        Ok(count)
    }

    /// Get count by status only
    async fn get_count_by_status(&self, status: &str) -> Result<i64> {
        let row = query(
            "SELECT COUNT(*) as count FROM ema_catalog WHERE authorization_status = $1"
        )
        .bind(status)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Count by status failed: {}", e)))?;

        let count: i64 = row.try_get("count")?;
        Ok(count)
    }

    /// Get count of all entries
    async fn get_count_all(&self) -> Result<i64> {
        let row = query("SELECT COUNT(*) as count FROM ema_catalog")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Count all failed: {}", e)))?;

        let count: i64 = row.try_get("count")?;
        Ok(count)
    }

    // ============================================================================
    // Sync Tracking
    // ============================================================================

    /// Start a new sync log entry
    pub async fn start_sync_log(
        &self,
        language: Option<String>,
        sync_type: Option<String>,
        record_limit: Option<i32>
    ) -> Result<Uuid> {
        let row = query(
            r#"
            INSERT INTO ema_sync_log (
                sync_started_at, language_code, sync_type, record_limit, status
            ) VALUES ($1, $2, $3, $4, 'in_progress')
            RETURNING id
            "#
        )
        .bind(Utc::now())
        .bind(language)
        .bind(sync_type)
        .bind(record_limit)
        .fetch_one(&self.pool)
        .await?;

        let id: Uuid = row.try_get("id")?;
        Ok(id)
    }

    /// Complete sync log with success
    pub async fn complete_sync_log(
        &self,
        log_id: Uuid,
        records_fetched: i32,
        records_inserted: i32,
        records_updated: i32,
        records_skipped: i32,
        records_failed: i32,
        api_response_time_ms: Option<i32>,
        processing_time_ms: Option<i32>,
    ) -> Result<()> {
        query(
            r#"
            UPDATE ema_sync_log
            SET sync_completed_at = $1,
                records_fetched = $2,
                records_inserted = $3,
                records_updated = $4,
                records_skipped = $5,
                records_failed = $6,
                api_response_time_ms = $7,
                processing_time_ms = $8,
                status = 'completed'
            WHERE id = $9
            "#
        )
        .bind(Utc::now())
        .bind(records_fetched)
        .bind(records_inserted)
        .bind(records_updated)
        .bind(records_skipped)
        .bind(records_failed)
        .bind(api_response_time_ms)
        .bind(processing_time_ms)
        .bind(log_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark sync log as failed
    pub async fn fail_sync_log(
        &self,
        log_id: Uuid,
        error_message: &str,
        warning_messages: Option<Vec<String>>,
    ) -> Result<()> {
        query(
            r#"
            UPDATE ema_sync_log
            SET sync_completed_at = $1,
                status = 'failed',
                error_message = $2,
                warning_messages = $3
            WHERE id = $4
            "#
        )
        .bind(Utc::now())
        .bind(error_message)
        .bind(warning_messages)
        .bind(log_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get the last successful sync
    pub async fn get_last_successful_sync(&self) -> Result<Option<EmaSyncLog>> {
        let log = query_as::<_, EmaSyncLog>(
            r#"
            SELECT * FROM ema_sync_log
            WHERE status = 'completed'
            ORDER BY sync_completed_at DESC
            LIMIT 1
            "#
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(log)
    }

    /// Get sync logs with pagination
    pub async fn get_sync_logs(&self, limit: i64, offset: i64) -> Result<Vec<EmaSyncLog>> {
        let logs = query_as::<_, EmaSyncLog>(
            r#"
            SELECT * FROM ema_sync_log
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

    // ============================================================================
    // Statistics and Analytics
    // ============================================================================

    /// Get comprehensive catalog statistics
    pub async fn get_catalog_stats(&self) -> Result<EmaCatalogStats> {
        // Get total count
        let total_entries = self.get_total_count().await?;

        // Get counts by language
        let entries_by_language = query_as::<_, LanguageCount>(
            r#"
            SELECT
                COALESCE(language_code, 'unknown') as language_code,
                COUNT(*) as count
            FROM ema_catalog
            GROUP BY language_code
            ORDER BY count DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        // Get counts by authorization status
        let entries_by_status = query_as::<_, StatusCount>(
            r#"
            SELECT
                COALESCE(authorization_status, 'unknown') as status,
                COUNT(*) as count
            FROM ema_catalog
            GROUP BY authorization_status
            ORDER BY count DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        // Get counts by therapeutic area (top 10)
        let entries_by_therapeutic_area = query_as::<_, TherapeuticAreaCount>(
            r#"
            SELECT
                COALESCE(therapeutic_area, 'unknown') as therapeutic_area,
                COUNT(*) as count
            FROM ema_catalog
            WHERE therapeutic_area IS NOT NULL
            GROUP BY therapeutic_area
            ORDER BY count DESC
            LIMIT 10
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        // Get orphan medicines count
        let orphan_medicines_count = query(
            "SELECT COUNT(*) as count FROM ema_catalog WHERE orphan_designation = true"
        )
        .fetch_one(&self.pool)
        .await
        .and_then(|row| row.try_get::<i64, _>("count"))?;

        // Get last sync info
        let last_sync = self.get_last_successful_sync().await?;
        let last_sync_at = last_sync.as_ref().map(|log| log.sync_started_at);
        let last_sync_status = last_sync.map(|log| log.status);

        Ok(EmaCatalogStats {
            total_entries,
            entries_by_language,
            entries_by_status,
            entries_by_therapeutic_area,
            orphan_medicines_count,
            last_sync_at,
            last_sync_status,
        })
    }

    /// Check if catalog needs refresh (older than specified days)
    pub async fn needs_refresh(&self, days_threshold: i64) -> Result<bool> {
        let last_sync = self.get_last_successful_sync().await?;

        let needs_refresh = match last_sync {
            Some(log) => {
                if let Some(completed_at) = log.sync_completed_at {
                    let threshold = Utc::now() - chrono::Duration::days(days_threshold);
                    completed_at < threshold
                } else {
                    true // If no completion time, needs refresh
                }
            }
            None => true, // No sync ever, needs refresh
        };

        Ok(needs_refresh)
    }

    /// Clean up old sync logs (keep last 30 days)
    pub async fn cleanup_old_sync_logs(&self) -> Result<i64> {
        let cutoff_date = Utc::now() - chrono::Duration::days(30);

        let result = query(
            "DELETE FROM ema_sync_log WHERE created_at < $1"
        )
        .bind(cutoff_date)
        .execute(&self.pool)
        .await?;

        let deleted_count = result.rows_affected();
        Ok(deleted_count as i64)
    }
}