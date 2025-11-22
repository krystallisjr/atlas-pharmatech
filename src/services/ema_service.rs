use uuid::Uuid;
use std::time::{Duration, Instant};
use sqlx::{query, query_as, Row};
use crate::models::ema::{
    EmaEpiApiResponse, EmaCatalogEntry, EmaCatalogResponse,
    EmaSearchRequest, EmaSyncLog, EmaCatalogStats
};
use crate::repositories::ema_repo::EmaRepository;
use crate::middleware::error_handling::{Result, AppError};

pub struct EmaService {
    repo: EmaRepository,
    api_base_url: String,
    default_language: String,
    default_sync_limit: usize,
    batch_delay_ms: u64,
    max_retries: usize,
}

impl EmaService {
    pub fn new(repo: EmaRepository) -> Self {
        Self {
            repo,
            api_base_url: std::env::var("EMA_API_BASE_URL")
                .unwrap_or_else(|_| "https://epi.ema.europa.eu".to_string()),
            default_language: std::env::var("EMA_API_DEFAULT_LANGUAGE")
                .unwrap_or_else(|_| "en".to_string()),
            default_sync_limit: std::env::var("EMA_API_SYNC_LIMIT")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()
                .unwrap_or(1000),
            batch_delay_ms: std::env::var("EMA_API_BATCH_DELAY_MS")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .unwrap_or(100),
            max_retries: std::env::var("EMA_API_MAX_RETRIES")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .unwrap_or(3),
        }
    }

    // ============================================================================
    // Public API Methods
    // ============================================================================

    /// Sync data from EMA ePI API
    pub async fn sync_from_api(
        &self,
        language: Option<String>,
        limit: Option<usize>,
        sync_type: Option<String>
    ) -> Result<EmaSyncLog> {
        let lang = language.unwrap_or_else(|| self.default_language.clone());
        let sync_limit = limit.unwrap_or(self.default_sync_limit);
        let sync_type_str = sync_type.unwrap_or_else(|| "full".to_string());

        let log_id = self.repo.start_sync_log(
            Some(lang.clone()),
            Some(sync_type_str.clone()),
            Some(sync_limit as i32)
        ).await?;

        let sync_start_time = Instant::now();

        match self.perform_sync(&lang, sync_limit, &sync_type_str, log_id).await {
            Ok((fetched, inserted, updated, skipped, failed, api_response_time_ms)) => {
                let processing_time_ms = sync_start_time.elapsed().as_millis() as i32;

                self.repo.complete_sync_log(
                    log_id,
                    fetched,
                    inserted,
                    updated,
                    skipped,
                    failed,
                    Some(api_response_time_ms),
                    Some(processing_time_ms),
                ).await?;

                // Retrieve and return the completed sync log
                let sync_log = query_as::<_, EmaSyncLog>(
                    "SELECT * FROM ema_sync_log WHERE id = $1"
                )
                .bind(log_id)
                .fetch_one(&self.repo.pool)
                .await?;

                Ok(sync_log)
            }
            Err(e) => {
                let error_msg = format!("EMA sync failed: {:?}", e);
                tracing::error!("EMA sync failed for language {}: {}", lang, error_msg);

                self.repo.fail_sync_log(log_id, &error_msg, None).await?;
                Err(e)
            }
        }
    }

    /// Search catalog with filters
    pub async fn search(&self, request: EmaSearchRequest) -> Result<Vec<EmaCatalogResponse>> {
        let entries = self.repo.search(&request).await?;
        let responses = entries.into_iter().map(Into::into).collect();
        Ok(responses)
    }

    /// Get medicine by EU number
    pub async fn get_by_eu_number(&self, eu_number: &str) -> Result<Option<EmaCatalogResponse>> {
        let entry = self.repo.find_by_eu_number(eu_number).await?;
        Ok(entry.map(Into::into))
    }

    /// Get catalog statistics
    pub async fn get_stats(&self) -> Result<EmaCatalogStats> {
        let stats = self.repo.get_catalog_stats().await?;
        Ok(stats)
    }

    /// Check if catalog needs refresh
    pub async fn needs_refresh(&self, days_threshold: Option<i64>) -> Result<bool> {
        let threshold = days_threshold.unwrap_or(7); // Default 7 days
        self.repo.needs_refresh(threshold).await
    }

    /// Get sync logs with pagination
    pub async fn get_sync_logs(&self, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<EmaSyncLog>> {
        let limit_val = limit.unwrap_or(20).min(100);
        let offset_val = offset.unwrap_or(0);
        self.repo.get_sync_logs(limit_val, offset_val).await
    }

    /// Clean up old sync logs
    pub async fn cleanup_old_sync_logs(&self) -> Result<i64> {
        self.repo.cleanup_old_sync_logs().await
    }

    // ============================================================================
    // Private Sync Implementation
    // ============================================================================

    /// Perform the actual synchronization with EMA API
    async fn perform_sync(
        &self,
        language: &str,
        limit: usize,
        sync_type: &str,
        log_id: Uuid
    ) -> Result<(i32, i32, i32, i32, i32, i32)> {
        tracing::info!(
            "Starting EMA ePI API sync (language: {}, limit: {}, type: {})",
            language, limit, sync_type
        );

        let mut total_fetched: i32 = 0;
        let mut total_inserted = 0;
        let mut total_updated = 0;
        let mut total_skipped = 0;
        let mut total_failed = 0;
        let mut total_api_response_time = 0;

        match sync_type {
            "full" => {
                let (fetched, inserted, updated, skipped, failed, api_time) =
                    self.perform_full_sync(language, limit).await?;
                total_fetched = fetched;
                total_inserted = inserted;
                total_updated = updated;
                total_skipped = skipped;
                total_failed = failed;
                total_api_response_time = api_time;
            }
            "incremental" => {
                // For incremental sync, we would need to track last sync timestamp
                // For now, fall back to full sync
                tracing::warn!("Incremental sync not yet implemented, falling back to full sync");
                let (fetched, inserted, updated, skipped, failed, api_time) =
                    self.perform_full_sync(language, limit).await?;
                total_fetched = fetched;
                total_inserted = inserted;
                total_updated = updated;
                total_skipped = skipped;
                total_failed = failed;
                total_api_response_time = api_time;
            }
            "by_language" => {
                let (fetched, inserted, updated, skipped, failed, api_time) =
                    self.perform_language_sync(language, limit).await?;
                total_fetched = fetched;
                total_inserted = inserted;
                total_updated = updated;
                total_skipped = skipped;
                total_failed = failed;
                total_api_response_time = api_time;
            }
            _ => {
                return Err(AppError::BadRequest(format!("Unknown sync type: {}", sync_type)));
            }
        }

        tracing::info!(
            "EMA sync completed: fetched={}, inserted={}, updated={}, skipped={}, failed={}, avg_api_time_ms={}",
            total_fetched, total_inserted, total_updated, total_skipped, total_failed,
            if total_fetched > 0 { total_api_response_time / total_fetched } else { 0 }
        );

        Ok((total_fetched, total_inserted, total_updated, total_skipped, total_failed, total_api_response_time))
    }

    /// Perform full synchronization of EPI data
    async fn perform_full_sync(
        &self,
        language: &str,
        limit: usize
    ) -> Result<(i32, i32, i32, i32, i32, i32)> {
        let mut total_fetched = 0;
        let mut total_inserted = 0;
        let mut total_updated = 0;
        let mut total_skipped = 0;
        let mut total_failed = 0;
        let mut total_api_time = 0;

        // EMA ePI API uses ListBySearchParameter endpoint
        let mut offset = 0;
        const BATCH_SIZE: usize = 50; // Process in smaller batches

        while total_fetched < limit as i32 {
            let batch_start = Instant::now();
            let current_limit = std::cmp::min(BATCH_SIZE, limit - total_fetched as usize);

            let url = format!(
                "{}/ListBySearchParameter?_format=json&language={}",
                self.api_base_url, language
            );

            let (api_response, response_time) = self.fetch_from_ema_api_with_retry(&url).await?;
            total_api_time += response_time;

            if api_response.entry.is_none() || api_response.entry.as_ref().unwrap().is_empty() {
                tracing::info!("No more entries from EMA API, ending sync");
                break;
            }

            let entries: Vec<EmaCatalogEntry> = api_response.entry
                .unwrap()
                .into_iter()
                .filter_map(|epi_entry| {
                    match epi_entry.to_catalog_entry() {
                        Ok(entry) => {
                            // Filter by language if needed
                            if let Some(entry_lang) = &entry.language_code {
                                if entry_lang == language {
                                    Some(entry)
                                } else {
                                    tracing::debug!("Skipping entry with language {}: expected {}", entry_lang, language);
                                    None
                                }
                            } else {
                                Some(entry) // Include if no language specified
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to convert EPI entry to catalog entry: {}", e);
                            None
                        }
                    }
                })
                .collect();

            if entries.is_empty() {
                tracing::debug!("No valid entries in this batch");
                break;
            }

            // Batch upsert to database
            let (inserted, updated) = self.repo.batch_upsert(entries.clone()).await?;
            total_inserted += inserted;
            total_updated += updated;
            total_fetched += entries.len() as i32;

            tracing::debug!(
                "Processed batch of {} entries (inserted: {}, updated: {}) in {}ms",
                entries.len(), inserted, updated, batch_start.elapsed().as_millis()
            );

            // Rate limiting - delay between batches
            if total_fetched < limit as i32 {
                tokio::time::sleep(Duration::from_millis(self.batch_delay_ms)).await;
            }
        }

        Ok((total_fetched, total_inserted, total_updated, total_skipped, total_failed, total_api_time))
    }

    /// Perform language-specific synchronization
    async fn perform_language_sync(
        &self,
        language: &str,
        limit: usize
    ) -> Result<(i32, i32, i32, i32, i32, i32)> {
        // For now, language sync is the same as full sync but filtered by language
        self.perform_full_sync(language, limit).await
    }

    /// Fetch data from EMA API with retry logic
    async fn fetch_from_ema_api_with_retry(
        &self,
        url: &str
    ) -> Result<(EmaEpiApiResponse, i32)> {
        let mut last_error = None;

        for attempt in 1..=self.max_retries {
            let start_time = Instant::now();

            match self.fetch_from_ema_api(url).await {
                Ok(response) => {
                    let response_time = start_time.elapsed().as_millis() as i32;
                    return Ok((response, response_time));
                }
                Err(e) => {
                    last_error = Some(e);
                    tracing::warn!("EMA API request failed (attempt {}): {:?}", attempt, last_error);

                    // Exponential backoff
                    if attempt < self.max_retries {
                        let delay = Duration::from_millis(1000 * (2_u64.pow(attempt as u32 - 1)));
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| AppError::Internal(anyhow::anyhow!("Unknown EMA API error"))))
    }

    /// Fetch data from EMA API
    async fn fetch_from_ema_api(&self, url: &str) -> Result<EmaEpiApiResponse> {
        tracing::debug!("Fetching from EMA API: {}", url);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Atlas-Pharma-EMA-Client/1.0")
            .build()
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create HTTP client: {}", e)))?;

        let response = client
            .get(url)
            .header("Accept", "application/fhir+json,application/json")
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to send request to EMA API: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Could not read error response".to_string());

            return Err(AppError::Internal(anyhow::anyhow!(
                "EMA API returned error status: {} - {}", status, error_text
            )));
        }

        let api_response: EmaEpiApiResponse = response
            .json()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse EMA API response: {}", e)))?;

        // Validate response structure
        if api_response.resource_type != "Bundle" {
            return Err(AppError::Internal(anyhow::anyhow!(
                "Unexpected EMA API response type: {}", api_response.resource_type
            )));
        }

        tracing::debug!(
            "Successfully fetched {} entries from EMA API",
            api_response.entry.as_ref().map_or(0, |e| e.len())
        );

        Ok(api_response)
    }

    // ============================================================================
    // Helper Methods
    // ============================================================================

    /// Validate EU number format
    pub fn validate_eu_number(&self, eu_number: &str) -> Result<()> {
        if eu_number.is_empty() {
            return Err(AppError::BadRequest("EU number cannot be empty".to_string()));
        }

        // Basic EU number format validation
        if eu_number.starts_with("EU/") {
            let parts: Vec<&str> = eu_number.split('/').collect();
            if parts.len() < 4 {
                return Err(AppError::BadRequest(
                    "Invalid EU number format. Expected EU/1/XX/XXX/XXX".to_string()
                ));
            }
        } else if eu_number.starts_with("AUTO-") {
            // Auto-generated numbers are valid
        } else {
            return Err(AppError::BadRequest(
                "EU number must start with 'EU/' or be auto-generated".to_string()
            ));
        }

        Ok(())
    }

    /// Get supported languages for EMA API
    pub fn get_supported_languages(&self) -> Vec<&'static str> {
        vec!["en", "de", "fr", "es", "it", "pt", "nl", "sv", "fi", "da", "no", "el"]
    }

    /// Validate language code
    pub fn validate_language(&self, language: &str) -> Result<()> {
        let supported = self.get_supported_languages();
        if !supported.contains(&language) {
            return Err(AppError::BadRequest(format!(
                "Unsupported language '{}'. Supported languages: {}",
                language,
                supported.join(", ")
            )));
        }
        Ok(())
    }

    /// Get service configuration info
    pub fn get_config_info(&self) -> serde_json::Value {
        serde_json::json!({
            "api_base_url": self.api_base_url,
            "default_language": self.default_language,
            "default_sync_limit": self.default_sync_limit,
            "batch_delay_ms": self.batch_delay_ms,
            "max_retries": self.max_retries,
            "supported_languages": self.get_supported_languages()
        })
    }
}