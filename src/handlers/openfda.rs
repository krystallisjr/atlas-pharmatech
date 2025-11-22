use axum::{
    extract::{Path, Query, State},
    Json,
    Extension,
};
use uuid::Uuid;
use serde::Deserialize;
use crate::{
    models::openfda::{OpenFdaSearchRequest, SyncProgressResponse},
    services::OpenFdaService,
    middleware::{error_handling::{Result, AppError}, Claims},
    config::AppConfig,
};

/// Search OpenFDA catalog with autocomplete
pub async fn search_catalog(
    State(config): State<AppConfig>,
    Query(request): Query<OpenFdaSearchRequest>,
) -> Result<Json<Vec<crate::models::openfda::OpenFdaCatalogResponse>>> {
    let openfda_service = OpenFdaService::new(
        crate::repositories::OpenFdaRepository::new(config.database_pool.clone()),
    );

    let results = openfda_service.search(request).await?;
    Ok(Json(results))
}

/// Get drug by NDC code
pub async fn get_by_ndc(
    State(config): State<AppConfig>,
    Path(ndc): Path<String>,
) -> Result<Json<Option<crate::models::openfda::OpenFdaCatalogResponse>>> {
    let openfda_service = OpenFdaService::new(
        crate::repositories::OpenFdaRepository::new(config.database_pool.clone()),
    );

    let result = openfda_service.get_by_ndc(&ndc).await?;
    Ok(Json(result))
}

/// Get catalog statistics
pub async fn get_stats(
    State(config): State<AppConfig>,
) -> Result<Json<crate::services::openfda_service::CatalogStats>> {
    let openfda_service = OpenFdaService::new(
        crate::repositories::OpenFdaRepository::new(config.database_pool.clone()),
    );

    let stats = openfda_service.get_stats().await?;
    Ok(Json(stats))
}

/// Get manufacturers from OpenFDA catalog with product counts
pub async fn get_manufacturers(
    State(config): State<AppConfig>,
) -> Result<Json<Vec<serde_json::Value>>> {
    use sqlx::{query, Row};

    let manufacturers = query(
        r#"
        SELECT
            labeler_name as manufacturer,
            COUNT(*) as count
        FROM openfda_catalog
        WHERE labeler_name IS NOT NULL AND labeler_name != ''
        GROUP BY labeler_name
        ORDER BY count DESC, labeler_name ASC
        LIMIT 100
        "#
    )
    .fetch_all(&config.database_pool)
    .await?;

    let result: Vec<serde_json::Value> = manufacturers.iter().map(|row| {
        serde_json::json!({
            "manufacturer": row.get::<String, _>("manufacturer"),
            "count": row.get::<i64, _>("count")
        })
    }).collect();

    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
pub struct TriggerSyncParams {
    pub sync_type: Option<String>,
    pub limit: Option<u64>,
}

/// Trigger sync from OpenFDA API (admin only)
/// Starts a background sync and returns the sync ID immediately
pub async fn trigger_sync(
    State(config): State<AppConfig>,
    Extension(_claims): Extension<Claims>,
    Query(params): Query<TriggerSyncParams>,
) -> Result<Json<TriggerSyncResponse>> {
    let openfda_service = OpenFdaService::new(
        crate::repositories::OpenFdaRepository::new(config.database_pool.clone()),
    );

    let sync_type = params.sync_type.unwrap_or_else(|| "manual".to_string());

    // Start background sync
    let sync_id = openfda_service.start_background_sync(&sync_type, config.database_pool.clone()).await?;

    Ok(Json(TriggerSyncResponse {
        sync_id,
        message: "Sync started in background. Use /api/openfda/sync/{id} to check progress.".to_string(),
    }))
}

#[derive(Debug, serde::Serialize)]
pub struct TriggerSyncResponse {
    pub sync_id: Uuid,
    pub message: String,
}

/// Get sync progress by ID
pub async fn get_sync_progress(
    State(config): State<AppConfig>,
    Path(sync_id): Path<Uuid>,
) -> Result<Json<SyncProgressResponse>> {
    let openfda_service = OpenFdaService::new(
        crate::repositories::OpenFdaRepository::new(config.database_pool.clone()),
    );

    match openfda_service.get_sync_progress(sync_id).await? {
        Some(progress) => Ok(Json(progress)),
        None => Err(AppError::NotFound(format!("Sync log {} not found", sync_id))),
    }
}

/// Get active sync (if any)
pub async fn get_active_sync(
    State(config): State<AppConfig>,
) -> Result<Json<Option<SyncProgressResponse>>> {
    let openfda_service = OpenFdaService::new(
        crate::repositories::OpenFdaRepository::new(config.database_pool.clone()),
    );

    let active = openfda_service.get_active_sync().await?;
    Ok(Json(active))
}

#[derive(Debug, Deserialize)]
pub struct SyncLogsParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Get sync logs history
pub async fn get_sync_logs(
    State(config): State<AppConfig>,
    Query(params): Query<SyncLogsParams>,
) -> Result<Json<Vec<SyncProgressResponse>>> {
    let openfda_service = OpenFdaService::new(
        crate::repositories::OpenFdaRepository::new(config.database_pool.clone()),
    );

    let logs = openfda_service.get_sync_logs(params.limit, params.offset).await?;
    Ok(Json(logs))
}

/// Cancel a running sync
pub async fn cancel_sync(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(sync_id): Path<Uuid>,
) -> Result<Json<CancelSyncResponse>> {
    let openfda_service = OpenFdaService::new(
        crate::repositories::OpenFdaRepository::new(config.database_pool.clone()),
    );

    let cancelled = openfda_service.cancel_sync(sync_id, claims.user_id).await?;

    Ok(Json(CancelSyncResponse {
        cancelled,
        message: if cancelled {
            "Sync cancellation requested".to_string()
        } else {
            "Sync not found or not in progress".to_string()
        },
    }))
}

#[derive(Debug, serde::Serialize)]
pub struct CancelSyncResponse {
    pub cancelled: bool,
    pub message: String,
}

/// Check if catalog needs refresh
pub async fn check_refresh_status(
    State(config): State<AppConfig>,
) -> Result<Json<RefreshStatusResponse>> {
    let openfda_service = OpenFdaService::new(
        crate::repositories::OpenFdaRepository::new(config.database_pool.clone()),
    );

    let needs_refresh = openfda_service.needs_refresh().await?;
    let is_running = openfda_service.is_sync_running().await?;
    let stats = openfda_service.get_stats().await?;

    Ok(Json(RefreshStatusResponse {
        needs_refresh,
        is_sync_running: is_running,
        total_entries: stats.total_entries,
        last_sync_at: stats.last_sync_at,
    }))
}

#[derive(Debug, serde::Serialize)]
pub struct RefreshStatusResponse {
    pub needs_refresh: bool,
    pub is_sync_running: bool,
    pub total_entries: i64,
    pub last_sync_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CleanupParams {
    pub days_to_keep: Option<i32>,
}

/// Cleanup old sync logs (admin only)
pub async fn cleanup_sync_logs(
    State(config): State<AppConfig>,
    Extension(_claims): Extension<Claims>,
    Query(params): Query<CleanupParams>,
) -> Result<Json<CleanupResponse>> {
    let openfda_service = OpenFdaService::new(
        crate::repositories::OpenFdaRepository::new(config.database_pool.clone()),
    );

    let days_to_keep = params.days_to_keep.unwrap_or(30);
    let deleted = openfda_service.cleanup_old_logs(days_to_keep).await?;

    Ok(Json(CleanupResponse {
        deleted_count: deleted,
        message: format!("Deleted {} old sync logs (kept last {} days)", deleted, days_to_keep),
    }))
}

#[derive(Debug, serde::Serialize)]
pub struct CleanupResponse {
    pub deleted_count: i64,
    pub message: String,
}

/// Health check endpoint
pub async fn health_check(
    State(config): State<AppConfig>,
) -> Result<Json<HealthCheckResponse>> {
    let openfda_service = OpenFdaService::new(
        crate::repositories::OpenFdaRepository::new(config.database_pool.clone()),
    );

    let stats = openfda_service.get_stats().await?;
    let is_running = openfda_service.is_sync_running().await?;

    Ok(Json(HealthCheckResponse {
        status: "healthy".to_string(),
        catalog_size: stats.total_entries,
        last_sync_at: stats.last_sync_at,
        sync_in_progress: is_running,
    }))
}

#[derive(Debug, serde::Serialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub catalog_size: i64,
    pub last_sync_at: Option<chrono::DateTime<chrono::Utc>>,
    pub sync_in_progress: bool,
}
