// ============================================================================
// Admin Service - Business Logic Layer for Admin Operations
// ============================================================================
//
// Production-ready service layer for admin dashboard functionality.
//
// Features:
// - User management (list, search, verify, role changes)
// - Verification queue management
// - System statistics and analytics
// - Comprehensive audit logging
// - Security enforcement
//
// ============================================================================

use uuid::Uuid;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use sqlx::Row;
use anyhow::anyhow;
use crate::models::user::{User, UserResponse, UserRole};
use crate::repositories::UserRepository;
use crate::middleware::error_handling::{Result, AppError};
use crate::services::comprehensive_audit_service::{
    ComprehensiveAuditService,
    AuditLogEntry,
    EventCategory,
    Severity,
    ActionResult,
};

// ============================================================================
// REQUEST/RESPONSE MODELS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ListUsersQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub role: Option<String>,
    pub verified: Option<bool>,
    pub search: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListUsersResponse {
    pub users: Vec<UserResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize)]
pub struct AdminStatsResponse {
    pub total_users: i64,
    pub verified_users: i64,
    pub pending_verifications: i64,
    pub total_admins: i64,
    pub total_inventory_items: i64,
    pub total_transactions: i64,
    pub recent_signups: Vec<RecentSignup>,
    pub system_health: SystemHealth,
}

#[derive(Debug, Serialize)]
pub struct RecentSignup {
    pub id: String,
    pub email: String,
    pub company_name: String,
    pub created_at: DateTime<Utc>,
    pub is_verified: bool,
}

#[derive(Debug, Serialize)]
pub struct SystemHealth {
    pub database_connected: bool,
    pub uptime_seconds: u64,
    pub total_api_calls_today: i64,
}

#[derive(Debug, Deserialize)]
pub struct VerifyUserRequest {
    pub verified: bool,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChangeUserRoleRequest {
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct VerificationQueueItem {
    pub user: UserResponse,
    pub inventory_count: i64,
    pub transaction_count: i64,
    pub days_waiting: i64,
}

#[derive(Debug, Serialize)]
pub struct AuditLogResponse {
    pub id: String,
    pub event_type: String,
    pub event_category: String,
    pub severity: String,
    pub actor_user_id: Option<String>,
    pub action: String,
    pub action_result: String,
    pub event_data: serde_json::Value,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub user_id: Option<String>,
    pub event_category: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}

// ============================================================================
// ADMIN SERVICE
// ============================================================================

pub struct AdminService {
    user_repo: UserRepository,
    audit_service: ComprehensiveAuditService,
}

impl AdminService {
    pub fn new(user_repo: UserRepository, audit_service: ComprehensiveAuditService) -> Self {
        Self {
            user_repo,
            audit_service,
        }
    }

    // ========================================================================
    // USER MANAGEMENT
    // ========================================================================

    /// List users with pagination and filters
    ///
    /// # Security
    /// - Requires admin role (enforced by middleware)
    /// - Logs access to PII data
    /// - Returns sanitized user data
    pub async fn list_users(
        &self,
        query: ListUsersQuery,
        admin_user_id: Uuid,
        ip_address: Option<String>,
    ) -> Result<ListUsersResponse> {
        // Parse role filter if provided
        let role_filter = if let Some(ref role_str) = query.role {
            Some(self.parse_role(role_str)?)
        } else {
            None
        };

        // Fetch users from repository
        let users = self.user_repo.list_users(
            query.limit,
            query.offset,
            role_filter.clone(),
            query.verified,
            query.search.clone(),
        ).await?;

        // Get total count for pagination
        let total = self.user_repo.count_users(role_filter, query.verified).await?;

        // Convert to response DTOs (excludes password_hash)
        let user_responses: Vec<UserResponse> = users.into_iter().map(|u| u.into()).collect();

        // Audit log: Admin accessed user list (PII access)
        self.audit_service.log(AuditLogEntry {
            event_type: "admin_list_users".to_string(),
            event_category: EventCategory::Admin,
            severity: Severity::Info,
            actor_user_id: Some(admin_user_id),
            actor_type: "user".to_string(),
            resource_type: Some("user".to_string()),
            action: "list_users".to_string(),
            action_result: ActionResult::Success,
            event_data: serde_json::json!({
                "total_users": total,
                "filters": {
                    "role": query.role,
                    "verified": query.verified,
                    "search": query.search.is_some(),
                },
            }),
            ip_address: None, // TODO: Extract from request
            is_pii_access: true,
            compliance_tags: vec!["admin".to_string(), "pii_access".to_string()],
            ..Default::default()
        }).await?;

        Ok(ListUsersResponse {
            users: user_responses,
            total,
            limit: query.limit.unwrap_or(50),
            offset: query.offset.unwrap_or(0),
        })
    }

    /// Get single user details
    ///
    /// # Security
    /// - Requires admin role
    /// - Logs PII access
    pub async fn get_user(
        &self,
        user_id: Uuid,
        admin_user_id: Uuid,
        ip_address: Option<String>,
    ) -> Result<UserResponse> {
        let user = self.user_repo
            .find_by_id(user_id)
            .await?
            .ok_or(AppError::NotFound("User not found".to_string()))?;

        // Audit log: Admin accessed specific user (PII access)
        self.audit_service.log(AuditLogEntry {
            event_type: "admin_view_user".to_string(),
            event_category: EventCategory::Admin,
            severity: Severity::Info,
            actor_user_id: Some(admin_user_id),
            actor_type: "user".to_string(),
            resource_type: Some("user".to_string()),
            resource_id: Some(user_id.to_string()),
            action: "view_user".to_string(),
            action_result: ActionResult::Success,
            event_data: serde_json::json!({
                "viewed_user_id": user_id,
                "viewed_user_email": user.email,
            }),
            ip_address: None,
            is_pii_access: true,
            compliance_tags: vec!["admin".to_string(), "pii_access".to_string()],
            ..Default::default()
        }).await?;

        Ok(user.into())
    }

    /// Verify or unverify a user
    ///
    /// # Security
    /// - Requires admin role
    /// - Comprehensive audit logging
    /// - Validates user exists
    pub async fn verify_user(
        &self,
        user_id: Uuid,
        request: VerifyUserRequest,
        admin_user_id: Uuid,
        admin_email: String,
        ip_address: Option<String>,
    ) -> Result<UserResponse> {
        // Fetch user first to validate exists
        let original_user = self.user_repo
            .find_by_id(user_id)
            .await?
            .ok_or(AppError::NotFound("User not found".to_string()))?;

        // Update verification status
        let updated_user = self.user_repo.set_verified(user_id, request.verified).await?;

        // Audit log: Admin changed verification status
        self.audit_service.log(AuditLogEntry {
            event_type: "admin_verify_user".to_string(),
            event_category: EventCategory::Admin,
            severity: Severity::Warning,
            actor_user_id: Some(admin_user_id),
            actor_type: "user".to_string(),
            resource_type: Some("user".to_string()),
            resource_id: Some(user_id.to_string()),
            action: if request.verified { "verify_user".to_string() } else { "unverify_user".to_string() },
            action_result: ActionResult::Success,
            event_data: serde_json::json!({
                "user_id": user_id,
                "user_email": original_user.email,
                "company_name": original_user.company_name,
                "previous_status": original_user.is_verified,
                "new_status": request.verified,
                "admin_email": admin_email,
                "notes": request.notes,
            }),
            ip_address: None,
            is_pii_access: false,
            compliance_tags: vec!["admin".to_string(), "verification".to_string(), "compliance".to_string()],
            ..Default::default()
        }).await?;

        tracing::info!(
            "User {} ({}) verification set to {} by admin {} ({})",
            user_id,
            original_user.email,
            request.verified,
            admin_user_id,
            admin_email
        );

        Ok(updated_user.into())
    }

    /// Change user role (superadmin only)
    ///
    /// # Security
    /// - Requires superadmin role (enforced in handler)
    /// - Prevents last superadmin demotion (DB constraint)
    /// - Comprehensive audit logging
    pub async fn change_user_role(
        &self,
        user_id: Uuid,
        request: ChangeUserRoleRequest,
        admin_user_id: Uuid,
        admin_email: String,
        ip_address: Option<String>,
    ) -> Result<UserResponse> {
        // Parse and validate role
        let new_role = self.parse_role(&request.role)?;

        // Fetch user to get original role
        let original_user = self.user_repo
            .find_by_id(user_id)
            .await?
            .ok_or(AppError::NotFound("User not found".to_string()))?;

        // Update role
        let updated_user = self.user_repo.set_role(user_id, new_role.clone(), admin_user_id).await?;

        // Audit log: Superadmin changed user role (critical operation)
        self.audit_service.log(AuditLogEntry {
            event_type: "admin_change_role".to_string(),
            event_category: EventCategory::Admin,
            severity: Severity::Critical,
            actor_user_id: Some(admin_user_id),
            actor_type: "user".to_string(),
            resource_type: Some("user".to_string()),
            resource_id: Some(user_id.to_string()),
            action: "change_user_role".to_string(),
            action_result: ActionResult::Success,
            event_data: serde_json::json!({
                "user_id": user_id,
                "user_email": original_user.email,
                "previous_role": format!("{:?}", original_user.role),
                "new_role": request.role,
                "admin_email": admin_email,
            }),
            ip_address: None,
            is_pii_access: false,
            compliance_tags: vec!["admin".to_string(), "security".to_string(), "role_change".to_string()],
            ..Default::default()
        }).await?;

        tracing::warn!(
            "User {} ({}) role changed from {:?} to {} by superadmin {} ({})",
            user_id,
            original_user.email,
            original_user.role,
            request.role,
            admin_user_id,
            admin_email
        );

        Ok(updated_user.into())
    }

    /// Delete user (superadmin only)
    ///
    /// # Security
    /// - Requires superadmin role
    /// - Prevents deletion of last superadmin (DB constraint)
    /// - Irreversible operation - comprehensive audit logging
    pub async fn delete_user(
        &self,
        user_id: Uuid,
        admin_user_id: Uuid,
        admin_email: String,
        ip_address: Option<String>,
    ) -> Result<()> {
        // Fetch user first for audit trail
        let user = self.user_repo
            .find_by_id(user_id)
            .await?
            .ok_or(AppError::NotFound("User not found".to_string()))?;

        // Delete user
        self.user_repo.delete(user_id).await?;

        // Audit log: Superadmin deleted user (critical operation)
        self.audit_service.log(AuditLogEntry {
            event_type: "admin_delete_user".to_string(),
            event_category: EventCategory::Admin,
            severity: Severity::Critical,
            actor_user_id: Some(admin_user_id),
            actor_type: "user".to_string(),
            resource_type: Some("user".to_string()),
            resource_id: Some(user_id.to_string()),
            action: "delete_user".to_string(),
            action_result: ActionResult::Success,
            event_data: serde_json::json!({
                "deleted_user_id": user_id,
                "deleted_user_email": user.email,
                "deleted_user_company": user.company_name,
                "deleted_user_role": format!("{:?}", user.role),
                "admin_email": admin_email,
            }),
            ip_address: None,
            is_pii_access: false,
            compliance_tags: vec!["admin".to_string(), "security".to_string(), "user_deletion".to_string()],
            ..Default::default()
        }).await?;

        tracing::warn!(
            "User {} ({}) DELETED by superadmin {} ({})",
            user_id,
            user.email,
            admin_user_id,
            admin_email
        );

        Ok(())
    }

    // ========================================================================
    // VERIFICATION QUEUE
    // ========================================================================

    /// Get pending verification queue
    ///
    /// # Returns
    /// List of unverified users with context (inventory count, transaction count, waiting time)
    pub async fn get_verification_queue(
        &self,
        admin_user_id: Uuid,
        ip_address: Option<String>,
    ) -> Result<Vec<VerificationQueueItem>> {
        let pending_users = self.user_repo.get_verification_queue().await?;

        // For now, return basic queue items (in production, would join with inventory/transactions)
        let queue_items: Vec<VerificationQueueItem> = pending_users.into_iter().map(|user| {
            let days_waiting = (Utc::now() - user.created_at).num_days();
            VerificationQueueItem {
                user: user.into(),
                inventory_count: 0, // TODO: Join with inventory table
                transaction_count: 0, // TODO: Join with transactions table
                days_waiting,
            }
        }).collect();

        // Audit log: Admin viewed verification queue
        self.audit_service.log(AuditLogEntry {
            event_type: "admin_view_verification_queue".to_string(),
            event_category: EventCategory::Admin,
            severity: Severity::Info,
            actor_user_id: Some(admin_user_id),
            actor_type: "user".to_string(),
            resource_type: Some("verification_queue".to_string()),
            action: "view_queue".to_string(),
            action_result: ActionResult::Success,
            event_data: serde_json::json!({
                "pending_count": queue_items.len(),
            }),
            ip_address: None,
            is_pii_access: false,
            compliance_tags: vec!["admin".to_string(), "verification".to_string()],
            ..Default::default()
        }).await?;

        Ok(queue_items)
    }

    // ========================================================================
    // STATISTICS & ANALYTICS
    // ========================================================================

    /// Get admin dashboard statistics
    ///
    /// # Returns
    /// Comprehensive system statistics for admin dashboard
    pub async fn get_admin_stats(
        &self,
        admin_user_id: Uuid,
        pool: &sqlx::PgPool,
    ) -> Result<AdminStatsResponse> {
        use sqlx::query;

        // Get user counts
        let total_users = self.user_repo.count_users(None, None).await?;
        let verified_users = self.user_repo.count_users(None, Some(true)).await?;
        let pending_verifications = self.user_repo.count_users(Some(UserRole::User), Some(false)).await?;
        let total_admins = self.user_repo.count_users(Some(UserRole::Admin), None).await? +
                           self.user_repo.count_users(Some(UserRole::Superadmin), None).await?;

        // Get inventory count
        let inventory_row = query("SELECT COUNT(*) as count FROM inventory")
            .fetch_one(pool)
            .await?;
        let total_inventory_items: i64 = inventory_row.try_get("count")?;

        // Get transaction count
        let transaction_row = query("SELECT COUNT(*) as count FROM transactions")
            .fetch_one(pool)
            .await?;
        let total_transactions: i64 = transaction_row.try_get("count")?;

        // Get recent signups (last 7 days)
        let recent_users = query(
            r#"
            SELECT id, email, company_name, is_verified, created_at
            FROM users
            WHERE created_at >= NOW() - INTERVAL '7 days'
            ORDER BY created_at DESC
            LIMIT 10
            "#
        )
        .fetch_all(pool)
        .await?;

        let recent_signups: Vec<RecentSignup> = recent_users.iter().map(|row| {
            RecentSignup {
                id: row.try_get::<Uuid, _>("id").unwrap().to_string(),
                email: row.try_get("email").unwrap(),
                company_name: row.try_get("company_name").unwrap(),
                created_at: row.try_get("created_at").unwrap(),
                is_verified: row.try_get("is_verified").unwrap(),
            }
        }).collect();

        // System health
        let system_health = SystemHealth {
            database_connected: true,
            uptime_seconds: 0, // TODO: Track server uptime
            total_api_calls_today: 0, // TODO: Track from audit logs
        };

        Ok(AdminStatsResponse {
            total_users,
            verified_users,
            pending_verifications,
            total_admins,
            total_inventory_items,
            total_transactions,
            recent_signups,
            system_health,
        })
    }

    // ========================================================================
    // AUDIT LOGS
    // ========================================================================

    /// Get audit logs with filters
    ///
    /// # Security
    /// - Requires admin role
    /// - Read-only access to audit trail
    pub async fn get_audit_logs(
        &self,
        audit_query: AuditLogQuery,
        pool: &sqlx::PgPool,
    ) -> Result<Vec<AuditLogResponse>> {
        use sqlx::query;

        let limit = audit_query.limit.unwrap_or(50).min(100);
        let offset = audit_query.offset.unwrap_or(0);

        // 🔒 SECURITY: Use static query with optional filters instead of dynamic query building
        // This prevents SQL injection risks and makes the query easier to audit

        // Parse and validate user_id if provided
        let user_id_filter = if let Some(ref user_id_str) = audit_query.user_id {
            Some(Uuid::parse_str(user_id_str)
                .map_err(|_| AppError::BadRequest("Invalid user ID format".to_string()))?)
        } else {
            None
        };

        // Build query using COALESCE for optional filters
        let rows = query(
            r#"
            SELECT id, event_type, event_category, severity, actor_user_id,
                   action, action_result, event_data, ip_address, created_at
            FROM audit_logs
            WHERE ($1::uuid IS NULL OR actor_user_id = $1)
              AND ($2::text IS NULL OR event_category = $2)
              AND ($3::timestamptz IS NULL OR created_at >= $3)
              AND ($4::timestamptz IS NULL OR created_at <= $4)
            ORDER BY created_at DESC
            LIMIT $5 OFFSET $6
            "#
        )
        .bind(user_id_filter)
        .bind(&audit_query.event_category)
        .bind(audit_query.start_date)
        .bind(audit_query.end_date)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        // 🔒 SECURITY: Remove unsafe .unwrap() calls - use proper error handling
        let logs: Result<Vec<AuditLogResponse>> = rows.iter().map(|row| {
            Ok(AuditLogResponse {
                id: row.try_get::<i64, _>("id")
                    .map_err(|e| AppError::Internal(anyhow!("Failed to get id: {:?}", e)))?
                    .to_string(),
                event_type: row.try_get("event_type")
                    .map_err(|e| AppError::Internal(anyhow!("Failed to get event_type: {:?}", e)))?,
                event_category: row.try_get("event_category")
                    .map_err(|e| AppError::Internal(anyhow!("Failed to get event_category: {:?}", e)))?,
                severity: row.try_get("severity")
                    .map_err(|e| AppError::Internal(anyhow!("Failed to get severity: {:?}", e)))?,
                actor_user_id: row.try_get::<Option<Uuid>, _>("actor_user_id")
                    .map_err(|e| AppError::Internal(anyhow!("Failed to get actor_user_id: {:?}", e)))?
                    .map(|u| u.to_string()),
                action: row.try_get("action")
                    .map_err(|e| AppError::Internal(anyhow!("Failed to get action: {:?}", e)))?,
                action_result: row.try_get("action_result")
                    .map_err(|e| AppError::Internal(anyhow!("Failed to get action_result: {:?}", e)))?,
                event_data: row.try_get("event_data")
                    .map_err(|e| AppError::Internal(anyhow!("Failed to get event_data: {:?}", e)))?,
                ip_address: row.try_get::<Option<std::net::IpAddr>, _>("ip_address")
                    .map_err(|e| AppError::Internal(anyhow!("Failed to get ip_address: {:?}", e)))?
                    .map(|ip| ip.to_string()),
                created_at: row.try_get("created_at")
                    .map_err(|e| AppError::Internal(anyhow!("Failed to get created_at: {:?}", e)))?,
            })
        }).collect();

        logs
    }

    // ========================================================================
    // HELPER METHODS
    // ========================================================================

    fn parse_role(&self, role_str: &str) -> Result<UserRole> {
        match role_str.to_lowercase().as_str() {
            "user" => Ok(UserRole::User),
            "admin" => Ok(UserRole::Admin),
            "superadmin" => Ok(UserRole::Superadmin),
            _ => Err(AppError::BadRequest(format!("Invalid role: {}", role_str))),
        }
    }
}
