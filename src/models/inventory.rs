use chrono::{DateTime, Utc, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::{Validate, ValidationError};
use crate::models::pharmaceutical::PharmaceuticalResponse;
use crate::models::user::UserResponse;

// Validation functions - defined before structs that use them
pub fn validate_expiry_date(expiry_date: &NaiveDate) -> Result<(), ValidationError> {
    let today = chrono::Utc::now().date_naive();
    if *expiry_date <= today {
        return Err(ValidationError::new("expiry_date_past"));
    }
    Ok(())
}

pub fn validate_positive_option_price(price: &rust_decimal::Decimal) -> Result<(), ValidationError> {
    if *price <= rust_decimal::Decimal::ZERO {
        return Err(ValidationError::new("positive_price"));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Inventory {
    pub id: Uuid,
    pub user_id: Uuid,
    pub pharmaceutical_id: Uuid,
    pub batch_number: String,
    pub quantity: i32,
    pub expiry_date: NaiveDate,
    pub unit_price: Option<rust_decimal::Decimal>,
    pub storage_location: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Clone, FromRow)]
pub struct InventoryWithDetails {
    #[sqlx(flatten)]
    pub inventory: Inventory,
    pub pharmaceutical: PharmaceuticalResponse,
    pub user: UserResponse,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateInventoryRequest {
    pub pharmaceutical_id: Uuid,
    #[validate(length(min = 1, message = "Batch number required"))]
    pub batch_number: String,
    #[validate(range(min = 1, message = "Quantity must be at least 1"))]
    pub quantity: i32,
    #[validate(custom(function = validate_expiry_date))]
    pub expiry_date: NaiveDate,
    #[validate(custom(function = validate_positive_option_price))]
    pub unit_price: Option<rust_decimal::Decimal>,
    pub storage_location: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateInventoryRequest {
    #[validate(range(min = 0, message = "Quantity cannot be negative"))]
    pub quantity: Option<i32>,
    #[validate(custom(function = validate_expiry_date))]
    pub expiry_date: Option<NaiveDate>,
    #[validate(custom(function = validate_positive_option_price))]
    pub unit_price: Option<rust_decimal::Decimal>,
    pub storage_location: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SearchInventoryRequest {
    pub pharmaceutical_id: Option<Uuid>,
    pub brand_name: Option<String>,
    pub generic_name: Option<String>,
    pub manufacturer: Option<String>,
    pub ndc_code: Option<String>,
    pub expiry_before: Option<NaiveDate>,
    pub expiry_after: Option<NaiveDate>,
    pub min_quantity: Option<i32>,
    pub max_quantity: Option<i32>,
    pub status: Option<String>,
    pub min_price: Option<rust_decimal::Decimal>,
    pub max_price: Option<rust_decimal::Decimal>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct InventoryResponse {
    pub id: Uuid,
    pub pharmaceutical: PharmaceuticalResponse,
    pub batch_number: String,
    pub quantity: i32,
    pub expiry_date: NaiveDate,
    pub days_to_expiry: i64,
    pub unit_price: Option<rust_decimal::Decimal>,
    pub storage_location: Option<String>,
    pub status: String,
    pub seller: UserResponse,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ExpiryAlertRequest {
    pub days_threshold: i64,
}

#[derive(Debug, Serialize)]
pub struct ExpiryAlert {
    pub inventory_id: Uuid,
    pub pharmaceutical_name: String,
    pub batch_number: String,
    pub quantity: i32,
    pub expiry_date: NaiveDate,
    pub days_to_expiry: i64,
    pub seller: String,
}

impl Inventory {
    pub fn days_to_expiry(&self) -> i64 {
        let today = chrono::Utc::now().date_naive();
        self.expiry_date.signed_duration_since(today).num_days()
    }

    pub fn is_expired(&self) -> bool {
        self.days_to_expiry() < 0
    }

    pub fn is_near_expiry(&self, days_threshold: i64) -> bool {
        let days_left = self.days_to_expiry();
        days_left >= 0 && days_left <= days_threshold
    }
}