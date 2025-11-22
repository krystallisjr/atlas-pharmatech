/// REST API handlers for AI-powered inventory import system

use axum::{
    extract::{State, Multipart, Path, Query},
    Extension,
    Json,
};
use uuid::Uuid;
use crate::{
    config::AppConfig,
    middleware::{error_handling::Result, Claims},
    models::ai_import::*,
    services::{
        AiImportService,
        FileParserService,
        BatchImportProcessor,
        AuditService,
        ApiQuotaService,
    },
    utils::encrypted_file_storage::EncryptedFileStorage,
};

/// POST /api/ai-import/upload
/// Upload and analyze a file for import
pub async fn upload_and_analyze(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    mut multipart: Multipart,
) -> Result<Json<ImportSessionResponse>> {
    tracing::info!("AI import upload requested by user: {}", claims.user_id);

    // Get Claude API key from environment
    let claude_api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| crate::middleware::error_handling::AppError::Internal(
            anyhow::anyhow!("ANTHROPIC_API_KEY not configured")
        ))?;

    let ai_service = AiImportService::new(config.database_pool.clone(), claude_api_key);

    // Parse multipart form data
    let mut file_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        crate::middleware::error_handling::AppError::InvalidInput(format!("Invalid multipart data: {}", e))
    })? {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "file" {
            filename = field.file_name().map(|s| s.to_string());
            file_data = Some(field.bytes().await.map_err(|e| {
                crate::middleware::error_handling::AppError::InvalidInput(format!("Failed to read file: {}", e))
            })?.to_vec());
        }
    }

    let file_data = file_data.ok_or_else(|| {
        crate::middleware::error_handling::AppError::InvalidInput("No file provided".to_string())
    })?;

    let filename = filename.ok_or_else(|| {
        crate::middleware::error_handling::AppError::InvalidInput("No filename provided".to_string())
    })?;

    // Validate file size (max 50MB)
    const MAX_FILE_SIZE: usize = 50 * 1024 * 1024;
    if file_data.len() > MAX_FILE_SIZE {
        return Err(crate::middleware::error_handling::AppError::InvalidInput(
            format!("File too large. Maximum size is {}MB", MAX_FILE_SIZE / 1024 / 1024)
        ));
    }

    // 🔒 SECURITY: Sanitize filename for log injection prevention
    tracing::info!("Processing file upload: {} ({} bytes)",
        crate::utils::log_sanitizer::sanitize_for_log(&filename),
        file_data.len());

    // Create import session
    let session_id = ai_service.create_session(
        claims.user_id,
        filename.clone(),
        Some("web_upload".to_string()),
    ).await?;

    // 🔒 PRODUCTION SECURITY: Save file encrypted to disk using AES-256-GCM
    let file_storage = EncryptedFileStorage::new(
        &config.file_storage_path,
        &config.encryption_key
    )?;
    let (file_path, file_hash) = file_storage.save_encrypted_file(session_id, &filename, &file_data)?;

    // 🔒 SECURITY: Sanitize file path for log injection prevention
    tracing::info!("File saved to: {}",
        crate::utils::log_sanitizer::sanitize_for_log(&file_path));

    // Update session with file path and hash
    sqlx::query(
        "UPDATE ai_import_sessions SET file_path = $1, file_hash = $2 WHERE id = $3"
    )
    .bind(&file_path)
    .bind(&file_hash)
    .bind(session_id)
    .execute(&config.database_pool)
    .await?;

    // 🔒 SECURITY: Check API quota before making Anthropic API call
    let quota_service = ApiQuotaService::new(config.database_pool.clone());
    let (allowed, used, remaining) = quota_service.check_quota(claims.user_id).await?;

    if !allowed {
        tracing::warn!("API quota exceeded for user: {} (used: {}, remaining: {:?})",
            claims.user_id, used, remaining);
        return Err(crate::middleware::error_handling::AppError::Forbidden(
            format!("API quota exceeded. You have used {} requests this month.", used)
        ));
    }

    tracing::info!("API quota check passed for user: {} (used: {}, remaining: {:?})",
        claims.user_id, used, remaining);

    // Analyze file with Claude AI (in background for large files)
    let start_time = std::time::Instant::now();
    let _mapping = ai_service.analyze_file(session_id, file_data.clone(), claims.user_id).await?;
    let latency_ms = start_time.elapsed().as_millis() as i64;

    // 📊 OBSERVABILITY: Track API usage
    let estimated_tokens_input = (file_data.len() / 4) as i32; // Rough estimate: 1 token ≈ 4 bytes
    let estimated_tokens_output = 500; // Rough estimate for AI response

    quota_service.record_usage(
        claims.user_id,
        "ai_import/file_analysis",
        estimated_tokens_input,
        estimated_tokens_output,
        latency_ms as i32,
    ).await?;

    tracing::info!("API usage tracked for user: {} (input tokens: ~{}, output tokens: ~{})",
        claims.user_id, estimated_tokens_input, estimated_tokens_output);

    // Get updated session
    let session = ai_service.get_session(session_id).await?;

    tracing::info!("File analysis completed for session: {}", session_id);

    Ok(Json(session.into()))
}

/// GET /api/ai-import/session/:id
/// Get import session details
pub async fn get_session(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<ImportSessionResponse>> {
    let claude_api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| crate::middleware::error_handling::AppError::Internal(
            anyhow::anyhow!("ANTHROPIC_API_KEY not configured")
        ))?;

    let ai_service = AiImportService::new(config.database_pool.clone(), claude_api_key);
    let session = ai_service.get_session(session_id).await?;

    // Verify user owns this session
    if session.user_id != claims.user_id {
        return Err(crate::middleware::error_handling::AppError::Forbidden(
            "Access denied".to_string()
        ));
    }

    Ok(Json(session.into()))
}

/// POST /api/ai-import/session/:id/start-import
/// Start the actual import process after mapping approval
pub async fn start_import(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<ImportSessionResponse>> {
    tracing::info!("Starting import for session: {}", session_id);

    let claude_api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| crate::middleware::error_handling::AppError::Internal(
            anyhow::anyhow!("ANTHROPIC_API_KEY not configured")
        ))?;

    let ai_service = AiImportService::new(config.database_pool.clone(), claude_api_key);
    let session = ai_service.get_session(session_id).await?;

    // Verify user owns this session
    if session.user_id != claims.user_id {
        return Err(crate::middleware::error_handling::AppError::Forbidden(
            "Access denied".to_string()
        ));
    }

    // Verify session is in correct state
    if session.status != "mapping_review" {
        return Err(crate::middleware::error_handling::AppError::BadRequest(
            format!("Session must be in mapping_review state. Current state: {}", session.status)
        ));
    }

    // Get mapping from session
    let mapping: ColumnMapping = serde_json::from_value(
        session.ai_mapping
            .ok_or_else(|| crate::middleware::error_handling::AppError::BadRequest(
                "No mapping available for this session".to_string()
            ))?
    ).map_err(|e: serde_json::Error| crate::middleware::error_handling::AppError::Internal(
        anyhow::anyhow!("Failed to parse mapping: {}", e)
    ))?;

    // Get file path from session
    let file_path = session.file_path
        .ok_or_else(|| crate::middleware::error_handling::AppError::BadRequest(
            "No file available for this session".to_string()
        ))?;

    // 🔒 PRODUCTION SECURITY: Load and decrypt file from disk
    let file_storage = EncryptedFileStorage::new(
        &config.file_storage_path,
        &config.encryption_key
    )?;
    let file_data = file_storage.read_encrypted_file(&file_path)?;

    // 🔒 SECURITY: Sanitize file path for log injection prevention
    tracing::info!("Loaded file from storage: {} ({} bytes)",
        crate::utils::log_sanitizer::sanitize_for_log(&file_path),
        file_data.len());

    // Update session status to importing
    sqlx::query!(
        "UPDATE ai_import_sessions SET status = 'importing', import_started_at = NOW() WHERE id = $1",
        session_id
    )
    .execute(&config.database_pool)
    .await?;

    // Parse the file
    let parsed_file = FileParserService::parse(&file_data, &session.original_filename)?;

    tracing::info!("File parsed: {} rows", parsed_file.rows.len());

    // Process import with batch processor
    let batch_processor = BatchImportProcessor::new(config.database_pool.clone());
    let stats = batch_processor.process_import(
        session_id,
        claims.user_id,
        parsed_file,
        mapping,
    ).await?;

    tracing::info!(
        "Import completed for session {}: {} imported, {} failed",
        session_id,
        stats.rows_imported,
        stats.rows_failed
    );

    // Update session status to completed
    sqlx::query!(
        "UPDATE ai_import_sessions SET status = 'completed', import_completed_at = NOW() WHERE id = $1",
        session_id
    )
    .execute(&config.database_pool)
    .await?;

    // Return updated session
    let updated_session = ai_service.get_session(session_id).await?;
    Ok(Json(updated_session.into()))
}

/// GET /api/ai-import/sessions
/// List user's import sessions
pub async fn list_sessions(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<ListSessionsQuery>,
) -> Result<Json<Vec<ImportSessionResponse>>> {
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);

    let sessions = sqlx::query_as::<_, AiImportSession>(
        r#"
        SELECT * FROM ai_import_sessions
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#
    )
    .bind(claims.user_id)
    .bind(limit as i64)
    .bind(offset as i64)
    .fetch_all(&config.database_pool)
    .await?;

    let responses: Vec<ImportSessionResponse> = sessions.into_iter()
        .map(|s| s.into())
        .collect();

    Ok(Json(responses))
}

/// GET /api/ai-import/session/:id/rows
/// Get detailed row results for a session
pub async fn get_session_rows(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(session_id): Path<Uuid>,
    Query(params): Query<GetRowsQuery>,
) -> Result<Json<Vec<AiImportRowResult>>> {
    // Verify session ownership
    let session = sqlx::query!(
        "SELECT user_id FROM ai_import_sessions WHERE id = $1",
        session_id
    )
    .fetch_optional(&config.database_pool)
    .await?
    .ok_or_else(|| crate::middleware::error_handling::AppError::NotFound(
        "Session not found".to_string()
    ))?;

    if session.user_id != claims.user_id {
        return Err(crate::middleware::error_handling::AppError::Forbidden(
            "Access denied".to_string()
        ));
    }

    let limit = params.limit.unwrap_or(100).min(500);
    let offset = params.offset.unwrap_or(0);

    // Build query safely with parameterized status filter
    let rows = if let Some(ref status_filter) = params.status_filter {
        sqlx::query_as!(
            AiImportRowResult,
            r#"
            SELECT * FROM ai_import_row_results
            WHERE session_id = $1 AND status = $2
            ORDER BY row_number ASC
            LIMIT $3 OFFSET $4
            "#,
            session_id,
            status_filter,
            limit as i64,
            offset as i64
        )
        .fetch_all(&config.database_pool)
        .await?
    } else {
        sqlx::query_as!(
            AiImportRowResult,
            r#"
            SELECT * FROM ai_import_row_results
            WHERE session_id = $1
            ORDER BY row_number ASC
            LIMIT $2 OFFSET $3
            "#,
            session_id,
            limit as i64,
            offset as i64
        )
        .fetch_all(&config.database_pool)
        .await?
    };

    Ok(Json(rows))
}

/// GET /api/ai-import/quota
/// Get user's AI usage quota and limits
pub async fn get_user_quota(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<UserQuotaResponse>> {
    let quota = sqlx::query!(
        r#"
        SELECT
            monthly_import_limit,
            monthly_imports_used,
            monthly_ai_cost_limit_usd,
            monthly_ai_cost_used_usd,
            limit_period_start,
            limit_period_end
        FROM user_ai_usage_limits
        WHERE user_id = $1
        "#,
        claims.user_id
    )
    .fetch_optional(&config.database_pool)
    .await?;

    let response = if let Some(q) = quota {
        UserQuotaResponse {
            monthly_import_limit: q.monthly_import_limit,
            monthly_imports_used: q.monthly_imports_used,
            monthly_cost_limit_usd: format!("{:.2}", q.monthly_ai_cost_limit_usd),
            monthly_cost_used_usd: format!("{:.4}", q.monthly_ai_cost_used_usd),
            imports_remaining: (q.monthly_import_limit - q.monthly_imports_used).max(0),
            cost_remaining_usd: format!("{:.4}", (q.monthly_ai_cost_limit_usd - q.monthly_ai_cost_used_usd).max(rust_decimal::Decimal::ZERO)),
            period_start: q.limit_period_start.to_string(),
            period_end: q.limit_period_end.to_string(),
        }
    } else {
        // Default quota for new users
        UserQuotaResponse {
            monthly_import_limit: 50,
            monthly_imports_used: 0,
            monthly_cost_limit_usd: "10.00".to_string(),
            monthly_cost_used_usd: "0.0000".to_string(),
            imports_remaining: 50,
            cost_remaining_usd: "10.0000".to_string(),
            period_start: chrono::Utc::now().date_naive().to_string(),
            period_end: (chrono::Utc::now() + chrono::Duration::days(30)).date_naive().to_string(),
        }
    };

    Ok(Json(response))
}

// ============================================================================
// Request/Response Models
// ============================================================================

#[derive(serde::Deserialize)]
pub struct ListSessionsQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(serde::Deserialize)]
pub struct GetRowsQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub status_filter: Option<String>,
}

#[derive(serde::Serialize)]
pub struct UserQuotaResponse {
    pub monthly_import_limit: i32,
    pub monthly_imports_used: i32,
    pub monthly_cost_limit_usd: String,
    pub monthly_cost_used_usd: String,
    pub imports_remaining: i32,
    pub cost_remaining_usd: String,
    pub period_start: String,
    pub period_end: String,
}
