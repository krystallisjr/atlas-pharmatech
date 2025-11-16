use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Pharmaceutical {
    pub id: Uuid,
    pub brand_name: String,
    pub generic_name: String,
    pub ndc_code: Option<String>,
    pub manufacturer: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub strength: Option<String>,
    pub dosage_form: Option<String>,
    pub storage_requirements: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePharmaceuticalRequest {
    #[validate(length(min = 2, message = "Brand name must be at least 2 characters"))]
    pub brand_name: String,
    #[validate(length(min = 2, message = "Generic name must be at least 2 characters"))]
    pub generic_name: String,
    #[validate(length(max = 20, message = "NDC code too long"))]
    pub ndc_code: Option<String>,
    #[validate(length(min = 2, message = "Manufacturer must be at least 2 characters"))]
    pub manufacturer: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub strength: Option<String>,
    pub dosage_form: Option<String>,
    pub storage_requirements: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SearchPharmaceuticalRequest {
    pub query: Option<String>,
    pub brand_name: Option<String>,
    pub generic_name: Option<String>,
    pub manufacturer: Option<String>,
    pub category: Option<String>,
    pub ndc_code: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct PharmaceuticalResponse {
    pub id: Uuid,
    pub brand_name: String,
    pub generic_name: String,
    pub ndc_code: Option<String>,
    pub manufacturer: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub strength: Option<String>,
    pub dosage_form: Option<String>,
    pub storage_requirements: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<Pharmaceutical> for PharmaceuticalResponse {
    fn from(pharma: Pharmaceutical) -> Self {
        Self {
            id: pharma.id,
            brand_name: pharma.brand_name,
            generic_name: pharma.generic_name,
            ndc_code: pharma.ndc_code,
            manufacturer: pharma.manufacturer,
            category: pharma.category,
            description: pharma.description,
            strength: pharma.strength,
            dosage_form: pharma.dosage_form,
            storage_requirements: pharma.storage_requirements,
            created_at: pharma.created_at,
        }
    }
}