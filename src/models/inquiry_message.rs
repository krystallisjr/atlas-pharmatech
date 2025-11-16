use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InquiryMessage {
    pub id: Uuid,
    pub inquiry_id: Uuid,
    pub sender_id: Uuid,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInquiryMessageRequest {
    pub inquiry_id: Uuid,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct InquiryMessageResponse {
    pub id: Uuid,
    pub inquiry_id: Uuid,
    pub sender_id: Uuid,
    pub sender_company: String,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

impl InquiryMessageResponse {
    pub fn new(message: InquiryMessage, sender_company: String) -> Self {
        Self {
            id: message.id,
            inquiry_id: message.inquiry_id,
            sender_id: message.sender_id,
            sender_company,
            message: message.message,
            created_at: message.created_at,
        }
    }
}
