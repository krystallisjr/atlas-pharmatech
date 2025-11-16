use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::{Validate, ValidationError};

// Validation function - defined before structs that use it
pub fn validate_positive_price(price: &rust_decimal::Decimal) -> Result<(), ValidationError> {
    if *price > rust_decimal::Decimal::ZERO {
        Ok(())
    } else {
        Err(ValidationError::new("positive_price"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Inquiry {
    pub id: Uuid,
    pub inventory_id: Uuid,
    pub buyer_id: Uuid,
    pub quantity_requested: i32,
    pub message: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateInquiryRequest {
    pub inventory_id: Uuid,
    #[validate(range(min = 1, message = "Quantity must be at least 1"))]
    pub quantity_requested: i32,
    #[validate(length(max = 1000, message = "Message too long"))]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateInquiryRequest {
    pub status: Option<String>,
    #[validate(length(max = 1000, message = "Response message too long"))]
    pub response_message: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct InquiryResponse {
    pub id: Uuid,
    pub inventory_id: Uuid,
    pub buyer_id: Uuid,
    pub quantity_requested: i32,
    pub message: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // Nested objects for frontend
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inventory: Option<crate::models::inventory::InventoryResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buyer: Option<crate::models::user::UserResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seller: Option<crate::models::user::UserResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Transaction {
    pub id: Uuid,
    pub inquiry_id: Uuid,
    pub seller_id: Uuid,
    pub buyer_id: Uuid,
    pub quantity: i32,
    pub unit_price: rust_decimal::Decimal,
    pub total_price: rust_decimal::Decimal,
    pub transaction_date: DateTime<Utc>,
    pub status: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateTransactionRequest {
    pub inquiry_id: Uuid,
    #[validate(range(min = 1, message = "Quantity must be at least 1"))]
    pub quantity: i32,
    #[validate(custom(function = validate_positive_price))]
    pub unit_price: rust_decimal::Decimal,
}

#[derive(Debug, Serialize, Clone)]
pub struct TransactionResponse {
    pub id: Uuid,
    pub inquiry_id: Uuid,
    pub seller_id: Uuid,
    pub buyer_id: Uuid,
    pub quantity: i32,
    pub unit_price: rust_decimal::Decimal,
    pub total_price: rust_decimal::Decimal,
    pub transaction_date: DateTime<Utc>,
    pub status: String,
}

impl From<Inquiry> for InquiryResponse {
    fn from(inquiry: Inquiry) -> Self {
        Self {
            id: inquiry.id,
            inventory_id: inquiry.inventory_id,
            buyer_id: inquiry.buyer_id,
            quantity_requested: inquiry.quantity_requested,
            message: inquiry.message,
            status: inquiry.status,
            created_at: inquiry.created_at,
            updated_at: inquiry.updated_at,
            inventory: None,
            buyer: None,
            seller: None,
        }
    }
}

impl From<Transaction> for TransactionResponse {
    fn from(transaction: Transaction) -> Self {
        Self {
            id: transaction.id,
            inquiry_id: transaction.inquiry_id,
            seller_id: transaction.seller_id,
            buyer_id: transaction.buyer_id,
            quantity: transaction.quantity,
            unit_price: transaction.unit_price,
            total_price: transaction.total_price,
            transaction_date: transaction.transaction_date,
            status: transaction.status,
        }
    }
}