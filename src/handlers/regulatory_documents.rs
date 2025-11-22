/// Regulatory Document Generation REST API Handlers
///
/// HTTP endpoints for AI-powered regulatory document generation (CoA, GDP, GMP)
/// with RAG, Ed25519 signatures, and immutable audit ledgers.

use axum::{
    extract::{Path, Query, State},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    config::AppConfig,
    middleware::{error_handling::Result, Claims},
    services::{GenerateDocumentRequest, GeneratedDocument, RegulatoryDocumentGenerator},
};

// ============================================================================
// REQUEST/RESPONSE MODELS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ListDocumentsQuery {
    #[serde(default)]
    pub document_type: Option<String>, // 'CoA', 'GDP', 'GMP'
    #[serde(default)]
    pub status: Option<String>, // 'draft', 'approved', etc.
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    20
}

#[derive(Debug, Serialize)]
pub struct DocumentListResponse {
    pub documents: Vec<DocumentSummary>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DocumentSummary {
    pub id: Uuid,
    pub document_type: String,
    pub document_number: String,
    pub title: String,
    pub status: String,
    pub generated_by_name: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ApproveDocumentRequest {
    pub comments: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DocumentVerificationResponse {
    pub document_id: Uuid,
    pub signature_valid: bool,
    pub ledger_valid: bool,
    pub overall_valid: bool,
    pub verified_at: chrono::DateTime<chrono::Utc>,
}

// ============================================================================
// DOCUMENT GENERATION ENDPOINTS
// ============================================================================

/// POST /api/regulatory/documents/generate
/// Generate a regulatory document (CoA, GDP, GMP) using RAG + Claude AI
///
/// This endpoint:
/// 1. Retrieves relevant regulations using semantic search (RAG)
/// 2. Generates document content using Claude AI
/// 3. Signs document with user's Ed25519 private key
/// 4. Creates immutable audit ledger entry
pub async fn generate_document(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<GenerateDocumentRequest>,
) -> Result<Json<GeneratedDocument>> {
    tracing::info!(
        "User {} generating {:?} document",
        claims.user_id,
        request.document_type
    );

    // Get API key from environment
    let anthropic_api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not configured"))?;

    // Create regulatory document generator
    let generator = RegulatoryDocumentGenerator::new(
        config.database_pool.clone(),
        anthropic_api_key,
        &config.encryption_key,
        claims.user_id,  // Use actual user for quota tracking
    )?;

    // Generate document with RAG + Claude AI + Ed25519 signature
    let document = generator.generate_document(request, claims.user_id).await?;

    // Audit log
    tracing::info!(
        "Audit: User {} generated {} document {}",
        claims.user_id,
        document.document_type,
        document.document_number
    );

    tracing::info!(
        "Successfully generated {} document: {} for user {}",
        document.document_type,
        document.document_number,
        claims.user_id
    );

    Ok(Json(document))
}

/// GET /api/regulatory/documents
/// List regulatory documents with filtering and pagination
pub async fn list_documents(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<ListDocumentsQuery>,
) -> Result<Json<DocumentListResponse>> {
    let offset = (query.page - 1) * query.page_size;

    // Build dynamic query
    let mut conditions = vec!["generated_by = $1".to_string()];
    let mut param_count = 2;

    if query.document_type.is_some() {
        conditions.push(format!("document_type = ${}", param_count));
        param_count += 1;
    }

    if query.status.is_some() {
        conditions.push(format!("status = ${}", param_count));
        param_count += 1;
    }

    let where_clause = conditions.join(" AND ");

    // Count total
    let count_query = format!(
        "SELECT COUNT(*) as count FROM regulatory_documents WHERE {}",
        where_clause
    );

    let total: i64 = if let Some(doc_type) = &query.document_type {
        if let Some(status) = &query.status {
            sqlx::query_scalar(&count_query)
                .bind(claims.user_id)
                .bind(doc_type)
                .bind(status)
                .fetch_one(&config.database_pool)
                .await?
        } else {
            sqlx::query_scalar(&count_query)
                .bind(claims.user_id)
                .bind(doc_type)
                .fetch_one(&config.database_pool)
                .await?
        }
    } else if let Some(status) = &query.status {
        sqlx::query_scalar(&count_query)
            .bind(claims.user_id)
            .bind(status)
            .fetch_one(&config.database_pool)
            .await?
    } else {
        sqlx::query_scalar(&count_query)
            .bind(claims.user_id)
            .fetch_one(&config.database_pool)
            .await?
    };

    // Fetch documents
    let docs_query = format!(
        r#"
        SELECT
            rd.id,
            rd.document_type,
            rd.document_number,
            rd.title,
            rd.status,
            rd.created_at,
            u.email as generated_by_name
        FROM regulatory_documents rd
        LEFT JOIN users u ON rd.generated_by = u.id
        WHERE {}
        ORDER BY rd.created_at DESC
        LIMIT ${} OFFSET ${}
        "#,
        where_clause, param_count, param_count + 1
    );

    let documents: Vec<DocumentSummary> = if let Some(doc_type) = &query.document_type {
        if let Some(status) = &query.status {
            sqlx::query_as::<_, DocumentSummary>(&docs_query)
                .bind(claims.user_id)
                .bind(doc_type)
                .bind(status)
                .bind(query.page_size)
                .bind(offset)
                .fetch_all(&config.database_pool)
                .await?
        } else {
            sqlx::query_as::<_, DocumentSummary>(&docs_query)
                .bind(claims.user_id)
                .bind(doc_type)
                .bind(query.page_size)
                .bind(offset)
                .fetch_all(&config.database_pool)
                .await?
        }
    } else if let Some(status) = &query.status {
        sqlx::query_as::<_, DocumentSummary>(&docs_query)
            .bind(claims.user_id)
            .bind(status)
            .bind(query.page_size)
            .bind(offset)
            .fetch_all(&config.database_pool)
            .await?
    } else {
        sqlx::query_as::<_, DocumentSummary>(&docs_query)
            .bind(claims.user_id)
            .bind(query.page_size)
            .bind(offset)
            .fetch_all(&config.database_pool)
            .await?
    };

    Ok(Json(DocumentListResponse {
        documents,
        total,
        page: query.page,
        page_size: query.page_size,
    }))
}

/// GET /api/regulatory/documents/:id
/// Get a single regulatory document by ID
pub async fn get_document(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(document_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Fetch document with full details - using explicit column selection for proper NULL handling
    let doc = sqlx::query!(
        r#"
        SELECT
            rd.id,
            rd.document_type,
            rd.document_number,
            rd.title,
            rd.content,
            rd.content_markdown as "content_markdown?",
            rd.content_hash,
            rd.generated_signature as "generated_signature?",
            rd.approved_signature as "approved_signature?",
            rd.rag_context,
            rd.status,
            rd.generated_by,
            rd.approved_by as "approved_by?",
            rd.approved_at as "approved_at?",
            rd.created_at,
            rd.updated_at,
            u1.email as "generated_by_name?",
            u1.email as "generated_by_email?",
            u2.email as "approved_by_name?",
            u2.email as "approved_by_email?"
        FROM regulatory_documents rd
        LEFT JOIN users u1 ON rd.generated_by = u1.id
        LEFT JOIN users u2 ON rd.approved_by = u2.id
        WHERE rd.id = $1 AND rd.generated_by = $2
        "#,
        document_id,
        claims.user_id
    )
    .fetch_one(&config.database_pool)
    .await?;

    // Fetch audit ledger
    let ledger = sqlx::query!(
        r#"
        SELECT
            id,
            operation,
            content_hash,
            signature,
            signature_public_key,
            created_at,
            chain_hash
        FROM regulatory_document_ledger
        WHERE document_id = $1
        ORDER BY id ASC
        "#,
        document_id
    )
    .fetch_all(&config.database_pool)
    .await?;

    let response = serde_json::json!({
        "id": doc.id,
        "document_type": doc.document_type,
        "document_number": doc.document_number,
        "title": doc.title,
        "content": doc.content,
        "content_markdown": doc.content_markdown,
        "content_hash": doc.content_hash,
        "status": doc.status,
        // Keep generated_by as UUID string for frontend compatibility
        "generated_by": doc.generated_by.to_string(),
        // Add user details in separate field
        "generated_by_user": {
            "id": doc.generated_by,
            "name": doc.generated_by_name,
            "email": doc.generated_by_email,
        },
        "generated_signature": doc.generated_signature,
        // Keep approved_by as UUID string for frontend compatibility
        "approved_by": doc.approved_by.map(|id| id.to_string()),
        "approved_by_user": doc.approved_by.map(|id| serde_json::json!({
            "id": id,
            "name": doc.approved_by_name,
            "email": doc.approved_by_email,
        })),
        "approved_signature": doc.approved_signature,
        "approved_at": doc.approved_at,
        "rag_context": doc.rag_context,
        "created_at": doc.created_at,
        "updated_at": doc.updated_at,
        "audit_ledger": ledger.iter().map(|entry| serde_json::json!({
            "id": entry.id,
            "operation": entry.operation,
            "content_hash": entry.content_hash,
            // Safe truncation - show preview without panic on short strings
            "signature": entry.signature.get(..16).unwrap_or(&entry.signature),
            "public_key": entry.signature_public_key.get(..16).unwrap_or(&entry.signature_public_key),
            "created_at": entry.created_at,
            "chain_hash": entry.chain_hash.get(..16).unwrap_or(&entry.chain_hash),
        })).collect::<Vec<_>>(),
    });

    Ok(Json(response))
}

/// POST /api/regulatory/documents/:id/approve
/// Approve a regulatory document (adds approval signature)
pub async fn approve_document(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(document_id): Path<Uuid>,
    Json(_request): Json<ApproveDocumentRequest>,
) -> Result<Json<serde_json::Value>> {
    tracing::info!(
        "User {} approving document {}",
        claims.user_id,
        document_id
    );

    // Get API key from environment
    let anthropic_api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not configured"))?;

    // Create regulatory document generator
    let generator = RegulatoryDocumentGenerator::new(
        config.database_pool.clone(),
        anthropic_api_key,
        &config.encryption_key,
        claims.user_id,  // Use actual user for quota tracking
    )?;

    // Approve document with Ed25519 signature
    generator
        .approve_document(document_id, claims.user_id)
        .await?;

    tracing::info!(
        "Audit: User {} approved document {}",
        claims.user_id,
        document_id
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "document_id": document_id,
        "approved_by": claims.user_id,
        "approved_at": chrono::Utc::now(),
    })))
}

/// GET /api/regulatory/documents/:id/verify
/// Verify document signature and ledger chain integrity
pub async fn verify_document(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(document_id): Path<Uuid>,
) -> Result<Json<DocumentVerificationResponse>> {
    tracing::info!("Verifying document {}", document_id);

    // Get API key from environment
    let anthropic_api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not configured"))?;

    // Create regulatory document generator
    let generator = RegulatoryDocumentGenerator::new(
        config.database_pool.clone(),
        anthropic_api_key,
        &config.encryption_key,
        claims.user_id,  // Use actual user for quota tracking
    )?;

    // Verify document (signature + ledger chain)
    let is_valid = generator.verify_document(document_id).await?;

    tracing::info!(
        "Document {} verification result: {}",
        document_id,
        is_valid
    );

    Ok(Json(DocumentVerificationResponse {
        document_id,
        signature_valid: is_valid,
        ledger_valid: is_valid,
        overall_valid: is_valid,
        verified_at: chrono::Utc::now(),
    }))
}

/// GET /api/regulatory/documents/:id/audit-trail
/// Get complete audit trail for a document
pub async fn get_audit_trail(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(document_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Verify user owns document
    let doc = sqlx::query!(
        "SELECT id FROM regulatory_documents WHERE id = $1 AND generated_by = $2",
        document_id,
        claims.user_id
    )
    .fetch_one(&config.database_pool)
    .await?;

    // Fetch complete audit ledger
    let ledger = sqlx::query!(
        r#"
        SELECT
            id,
            document_id,
            operation,
            content_hash,
            signature,
            signature_public_key,
            signature_algorithm,
            previous_entry_hash,
            chain_hash,
            created_at,
            metadata
        FROM regulatory_document_ledger
        WHERE document_id = $1
        ORDER BY id ASC
        "#,
        document_id
    )
    .fetch_all(&config.database_pool)
    .await?;

    let response = serde_json::json!({
        "document_id": doc.id,
        "ledger_entries": ledger.iter().map(|entry| serde_json::json!({
            "id": entry.id,
            "operation": entry.operation,
            "content_hash": entry.content_hash,
            "signature": entry.signature,
            "public_key": entry.signature_public_key,
            "algorithm": entry.signature_algorithm,
            "previous_hash": entry.previous_entry_hash,
            "chain_hash": entry.chain_hash,
            "created_at": entry.created_at,
            "metadata": entry.metadata,
        })).collect::<Vec<_>>(),
        "total_entries": ledger.len(),
    });

    Ok(Json(response))
}

/// GET /api/regulatory/knowledge-base/stats
/// Get statistics about the regulatory knowledge base
pub async fn get_knowledge_base_stats(
    State(config): State<AppConfig>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>> {
    // Count entries by document type
    let stats = sqlx::query!(
        r#"
        SELECT
            document_type,
            COUNT(*) as count,
            COUNT(DISTINCT regulation_source) as unique_sources
        FROM regulatory_knowledge_base
        GROUP BY document_type
        "#
    )
    .fetch_all(&config.database_pool)
    .await?;

    let total = sqlx::query!(
        "SELECT COUNT(*) as count FROM regulatory_knowledge_base"
    )
    .fetch_one(&config.database_pool)
    .await?;

    Ok(Json(serde_json::json!({
        "total_entries": total.count,
        "by_document_type": stats.iter().map(|s| serde_json::json!({
            "document_type": s.document_type,
            "count": s.count,
            "unique_sources": s.unique_sources,
        })).collect::<Vec<_>>(),
    })))
}
