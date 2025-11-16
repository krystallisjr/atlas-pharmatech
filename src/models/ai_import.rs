/// Models for AI-powered inventory import system

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use validator::Validate;

// ============================================================================
// Database Models
// ============================================================================

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AiImportSession {
    pub id: Uuid,
    pub user_id: Uuid,

    // File metadata
    pub original_filename: String,
    pub file_size_bytes: i64,
    pub file_type: String,
    pub file_hash: String,
    pub file_path: Option<String>,

    // Status
    pub status: String,

    // AI Analysis
    pub detected_format: Option<String>,
    pub detected_columns: Option<serde_json::Value>,
    pub ai_mapping: Option<serde_json::Value>,
    pub ai_confidence_scores: Option<serde_json::Value>,
    pub ai_warnings: Option<Vec<serde_json::Value>>,
    pub user_mapping_overrides: Option<serde_json::Value>,

    // Statistics
    pub total_rows: Option<i32>,
    pub rows_processed: Option<i32>,
    pub rows_imported: Option<i32>,
    pub rows_failed: Option<i32>,
    pub rows_flagged_for_review: Option<i32>,

    // OpenFDA validation
    pub ndc_validated_count: Option<i32>,
    pub ndc_not_found_count: Option<i32>,
    pub auto_enriched_count: Option<i32>,

    // Cost tracking
    pub ai_api_cost_usd: Option<rust_decimal::Decimal>,
    pub ai_tokens_used: Option<i32>,

    // Errors
    pub error_message: Option<String>,
    pub error_details: Option<serde_json::Value>,

    // Timestamps
    pub created_at: DateTime<Utc>,
    pub analysis_completed_at: Option<DateTime<Utc>>,
    pub mapping_approved_at: Option<DateTime<Utc>>,
    pub import_started_at: Option<DateTime<Utc>>,
    pub import_completed_at: Option<DateTime<Utc>>,

    // Metadata
    pub import_source: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AiImportRowResult {
    pub id: Uuid,
    pub session_id: Uuid,
    pub row_number: i32,
    pub source_data: serde_json::Value,
    pub status: String,

    // Processing results
    pub mapped_data: Option<serde_json::Value>,
    pub validated_data: Option<serde_json::Value>,

    // Validation
    pub validation_errors: Option<Vec<serde_json::Value>>,
    pub validation_warnings: Option<Vec<serde_json::Value>>,

    // OpenFDA enrichment
    pub matched_ndc: Option<String>,
    pub openfda_match_confidence: Option<rust_decimal::Decimal>,
    pub openfda_enriched_fields: Option<serde_json::Value>,

    // Created records
    pub created_inventory_id: Option<Uuid>,
    pub created_pharmaceutical_id: Option<Uuid>,

    // Errors
    pub error_message: Option<String>,
    pub error_type: Option<String>,

    // Timestamps
    pub created_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub imported_at: Option<DateTime<Utc>>,
}

// ============================================================================
// API Request/Response Models
// ============================================================================

#[derive(Debug, Deserialize, Validate)]
pub struct StartImportRequest {
    #[validate(length(min = 1, max = 500))]
    pub filename: String,

    #[validate(length(max = 100))]
    pub import_source: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ImportSessionResponse {
    pub id: Uuid,
    pub status: ImportStatus,
    pub original_filename: String,
    pub file_type: String,

    // Analysis results
    pub detected_columns: Option<Vec<String>>,
    pub suggested_mapping: Option<ColumnMapping>,
    pub confidence_scores: Option<serde_json::Value>,
    pub warnings: Vec<String>,

    // Statistics
    pub total_rows: Option<u32>,
    pub rows_processed: u32,
    pub rows_imported: u32,
    pub rows_failed: u32,
    pub rows_flagged: u32,

    // Validation stats
    pub ndc_validated: u32,
    pub ndc_not_found: u32,
    pub auto_enriched: u32,

    // Cost
    pub ai_cost_usd: String,

    // Progress
    pub progress_percentage: u32,

    // Timestamps
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,

    // Error if failed
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ImportStatus {
    Analyzing,
    MappingReview,
    Importing,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for ImportStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportStatus::Analyzing => write!(f, "analyzing"),
            ImportStatus::MappingReview => write!(f, "mapping_review"),
            ImportStatus::Importing => write!(f, "importing"),
            ImportStatus::Completed => write!(f, "completed"),
            ImportStatus::Failed => write!(f, "failed"),
            ImportStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnMapping {
    pub ndc_code: Option<String>,
    pub brand_name: Option<String>,
    pub generic_name: Option<String>,
    pub manufacturer: Option<String>,
    pub quantity: Option<String>,
    pub batch_number: Option<String>,
    pub expiry_date: Option<String>,
    pub unit_price: Option<String>,
    pub storage_location: Option<String>,
    pub category: Option<String>,
    pub strength: Option<String>,
    pub dosage_form: Option<String>,
}

impl ColumnMapping {
    pub fn new() -> Self {
        Self {
            ndc_code: None,
            brand_name: None,
            generic_name: None,
            manufacturer: None,
            quantity: None,
            batch_number: None,
            expiry_date: None,
            unit_price: None,
            storage_location: None,
            category: None,
            strength: None,
            dosage_form: None,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MappedInventoryRow {
    pub row_number: usize,
    pub ndc_code: Option<String>,
    pub brand_name: Option<String>,
    pub generic_name: Option<String>,
    pub manufacturer: Option<String>,
    pub quantity: Option<i32>,
    pub batch_number: Option<String>,
    pub expiry_date: Option<chrono::NaiveDate>,
    pub unit_price: Option<rust_decimal::Decimal>,
    pub storage_location: Option<String>,
    pub category: Option<String>,
    pub strength: Option<String>,
    pub dosage_form: Option<String>,
    pub validation_errors: Vec<String>,
    pub validation_warnings: Vec<String>,
}

impl ImportStatus {
    pub fn from_str(s: &str) -> Self {
        match s {
            "analyzing" => ImportStatus::Analyzing,
            "mapping_review" => ImportStatus::MappingReview,
            "importing" => ImportStatus::Importing,
            "completed" => ImportStatus::Completed,
            "failed" => ImportStatus::Failed,
            "cancelled" => ImportStatus::Cancelled,
            _ => ImportStatus::Analyzing,
        }
    }
}

impl From<AiImportSession> for ImportSessionResponse {
    fn from(session: AiImportSession) -> Self {
        let total_rows = session.total_rows.unwrap_or(0) as u32;
        let rows_processed = session.rows_processed.unwrap_or(0);
        let progress = if total_rows > 0 {
            ((rows_processed as f32 / total_rows as f32) * 100.0) as u32
        } else {
            0
        };

        let detected_columns = session.detected_columns.and_then(|v| {
            serde_json::from_value::<Vec<String>>(v).ok()
        });

        let suggested_mapping = session.ai_mapping.and_then(|v| {
            serde_json::from_value::<ColumnMapping>(v).ok()
        });

        let warnings = session.ai_warnings.unwrap_or_default()
            .into_iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        Self {
            id: session.id,
            status: ImportStatus::from_str(&session.status),
            original_filename: session.original_filename,
            file_type: session.file_type,
            detected_columns,
            suggested_mapping,
            confidence_scores: session.ai_confidence_scores,
            warnings,
            total_rows: session.total_rows.map(|r| r as u32),
            rows_processed: rows_processed as u32,
            rows_imported: session.rows_imported.unwrap_or(0) as u32,
            rows_failed: session.rows_failed.unwrap_or(0) as u32,
            rows_flagged: session.rows_flagged_for_review.unwrap_or(0) as u32,
            ndc_validated: session.ndc_validated_count.unwrap_or(0) as u32,
            ndc_not_found: session.ndc_not_found_count.unwrap_or(0) as u32,
            auto_enriched: session.auto_enriched_count.unwrap_or(0) as u32,
            ai_cost_usd: format!("{:.4}", session.ai_api_cost_usd.unwrap_or(rust_decimal::Decimal::ZERO)),
            progress_percentage: progress.min(100),
            created_at: session.created_at,
            completed_at: session.import_completed_at,
            error_message: session.error_message,
        }
    }
}
