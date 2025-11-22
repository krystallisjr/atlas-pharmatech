use axum::{
    extract::{Path, Query, State},
    Json,
    Extension,
    http::StatusCode,
};
use crate::{
    models::ema::{
        EmaSearchRequest, EmaCatalogResponse, EmaCatalogStats, EmaSyncLog
    },
    services::ema_service::EmaService,
    repositories::ema_repo::EmaRepository,
    middleware::{
        error_handling::Result,
        auth::Claims
    },
    config::AppConfig,
};

/// Search EMA catalog with full-text search and filters
///
/// # Query Parameters:
/// - `query`: Search term for product name, INN, MAH, or EU number
/// - `language`: Filter by language code (en, de, fr, etc.)
/// - `authorization_status`: Filter by authorization status
/// - `therapeutic_area`: Filter by therapeutic area
/// - `atc_code`: Filter by ATC code
/// - `mah_name`: Filter by Marketing Authorization Holder name
/// - `limit`: Maximum number of results (default: 20, max: 100)
/// - `offset`: Offset for pagination (default: 0)
///
/// # Response:
/// Returns array of EMA catalog entries matching search criteria
pub async fn search_catalog(
    State(config): State<AppConfig>,
    Query(request): Query<EmaSearchRequest>,
) -> Result<Json<Vec<EmaCatalogResponse>>> {
    // Validate language if provided
    if let Some(ref lang) = request.language {
        let ema_service = EmaService::new(EmaRepository::new(config.database_pool.clone()));
        ema_service.validate_language(lang)?;
    }

    let ema_service = EmaService::new(EmaRepository::new(config.database_pool.clone()));
    let results = ema_service.search(request).await?;
    Ok(Json(results))
}

/// Get medicine by EU number
///
/// # Path Parameters:
/// - `eu_number`: The EU number of the medicine (format: EU/1/XX/XXX/XXX)
///
/// # Response:
/// Returns the EMA catalog entry if found, otherwise null
pub async fn get_by_eu_number(
    State(config): State<AppConfig>,
    Path(eu_number): Path<String>,
) -> Result<Json<Option<EmaCatalogResponse>>> {
    // Validate EU number format
    let ema_service = EmaService::new(EmaRepository::new(config.database_pool.clone()));
    ema_service.validate_eu_number(&eu_number)?;

    let result = ema_service.get_by_eu_number(&eu_number).await?;
    Ok(Json(result))
}

/// Get catalog statistics and metadata
///
/// # Response:
/// Returns comprehensive statistics about the EMA catalog including:
/// - Total number of entries
/// - Counts by language, status, therapeutic area
/// - Orphan medicines count
/// - Last sync information
pub async fn get_stats(
    State(config): State<AppConfig>,
) -> Result<Json<EmaCatalogStats>> {
    let ema_service = EmaService::new(EmaRepository::new(config.database_pool.clone()));
    let stats = ema_service.get_stats().await?;
    Ok(Json(stats))
}

/// Get synchronization logs with pagination
///
/// # Query Parameters:
/// - `limit`: Number of logs to return (default: 20, max: 100)
/// - `offset`: Offset for pagination (default: 0)
///
/// # Response:
/// Returns array of sync log entries
pub async fn get_sync_logs(
    State(config): State<AppConfig>,
    Query(params): Query<serde_json::Value>,
) -> Result<Json<Vec<EmaSyncLog>>> {
    let limit: i64 = params.get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(20)
        .min(100) as i64;

    let offset: i64 = params.get("offset")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as i64;

    let ema_service = EmaService::new(EmaRepository::new(config.database_pool.clone()));
    let logs = ema_service.get_sync_logs(Some(limit), Some(offset)).await?;
    Ok(Json(logs))
}

/// Trigger sync from EMA API (admin only)
///
/// # Query Parameters:
/// - `language`: Language to sync (default: en)
/// - `limit`: Maximum number of records to sync (default: 1000)
/// - `sync_type`: Type of sync - "full", "incremental", "by_language" (default: full)
///
/// # Response:
/// Returns sync log information about the triggered sync operation
///
/// # Security:
/// Requires admin authentication
pub async fn trigger_sync(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<serde_json::Value>,
) -> Result<Json<EmaSyncLog>> {
    // Verify admin role
    if !claims.is_admin() {
        return Err(crate::middleware::error_handling::AppError::Forbidden(
            "Admin access required to trigger EMA sync".to_string()
        ));
    }

    // Extract parameters with validation
    let language = params.get("language")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let limit = params.get("limit")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);

    let sync_type = params.get("sync_type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Validate parameters
    let ema_service = EmaService::new(EmaRepository::new(config.database_pool.clone()));

    if let Some(ref lang) = language {
        ema_service.validate_language(lang)?;
    }

    if let Some(sync_type_str) = &sync_type {
        if !["full", "incremental", "by_language"].contains(&sync_type_str.as_str()) {
            return Err(crate::middleware::error_handling::AppError::BadRequest(
                "Invalid sync_type. Must be one of: full, incremental, by_language".to_string()
            ));
        }
    }

    // Trigger sync operation
    tracing::info!(
        "Admin '{}' triggering EMA sync: language={:?}, limit={:?}, sync_type={:?}",
        claims.sub, language, limit, sync_type
    );

    let sync_log = ema_service.sync_from_api(language, limit, sync_type).await?;

    tracing::info!(
        "EMA sync triggered successfully. Sync log ID: {}, Status: {}",
        sync_log.id, sync_log.status
    );

    Ok(Json(sync_log))
}

/// Check if catalog needs refresh
///
/// # Query Parameters:
/// - `days_threshold`: Number of days to consider data stale (default: 7)
///
/// # Response:
/// Returns JSON object with `needs_refresh` boolean field
pub async fn check_refresh_status(
    State(config): State<AppConfig>,
    Query(params): Query<serde_json::Value>,
) -> Result<Json<serde_json::Value>> {
    let days_threshold = params.get("days_threshold")
        .and_then(|v| v.as_i64())
        .unwrap_or(7);

    let ema_service = EmaService::new(EmaRepository::new(config.database_pool.clone()));
    let needs_refresh = ema_service.needs_refresh(Some(days_threshold)).await?;

    Ok(Json(serde_json::json!({
        "needs_refresh": needs_refresh,
        "days_threshold": days_threshold,
        "timestamp": chrono::Utc::now()
    })))
}

/// Get service configuration and supported languages
///
/// # Response:
/// Returns configuration information including API URLs, supported languages, etc.
pub async fn get_config_info(
    State(config): State<AppConfig>,
) -> Result<Json<serde_json::Value>> {
    let ema_service = EmaService::new(EmaRepository::new(config.database_pool.clone()));
    let config_info = ema_service.get_config_info();

    Ok(Json(serde_json::json!({
        "ema_service": config_info,
        "service_version": "1.0.0",
        "api_documentation": "https://epi.developer.ema.europa.eu",
        "features": {
            "full_text_search": true,
            "multi_language": true,
            "batch_sync": true,
            "real_time_sync": true,
            "sync_tracking": true
        }
    })))
}

/// Clean up old sync logs (admin only)
///
/// # Response:
/// Returns JSON object with `deleted_count` field
///
/// # Security:
/// Requires admin authentication
pub async fn cleanup_sync_logs(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>> {
    // Verify admin role
    if !claims.is_admin() {
        return Err(crate::middleware::error_handling::AppError::Forbidden(
            "Admin access required to cleanup sync logs".to_string()
        ));
    }

    let ema_service = EmaService::new(EmaRepository::new(config.database_pool.clone()));
    let deleted_count = ema_service.cleanup_old_sync_logs().await?;

    tracing::info!(
        "Admin '{}' cleaned up {} old sync logs",
        claims.sub, deleted_count
    );

    Ok(Json(serde_json::json!({
        "deleted_count": deleted_count,
        "timestamp": chrono::Utc::now()
    })))
}

/// Health check endpoint for EMA service
///
/// # Response:
/// Returns health status including database connectivity and last sync info
pub async fn health_check(
    State(config): State<AppConfig>,
) -> Result<Json<serde_json::Value>> {
    let ema_service = EmaService::new(EmaRepository::new(config.database_pool.clone()));

    // Test database connectivity
    let db_healthy = config.database_pool.acquire().await.is_ok();

    // Get last sync info
    let last_sync = ema_service.get_sync_logs(Some(1), Some(0)).await.ok();
    let last_successful_sync = last_sync.and_then(|logs| {
        logs.into_iter().find(|log| log.status == "completed")
    });

    let health_status = match db_healthy {
        true => "healthy",
        false => "unhealthy",
    };

    let status_code = match db_healthy {
        true => StatusCode::OK,
        false => StatusCode::SERVICE_UNAVAILABLE,
    };

    let health_info = serde_json::json!({
        "status": health_status,
        "service": "EMA Catalog Service",
        "version": "1.0.0",
        "timestamp": chrono::Utc::now(),
        "database": {
            "status": if db_healthy { "connected" } else { "disconnected" }
        },
        "last_sync": last_successful_sync.map(|log| serde_json::json!({
            "id": log.id,
            "started_at": log.sync_started_at,
            "completed_at": log.sync_completed_at,
            "status": log.status,
            "records_processed": log.records_fetched
        })),
        "features": {
            "search": true,
            "sync": true,
            "multi_language": true
        }
    });

    // Set appropriate status code based on health
    if !db_healthy {
        return Err(crate::middleware::error_handling::AppError::Internal(
            anyhow::anyhow!("Database connection failed")
        ));
    }

    Ok(Json(health_info))
}