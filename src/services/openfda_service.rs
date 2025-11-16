use uuid::Uuid;
use crate::models::openfda::{
    OpenFdaApiResponse, OpenFdaCatalogEntry, OpenFdaCatalogResponse,
    OpenFdaSearchRequest, OpenFdaSyncLog
};
use crate::repositories::OpenFdaRepository;
use crate::middleware::error_handling::{Result, AppError};

pub struct OpenFdaService {
    repo: OpenFdaRepository,
    api_base_url: String,
}

impl OpenFdaService {
    pub fn new(repo: OpenFdaRepository) -> Self {
        Self {
            repo,
            api_base_url: "https://api.fda.gov/drug/ndc.json".to_string(),
        }
    }

    /// Sync data from OpenFDA API
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
            let url = format!("{}?limit={}&skip={}", self.api_base_url, current_limit, skip);
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
}

#[derive(Debug, serde::Serialize)]
pub struct CatalogStats {
    pub total_entries: i64,
    pub last_sync_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_sync_records_fetched: Option<i32>,
    pub last_sync_records_inserted: Option<i32>,
    pub last_sync_records_updated: Option<i32>,
}
