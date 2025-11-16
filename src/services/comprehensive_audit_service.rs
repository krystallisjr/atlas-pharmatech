/// Production-Grade Comprehensive Audit Logging System
/// Compliance: SOC 2, HIPAA, ISO 27001, GDPR
///
/// This service provides immutable, tamper-proof audit logging for:
/// - Authentication events (login, logout, failures)
/// - Authorization events (access denied, permission changes)
/// - Data access and modifications (who accessed what, when)
/// - Security events (rate limiting, blacklisting, suspicious activity)
/// - System events (errors, configuration changes)

use sqlx::PgPool;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use serde_json::Value as JsonValue;
use std::net::IpAddr;
use crate::middleware::error_handling::Result;

#[derive(Debug, Clone)]
pub struct ComprehensiveAuditService {
    db_pool: PgPool,
}

/// Event categories for audit logs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventCategory {
    Auth,             // Authentication events
    DataAccess,       // Reading data
    DataModification, // Creating/updating/deleting data
    Security,         // Security-related events
    System,           // System events
    Admin,            // Administrative actions
}

impl EventCategory {
    fn as_str(&self) -> &str {
        match self {
            EventCategory::Auth => "auth",
            EventCategory::DataAccess => "data_access",
            EventCategory::DataModification => "data_modification",
            EventCategory::Security => "security",
            EventCategory::System => "system",
            EventCategory::Admin => "admin",
        }
    }
}

/// Event severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

impl Severity {
    fn as_str(&self) -> &str {
        match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
            Severity::Critical => "critical",
        }
    }
}

/// Action results
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionResult {
    Success,
    Failure,
    Partial,
}

impl ActionResult {
    fn as_str(&self) -> &str {
        match self {
            ActionResult::Success => "success",
            ActionResult::Failure => "failure",
            ActionResult::Partial => "partial",
        }
    }
}

/// Audit log entry builder
#[derive(Debug, Clone)]
pub struct AuditLogEntry {
    // Event identification
    pub event_type: String,
    pub event_category: EventCategory,
    pub severity: Severity,

    // Actor (who performed the action)
    pub actor_user_id: Option<Uuid>,
    pub actor_type: String,
    pub actor_identifier: Option<String>,

    // Target (what was affected)
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub resource_name: Option<String>,

    // Action
    pub action: String,
    pub action_result: ActionResult,

    // Details
    pub event_data: JsonValue,
    pub changes_summary: Option<String>,
    pub old_values: Option<JsonValue>,
    pub new_values: Option<JsonValue>,

    // Request metadata
    pub ip_address: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
    pub session_id: Option<String>,

    // Compliance
    pub is_pii_access: bool,
    pub compliance_tags: Vec<String>,
}

impl Default for AuditLogEntry {
    fn default() -> Self {
        Self {
            event_type: String::new(),
            event_category: EventCategory::System,
            severity: Severity::Info,
            actor_user_id: None,
            actor_type: "system".to_string(),
            actor_identifier: None,
            resource_type: None,
            resource_id: None,
            resource_name: None,
            action: String::new(),
            action_result: ActionResult::Success,
            event_data: serde_json::json!({}),
            changes_summary: None,
            old_values: None,
            new_values: None,
            ip_address: None,
            user_agent: None,
            request_id: None,
            session_id: None,
            is_pii_access: false,
            compliance_tags: vec![],
        }
    }
}

impl ComprehensiveAuditService {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    /// Log an audit event
    pub async fn log(&self, entry: AuditLogEntry) -> Result<i64> {
        let compliance_tags: Vec<&str> = entry.compliance_tags.iter().map(|s| s.as_str()).collect();
        let ip_str = entry.ip_address.map(|ip| ip.to_string());

        let record = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO audit_logs (
                event_type,
                event_category,
                severity,
                actor_user_id,
                actor_type,
                actor_identifier,
                resource_type,
                resource_id,
                resource_name,
                action,
                action_result,
                event_data,
                changes_summary,
                old_values,
                new_values,
                ip_address,
                user_agent,
                request_id,
                session_id,
                is_pii_access,
                compliance_tags
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16::inet, $17, $18, $19, $20, $21
            )
            RETURNING id
            "#
        )
        .bind(&entry.event_type)
        .bind(entry.event_category.as_str())
        .bind(entry.severity.as_str())
        .bind(entry.actor_user_id)
        .bind(&entry.actor_type)
        .bind(&entry.actor_identifier)
        .bind(&entry.resource_type)
        .bind(&entry.resource_id)
        .bind(&entry.resource_name)
        .bind(&entry.action)
        .bind(entry.action_result.as_str())
        .bind(&entry.event_data)
        .bind(&entry.changes_summary)
        .bind(&entry.old_values)
        .bind(&entry.new_values)
        .bind(ip_str.as_deref())
        .bind(&entry.user_agent)
        .bind(&entry.request_id)
        .bind(&entry.session_id)
        .bind(entry.is_pii_access)
        .bind(&compliance_tags)
        .fetch_one(&self.db_pool)
        .await?;

        // Log to console for real-time monitoring
        let emoji = match entry.severity {
            Severity::Info => "ℹ️",
            Severity::Warning => "⚠️",
            Severity::Error => "❌",
            Severity::Critical => "🚨",
        };

        tracing::info!(
            "{} AUDIT [{}]: {} {} by {} (result: {})",
            emoji,
            entry.event_category.as_str(),
            entry.action,
            entry.event_type,
            entry.actor_identifier.as_deref().unwrap_or("system"),
            entry.action_result.as_str()
        );

        Ok(record)
    }

    // ========================================================================
    // AUTHENTICATION EVENTS
    // ========================================================================

    pub async fn log_login_success(
        &self,
        user_id: Uuid,
        email: &str,
        ip_address: Option<IpAddr>,
        user_agent: Option<String>,
    ) -> Result<i64> {
        self.log(AuditLogEntry {
            event_type: "login_success".to_string(),
            event_category: EventCategory::Auth,
            severity: Severity::Info,
            actor_user_id: Some(user_id),
            actor_type: "user".to_string(),
            actor_identifier: Some(email.to_string()),
            action: "login".to_string(),
            action_result: ActionResult::Success,
            event_data: serde_json::json!({
                "login_method": "password",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
            ip_address,
            user_agent,
            ..Default::default()
        }).await
    }

    pub async fn log_login_failed(
        &self,
        email: &str,
        reason: &str,
        ip_address: Option<IpAddr>,
        user_agent: Option<String>,
    ) -> Result<i64> {
        self.log(AuditLogEntry {
            event_type: "login_failed".to_string(),
            event_category: EventCategory::Security,
            severity: Severity::Warning,
            actor_type: "user".to_string(),
            actor_identifier: Some(email.to_string()),
            action: "login".to_string(),
            action_result: ActionResult::Failure,
            event_data: serde_json::json!({
                "reason": reason,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
            ip_address,
            user_agent,
            ..Default::default()
        }).await
    }

    pub async fn log_logout(
        &self,
        user_id: Uuid,
        email: &str,
        ip_address: Option<IpAddr>,
    ) -> Result<i64> {
        self.log(AuditLogEntry {
            event_type: "logout".to_string(),
            event_category: EventCategory::Auth,
            severity: Severity::Info,
            actor_user_id: Some(user_id),
            actor_type: "user".to_string(),
            actor_identifier: Some(email.to_string()),
            action: "logout".to_string(),
            action_result: ActionResult::Success,
            event_data: serde_json::json!({
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
            ip_address,
            ..Default::default()
        }).await
    }

    pub async fn log_token_blacklisted(
        &self,
        user_id: Uuid,
        email: &str,
        jti: &str,
        reason: &str,
    ) -> Result<i64> {
        self.log(AuditLogEntry {
            event_type: "token_blacklisted".to_string(),
            event_category: EventCategory::Security,
            severity: Severity::Info,
            actor_user_id: Some(user_id),
            actor_type: "user".to_string(),
            actor_identifier: Some(email.to_string()),
            action: "blacklist".to_string(),
            action_result: ActionResult::Success,
            event_data: serde_json::json!({
                "jti": jti,
                "reason": reason,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
            ..Default::default()
        }).await
    }

    // ========================================================================
    // SECURITY EVENTS
    // ========================================================================

    pub async fn log_rate_limit_exceeded(
        &self,
        ip_address: IpAddr,
        endpoint: &str,
        limit: u32,
    ) -> Result<i64> {
        self.log(AuditLogEntry {
            event_type: "rate_limit_exceeded".to_string(),
            event_category: EventCategory::Security,
            severity: Severity::Warning,
            actor_type: "system".to_string(),
            action: "rate_limit".to_string(),
            action_result: ActionResult::Success,
            event_data: serde_json::json!({
                "endpoint": endpoint,
                "limit": limit,
                "blocked": true
            }),
            ip_address: Some(ip_address),
            ..Default::default()
        }).await
    }

    pub async fn log_unauthorized_access_attempt(
        &self,
        user_id: Option<Uuid>,
        email: Option<&str>,
        resource_type: &str,
        resource_id: &str,
        ip_address: Option<IpAddr>,
    ) -> Result<i64> {
        self.log(AuditLogEntry {
            event_type: "unauthorized_access".to_string(),
            event_category: EventCategory::Security,
            severity: Severity::Error,
            actor_user_id: user_id,
            actor_type: "user".to_string(),
            actor_identifier: email.map(|s| s.to_string()),
            resource_type: Some(resource_type.to_string()),
            resource_id: Some(resource_id.to_string()),
            action: "access".to_string(),
            action_result: ActionResult::Failure,
            event_data: serde_json::json!({
                "reason": "insufficient_permissions"
            }),
            ip_address,
            ..Default::default()
        }).await
    }

    // ========================================================================
    // DATA ACCESS EVENTS (for compliance)
    // ========================================================================

    pub async fn log_pii_access(
        &self,
        user_id: Uuid,
        email: &str,
        resource_type: &str,
        resource_id: &str,
        pii_fields: Vec<String>,
        ip_address: Option<IpAddr>,
    ) -> Result<i64> {
        self.log(AuditLogEntry {
            event_type: "pii_accessed".to_string(),
            event_category: EventCategory::DataAccess,
            severity: Severity::Info,
            actor_user_id: Some(user_id),
            actor_type: "user".to_string(),
            actor_identifier: Some(email.to_string()),
            resource_type: Some(resource_type.to_string()),
            resource_id: Some(resource_id.to_string()),
            action: "read".to_string(),
            action_result: ActionResult::Success,
            event_data: serde_json::json!({
                "pii_fields_accessed": pii_fields,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
            ip_address,
            is_pii_access: true,
            compliance_tags: vec!["gdpr".to_string(), "hipaa".to_string()],
            ..Default::default()
        }).await
    }

    pub async fn log_inventory_access(
        &self,
        user_id: Uuid,
        email: &str,
        inventory_id: Uuid,
        product_name: &str,
        action: &str,
    ) -> Result<i64> {
        self.log(AuditLogEntry {
            event_type: "inventory_accessed".to_string(),
            event_category: EventCategory::DataAccess,
            severity: Severity::Info,
            actor_user_id: Some(user_id),
            actor_type: "user".to_string(),
            actor_identifier: Some(email.to_string()),
            resource_type: Some("inventory".to_string()),
            resource_id: Some(inventory_id.to_string()),
            resource_name: Some(product_name.to_string()),
            action: action.to_string(),
            action_result: ActionResult::Success,
            event_data: serde_json::json!({
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
            ..Default::default()
        }).await
    }

    // ========================================================================
    // DATA MODIFICATION EVENTS
    // ========================================================================

    pub async fn log_data_created(
        &self,
        user_id: Uuid,
        email: &str,
        resource_type: &str,
        resource_id: &str,
        resource_name: Option<&str>,
        new_values: JsonValue,
    ) -> Result<i64> {
        self.log(AuditLogEntry {
            event_type: format!("{}_created", resource_type),
            event_category: EventCategory::DataModification,
            severity: Severity::Info,
            actor_user_id: Some(user_id),
            actor_type: "user".to_string(),
            actor_identifier: Some(email.to_string()),
            resource_type: Some(resource_type.to_string()),
            resource_id: Some(resource_id.to_string()),
            resource_name: resource_name.map(|s| s.to_string()),
            action: "create".to_string(),
            action_result: ActionResult::Success,
            new_values: Some(new_values),
            changes_summary: Some(format!("Created new {}", resource_type)),
            ..Default::default()
        }).await
    }

    pub async fn log_data_updated(
        &self,
        user_id: Uuid,
        email: &str,
        resource_type: &str,
        resource_id: &str,
        resource_name: Option<&str>,
        old_values: JsonValue,
        new_values: JsonValue,
        changes_summary: &str,
    ) -> Result<i64> {
        self.log(AuditLogEntry {
            event_type: format!("{}_updated", resource_type),
            event_category: EventCategory::DataModification,
            severity: Severity::Info,
            actor_user_id: Some(user_id),
            actor_type: "user".to_string(),
            actor_identifier: Some(email.to_string()),
            resource_type: Some(resource_type.to_string()),
            resource_id: Some(resource_id.to_string()),
            resource_name: resource_name.map(|s| s.to_string()),
            action: "update".to_string(),
            action_result: ActionResult::Success,
            old_values: Some(old_values),
            new_values: Some(new_values),
            changes_summary: Some(changes_summary.to_string()),
            ..Default::default()
        }).await
    }

    pub async fn log_data_deleted(
        &self,
        user_id: Uuid,
        email: &str,
        resource_type: &str,
        resource_id: &str,
        resource_name: Option<&str>,
        old_values: JsonValue,
    ) -> Result<i64> {
        self.log(AuditLogEntry {
            event_type: format!("{}_deleted", resource_type),
            event_category: EventCategory::DataModification,
            severity: Severity::Warning,
            actor_user_id: Some(user_id),
            actor_type: "user".to_string(),
            actor_identifier: Some(email.to_string()),
            resource_type: Some(resource_type.to_string()),
            resource_id: Some(resource_id.to_string()),
            resource_name: resource_name.map(|s| s.to_string()),
            action: "delete".to_string(),
            action_result: ActionResult::Success,
            old_values: Some(old_values),
            changes_summary: Some(format!("Deleted {}", resource_type)),
            ..Default::default()
        }).await
    }

    // ========================================================================
    // FILE OPERATIONS (for encrypted file storage)
    // ========================================================================

    pub async fn log_file_uploaded(
        &self,
        user_id: Uuid,
        email: &str,
        filename: &str,
        file_size: usize,
        file_hash: &str,
        encrypted: bool,
        ip_address: Option<IpAddr>,
    ) -> Result<i64> {
        self.log(AuditLogEntry {
            event_type: "file_uploaded".to_string(),
            event_category: EventCategory::DataModification,
            severity: Severity::Info,
            actor_user_id: Some(user_id),
            actor_type: "user".to_string(),
            actor_identifier: Some(email.to_string()),
            resource_type: Some("file".to_string()),
            resource_name: Some(filename.to_string()),
            action: "upload".to_string(),
            action_result: ActionResult::Success,
            event_data: serde_json::json!({
                "filename": filename,
                "file_size_bytes": file_size,
                "file_hash": file_hash,
                "encrypted": encrypted,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
            ip_address,
            ..Default::default()
        }).await
    }

    pub async fn log_file_downloaded(
        &self,
        user_id: Uuid,
        email: &str,
        filename: &str,
        encrypted: bool,
        ip_address: Option<IpAddr>,
    ) -> Result<i64> {
        self.log(AuditLogEntry {
            event_type: "file_downloaded".to_string(),
            event_category: EventCategory::DataAccess,
            severity: Severity::Info,
            actor_user_id: Some(user_id),
            actor_type: "user".to_string(),
            actor_identifier: Some(email.to_string()),
            resource_type: Some("file".to_string()),
            resource_name: Some(filename.to_string()),
            action: "download".to_string(),
            action_result: ActionResult::Success,
            event_data: serde_json::json!({
                "filename": filename,
                "encrypted": encrypted,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
            ip_address,
            ..Default::default()
        }).await
    }

    // ========================================================================
    // SYSTEM EVENTS
    // ========================================================================

    pub async fn log_system_error(
        &self,
        error_type: &str,
        error_message: &str,
        context: JsonValue,
    ) -> Result<i64> {
        self.log(AuditLogEntry {
            event_type: "system_error".to_string(),
            event_category: EventCategory::System,
            severity: Severity::Error,
            actor_type: "system".to_string(),
            action: "error".to_string(),
            action_result: ActionResult::Failure,
            event_data: serde_json::json!({
                "error_type": error_type,
                "error_message": error_message,
                "context": context,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
            ..Default::default()
        }).await
    }
}
