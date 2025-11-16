/// Production-grade audit logging for AI import system
use uuid::Uuid;
use sqlx::PgPool;
use serde_json::Value as JsonValue;
use crate::middleware::error_handling::Result;

pub struct AuditService {
    db_pool: PgPool,
}

impl AuditService {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    /// Log file upload event
    pub async fn log_file_uploaded(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        filename: &str,
        file_size: i64,
        ip_address: Option<String>,
    ) -> Result<()> {
        self.log_event(
            session_id,
            user_id,
            "file_uploaded",
            serde_json::json!({
                "filename": filename,
                "file_size_bytes": file_size,
                "action": "User uploaded file for AI analysis"
            }),
            ip_address,
        ).await
    }

    /// Log AI analysis started
    pub async fn log_analysis_started(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        file_type: &str,
        row_count: usize,
    ) -> Result<()> {
        self.log_event(
            session_id,
            user_id,
            "analysis_started",
            serde_json::json!({
                "file_type": file_type,
                "total_rows": row_count,
                "action": "Claude AI analysis initiated"
            }),
            None,
        ).await
    }

    /// Log AI analysis completed
    pub async fn log_analysis_completed(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        confidence_score: Option<f64>,
        tokens_used: i32,
        cost_usd: f64,
    ) -> Result<()> {
        self.log_event(
            session_id,
            user_id,
            "analysis_completed",
            serde_json::json!({
                "confidence_score": confidence_score,
                "ai_tokens_used": tokens_used,
                "ai_cost_usd": cost_usd,
                "action": "Claude AI completed column mapping analysis"
            }),
            None,
        ).await
    }

    /// Log mapping approved by user
    pub async fn log_mapping_approved(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        had_overrides: bool,
    ) -> Result<()> {
        self.log_event(
            session_id,
            user_id,
            "mapping_approved",
            serde_json::json!({
                "user_modified_mapping": had_overrides,
                "action": "User approved column mapping and started import"
            }),
            None,
        ).await
    }

    /// Log import started
    pub async fn log_import_started(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        total_rows: i32,
    ) -> Result<()> {
        self.log_event(
            session_id,
            user_id,
            "import_started",
            serde_json::json!({
                "total_rows": total_rows,
                "action": "Batch import processing started"
            }),
            None,
        ).await
    }

    /// Log import batch processed
    pub async fn log_batch_processed(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        batch_number: usize,
        rows_in_batch: usize,
        rows_imported: usize,
        rows_failed: usize,
    ) -> Result<()> {
        self.log_event(
            session_id,
            user_id,
            "batch_processed",
            serde_json::json!({
                "batch_number": batch_number,
                "rows_in_batch": rows_in_batch,
                "rows_imported": rows_imported,
                "rows_failed": rows_failed,
                "action": format!("Batch {} processed", batch_number)
            }),
            None,
        ).await
    }

    /// Log import completed successfully
    pub async fn log_import_completed(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        total_rows: i32,
        rows_imported: i32,
        rows_failed: i32,
        rows_flagged: i32,
    ) -> Result<()> {
        self.log_event(
            session_id,
            user_id,
            "import_completed",
            serde_json::json!({
                "total_rows": total_rows,
                "rows_imported": rows_imported,
                "rows_failed": rows_failed,
                "rows_flagged_for_review": rows_flagged,
                "success_rate": if total_rows > 0 {
                    (rows_imported as f64 / total_rows as f64) * 100.0
                } else { 0.0 },
                "action": "Import completed successfully"
            }),
            None,
        ).await
    }

    /// Log import failed
    pub async fn log_import_failed(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        error_message: &str,
        rows_processed: i32,
    ) -> Result<()> {
        self.log_event(
            session_id,
            user_id,
            "import_failed",
            serde_json::json!({
                "error": error_message,
                "rows_processed_before_failure": rows_processed,
                "action": "Import failed with error"
            }),
            None,
        ).await
    }

    /// Log import cancelled by user
    pub async fn log_import_cancelled(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        reason: Option<&str>,
    ) -> Result<()> {
        self.log_event(
            session_id,
            user_id,
            "import_cancelled",
            serde_json::json!({
                "reason": reason,
                "action": "User cancelled import"
            }),
            None,
        ).await
    }

    /// Generic event logger - all specific logs call this
    async fn log_event(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        event_type: &str,
        event_data: JsonValue,
        ip_address: Option<String>,
    ) -> Result<()> {
        // Use query instead of query! to avoid INET type compile-time check
        sqlx::query(
            r#"
            INSERT INTO ai_import_audit_log (
                session_id,
                user_id,
                event_type,
                event_data,
                ip_address,
                created_at
            ) VALUES ($1, $2, $3, $4, $5::inet, NOW())
            "#
        )
        .bind(session_id)
        .bind(user_id)
        .bind(event_type)
        .bind(event_data)
        .bind(ip_address)
        .execute(&self.db_pool)
        .await?;

        tracing::info!(
            "Audit log: session={}, user={}, event={}",
            session_id,
            user_id,
            event_type
        );

        Ok(())
    }
}
