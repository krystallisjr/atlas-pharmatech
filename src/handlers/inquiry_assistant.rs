/// REST API handlers for Inquiry Assistant system

use axum::{
    extract::{State, Path},
    Extension,
    Json,
};
use uuid::Uuid;
use crate::{
    config::AppConfig,
    middleware::{error_handling::Result, Claims},
    models::inquiry_assistant::*,
    services::InquiryAssistantService,
};

/// POST /api/inquiry-assistant/inquiries/:inquiry_id/suggestions
/// Generate AI suggestion for inquiry response
pub async fn generate_suggestion(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(inquiry_id): Path<Uuid>,
    Json(request): Json<GenerateSuggestionRequest>,
) -> Result<Json<SuggestionResponse>> {
    tracing::info!(
        "Inquiry assistant suggestion requested: inquiry={}, user={}, type={:?}",
        inquiry_id,
        claims.user_id,
        request.suggestion_type
    );

    // Get Claude API key
    let claude_api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| crate::middleware::error_handling::AppError::Internal(
            anyhow::anyhow!("ANTHROPIC_API_KEY not configured")
        ))?;

    let service = InquiryAssistantService::new(config.database_pool.clone(), claude_api_key);

    // Generate suggestion
    let suggestion = service.generate_suggestion(
        inquiry_id,
        claims.user_id,
        request.suggestion_type,
        request.custom_instructions,
    ).await?;

    Ok(Json(suggestion.into()))
}

/// POST /api/inquiry-assistant/suggestions/:suggestion_id/accept
/// Accept a suggestion and send it as a message
pub async fn accept_suggestion(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(suggestion_id): Path<Uuid>,
    Json(request): Json<AcceptSuggestionRequest>,
) -> Result<Json<AcceptSuggestionResponse>> {
    tracing::info!(
        "Accepting suggestion: id={}, user={}, edited={}",
        suggestion_id,
        claims.user_id,
        request.edited_text.is_some()
    );

    let claude_api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| crate::middleware::error_handling::AppError::Internal(
            anyhow::anyhow!("ANTHROPIC_API_KEY not configured")
        ))?;

    let service = InquiryAssistantService::new(config.database_pool.clone(), claude_api_key);

    // Accept suggestion and create message
    let message_id = service.accept_suggestion(
        suggestion_id,
        claims.user_id,
        request.edited_text.clone(),
    ).await?;

    Ok(Json(AcceptSuggestionResponse {
        message_id,
        was_edited: request.edited_text.is_some(),
    }))
}

/// GET /api/inquiry-assistant/suggestions/:suggestion_id
/// Get suggestion by ID
pub async fn get_suggestion(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(suggestion_id): Path<Uuid>,
) -> Result<Json<SuggestionResponse>> {
    let claude_api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| crate::middleware::error_handling::AppError::Internal(
            anyhow::anyhow!("ANTHROPIC_API_KEY not configured")
        ))?;

    let service = InquiryAssistantService::new(config.database_pool.clone(), claude_api_key);
    let suggestion = service.get_suggestion(suggestion_id, claims.user_id).await?;

    Ok(Json(suggestion.into()))
}

/// GET /api/inquiry-assistant/inquiries/:inquiry_id/suggestions
/// Get all suggestions for an inquiry
pub async fn get_inquiry_suggestions(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(inquiry_id): Path<Uuid>,
) -> Result<Json<Vec<SuggestionResponse>>> {
    let claude_api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| crate::middleware::error_handling::AppError::Internal(
            anyhow::anyhow!("ANTHROPIC_API_KEY not configured")
        ))?;

    let service = InquiryAssistantService::new(config.database_pool.clone(), claude_api_key);
    let suggestions = service.get_inquiry_suggestions(inquiry_id, claims.user_id).await?;

    let responses: Vec<SuggestionResponse> = suggestions.into_iter()
        .map(|s| s.into())
        .collect();

    Ok(Json(responses))
}

/// GET /api/inquiry-assistant/quota
/// Get user's inquiry assistant quota status
pub async fn get_quota(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>> {
    let claude_api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| crate::middleware::error_handling::AppError::Internal(
            anyhow::anyhow!("ANTHROPIC_API_KEY not configured")
        ))?;

    let service = InquiryAssistantService::new(config.database_pool.clone(), claude_api_key);
    let (limit, used, remaining) = service.get_quota_status(claims.user_id).await?;

    Ok(Json(serde_json::json!({
        "assist_limit": limit,
        "assists_used": used,
        "assists_remaining": remaining
    })))
}
