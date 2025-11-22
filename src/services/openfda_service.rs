use uuid::Uuid;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use sqlx::PgPool;
use crate::models::openfda::{
    OpenFdaApiResponse, OpenFdaCatalogEntry, OpenFdaCatalogResponse,
    OpenFdaSearchRequest, OpenFdaSyncLog, SyncProgressResponse
};
use crate::repositories::OpenFdaRepository;
use crate::middleware::error_handling::{Result, AppError};

/// Configuration for OpenFDA sync
#[derive(Debug, Clone)]
pub struct OpenFdaSyncConfig {
    pub api_base_url: String,
    pub batch_size: usize,
    pub batch_delay_ms: u64,
    pub max_retries: u32,
    pub request_timeout_secs: u64,
    pub sync_limit: Option<usize>, // None = unlimited (full sync)
}

impl Default for OpenFdaSyncConfig {
    fn default() -> Self {
        Self {
            api_base_url: std::env::var("OPENFDA_API_BASE_URL")
                .unwrap_or_else(|_| "https://api.fda.gov/drug/ndc.json".to_string()),
            batch_size: std::env::var("OPENFDA_BATCH_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),
            batch_delay_ms: std::env::var("OPENFDA_BATCH_DELAY_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),
            max_retries: std::env::var("OPENFDA_MAX_RETRIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
            request_timeout_secs: std::env::var("OPENFDA_REQUEST_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            sync_limit: std::env::var("OPENFDA_SYNC_LIMIT")
                .ok()
                .and_then(|s| s.parse().ok())
                .filter(|&v| v > 0), // 0 or unset = unlimited
        }
    }
}

/// Shared state for tracking active syncs
pub struct SyncState {
    pub active_sync_id: Option<Uuid>,
    pub cancel_requested: bool,
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            active_sync_id: None,
            cancel_requested: false,
        }
    }
}

pub struct OpenFdaService {
    repo: OpenFdaRepository,
    config: OpenFdaSyncConfig,
    http_client: reqwest::Client,
    sync_state: Arc<RwLock<SyncState>>,
}

impl OpenFdaService {
    pub fn new(repo: OpenFdaRepository) -> Self {
        Self::with_config(repo, OpenFdaSyncConfig::default())
    }

    pub fn with_config(repo: OpenFdaRepository, config: OpenFdaSyncConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.request_timeout_secs))
            .build()
            .unwrap_or_default();

        Self {
            repo,
            config,
            http_client,
            sync_state: Arc::new(RwLock::new(SyncState::default())),
        }
    }

    /// Create a service from a database pool (for spawning background tasks)
    pub fn from_pool(pool: PgPool) -> Self {
        Self::new(OpenFdaRepository::new(pool))
    }

    /// Check if a sync is currently running
    pub async fn is_sync_running(&self) -> Result<bool> {
        self.repo.is_sync_running().await
    }

    /// Get the current active sync progress
    pub async fn get_active_sync(&self) -> Result<Option<SyncProgressResponse>> {
        match self.repo.get_active_sync().await? {
            Some(log) => Ok(Some(log.into())),
            None => Ok(None),
        }
    }

    /// Get sync progress by ID
    pub async fn get_sync_progress(&self, sync_id: Uuid) -> Result<Option<SyncProgressResponse>> {
        match self.repo.get_sync_log(sync_id).await? {
            Some(log) => Ok(Some(log.into())),
            None => Ok(None),
        }
    }

    /// Get sync logs history
    pub async fn get_sync_logs(&self, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<SyncProgressResponse>> {
        let logs = self.repo.get_sync_logs(limit.unwrap_or(20), offset.unwrap_or(0)).await?;
        Ok(logs.into_iter().map(Into::into).collect())
    }

    /// Start a full sync in the background
    /// Returns the sync log ID immediately
    pub async fn start_background_sync(&self, sync_type: &str, pool: PgPool) -> Result<Uuid> {
        // Check if sync is already running
        if self.repo.is_sync_running().await? {
            return Err(AppError::BadRequest("A sync is already in progress".to_string()));
        }

        // Create sync log
        let log_id = self.repo.start_sync_log_with_type(sync_type, None).await?;

        // Update sync state
        {
            let mut state = self.sync_state.write().await;
            state.active_sync_id = Some(log_id);
            state.cancel_requested = false;
        }

        // Clone values for the spawned task
        let config = self.config.clone();
        let sync_state = Arc::clone(&self.sync_state);

        // Spawn background task
        tokio::spawn(async move {
            let service = OpenFdaService::from_pool(pool);
            if let Err(e) = service.perform_full_sync(log_id, config, sync_state).await {
                tracing::error!("OpenFDA sync failed: {:?}", e);
            }
        });

        tracing::info!("OpenFDA background sync started with ID: {}", log_id);
        Ok(log_id)
    }

    /// Perform the actual sync (runs in background)
    /// Uses alphabetical partitioning to work around OpenFDA's 25000 skip limit
    async fn perform_full_sync(
        &self,
        log_id: Uuid,
        config: OpenFdaSyncConfig,
        sync_state: Arc<RwLock<SyncState>>,
    ) -> Result<()> {
        let start_time = Instant::now();
        let batch_size = config.batch_size;
        let mut total_fetched: i32 = 0;
        let mut total_inserted: i32 = 0;
        let mut total_updated: i32 = 0;
        let mut total_skipped: i32 = 0;
        let mut total_failed: i32 = 0;
        let mut total_api_time_ms: i32 = 0;
        let mut current_batch: i32 = 0;

        // OpenFDA has a 25000 skip limit, so we partition by brand_name first letter
        // Include letters A-Z, numbers 0-9, and a catch-all for others
        let search_prefixes: Vec<Option<String>> = {
            let mut prefixes: Vec<Option<String>> = Vec::new();
            // First, try without any filter to get small datasets
            prefixes.push(None);
            // Then letters A-Z
            for c in 'a'..='z' {
                prefixes.push(Some(format!("brand_name:{}*", c)));
            }
            // Numbers 0-9
            for c in '0'..='9' {
                prefixes.push(Some(format!("brand_name:{}*", c)));
            }
            prefixes
        };

        tracing::info!("Starting OpenFDA full sync with alphabetical partitioning (log_id: {})", log_id);

        // First, get total count
        let initial_response = self.fetch_batch_with_search(&config, None, 0, 1).await?;
        let total_expected = initial_response.meta.results.total as i32;
        self.repo.set_total_expected(log_id, total_expected).await?;
        tracing::info!("OpenFDA API reports {} total records", total_expected);

        // Track which NDCs we've seen to avoid duplicates
        let mut seen_ndcs = std::collections::HashSet::new();

        // Process each partition
        for search_filter in &search_prefixes {
            // Check for cancellation
            {
                let state = sync_state.read().await;
                if state.cancel_requested {
                    tracing::info!("OpenFDA sync cancelled by user");
                    self.repo.fail_sync_log(log_id, "Cancelled by user").await?;
                    return Ok(());
                }
            }

            // Check sync limit
            if let Some(limit) = config.sync_limit {
                if total_fetched >= limit as i32 {
                    tracing::info!("Reached sync limit of {} records", limit);
                    break;
                }
            }

            let partition_name = search_filter.as_deref().unwrap_or("all");
            tracing::info!("Processing partition: {}", partition_name);

            let mut skip = 0;
            let max_skip = 25000; // OpenFDA limit

            loop {
                if skip >= max_skip {
                    tracing::warn!("Reached max skip {} for partition {}, moving to next", max_skip, partition_name);
                    break;
                }

                current_batch += 1;
                tracing::info!(
                    "Fetching batch {} from OpenFDA API: partition={}, skip={}, limit={}",
                    current_batch, partition_name, skip, batch_size
                );

                // Fetch from API with retries
                let api_start = Instant::now();
                let api_response = match self.fetch_batch_with_search(&config, search_filter.as_deref(), skip, batch_size).await {
                    Ok(response) => response,
                    Err(e) => {
                        // Log error but continue with next partition
                        tracing::error!("Failed to fetch batch {} (partition {}): {:?}", current_batch, partition_name, e);
                        break;
                    }
                };
                let api_time_ms = api_start.elapsed().as_millis() as i32;
                total_api_time_ms += api_time_ms;

                let batch_count = api_response.results.len();
                if batch_count == 0 {
                    tracing::info!("No more records in partition {}", partition_name);
                    break;
                }

                // Convert and insert records (skip duplicates)
                let mut batch_inserted = 0;
                let mut batch_updated = 0;
                let mut batch_skipped = 0;
                let mut batch_failed = 0;
                let mut entries = Vec::with_capacity(batch_count);

                for drug_record in api_response.results {
                    // Skip if we've already processed this NDC
                    if seen_ndcs.contains(&drug_record.product_ndc) {
                        batch_skipped += 1;
                        continue;
                    }
                    seen_ndcs.insert(drug_record.product_ndc.clone());

                    match drug_record.to_catalog_entry() {
                        Ok(entry) => entries.push(entry),
                        Err(e) => {
                            tracing::warn!(
                                "Failed to convert drug record {}: {}",
                                drug_record.product_ndc, e
                            );
                            batch_skipped += 1;
                            continue;
                        }
                    }
                }

                // Batch upsert
                if !entries.is_empty() {
                    match self.repo.batch_upsert(entries).await {
                        Ok((inserted, updated)) => {
                            batch_inserted = inserted;
                            batch_updated = updated;
                        }
                        Err(e) => {
                            tracing::error!("Batch upsert failed: {:?}", e);
                            batch_failed = batch_count as i32 - batch_skipped;
                        }
                    }
                }

                total_fetched += batch_count as i32;
                total_inserted += batch_inserted;
                total_updated += batch_updated;
                total_skipped += batch_skipped;
                total_failed += batch_failed;

                // Update progress
                self.repo.update_sync_progress(
                    log_id,
                    total_fetched,
                    total_inserted,
                    total_updated,
                    total_skipped,
                    total_failed,
                    current_batch,
                    api_time_ms,
                ).await?;

                tracing::info!(
                    "Batch {} complete: fetched={}, inserted={}, updated={}, skipped={}, failed={}",
                    current_batch, batch_count, batch_inserted, batch_updated, batch_skipped, batch_failed
                );

                skip += batch_size;

                // Rate limiting delay
                tokio::time::sleep(Duration::from_millis(config.batch_delay_ms)).await;

                // For the "all" partition (no filter), we only get first 25000
                // Then we rely on letter partitions for the rest
                if search_filter.is_none() && skip >= max_skip {
                    break;
                }
            }
        }

        // Complete sync
        let processing_time_ms = start_time.elapsed().as_millis() as i32;
        self.repo.complete_sync_log_full(
            log_id,
            total_fetched,
            total_inserted,
            total_updated,
            total_skipped,
            total_failed,
            processing_time_ms,
        ).await?;

        // Clear sync state
        {
            let mut state = sync_state.write().await;
            state.active_sync_id = None;
            state.cancel_requested = false;
        }

        tracing::info!(
            "OpenFDA sync completed: {} records fetched, {} inserted, {} updated, {} skipped, {} failed in {}ms",
            total_fetched, total_inserted, total_updated, total_skipped, total_failed, processing_time_ms
        );

        Ok(())
    }

    /// Fetch a batch with optional search filter
    async fn fetch_batch_with_search(
        &self,
        config: &OpenFdaSyncConfig,
        search: Option<&str>,
        skip: usize,
        limit: usize,
    ) -> Result<OpenFdaApiResponse> {
        let url = if let Some(search_query) = search {
            format!("{}?search={}&limit={}&skip={}", config.api_base_url, search_query, limit, skip)
        } else {
            format!("{}?limit={}&skip={}", config.api_base_url, limit, skip)
        };

        let mut last_error = None;

        for attempt in 0..config.max_retries {
            if attempt > 0 {
                let delay = Duration::from_secs(1 << attempt);
                tracing::warn!("Retry attempt {} after {:?} delay", attempt + 1, delay);
                tokio::time::sleep(delay).await;
            }

            match self.http_client.get(&url).send().await {
                Ok(response) => {
                    if !response.status().is_success() {
                        let status = response.status();
                        if status.as_u16() == 429 {
                            tracing::warn!("Rate limited by OpenFDA API, backing off...");
                            tokio::time::sleep(Duration::from_secs(60)).await;
                            continue;
                        }
                        if status.as_u16() == 404 {
                            // No results for this search - return empty
                            return Ok(OpenFdaApiResponse {
                                meta: crate::models::openfda::OpenFdaMeta {
                                    disclaimer: None,
                                    terms: None,
                                    license: None,
                                    last_updated: None,
                                    results: crate::models::openfda::OpenFdaMetaResults {
                                        skip: skip as i32,
                                        limit: limit as i32,
                                        total: 0,
                                    },
                                },
                                results: vec![],
                            });
                        }
                        last_error = Some(AppError::Internal(anyhow::anyhow!(
                            "OpenFDA API returned status: {}", status
                        )));
                        continue;
                    }

                    match response.json::<OpenFdaApiResponse>().await {
                        Ok(data) => return Ok(data),
                        Err(e) => {
                            last_error = Some(AppError::Internal(anyhow::anyhow!(
                                "Failed to parse OpenFDA response: {}", e
                            )));
                            continue;
                        }
                    }
                }
                Err(e) => {
                    last_error = Some(AppError::Internal(anyhow::anyhow!(
                        "HTTP request failed: {}", e
                    )));
                    continue;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            AppError::Internal(anyhow::anyhow!("Failed after {} retries", config.max_retries))
        }))
    }

    /// Fetch a batch with retry logic (legacy - kept for compatibility)
    async fn fetch_batch_with_retry(
        &self,
        config: &OpenFdaSyncConfig,
        skip: usize,
        limit: usize,
    ) -> Result<OpenFdaApiResponse> {
        let url = format!("{}?limit={}&skip={}", config.api_base_url, limit, skip);
        let mut last_error = None;

        for attempt in 0..config.max_retries {
            if attempt > 0 {
                // Exponential backoff: 1s, 2s, 4s
                let delay = Duration::from_secs(1 << attempt);
                tracing::warn!(
                    "Retry attempt {} after {:?} delay",
                    attempt + 1, delay
                );
                tokio::time::sleep(delay).await;
            }

            match self.http_client.get(&url).send().await {
                Ok(response) => {
                    if !response.status().is_success() {
                        let status = response.status();
                        // Handle rate limiting
                        if status.as_u16() == 429 {
                            tracing::warn!("Rate limited by OpenFDA API, backing off...");
                            tokio::time::sleep(Duration::from_secs(60)).await;
                            continue;
                        }
                        last_error = Some(AppError::Internal(anyhow::anyhow!(
                            "OpenFDA API returned status: {}", status
                        )));
                        continue;
                    }

                    match response.json::<OpenFdaApiResponse>().await {
                        Ok(data) => return Ok(data),
                        Err(e) => {
                            last_error = Some(AppError::Internal(anyhow::anyhow!(
                                "Failed to parse OpenFDA response: {}", e
                            )));
                            continue;
                        }
                    }
                }
                Err(e) => {
                    last_error = Some(AppError::Internal(anyhow::anyhow!(
                        "HTTP request failed: {}", e
                    )));
                    continue;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            AppError::Internal(anyhow::anyhow!("Failed after {} retries", config.max_retries))
        }))
    }

    /// Cancel a running sync
    pub async fn cancel_sync(&self, sync_id: Uuid, user_id: Uuid) -> Result<bool> {
        // Set cancel flag
        {
            let mut state = self.sync_state.write().await;
            if state.active_sync_id == Some(sync_id) {
                state.cancel_requested = true;
            }
        }

        // Update database
        self.repo.cancel_sync(sync_id, user_id).await
    }

    /// Sync data from OpenFDA API (legacy synchronous method)
    pub async fn sync_from_api(&self, limit: Option<usize>) -> Result<OpenFdaSyncLog> {
        let log_id = self.repo.start_sync_log().await?;

        match self.perform_sync(limit).await {
            Ok((fetched, inserted, updated)) => {
                self.repo.complete_sync_log(log_id, fetched, inserted, updated).await?;

                Ok(OpenFdaSyncLog {
                    id: log_id,
                    sync_started_at: chrono::Utc::now(),
                    sync_completed_at: Some(chrono::Utc::now()),
                    records_fetched: Some(fetched),
                    records_inserted: Some(inserted),
                    records_updated: Some(updated),
                    status: "completed".to_string(),
                    error_message: None,
                    created_at: chrono::Utc::now(),
                    total_expected: None,
                    records_processed: Some(fetched),
                    records_skipped: None,
                    records_failed: None,
                    current_batch: None,
                    total_batches: None,
                    api_response_time_ms: None,
                    processing_time_ms: None,
                    sync_type: Some("manual".to_string()),
                    cancelled_at: None,
                    cancelled_by: None,
                })
            }
            Err(e) => {
                let error_msg = format!("{:?}", e);
                self.repo.fail_sync_log(log_id, &error_msg).await?;
                Err(e)
            }
        }
    }

    async fn perform_sync(&self, limit: Option<usize>) -> Result<(i32, i32, i32)> {
        let batch_size = 100; // OpenFDA API limit
        let total_limit = limit.unwrap_or(150000); // Default: fetch 150000 records for enterprise-grade catalog
        let mut skip = 0;
        let mut total_fetched: i32 = 0;
        let mut total_inserted = 0;
        let mut total_updated = 0;

        loop {
            if total_fetched >= total_limit as i32 {
                break;
            }

            let remaining = total_limit as i32 - total_fetched;
            let current_limit = std::cmp::min(batch_size, remaining as usize);

            tracing::info!("Fetching batch from OpenFDA API: skip={}, limit={}", skip, current_limit);

            // Fetch from API
            let url = format!("{}?limit={}&skip={}", self.config.api_base_url, current_limit, skip);
            let response = reqwest::get(&url)
                .await
                .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to fetch from OpenFDA: {}", e)))?;

            if !response.status().is_success() {
                let status = response.status();
                return Err(AppError::Internal(anyhow::anyhow!(
                    "OpenFDA API returned error status: {}", status
                )));
            }

            let api_response: OpenFdaApiResponse = response
                .json()
                .await
                .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse OpenFDA response: {}", e)))?;

            let batch_count = api_response.results.len();
            if batch_count == 0 {
                break; // No more results
            }

            tracing::info!("Processing {} records from OpenFDA", batch_count);

            // Convert to catalog entries
            let mut entries = Vec::new();
            for drug_record in api_response.results {
                match drug_record.to_catalog_entry() {
                    Ok(entry) => entries.push(entry),
                    Err(e) => {
                        tracing::warn!("Failed to convert drug record {}: {}", drug_record.product_ndc, e);
                        continue;
                    }
                }
            }

            // Batch upsert
            let (inserted, updated) = self.repo.batch_upsert(entries).await?;
            total_inserted += inserted;
            total_updated += updated;
            total_fetched += batch_count as i32;

            tracing::info!(
                "Batch processed: fetched={}, inserted={}, updated={}",
                batch_count, inserted, updated
            );

            skip += batch_size;

            // Avoid rate limiting
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        Ok((total_fetched, total_inserted, total_updated))
    }

    /// Search catalog
    pub async fn search(&self, request: OpenFdaSearchRequest) -> Result<Vec<OpenFdaCatalogResponse>> {
        let entries = self.repo.search(&request).await?;
        let responses = entries.into_iter().map(Into::into).collect();
        Ok(responses)
    }

    /// Get by NDC
    pub async fn get_by_ndc(&self, ndc: &str) -> Result<Option<OpenFdaCatalogResponse>> {
        let entry = self.repo.find_by_ndc(ndc).await?;
        Ok(entry.map(Into::into))
    }

    /// Get catalog statistics
    pub async fn get_stats(&self) -> Result<CatalogStats> {
        let total_count = self.repo.get_total_count().await?;
        let last_sync = self.repo.get_last_successful_sync().await?;

        Ok(CatalogStats {
            total_entries: total_count,
            last_sync_at: last_sync.as_ref().and_then(|s| s.sync_completed_at),
            last_sync_records_fetched: last_sync.as_ref().and_then(|s| s.records_fetched),
            last_sync_records_inserted: last_sync.as_ref().and_then(|s| s.records_inserted),
            last_sync_records_updated: last_sync.as_ref().and_then(|s| s.records_updated),
        })
    }

    /// Check if catalog needs refresh (older than 7 days)
    pub async fn needs_refresh(&self) -> Result<bool> {
        match self.repo.get_last_successful_sync().await? {
            Some(log) => {
                if let Some(completed_at) = log.sync_completed_at {
                    let age = chrono::Utc::now() - completed_at;
                    Ok(age.num_days() > 7)
                } else {
                    Ok(true)
                }
            }
            None => Ok(true), // Never synced
        }
    }

    /// Cleanup old sync logs
    pub async fn cleanup_old_logs(&self, days_to_keep: i32) -> Result<i64> {
        self.repo.cleanup_old_logs(days_to_keep).await
    }
}

#[derive(Debug, serde::Serialize)]
pub struct CatalogStats {
    pub total_entries: i64,
    pub last_sync_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_sync_records_fetched: Option<i32>,
    pub last_sync_records_inserted: Option<i32>,
    pub last_sync_records_updated: Option<i32>,
}

/// Background scheduler for OpenFDA sync
pub struct OpenFdaSyncScheduler {
    pool: PgPool,
    interval_hours: u64,
}

impl OpenFdaSyncScheduler {
    pub fn new(pool: PgPool) -> Self {
        let interval_hours = std::env::var("OPENFDA_SYNC_INTERVAL_HOURS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(168); // Default: weekly (168 hours)

        Self { pool, interval_hours }
    }

    pub fn with_interval(pool: PgPool, interval_hours: u64) -> Self {
        Self { pool, interval_hours }
    }

    /// Run the scheduler loop
    pub async fn run(&self) {
        let interval = Duration::from_secs(self.interval_hours * 3600);
        let mut ticker = tokio::time::interval(interval);

        // Skip first tick (runs immediately on start)
        ticker.tick().await;

        tracing::info!(
            "OpenFDA sync scheduler started - syncing every {} hours",
            self.interval_hours
        );

        loop {
            ticker.tick().await;
            self.run_scheduled_sync().await;
        }
    }

    /// Run a single scheduled sync
    pub async fn run_scheduled_sync(&self) {
        tracing::info!("Running scheduled OpenFDA sync...");

        let service = OpenFdaService::from_pool(self.pool.clone());

        // Check if sync is needed
        match service.needs_refresh().await {
            Ok(needs_refresh) => {
                if !needs_refresh {
                    tracing::info!("OpenFDA catalog is up to date, skipping scheduled sync");
                    return;
                }
            }
            Err(e) => {
                tracing::error!("Failed to check if OpenFDA refresh needed: {:?}", e);
                return;
            }
        }

        // Check if sync is already running
        match service.is_sync_running().await {
            Ok(true) => {
                tracing::info!("OpenFDA sync already in progress, skipping scheduled sync");
                return;
            }
            Ok(false) => {}
            Err(e) => {
                tracing::error!("Failed to check OpenFDA sync status: {:?}", e);
                return;
            }
        }

        // Start background sync
        match service.start_background_sync("scheduled", self.pool.clone()).await {
            Ok(sync_id) => {
                tracing::info!("Scheduled OpenFDA sync started with ID: {}", sync_id);
            }
            Err(e) => {
                tracing::error!("Failed to start scheduled OpenFDA sync: {:?}", e);
            }
        }
    }
}
