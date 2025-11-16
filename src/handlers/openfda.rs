use axum::{
    extract::{Path, Query, State},
    Json,
    Extension,
};
use crate::{
    models::openfda::OpenFdaSearchRequest,
    services::OpenFdaService,
    middleware::{error_handling::Result, Claims},
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

/// Trigger sync from OpenFDA API (admin only)
pub async fn trigger_sync(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<serde_json::Value>,
) -> Result<Json<crate::models::openfda::OpenFdaSyncLog>> {
    // Extract limit from query params
    let limit = params.get("limit")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);

    let openfda_service = OpenFdaService::new(
        crate::repositories::OpenFdaRepository::new(config.database_pool.clone()),
    );

    // Trigger sync in background (for production, consider using a job queue)
    let sync_log = openfda_service.sync_from_api(limit).await?;

    Ok(Json(sync_log))
}
