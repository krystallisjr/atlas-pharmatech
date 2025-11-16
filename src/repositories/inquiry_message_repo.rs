use sqlx::PgPool;
use uuid::Uuid;
use crate::middleware::error_handling::{Result, AppError};
use crate::models::{InquiryMessage, CreateInquiryMessageRequest};

pub struct InquiryMessageRepository {
    pool: PgPool,
}

impl InquiryMessageRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new message in an inquiry conversation
    pub async fn create(&self, user_id: Uuid, request: CreateInquiryMessageRequest) -> Result<InquiryMessage> {
        // Verify user is part of the inquiry (either buyer or seller)
        let inquiry = sqlx::query!(
            r#"
            SELECT i.id, i.buyer_id, inv.user_id as seller_id
            FROM inquiries i
            JOIN inventory inv ON i.inventory_id = inv.id
            WHERE i.id = $1
            "#,
            request.inquiry_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let inquiry = inquiry.ok_or(AppError::NotFound("Inquiry not found".to_string()))?;

        // Check if user is buyer or seller
        if inquiry.buyer_id != user_id && inquiry.seller_id != user_id {
            return Err(AppError::Forbidden("You are not part of this inquiry".to_string()));
        }

        // Validate message is not empty
        if request.message.trim().is_empty() {
            return Err(AppError::BadRequest("Message cannot be empty".to_string()));
        }

        // Create message
        let message = sqlx::query_as!(
            InquiryMessage,
            r#"
            INSERT INTO inquiry_messages (inquiry_id, sender_id, message)
            VALUES ($1, $2, $3)
            RETURNING id, inquiry_id, sender_id, message, created_at as "created_at!"
            "#,
            request.inquiry_id,
            user_id,
            request.message.trim()
        )
        .fetch_one(&self.pool)
        .await?;

        // Update inquiry status to 'negotiating' if it's still 'pending'
        sqlx::query!(
            r#"
            UPDATE inquiries
            SET status = 'negotiating'
            WHERE id = $1 AND status = 'pending'
            "#,
            request.inquiry_id
        )
        .execute(&self.pool)
        .await?;

        Ok(message)
    }

    /// Get all messages for an inquiry
    pub async fn get_by_inquiry_id(&self, user_id: Uuid, inquiry_id: Uuid) -> Result<Vec<InquiryMessage>> {
        // Verify user is part of the inquiry
        let inquiry = sqlx::query!(
            r#"
            SELECT i.id, i.buyer_id, inv.user_id as seller_id
            FROM inquiries i
            JOIN inventory inv ON i.inventory_id = inv.id
            WHERE i.id = $1
            "#,
            inquiry_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let inquiry = inquiry.ok_or(AppError::NotFound("Inquiry not found".to_string()))?;

        if inquiry.buyer_id != user_id && inquiry.seller_id != user_id {
            return Err(AppError::Forbidden("You are not part of this inquiry".to_string()));
        }

        // Get messages ordered by creation time
        let messages = sqlx::query_as!(
            InquiryMessage,
            r#"
            SELECT id, inquiry_id, sender_id, message, created_at as "created_at!"
            FROM inquiry_messages
            WHERE inquiry_id = $1
            ORDER BY created_at ASC
            "#,
            inquiry_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(messages)
    }

    /// Get message count for an inquiry
    pub async fn get_message_count(&self, inquiry_id: Uuid) -> Result<i64> {
        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM inquiry_messages
            WHERE inquiry_id = $1
            "#,
            inquiry_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }
}
