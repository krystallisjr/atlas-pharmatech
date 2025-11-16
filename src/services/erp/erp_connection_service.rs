// ERP Connection Service
// Manages ERP connections with encrypted credential storage
// Handles connection lifecycle, testing, and credential management

use sqlx::PgPool;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::services::encryption_service::EncryptionService;
use crate::services::erp::{NetSuiteClient, NetSuiteConfig, SapClient, SapConfig, SapEnvironment};

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum ErpConnectionError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Connection not found: {0}")]
    NotFound(Uuid),

    #[error("Invalid ERP type: {0}")]
    InvalidErpType(String),

    #[error("Connection test failed: {0}")]
    TestFailed(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("NetSuite error: {0}")]
    NetSuiteError(String),

    #[error("SAP error: {0}")]
    SapError(String),
}

pub type Result<T> = std::result::Result<T, ErpConnectionError>;

// ============================================================================
// Data Models
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ErpType {
    #[serde(rename = "netsuite")]
    NetSuite,
    #[serde(rename = "sap_s4hana")]
    SapS4Hana,
}

impl ErpType {
    pub fn as_str(&self) -> &str {
        match self {
            ErpType::NetSuite => "netsuite",
            ErpType::SapS4Hana => "sap_s4hana",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "netsuite" => Ok(ErpType::NetSuite),
            "sap_s4hana" => Ok(ErpType::SapS4Hana),
            _ => Err(ErpConnectionError::InvalidErpType(s.to_string())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ErpConnection {
    pub id: Uuid,
    pub user_id: Uuid,
    pub erp_type: ErpType,
    pub connection_name: String,
    pub status: ConnectionStatus,

    // NetSuite credentials (decrypted in memory)
    pub netsuite_config: Option<NetSuiteConfig>,

    // SAP credentials (decrypted in memory)
    pub sap_config: Option<SapConfig>,

    // Sync configuration
    pub sync_enabled: bool,
    pub sync_frequency_minutes: i32,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_sync_status: Option<String>,

    // Feature flags
    pub sync_stock_levels: bool,
    pub sync_product_master: bool,
    pub sync_transactions: bool,
    pub sync_lot_batch: bool,

    // Sync direction
    pub default_sync_direction: SyncDirection,
    pub conflict_resolution: ConflictResolution,

    // Metadata
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    Active,
    Paused,
    Error,
    Disabled,
}

impl ConnectionStatus {
    pub fn as_str(&self) -> &str {
        match self {
            ConnectionStatus::Active => "active",
            ConnectionStatus::Paused => "paused",
            ConnectionStatus::Error => "error",
            ConnectionStatus::Disabled => "disabled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncDirection {
    AtlasToErp,
    ErpToAtlas,
    Bidirectional,
}

impl SyncDirection {
    pub fn as_str(&self) -> &str {
        match self {
            SyncDirection::AtlasToErp => "atlas_to_erp",
            SyncDirection::ErpToAtlas => "erp_to_atlas",
            SyncDirection::Bidirectional => "bidirectional",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    AtlasWins,
    ErpWins,
    Manual,
    LatestTimestamp,
}

impl ConflictResolution {
    pub fn as_str(&self) -> &str {
        match self {
            ConflictResolution::AtlasWins => "atlas_wins",
            ConflictResolution::ErpWins => "erp_wins",
            ConflictResolution::Manual => "manual",
            ConflictResolution::LatestTimestamp => "latest_timestamp",
        }
    }
}

// Request/Response DTOs
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateConnectionRequest {
    pub connection_name: String,
    pub erp_type: ErpType,

    // NetSuite fields
    pub netsuite_account_id: Option<String>,
    pub netsuite_consumer_key: Option<String>,
    pub netsuite_consumer_secret: Option<String>,
    pub netsuite_token_id: Option<String>,
    pub netsuite_token_secret: Option<String>,
    pub netsuite_realm: Option<String>,

    // SAP fields
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

#[derive(Debug, Serialize)]
pub struct ConnectionResponse {
    pub id: Uuid,
    pub erp_type: ErpType,
    pub connection_name: String,
    pub status: ConnectionStatus,
    pub sync_enabled: bool,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_sync_status: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

// ============================================================================
// ERP Connection Service
// ============================================================================

pub struct ErpConnectionService {
    db_pool: PgPool,
    encryption_service: EncryptionService,
}

impl ErpConnectionService {
    pub fn new(db_pool: PgPool) -> Self {
        let encryption_key = std::env::var("ENCRYPTION_KEY")
            .expect("ENCRYPTION_KEY must be set in environment");
        let encryption_service = EncryptionService::new(&encryption_key)
            .expect("Failed to initialize encryption service");

        Self {
            db_pool,
            encryption_service,
        }
    }

    // ========================================================================
    // Connection CRUD Operations
    // ========================================================================

    /// Create a new ERP connection with encrypted credentials
    pub async fn create_connection(
        &self,
        user_id: Uuid,
        request: CreateConnectionRequest,
    ) -> Result<ErpConnection> {
        // Validate request
        self.validate_create_request(&request)?;

        let connection_id = Uuid::new_v4();
        let now = Utc::now();

        match request.erp_type {
            ErpType::NetSuite => {
                self.create_netsuite_connection(connection_id, user_id, request, now)
                    .await
            }
            ErpType::SapS4Hana => {
                self.create_sap_connection(connection_id, user_id, request, now)
                    .await
            }
        }
    }

    async fn create_netsuite_connection(
        &self,
        connection_id: Uuid,
        user_id: Uuid,
        request: CreateConnectionRequest,
        now: DateTime<Utc>,
    ) -> Result<ErpConnection> {
        let account_id = request.netsuite_account_id.as_ref()
            .ok_or_else(|| ErpConnectionError::ConfigError("netsuite_account_id is required".to_string()))?;
        let consumer_key = request.netsuite_consumer_key.as_ref()
            .ok_or_else(|| ErpConnectionError::ConfigError("netsuite_consumer_key is required".to_string()))?;
        let consumer_secret = request.netsuite_consumer_secret.as_ref()
            .ok_or_else(|| ErpConnectionError::ConfigError("netsuite_consumer_secret is required".to_string()))?;
        let token_id = request.netsuite_token_id.as_ref()
            .ok_or_else(|| ErpConnectionError::ConfigError("netsuite_token_id is required".to_string()))?;
        let token_secret = request.netsuite_token_secret.as_ref()
            .ok_or_else(|| ErpConnectionError::ConfigError("netsuite_token_secret is required".to_string()))?;

        // Encrypt credentials
        let encrypted_consumer_key = self.encryption_service.encrypt(consumer_key)
            .map_err(|e| ErpConnectionError::EncryptionError(e.to_string()))?;
        let encrypted_consumer_secret = self.encryption_service.encrypt(consumer_secret)
            .map_err(|e| ErpConnectionError::EncryptionError(e.to_string()))?;
        let encrypted_token_id = self.encryption_service.encrypt(token_id)
            .map_err(|e| ErpConnectionError::EncryptionError(e.to_string()))?;
        let encrypted_token_secret = self.encryption_service.encrypt(token_secret)
            .map_err(|e| ErpConnectionError::EncryptionError(e.to_string()))?;

        // Insert into database
        sqlx::query!(
            r#"
            INSERT INTO erp_connections (
                id, user_id, erp_type, connection_name, status,
                netsuite_account_id, netsuite_consumer_key, netsuite_consumer_secret,
                netsuite_token_id, netsuite_token_secret, netsuite_realm,
                sync_enabled, sync_frequency_minutes,
                sync_stock_levels, sync_product_master, sync_transactions, sync_lot_batch,
                default_sync_direction, conflict_resolution,
                created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8, $9, $10, $11,
                $12, $13,
                $14, $15, $16, $17,
                $18, $19,
                $20, $21
            )
            "#,
            connection_id,
            user_id,
            ErpType::NetSuite.as_str(),
            request.connection_name,
            ConnectionStatus::Active.as_str(),
            account_id,
            encrypted_consumer_key,
            encrypted_consumer_secret,
            encrypted_token_id,
            encrypted_token_secret,
            request.netsuite_realm.unwrap_or_else(|| account_id.clone()),
            request.sync_enabled.unwrap_or(true),
            request.sync_frequency_minutes.unwrap_or(15),
            request.sync_stock_levels.unwrap_or(true),
            request.sync_product_master.unwrap_or(true),
            request.sync_transactions.unwrap_or(true),
            request.sync_lot_batch.unwrap_or(true),
            SyncDirection::Bidirectional.as_str(),
            ConflictResolution::AtlasWins.as_str(),
            now,
            now
        )
        .execute(&self.db_pool)
        .await?;

        // Return the created connection
        self.get_connection_by_id(connection_id).await
    }

    async fn create_sap_connection(
        &self,
        connection_id: Uuid,
        user_id: Uuid,
        request: CreateConnectionRequest,
        now: DateTime<Utc>,
    ) -> Result<ErpConnection> {
        let base_url = request.sap_base_url.as_ref()
            .ok_or_else(|| ErpConnectionError::ConfigError("sap_base_url is required".to_string()))?;
        let client_id = request.sap_client_id.as_ref()
            .ok_or_else(|| ErpConnectionError::ConfigError("sap_client_id is required".to_string()))?;
        let client_secret = request.sap_client_secret.as_ref()
            .ok_or_else(|| ErpConnectionError::ConfigError("sap_client_secret is required".to_string()))?;
        let token_endpoint = request.sap_token_endpoint.as_ref()
            .ok_or_else(|| ErpConnectionError::ConfigError("sap_token_endpoint is required".to_string()))?;

        // Encrypt credentials
        let encrypted_client_id = self.encryption_service.encrypt(client_id)
            .map_err(|e| ErpConnectionError::EncryptionError(e.to_string()))?;
        let encrypted_client_secret = self.encryption_service.encrypt(client_secret)
            .map_err(|e| ErpConnectionError::EncryptionError(e.to_string()))?;

        let environment = request.sap_environment.as_deref().unwrap_or("cloud");

        // Insert into database
        sqlx::query!(
            r#"
            INSERT INTO erp_connections (
                id, user_id, erp_type, connection_name, status,
                sap_base_url, sap_client_id, sap_client_secret, sap_token_endpoint,
                sap_environment, sap_plant, sap_company_code,
                sync_enabled, sync_frequency_minutes,
                sync_stock_levels, sync_product_master, sync_transactions, sync_lot_batch,
                default_sync_direction, conflict_resolution,
                created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8, $9, $10, $11, $12,
                $13, $14,
                $15, $16, $17, $18,
                $19, $20,
                $21, $22
            )
            "#,
            connection_id,
            user_id,
            ErpType::SapS4Hana.as_str(),
            request.connection_name,
            ConnectionStatus::Active.as_str(),
            base_url,
            encrypted_client_id,
            encrypted_client_secret,
            token_endpoint,
            environment,
            request.sap_plant,
            request.sap_company_code,
            request.sync_enabled.unwrap_or(true),
            request.sync_frequency_minutes.unwrap_or(15),
            request.sync_stock_levels.unwrap_or(true),
            request.sync_product_master.unwrap_or(true),
            request.sync_transactions.unwrap_or(true),
            request.sync_lot_batch.unwrap_or(true),
            SyncDirection::Bidirectional.as_str(),
            ConflictResolution::AtlasWins.as_str(),
            now,
            now
        )
        .execute(&self.db_pool)
        .await?;

        self.get_connection_by_id(connection_id).await
    }

    /// Get connection by ID with decrypted credentials
    pub async fn get_connection_by_id(&self, connection_id: Uuid) -> Result<ErpConnection> {
        let row = sqlx::query(
            r#"
            SELECT
                id, user_id, erp_type, connection_name, status,
                netsuite_account_id, netsuite_consumer_key, netsuite_consumer_secret,
                netsuite_token_id, netsuite_token_secret, netsuite_realm,
                sap_base_url, sap_client_id, sap_client_secret, sap_token_endpoint,
                sap_environment, sap_plant, sap_company_code,
                sync_enabled, sync_frequency_minutes, last_sync_at, last_sync_status,
                sync_stock_levels, sync_product_master, sync_transactions, sync_lot_batch,
                default_sync_direction, conflict_resolution,
                created_at, updated_at
            FROM erp_connections
            WHERE id = $1
            "#
        )
        .bind(connection_id)
        .fetch_optional(&self.db_pool)
        .await?
        .ok_or(ErpConnectionError::NotFound(connection_id))?;

        self.build_connection_from_row(row).await
    }

    /// Get all connections for a user
    pub async fn get_user_connections(&self, user_id: Uuid) -> Result<Vec<ErpConnection>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id, user_id, erp_type, connection_name, status,
                netsuite_account_id, netsuite_consumer_key, netsuite_consumer_secret,
                netsuite_token_id, netsuite_token_secret, netsuite_realm,
                sap_base_url, sap_client_id, sap_client_secret, sap_token_endpoint,
                sap_environment, sap_plant, sap_company_code,
                sync_enabled, sync_frequency_minutes, last_sync_at, last_sync_status,
                sync_stock_levels, sync_product_master, sync_transactions, sync_lot_batch,
                default_sync_direction, conflict_resolution,
                created_at, updated_at
            FROM erp_connections
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#
        )
        .bind(user_id)
        .fetch_all(&self.db_pool)
        .await?;

        let mut connections = Vec::new();
        for row in rows {
            connections.push(self.build_connection_from_row(row).await?);
        }

        Ok(connections)
    }

    /// Get active connection for user (first active connection)
    pub async fn get_active_connection_for_user(&self, user_id: Uuid) -> Result<ErpConnection> {
        let row = sqlx::query(
            r#"
            SELECT
                id, user_id, erp_type, connection_name, status,
                netsuite_account_id, netsuite_consumer_key, netsuite_consumer_secret,
                netsuite_token_id, netsuite_token_secret, netsuite_realm,
                sap_base_url, sap_client_id, sap_client_secret, sap_token_endpoint,
                sap_environment, sap_plant, sap_company_code,
                sync_enabled, sync_frequency_minutes, last_sync_at, last_sync_status,
                sync_stock_levels, sync_product_master, sync_transactions, sync_lot_batch,
                default_sync_direction, conflict_resolution,
                created_at, updated_at
            FROM erp_connections
            WHERE user_id = $1 AND status = 'active' AND sync_enabled = true
            ORDER BY created_at DESC
            LIMIT 1
            "#
        )
        .bind(user_id)
        .fetch_optional(&self.db_pool)
        .await?
        .ok_or_else(|| ErpConnectionError::NotFound(Uuid::nil()))?;

        self.build_connection_from_row(row).await
    }

    /// Delete connection
    pub async fn delete_connection(&self, connection_id: Uuid, user_id: Uuid) -> Result<()> {
        let result = sqlx::query!(
            r#"
            DELETE FROM erp_connections
            WHERE id = $1 AND user_id = $2
            "#,
            connection_id,
            user_id
        )
        .execute(&self.db_pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(ErpConnectionError::NotFound(connection_id));
        }

        Ok(())
    }

    /// Update connection status
    pub async fn update_connection_status(
        &self,
        connection_id: Uuid,
        status: ConnectionStatus,
        error_message: Option<String>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE erp_connections
            SET status = $2, last_sync_error = $3, updated_at = NOW()
            WHERE id = $1
            "#,
            connection_id,
            status.as_str(),
            error_message
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    /// Update last sync metadata
    pub async fn update_sync_metadata(
        &self,
        connection_id: Uuid,
        status: &str,
        duration_seconds: Option<i32>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE erp_connections
            SET last_sync_at = NOW(),
                last_sync_status = $2,
                last_sync_duration_seconds = $3,
                updated_at = NOW()
            WHERE id = $1
            "#,
            connection_id,
            status,
            duration_seconds
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    // ========================================================================
    // Connection Testing
    // ========================================================================

    /// Test connection to ERP system
    pub async fn test_connection(&self, connection: &ErpConnection) -> Result<ConnectionTestResult> {
        match &connection.erp_type {
            ErpType::NetSuite => self.test_netsuite_connection(connection).await,
            ErpType::SapS4Hana => self.test_sap_connection(connection).await,
        }
    }

    async fn test_netsuite_connection(&self, connection: &ErpConnection) -> Result<ConnectionTestResult> {
        let config = connection.netsuite_config.as_ref()
            .ok_or_else(|| ErpConnectionError::ConfigError("NetSuite config not loaded".to_string()))?;

        let client = NetSuiteClient::new(config.clone())
            .map_err(|e| ErpConnectionError::NetSuiteError(e.to_string()))?;

        match client.test_connection().await {
            Ok(true) => Ok(ConnectionTestResult {
                success: true,
                message: "Successfully connected to NetSuite".to_string(),
                details: Some(serde_json::json!({
                    "account_id": config.account_id,
                    "base_url": format!("https://{}.suitetalk.api.netsuite.com", config.account_id)
                })),
            }),
            Ok(false) => Ok(ConnectionTestResult {
                success: false,
                message: "Connection failed - invalid response from NetSuite".to_string(),
                details: None,
            }),
            Err(e) => Ok(ConnectionTestResult {
                success: false,
                message: format!("Connection test failed: {}", e),
                details: None,
            }),
        }
    }

    async fn test_sap_connection(&self, connection: &ErpConnection) -> Result<ConnectionTestResult> {
        let config = connection.sap_config.as_ref()
            .ok_or_else(|| ErpConnectionError::ConfigError("SAP config not loaded".to_string()))?;

        let client = SapClient::new(config.clone())
            .map_err(|e| ErpConnectionError::SapError(e.to_string()))?;

        match client.test_connection().await {
            Ok(true) => Ok(ConnectionTestResult {
                success: true,
                message: "Successfully connected to SAP S/4HANA".to_string(),
                details: Some(serde_json::json!({
                    "base_url": config.base_url,
                    "environment": format!("{:?}", config.environment)
                })),
            }),
            Ok(false) => Ok(ConnectionTestResult {
                success: false,
                message: "Connection failed - invalid response from SAP".to_string(),
                details: None,
            }),
            Err(e) => Ok(ConnectionTestResult {
                success: false,
                message: format!("Connection test failed: {}", e),
                details: None,
            }),
        }
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    async fn build_connection_from_row(&self, row: sqlx::postgres::PgRow) -> Result<ErpConnection> {
        use sqlx::Row;

        let id: Uuid = row.get("id");
        let user_id: Uuid = row.get("user_id");
        let erp_type_str: String = row.get("erp_type");
        let erp_type = ErpType::from_str(&erp_type_str)?;

        let netsuite_config = if erp_type == ErpType::NetSuite {
            let account_id: String = row.get("netsuite_account_id");
            let encrypted_consumer_key: String = row.get("netsuite_consumer_key");
            let encrypted_consumer_secret: String = row.get("netsuite_consumer_secret");
            let encrypted_token_id: String = row.get("netsuite_token_id");
            let encrypted_token_secret: String = row.get("netsuite_token_secret");
            let realm: Option<String> = row.get("netsuite_realm");

            // Decrypt credentials
            let consumer_key = self.encryption_service.decrypt(&encrypted_consumer_key)
                .map_err(|e| ErpConnectionError::EncryptionError(e.to_string()))?;
            let consumer_secret = self.encryption_service.decrypt(&encrypted_consumer_secret)
                .map_err(|e| ErpConnectionError::EncryptionError(e.to_string()))?;
            let token_id = self.encryption_service.decrypt(&encrypted_token_id)
                .map_err(|e| ErpConnectionError::EncryptionError(e.to_string()))?;
            let token_secret = self.encryption_service.decrypt(&encrypted_token_secret)
                .map_err(|e| ErpConnectionError::EncryptionError(e.to_string()))?;

            Some(NetSuiteConfig {
                account_id,
                consumer_key,
                consumer_secret,
                token_id,
                token_secret,
                realm,
            })
        } else {
            None
        };

        let sap_config = if erp_type == ErpType::SapS4Hana {
            let base_url: String = row.get("sap_base_url");
            let encrypted_client_id: String = row.get("sap_client_id");
            let encrypted_client_secret: String = row.get("sap_client_secret");
            let token_endpoint: String = row.get("sap_token_endpoint");
            let environment_str: String = row.get("sap_environment");
            let plant: Option<String> = row.get("sap_plant");
            let company_code: Option<String> = row.get("sap_company_code");

            // Decrypt credentials
            let client_id = self.encryption_service.decrypt(&encrypted_client_id)
                .map_err(|e| ErpConnectionError::EncryptionError(e.to_string()))?;
            let client_secret = self.encryption_service.decrypt(&encrypted_client_secret)
                .map_err(|e| ErpConnectionError::EncryptionError(e.to_string()))?;

            let environment = match environment_str.as_str() {
                "cloud" => SapEnvironment::Cloud,
                "on_premise" => SapEnvironment::OnPremise,
                _ => SapEnvironment::Cloud,
            };

            Some(SapConfig {
                base_url,
                client_id,
                client_secret,
                token_endpoint,
                environment,
                plant,
                company_code,
            })
        } else {
            None
        };

        let status_str: String = row.get("status");
        let status = match status_str.as_str() {
            "active" => ConnectionStatus::Active,
            "paused" => ConnectionStatus::Paused,
            "error" => ConnectionStatus::Error,
            "disabled" => ConnectionStatus::Disabled,
            _ => ConnectionStatus::Active,
        };

        let sync_direction_str: String = row.get("default_sync_direction");
        let default_sync_direction = match sync_direction_str.as_str() {
            "atlas_to_erp" => SyncDirection::AtlasToErp,
            "erp_to_atlas" => SyncDirection::ErpToAtlas,
            _ => SyncDirection::Bidirectional,
        };

        let conflict_str: String = row.get("conflict_resolution");
        let conflict_resolution = match conflict_str.as_str() {
            "erp_wins" => ConflictResolution::ErpWins,
            "manual" => ConflictResolution::Manual,
            "latest_timestamp" => ConflictResolution::LatestTimestamp,
            _ => ConflictResolution::AtlasWins,
        };

        Ok(ErpConnection {
            id,
            user_id,
            erp_type,
            connection_name: row.get("connection_name"),
            status,
            netsuite_config,
            sap_config,
            sync_enabled: row.get("sync_enabled"),
            sync_frequency_minutes: row.get("sync_frequency_minutes"),
            last_sync_at: row.get("last_sync_at"),
            last_sync_status: row.get("last_sync_status"),
            sync_stock_levels: row.get("sync_stock_levels"),
            sync_product_master: row.get("sync_product_master"),
            sync_transactions: row.get("sync_transactions"),
            sync_lot_batch: row.get("sync_lot_batch"),
            default_sync_direction,
            conflict_resolution,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    fn validate_create_request(&self, request: &CreateConnectionRequest) -> Result<()> {
        if request.connection_name.is_empty() {
            return Err(ErpConnectionError::ConfigError("connection_name is required".to_string()));
        }

        match request.erp_type {
            ErpType::NetSuite => {
                if request.netsuite_account_id.is_none() {
                    return Err(ErpConnectionError::ConfigError("netsuite_account_id is required".to_string()));
                }
                if request.netsuite_consumer_key.is_none() {
                    return Err(ErpConnectionError::ConfigError("netsuite_consumer_key is required".to_string()));
                }
                if request.netsuite_consumer_secret.is_none() {
                    return Err(ErpConnectionError::ConfigError("netsuite_consumer_secret is required".to_string()));
                }
                if request.netsuite_token_id.is_none() {
                    return Err(ErpConnectionError::ConfigError("netsuite_token_id is required".to_string()));
                }
                if request.netsuite_token_secret.is_none() {
                    return Err(ErpConnectionError::ConfigError("netsuite_token_secret is required".to_string()));
                }
            }
            ErpType::SapS4Hana => {
                if request.sap_base_url.is_none() {
                    return Err(ErpConnectionError::ConfigError("sap_base_url is required".to_string()));
                }
                if request.sap_client_id.is_none() {
                    return Err(ErpConnectionError::ConfigError("sap_client_id is required".to_string()));
                }
                if request.sap_client_secret.is_none() {
                    return Err(ErpConnectionError::ConfigError("sap_client_secret is required".to_string()));
                }
                if request.sap_token_endpoint.is_none() {
                    return Err(ErpConnectionError::ConfigError("sap_token_endpoint is required".to_string()));
                }
            }
        }

        Ok(())
    }

    pub fn to_response(&self, connection: &ErpConnection) -> ConnectionResponse {
        ConnectionResponse {
            id: connection.id,
            erp_type: connection.erp_type.clone(),
            connection_name: connection.connection_name.clone(),
            status: connection.status.clone(),
            sync_enabled: connection.sync_enabled,
            last_sync_at: connection.last_sync_at,
            last_sync_status: connection.last_sync_status.clone(),
            created_at: connection.created_at,
            updated_at: connection.updated_at,
        }
    }
}
