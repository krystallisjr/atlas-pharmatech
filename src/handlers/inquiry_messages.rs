use axum::{
    extract::{Path, State},
    Extension, Json,
};
use uuid::Uuid;

use crate::{
    config::AppConfig,
    middleware::{auth::Claims, error_handling::Result},
    models::{CreateInquiryMessageRequest, InquiryMessage, InquiryMessageResponse},
    repositories::InquiryMessageRepository,
};

/// Create a new message in an inquiry conversation
pub async fn create_message(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<CreateInquiryMessageRequest>,
) -> Result<Json<InquiryMessageResponse>> {
    let inquiry_id = request.inquiry_id;
    let repo = InquiryMessageRepository::new(config.database_pool.clone());

    let message = repo.create(claims.user_id, request).await?;

    // Get sender company name
    let sender = sqlx::query!(
        r#"
        SELECT company_name
        FROM users
        WHERE id = $1
        "#,
        message.sender_id
    )
    .fetch_one(&config.database_pool)
    .await?;

    // Get the other party in the inquiry to notify them
    let inquiry_info = sqlx::query!(
        r#"
        SELECT i.buyer_id, inv.user_id as seller_id
        FROM inquiries i
        JOIN inventory inv ON i.inventory_id = inv.id
        WHERE i.id = $1
        "#,
        inquiry_id
    )
    .fetch_one(&config.database_pool)
    .await?;

    // Determine recipient (the other party)
    let recipient_id = if claims.user_id == inquiry_info.buyer_id {
        inquiry_info.seller_id
    } else {
        inquiry_info.buyer_id
    };

    // Create notification for recipient
    let notification_service = crate::services::NotificationService::new(config.database_pool.clone());
    let alert_payload = crate::models::alerts::AlertPayload::new_inquiry_message(
        recipient_id,
        claims.user_id,
        &sender.company_name,
        inquiry_id,
    );

    // Fire and forget - don't fail message send if notification fails
    if let Err(e) = notification_service.create_alert(alert_payload).await {
        tracing::warn!("Failed to create message notification: {}", e);
    }

    let response = InquiryMessageResponse::new(message, sender.company_name);

    Ok(Json(response))
}

/// Get all messages for an inquiry
pub async fn get_inquiry_messages(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(inquiry_id): Path<Uuid>,
) -> Result<Json<Vec<InquiryMessageResponse>>> {
    let repo = InquiryMessageRepository::new(config.database_pool.clone());

    let messages = repo.get_by_inquiry_id(claims.user_id, inquiry_id).await?;

    // Get all sender company names in one query
    let sender_ids: Vec<Uuid> = messages.iter().map(|m| m.sender_id).collect();

    let senders = sqlx::query!(
        r#"
        SELECT id, company_name
        FROM users
        WHERE id = ANY($1)
        "#,
        &sender_ids
    )
    .fetch_all(&config.database_pool)
    .await?;

    // Create a map of sender_id -> company_name
    let sender_map: std::collections::HashMap<Uuid, String> = senders
        .into_iter()
        .map(|s| (s.id, s.company_name))
        .collect();

    // Build responses with company names
    let responses: Vec<InquiryMessageResponse> = messages
        .into_iter()
        .map(|m| {
            let company = sender_map
                .get(&m.sender_id)
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string());
            InquiryMessageResponse::new(m, company)
        })
        .collect();

    Ok(Json(responses))
}

/// Get message count for an inquiry
pub async fn get_message_count(
    State(config): State<AppConfig>,
    Path(inquiry_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let repo = InquiryMessageRepository::new(config.database_pool.clone());

    let count = repo.get_message_count(inquiry_id).await?;

    Ok(Json(serde_json::json!({ "count": count })))
}
