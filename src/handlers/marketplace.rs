use axum::{
    extract::{Path, Query, State},
    Json,
    Extension,
};
use validator::Validate;
use crate::{
    models::{
        marketplace::{CreateInquiryRequest, UpdateInquiryRequest, CreateTransactionRequest},
    },
    services::MarketplaceService,
    middleware::{error_handling::Result, Claims},
    config::AppConfig,
};

pub async fn create_inquiry(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<CreateInquiryRequest>,
) -> Result<Json<crate::models::marketplace::InquiryResponse>> {
    request.validate()
        .map_err(|e| crate::middleware::error_handling::AppError::Validation(e))?;

    let inventory_repo = crate::repositories::InventoryRepository::new(config.database_pool.clone());
    let user_repo = crate::repositories::UserRepository::new(config.database_pool.clone(), &config.encryption_key)?;

    // Get inventory to find seller and product name
    let inventory = inventory_repo
        .find_by_id(request.inventory_id)
        .await?
        .ok_or(crate::middleware::error_handling::AppError::NotFound("Inventory not found".to_string()))?;

    let seller_id = inventory.user_id;

    // Get buyer company name
    let buyer = user_repo.find_by_id(claims.user_id).await?
        .ok_or(crate::middleware::error_handling::AppError::NotFound("User not found".to_string()))?;

    // Get product name from pharmaceuticals
    let pharma_repo = crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone());
    let pharma = pharma_repo.find_by_id(inventory.pharmaceutical_id).await?
        .ok_or(crate::middleware::error_handling::AppError::NotFound("Product not found".to_string()))?;

    let marketplace_service = MarketplaceService::new(
        crate::repositories::MarketplaceRepository::new(config.database_pool.clone()),
        inventory_repo,
        user_repo,
        pharma_repo,
        crate::services::InventoryService::new(
            crate::repositories::InventoryRepository::new(config.database_pool.clone()),
            crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
        ),
    );

    let inquiry = marketplace_service.create_inquiry(request.clone(), claims.user_id).await?;

    // Create notification for seller
    let notification_service = crate::services::NotificationService::new(config.database_pool.clone());
    let product_name = format!("{} {}", pharma.brand_name, pharma.generic_name);
    let alert_payload = crate::models::alerts::AlertPayload::new_inquiry(
        seller_id,
        claims.user_id,
        &buyer.company_name,
        &product_name,
        request.quantity_requested,
        inquiry.id,
        request.inventory_id,
    );

    // Fire and forget - don't fail inquiry creation if notification fails
    if let Err(e) = notification_service.create_alert(alert_payload).await {
        tracing::warn!("Failed to create inquiry notification: {}", e);
    }

    Ok(Json(inquiry))
}

pub async fn get_inquiry(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(inquiry_id): Path<uuid::Uuid>,
) -> Result<Json<crate::models::marketplace::InquiryResponse>> {
    let marketplace_service = MarketplaceService::new(
        crate::repositories::MarketplaceRepository::new(config.database_pool.clone()),
        crate::repositories::InventoryRepository::new(config.database_pool.clone()),
        crate::repositories::UserRepository::new(config.database_pool.clone(), &config.encryption_key)?,
        crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
        crate::services::InventoryService::new(
            crate::repositories::InventoryRepository::new(config.database_pool.clone()),
            crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
        ),
    );

    let inquiry = marketplace_service.get_inquiry(inquiry_id, claims.user_id).await?;
    Ok(Json(inquiry))
}

pub async fn get_buyer_inquiries(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<serde_json::Value>,
) -> Result<Json<Vec<crate::models::marketplace::InquiryResponse>>> {
    let limit = params.get("limit").and_then(|v| v.as_i64()).map(|v| v as i64);
    let offset = params.get("offset").and_then(|v| v.as_i64()).map(|v| v as i64);

    let marketplace_service = MarketplaceService::new(
        crate::repositories::MarketplaceRepository::new(config.database_pool.clone()),
        crate::repositories::InventoryRepository::new(config.database_pool.clone()),
        crate::repositories::UserRepository::new(config.database_pool.clone(), &config.encryption_key)?,
        crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
        crate::services::InventoryService::new(
            crate::repositories::InventoryRepository::new(config.database_pool.clone()),
            crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
        ),
    );

    let inquiries = marketplace_service.get_buyer_inquiries(claims.user_id, limit, offset).await?;
    Ok(Json(inquiries))
}

pub async fn get_seller_inquiries(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<serde_json::Value>,
) -> Result<Json<Vec<crate::models::marketplace::InquiryResponse>>> {
    let limit = params.get("limit").and_then(|v| v.as_i64()).map(|v| v as i64);
    let offset = params.get("offset").and_then(|v| v.as_i64()).map(|v| v as i64);

    let marketplace_service = MarketplaceService::new(
        crate::repositories::MarketplaceRepository::new(config.database_pool.clone()),
        crate::repositories::InventoryRepository::new(config.database_pool.clone()),
        crate::repositories::UserRepository::new(config.database_pool.clone(), &config.encryption_key)?,
        crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
        crate::services::InventoryService::new(
            crate::repositories::InventoryRepository::new(config.database_pool.clone()),
            crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
        ),
    );

    let inquiries = marketplace_service.get_seller_inquiries(claims.user_id, limit, offset).await?;
    Ok(Json(inquiries))
}

pub async fn update_inquiry_status(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(inquiry_id): Path<uuid::Uuid>,
    Json(request): Json<UpdateInquiryRequest>,
) -> Result<Json<crate::models::marketplace::InquiryResponse>> {
    request.validate()
        .map_err(|e| crate::middleware::error_handling::AppError::Validation(e))?;

    let marketplace_service = MarketplaceService::new(
        crate::repositories::MarketplaceRepository::new(config.database_pool.clone()),
        crate::repositories::InventoryRepository::new(config.database_pool.clone()),
        crate::repositories::UserRepository::new(config.database_pool.clone(), &config.encryption_key)?,
        crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
        crate::services::InventoryService::new(
            crate::repositories::InventoryRepository::new(config.database_pool.clone()),
            crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
        ),
    );

    let inquiry = marketplace_service.update_inquiry_status(inquiry_id, claims.user_id, request).await?;
    Ok(Json(inquiry))
}

pub async fn create_transaction(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<CreateTransactionRequest>,
) -> Result<Json<crate::models::marketplace::TransactionResponse>> {
    request.validate()
        .map_err(|e| crate::middleware::error_handling::AppError::Validation(e))?;

    let marketplace_repo = crate::repositories::MarketplaceRepository::new(config.database_pool.clone());
    let inventory_repo = crate::repositories::InventoryRepository::new(config.database_pool.clone());

    // First, get the inquiry to determine buyer and seller
    let inquiry = marketplace_repo
        .find_inquiry_by_id(request.inquiry_id)
        .await?
        .ok_or(crate::middleware::error_handling::AppError::NotFound("Inquiry not found".to_string()))?;

    // Get inventory to determine seller
    let inventory = inventory_repo
        .find_by_id(inquiry.inventory_id)
        .await?
        .ok_or(crate::middleware::error_handling::AppError::NotFound("Inventory not found".to_string()))?;

    let seller_id = inventory.user_id;
    let buyer_id = inquiry.buyer_id;

    // Verify current user is either buyer or seller
    if claims.user_id != seller_id && claims.user_id != buyer_id {
        return Err(crate::middleware::error_handling::AppError::Forbidden("Access denied".to_string()));
    }

    let marketplace_service = MarketplaceService::new(
        marketplace_repo,
        inventory_repo,
        crate::repositories::UserRepository::new(config.database_pool.clone(), &config.encryption_key)?,
        crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
        crate::services::InventoryService::new(
            crate::repositories::InventoryRepository::new(config.database_pool.clone()),
            crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
        ),
    );

    let transaction = marketplace_service.create_transaction(request, seller_id, buyer_id).await?;
    Ok(Json(transaction))
}

pub async fn get_transaction(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(transaction_id): Path<uuid::Uuid>,
) -> Result<Json<crate::models::marketplace::TransactionResponse>> {
    let marketplace_service = MarketplaceService::new(
        crate::repositories::MarketplaceRepository::new(config.database_pool.clone()),
        crate::repositories::InventoryRepository::new(config.database_pool.clone()),
        crate::repositories::UserRepository::new(config.database_pool.clone(), &config.encryption_key)?,
        crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
        crate::services::InventoryService::new(
            crate::repositories::InventoryRepository::new(config.database_pool.clone()),
            crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
        ),
    );

    let transaction = marketplace_service.get_transaction(transaction_id, claims.user_id).await?;
    Ok(Json(transaction))
}

pub async fn get_user_transactions(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<serde_json::Value>,
) -> Result<Json<Vec<crate::models::marketplace::TransactionResponse>>> {
    let limit = params.get("limit").and_then(|v| v.as_i64()).map(|v| v as i64);
    let offset = params.get("offset").and_then(|v| v.as_i64()).map(|v| v as i64);

    let marketplace_service = MarketplaceService::new(
        crate::repositories::MarketplaceRepository::new(config.database_pool.clone()),
        crate::repositories::InventoryRepository::new(config.database_pool.clone()),
        crate::repositories::UserRepository::new(config.database_pool.clone(), &config.encryption_key)?,
        crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
        crate::services::InventoryService::new(
            crate::repositories::InventoryRepository::new(config.database_pool.clone()),
            crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
        ),
    );

    let transactions = marketplace_service.get_user_transactions(claims.user_id, limit, offset).await?;
    Ok(Json(transactions))
}

pub async fn complete_transaction(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(transaction_id): Path<uuid::Uuid>,
) -> Result<Json<crate::models::marketplace::TransactionResponse>> {
    let marketplace_service = MarketplaceService::new(
        crate::repositories::MarketplaceRepository::new(config.database_pool.clone()),
        crate::repositories::InventoryRepository::new(config.database_pool.clone()),
        crate::repositories::UserRepository::new(config.database_pool.clone(), &config.encryption_key)?,
        crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
        crate::services::InventoryService::new(
            crate::repositories::InventoryRepository::new(config.database_pool.clone()),
            crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
        ),
    );

    let transaction = marketplace_service.complete_transaction(transaction_id, claims.user_id).await?;
    Ok(Json(transaction))
}

pub async fn cancel_transaction(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(transaction_id): Path<uuid::Uuid>,
) -> Result<Json<crate::models::marketplace::TransactionResponse>> {
    let marketplace_service = MarketplaceService::new(
        crate::repositories::MarketplaceRepository::new(config.database_pool.clone()),
        crate::repositories::InventoryRepository::new(config.database_pool.clone()),
        crate::repositories::UserRepository::new(config.database_pool.clone(), &config.encryption_key)?,
        crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
        crate::services::InventoryService::new(
            crate::repositories::InventoryRepository::new(config.database_pool.clone()),
            crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
        ),
    );

    let transaction = marketplace_service.cancel_transaction(transaction_id, claims.user_id).await?;
    Ok(Json(transaction))
}