use axum::{
    extract::{Path, Query, State},
    Json,
    Extension,
};
use validator::Validate;
use crate::{
    models::{pharmaceutical::{CreatePharmaceuticalRequest, SearchPharmaceuticalRequest}},
    services::PharmaService,
    middleware::{error_handling::Result, Claims},
    config::AppConfig,
};

pub async fn create_pharmaceutical(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<CreatePharmaceuticalRequest>,
) -> Result<Json<crate::models::pharmaceutical::PharmaceuticalResponse>> {
    if !claims.is_verified {
        return Err(crate::middleware::error_handling::AppError::Forbidden("Access denied".to_string()));
    }

    request.validate()
        .map_err(|e| crate::middleware::error_handling::AppError::Validation(e))?;

    let pharma_service = PharmaService::new(
        crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone())
    );

    let pharma = pharma_service.create_pharmaceutical(request).await?;
    Ok(Json(pharma))
}

pub async fn get_pharmaceutical(
    State(config): State<AppConfig>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<crate::models::pharmaceutical::PharmaceuticalResponse>> {
    let pharma_service = PharmaService::new(
        crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone())
    );

    let pharma = pharma_service.get_pharmaceutical(id).await?;
    Ok(Json(pharma))
}

pub async fn search_pharmaceuticals(
    State(config): State<AppConfig>,
    Query(request): Query<SearchPharmaceuticalRequest>,
) -> Result<Json<Vec<crate::models::pharmaceutical::PharmaceuticalResponse>>> {
    let pharma_service = PharmaService::new(
        crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone())
    );

    let results = pharma_service.search_pharmaceuticals(request).await?;
    Ok(Json(results))
}

pub async fn get_manufacturers(
    State(config): State<AppConfig>,
) -> Result<Json<Vec<String>>> {
    let pharma_service = PharmaService::new(
        crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone())
    );

    let manufacturers = pharma_service.get_manufacturers().await?;
    Ok(Json(manufacturers))
}

pub async fn get_categories(
    State(config): State<AppConfig>,
) -> Result<Json<Vec<String>>> {
    let pharma_service = PharmaService::new(
        crate::repositories::PharmaceuticalRepository::new(config.database_pool.clone())
    );

    let categories = pharma_service.get_categories().await?;
    Ok(Json(categories))
}