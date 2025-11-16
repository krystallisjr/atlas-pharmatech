/// Enterprise batch import processor
/// Handles parallel processing of thousands of inventory rows with progress tracking

use uuid::Uuid;
use sqlx::PgPool;
use crate::middleware::error_handling::{Result, AppError};
use crate::models::ai_import::{AiImportSession, MappedInventoryRow, ColumnMapping};
use crate::services::inventory_validator_service::{InventoryValidatorService, ValidationResult};
use crate::services::file_parser_service::{FileParserService, ParsedFile};
use crate::repositories::{InventoryRepository, PharmaceuticalRepository};
use crate::models::inventory::CreateInventoryRequest;
use crate::models::pharmaceutical::CreatePharmaceuticalRequest;
use std::sync::Arc;
use tokio::sync::Semaphore;

const MAX_CONCURRENT_VALIDATIONS: usize = 10;
const BATCH_SIZE: usize = 100;

pub struct BatchImportProcessor {
    db_pool: PgPool,
    validator: InventoryValidatorService,
    inventory_repo: InventoryRepository,
    pharma_repo: PharmaceuticalRepository,
}

impl BatchImportProcessor {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            validator: InventoryValidatorService::new(db_pool.clone()),
            inventory_repo: InventoryRepository::new(db_pool.clone()),
            pharma_repo: PharmaceuticalRepository::new(db_pool.clone()),
            db_pool,
        }
    }

    /// Main import orchestration - processes entire file
    pub async fn process_import(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        parsed_file: ParsedFile,
        mapping: ColumnMapping,
    ) -> Result<ImportStats> {
        tracing::info!("Starting batch import for session: {}", session_id);

        // Update session status
        self.update_session_status(session_id, "importing").await?;

        let total_rows = parsed_file.rows.len();
        let mut stats = ImportStats::default();

        // Process in batches for better performance
        for (batch_idx, chunk) in parsed_file.rows.chunks(BATCH_SIZE).enumerate() {
            let batch_start_row = batch_idx * BATCH_SIZE;
            
            tracing::info!(
                "Processing batch {}/{} (rows {}-{})",
                batch_idx + 1,
                (total_rows + BATCH_SIZE - 1) / BATCH_SIZE,
                batch_start_row + 1,
                (batch_start_row + chunk.len()).min(total_rows)
            );

            let batch_stats = self.process_batch(
                session_id,
                user_id,
                &parsed_file.headers,
                chunk,
                &mapping,
                batch_start_row,
            ).await?;

            stats.merge(batch_stats);

            // Update progress
            self.update_progress(session_id, stats.rows_processed as i32, stats.rows_imported as i32, stats.rows_failed as i32).await?;
        }

        // Mark import as completed
        self.complete_import(session_id, &stats).await?;

        tracing::info!(
            "Import completed for session {}: {} imported, {} failed, {} flagged",
            session_id,
            stats.rows_imported,
            stats.rows_failed,
            stats.rows_flagged
        );

        Ok(stats)
    }

    /// Process a single batch of rows with transaction safety
    async fn process_batch(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        headers: &[String],
        rows: &[Vec<String>],
        mapping: &ColumnMapping,
        batch_offset: usize,
    ) -> Result<ImportStats> {
        let mut stats = ImportStats::default();

        // Start transaction for the entire batch
        let mut tx = self.db_pool.begin().await?;

        // Semaphore for controlling concurrency (validation only, not DB writes)
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_VALIDATIONS));

        // Step 1: Validate all rows concurrently (read-only operations)
        let mut validation_handles = vec![];

        for (idx, row_data) in rows.iter().enumerate() {
            let row_number = batch_offset + idx + 1;
            let permit = semaphore.clone().acquire_owned().await.unwrap();

            let validator = InventoryValidatorService::new(self.db_pool.clone());
            let headers = headers.to_vec();
            let row_data = row_data.clone();
            let mapping = mapping.clone();

            let handle = tokio::spawn(async move {
                let _permit = permit;

                // Map and validate (read-only operations)
                let mapped_row = match validator.map_row_to_inventory(
                    row_number,
                    &headers,
                    &row_data,
                    &mapping,
                ) {
                    Ok(row) => row,
                    Err(e) => {
                        return (row_number, Err(e));
                    }
                };

                let validation = match validator.validate_row(&mapped_row).await {
                    Ok(v) => v,
                    Err(e) => {
                        return (row_number, Err(e));
                    }
                };

                (row_number, Ok((mapped_row, validation, row_data)))
            });

            validation_handles.push(handle);
        }

        // Collect validation results
        let mut validated_rows = vec![];
        for handle in validation_handles {
            match handle.await {
                Ok((row_number, Ok((mapped_row, validation, row_data)))) => {
                    validated_rows.push((row_number, mapped_row, validation, row_data));
                }
                Ok((row_number, Err(e))) => {
                    tracing::error!("Row {} validation failed: {}", row_number, e);
                    stats.rows_failed += 1;
                    stats.rows_processed += 1;
                }
                Err(e) => {
                    tracing::error!("Task panicked: {}", e);
                    stats.rows_failed += 1;
                    stats.rows_processed += 1;
                }
            }
        }

        // Step 2: Process valid rows sequentially within transaction
        for (row_number, mapped_row, validation, row_data) in validated_rows {
            stats.rows_processed += 1;

            let row_status = if !validation.is_valid {
                "failed"
            } else if !validation.warnings.is_empty() {
                "flagged_for_review"
            } else {
                "imported"
            };

            // Create inventory if valid
            let (inventory_id, pharma_id) = if validation.is_valid {
                match self.create_inventory_from_row_tx(
                    &mut tx,
                    &mapped_row,
                    &validation,
                    user_id,
                ).await {
                    Ok(ids) => {
                        stats.rows_imported += 1;
                        ids
                    }
                    Err(e) => {
                        tracing::error!("Failed to create inventory for row {}: {}", row_number, e);
                        // Rollback transaction on error
                        tx.rollback().await?;
                        return Err(e);
                    }
                }
            } else {
                (None, None)
            };

            if !validation.warnings.is_empty() {
                stats.rows_flagged += 1;
            }

            // Save row result within transaction
            Self::save_row_result_tx(
                &mut tx,
                session_id,
                row_number as i32,
                &row_data,
                &mapped_row,
                &validation,
                row_status,
                inventory_id,
                pharma_id,
            ).await?;
        }

        // Commit transaction - all or nothing
        tx.commit().await?;

        tracing::info!("Batch committed: {} rows processed", stats.rows_processed);

        Ok(stats)
    }

    /// Create pharmaceutical and inventory records from validated row
    async fn create_inventory_from_row(
        row: &MappedInventoryRow,
        validation: &ValidationResult,
        user_id: Uuid,
        pharma_repo: &PharmaceuticalRepository,
        inventory_repo: &InventoryRepository,
    ) -> Result<(Option<Uuid>, Option<Uuid>)> {
        // Find or create pharmaceutical
        let pharma_id = if let Some(ref ndc) = row.ndc_code {
            match pharma_repo.find_by_ndc(ndc).await? {
                Some(existing) => existing.id,
                None => {
                    // Use OpenFDA enriched data if available
                    let (brand_name, generic_name, manufacturer) = if let Some(ref enriched) = validation.enriched_data {
                        (
                            enriched.openfda_brand_name.clone().or_else(|| row.brand_name.clone()),
                            enriched.openfda_generic_name.clone().or_else(|| row.generic_name.clone()),
                            enriched.openfda_manufacturer.clone().or_else(|| row.manufacturer.clone()),
                        )
                    } else {
                        (row.brand_name.clone(), row.generic_name.clone(), row.manufacturer.clone())
                    };

                    let brand = brand_name.clone().unwrap_or_else(|| "Unknown".to_string());
                    let generic = generic_name.unwrap_or_else(|| "Unknown".to_string());
                    let mfr = manufacturer.unwrap_or_else(|| "Unknown".to_string());

                    let pharma_request = CreatePharmaceuticalRequest {
                        brand_name: brand.clone(),
                        generic_name: generic,
                        ndc_code: Some(ndc.clone()),
                        manufacturer: mfr,
                        category: Some(row.category.clone().unwrap_or_else(|| "General".to_string())),
                        description: Some(format!("{} - Auto-imported", brand)),
                        strength: row.strength.clone(),
                        dosage_form: row.dosage_form.clone(),
                        storage_requirements: None,
                    };

                    pharma_repo.create(&pharma_request).await?.id
                }
            }
        } else {
            // No NDC - try to find by name or create generic entry
            let brand = row.brand_name.clone().unwrap_or_else(|| "Unknown".to_string());
            let generic = row.generic_name.clone().unwrap_or_else(|| "Unknown".to_string());

            let pharma_request = CreatePharmaceuticalRequest {
                brand_name: brand.clone(),
                generic_name: generic,
                ndc_code: None,
                manufacturer: row.manufacturer.clone().unwrap_or_else(|| "Unknown".to_string()),
                category: Some(row.category.clone().unwrap_or_else(|| "General".to_string())),
                description: Some(format!("{} - Auto-imported (no NDC)", brand)),
                strength: row.strength.clone(),
                dosage_form: row.dosage_form.clone(),
                storage_requirements: None,
            };

            pharma_repo.create(&pharma_request).await?.id
        };

        // Create inventory record
        let inventory_request = CreateInventoryRequest {
            pharmaceutical_id: pharma_id,
            batch_number: row.batch_number.clone().unwrap_or_else(|| "UNKNOWN".to_string()),
            quantity: row.quantity.unwrap_or(0),
            expiry_date: row.expiry_date.unwrap_or_else(|| {
                chrono::Utc::now().date_naive() + chrono::Duration::days(365)
            }),
            unit_price: row.unit_price,
            storage_location: row.storage_location.clone(),
        };

        let inventory = inventory_repo.create(&inventory_request, user_id).await?;

        Ok((Some(inventory.id), Some(pharma_id)))
    }

    /// Save individual row processing result to database
    async fn save_row_result(
        db_pool: &PgPool,
        session_id: Uuid,
        row_number: i32,
        source_data: &[String],
        mapped_row: &MappedInventoryRow,
        validation: &ValidationResult,
        status: &str,
        inventory_id: Option<Uuid>,
        pharma_id: Option<Uuid>,
    ) -> Result<()> {
        let source_json = serde_json::json!(source_data);
        let mapped_json = serde_json::to_value(mapped_row).ok();
        let errors: Vec<serde_json::Value> = validation.errors.iter().map(|e| serde_json::json!(e)).collect();
        let warnings: Vec<serde_json::Value> = validation.warnings.iter().map(|w| serde_json::json!(w)).collect();

        let (matched_ndc, confidence, enriched) = if let Some(ref enriched) = validation.enriched_data {
            (
                enriched.matched_ndc.clone(),
                Some(rust_decimal::Decimal::try_from(enriched.confidence_score).ok()),
                Some(serde_json::to_value(enriched).ok()),
            )
        } else {
            (None, None, None)
        };

        sqlx::query!(
            r#"
            INSERT INTO ai_import_row_results (
                id, session_id, row_number, source_data, status,
                mapped_data, validation_errors, validation_warnings,
                matched_ndc, openfda_match_confidence, openfda_enriched_fields,
                created_inventory_id, created_pharmaceutical_id,
                processed_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, NOW())
            "#,
            Uuid::new_v4(),
            session_id,
            row_number,
            source_json,
            status,
            mapped_json,
            &errors,
            &warnings,
            matched_ndc,
            confidence.flatten(),
            enriched.flatten(),
            inventory_id,
            pharma_id,
        )
        .execute(db_pool)
        .await?;

        Ok(())
    }

    async fn update_session_status(&self, session_id: Uuid, status: &str) -> Result<()> {
        sqlx::query!(
            "UPDATE ai_import_sessions SET status = $1, import_started_at = NOW() WHERE id = $2",
            status,
            session_id
        )
        .execute(&self.db_pool)
        .await?;
        Ok(())
    }

    async fn update_progress(&self, session_id: Uuid, processed: i32, imported: i32, failed: i32) -> Result<()> {
        sqlx::query!(
            r#"UPDATE ai_import_sessions 
               SET rows_processed = $1, rows_imported = $2, rows_failed = $3 
               WHERE id = $4"#,
            processed,
            imported,
            failed,
            session_id
        )
        .execute(&self.db_pool)
        .await?;
        Ok(())
    }

    async fn complete_import(&self, session_id: Uuid, stats: &ImportStats) -> Result<()> {
        sqlx::query!(
            r#"UPDATE ai_import_sessions 
               SET status = 'completed', 
                   rows_processed = $1,
                   rows_imported = $2, 
                   rows_failed = $3,
                   rows_flagged_for_review = $4,
                   import_completed_at = NOW() 
               WHERE id = $5"#,
            stats.rows_processed as i32,
            stats.rows_imported as i32,
            stats.rows_failed as i32,
            stats.rows_flagged as i32,
            session_id
        )
        .execute(&self.db_pool)
        .await?;
        Ok(())
    }

    /// Transaction-safe version: Create pharmaceutical and inventory within transaction
    async fn create_inventory_from_row_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        row: &MappedInventoryRow,
        validation: &ValidationResult,
        user_id: Uuid,
    ) -> Result<(Option<Uuid>, Option<Uuid>)> {
        // Find or create pharmaceutical with SELECT FOR UPDATE to prevent race conditions
        let pharma_id = if let Some(ref ndc) = row.ndc_code {
            // Lock row to prevent concurrent creation
            let existing = sqlx::query!(
                "SELECT id FROM pharmaceuticals WHERE ndc_code = $1 FOR UPDATE",
                ndc
            )
            .fetch_optional(&mut **tx)
            .await?;

            if let Some(pharma) = existing {
                pharma.id
            } else {
                // Use OpenFDA enriched data if available
                let (brand_name, generic_name, manufacturer) = if let Some(ref enriched) = validation.enriched_data {
                    (
                        enriched.openfda_brand_name.clone().or_else(|| row.brand_name.clone()),
                        enriched.openfda_generic_name.clone().or_else(|| row.generic_name.clone()),
                        enriched.openfda_manufacturer.clone().or_else(|| row.manufacturer.clone()),
                    )
                } else {
                    (row.brand_name.clone(), row.generic_name.clone(), row.manufacturer.clone())
                };

                let brand = brand_name.clone().unwrap_or_else(|| "Unknown".to_string());
                let generic = generic_name.unwrap_or_else(|| "Unknown".to_string());
                let mfr = manufacturer.unwrap_or_else(|| "Unknown".to_string());

                // Insert pharmaceutical within transaction
                let pharma_id = sqlx::query!(
                    r#"
                    INSERT INTO pharmaceuticals (
                        id, brand_name, generic_name, ndc_code, manufacturer,
                        category, description, strength, dosage_form
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                    RETURNING id
                    "#,
                    Uuid::new_v4(),
                    brand,
                    generic,
                    Some(ndc.clone()),
                    mfr,
                    Some(row.category.clone().unwrap_or_else(|| "General".to_string())),
                    Some(format!("{} - Auto-imported", brand)),
                    row.strength,
                    row.dosage_form,
                )
                .fetch_one(&mut **tx)
                .await?
                .id;

                pharma_id
            }
        } else {
            // No NDC - create generic entry
            let brand = row.brand_name.clone().unwrap_or_else(|| "Unknown".to_string());
            let generic = row.generic_name.clone().unwrap_or_else(|| "Unknown".to_string());

            let pharma_id = sqlx::query!(
                r#"
                INSERT INTO pharmaceuticals (
                    id, brand_name, generic_name, manufacturer,
                    category, description, strength, dosage_form
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                RETURNING id
                "#,
                Uuid::new_v4(),
                brand.clone(),
                generic,
                row.manufacturer.clone().unwrap_or_else(|| "Unknown".to_string()),
                Some(row.category.clone().unwrap_or_else(|| "General".to_string())),
                Some(format!("{} - Auto-imported (no NDC)", brand)),
                row.strength,
                row.dosage_form,
            )
            .fetch_one(&mut **tx)
            .await?
            .id;

            pharma_id
        };

        // Create inventory record within transaction
        let inventory_id = sqlx::query!(
            r#"
            INSERT INTO inventory (
                id, pharmaceutical_id, user_id, batch_number, quantity,
                expiry_date, unit_price, storage_location, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
            RETURNING id
            "#,
            Uuid::new_v4(),
            pharma_id,
            user_id,
            row.batch_number.clone().unwrap_or_else(|| "UNKNOWN".to_string()),
            row.quantity.unwrap_or(0),
            row.expiry_date.unwrap_or_else(|| {
                chrono::Utc::now().date_naive() + chrono::Duration::days(365)
            }),
            row.unit_price,
            row.storage_location,
        )
        .fetch_one(&mut **tx)
        .await?
        .id;

        Ok((Some(inventory_id), Some(pharma_id)))
    }

    /// Transaction-safe version: Save row result within transaction
    async fn save_row_result_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        session_id: Uuid,
        row_number: i32,
        source_data: &[String],
        mapped_row: &MappedInventoryRow,
        validation: &ValidationResult,
        status: &str,
        inventory_id: Option<Uuid>,
        pharma_id: Option<Uuid>,
    ) -> Result<()> {
        let source_json = serde_json::json!(source_data);
        let mapped_json = serde_json::to_value(mapped_row).ok();
        let errors: Vec<serde_json::Value> = validation.errors.iter().map(|e| serde_json::json!(e)).collect();
        let warnings: Vec<serde_json::Value> = validation.warnings.iter().map(|w| serde_json::json!(w)).collect();

        let (matched_ndc, confidence, enriched) = if let Some(ref enriched) = validation.enriched_data {
            (
                enriched.matched_ndc.clone(),
                Some(rust_decimal::Decimal::try_from(enriched.confidence_score).ok()),
                Some(serde_json::to_value(enriched).ok()),
            )
        } else {
            (None, None, None)
        };

        sqlx::query!(
            r#"
            INSERT INTO ai_import_row_results (
                id, session_id, row_number, source_data, status,
                mapped_data, validation_errors, validation_warnings,
                matched_ndc, openfda_match_confidence, openfda_enriched_fields,
                created_inventory_id, created_pharmaceutical_id,
                processed_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, NOW())
            "#,
            Uuid::new_v4(),
            session_id,
            row_number,
            source_json,
            status,
            mapped_json,
            &errors,
            &warnings,
            matched_ndc,
            confidence.flatten(),
            enriched.flatten(),
            inventory_id,
            pharma_id,
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct ImportStats {
    pub rows_processed: usize,
    pub rows_imported: usize,
    pub rows_failed: usize,
    pub rows_flagged: usize,
}

impl ImportStats {
    fn merge(&mut self, other: ImportStats) {
        self.rows_processed += other.rows_processed;
        self.rows_imported += other.rows_imported;
        self.rows_failed += other.rows_failed;
        self.rows_flagged += other.rows_flagged;
    }
}

struct RowProcessResult {
    row_number: usize,
    success: bool,
    flagged: bool,
    errors: Vec<String>,
}

impl RowProcessResult {
    fn failed(row_number: usize, error: String) -> Self {
        Self {
            row_number,
            success: false,
            flagged: false,
            errors: vec![error],
        }
    }
}
