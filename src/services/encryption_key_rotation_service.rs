// ============================================================================
// Encryption Key Rotation Service - Production Key Management
// ============================================================================
//
// 🔒 SECURITY: Implements envelope encryption and key rotation strategy
//
// ## Problem:
// Storing encryption keys in environment variables is insecure because:
// - Keys accessible to anyone with server access
// - Keys logged in environment dumps
// - No rotation mechanism
// - Single key compromises all data
//
// ## Solution: Envelope Encryption
//
// **Architecture:**
// 1. Master Key (KEK - Key Encryption Key)
//    - Stored in environment or external KMS
//    - Used to encrypt Data Encryption Keys
//    - Rotated annually
//
// 2. Data Encryption Keys (DEKs)
//    - Generated per-tenant or per-purpose
//    - Encrypted by Master Key
//    - Stored in database (encrypted)
//    - Can be rotated frequently
//
// 3. Data
//    - Encrypted with DEK
//    - Re-encryption on DEK rotation
//
// **Benefits:**
// - Master key never used directly for data
// - DEKs can be rotated without re-encrypting all data
// - Compromised DEK only affects subset of data
// - Supports multi-tenancy
// - KMS-ready architecture
//
// ## Key Rotation Workflow:
//
// 1. Generate new DEK
// 2. Encrypt new DEK with current Master Key
// 3. Store encrypted DEK in database
// 4. Mark old DEK as deprecated (keep for decryption)
// 5. Background job re-encrypts data with new DEK
// 6. Delete old DEK after re-encryption complete
//
// ## Future: KMS Integration
//
// This service is designed to easily integrate with:
// - AWS KMS
// - HashiCorp Vault
// - Azure Key Vault
// - Google Cloud KMS
//
// ============================================================================

use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::services::EncryptionService;
use crate::middleware::error_handling::{Result, AppError};

/// Key rotation status
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "key_status", rename_all = "lowercase")]
pub enum KeyStatus {
    Active,      // Currently used for encryption
    Deprecated,  // Still used for decryption, not for encryption
    Rotated,     // Fully rotated, can be deleted
}

/// Data Encryption Key (DEK) metadata
#[derive(Debug, Clone)]
pub struct DataEncryptionKey {
    pub id: Uuid,
    pub key_version: i32,
    pub encrypted_key: String,  // DEK encrypted with Master Key
    pub status: KeyStatus,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub deprecated_at: Option<DateTime<Utc>>,
    pub rotated_at: Option<DateTime<Utc>>,
    pub rotated_by: Option<Uuid>,
    pub rotation_reason: Option<String>,
}

pub struct EncryptionKeyRotationService {
    db_pool: PgPool,
    master_key: String,  // Master Key (KEK) from environment
}

impl EncryptionKeyRotationService {
    pub fn new(db_pool: PgPool, master_key: String) -> Self {
        Self {
            db_pool,
            master_key,
        }
    }

    /// Initialize key rotation system
    ///
    /// Creates database table for storing encrypted DEKs
    /// Run this once during application setup
    ///
    pub async fn initialize(&self) -> Result<()> {
        // Create data_encryption_keys table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS data_encryption_keys (
                id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                key_version INTEGER NOT NULL UNIQUE,
                encrypted_key TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'active',
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                deprecated_at TIMESTAMPTZ,
                rotated_at TIMESTAMPTZ,
                CONSTRAINT check_status CHECK (status IN ('active', 'deprecated', 'rotated'))
            )
            "#
        )
        .execute(&self.db_pool)
        .await?;

        // Create indexes separately (PostgreSQL doesn't allow multiple statements in prepared query)
        sqlx::query(
            r#"CREATE INDEX IF NOT EXISTS idx_data_encryption_keys_status
               ON data_encryption_keys(status) WHERE status = 'active'"#
        )
        .execute(&self.db_pool)
        .await?;

        sqlx::query(
            r#"CREATE INDEX IF NOT EXISTS idx_data_encryption_keys_version
               ON data_encryption_keys(key_version DESC)"#
        )
        .execute(&self.db_pool)
        .await?;

        tracing::info!("✅ Encryption key rotation system initialized");

        Ok(())
    }

    /// Get the current active DEK
    ///
    /// Returns the most recent active Data Encryption Key
    ///
    pub async fn get_active_key(&self) -> Result<DataEncryptionKey> {
        let key = sqlx::query_as!(
            DataEncryptionKey,
            r#"
            SELECT id, key_version, encrypted_key,
                   status as "status: KeyStatus",
                   is_active, created_at, valid_until,
                   deprecated_at, rotated_at, rotated_by, rotation_reason
            FROM data_encryption_keys
            WHERE status = 'active'
            ORDER BY key_version DESC
            LIMIT 1
            "#
        )
        .fetch_optional(&self.db_pool)
        .await?;

        match key {
            Some(k) => Ok(k),
            None => {
                // No active key exists, create initial key
                tracing::warn!("No active encryption key found, creating initial key");
                self.create_initial_key().await
            }
        }
    }

    /// Create initial encryption key
    ///
    /// Called automatically if no active key exists
    ///
    async fn create_initial_key(&self) -> Result<DataEncryptionKey> {
        // Generate new random DEK (256 bits)
        use rand::RngCore;
        let mut dek_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut dek_bytes);
        let dek = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, dek_bytes);

        // Encrypt DEK with Master Key
        let master_encryption = EncryptionService::new(&self.master_key)?;
        let encrypted_dek = master_encryption.encrypt(&dek)?;

        // Store in database
        let key = sqlx::query_as!(
            DataEncryptionKey,
            r#"
            INSERT INTO data_encryption_keys (key_version, encrypted_key, status)
            VALUES (1, $1, 'active')
            RETURNING id, key_version, encrypted_key,
                      status as "status: KeyStatus",
                      is_active, created_at, valid_until,
                      deprecated_at, rotated_at, rotated_by, rotation_reason
            "#,
            encrypted_dek
        )
        .fetch_one(&self.db_pool)
        .await?;

        tracing::info!(
            "✅ Created initial encryption key (version {})",
            key.key_version
        );

        Ok(key)
    }

    /// Decrypt a DEK using the Master Key
    ///
    /// Returns the plaintext DEK for use in data encryption/decryption
    ///
    pub fn decrypt_dek(&self, encrypted_dek: &str) -> Result<String> {
        let master_encryption = EncryptionService::new(&self.master_key)?;
        master_encryption.decrypt(encrypted_dek)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to decrypt DEK: {}", e)))
    }

    /// Rotate encryption key
    ///
    /// 🔒 SECURITY: Creates new DEK and marks old one as deprecated
    ///
    /// **Workflow:**
    /// 1. Get current active key
    /// 2. Generate new DEK
    /// 3. Encrypt new DEK with Master Key
    /// 4. Store new DEK as active
    /// 5. Mark old DEK as deprecated
    /// 6. Return new key for immediate use
    ///
    /// **Note:** Old key kept for decrypting existing data
    /// Background job should re-encrypt data with new key
    ///
    pub async fn rotate_key(&self) -> Result<DataEncryptionKey> {
        // Get current active key
        let current_key = self.get_active_key().await?;

        // Generate new random DEK
        use rand::RngCore;
        let mut dek_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut dek_bytes);
        let dek = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, dek_bytes);

        // Encrypt new DEK with Master Key
        let master_encryption = EncryptionService::new(&self.master_key)?;
        let encrypted_dek = master_encryption.encrypt(&dek)?;

        // Start transaction
        let mut tx = self.db_pool.begin().await?;

        // Mark current key as deprecated
        sqlx::query!(
            r#"
            UPDATE data_encryption_keys
            SET status = 'deprecated', deprecated_at = NOW()
            WHERE id = $1
            "#,
            current_key.id
        )
        .execute(&mut *tx)
        .await?;

        // Create new active key
        let new_key = sqlx::query_as!(
            DataEncryptionKey,
            r#"
            INSERT INTO data_encryption_keys (key_version, encrypted_key, status)
            VALUES ($1, $2, 'active')
            RETURNING id, key_version, encrypted_key,
                      status as "status: KeyStatus",
                      is_active, created_at, valid_until,
                      deprecated_at, rotated_at, rotated_by, rotation_reason
            "#,
            current_key.key_version + 1,
            encrypted_dek
        )
        .fetch_one(&mut *tx)
        .await?;

        // Commit transaction
        tx.commit().await?;

        tracing::warn!(
            "🔑 ENCRYPTION KEY ROTATED: v{} → v{} (old key deprecated, re-encryption required)",
            current_key.key_version,
            new_key.key_version
        );

        Ok(new_key)
    }

    /// Get key by version (for decrypting old data)
    ///
    /// Allows decrypting data encrypted with older DEK versions
    ///
    pub async fn get_key_by_version(&self, version: i32) -> Result<DataEncryptionKey> {
        sqlx::query_as!(
            DataEncryptionKey,
            r#"
            SELECT id, key_version, encrypted_key,
                   status as "status: KeyStatus",
                   is_active, created_at, valid_until,
                   deprecated_at, rotated_at, rotated_by, rotation_reason
            FROM data_encryption_keys
            WHERE key_version = $1
            "#,
            version
        )
        .fetch_optional(&self.db_pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Encryption key version {} not found", version)))
    }

    /// List all encryption keys with their status
    ///
    /// For administrative purposes and auditing
    ///
    pub async fn list_keys(&self) -> Result<Vec<DataEncryptionKey>> {
        Ok(sqlx::query_as!(
            DataEncryptionKey,
            r#"
            SELECT id, key_version, encrypted_key,
                   status as "status: KeyStatus",
                   is_active, created_at, valid_until,
                   deprecated_at, rotated_at, rotated_by, rotation_reason
            FROM data_encryption_keys
            ORDER BY key_version DESC
            "#
        )
        .fetch_all(&self.db_pool)
        .await?)
    }

    /// Mark a deprecated key as fully rotated
    ///
    /// Call this after all data has been re-encrypted with new key
    /// Allows safe deletion of old key
    ///
    pub async fn mark_key_rotated(&self, key_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE data_encryption_keys
            SET status = 'rotated', rotated_at = NOW()
            WHERE id = $1 AND status = 'deprecated'
            "#,
            key_id
        )
        .execute(&self.db_pool)
        .await?;

        tracing::info!("✅ Encryption key {} marked as rotated", key_id);

        Ok(())
    }

    /// Get rotation schedule recommendation
    ///
    /// Returns days until next recommended rotation
    ///
    pub async fn get_rotation_recommendation(&self) -> Result<i64> {
        let active_key = self.get_active_key().await?;
        let age_days = (Utc::now() - active_key.created_at).num_days();

        // 🔒 SECURITY: Recommended rotation schedule
        const ROTATION_INTERVAL_DAYS: i64 = 90; // Rotate every 90 days

        let days_until_rotation = ROTATION_INTERVAL_DAYS - age_days;

        if days_until_rotation <= 0 {
            tracing::warn!(
                "⚠️  ENCRYPTION KEY ROTATION OVERDUE: Key v{} is {} days old (recommend rotation every {} days)",
                active_key.key_version,
                age_days,
                ROTATION_INTERVAL_DAYS
            );
        } else if days_until_rotation <= 7 {
            tracing::warn!(
                "⚠️  ENCRYPTION KEY ROTATION DUE SOON: {} days remaining",
                days_until_rotation
            );
        }

        Ok(days_until_rotation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a database connection
    // Run with: cargo test --features test-db

    #[tokio::test]
    #[ignore] // Requires database
    async fn test_key_rotation_workflow() {
        // This would test the full rotation workflow
        // 1. Create initial key
        // 2. Rotate to new key
        // 3. Verify old key deprecated
        // 4. Verify new key active
    }
}
