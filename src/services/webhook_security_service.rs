use crate::middleware::error_handling::{AppError, Result};
use crate::services::encryption_service::EncryptionService;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use sqlx::PgPool;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// Webhook security service for signature verification and rate limiting
pub struct WebhookSecurityService {
    pool: PgPool,
    encryption_service: EncryptionService,
}

#[derive(Debug, Clone)]
pub struct WebhookVerificationResult {
    pub connection_id: Uuid,
    pub signature_valid: bool,
    pub rate_limit_allowed: bool,
    pub requests_remaining: i32,
    pub blocked: bool,
}

#[derive(Debug)]
pub struct WebhookAuditLog {
    pub connection_id: Uuid,
    pub event_type: String,
    pub request_id: Uuid,
    pub source_ip: Option<String>,
    pub signature_valid: bool,
    pub payload_size_bytes: i32,
    pub http_status: i32,
    pub error_message: Option<String>,
    pub processing_time_ms: Option<i32>,
}

impl WebhookSecurityService {
    pub fn new(pool: PgPool) -> Result<Self> {
        let encryption_key = std::env::var("ENCRYPTION_KEY")
            .map_err(|_| AppError::Internal(anyhow::anyhow!("ENCRYPTION_KEY not set")))?;

        let encryption_service = EncryptionService::new(&encryption_key)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to init encryption: {:?}", e)))?;

        Ok(Self {
            pool,
            encryption_service,
        })
    }

    /// Verify webhook HMAC signature
    ///
    /// Signature format: HMAC-SHA256(secret, payload)
    /// Header: X-Webhook-Signature: sha256=<hex_signature>
    pub async fn verify_signature(
        &self,
        connection_id: Uuid,
        payload: &[u8],
        signature_header: &str,
    ) -> Result<bool> {
        // Get encrypted webhook secret from database
        let secret_encrypted: Option<String> = sqlx::query_scalar(
            "SELECT webhook_secret_encrypted FROM erp_connections WHERE id = $1 AND webhook_enabled = TRUE"
        )
        .bind(connection_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Webhook not configured for this connection".to_string()))?;

        let secret_encrypted = secret_encrypted
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Webhook secret not set")))?;

        // Decrypt webhook secret
        let secret = self.encryption_service.decrypt(&secret_encrypted)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to decrypt webhook secret: {:?}", e)))?;

        // Parse signature header (format: "sha256=<hex>")
        let signature_hex = signature_header
            .strip_prefix("sha256=")
            .ok_or_else(|| AppError::BadRequest("Invalid signature format. Expected: sha256=<hex>".to_string()))?;

        let expected_signature = hex::decode(signature_hex)
            .map_err(|_| AppError::BadRequest("Invalid signature encoding".to_string()))?;

        // Compute HMAC
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| AppError::Internal(anyhow::anyhow!("HMAC init failed: {:?}", e)))?;

        mac.update(payload);

        // Constant-time comparison
        Ok(mac.verify_slice(&expected_signature).is_ok())
    }

    /// Check rate limit for webhook connection
    pub async fn check_rate_limit(&self, connection_id: Uuid) -> Result<WebhookVerificationResult> {
        #[derive(sqlx::FromRow)]
        struct RateLimitResult {
            allowed: bool,
            requests_remaining: i32,
            reset_at: DateTime<Utc>,
            blocked: bool,
        }

        let result = sqlx::query_as::<_, RateLimitResult>(
            "SELECT * FROM check_webhook_rate_limit($1)"
        )
        .bind(connection_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(WebhookVerificationResult {
            connection_id,
            signature_valid: false, // Will be set by caller after signature verification
            rate_limit_allowed: result.allowed,
            requests_remaining: result.requests_remaining,
            blocked: result.blocked,
        })
    }

    /// Log webhook request attempt (success or failure)
    pub async fn log_webhook_attempt(&self, log: WebhookAuditLog) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO webhook_audit_log (
                connection_id, event_type, request_id, source_ip, signature_valid,
                payload_size_bytes, http_status, error_message, processing_time_ms
            ) VALUES ($1, $2, $3, $4::inet, $5, $6, $7, $8, $9)
            "#
        )
        .bind(log.connection_id)
        .bind(&log.event_type)
        .bind(log.request_id)
        .bind(log.source_ip)
        .bind(log.signature_valid)
        .bind(log.payload_size_bytes)
        .bind(log.http_status)
        .bind(log.error_message)
        .bind(log.processing_time_ms)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Generate new webhook secret for a connection
    pub async fn generate_webhook_secret(&self, connection_id: Uuid) -> Result<String> {
        // Generate cryptographically secure random secret (32 bytes, base64 encoded)
        let secret: String = sqlx::query_scalar("SELECT generate_webhook_secret()")
            .fetch_one(&self.pool)
            .await?;

        // Encrypt the secret before storing
        let secret_encrypted = self.encryption_service.encrypt(&secret)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to encrypt webhook secret: {:?}", e)))?;

        // Update connection with encrypted secret
        sqlx::query(
            "UPDATE erp_connections SET webhook_secret_encrypted = $1, webhook_enabled = TRUE WHERE id = $2"
        )
        .bind(&secret_encrypted)
        .bind(connection_id)
        .execute(&self.pool)
        .await?;

        // Return plaintext secret ONCE (for user to configure in their ERP system)
        Ok(secret)
    }

    /// Validate connection exists and webhooks are enabled
    pub async fn validate_connection(&self, connection_id: Uuid) -> Result<()> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM erp_connections WHERE id = $1 AND webhook_enabled = TRUE)"
        )
        .bind(connection_id)
        .fetch_one(&self.pool)
        .await?;

        if !exists {
            return Err(AppError::NotFound(
                "Connection not found or webhooks not enabled".to_string()
            ));
        }

        Ok(())
    }

    /// Get failed webhook attempts (for monitoring)
    pub async fn get_failed_attempts(
        &self,
        connection_id: Option<Uuid>,
        hours: i32,
    ) -> Result<i64> {
        let count: i64 = if let Some(conn_id) = connection_id {
            sqlx::query_scalar(
                r#"
                SELECT COUNT(*) FROM webhook_audit_log
                WHERE connection_id = $1
                AND signature_valid = FALSE
                AND created_at > NOW() - ($2 || ' hours')::INTERVAL
                "#
            )
            .bind(conn_id)
            .bind(hours)
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query_scalar(
                r#"
                SELECT COUNT(*) FROM webhook_audit_log
                WHERE signature_valid = FALSE
                AND created_at > NOW() - ($1 || ' hours')::INTERVAL
                "#
            )
            .bind(hours)
            .fetch_one(&self.pool)
            .await?
        };

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_signature_generation() {
        let secret = "test_secret_key_123";
        let payload = b"test payload data";

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload);
        let signature = mac.finalize().into_bytes();

        let signature_hex = hex::encode(signature);

        // Verify format
        assert_eq!(signature_hex.len(), 64); // SHA256 = 32 bytes = 64 hex chars
    }

    #[test]
    fn test_signature_header_parsing() {
        let header = "sha256=abcdef1234567890";
        let result = header.strip_prefix("sha256=");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "abcdef1234567890");
    }
}
