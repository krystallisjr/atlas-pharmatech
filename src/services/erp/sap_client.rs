// SAP S/4HANA OAuth 2.0 Client with OData API
// Implements OAuth 2.0 Client Credentials flow with automatic token refresh
// Production-ready with comprehensive error handling and CSRF token management

use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use chrono::{DateTime, Duration, Utc};
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum SapError {
    #[error("SAP API error ({0}): {1}")]
    ApiError(StatusCode, String),

    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("CSRF token required but not available")]
    CsrfTokenError,

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Item not found: {0}")]
    NotFound(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    #[error("OData error: {0}")]
    ODataError(String),
}

pub type Result<T> = std::result::Result<T, SapError>;

// ============================================================================
// Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct SapConfig {
    pub base_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub token_endpoint: String,
    pub environment: SapEnvironment,
    pub plant: Option<String>,  // Default plant for inventory operations
    pub company_code: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SapEnvironment {
    Cloud,
    OnPremise,
}

impl SapConfig {
    pub fn validate(&self) -> Result<()> {
        if self.base_url.is_empty() {
            return Err(SapError::ConfigError("base_url is required".to_string()));
        }
        if self.client_id.is_empty() {
            return Err(SapError::ConfigError("client_id is required".to_string()));
        }
        if self.client_secret.is_empty() {
            return Err(SapError::ConfigError("client_secret is required".to_string()));
        }
        if self.token_endpoint.is_empty() {
            return Err(SapError::ConfigError("token_endpoint is required".to_string()));
        }
        Ok(())
    }
}

// ============================================================================
// Data Models - OData Responses
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ODataResponse<T> {
    pub d: ODataData<T>,
}

#[derive(Debug, Deserialize)]
pub struct ODataData<T> {
    pub results: Vec<T>,
}

#[derive(Debug, Deserialize)]
pub struct ODataSingleResponse<T> {
    pub d: T,
}

// Material Stock
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct MaterialStock {
    pub material: String,
    pub plant: String,
    pub storage_location: String,
    #[serde(rename = "MatlWrhsStkQtyInMatlBaseUnit")]
    pub stock_quantity: String,  // SAP returns as string
    pub material_base_unit: String,
    #[serde(rename = "SDDocument")]
    pub sd_document: Option<String>,
}

// Material Document Header (for goods movements)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MaterialDocumentHeader {
    pub material_document: String,
    pub material_document_year: String,
    pub posting_date: String,
    pub document_date: String,
    pub goods_movement_code: String,
}

// Material Document Item
#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct MaterialDocumentItem {
    pub material: String,
    pub plant: String,
    pub storage_location: String,
    pub goods_movement_type: String,  // "501" = receipt without PO, "101" = GR for PO
    pub quantity_in_entry_unit: String,
    pub entry_unit: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch: Option<String>,

    // Custom pharmaceutical fields (requires configuration in SAP)
    #[serde(rename = "YY1_ExpiryDate_MDI", skip_serializing_if = "Option::is_none")]
    pub expiry_date: Option<String>,
    #[serde(rename = "YY1_NDCCode_MDI", skip_serializing_if = "Option::is_none")]
    pub ndc_code: Option<String>,
}

// Product Master Data
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Product {
    pub product: String,
    pub product_type: String,
    #[serde(rename = "ProductDescription")]
    pub description: Option<String>,
    pub base_unit: String,
    pub product_group: Option<String>,
    pub manufacturer: Option<String>,
}

// Goods Movement Request
#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct GoodsMovement {
    pub goods_movement_code: String,  // "01" = Goods receipt
    pub posting_date: String,         // YYYY-MM-DD
    pub document_date: String,        // YYYY-MM-DD
    pub to_material_document_item: MaterialDocumentItems,
}

#[derive(Debug, Serialize)]
pub struct MaterialDocumentItems {
    pub results: Vec<MaterialDocumentItem>,
}

// ============================================================================
// Token Cache
// ============================================================================

#[derive(Debug, Clone)]
struct TokenCache {
    access_token: String,
    expires_at: DateTime<Utc>,
}

// ============================================================================
// SAP Client
// ============================================================================

pub struct SapClient {
    config: SapConfig,
    http_client: Client,
    token_cache: Arc<RwLock<Option<TokenCache>>>,
}

impl SapClient {
    /// Create a new SAP client
    pub fn new(config: SapConfig) -> Result<Self> {
        config.validate()?;

        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .cookie_store(true)  // Required for CSRF token handling
            .build()
            .map_err(SapError::NetworkError)?;

        Ok(Self {
            config,
            http_client,
            token_cache: Arc::new(RwLock::new(None)),
        })
    }

    // ========================================================================
    // Material Stock Operations
    // ========================================================================

    /// Get material stock for a specific plant and storage location
    pub async fn get_material_stock(
        &self,
        material_number: &str,
        plant: &str,
        storage_location: &str,
    ) -> Result<MaterialStock> {
        let token = self.get_access_token().await?;

        let url = format!(
            "{}/sap/opu/odata/sap/API_MATERIAL_STOCK_SRV/MaterialStock",
            self.config.base_url
        );

        let filter = format!(
            "Material eq '{}' and Plant eq '{}' and StorageLocation eq '{}'",
            material_number, plant, storage_location
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .header("Accept", "application/json")
            .query(&[("$filter", filter)])
            .send()
            .await?;

        self.handle_odata_response::<MaterialStock>(response)
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| SapError::NotFound("Material not found".to_string()))
    }

    /// Get all stock for a material across all locations
    pub async fn get_material_stock_all_locations(
        &self,
        material_number: &str,
    ) -> Result<Vec<MaterialStock>> {
        let token = self.get_access_token().await?;

        let url = format!(
            "{}/sap/opu/odata/sap/API_MATERIAL_STOCK_SRV/MaterialStock",
            self.config.base_url
        );

        let filter = format!("Material eq '{}'", material_number);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .header("Accept", "application/json")
            .query(&[("$filter", filter)])
            .send()
            .await?;

        self.handle_odata_response::<MaterialStock>(response).await
    }

    // ========================================================================
    // Goods Movement Operations
    // ========================================================================

    /// Post a goods movement (inventory adjustment)
    pub async fn post_goods_movement(&self, movement: GoodsMovement) -> Result<String> {
        let token = self.get_access_token().await?;

        // Get CSRF token (required for POST/PATCH/DELETE)
        let csrf_token = self.get_csrf_token(&token).await?;

        let url = format!(
            "{}/sap/opu/odata/sap/API_MATERIAL_DOCUMENT_SRV/A_MaterialDocumentHeader",
            self.config.base_url
        );

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .header("X-CSRF-Token", csrf_token)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&movement)
            .send()
            .await?;

        let result: ODataSingleResponse<MaterialDocumentHeader> = self.parse_response(response).await?;
        Ok(result.d.material_document)
    }

    /// Create inventory adjustment (movement type 501 - receipt without PO)
    pub async fn adjust_inventory(
        &self,
        material: &str,
        plant: &str,
        storage_location: &str,
        quantity_delta: f64,
        unit: &str,
        batch: Option<String>,
        expiry_date: Option<String>,
        ndc_code: Option<String>,
    ) -> Result<String> {
        let item = MaterialDocumentItem {
            material: material.to_string(),
            plant: plant.to_string(),
            storage_location: storage_location.to_string(),
            goods_movement_type: "501".to_string(),
            quantity_in_entry_unit: quantity_delta.to_string(),
            entry_unit: unit.to_string(),
            batch,
            expiry_date,
            ndc_code,
        };

        let movement = GoodsMovement {
            goods_movement_code: "01".to_string(),
            posting_date: Utc::now().format("%Y-%m-%d").to_string(),
            document_date: Utc::now().format("%Y-%m-%d").to_string(),
            to_material_document_item: MaterialDocumentItems {
                results: vec![item],
            },
        };

        self.post_goods_movement(movement).await
    }

    // ========================================================================
    // Product Master Data
    // ========================================================================

    /// Get product master data
    pub async fn get_product(&self, material_number: &str) -> Result<Product> {
        let token = self.get_access_token().await?;

        let url = format!(
            "{}/sap/opu/odata/sap/API_PRODUCT_SRV/A_Product('{}')",
            self.config.base_url, material_number
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .header("Accept", "application/json")
            .send()
            .await?;

        let result: ODataSingleResponse<Product> = self.parse_response(response).await?;
        Ok(result.d)
    }

    /// Search products
    pub async fn search_products(&self, search_term: &str) -> Result<Vec<Product>> {
        let token = self.get_access_token().await?;

        let url = format!(
            "{}/sap/opu/odata/sap/API_PRODUCT_SRV/A_Product",
            self.config.base_url
        );

        let filter = format!("contains(Product,'{}') or contains(ProductDescription,'{}')", search_term, search_term);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .header("Accept", "application/json")
            .query(&[("$filter", filter), ("$top", "100".to_string())])
            .send()
            .await?;

        self.handle_odata_response::<Product>(response).await
    }

    // ========================================================================
    // OAuth 2.0 Token Management
    // ========================================================================

    async fn get_access_token(&self) -> Result<String> {
        // Check cache first
        {
            let cache = self.token_cache.read().unwrap();
            if let Some(cached) = &*cache {
                // Check if token is still valid (with 10-minute buffer)
                let buffer = Duration::minutes(10);
                if Utc::now() + buffer < cached.expires_at {
                    return Ok(cached.access_token.clone());
                }
            }
        }

        // Request new token
        let token = self.request_new_token().await?;

        // Update cache
        {
            let mut cache = self.token_cache.write().unwrap();
            *cache = Some(token.clone());
        }

        Ok(token.access_token)
    }

    async fn request_new_token(&self) -> Result<TokenCache> {
        let credentials = base64::encode(format!("{}:{}", self.config.client_id, self.config.client_secret));

        let response = self
            .http_client
            .post(&self.config.token_endpoint)
            .header("Authorization", format!("Basic {}", credentials))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body("grant_type=client_credentials")
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(SapError::AuthError(error_text));
        }

        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
            expires_in: i64,  // Seconds
        }

        let token_response: TokenResponse = response.json().await?;

        let expires_at = Utc::now() + Duration::seconds(token_response.expires_in);

        Ok(TokenCache {
            access_token: token_response.access_token,
            expires_at,
        })
    }

    // ========================================================================
    // CSRF Token Management (Required for Write Operations)
    // ========================================================================

    async fn get_csrf_token(&self, access_token: &str) -> Result<String> {
        let url = format!(
            "{}/sap/opu/odata/sap/API_MATERIAL_DOCUMENT_SRV",
            self.config.base_url
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(access_token)
            .header("X-CSRF-Token", "Fetch")
            .send()
            .await?;

        response
            .headers()
            .get("X-CSRF-Token")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .ok_or(SapError::CsrfTokenError)
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    async fn handle_odata_response<T: serde::de::DeserializeOwned>(
        &self,
        response: Response,
    ) -> Result<Vec<T>> {
        let status = response.status();

        if !status.is_success() {
            return self.handle_error_response(response).await;
        }

        let odata_response: ODataResponse<T> = response.json().await?;
        Ok(odata_response.d.results)
    }

    async fn parse_response<T: serde::de::DeserializeOwned>(&self, response: Response) -> Result<T> {
        let status = response.status();

        if !status.is_success() {
            return self.handle_error_response(response).await;
        }

        response.json().await.map_err(SapError::NetworkError)
    }

    async fn handle_error_response<T>(&self, response: Response) -> Result<T> {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());

        Err(match status {
            StatusCode::NOT_FOUND => SapError::NotFound(error_text),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => SapError::AuthError(error_text),
            StatusCode::TOO_MANY_REQUESTS => SapError::RateLimitExceeded,
            _ => SapError::ApiError(status, error_text),
        })
    }

    /// Test connection to SAP
    pub async fn test_connection(&self) -> Result<bool> {
        // Try to get an access token
        let token = self.get_access_token().await?;

        // Try a simple API call
        let url = format!(
            "{}/sap/opu/odata/sap/API_MATERIAL_STOCK_SRV/$metadata",
            self.config.base_url
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await?;

        Ok(response.status().is_success())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let config = SapConfig {
            base_url: "".to_string(),
            client_id: "test".to_string(),
            client_secret: "test".to_string(),
            token_endpoint: "test".to_string(),
            environment: SapEnvironment::Cloud,
            plant: None,
            company_code: None,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_sap_environment() {
        assert_eq!(SapEnvironment::Cloud, SapEnvironment::Cloud);
        assert_ne!(SapEnvironment::Cloud, SapEnvironment::OnPremise);
    }
}
