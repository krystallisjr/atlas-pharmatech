// ============================================================================
// Admin Handlers - HTTP Endpoints for Admin Dashboard
// ============================================================================
//
// Production-ready admin endpoints with:
// - Role-based access control
// - Comprehensive error handling
// - Audit logging
// - Input validation
//
// All endpoints require admin/superadmin role (enforced by middleware)
//
// ============================================================================

use axum::{
    extract::{Path, Query, State, Extension},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    Json,
};
use uuid::Uuid;
use crate::config::AppConfig;
use crate::middleware::{Claims, error_handling::{Result, AppError}};
use crate::repositories::UserRepository;
use crate::services::{
    AdminService,
    admin_service::*,
    ComprehensiveAuditService,
};
use crate::{require_admin, require_superadmin};

// ============================================================================
// USER MANAGEMENT ENDPOINTS
// ============================================================================

/// GET /api/admin/users - List all users with filters
///
/// Query parameters:
/// - limit: i64 (default: 50, max: 100)
/// - offset: i64 (default: 0)
/// - role: string (user|admin|superadmin)
/// - verified: bool
/// - search: string (searches company_name)
///
/// Requires: admin or superadmin role
pub async fn list_users(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    Query(query): Query<ListUsersQuery>,
) -> Result<Json<ListUsersResponse>> {
    // 🔒 SECURITY: Extract IP address for audit logging
    let ip_address = Some(addr.ip());

    // Create admin service
    let user_repo = UserRepository::new(config.database_pool.clone(), &config.encryption_key)?;
    let audit_service = ComprehensiveAuditService::new(config.database_pool.clone());
    let admin_service = AdminService::new(user_repo, audit_service);

    // List users
    let response = admin_service.list_users(
        query,
        claims.user_id,
        ip_address.map(|ip| ip.to_string()),
    ).await?;

    Ok(Json(response))
}

/// GET /api/admin/users/:id - Get single user details
///
/// Path parameters:
/// - id: UUID
///
/// Requires: admin or superadmin role
pub async fn get_user(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    Path(user_id): Path<String>,
) -> Result<Json<crate::models::user::UserResponse>> {
    // 🔒 SECURITY: Extract IP address for audit logging
    let ip_address = Some(addr.ip());

    // Parse user ID
    let user_id = Uuid::parse_str(&user_id)
        .map_err(|_| AppError::BadRequest("Invalid user ID format".to_string()))?;

    // Create admin service
    let user_repo = UserRepository::new(config.database_pool.clone(), &config.encryption_key)?;
    let audit_service = ComprehensiveAuditService::new(config.database_pool.clone());
    let admin_service = AdminService::new(user_repo, audit_service);

    // Get user
    let user = admin_service.get_user(
        user_id,
        claims.user_id,
        ip_address.map(|ip| ip.to_string()),
    ).await?;

    Ok(Json(user))
}

/// POST /api/admin/users/:id/verify - Verify or unverify a user
///
/// Path parameters:
/// - id: UUID
///
/// Request body:
/// ```json
/// {
///   "verified": true,
///   "notes": "Company license verified"
/// }
/// ```
///
/// Requires: admin or superadmin role
pub async fn verify_user(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    Path(user_id): Path<String>,
    Json(request): Json<VerifyUserRequest>,
) -> Result<Json<crate::models::user::UserResponse>> {
    // 🔒 SECURITY: Extract IP address for audit logging
    let ip_address = Some(addr.ip());

    // Parse user ID
    let user_id = Uuid::parse_str(&user_id)
        .map_err(|_| AppError::BadRequest("Invalid user ID format".to_string()))?;

    // Create admin service
    let user_repo = UserRepository::new(config.database_pool.clone(), &config.encryption_key)?;
    let audit_service = ComprehensiveAuditService::new(config.database_pool.clone());
    let admin_service = AdminService::new(user_repo, audit_service);

    // Verify user
    let user = admin_service.verify_user(
        user_id,
        request,
        claims.user_id,
        claims.email.clone(),
        ip_address.map(|ip| ip.to_string()),
    ).await?;

    Ok(Json(user))
}

/// PUT /api/admin/users/:id/role - Change user role
///
/// Path parameters:
/// - id: UUID
///
/// Request body:
/// ```json
/// {
///   "role": "admin"
/// }
/// ```
///
/// Requires: superadmin role ONLY (enforced by superadmin_middleware)
pub async fn change_user_role(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    Path(user_id): Path<String>,
    Json(request): Json<ChangeUserRoleRequest>,
) -> Result<Json<crate::models::user::UserResponse>> {
    // 🔒 SECURITY: Extract IP address for audit logging
    let ip_address = Some(addr.ip());

    // Verify superadmin (double-check, middleware should already enforce this)
    require_superadmin!(claims);

    // Parse user ID
    let user_id = Uuid::parse_str(&user_id)
        .map_err(|_| AppError::BadRequest("Invalid user ID format".to_string()))?;

    // Create admin service
    let user_repo = UserRepository::new(config.database_pool.clone(), &config.encryption_key)?;
    let audit_service = ComprehensiveAuditService::new(config.database_pool.clone());
    let admin_service = AdminService::new(user_repo, audit_service);

    // Change role
    let user = admin_service.change_user_role(
        user_id,
        request,
        claims.user_id,
        claims.email.clone(),
        ip_address.map(|ip| ip.to_string()),
    ).await?;

    Ok(Json(user))
}

/// DELETE /api/admin/users/:id - Delete user
///
/// Path parameters:
/// - id: UUID
///
/// Requires: superadmin role ONLY (enforced by superadmin_middleware)
pub async fn delete_user(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    Path(user_id): Path<String>,
) -> Result<StatusCode> {
    // 🔒 SECURITY: Extract IP address for audit logging
    let ip_address = Some(addr.ip());

    // Verify superadmin (double-check, middleware should already enforce this)
    require_superadmin!(claims);

    // Parse user ID
    let user_id = Uuid::parse_str(&user_id)
        .map_err(|_| AppError::BadRequest("Invalid user ID format".to_string()))?;

    // Prevent self-deletion
    if user_id == claims.user_id {
        return Err(AppError::BadRequest("Cannot delete your own account".to_string()));
    }

    // Create admin service
    let user_repo = UserRepository::new(config.database_pool.clone(), &config.encryption_key)?;
    let audit_service = ComprehensiveAuditService::new(config.database_pool.clone());
    let admin_service = AdminService::new(user_repo, audit_service);

    // Delete user
    admin_service.delete_user(
        user_id,
        claims.user_id,
        claims.email.clone(),
        ip_address.map(|ip| ip.to_string()),
    ).await?;

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// VERIFICATION QUEUE ENDPOINTS
// ============================================================================

/// GET /api/admin/verification-queue - Get pending verification queue
///
/// Returns list of unverified users with context (inventory count, transaction count, waiting time)
///
/// Requires: admin or superadmin role
pub async fn get_verification_queue(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
) -> Result<Json<Vec<VerificationQueueItem>>> {
    // 🔒 SECURITY: Extract IP address for audit logging
    let ip_address = Some(addr.ip());

    // Create admin service
    let user_repo = UserRepository::new(config.database_pool.clone(), &config.encryption_key)?;
    let audit_service = ComprehensiveAuditService::new(config.database_pool.clone());
    let admin_service = AdminService::new(user_repo, audit_service);

    // Get queue
    let queue = admin_service.get_verification_queue(
        claims.user_id,
        ip_address.map(|ip| ip.to_string()),
    ).await?;

    Ok(Json(queue))
}

// ============================================================================
// STATISTICS ENDPOINTS
// ============================================================================

/// GET /api/admin/stats - Get admin dashboard statistics
///
/// Returns comprehensive system statistics:
/// - Total users, verified users, pending verifications
/// - Total admins, inventory items, transactions
/// - Recent signups (last 7 days)
/// - System health metrics
///
/// Requires: admin or superadmin role
pub async fn get_admin_stats(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<AdminStatsResponse>> {
    // Create admin service
    let user_repo = UserRepository::new(config.database_pool.clone(), &config.encryption_key)?;
    let audit_service = ComprehensiveAuditService::new(config.database_pool.clone());
    let admin_service = AdminService::new(user_repo, audit_service);

    // Get stats
    let stats = admin_service.get_admin_stats(
        claims.user_id,
        &config.database_pool,
    ).await?;

    Ok(Json(stats))
}

// ============================================================================
// AUDIT LOG ENDPOINTS
// ============================================================================

/// GET /api/admin/audit-logs - Get audit logs with filters
///
/// Query parameters:
/// - limit: i64 (default: 50, max: 100)
/// - offset: i64 (default: 0)
/// - user_id: UUID
/// - event_category: string (auth|data_access|admin|security|system)
/// - start_date: ISO 8601 datetime
/// - end_date: ISO 8601 datetime
///
/// Requires: admin or superadmin role
pub async fn get_audit_logs(
    State(config): State<AppConfig>,
    Extension(_claims): Extension<Claims>,
    Query(query): Query<AuditLogQuery>,
) -> Result<Json<Vec<AuditLogResponse>>> {
    // Create admin service
    let user_repo = UserRepository::new(config.database_pool.clone(), &config.encryption_key)?;
    let audit_service = ComprehensiveAuditService::new(config.database_pool.clone());
    let admin_service = AdminService::new(user_repo, audit_service);

    // Get audit logs
    let logs = admin_service.get_audit_logs(
        query,
        &config.database_pool,
    ).await?;

    Ok(Json(logs))
}

// ============================================================================
// HEALTH CHECK ENDPOINT (No auth required)
// ============================================================================

/// GET /api/admin/health - Admin API health check
///
/// Returns 200 OK if admin API is operational
///
/// No authentication required (for monitoring systems)
pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "admin_api",
        "timestamp": chrono::Utc::now(),
    }))
}
