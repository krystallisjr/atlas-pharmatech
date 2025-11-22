use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ============================================================================
// OpenFDA API Response Models
// ============================================================================

#[derive(Debug, Deserialize, Clone)]
pub struct OpenFdaApiResponse {
    pub meta: OpenFdaMeta,
    pub results: Vec<OpenFdaDrugRecord>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OpenFdaMeta {
    pub disclaimer: Option<String>,
    pub terms: Option<String>,
    pub license: Option<String>,
    pub last_updated: Option<String>,
    pub results: OpenFdaMetaResults,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OpenFdaMetaResults {
    pub skip: i32,
    pub limit: i32,
    pub total: i64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OpenFdaDrugRecord {
    pub product_ndc: String,
    pub product_id: Option<String>,
    pub brand_name: Option<String>,
    pub brand_name_base: Option<String>,
    pub generic_name: Option<String>,
    pub labeler_name: Option<String>,
    pub dosage_form: Option<String>,
    pub route: Option<Vec<String>>,
    pub product_type: Option<String>,
    pub marketing_category: Option<String>,
    pub pharm_class: Option<Vec<String>>,
    pub dea_schedule: Option<String>,
    pub active_ingredients: Option<Vec<ActiveIngredient>>,
    pub packaging: Option<Vec<Packaging>>,
    pub finished: Option<bool>,
    pub marketing_start_date: Option<String>,
    pub listing_expiration_date: Option<String>,
    pub openfda: Option<OpenFdaData>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ActiveIngredient {
    pub name: String,
    pub strength: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Packaging {
    pub package_ndc: Option<String>,
    pub description: Option<String>,
    pub marketing_start_date: Option<String>,
    pub sample: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OpenFdaData {
    pub manufacturer_name: Option<Vec<String>>,
    pub rxcui: Option<Vec<String>>,
    pub spl_set_id: Option<Vec<String>>,
    pub is_original_packager: Option<Vec<bool>>,
    pub unii: Option<Vec<String>>,
}

// ============================================================================
// Database Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OpenFdaCatalogEntry {
    pub id: Uuid,
    pub product_ndc: String,
    pub product_id: Option<String>,
    pub brand_name: String,
    pub brand_name_base: Option<String>,
    pub generic_name: String,
    pub labeler_name: String,
    pub dosage_form: Option<String>,
    pub route: Option<Vec<String>>,
    pub strength: Option<String>,
    pub active_ingredients: Option<serde_json::Value>,
    pub product_type: Option<String>,
    pub marketing_category: Option<String>,
    pub pharm_class: Option<Vec<String>>,
    pub dea_schedule: Option<String>,
    pub packaging: Option<serde_json::Value>,
    pub finished: Option<bool>,
    pub marketing_start_date: Option<NaiveDate>,
    pub listing_expiration_date: Option<NaiveDate>,
    pub openfda_data: Option<serde_json::Value>,
    pub last_synced_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Clone)]
pub struct OpenFdaCatalogResponse {
    pub id: Uuid,
    pub product_ndc: String,
    pub brand_name: String,
    pub generic_name: String,
    pub labeler_name: String,
    pub dosage_form: Option<String>,
    pub strength: Option<String>,
    pub route: Option<Vec<String>>,
    pub marketing_category: Option<String>,
    pub dea_schedule: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OpenFdaSearchRequest {
    pub query: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OpenFdaSyncLog {
    pub id: Uuid,
    pub sync_started_at: DateTime<Utc>,
    pub sync_completed_at: Option<DateTime<Utc>>,
    pub records_fetched: Option<i32>,
    pub records_inserted: Option<i32>,
    pub records_updated: Option<i32>,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    // Progress tracking fields
    pub total_expected: Option<i32>,
    pub records_processed: Option<i32>,
    pub records_skipped: Option<i32>,
    pub records_failed: Option<i32>,
    pub current_batch: Option<i32>,
    pub total_batches: Option<i32>,
    pub api_response_time_ms: Option<i32>,
    pub processing_time_ms: Option<i32>,
    pub sync_type: Option<String>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub cancelled_by: Option<Uuid>,
}

/// Response for sync progress queries
#[derive(Debug, Clone, Serialize)]
pub struct SyncProgressResponse {
    pub id: Uuid,
    pub status: String,
    pub sync_type: Option<String>,
    pub progress_percent: f64,
    pub records_processed: i32,
    pub total_expected: i32,
    pub records_inserted: i32,
    pub records_updated: i32,
    pub records_skipped: i32,
    pub records_failed: i32,
    pub current_batch: i32,
    pub total_batches: i32,
    pub elapsed_seconds: i64,
    pub estimated_remaining_seconds: Option<i64>,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl From<OpenFdaSyncLog> for SyncProgressResponse {
    fn from(log: OpenFdaSyncLog) -> Self {
        let total_expected = log.total_expected.unwrap_or(0);
        let records_processed = log.records_processed.unwrap_or(0);
        let progress_percent = if total_expected > 0 {
            (records_processed as f64 / total_expected as f64) * 100.0
        } else {
            0.0
        };

        let elapsed_seconds = (Utc::now() - log.sync_started_at).num_seconds();
        let estimated_remaining = if progress_percent > 0.0 && log.status == "in_progress" {
            let total_estimated = (elapsed_seconds as f64 / progress_percent) * 100.0;
            Some((total_estimated - elapsed_seconds as f64) as i64)
        } else {
            None
        };

        Self {
            id: log.id,
            status: log.status,
            sync_type: log.sync_type,
            progress_percent,
            records_processed,
            total_expected,
            records_inserted: log.records_inserted.unwrap_or(0),
            records_updated: log.records_updated.unwrap_or(0),
            records_skipped: log.records_skipped.unwrap_or(0),
            records_failed: log.records_failed.unwrap_or(0),
            current_batch: log.current_batch.unwrap_or(0),
            total_batches: log.total_batches.unwrap_or(0),
            elapsed_seconds,
            estimated_remaining_seconds: estimated_remaining,
            error_message: log.error_message,
            started_at: log.sync_started_at,
            completed_at: log.sync_completed_at,
        }
    }
}

// ============================================================================
// Conversion Implementations
// ============================================================================

impl OpenFdaDrugRecord {
    /// Extracts combined strength from active ingredients
    pub fn get_combined_strength(&self) -> Option<String> {
        self.active_ingredients.as_ref().map(|ingredients| {
            ingredients
                .iter()
                .map(|i| {
                    if let Some(ref strength) = i.strength {
                        format!("{} {}", i.name, strength)
                    } else {
                        i.name.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
    }

    /// Parse date string in YYYYMMDD format to NaiveDate
    fn parse_date(date_str: &Option<String>) -> Option<NaiveDate> {
        date_str.as_ref().and_then(|s| {
            // OpenFDA dates are in YYYYMMDD format
            if s.len() == 8 {
                let year = s[0..4].parse::<i32>().ok()?;
                let month = s[4..6].parse::<u32>().ok()?;
                let day = s[6..8].parse::<u32>().ok()?;
                NaiveDate::from_ymd_opt(year, month, day)
            } else {
                None
            }
        })
    }

    /// Convert to database entry
    pub fn to_catalog_entry(&self) -> Result<OpenFdaCatalogEntry, serde_json::Error> {
        Ok(OpenFdaCatalogEntry {
            id: Uuid::new_v4(),
            product_ndc: self.product_ndc.clone(),
            product_id: self.product_id.clone(),
            brand_name: self.brand_name.clone().unwrap_or_else(|| "Unknown".to_string()),
            brand_name_base: self.brand_name_base.clone(),
            generic_name: self.generic_name.clone().unwrap_or_else(|| "Unknown".to_string()),
            labeler_name: self.labeler_name.clone().unwrap_or_else(|| "Unknown".to_string()),
            dosage_form: self.dosage_form.clone(),
            route: self.route.clone(),
            strength: self.get_combined_strength(),
            active_ingredients: self.active_ingredients.as_ref()
                .map(|v| serde_json::to_value(v))
                .transpose()?,
            product_type: self.product_type.clone(),
            marketing_category: self.marketing_category.clone(),
            pharm_class: self.pharm_class.clone(),
            dea_schedule: self.dea_schedule.clone(),
            packaging: self.packaging.as_ref()
                .map(|v| serde_json::to_value(v))
                .transpose()?,
            finished: self.finished,
            marketing_start_date: Self::parse_date(&self.marketing_start_date),
            listing_expiration_date: Self::parse_date(&self.listing_expiration_date),
            openfda_data: self.openfda.as_ref()
                .map(|v| serde_json::to_value(v))
                .transpose()?,
            last_synced_at: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }
}

impl From<OpenFdaCatalogEntry> for OpenFdaCatalogResponse {
    fn from(entry: OpenFdaCatalogEntry) -> Self {
        Self {
            id: entry.id,
            product_ndc: entry.product_ndc,
            brand_name: entry.brand_name,
            generic_name: entry.generic_name,
            labeler_name: entry.labeler_name,
            dosage_form: entry.dosage_form,
            strength: entry.strength,
            route: entry.route,
            marketing_category: entry.marketing_category,
            dea_schedule: entry.dea_schedule,
        }
    }
}
