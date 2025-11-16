use axum::{
    extract::{Path, Query, State},
    Json,
    Extension,
};
use validator::Validate;
use crate::{
    models::{
        inventory::{CreateInventoryRequest, UpdateInventoryRequest, SearchInventoryRequest},
    },
    services::InventoryService,
    middleware::{error_handling::Result, Claims},
    config::AppConfig,
};

pub async fn add_inventory(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<CreateInventoryRequest>,
) -> Result<Json<crate::models::inventory::InventoryResponse>> {
    request.validate()
        .map_err(|e| crate::middleware::error_handling::AppError::Validation(e))?;

    let inventory_service = InventoryService::new(
        crate::repositories::InventoryRepository::new(config.database_pool.clone()),
        crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
    );

    let inventory = inventory_service.add_inventory(request, claims.user_id).await?;
    Ok(Json(inventory))
}

pub async fn get_inventory(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(inventory_id): Path<uuid::Uuid>,
) -> Result<Json<crate::models::inventory::InventoryResponse>> {
    let inventory_service = InventoryService::new(
        crate::repositories::InventoryRepository::new(config.database_pool.clone()),
        crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
    );

    let inventory = inventory_service.get_inventory(inventory_id, claims.user_id).await?;
    Ok(Json(inventory))
}

pub async fn get_user_inventory(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<serde_json::Value>,
) -> Result<Json<Vec<crate::models::inventory::InventoryResponse>>> {
    let limit = params.get("limit").and_then(|v| v.as_i64()).map(|v| v as i64);
    let offset = params.get("offset").and_then(|v| v.as_i64()).map(|v| v as i64);

    let inventory_service = InventoryService::new(
        crate::repositories::InventoryRepository::new(config.database_pool.clone()),
        crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
    );

    let inventories = inventory_service.get_user_inventory(claims.user_id, limit, offset).await?;
    Ok(Json(inventories))
}

pub async fn update_inventory(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(inventory_id): Path<uuid::Uuid>,
    Json(request): Json<UpdateInventoryRequest>,
) -> Result<Json<crate::models::inventory::InventoryResponse>> {
    request.validate()
        .map_err(|e| crate::middleware::error_handling::AppError::Validation(e))?;

    let inventory_service = InventoryService::new(
        crate::repositories::InventoryRepository::new(config.database_pool.clone()),
        crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
    );

    let inventory = inventory_service.update_inventory(inventory_id, claims.user_id, request).await?;
    Ok(Json(inventory))
}

pub async fn delete_inventory(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(inventory_id): Path<uuid::Uuid>,
) -> Result<axum::http::StatusCode> {
    let inventory_service = InventoryService::new(
        crate::repositories::InventoryRepository::new(config.database_pool.clone()),
        crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
    );

    inventory_service.delete_inventory(inventory_id, claims.user_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn search_marketplace(
    State(config): State<AppConfig>,
    Query(request): Query<SearchInventoryRequest>,
) -> Result<Json<Vec<crate::models::inventory::InventoryResponse>>> {
    let inventory_service = InventoryService::new(
        crate::repositories::InventoryRepository::new(config.database_pool.clone()),
        crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
    );

    let results = inventory_service.search_marketplace(request).await?;
    Ok(Json(results))
}

pub async fn get_expiry_alerts(
    State(config): State<AppConfig>,
    Query(request): Query<crate::models::inventory::ExpiryAlertRequest>,
) -> Result<Json<Vec<crate::models::inventory::ExpiryAlert>>> {
    let inventory_service = InventoryService::new(
        crate::repositories::InventoryRepository::new(config.database_pool.clone()),
        crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone()),
    );

    let alerts = inventory_service.get_expiry_alerts(request.days_threshold).await?;
    Ok(Json(alerts))
}