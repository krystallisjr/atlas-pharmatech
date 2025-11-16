// ERP AI Integration API Handlers
// AI-powered features for ERP integration: auto-discovery, sync analysis, conflict resolution
// Production-ready with authentication, validation, audit logging, and quota enforcement

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    Extension,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::middleware::auth::Claims;
use crate::middleware::error_handling::{AppError, Result};
use crate::services::erp::erp_ai_assistant_service::{
    ErpAiAssistantService, MappingDiscoveryResponse, SyncInsight,
    ConflictResolutionResponse,
};
use crate::services::comprehensive_audit_service::{
    ComprehensiveAuditService, AuditLogEntry, EventCategory, Severity, ActionResult,
};

// ============================================================================
// Request/Response DTOs
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AnalyzeSyncRequest {
    pub sync_log_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ResolveConflictsRequest {
    pub conflicts: Vec<ConflictInput>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConflictInput {
    pub atlas_inventory_id: Uuid,
    pub erp_item_id: String,
    pub conflict_type: String,
    pub atlas_value: serde_json::Value,
    pub erp_value: serde_json::Value,
    pub atlas_updated_at: Option<String>,
    pub erp_updated_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MappingDiscoveryStatusResponse {
    pub discovery_in_progress: bool,
    pub total_suggestions: usize,
    pub high_confidence_count: usize,
    pub medium_confidence_count: usize,
    pub low_confidence_count: usize,
    pub reviewed_count: usize,
    pub accepted_count: usize,
    pub rejected_count: usize,
}

#[derive(Debug, Serialize)]
pub struct MappingSuggestionResponse {
    pub id: Uuid,
    pub atlas_inventory_id: Option<Uuid>,
    pub erp_item_id: Option<String>,
    pub erp_item_name: Option<String>,
    pub confidence_score: Option<String>,
    pub matching_factors: Option<serde_json::Value>,
    pub ai_reasoning: Option<String>,
    pub status: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReviewMappingRequest {
    pub status: String,  // "accepted", "rejected", "skipped"
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /api/erp/connections/{connection_id}/auto-discover-mappings
/// Trigger AI auto-discovery of inventory mappings
pub async fn auto_discover_mappings(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(connection_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    tracing::info!(
        "User {} requesting AI mapping discovery for connection {}",
        claims.user_id,
        connection_id
    );

    // Verify connection ownership
    verify_connection_ownership(&pool, connection_id, claims.user_id).await?;

    // Initialize services
    let anthropic_api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| AppError::Internal(anyhow::anyhow!("ANTHROPIC_API_KEY not configured")))?;

    let ai_service = ErpAiAssistantService::new(pool.clone(), anthropic_api_key);
    let audit_service = ComprehensiveAuditService::new(pool.clone());

    // Start AI discovery
    let discovery_response = ai_service
        .auto_discover_mappings(connection_id, claims.user_id)
        .await?;

    // Audit log
    audit_service.log(AuditLogEntry {
        event_type: "erp_ai_mapping_discovery_triggered".to_string(),
        event_category: EventCategory::DataModification,
        severity: Severity::Info,
        actor_user_id: Some(claims.user_id),
        actor_type: "user".to_string(),
        actor_identifier: Some(claims.email.clone()),
        resource_type: Some("erp_connection".to_string()),
        resource_id: Some(connection_id.to_string()),
        resource_name: None,
        action: "erp_ai_mapping_discovery".to_string(),
        action_result: ActionResult::Success,
        event_data: serde_json::json!({
            "total_mappings": discovery_response.mappings.len(),
            "unmapped_atlas": discovery_response.unmapped_atlas_items.len(),
            "unmapped_erp": discovery_response.unmapped_erp_items.len(),
            "warnings": discovery_response.warnings,
        }),
        changes_summary: Some(format!(
            "AI discovered {} mapping suggestions with {} unmapped items",
            discovery_response.mappings.len(),
            discovery_response.unmapped_atlas_items.len()
        )),
        old_values: None,
        new_values: None,
        ip_address: None,
        user_agent: None,
        request_id: None,
        session_id: None,
        is_pii_access: false,
        compliance_tags: vec!["erp_integration".to_string(), "ai_operation".to_string()],
    }).await.ok();

    tracing::info!(
        "AI mapping discovery complete for connection {}: {} mappings",
        connection_id,
        discovery_response.mappings.len()
    );

    Ok((StatusCode::OK, Json(discovery_response)))
}

/// GET /api/erp/connections/{connection_id}/mapping-suggestions
/// Get AI-suggested mappings for review
pub async fn get_mapping_suggestions(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(connection_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    // Verify connection ownership
    verify_connection_ownership(&pool, connection_id, claims.user_id).await?;

    // Get suggestions from database
    let suggestions = sqlx::query_as!(
        MappingSuggestionResponse,
        r#"
        SELECT
            id,
            atlas_inventory_id,
            erp_item_id,
            erp_item_name,
            confidence_score::text,
            matching_factors,
            ai_reasoning,
            status,
            created_at::text
        FROM erp_ai_mapping_suggestions
        WHERE erp_connection_id = $1
        ORDER BY confidence_score DESC, created_at DESC
        "#,
        connection_id
    )
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, Json(suggestions)))
}

/// POST /api/erp/connections/{connection_id}/mapping-suggestions/{suggestion_id}/review
/// Review and accept/reject AI mapping suggestion
pub async fn review_mapping_suggestion(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path((connection_id, suggestion_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<ReviewMappingRequest>,
) -> Result<impl IntoResponse> {
    // Verify connection ownership
    verify_connection_ownership(&pool, connection_id, claims.user_id).await?;

    // Validate status
    if !["accepted", "rejected", "skipped"].contains(&request.status.as_str()) {
        return Err(AppError::BadRequest(
            "Invalid status. Must be 'accepted', 'rejected', or 'skipped'".to_string()
        ));
    }

    // Update suggestion
    sqlx::query!(
        r#"
        UPDATE erp_ai_mapping_suggestions
        SET status = $1,
            reviewed_by = $2,
            reviewed_at = NOW(),
            updated_at = NOW()
        WHERE id = $3 AND erp_connection_id = $4
        "#,
        request.status,
        claims.user_id,
        suggestion_id,
        connection_id
    )
    .execute(&pool)
    .await?;

    // If accepted, create actual mapping in erp_inventory_mappings table
    if request.status == "accepted" {
        let suggestion = sqlx::query!(
            r#"
            SELECT atlas_inventory_id, erp_item_id
            FROM erp_ai_mapping_suggestions
            WHERE id = $1
            "#,
            suggestion_id
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO erp_inventory_mappings (
                erp_connection_id,
                atlas_inventory_id,
                erp_item_id
            ) VALUES ($1, $2, $3)
            ON CONFLICT (erp_connection_id, atlas_inventory_id) DO UPDATE
            SET erp_item_id = $3, updated_at = NOW()
            "#,
            connection_id,
            suggestion.atlas_inventory_id,
            suggestion.erp_item_id
        )
        .execute(&pool)
        .await?;
    }

    // Audit log
    let audit_service = ComprehensiveAuditService::new(pool.clone());
    audit_service.log(AuditLogEntry {
        event_type: "erp_ai_mapping_reviewed".to_string(),
        event_category: EventCategory::DataModification,
        severity: Severity::Info,
        actor_user_id: Some(claims.user_id),
        actor_type: "user".to_string(),
        actor_identifier: Some(claims.email.clone()),
        resource_type: Some("erp_mapping_suggestion".to_string()),
        resource_id: Some(suggestion_id.to_string()),
        resource_name: None,
        action: format!("review_mapping_{}", request.status),
        action_result: ActionResult::Success,
        event_data: serde_json::json!({
            "connection_id": connection_id,
            "status": request.status,
        }),
        changes_summary: Some(format!("AI mapping suggestion {}", request.status)),
        old_values: None,
        new_values: None,
        ip_address: None,
        user_agent: None,
        request_id: None,
        session_id: None,
        is_pii_access: false,
        compliance_tags: vec!["erp_integration".to_string()],
    }).await.ok();

    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": format!("Mapping suggestion {} successfully", request.status),
        "status": request.status
    }))))
}

/// GET /api/erp/sync-logs/{sync_log_id}/ai-analysis
/// Get AI analysis of sync operation
pub async fn get_sync_analysis(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(sync_log_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    tracing::info!(
        "User {} requesting AI analysis for sync log {}",
        claims.user_id,
        sync_log_id
    );

    // Verify sync log ownership via connection
    verify_sync_log_ownership(&pool, sync_log_id, claims.user_id).await?;

    // Initialize services
    let anthropic_api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| AppError::Internal(anyhow::anyhow!("ANTHROPIC_API_KEY not configured")))?;

    let ai_service = ErpAiAssistantService::new(pool.clone(), anthropic_api_key);
    let audit_service = ComprehensiveAuditService::new(pool.clone());

    // Get AI analysis
    let insight = ai_service.analyze_sync_result(sync_log_id, claims.user_id).await?;

    // Audit log
    audit_service.log(AuditLogEntry {
        event_type: "erp_ai_sync_analysis_requested".to_string(),
        event_category: EventCategory::DataAccess,
        severity: Severity::Info,
        actor_user_id: Some(claims.user_id),
        actor_type: "user".to_string(),
        actor_identifier: Some(claims.email.clone()),
        resource_type: Some("erp_sync_log".to_string()),
        resource_id: Some(sync_log_id.to_string()),
        resource_name: None,
        action: "ai_sync_analysis".to_string(),
        action_result: ActionResult::Success,
        event_data: serde_json::json!({
            "insight_type": insight.insight_type,
            "severity": insight.severity,
        }),
        changes_summary: Some(format!("AI sync analysis: {}", insight.title)),
        old_values: None,
        new_values: None,
        ip_address: None,
        user_agent: None,
        request_id: None,
        session_id: None,
        is_pii_access: false,
        compliance_tags: vec!["erp_integration".to_string(), "ai_operation".to_string()],
    }).await.ok();

    Ok((StatusCode::OK, Json(insight)))
}

/// POST /api/erp/connections/{connection_id}/resolve-conflicts
/// Get AI suggestions for conflict resolution
pub async fn suggest_conflict_resolution(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(connection_id): Path<Uuid>,
    Json(request): Json<ResolveConflictsRequest>,
) -> Result<impl IntoResponse> {
    tracing::info!(
        "User {} requesting AI conflict resolution for {} conflicts",
        claims.user_id,
        request.conflicts.len()
    );

    // Verify connection ownership
    verify_connection_ownership(&pool, connection_id, claims.user_id).await?;

    // Validate conflicts
    if request.conflicts.is_empty() {
        return Err(AppError::BadRequest("No conflicts provided".to_string()));
    }

    if request.conflicts.len() > 100 {
        return Err(AppError::BadRequest(
            "Too many conflicts. Maximum 100 per request.".to_string()
        ));
    }

    // Initialize services
    let anthropic_api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| AppError::Internal(anyhow::anyhow!("ANTHROPIC_API_KEY not configured")))?;

    let ai_service = ErpAiAssistantService::new(pool.clone(), anthropic_api_key);
    let audit_service = ComprehensiveAuditService::new(pool.clone());

    // Convert to internal format
    let conflicts: Vec<_> = request.conflicts.iter().map(|c| {
        crate::services::erp::erp_ai_assistant_service::ConflictData {
            atlas_inventory_id: c.atlas_inventory_id,
            erp_item_id: c.erp_item_id.clone(),
            conflict_type: c.conflict_type.clone(),
            atlas_value: c.atlas_value.clone(),
            erp_value: c.erp_value.clone(),
            atlas_updated_at: c.atlas_updated_at.clone(),
            erp_updated_at: c.erp_updated_at.clone(),
        }
    }).collect();

    // Get AI suggestions
    let resolution_response = ai_service
        .suggest_conflict_resolution(connection_id, conflicts, claims.user_id)
        .await?;

    // Audit log
    audit_service.log(AuditLogEntry {
        event_type: "erp_ai_conflict_resolution_requested".to_string(),
        event_category: EventCategory::DataAccess,
        severity: Severity::Info,
        actor_user_id: Some(claims.user_id),
        actor_type: "user".to_string(),
        actor_identifier: Some(claims.email.clone()),
        resource_type: Some("erp_connection".to_string()),
        resource_id: Some(connection_id.to_string()),
        resource_name: None,
        action: "ai_conflict_resolution".to_string(),
        action_result: ActionResult::Success,
        event_data: serde_json::json!({
            "conflict_count": request.conflicts.len(),
            "resolutions_suggested": resolution_response.resolutions.len(),
        }),
        changes_summary: Some(format!(
            "AI analyzed {} conflicts and provided resolution suggestions",
            request.conflicts.len()
        )),
        old_values: None,
        new_values: None,
        ip_address: None,
        user_agent: None,
        request_id: None,
        session_id: None,
        is_pii_access: false,
        compliance_tags: vec!["erp_integration".to_string(), "ai_operation".to_string()],
    }).await.ok();

    Ok((StatusCode::OK, Json(resolution_response)))
}

/// GET /api/erp/connections/{connection_id}/mapping-status
/// Get mapping discovery status and statistics
pub async fn get_mapping_status(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(connection_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    // Verify connection ownership
    verify_connection_ownership(&pool, connection_id, claims.user_id).await?;

    // Get statistics
    let stats = sqlx::query!(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE status = 'suggested') as total_suggestions,
            COUNT(*) FILTER (WHERE status = 'suggested' AND confidence_score >= 0.90) as high_confidence,
            COUNT(*) FILTER (WHERE status = 'suggested' AND confidence_score >= 0.75 AND confidence_score < 0.90) as medium_confidence,
            COUNT(*) FILTER (WHERE status = 'suggested' AND confidence_score < 0.75) as low_confidence,
            COUNT(*) FILTER (WHERE status != 'suggested') as reviewed,
            COUNT(*) FILTER (WHERE status = 'accepted') as accepted,
            COUNT(*) FILTER (WHERE status = 'rejected') as rejected
        FROM erp_ai_mapping_suggestions
        WHERE erp_connection_id = $1
        "#,
        connection_id
    )
    .fetch_one(&pool)
    .await?;

    let response = MappingDiscoveryStatusResponse {
        discovery_in_progress: false,  // TODO: Track actual discovery jobs
        total_suggestions: stats.total_suggestions.unwrap_or(0) as usize,
        high_confidence_count: stats.high_confidence.unwrap_or(0) as usize,
        medium_confidence_count: stats.medium_confidence.unwrap_or(0) as usize,
        low_confidence_count: stats.low_confidence.unwrap_or(0) as usize,
        reviewed_count: stats.reviewed.unwrap_or(0) as usize,
        accepted_count: stats.accepted.unwrap_or(0) as usize,
        rejected_count: stats.rejected.unwrap_or(0) as usize,
    };

    Ok((StatusCode::OK, Json(response)))
}

// ============================================================================
// Helper Functions
// ============================================================================

async fn verify_connection_ownership(pool: &PgPool, connection_id: Uuid, user_id: Uuid) -> Result<()> {
    let exists = sqlx::query!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM erp_connections
            WHERE id = $1 AND user_id = $2
        ) as "exists!"
        "#,
        connection_id,
        user_id
    )
    .fetch_one(pool)
    .await?
    .exists;

    if !exists {
        return Err(AppError::NotFound("ERP connection not found".to_string()));
    }

    Ok(())
}

async fn verify_sync_log_ownership(pool: &PgPool, sync_log_id: Uuid, user_id: Uuid) -> Result<()> {
    let exists = sqlx::query!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM erp_sync_logs esl
            JOIN erp_connections ec ON esl.erp_connection_id = ec.id
            WHERE esl.id = $1 AND ec.user_id = $2
        ) as "exists!"
        "#,
        sync_log_id,
        user_id
    )
    .fetch_one(pool)
    .await?
    .exists;

    if !exists {
        return Err(AppError::NotFound("Sync log not found".to_string()));
    }

    Ok(())
}
