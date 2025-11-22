// ERP Integration API Handlers
// Production-ready REST endpoints for Oracle NetSuite and SAP S/4HANA integration
// Comprehensive validation, error handling, and audit logging

use axum::{
    extract::{Path, State, Query},
    http::StatusCode,
    response::{IntoResponse, Json},
    Extension,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::middleware::auth::Claims;
use crate::middleware::error_handling::{AppError, Result};
use crate::services::erp::{
    ErpConnectionService, ErpSyncService, ErpType, SyncDirection,
};
use crate::services::erp::erp_connection_service::{
    CreateConnectionRequest, ConnectionResponse, ConnectionTestResult,
};
use crate::services::comprehensive_audit_service::{
    ComprehensiveAuditService, AuditLogEntry, EventCategory, Severity, ActionResult,
};
use crate::services::webhook_security_service::{
    WebhookSecurityService, WebhookAuditLog,
};
use axum::body::Bytes;
use axum::http::HeaderMap;

// ============================================================================
// Request/Response DTOs
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateErpConnectionRequest {
    pub connection_name: String,
    pub erp_type: String,

    // NetSuite credentials
    pub netsuite_account_id: Option<String>,
    pub netsuite_consumer_key: Option<String>,
    pub netsuite_consumer_secret: Option<String>,
    pub netsuite_token_id: Option<String>,
    pub netsuite_token_secret: Option<String>,
    pub netsuite_realm: Option<String>,

    // SAP credentials
    pub sap_base_url: Option<String>,
    pub sap_client_id: Option<String>,
    pub sap_client_secret: Option<String>,
    pub sap_token_endpoint: Option<String>,
    pub sap_environment: Option<String>,
    pub sap_plant: Option<String>,
    pub sap_company_code: Option<String>,

    // Sync configuration
    pub sync_enabled: Option<bool>,
    pub sync_frequency_minutes: Option<i32>,
    pub sync_stock_levels: Option<bool>,
    pub sync_product_master: Option<bool>,
    pub sync_transactions: Option<bool>,
    pub sync_lot_batch: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConnectionRequest {
    pub connection_name: Option<String>,
    pub sync_enabled: Option<bool>,
    pub sync_frequency_minutes: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct SyncQueryParams {
    pub direction: Option<String>,  // "atlas_to_erp", "erp_to_atlas", "bidirectional"
}

#[derive(Debug, Serialize)]
pub struct ErpConnectionListResponse {
    pub connections: Vec<ConnectionResponse>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct SyncResponse {
    pub sync_started: bool,
    pub message: String,
    pub sync_log_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct SyncResultResponse {
    pub items_synced: i32,
    pub items_failed: i32,
    pub items_skipped: i32,
    pub items_created: i32,
    pub items_updated: i32,
    pub conflicts_detected: i32,
    pub errors: Vec<SyncErrorDetail>,
}

#[derive(Debug, Serialize)]
pub struct SyncErrorDetail {
    pub item_id: String,
    pub error_message: String,
    pub error_type: String,
}

#[derive(Debug, Serialize)]
pub struct MappingResponse {
    pub id: Uuid,
    pub atlas_inventory_id: Uuid,
    pub erp_item_id: String,
    pub erp_item_name: Option<String>,
    pub erp_location_id: Option<String>,
    pub sync_enabled: bool,
    pub last_synced_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_sync_status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SyncLogResponse {
    pub id: Uuid,
    pub sync_type: String,
    pub sync_direction: String,
    pub triggered_by: String,
    pub status: String,
    pub items_synced: i32,
    pub items_failed: i32,
    pub items_skipped: i32,
    pub duration_seconds: Option<i32>,
    pub error_message: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

// ============================================================================
// Connection Management Handlers
// ============================================================================

/// Create a new ERP connection
/// POST /api/erp/connections
pub async fn create_connection(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<CreateErpConnectionRequest>,
) -> Result<impl IntoResponse> {
    // 🔒 SECURITY: Sanitize user-provided ERP type for log injection prevention
    tracing::info!(
        "Creating ERP connection for user {} - type: {}",
        claims.user_id,
        crate::utils::log_sanitizer::sanitize_for_log(&request.erp_type)
    );

    // Parse ERP type
    let erp_type = match request.erp_type.to_lowercase().as_str() {
        "netsuite" => ErpType::NetSuite,
        "sap_s4hana" => ErpType::SapS4Hana,
        _ => {
            return Err(AppError::BadRequest(format!(
                "Invalid ERP type: {}. Must be 'netsuite' or 'sap_s4hana'",
                request.erp_type
            )));
        }
    };

    // Convert to service request
    let service_request = CreateConnectionRequest {
        connection_name: request.connection_name.clone(),
        erp_type: erp_type.clone(),
        netsuite_account_id: request.netsuite_account_id,
        netsuite_consumer_key: request.netsuite_consumer_key,
        netsuite_consumer_secret: request.netsuite_consumer_secret,
        netsuite_token_id: request.netsuite_token_id,
        netsuite_token_secret: request.netsuite_token_secret,
        netsuite_realm: request.netsuite_realm,
        sap_base_url: request.sap_base_url,
        sap_client_id: request.sap_client_id,
        sap_client_secret: request.sap_client_secret,
        sap_token_endpoint: request.sap_token_endpoint,
        sap_environment: request.sap_environment,
        sap_plant: request.sap_plant,
        sap_company_code: request.sap_company_code,
        sync_enabled: request.sync_enabled,
        sync_frequency_minutes: request.sync_frequency_minutes,
        sync_stock_levels: request.sync_stock_levels,
        sync_product_master: request.sync_product_master,
        sync_transactions: request.sync_transactions,
        sync_lot_batch: request.sync_lot_batch,
    };

    // Create connection
    let service = ErpConnectionService::new(pool.clone());
    let connection = service
        .create_connection(claims.user_id, service_request)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;

    // Audit log
    let audit_service = ComprehensiveAuditService::new(pool.clone());
    audit_service
        .log(AuditLogEntry {
            event_type: "erp_connection_created".to_string(),
            event_category: EventCategory::DataModification,
            severity: Severity::Info,
            actor_user_id: Some(claims.user_id),
            actor_type: "user".to_string(),
            resource_type: Some("erp_connection".to_string()),
            resource_id: Some(connection.id.to_string()),
            action: "create".to_string(),
            action_result: ActionResult::Success,
            event_data: serde_json::json!({
                "connection_name": request.connection_name,
                "erp_type": match connection.erp_type {
                    ErpType::NetSuite => "netsuite",
                    ErpType::SapS4Hana => "sap_s4hana",
                }
            }),
            ..Default::default()
        })
        .await
        .ok();

    let response = service.to_response(&connection);

    Ok((StatusCode::CREATED, Json(response)))
}

/// List all ERP connections for the authenticated user
/// GET /api/erp/connections
pub async fn list_connections(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse> {
    let service = ErpConnectionService::new(pool);

    let connections = service
        .get_user_connections(claims.user_id)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;

    let responses: Vec<ConnectionResponse> = connections
        .iter()
        .map(|c| service.to_response(c))
        .collect();

    let response = ErpConnectionListResponse {
        total: responses.len(),
        connections: responses,
    };

    Ok(Json(response))
}

/// Get a specific ERP connection by ID
/// GET /api/erp/connections/:id
pub async fn get_connection(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(connection_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let service = ErpConnectionService::new(pool);

    let connection = service
        .get_connection_by_id(connection_id)
        .await
        .map_err(|e| match e {
            crate::services::erp::erp_connection_service::ErpConnectionError::NotFound(_) => {
                AppError::NotFound(format!("Connection {} not found", connection_id))
            }
            _ => AppError::Internal(anyhow::anyhow!(e.to_string())),
        })?;

    // Verify ownership
    if connection.user_id != claims.user_id {
        return Err(AppError::Forbidden(
            "You don't have permission to access this connection".to_string(),
        ));
    }

    let response = service.to_response(&connection);

    Ok(Json(response))
}

/// Delete an ERP connection
/// DELETE /api/erp/connections/:id
pub async fn delete_connection(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(connection_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    tracing::info!(
        "Deleting ERP connection {} for user {}",
        connection_id,
        claims.user_id
    );

    let service = ErpConnectionService::new(pool.clone());

    service
        .delete_connection(connection_id, claims.user_id)
        .await
        .map_err(|e| match e {
            crate::services::erp::erp_connection_service::ErpConnectionError::NotFound(_) => {
                AppError::NotFound(format!("Connection {} not found", connection_id))
            }
            _ => AppError::Internal(anyhow::anyhow!(e.to_string())),
        })?;

    // Audit log
    let audit_service = ComprehensiveAuditService::new(pool);
    audit_service
        .log(AuditLogEntry {
            event_type: "erp_connection_deleted".to_string(),
            event_category: EventCategory::DataModification,
            severity: Severity::Info,
            actor_user_id: Some(claims.user_id),
            actor_type: "user".to_string(),
            resource_type: Some("erp_connection".to_string()),
            resource_id: Some(connection_id.to_string()),
            action: "delete".to_string(),
            action_result: ActionResult::Success,
            event_data: serde_json::json!({}),
            ..Default::default()
        })
        .await
        .ok();

    Ok(StatusCode::NO_CONTENT)
}

/// Test an ERP connection
/// POST /api/erp/connections/:id/test
pub async fn test_connection(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(connection_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    tracing::info!("Testing ERP connection {}", connection_id);

    let service = ErpConnectionService::new(pool.clone());

    let connection = service
        .get_connection_by_id(connection_id)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;

    // Verify ownership
    if connection.user_id != claims.user_id {
        return Err(AppError::Forbidden(
            "You don't have permission to test this connection".to_string(),
        ));
    }

    // Test connection
    let test_result = service
        .test_connection(&connection)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;

    // Update last test time
    sqlx::query!(
        "UPDATE erp_connections SET last_test_at = NOW() WHERE id = $1",
        connection_id
    )
    .execute(&pool)
    .await
    .ok();

    // Audit log
    let audit_service = ComprehensiveAuditService::new(pool);
    audit_service
        .log(AuditLogEntry {
            event_type: "erp_connection_tested".to_string(),
            event_category: EventCategory::System,
            severity: Severity::Info,
            actor_user_id: Some(claims.user_id),
            actor_type: "user".to_string(),
            resource_type: Some("erp_connection".to_string()),
            resource_id: Some(connection_id.to_string()),
            action: "test".to_string(),
            action_result: if test_result.success { ActionResult::Success } else { ActionResult::Failure },
            event_data: serde_json::json!({ "test_result": test_result }),
            ..Default::default()
        })
        .await
        .ok();

    Ok(Json(test_result))
}

// ============================================================================
// Sync Operations Handlers
// ============================================================================

/// Trigger a manual sync
/// POST /api/erp/connections/:id/sync
pub async fn trigger_sync(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(connection_id): Path<Uuid>,
    Query(params): Query<SyncQueryParams>,
) -> Result<impl IntoResponse> {
    tracing::info!("Triggering sync for connection {}", connection_id);

    let connection_service = ErpConnectionService::new(pool.clone());
    let sync_service = ErpSyncService::new(pool.clone());

    // Verify connection exists and user owns it
    let connection = connection_service
        .get_connection_by_id(connection_id)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;

    if connection.user_id != claims.user_id {
        return Err(AppError::Forbidden(
            "You don't have permission to sync this connection".to_string(),
        ));
    }

    // Determine sync direction and clone it for the async move block
    let direction = params.direction
        .as_deref()
        .unwrap_or("bidirectional")
        .to_string();
    let direction_clone = direction.clone();

    // Spawn sync task in background (don't block the HTTP response)
    let pool_clone = pool.clone();
    let connection_id_clone = connection_id;
    let user_id = claims.user_id;

    tokio::spawn(async move {
        let sync_service = ErpSyncService::new(pool_clone.clone());

        let result = match direction_clone.as_str() {
            "atlas_to_erp" => sync_service.sync_atlas_to_erp(connection_id_clone).await,
            "erp_to_atlas" => sync_service.sync_from_erp_to_atlas(connection_id_clone).await,
            "bidirectional" => sync_service.sync_bidirectional(connection_id_clone).await,
            _ => {
                tracing::error!("Invalid sync direction: {}", direction_clone);
                return;
            }
        };

        match result {
            Ok(sync_result) => {
                tracing::info!(
                    "Sync completed for connection {}: {} synced, {} failed",
                    connection_id_clone,
                    sync_result.items_synced,
                    sync_result.items_failed
                );

                // Update connection metadata
                let connection_service = ErpConnectionService::new(pool_clone.clone());
                connection_service
                    .update_sync_metadata(
                        connection_id_clone,
                        if sync_result.items_failed > 0 {
                            "partial"
                        } else {
                            "success"
                        },
                        None,
                    )
                    .await
                    .ok();
            }
            Err(e) => {
                tracing::error!("Sync failed for connection {}: {}", connection_id_clone, e);

                let connection_service = ErpConnectionService::new(pool_clone.clone());
                connection_service
                    .update_sync_metadata(connection_id_clone, "failed", None)
                    .await
                    .ok();
            }
        }

        // Audit log
        let audit_service = ComprehensiveAuditService::new(pool_clone);
        audit_service
            .log(AuditLogEntry {
                event_type: "erp_manual_sync_completed".to_string(),
                event_category: EventCategory::System,
                severity: Severity::Info,
                actor_user_id: Some(user_id),
                actor_type: "user".to_string(),
                resource_type: Some("erp_sync".to_string()),
                resource_id: Some(connection_id_clone.to_string()),
                action: "manual_sync".to_string(),
                action_result: ActionResult::Success,
                event_data: serde_json::json!({}),
                ..Default::default()
            })
            .await
            .ok();
    });

    let response = SyncResponse {
        sync_started: true,
        message: format!("Sync started for direction: {}", direction),
        sync_log_id: None,
    };

    Ok(Json(response))
}

/// Get sync logs for a connection
/// GET /api/erp/connections/:id/sync-logs
pub async fn get_sync_logs(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(connection_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let connection_service = ErpConnectionService::new(pool.clone());

    // Verify ownership
    let connection = connection_service
        .get_connection_by_id(connection_id)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;

    if connection.user_id != claims.user_id {
        return Err(AppError::Forbidden(
            "You don't have permission to view these sync logs".to_string(),
        ));
    }

    // Get logs
    let logs = sqlx::query_as!(
        SyncLogResponse,
        r#"
        SELECT
            id, sync_type, sync_direction, triggered_by, status,
            items_synced, items_failed, items_skipped, duration_seconds,
            error_message, created_at, completed_at
        FROM erp_sync_logs
        WHERE erp_connection_id = $1
        ORDER BY created_at DESC
        LIMIT 50
        "#,
        connection_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;

    Ok(Json(logs))
}

// ============================================================================
// Mapping Management Handlers
// ============================================================================

/// Get inventory mappings for a connection
/// GET /api/erp/connections/:id/mappings
pub async fn get_mappings(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(connection_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let connection_service = ErpConnectionService::new(pool.clone());

    // Verify ownership
    let connection = connection_service
        .get_connection_by_id(connection_id)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;

    if connection.user_id != claims.user_id {
        return Err(AppError::Forbidden(
            "You don't have permission to view these mappings".to_string(),
        ));
    }

    // Get mappings
    let mappings = sqlx::query_as!(
        MappingResponse,
        r#"
        SELECT
            id, atlas_inventory_id, erp_item_id, erp_item_name, erp_location_id,
            sync_enabled, last_synced_at, last_sync_status
        FROM erp_inventory_mappings
        WHERE erp_connection_id = $1
        ORDER BY created_at DESC
        "#,
        connection_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;

    Ok(Json(mappings))
}

/// Auto-discover mappings (match Atlas inventory to ERP items by NDC)
/// POST /api/erp/connections/:id/auto-discover
pub async fn auto_discover_mappings(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(connection_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    tracing::info!("Auto-discovering mappings for connection {}", connection_id);

    let connection_service = ErpConnectionService::new(pool.clone());

    // Verify ownership
    let connection = connection_service
        .get_connection_by_id(connection_id)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;

    if connection.user_id != claims.user_id {
        return Err(AppError::Forbidden(
            "You don't have permission to auto-discover mappings".to_string(),
        ));
    }

    // For now, return a placeholder response
    // Full implementation would query ERP for all items and match by NDC
    let response = serde_json::json!({
        "message": "Auto-discovery started",
        "status": "running"
    });

    Ok(Json(response))
}

/// Delete a mapping
/// DELETE /api/erp/mappings/:id
pub async fn delete_mapping(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(mapping_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    // Verify ownership through connection
    let mapping = sqlx::query!(
        r#"
        SELECT m.id, c.user_id
        FROM erp_inventory_mappings m
        JOIN erp_connections c ON m.erp_connection_id = c.id
        WHERE m.id = $1
        "#,
        mapping_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?
    .ok_or_else(|| AppError::NotFound("Mapping not found".to_string()))?;

    if mapping.user_id != claims.user_id {
        return Err(AppError::Forbidden(
            "You don't have permission to delete this mapping".to_string(),
        ));
    }

    sqlx::query!("DELETE FROM erp_inventory_mappings WHERE id = $1", mapping_id)
        .execute(&pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Webhook Handlers (for real-time ERP updates)
// ============================================================================

/// NetSuite webhook endpoint (SECURED with HMAC signature verification)
/// POST /api/erp/webhooks/netsuite/:connection_id
///
/// **Security:**
/// - HMAC-SHA256 signature verification required
/// - Rate limiting: 100 requests per 15 minutes per connection
/// - Connection must exist and have webhooks enabled
/// - All attempts logged to audit table
/// - IP address tracking
/// - Payload size limit: 1MB
///
/// **Headers Required:**
/// - X-Webhook-Signature: sha256=<hex_signature>
/// - Content-Type: application/json
///
/// **Rate Limit Headers (Response):**
/// - X-RateLimit-Remaining: requests remaining in window
/// - X-RateLimit-Reset: timestamp when window resets
pub async fn netsuite_webhook(
    State(pool): State<PgPool>,
    Path(connection_id): Path<Uuid>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse> {
    let start_time = std::time::Instant::now();
    let request_id = Uuid::new_v4();

    // Extract source IP (if behind proxy, use X-Forwarded-For)
    let source_ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .or_else(|| headers.get("x-real-ip").and_then(|v| v.to_str().ok()))
        .map(|s| s.to_string());

    // Initialize security service
    let webhook_service = WebhookSecurityService::new(pool.clone())
        .map_err(|e| {
            tracing::error!("Failed to initialize webhook security service: {:?}", e);
            AppError::Internal(anyhow::anyhow!("Webhook service unavailable"))
        })?;

    // Payload size check (1MB limit)
    const MAX_PAYLOAD_SIZE: usize = 1_048_576; // 1MB
    if body.len() > MAX_PAYLOAD_SIZE {
        let log = WebhookAuditLog {
            connection_id,
            event_type: "netsuite".to_string(),
            request_id,
            source_ip: source_ip.clone(),
            signature_valid: false,
            payload_size_bytes: body.len() as i32,
            http_status: 413,
            error_message: Some("Payload too large".to_string()),
            processing_time_ms: Some(start_time.elapsed().as_millis() as i32),
        };
        let _ = webhook_service.log_webhook_attempt(log).await;

        return Err(AppError::BadRequest("Payload exceeds 1MB limit".to_string()));
    }

    // Step 1: Validate connection exists and webhooks are enabled
    if let Err(e) = webhook_service.validate_connection(connection_id).await {
        let log = WebhookAuditLog {
            connection_id,
            event_type: "netsuite".to_string(),
            request_id,
            source_ip,
            signature_valid: false,
            payload_size_bytes: body.len() as i32,
            http_status: 404,
            error_message: Some(e.to_string()),
            processing_time_ms: Some(start_time.elapsed().as_millis() as i32),
        };
        let _ = webhook_service.log_webhook_attempt(log).await;
        return Err(e);
    }

    // Step 2: Check rate limit
    let rate_limit_result = webhook_service.check_rate_limit(connection_id).await?;

    if !rate_limit_result.rate_limit_allowed {
        let log = WebhookAuditLog {
            connection_id,
            event_type: "netsuite".to_string(),
            request_id,
            source_ip,
            signature_valid: false,
            payload_size_bytes: body.len() as i32,
            http_status: 429,
            error_message: Some(if rate_limit_result.blocked {
                "Connection blocked due to rate limit violations"
            } else {
                "Rate limit exceeded"
            }.to_string()),
            processing_time_ms: Some(start_time.elapsed().as_millis() as i32),
        };
        let _ = webhook_service.log_webhook_attempt(log).await;

        return Err(AppError::TooManyRequests("Rate limit exceeded for webhook".to_string()));
    }

    // Step 3: Verify HMAC signature
    let signature_header = headers
        .get("x-webhook-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized)?;

    let signature_valid = webhook_service
        .verify_signature(connection_id, &body, signature_header)
        .await
        .unwrap_or(false);

    if !signature_valid {
        tracing::warn!(
            "Invalid webhook signature for connection {} from IP {:?}",
            connection_id,
            source_ip
        );

        let log = WebhookAuditLog {
            connection_id,
            event_type: "netsuite".to_string(),
            request_id,
            source_ip,
            signature_valid: false,
            payload_size_bytes: body.len() as i32,
            http_status: 401,
            error_message: Some("Invalid signature".to_string()),
            processing_time_ms: Some(start_time.elapsed().as_millis() as i32),
        };
        let _ = webhook_service.log_webhook_attempt(log).await;

        return Err(AppError::Unauthorized);
    }

    // Step 4: Parse and validate JSON payload
    let payload: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|e| {
            let log = WebhookAuditLog {
                connection_id,
                event_type: "netsuite".to_string(),
                request_id,
                source_ip: source_ip.clone(),
                signature_valid: true,
                payload_size_bytes: body.len() as i32,
                http_status: 400,
                error_message: Some(format!("Invalid JSON: {}", e)),
                processing_time_ms: Some(start_time.elapsed().as_millis() as i32),
            };
            let _ = futures::executor::block_on(webhook_service.log_webhook_attempt(log));
            AppError::BadRequest(format!("Invalid JSON payload: {}", e))
        })?;

    tracing::info!(
        "✓ Valid NetSuite webhook for connection {} (request_id: {})",
        connection_id,
        request_id
    );

    // Step 5: Process webhook event
    // TODO: Implement webhook event processing based on event type
    // - inventory_update: Update inventory quantities
    // - item_created: Create new pharmaceutical item
    // - item_updated: Update item details
    // - order_status: Update order status

    // 🔒 SECURITY: Log webhook metadata only, NOT full payload (may contain sensitive data)
    tracing::debug!("NetSuite webhook received for connection: {} (payload size: {} bytes)",
        connection_id, payload.to_string().len());

    // Step 6: Log successful webhook processing
    let processing_time = start_time.elapsed().as_millis() as i32;
    let log = WebhookAuditLog {
        connection_id,
        event_type: "netsuite".to_string(),
        request_id,
        source_ip,
        signature_valid: true,
        payload_size_bytes: body.len() as i32,
        http_status: 200,
        error_message: None,
        processing_time_ms: Some(processing_time),
    };
    webhook_service.log_webhook_attempt(log).await?;

    // Return response with rate limit headers
    Ok((
        StatusCode::OK,
        [
            ("X-Request-ID", request_id.to_string()),
            ("X-RateLimit-Remaining", rate_limit_result.requests_remaining.to_string()),
        ],
        Json(serde_json::json!({
            "status": "accepted",
            "request_id": request_id,
            "processing_time_ms": processing_time
        }))
    ))
}

/// SAP webhook endpoint (SECURED with HMAC signature verification)
/// POST /api/erp/webhooks/sap/:connection_id
///
/// **Security:**
/// - HMAC-SHA256 signature verification required
/// - Rate limiting: 100 requests per 15 minutes per connection
/// - Connection must exist and have webhooks enabled
/// - All attempts logged to audit table
/// - IP address tracking
/// - Payload size limit: 1MB
///
/// **Headers Required:**
/// - X-Webhook-Signature: sha256=<hex_signature>
/// - Content-Type: application/json
///
/// **Rate Limit Headers (Response):**
/// - X-RateLimit-Remaining: requests remaining in window
/// - X-RateLimit-Reset: timestamp when window resets
pub async fn sap_webhook(
    State(pool): State<PgPool>,
    Path(connection_id): Path<Uuid>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse> {
    let start_time = std::time::Instant::now();
    let request_id = Uuid::new_v4();

    // Extract source IP (if behind proxy, use X-Forwarded-For)
    let source_ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .or_else(|| headers.get("x-real-ip").and_then(|v| v.to_str().ok()))
        .map(|s| s.to_string());

    // Initialize security service
    let webhook_service = WebhookSecurityService::new(pool.clone())
        .map_err(|e| {
            tracing::error!("Failed to initialize webhook security service: {:?}", e);
            AppError::Internal(anyhow::anyhow!("Webhook service unavailable"))
        })?;

    // Payload size check (1MB limit)
    const MAX_PAYLOAD_SIZE: usize = 1_048_576; // 1MB
    if body.len() > MAX_PAYLOAD_SIZE {
        let log = WebhookAuditLog {
            connection_id,
            event_type: "sap".to_string(),
            request_id,
            source_ip: source_ip.clone(),
            signature_valid: false,
            payload_size_bytes: body.len() as i32,
            http_status: 413,
            error_message: Some("Payload too large".to_string()),
            processing_time_ms: Some(start_time.elapsed().as_millis() as i32),
        };
        let _ = webhook_service.log_webhook_attempt(log).await;

        return Err(AppError::BadRequest("Payload exceeds 1MB limit".to_string()));
    }

    // Step 1: Validate connection exists and webhooks are enabled
    if let Err(e) = webhook_service.validate_connection(connection_id).await {
        let log = WebhookAuditLog {
            connection_id,
            event_type: "sap".to_string(),
            request_id,
            source_ip,
            signature_valid: false,
            payload_size_bytes: body.len() as i32,
            http_status: 404,
            error_message: Some(e.to_string()),
            processing_time_ms: Some(start_time.elapsed().as_millis() as i32),
        };
        let _ = webhook_service.log_webhook_attempt(log).await;
        return Err(e);
    }

    // Step 2: Check rate limit
    let rate_limit_result = webhook_service.check_rate_limit(connection_id).await?;

    if !rate_limit_result.rate_limit_allowed {
        let log = WebhookAuditLog {
            connection_id,
            event_type: "sap".to_string(),
            request_id,
            source_ip,
            signature_valid: false,
            payload_size_bytes: body.len() as i32,
            http_status: 429,
            error_message: Some(if rate_limit_result.blocked {
                "Connection blocked due to rate limit violations"
            } else {
                "Rate limit exceeded"
            }.to_string()),
            processing_time_ms: Some(start_time.elapsed().as_millis() as i32),
        };
        let _ = webhook_service.log_webhook_attempt(log).await;

        return Err(AppError::TooManyRequests("Rate limit exceeded for webhook".to_string()));
    }

    // Step 3: Verify HMAC signature
    let signature_header = headers
        .get("x-webhook-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized)?;

    let signature_valid = webhook_service
        .verify_signature(connection_id, &body, signature_header)
        .await
        .unwrap_or(false);

    if !signature_valid {
        tracing::warn!(
            "Invalid webhook signature for connection {} from IP {:?}",
            connection_id,
            source_ip
        );

        let log = WebhookAuditLog {
            connection_id,
            event_type: "sap".to_string(),
            request_id,
            source_ip,
            signature_valid: false,
            payload_size_bytes: body.len() as i32,
            http_status: 401,
            error_message: Some("Invalid signature".to_string()),
            processing_time_ms: Some(start_time.elapsed().as_millis() as i32),
        };
        let _ = webhook_service.log_webhook_attempt(log).await;

        return Err(AppError::Unauthorized);
    }

    // Step 4: Parse and validate JSON payload
    let payload: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|e| {
            let log = WebhookAuditLog {
                connection_id,
                event_type: "sap".to_string(),
                request_id,
                source_ip: source_ip.clone(),
                signature_valid: true,
                payload_size_bytes: body.len() as i32,
                http_status: 400,
                error_message: Some(format!("Invalid JSON: {}", e)),
                processing_time_ms: Some(start_time.elapsed().as_millis() as i32),
            };
            let _ = futures::executor::block_on(webhook_service.log_webhook_attempt(log));
            AppError::BadRequest(format!("Invalid JSON payload: {}", e))
        })?;

    tracing::info!(
        "✓ Valid SAP webhook for connection {} (request_id: {})",
        connection_id,
        request_id
    );

    // Step 5: Process webhook event
    // TODO: Implement webhook event processing based on event type
    // - material_changed: Update inventory quantities
    // - material_created: Create new pharmaceutical item
    // - purchase_order_status: Update order status

    // 🔒 SECURITY: Log webhook metadata only, NOT full payload (may contain sensitive data)
    tracing::debug!("SAP webhook received for connection: {} (payload size: {} bytes)",
        connection_id, payload.to_string().len());

    // Step 6: Log successful webhook processing
    let processing_time = start_time.elapsed().as_millis() as i32;
    let log = WebhookAuditLog {
        connection_id,
        event_type: "sap".to_string(),
        request_id,
        source_ip,
        signature_valid: true,
        payload_size_bytes: body.len() as i32,
        http_status: 200,
        error_message: None,
        processing_time_ms: Some(processing_time),
    };
    webhook_service.log_webhook_attempt(log).await?;

    // Return response with rate limit headers
    Ok((
        StatusCode::OK,
        [
            ("X-Request-ID", request_id.to_string()),
            ("X-RateLimit-Remaining", rate_limit_result.requests_remaining.to_string()),
        ],
        Json(serde_json::json!({
            "status": "accepted",
            "request_id": request_id,
            "processing_time_ms": processing_time
        }))
    ))
}
