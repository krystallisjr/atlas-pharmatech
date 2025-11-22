// ============================================================================
// Admin Security Handler - Security Monitoring Dashboard Endpoints
// ============================================================================
//
// 🔒 ADMIN ONLY: Provides security monitoring and management capabilities
//
// ## Endpoints:
// - GET  /api/admin/security/api-usage      - API usage logs and analytics
// - GET  /api/admin/security/quotas         - User quota tiers and usage
// - PUT  /api/admin/security/quotas/:id     - Update user quota tier
// - GET  /api/admin/security/encryption     - Encryption key rotation status
// - POST /api/admin/security/encryption/rotate - Trigger key rotation
// - GET  /api/admin/security/metrics        - Prometheus metrics summary
// - GET  /api/admin/security/rate-limits    - Rate limiting overview
//
// ============================================================================

use axum::{
    extract::{Path, Query, State},
    Extension, Json,
};
use chrono::{DateTime, Utc, Datelike};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    config::AppConfig,
    middleware::{auth::Claims, error_handling::{AppError, Result}},
    services::{
        api_quota_service::{ApiQuotaService, QuotaTier},
        encryption_key_rotation_service::EncryptionKeyRotationService,
        comprehensive_audit_service::{ComprehensiveAuditService, AuditLogEntry, EventCategory, Severity, ActionResult},
    },
};

// ============================================================================
// Request/Response Types
// ============================================================================

/// API Usage Query Filters
#[derive(Debug, Deserialize)]
pub struct ApiUsageFilters {
    pub user_id: Option<Uuid>,
    pub endpoint: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// API Usage Record Response
#[derive(Debug, Serialize)]
pub struct ApiUsageRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_email: Option<String>,
    pub endpoint: String,
    pub tokens_input: i32,
    pub tokens_output: i32,
    pub total_tokens: i32,
    pub cost_cents: f64,
    pub latency_ms: i32,
    pub created_at: DateTime<Utc>,
}

/// API Usage Analytics Response
#[derive(Debug, Serialize)]
pub struct ApiUsageAnalytics {
    pub total_requests: i64,
    pub total_cost_cents: f64,
    pub total_tokens: i64,
    pub avg_latency_ms: f64,
    pub usage_by_endpoint: Vec<EndpointUsage>,
    pub usage_by_user: Vec<UserUsage>,
    pub usage_over_time: Vec<TimeSeriesPoint>,
    pub recent_requests: Vec<ApiUsageRecord>,
}

#[derive(Debug, Serialize)]
pub struct EndpointUsage {
    pub endpoint: String,
    pub request_count: i64,
    pub total_cost_cents: f64,
    pub avg_latency_ms: f64,
}

#[derive(Debug, Serialize)]
pub struct UserUsage {
    pub user_id: Uuid,
    pub user_email: String,
    pub request_count: i64,
    pub total_cost_cents: f64,
    pub quota_tier: QuotaTier,
}

#[derive(Debug, Serialize)]
pub struct TimeSeriesPoint {
    pub date: String,
    pub requests: i64,
    pub cost_cents: f64,
}

/// User Quota Info
#[derive(Debug, Serialize)]
pub struct UserQuotaInfo {
    pub user_id: Uuid,
    pub user_email: String,
    pub quota_tier: QuotaTier,
    pub monthly_limit: Option<i32>,
    pub monthly_used: i32,
    pub monthly_remaining: Option<i32>,
    pub usage_percent: f64,
    pub total_cost_cents: f64,
    pub is_over_quota: bool,
}

/// Quota Update Request
#[derive(Debug, Deserialize)]
pub struct QuotaUpdateRequest {
    pub quota_tier: QuotaTier,
}

/// Encryption Key Info
#[derive(Debug, Serialize)]
pub struct EncryptionKeyInfo {
    pub id: Uuid,
    pub key_version: i32,
    pub status: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub age_days: i64,
    pub days_until_expiry: i64,
}

/// Encryption Status Response
#[derive(Debug, Serialize)]
pub struct EncryptionStatus {
    pub active_key: EncryptionKeyInfo,
    pub rotation_status: String, // "OK", "SOON", "OVERDUE"
    pub days_until_rotation: i64,
    pub all_keys: Vec<EncryptionKeyInfo>,
    pub rotation_history: Vec<KeyRotationEvent>,
}

#[derive(Debug, Serialize)]
pub struct KeyRotationEvent {
    pub id: Uuid,
    pub old_version: i32,
    pub new_version: i32,
    pub rotated_at: DateTime<Utc>,
    pub rotated_by_email: Option<String>,
    pub rotation_reason: Option<String>,
}

/// Key Rotation Request
#[derive(Debug, Deserialize)]
pub struct KeyRotationRequest {
    pub reason: Option<String>,
}

/// Metrics Summary Response
#[derive(Debug, Serialize)]
pub struct MetricsSummary {
    pub http_requests_total: i64,
    pub http_requests_per_minute: f64,
    pub avg_request_duration_ms: f64,
    pub active_connections: i64,
    pub auth_failures_total: i64,
    pub auth_failures_last_hour: i64,
    pub db_pool_active: i64,
    pub db_pool_idle: i64,
    pub request_duration_p50: f64,
    pub request_duration_p95: f64,
    pub request_duration_p99: f64,
    pub status_code_breakdown: HashMap<String, i64>,
}

/// Rate Limit Status Response
#[derive(Debug, Serialize)]
pub struct RateLimitStatus {
    pub active_rate_limits: Vec<RateLimitEntry>,
    pub top_limited_ips: Vec<IpLimitInfo>,
    pub configuration: RateLimitConfig,
}

#[derive(Debug, Serialize)]
pub struct RateLimitEntry {
    pub ip_address: String,
    pub current_tokens: i32,
    pub max_tokens: i32,
    pub last_request: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct IpLimitInfo {
    pub ip_address: String,
    pub hit_count: i64,
    pub last_hit: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct RateLimitConfig {
    pub auth_limit: String,      // "5 requests per 15 minutes"
    pub api_limit: String,        // "100 requests per minute"
    pub public_limit: String,     // "20 requests per 15 minutes"
}

// ============================================================================
// Handler Functions
// ============================================================================

/// GET /api/admin/security/api-usage
///
/// Returns API usage analytics with filters
/// Note: Admin authorization is handled by middleware
///
pub async fn get_api_usage_analytics(
    State(config): State<AppConfig>,
    Extension(_claims): Extension<Claims>,
    Query(filters): Query<ApiUsageFilters>,
) -> Result<Json<ApiUsageAnalytics>> {
    // Authorization handled by admin_middleware

    let pool = &config.database_pool;
    let start_date = filters.start_date.unwrap_or_else(|| {
        Utc::now() - chrono::Duration::days(30)
    });
    let end_date = filters.end_date.unwrap_or_else(Utc::now);

    // Get total stats
    let stats = sqlx::query!(
        r#"
        SELECT
            COUNT(*)::BIGINT as total_requests,
            COALESCE(SUM(cost_cents), 0) as total_cost_cents,
            COALESCE(SUM(tokens_input + tokens_output), 0) as total_tokens,
            COALESCE(AVG(latency_ms), 0)::DOUBLE PRECISION as avg_latency_ms
        FROM api_usage_log
        WHERE created_at >= $1 AND created_at <= $2
            AND ($3::UUID IS NULL OR user_id = $3)
            AND ($4::TEXT IS NULL OR endpoint = $4)
        "#,
        start_date,
        end_date,
        filters.user_id,
        filters.endpoint.as_deref()
    )
    .fetch_one(pool)
    .await?;

    // Usage by endpoint
    let endpoint_usage = sqlx::query!(
        r#"
        SELECT
            endpoint,
            COUNT(*)::BIGINT as request_count,
            COALESCE(SUM(cost_cents), 0) as total_cost_cents,
            COALESCE(AVG(latency_ms), 0)::DOUBLE PRECISION as avg_latency_ms
        FROM api_usage_log
        WHERE created_at >= $1 AND created_at <= $2
            AND ($3::UUID IS NULL OR user_id = $3)
        GROUP BY endpoint
        ORDER BY request_count DESC
        LIMIT 10
        "#,
        start_date,
        end_date,
        filters.user_id
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| EndpointUsage {
        endpoint: row.endpoint,
        request_count: row.request_count.unwrap_or(0),
        total_cost_cents: row.total_cost_cents.unwrap_or(rust_decimal::Decimal::ZERO).to_string().parse().unwrap_or(0.0),
        avg_latency_ms: row.avg_latency_ms.unwrap_or(0.0),
    })
    .collect();

    // Usage by user
    let user_usage = sqlx::query!(
        r#"
        SELECT
            l.user_id,
            u.email as user_email,
            COALESCE(q.quota_tier, 'Free'::"quota_tier") as "quota_tier: QuotaTier",
            COUNT(*)::BIGINT as request_count,
            COALESCE(SUM(l.cost_cents), 0) as total_cost_cents
        FROM api_usage_log l
        JOIN users u ON l.user_id = u.id
        LEFT JOIN user_api_quotas q ON l.user_id = q.user_id
        WHERE l.created_at >= $1 AND l.created_at <= $2
            AND ($3::TEXT IS NULL OR l.endpoint = $3)
        GROUP BY l.user_id, u.email, q.quota_tier
        ORDER BY request_count DESC
        LIMIT 10
        "#,
        start_date,
        end_date,
        filters.endpoint.as_deref()
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| UserUsage {
        user_id: row.user_id,
        user_email: row.user_email,
        request_count: row.request_count.unwrap_or(0),
        total_cost_cents: row.total_cost_cents.unwrap_or(rust_decimal::Decimal::ZERO).to_string().parse().unwrap_or(0.0),
        quota_tier: row.quota_tier.unwrap_or(QuotaTier::Free),
    })
    .collect();

    // Usage over time (daily aggregates)
    let time_series = sqlx::query!(
        r#"
        SELECT
            DATE(created_at) as date,
            COUNT(*)::BIGINT as requests,
            COALESCE(SUM(cost_cents), 0) as cost_cents
        FROM api_usage_log
        WHERE created_at >= $1 AND created_at <= $2
            AND ($3::UUID IS NULL OR user_id = $3)
            AND ($4::TEXT IS NULL OR endpoint = $4)
        GROUP BY DATE(created_at)
        ORDER BY date ASC
        "#,
        start_date,
        end_date,
        filters.user_id,
        filters.endpoint.as_deref()
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| TimeSeriesPoint {
        date: row.date.unwrap_or_default().to_string(),
        requests: row.requests.unwrap_or(0),
        cost_cents: row.cost_cents.unwrap_or(rust_decimal::Decimal::ZERO).to_string().parse().unwrap_or(0.0),
    })
    .collect();

    // Recent requests
    let limit = filters.limit.unwrap_or(20);
    let offset = filters.offset.unwrap_or(0);

    let recent_requests = sqlx::query!(
        r#"
        SELECT
            l.id,
            l.user_id,
            u.email as user_email,
            l.endpoint,
            COALESCE(l.tokens_input, 0) as tokens_input,
            COALESCE(l.tokens_output, 0) as tokens_output,
            COALESCE(l.cost_cents, 0) as cost_cents,
            COALESCE(l.latency_ms, 0) as latency_ms,
            l.created_at
        FROM api_usage_log l
        LEFT JOIN users u ON l.user_id = u.id
        WHERE l.created_at >= $1 AND l.created_at <= $2
            AND ($3::UUID IS NULL OR l.user_id = $3)
            AND ($4::TEXT IS NULL OR l.endpoint = $4)
        ORDER BY l.created_at DESC
        LIMIT $5 OFFSET $6
        "#,
        start_date,
        end_date,
        filters.user_id,
        filters.endpoint.as_deref(),
        limit,
        offset
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| ApiUsageRecord {
        id: row.id,
        user_id: row.user_id,
        user_email: Some(row.user_email),
        endpoint: row.endpoint,
        tokens_input: row.tokens_input.unwrap_or(0),
        tokens_output: row.tokens_output.unwrap_or(0),
        total_tokens: row.tokens_input.unwrap_or(0) + row.tokens_output.unwrap_or(0),
        cost_cents: row.cost_cents.unwrap_or(rust_decimal::Decimal::ZERO).to_string().parse().unwrap_or(0.0),
        latency_ms: row.latency_ms.unwrap_or(0) as i32,
        created_at: row.created_at,
    })
    .collect();

    Ok(Json(ApiUsageAnalytics {
        total_requests: stats.total_requests.unwrap_or(0),
        total_cost_cents: stats.total_cost_cents.unwrap_or(rust_decimal::Decimal::ZERO).to_string().parse().unwrap_or(0.0),
        total_tokens: stats.total_tokens.unwrap_or(0),
        avg_latency_ms: stats.avg_latency_ms.unwrap_or(0.0),
        usage_by_endpoint: endpoint_usage,
        usage_by_user: user_usage,
        usage_over_time: time_series,
        recent_requests,
    }))
}

/// GET /api/admin/security/quotas
///
/// Returns all users' quota tiers and usage
/// Note: Admin authorization is handled by middleware
///
pub async fn get_user_quotas(
    State(config): State<AppConfig>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<Vec<UserQuotaInfo>>> {
    // Authorization handled by admin_middleware

    let pool = &config.database_pool;
    let now = Utc::now();

    let quotas = sqlx::query!(
        r#"
        SELECT
            u.id as user_id,
            u.email as user_email,
            COALESCE(q.quota_tier, 'Free'::"quota_tier") as "quota_tier: QuotaTier",
            COUNT(l.id)::INTEGER as monthly_used,
            COALESCE(SUM(l.cost_cents), 0) as total_cost_cents
        FROM users u
        LEFT JOIN user_api_quotas q ON u.id = q.user_id
        LEFT JOIN api_usage_log l ON u.id = l.user_id
            AND EXTRACT(YEAR FROM l.created_at) = $1
            AND EXTRACT(MONTH FROM l.created_at) = $2
        GROUP BY u.id, u.email, q.quota_tier
        ORDER BY monthly_used DESC
        "#,
        now.year() as f64,
        now.month() as f64
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| {
        let quota_tier = row.quota_tier.unwrap_or(QuotaTier::Free);
        let monthly_limit = quota_tier.monthly_limit();
        let monthly_used = row.monthly_used.unwrap_or(0);
        let monthly_remaining = monthly_limit.map(|limit| limit - monthly_used);
        let usage_percent = match monthly_limit {
            Some(limit) => (monthly_used as f64 / limit as f64 * 100.0).min(100.0),
            None => 0.0,
        };
        let is_over_quota = monthly_remaining.map_or(false, |r| r <= 0);

        UserQuotaInfo {
            user_id: row.user_id,
            user_email: row.user_email,
            quota_tier,
            monthly_limit,
            monthly_used,
            monthly_remaining,
            usage_percent,
            total_cost_cents: row.total_cost_cents.unwrap_or(rust_decimal::Decimal::ZERO).to_string().parse().unwrap_or(0.0),
            is_over_quota,
        }
    })
    .collect();

    Ok(Json(quotas))
}

/// PUT /api/admin/security/quotas/:user_id
///
/// Update user's quota tier
/// Note: Superadmin authorization is handled by middleware
///
pub async fn update_user_quota(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(user_id): Path<Uuid>,
    Json(request): Json<QuotaUpdateRequest>,
) -> Result<Json<UserQuotaInfo>> {
    // Authorization handled by superadmin_middleware

    // Update quota tier
    let quota_service = ApiQuotaService::new(config.database_pool.clone());
    quota_service.upgrade_tier(user_id, request.quota_tier).await?;

    // Get updated quota info
    let pool = &config.database_pool;
    let now = Utc::now();

    let quota_info = sqlx::query!(
        r#"
        SELECT
            u.id as user_id,
            u.email as user_email,
            q.quota_tier as "quota_tier: QuotaTier",
            COUNT(l.id)::INTEGER as monthly_used,
            COALESCE(SUM(l.cost_cents), 0) as total_cost_cents
        FROM users u
        JOIN user_api_quotas q ON u.id = q.user_id
        LEFT JOIN api_usage_log l ON u.id = l.user_id
            AND EXTRACT(YEAR FROM l.created_at) = $1
            AND EXTRACT(MONTH FROM l.created_at) = $2
        WHERE u.id = $3
        GROUP BY u.id, u.email, q.quota_tier
        "#,
        now.year() as f64,
        now.month() as f64,
        user_id
    )
    .fetch_one(pool)
    .await?;

    let quota_tier = quota_info.quota_tier;
    let monthly_limit = quota_tier.monthly_limit();
    let monthly_used = quota_info.monthly_used.unwrap_or(0);
    let monthly_remaining = monthly_limit.map(|limit| limit - monthly_used);
    let usage_percent = match monthly_limit {
        Some(limit) => (monthly_used as f64 / limit as f64 * 100.0).min(100.0),
        None => 0.0,
    };
    let is_over_quota = monthly_remaining.map_or(false, |r| r <= 0);

    // Audit log
    let audit_service = ComprehensiveAuditService::new(config.database_pool.clone());
    audit_service.log(AuditLogEntry {
        event_type: "admin_quota_update".to_string(),
        event_category: EventCategory::Admin,
        severity: Severity::Warning,
        actor_user_id: Some(claims.user_id),
        actor_type: "user".to_string(),
        resource_type: Some("api_quota".to_string()),
        resource_id: Some(user_id.to_string()),
        action: "update_quota_tier".to_string(),
        action_result: ActionResult::Success,
        event_data: serde_json::json!({
            "user_id": user_id,
            "new_tier": format!("{:?}", request.quota_tier),
        }),
        ip_address: None,
        is_pii_access: false,
        compliance_tags: vec!["admin".to_string()],
        ..Default::default()
    }).await?;

    Ok(Json(UserQuotaInfo {
        user_id: quota_info.user_id,
        user_email: quota_info.user_email,
        quota_tier,
        monthly_limit,
        monthly_used,
        monthly_remaining,
        usage_percent,
        total_cost_cents: quota_info.total_cost_cents.unwrap_or(rust_decimal::Decimal::ZERO).to_string().parse().unwrap_or(0.0),
        is_over_quota,
    }))
}

/// GET /api/admin/security/encryption
///
/// Returns encryption key rotation status
/// Note: Admin authorization is handled by middleware
///
pub async fn get_encryption_status(
    State(config): State<AppConfig>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<EncryptionStatus>> {
    // Authorization handled by admin_middleware

    let key_service = EncryptionKeyRotationService::new(
        config.database_pool.clone(),
        config.encryption_key.clone(),
    );

    // Get active key
    let active_key_data = key_service.get_active_key().await?;
    let now = Utc::now();
    let age_days = (now - active_key_data.created_at).num_days();
    let days_until_expiry = (active_key_data.valid_until - now).num_days();

    let rotation_status = if days_until_expiry <= 0 {
        "OVERDUE"
    } else if days_until_expiry <= 7 {
        "SOON"
    } else {
        "OK"
    };

    let active_key = EncryptionKeyInfo {
        id: active_key_data.id,
        key_version: active_key_data.key_version,
        status: format!("{:?}", active_key_data.status),
        is_active: active_key_data.is_active,
        created_at: active_key_data.created_at,
        valid_until: active_key_data.valid_until,
        age_days,
        days_until_expiry,
    };

    // Get all keys
    let all_keys_data = key_service.list_keys().await?;
    let all_keys = all_keys_data
        .into_iter()
        .map(|key| {
            let age = (now - key.created_at).num_days();
            let expiry = (key.valid_until - now).num_days();
            EncryptionKeyInfo {
                id: key.id,
                key_version: key.key_version,
                status: format!("{:?}", key.status),
                is_active: key.is_active,
                created_at: key.created_at,
                valid_until: key.valid_until,
                age_days: age,
                days_until_expiry: expiry,
            }
        })
        .collect();

    // Get rotation history
    let rotation_history = sqlx::query!(
        r#"
        SELECT
            r.id,
            r.old_key_version,
            r.new_key_version,
            r.rotated_at,
            r.rotated_by,
            r.rotation_reason,
            u.email as rotated_by_email
        FROM key_rotation_log r
        LEFT JOIN users u ON r.rotated_by = u.id
        ORDER BY r.rotated_at DESC
        LIMIT 10
        "#
    )
    .fetch_all(&config.database_pool)
    .await?
    .into_iter()
    .map(|row| KeyRotationEvent {
        id: row.id,
        old_version: row.old_key_version.unwrap_or(0),
        new_version: row.new_key_version.unwrap_or(0),
        rotated_at: row.rotated_at,
        rotated_by_email: Some(row.rotated_by_email),
        rotation_reason: row.rotation_reason,
    })
    .collect();

    Ok(Json(EncryptionStatus {
        active_key,
        rotation_status: rotation_status.to_string(),
        days_until_rotation: days_until_expiry,
        all_keys,
        rotation_history,
    }))
}

/// POST /api/admin/security/encryption/rotate
///
/// Trigger manual encryption key rotation
/// Note: Superadmin authorization is handled by middleware
///
pub async fn rotate_encryption_key(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<KeyRotationRequest>,
) -> Result<Json<EncryptionKeyInfo>> {
    // Authorization handled by superadmin_middleware

    let key_service = EncryptionKeyRotationService::new(
        config.database_pool.clone(),
        config.encryption_key.clone(),
    );

    // Perform rotation
    let new_key = key_service.rotate_key().await?;

    let now = Utc::now();
    let age_days = (now - new_key.created_at).num_days();
    let days_until_expiry = (new_key.valid_until - now).num_days();

    // Audit log
    let audit_service = ComprehensiveAuditService::new(config.database_pool.clone());
    audit_service.log(AuditLogEntry {
        event_type: "admin_key_rotation".to_string(),
        event_category: EventCategory::Admin,
        severity: Severity::Critical,
        actor_user_id: Some(claims.user_id),
        actor_type: "user".to_string(),
        resource_type: Some("encryption_key".to_string()),
        resource_id: Some(new_key.key_version.to_string()),
        action: "rotate_encryption_key".to_string(),
        action_result: ActionResult::Success,
        event_data: serde_json::json!({
            "new_key_version": new_key.key_version,
            "reason": request.reason.clone().unwrap_or_else(|| "Manual rotation".to_string()),
        }),
        ip_address: None,
        is_pii_access: false,
        compliance_tags: vec!["admin".to_string(), "security".to_string()],
        ..Default::default()
    }).await?;

    tracing::warn!(
        "🔐 ENCRYPTION KEY ROTATED by admin {} to version {}",
        claims.user_id,
        new_key.key_version
    );

    Ok(Json(EncryptionKeyInfo {
        id: new_key.id,
        key_version: new_key.key_version,
        status: format!("{:?}", new_key.status),
        is_active: new_key.is_active,
        created_at: new_key.created_at,
        valid_until: new_key.valid_until,
        age_days,
        days_until_expiry,
    }))
}

/// GET /api/admin/security/metrics
///
/// Returns Prometheus metrics summary for admin UI
/// Note: Admin authorization is handled by middleware
///
pub async fn get_metrics_summary(
    State(_config): State<AppConfig>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<MetricsSummary>> {
    // Authorization handled by admin_middleware

    // Note: This is a simplified version that returns mock data
    // In production, you would parse the /metrics endpoint or use
    // Prometheus API to fetch real-time metrics

    // TODO: Integrate with Prometheus API or parse /metrics endpoint

    Ok(Json(MetricsSummary {
        http_requests_total: 0,
        http_requests_per_minute: 0.0,
        avg_request_duration_ms: 0.0,
        active_connections: 0,
        auth_failures_total: 0,
        auth_failures_last_hour: 0,
        db_pool_active: 0,
        db_pool_idle: 0,
        request_duration_p50: 0.0,
        request_duration_p95: 0.0,
        request_duration_p99: 0.0,
        status_code_breakdown: HashMap::new(),
    }))
}

/// GET /api/admin/security/rate-limits
///
/// Returns current rate limiting state
/// Note: Admin authorization is handled by middleware
///
pub async fn get_rate_limit_status(
    State(_config): State<AppConfig>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<RateLimitStatus>> {
    // Authorization handled by admin_middleware

    // Note: Rate limiting is in-memory (DashMap) and doesn't have an easy way
    // to export current state. This returns configuration only.

    // TODO: Add method to ip_rate_limiter to export current state

    Ok(Json(RateLimitStatus {
        active_rate_limits: vec![],
        top_limited_ips: vec![],
        configuration: RateLimitConfig {
            auth_limit: "5 requests per 15 minutes".to_string(),
            api_limit: "100 requests per minute".to_string(),
            public_limit: "20 requests per 15 minutes".to_string(),
        },
    }))
}
