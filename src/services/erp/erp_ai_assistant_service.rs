/// Production-grade ERP AI Assistant Service
/// Provides AI-powered features for ERP integration: auto-discovery, conflict resolution, sync analysis
/// Follows Atlas Pharma AI service patterns with quota management and cost tracking

use uuid::Uuid;
use sqlx::PgPool;
use serde::{Deserialize, Serialize};
use crate::middleware::error_handling::{Result, AppError};
use crate::services::claude_ai_service::{ClaudeAIService, ClaudeRequestConfig, user_message};
use crate::services::erp::{ErpConnection, ErpType, ConnectionStatus, ConflictResolution};
use crate::services::erp::erp_connection_service::{SyncDirection, ErpConnectionService};
use crate::services::erp::netsuite_client::{NetSuiteClient, NetSuiteSearchParams, NetSuiteError};
use crate::services::erp::sap_client::{SapClient, SapError};
use std::collections::HashMap;
use rust_decimal::Decimal;

// ============================================================================
// Constants
// ============================================================================

const MAPPING_DISCOVERY_SYSTEM_PROMPT: &str = r#"You are an expert pharmaceutical ERP integration specialist. Your task is to match Atlas Pharma inventory items with ERP (NetSuite/SAP) inventory items.

MATCHING RULES:
1. NDC codes are PRIMARY identifiers - exact match = highest confidence
2. Product names should match accounting for brand/generic variations
3. Manufacturer names help disambiguate similar products
4. Package sizes and strengths must match (e.g., "500mg" vs "250mg" are different)
5. Be conservative with confidence scores - only high scores for certain matches

CONFIDENCE SCORING:
- 1.00: Perfect NDC match + name match
- 0.95-0.99: NDC match OR strong name + manufacturer match
- 0.80-0.94: Good name similarity + partial manufacturer match
- 0.60-0.79: Weak name match, recommend manual review
- Below 0.60: Do not suggest, too uncertain

Your response MUST be valid JSON with this structure:
{
  "mappings": [
    {
      "atlas_inventory_id": "uuid",
      "erp_item_id": "string",
      "erp_item_name": "string",
      "confidence_score": 0.0-1.0,
      "matching_factors": {
        "ndc_match": true/false,
        "name_similarity": 0.0-1.0,
        "manufacturer_match": true/false,
        "strength_match": true/false
      },
      "reasoning": "Explain why this mapping is suggested"
    }
  ],
  "unmapped_atlas_items": ["uuid1", "uuid2"],
  "unmapped_erp_items": ["item_id1", "item_id2"],
  "warnings": ["Any data quality issues noticed"]
}"#;

const SYNC_ANALYSIS_SYSTEM_PROMPT: &str = r#"You are an expert pharmaceutical ERP integration analyst. Analyze sync operation results and provide clear, actionable insights.

YOUR ROLE:
1. Explain errors in plain English (not just error codes)
2. Suggest specific fixes for failures
3. Identify data quality issues
4. Detect anomalies (unusual quantity changes, pricing errors)
5. Provide actionable recommendations

Your response MUST be valid JSON:
{
  "insight_type": "error_explanation|performance_analysis|data_quality|anomaly_detection|success_summary",
  "severity": "info|warning|error|critical",
  "title": "Brief summary",
  "explanation": "Clear explanation in plain English",
  "recommendations": [
    {
      "action": "Specific action to take",
      "priority": "high|medium|low",
      "description": "Why this helps"
    }
  ],
  "actionable": true/false
}"#;

const CONFLICT_RESOLUTION_SYSTEM_PROMPT: &str = r#"You are an expert pharmaceutical inventory reconciliation specialist. Analyze conflicts between Atlas and ERP systems and recommend resolutions.

ANALYSIS CRITERIA:
1. Timestamp - which system was updated more recently?
2. Transaction history - are there recent sales/receipts in one system?
3. Data patterns - which system is typically more accurate?
4. Business rules - what makes sense in pharmaceutical context?
5. Risk assessment - high-value controlled substances need extra caution

RESOLUTION OPTIONS:
- "atlas_wins": Atlas data is correct
- "erp_wins": ERP data is correct
- "manual_review": Too risky to auto-resolve
- "merge": Combine data from both systems
- "reject_sync": Block this sync, investigate first

RISK LEVELS:
- "critical": Controlled substances, large quantity changes
- "high": High-value items, major discrepancies
- "medium": Standard items, moderate discrepancies
- "low": Minor differences, non-critical items

Your response MUST be valid JSON:
{
  "resolutions": [
    {
      "conflict_type": "quantity_mismatch|price_mismatch|data_quality|timestamp_conflict",
      "suggested_resolution": "atlas_wins|erp_wins|manual_review|merge|reject_sync",
      "confidence_score": 0.0-1.0,
      "risk_level": "low|medium|high|critical",
      "reasoning": "Detailed explanation of recommendation",
      "evidence": {
        "atlas_timestamp": "ISO timestamp or null",
        "erp_timestamp": "ISO timestamp or null",
        "recent_atlas_transactions": "description",
        "recent_erp_transactions": "description"
      }
    }
  ]
}"#;

// ============================================================================
// Request/Response Models
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct MappingSuggestion {
    pub atlas_inventory_id: Uuid,
    pub erp_item_id: String,
    pub erp_item_name: String,
    pub erp_item_description: Option<String>,
    pub confidence_score: Decimal,
    pub matching_factors: serde_json::Value,
    pub reasoning: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MappingDiscoveryResponse {
    pub mappings: Vec<MappingSuggestion>,
    pub unmapped_atlas_items: Vec<Uuid>,
    pub unmapped_erp_items: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncInsight {
    pub insight_type: String,
    pub severity: String,
    pub title: String,
    pub explanation: String,
    pub recommendations: Vec<Recommendation>,
    pub actionable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Recommendation {
    pub action: String,
    pub priority: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConflictResolutionSuggestion {
    pub conflict_type: String,
    pub suggested_resolution: String,
    pub confidence_score: Decimal,
    pub risk_level: String,
    pub reasoning: String,
    pub evidence: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConflictResolutionResponse {
    pub resolutions: Vec<ConflictResolutionSuggestion>,
}

// Input models for AI analysis
#[derive(Debug, Serialize)]
struct AtlasInventoryItem {
    id: Uuid,
    ndc_code: Option<String>,
    product_name: String,
    manufacturer: Option<String>,
    strength: Option<String>,
    batch_number: String,
    quantity: i32,
}

#[derive(Debug, Serialize)]
struct ErpInventoryItem {
    id: String,
    name: String,
    description: Option<String>,
    quantity: f64,
    custom_fields: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct ConflictData {
    pub atlas_inventory_id: Uuid,
    pub erp_item_id: String,
    pub conflict_type: String,
    pub atlas_value: serde_json::Value,
    pub erp_value: serde_json::Value,
    pub atlas_updated_at: Option<String>,
    pub erp_updated_at: Option<String>,
}

// ============================================================================
// ERP AI Assistant Service
// ============================================================================

pub struct ErpAiAssistantService {
    db_pool: PgPool,
    claude_service: ClaudeAIService,
    connection_service: ErpConnectionService,
}

// Helper struct for sync log database queries
#[allow(dead_code)]
struct SyncLogRow {
    sync_direction: String,
    status: String,
    items_synced: i32,
    items_failed: i32,
    duration_seconds: Option<i32>,
    error_message: Option<String>,
    error_details: Option<serde_json::Value>,
    erp_connection_id: Uuid,
}

impl ErpAiAssistantService {
    pub fn new(db_pool: PgPool, claude_api_key: String) -> Self {
        let claude_service = ClaudeAIService::new(claude_api_key, db_pool.clone());
        let connection_service = ErpConnectionService::new(db_pool.clone());
        Self {
            db_pool,
            claude_service,
            connection_service,
        }
    }

    /// Auto-discover inventory mappings using AI
    /// Matches Atlas inventory items with ERP items based on NDC codes, names, manufacturers
    pub async fn auto_discover_mappings(
        &self,
        connection_id: Uuid,
        user_id: Uuid,
    ) -> Result<MappingDiscoveryResponse> {
        tracing::info!("Starting AI mapping discovery for connection: {}", connection_id);

        // Check quota BEFORE expensive operations
        if !self.check_erp_ai_mapping_quota(user_id).await? {
            return Err(AppError::QuotaExceeded(
                "Monthly ERP AI mapping discovery limit reached. Please contact support to increase limit.".to_string()
            ));
        }

        // Get ERP connection details
        let connection = self.get_connection(connection_id, user_id).await?;

        // Get Atlas inventory for this user (limit to 1000 items to avoid token limits)
        let atlas_items = self.get_atlas_inventory(user_id, 1000).await?;

        if atlas_items.is_empty() {
            return Err(AppError::BadRequest(
                "No inventory items found in Atlas. Please add inventory first.".to_string()
            ));
        }

        // Get ERP inventory items (mocked for now - real implementation would call ERP API)
        let erp_items = self.fetch_erp_inventory(&connection).await?;

        if erp_items.is_empty() {
            return Err(AppError::BadRequest(
                "No inventory items found in ERP system. Please ensure ERP connection is configured correctly.".to_string()
            ));
        }

        // Build AI analysis prompt
        let prompt = format!(
            r#"Match these Atlas Pharma inventory items with ERP items:

ATLAS INVENTORY ({} items):
{}

ERP INVENTORY ({} items):
{}

Provide mapping suggestions with confidence scores. Focus on NDC code matches first, then product name similarity."#,
            atlas_items.len(),
            serde_json::to_string_pretty(&atlas_items)?,
            erp_items.len(),
            serde_json::to_string_pretty(&erp_items)?
        );

        // Call Claude AI (quota already checked and reserved)
        let config = ClaudeRequestConfig {
            max_tokens: 4096,
            temperature: Some(0.3), // Low temperature for consistency
            system_prompt: Some(MAPPING_DISCOVERY_SYSTEM_PROMPT.to_string()),
        };

        let ai_response = self.claude_service.send_message(
            vec![user_message(&prompt)],
            config,
            user_id,
            None,
        ).await?;

        // Parse AI response
        let discovery_response: MappingDiscoveryResponse = serde_json::from_str(&ai_response.content)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse AI mapping response: {}", e)))?;

        // Save suggestions to database
        for suggestion in &discovery_response.mappings {
            self.save_mapping_suggestion(connection_id, suggestion).await?;
        }

        // Increment usage counter
        self.increment_erp_ai_mapping_usage(user_id).await?;

        tracing::info!(
            "AI mapping discovery complete: {} mappings, {} unmapped Atlas items, {} unmapped ERP items",
            discovery_response.mappings.len(),
            discovery_response.unmapped_atlas_items.len(),
            discovery_response.unmapped_erp_items.len()
        );

        Ok(discovery_response)
    }

    /// Analyze sync operation result with AI
    /// Provides plain-English explanations of errors and actionable recommendations
    pub async fn analyze_sync_result(
        &self,
        sync_log_id: Uuid,
        user_id: Uuid,
    ) -> Result<SyncInsight> {
        tracing::info!("Starting AI sync analysis for log: {}", sync_log_id);

        // Check quota
        if !self.check_erp_ai_analysis_quota(user_id).await? {
            return Err(AppError::QuotaExceeded(
                "Monthly ERP AI analysis limit reached.".to_string()
            ));
        }

        // Get sync log details
        let sync_log = self.get_sync_log(sync_log_id).await?;

        // Build AI analysis prompt
        let prompt = format!(
            r#"Analyze this ERP sync operation result:

SYNC DETAILS:
- Direction: {}
- Status: {}
- Items Synced: {}
- Items Failed: {}
- Duration: {}ms
- Error Message: {}
- Error Details: {}

Provide clear explanation of what happened, why it happened, and what to do next."#,
            sync_log.sync_direction,
            sync_log.status,
            sync_log.items_synced,
            sync_log.items_failed,
            sync_log.duration_seconds.unwrap_or(0),
            sync_log.error_message.as_ref().map(|s| s.as_str()).unwrap_or("None"),
            sync_log.error_details.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "None".to_string())
        );

        let config = ClaudeRequestConfig {
            max_tokens: 2048,
            temperature: Some(0.3),
            system_prompt: Some(SYNC_ANALYSIS_SYSTEM_PROMPT.to_string()),
        };

        let ai_response = self.claude_service.send_message(
            vec![user_message(&prompt)],
            config,
            user_id,
            None,
        ).await?;

        // Parse response
        let insight: SyncInsight = serde_json::from_str(&ai_response.content)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse AI sync analysis: {}", e)))?;

        // Save insight to database
        self.save_sync_insight(sync_log_id, &sync_log, &insight).await?;

        // Increment usage
        self.increment_erp_ai_analysis_usage(user_id).await?;

        tracing::info!("AI sync analysis complete: {} ({})", insight.title, insight.severity);

        Ok(insight)
    }

    /// Suggest conflict resolution using AI
    /// Analyzes data conflicts and recommends which system's data to trust
    pub async fn suggest_conflict_resolution(
        &self,
        connection_id: Uuid,
        conflicts: Vec<ConflictData>,
        user_id: Uuid,
    ) -> Result<ConflictResolutionResponse> {
        tracing::info!("Starting AI conflict resolution for {} conflicts", conflicts.len());

        // Check quota
        if !self.check_erp_ai_conflict_quota(user_id).await? {
            return Err(AppError::QuotaExceeded(
                "Monthly ERP AI conflict resolution limit reached.".to_string()
            ));
        }

        // Get connection for context
        let connection = self.get_connection(connection_id, user_id).await?;

        // Build analysis prompt
        let prompt = format!(
            r#"Analyze these inventory conflicts between Atlas and {} and recommend resolutions:

CONNECTION: {}
ERP TYPE: {}

CONFLICTS:
{}

For each conflict, recommend resolution with confidence and risk assessment."#,
            connection.erp_type.as_str(),
            connection.connection_name,
            connection.erp_type.as_str(),
            serde_json::to_string_pretty(&conflicts)?
        );

        let config = ClaudeRequestConfig {
            max_tokens: 3072,
            temperature: Some(0.3),
            system_prompt: Some(CONFLICT_RESOLUTION_SYSTEM_PROMPT.to_string()),
        };

        let ai_response = self.claude_service.send_message(
            vec![user_message(&prompt)],
            config,
            user_id,
            None,
        ).await?;

        // Parse response
        let resolution_response: ConflictResolutionResponse = serde_json::from_str(&ai_response.content)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse AI conflict resolution: {}", e)))?;

        // Save resolutions to database
        for (conflict, resolution) in conflicts.iter().zip(resolution_response.resolutions.iter()) {
            self.save_conflict_resolution(connection_id, conflict, resolution).await?;
        }

        // Increment usage
        self.increment_erp_ai_conflict_usage(user_id).await?;

        tracing::info!(
            "AI conflict resolution complete: {} resolutions suggested",
            resolution_response.resolutions.len()
        );

        Ok(resolution_response)
    }

    // ========================================================================
    // Quota Management (following ClaudeAIService pattern)
    // ========================================================================

    async fn check_erp_ai_mapping_quota(&self, user_id: Uuid) -> Result<bool> {
        let mut tx = self.db_pool.begin().await?;

        // Ensure user limits exist
        sqlx::query!(
            r#"
            INSERT INTO user_ai_usage_limits (user_id)
            VALUES ($1)
            ON CONFLICT (user_id) DO NOTHING
            "#,
            user_id
        )
        .execute(&mut *tx)
        .await?;

        // Get limits with row lock
        let limits = sqlx::query!(
            r#"
            SELECT
                monthly_erp_ai_mapping_limit,
                monthly_erp_ai_mapping_used
            FROM user_ai_usage_limits
            WHERE user_id = $1
            FOR UPDATE
            "#,
            user_id
        )
        .fetch_one(&mut *tx)
        .await?;

        let has_quota = limits.monthly_erp_ai_mapping_used < limits.monthly_erp_ai_mapping_limit;

        if !has_quota {
            tx.rollback().await?;
            return Ok(false);
        }

        tx.commit().await?;
        Ok(true)
    }

    async fn check_erp_ai_analysis_quota(&self, user_id: Uuid) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            SELECT monthly_erp_ai_analysis_used < monthly_erp_ai_analysis_limit as has_quota
            FROM user_ai_usage_limits
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.db_pool)
        .await?;

        Ok(result.map(|r| r.has_quota.unwrap_or(true)).unwrap_or(true))
    }

    async fn check_erp_ai_conflict_quota(&self, user_id: Uuid) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            SELECT monthly_erp_ai_conflict_used < monthly_erp_ai_conflict_limit as has_quota
            FROM user_ai_usage_limits
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.db_pool)
        .await?;

        Ok(result.map(|r| r.has_quota.unwrap_or(true)).unwrap_or(true))
    }

    async fn increment_erp_ai_mapping_usage(&self, user_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE user_ai_usage_limits
            SET monthly_erp_ai_mapping_used = monthly_erp_ai_mapping_used + 1,
                updated_at = NOW()
            WHERE user_id = $1
            "#,
            user_id
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    async fn increment_erp_ai_analysis_usage(&self, user_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE user_ai_usage_limits
            SET monthly_erp_ai_analysis_used = monthly_erp_ai_analysis_used + 1,
                updated_at = NOW()
            WHERE user_id = $1
            "#,
            user_id
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    async fn increment_erp_ai_conflict_usage(&self, user_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE user_ai_usage_limits
            SET monthly_erp_ai_conflict_used = monthly_erp_ai_conflict_used + 1,
                updated_at = NOW()
            WHERE user_id = $1
            "#,
            user_id
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    // ========================================================================
    // Database Helper Methods
    // ========================================================================

    async fn get_connection(&self, connection_id: Uuid, user_id: Uuid) -> Result<ErpConnection> {
        // Get connection with full credentials from connection service
        let connection = self.connection_service
            .get_connection_by_id(connection_id)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get ERP connection: {:?}", e)))?;

        // Verify ownership
        if connection.user_id != user_id {
            return Err(AppError::Forbidden("Access denied to this ERP connection".to_string()));
        }

        Ok(connection)
    }

    async fn get_atlas_inventory(&self, user_id: Uuid, limit: i64) -> Result<Vec<AtlasInventoryItem>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                i.id,
                i.batch_number,
                i.quantity,
                p.ndc_code,
                p.brand_name,
                p.generic_name,
                p.manufacturer,
                p.strength
            FROM inventory i
            JOIN pharmaceuticals p ON i.pharmaceutical_id = p.id
            WHERE i.user_id = $1
            LIMIT $2
            "#,
            user_id,
            limit
        )
        .fetch_all(&self.db_pool)
        .await?;

        Ok(rows.into_iter().map(|row| {
            // brand_name and generic_name are String (not Option), so check if empty
            let product_name = if !row.brand_name.is_empty() {
                row.brand_name
            } else if !row.generic_name.is_empty() {
                row.generic_name
            } else {
                "Unknown".to_string()
            };

            AtlasInventoryItem {
                id: row.id,
                ndc_code: row.ndc_code,
                product_name,
                manufacturer: Some(row.manufacturer),
                strength: row.strength,
                batch_number: row.batch_number,
                quantity: row.quantity,
            }
        }).collect())
    }

    async fn fetch_erp_inventory(&self, connection: &ErpConnection) -> Result<Vec<ErpInventoryItem>> {
        tracing::info!("Fetching real inventory from {} ERP", connection.erp_type.as_str());

        match connection.erp_type {
            ErpType::NetSuite => self.fetch_netsuite_inventory(connection).await,
            ErpType::SapS4Hana => self.fetch_sap_inventory(connection).await,
        }
    }

    /// Fetch inventory from NetSuite via SuiteTalk REST API
    async fn fetch_netsuite_inventory(&self, connection: &ErpConnection) -> Result<Vec<ErpInventoryItem>> {
        let netsuite_config = connection.netsuite_config.as_ref()
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("NetSuite credentials not found")))?;

        // Create NetSuite client
        let client = NetSuiteClient::new(netsuite_config.clone())
            .map_err(|e| self.map_netsuite_error(e))?;

        // Search for inventory items (limit to 1000 for performance)
        let search_params = NetSuiteSearchParams {
            q: None, // Get all inventory items
            limit: Some(1000),
            offset: Some(0),
            fields: Some(vec![
                "id".to_string(),
                "itemId".to_string(),
                "displayName".to_string(),
                "quantityOnHand".to_string(),
                "custitem_ndc_code".to_string(),
                "custitem_lot_number".to_string(),
                "custitem_expiry_date".to_string(),
                "description".to_string(),
                "manufacturer".to_string(),
            ]),
        };

        tracing::info!("Calling NetSuite inventory search API...");
        let search_result = client.search_inventory(search_params).await
            .map_err(|e| self.map_netsuite_error(e))?;

        tracing::info!("NetSuite returned {} inventory items", search_result.items.len());

        // Transform NetSuite items to generic ERP inventory items
        let erp_items = search_result.items.into_iter().map(|ns_item| {
            let mut custom_fields = HashMap::new();

            // Add NDC code if present
            if let Some(ndc) = ns_item.ndc_code {
                custom_fields.insert("ndc_code".to_string(), ndc);
            }

            // Add lot number if present
            if let Some(lot) = ns_item.lot_number {
                custom_fields.insert("lot_number".to_string(), lot);
            }

            // Add expiry date if present
            if let Some(expiry) = ns_item.expiry_date {
                custom_fields.insert("expiry_date".to_string(), expiry);
            }

            // Add manufacturer if present
            if let Some(ref mfg) = ns_item.manufacturer {
                custom_fields.insert("manufacturer".to_string(), mfg.name.clone());
            }

            ErpInventoryItem {
                id: ns_item.id,
                name: ns_item.display_name,
                description: ns_item.description,
                quantity: ns_item.quantity_on_hand.unwrap_or(0.0),
                custom_fields,
            }
        }).collect();

        Ok(erp_items)
    }

    /// Fetch inventory from SAP via OData API
    async fn fetch_sap_inventory(&self, connection: &ErpConnection) -> Result<Vec<ErpInventoryItem>> {
        let sap_config = connection.sap_config.as_ref()
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("SAP credentials not found")))?;

        // Create SAP client
        let client = SapClient::new(sap_config.clone())
            .map_err(|e| self.map_sap_error(e))?;

        tracing::info!("Fetching SAP product master data...");

        // For SAP, we need to search products and then get their stock
        // This is a simplified approach - in production you might want pagination
        let products = client.search_products("").await // Empty search gets all products (limited by SAP)
            .map_err(|e| self.map_sap_error(e))?;

        tracing::info!("SAP returned {} products, fetching stock data...", products.len());

        let mut erp_items = Vec::new();

        // For each product, get stock information
        for product in products.into_iter().take(1000) { // Limit to 1000 for performance
            // Get stock for all locations
            let stock_result = client.get_material_stock_all_locations(&product.product).await;

            match stock_result {
                Ok(stock_locations) => {
                    // Calculate total quantity across all locations
                    let total_quantity: f64 = stock_locations.iter()
                        .filter_map(|loc| loc.stock_quantity.parse::<f64>().ok())
                        .sum();

                    let mut custom_fields = HashMap::new();

                    // Add manufacturer if present
                    if let Some(mfg) = product.manufacturer {
                        custom_fields.insert("manufacturer".to_string(), mfg);
                    }

                    // Add product group if present
                    if let Some(group) = product.product_group {
                        custom_fields.insert("product_group".to_string(), group);
                    }

                    // Add base unit
                    custom_fields.insert("base_unit".to_string(), product.base_unit.clone());

                    erp_items.push(ErpInventoryItem {
                        id: product.product.clone(),
                        name: product.product, // SAP uses material number as name
                        description: product.description,
                        quantity: total_quantity,
                        custom_fields,
                    });
                }
                Err(e) => {
                    // Log error but continue processing other products
                    tracing::warn!("Failed to get stock for product {}: {:?}", product.product, e);
                    // Still add the product with zero quantity
                    erp_items.push(ErpInventoryItem {
                        id: product.product.clone(),
                        name: product.product,
                        description: product.description,
                        quantity: 0.0,
                        custom_fields: HashMap::new(),
                    });
                }
            }
        }

        tracing::info!("SAP inventory fetch complete: {} items with stock data", erp_items.len());

        Ok(erp_items)
    }

    /// Map NetSuite errors to AppError
    fn map_netsuite_error(&self, error: NetSuiteError) -> AppError {
        match error {
            NetSuiteError::AuthError(msg) => {
                tracing::error!("NetSuite authentication failed: {}", msg);
                AppError::Unauthorized
            },
            NetSuiteError::RateLimitExceeded => AppError::TooManyRequests("NetSuite API rate limit exceeded. Please try again later.".to_string()),
            NetSuiteError::NotFound(msg) => AppError::NotFound(format!("NetSuite resource not found: {}", msg)),
            NetSuiteError::ApiError(status, msg) => AppError::Internal(anyhow::anyhow!("NetSuite API error ({}): {}", status, msg)),
            NetSuiteError::NetworkError(e) => AppError::Internal(anyhow::anyhow!("NetSuite network error: {}", e)),
            NetSuiteError::ConfigError(msg) => AppError::BadRequest(format!("NetSuite configuration error: {}", msg)),
            _ => AppError::Internal(anyhow::anyhow!("NetSuite error: {:?}", error)),
        }
    }

    /// Map SAP errors to AppError
    fn map_sap_error(&self, error: SapError) -> AppError {
        match error {
            SapError::AuthError(msg) => {
                tracing::error!("SAP authentication failed: {}", msg);
                AppError::Unauthorized
            },
            SapError::RateLimitExceeded => AppError::TooManyRequests("SAP API rate limit exceeded. Please try again later.".to_string()),
            SapError::NotFound(msg) => AppError::NotFound(format!("SAP resource not found: {}", msg)),
            SapError::ApiError(status, msg) => AppError::Internal(anyhow::anyhow!("SAP API error ({}): {}", status, msg)),
            SapError::NetworkError(e) => AppError::Internal(anyhow::anyhow!("SAP network error: {}", e)),
            SapError::ConfigError(msg) => AppError::BadRequest(format!("SAP configuration error: {}", msg)),
            SapError::ODataError(msg) => AppError::Internal(anyhow::anyhow!("SAP OData error: {}", msg)),
            _ => AppError::Internal(anyhow::anyhow!("SAP error: {:?}", error)),
        }
    }

    async fn save_mapping_suggestion(&self, connection_id: Uuid, suggestion: &MappingSuggestion) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO erp_ai_mapping_suggestions (
                erp_connection_id,
                atlas_inventory_id,
                erp_item_id,
                erp_item_name,
                erp_item_description,
                confidence_score,
                ai_reasoning,
                matching_factors,
                status
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'suggested')
            ON CONFLICT (erp_connection_id, atlas_inventory_id, erp_item_id) DO UPDATE
            SET confidence_score = $6,
                ai_reasoning = $7,
                matching_factors = $8,
                updated_at = NOW()
            "#,
            connection_id,
            suggestion.atlas_inventory_id,
            suggestion.erp_item_id,
            suggestion.erp_item_name,
            suggestion.erp_item_description,
            suggestion.confidence_score,
            suggestion.reasoning,
            suggestion.matching_factors
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    async fn get_sync_log(&self, sync_log_id: Uuid) -> Result<SyncLogRow> {
        let row = sqlx::query_as!(
            SyncLogRow,
            r#"
            SELECT
                sync_direction,
                status,
                items_synced,
                items_failed,
                duration_seconds,
                error_message,
                error_details,
                erp_connection_id
            FROM erp_sync_logs
            WHERE id = $1
            "#,
            sync_log_id
        )
        .fetch_optional(&self.db_pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Sync log not found".to_string()))?;

        Ok(row)
    }

    async fn save_sync_insight(&self, sync_log_id: Uuid, sync_log: &SyncLogRow, insight: &SyncInsight) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO erp_ai_sync_insights (
                erp_sync_log_id,
                erp_connection_id,
                insight_type,
                severity,
                insight_title,
                insight_text,
                ai_explanation,
                recommendations,
                actionable
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            sync_log_id,
            sync_log.erp_connection_id,
            insight.insight_type,
            insight.severity,
            insight.title,
            insight.explanation,
            insight.explanation,
            serde_json::to_value(&insight.recommendations)?,
            insight.actionable
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    async fn save_conflict_resolution(
        &self,
        connection_id: Uuid,
        conflict: &ConflictData,
        resolution: &ConflictResolutionSuggestion,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO erp_ai_conflict_resolutions (
                erp_connection_id,
                conflict_type,
                atlas_inventory_id,
                erp_item_id,
                conflict_data,
                ai_suggested_resolution,
                ai_reasoning,
                confidence_score,
                risk_level
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            connection_id,
            resolution.conflict_type,
            conflict.atlas_inventory_id,
            conflict.erp_item_id,
            serde_json::to_value(&conflict)?,
            resolution.suggested_resolution,
            resolution.reasoning,
            resolution.confidence_score,
            resolution.risk_level
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }
}
