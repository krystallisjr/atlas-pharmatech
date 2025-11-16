/// REST API handlers for Natural Language Query system

use axum::{
    extract::{State, Path},
    Extension,
    Json,
};
use uuid::Uuid;
use crate::{
    config::AppConfig,
    middleware::{error_handling::Result, Claims},
    models::nl_query::*,
    services::NlQueryService,
};

/// POST /api/nl-query/execute
/// Execute a natural language query
pub async fn execute_query(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<ExecuteQueryRequest>,
) -> Result<Json<QueryResponse>> {
    tracing::info!("NL query requested by user: {}", claims.user_id);

    // Get Claude API key
    let claude_api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| crate::middleware::error_handling::AppError::Internal(
            anyhow::anyhow!("ANTHROPIC_API_KEY not configured")
        ))?;

    let service = NlQueryService::new(config.database_pool.clone(), claude_api_key);

    // Execute query
    let session = service.execute_query(claims.user_id, request.query).await?;

    Ok(Json(session.into()))
}

/// GET /api/nl-query/session/:id
/// Get query session details
pub async fn get_session(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<QueryResponse>> {
    let claude_api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| crate::middleware::error_handling::AppError::Internal(
            anyhow::anyhow!("ANTHROPIC_API_KEY not configured")
        ))?;

    let service = NlQueryService::new(config.database_pool.clone(), claude_api_key);
    let session = service.get_session(session_id).await?;

    // Verify user owns this session
    if session.user_id != claims.user_id {
        return Err(crate::middleware::error_handling::AppError::Forbidden(
            "Access denied".to_string()
        ));
    }

    Ok(Json(session.into()))
}

/// GET /api/nl-query/history
/// Get user's query history
pub async fn get_history(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<QueryHistoryItem>>> {
    let claude_api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| crate::middleware::error_handling::AppError::Internal(
            anyhow::anyhow!("ANTHROPIC_API_KEY not configured")
        ))?;

    let service = NlQueryService::new(config.database_pool.clone(), claude_api_key);
    let sessions = service.get_history(claims.user_id, 50, 0).await?;

    let history: Vec<QueryHistoryItem> = sessions.into_iter()
        .map(|s| QueryHistoryItem {
            id: s.id,
            query_text: s.query_text,
            status: s.status,
            result_count: s.result_count,
            execution_time_ms: s.execution_time_ms,
            created_at: s.created_at,
        })
        .collect();

    Ok(Json(history))
}

/// POST /api/nl-query/favorites
/// Save query as favorite
pub async fn save_favorite(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<SaveFavoriteRequest>,
) -> Result<Json<FavoriteResponse>> {
    let claude_api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| crate::middleware::error_handling::AppError::Internal(
            anyhow::anyhow!("ANTHROPIC_API_KEY not configured")
        ))?;

    let service = NlQueryService::new(config.database_pool.clone(), claude_api_key);
    let favorite = service.save_favorite(
        claims.user_id,
        request.query_text,
        request.description,
        request.category,
    ).await?;

    Ok(Json(favorite.into()))
}

/// GET /api/nl-query/favorites
/// Get user's favorite queries
pub async fn get_favorites(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<FavoriteResponse>>> {
    let claude_api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| crate::middleware::error_handling::AppError::Internal(
            anyhow::anyhow!("ANTHROPIC_API_KEY not configured")
        ))?;

    let service = NlQueryService::new(config.database_pool.clone(), claude_api_key);
    let favorites = service.get_favorites(claims.user_id).await?;

    let responses: Vec<FavoriteResponse> = favorites.into_iter()
        .map(|f| f.into())
        .collect();

    Ok(Json(responses))
}

/// GET /api/nl-query/quota
/// Get user's NL query quota status
pub async fn get_quota(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>> {
    let claude_api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| crate::middleware::error_handling::AppError::Internal(
            anyhow::anyhow!("ANTHROPIC_API_KEY not configured")
        ))?;

    let service = NlQueryService::new(config.database_pool.clone(), claude_api_key);
    let (limit, used, remaining) = service.get_quota_status(claims.user_id).await?;

    Ok(Json(serde_json::json!({
        "query_limit": limit,
        "queries_used": used,
        "queries_remaining": remaining
    })))
}
