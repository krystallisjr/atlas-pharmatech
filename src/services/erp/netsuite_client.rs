// NetSuite OAuth 1.0 (TBA - Token-Based Authentication) Client
// Implements RFC 5849 OAuth 1.0a with HMAC-SHA256 signatures
// Production-ready with comprehensive error handling and retry logic

use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use thiserror::Error;

type HmacSha256 = Hmac<Sha256>;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum NetSuiteError {
    #[error("NetSuite API error ({0}): {1}")]
    ApiError(StatusCode, String),

    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Invalid OAuth signature")]
    InvalidSignature,

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
}

pub type Result<T> = std::result::Result<T, NetSuiteError>;

// ============================================================================
// Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct NetSuiteConfig {
    pub account_id: String,
    pub consumer_key: String,
    pub consumer_secret: String,
    pub token_id: String,
    pub token_secret: String,
    pub realm: Option<String>,  // Optional realm (defaults to account_id)
}

impl NetSuiteConfig {
    pub fn validate(&self) -> Result<()> {
        if self.account_id.is_empty() {
            return Err(NetSuiteError::ConfigError("account_id is required".to_string()));
        }
        if self.consumer_key.is_empty() {
            return Err(NetSuiteError::ConfigError("consumer_key is required".to_string()));
        }
        if self.consumer_secret.is_empty() {
            return Err(NetSuiteError::ConfigError("consumer_secret is required".to_string()));
        }
        if self.token_id.is_empty() {
            return Err(NetSuiteError::ConfigError("token_id is required".to_string()));
        }
        if self.token_secret.is_empty() {
            return Err(NetSuiteError::ConfigError("token_secret is required".to_string()));
        }
        Ok(())
    }
}

// ============================================================================
// Data Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetSuiteInventoryItem {
    pub id: String,
    #[serde(rename = "itemId")]
    pub item_id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "quantityOnHand")]
    pub quantity_on_hand: Option<f64>,
    pub locations: Option<NetSuiteLocations>,

    // Custom pharmaceutical fields (configured in NetSuite)
    #[serde(rename = "custitem_ndc_code")]
    pub ndc_code: Option<String>,
    #[serde(rename = "custitem_lot_number")]
    pub lot_number: Option<String>,
    #[serde(rename = "custitem_expiry_date")]
    pub expiry_date: Option<String>,

    // Standard fields
    pub cost: Option<f64>,
    pub manufacturer: Option<NetSuiteManufacturer>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetSuiteLocations {
    pub items: Vec<NetSuiteLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetSuiteLocation {
    pub location: NetSuiteLocationRef,
    #[serde(rename = "quantityOnHand")]
    pub quantity_on_hand: Option<f64>,
    #[serde(rename = "quantityAvailable")]
    pub quantity_available: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetSuiteLocationRef {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetSuiteManufacturer {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetSuiteListResponse<T> {
    pub items: Vec<T>,
    #[serde(rename = "hasMore")]
    pub has_more: bool,
    #[serde(rename = "totalResults")]
    pub total_results: Option<i32>,
    pub offset: i32,
    pub count: i32,
}

#[derive(Debug, Serialize)]
pub struct NetSuiteSearchParams {
    pub q: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub fields: Option<Vec<String>>,
}

// ============================================================================
// NetSuite Client
// ============================================================================

pub struct NetSuiteClient {
    config: NetSuiteConfig,
    base_url: String,
    http_client: Client,
}

impl NetSuiteClient {
    /// Create a new NetSuite client
    pub fn new(config: NetSuiteConfig) -> Result<Self> {
        config.validate()?;

        let base_url = format!(
            "https://{}.suitetalk.api.netsuite.com/services/rest/record/v1",
            config.account_id
        );

        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| NetSuiteError::NetworkError(e))?;

        Ok(Self {
            config,
            base_url,
            http_client,
        })
    }

    // ========================================================================
    // Inventory Operations
    // ========================================================================

    /// Get a single inventory item by ID
    pub async fn get_inventory_item(&self, item_id: &str) -> Result<NetSuiteInventoryItem> {
        let url = format!("{}/inventoryItem/{}", self.base_url, item_id);
        let response = self.get(&url, &[]).await?;
        self.parse_response(response).await
    }

    /// Search inventory items
    pub async fn search_inventory(
        &self,
        params: NetSuiteSearchParams,
    ) -> Result<NetSuiteListResponse<NetSuiteInventoryItem>> {
        let url = format!("{}/inventoryItem", self.base_url);

        let mut query_params = vec![];
        if let Some(q) = params.q {
            query_params.push(("q", q));
        }
        if let Some(limit) = params.limit {
            query_params.push(("limit", limit.to_string()));
        }
        if let Some(offset) = params.offset {
            query_params.push(("offset", offset.to_string()));
        }
        if let Some(fields) = params.fields {
            query_params.push(("fields", fields.join(",")));
        }

        let response = self.get(&url, &query_params).await?;
        self.parse_response(response).await
    }

    /// Update inventory item quantity
    pub async fn update_inventory_quantity(
        &self,
        item_id: &str,
        location_id: &str,
        new_quantity: f64,
    ) -> Result<()> {
        let url = format!("{}/inventoryItem/{}", self.base_url, item_id);

        let payload = serde_json::json!({
            "locations": {
                "items": [{
                    "location": {"id": location_id},
                    "quantityOnHand": new_quantity
                }]
            }
        });

        let response = self.patch(&url, &[], &payload).await?;
        self.check_success(response).await?;
        Ok(())
    }

    /// Update custom fields (lot number, expiry date, etc.)
    pub async fn update_custom_fields(
        &self,
        item_id: &str,
        fields: &HashMap<String, String>,
    ) -> Result<()> {
        let url = format!("{}/inventoryItem/{}", self.base_url, item_id);

        let payload = serde_json::json!(fields);
        let response = self.patch(&url, &[], &payload).await?;
        self.check_success(response).await?;
        Ok(())
    }

    /// Create a new inventory item
    pub async fn create_inventory_item(
        &self,
        payload: &serde_json::Value,
    ) -> Result<String> {
        let url = format!("{}/inventoryItem", self.base_url);
        let response = self.post(&url, &[], payload).await?;

        #[derive(Deserialize)]
        struct CreateResponse {
            id: String,
        }

        let create_response: CreateResponse = self.parse_response(response).await?;
        Ok(create_response.id)
    }

    // ========================================================================
    // Purchase Order Operations
    // ========================================================================

    /// Create a purchase order
    pub async fn create_purchase_order(
        &self,
        payload: &serde_json::Value,
    ) -> Result<String> {
        let url = format!("{}/purchaseOrder", self.base_url);
        let response = self.post(&url, &[], payload).await?;

        #[derive(Deserialize)]
        struct CreateResponse {
            id: String,
        }

        let create_response: CreateResponse = self.parse_response(response).await?;
        Ok(create_response.id)
    }

    // ========================================================================
    // HTTP Methods with OAuth 1.0 Signatures
    // ========================================================================

    async fn get(&self, url: &str, query_params: &[(&str, String)]) -> Result<Response> {
        let auth_header = self.generate_oauth_header("GET", url, query_params)?;

        let mut request = self.http_client.get(url).header("Authorization", auth_header);

        for (key, value) in query_params {
            request = request.query(&[(key, value)]);
        }

        self.execute_with_retry(request).await
    }

    async fn post(
        &self,
        url: &str,
        query_params: &[(&str, String)],
        body: &serde_json::Value,
    ) -> Result<Response> {
        let auth_header = self.generate_oauth_header("POST", url, query_params)?;

        let request = self
            .http_client
            .post(url)
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .json(body);

        self.execute_with_retry(request).await
    }

    async fn patch(
        &self,
        url: &str,
        query_params: &[(&str, String)],
        body: &serde_json::Value,
    ) -> Result<Response> {
        let auth_header = self.generate_oauth_header("PATCH", url, query_params)?;

        let request = self
            .http_client
            .patch(url)
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .json(body);

        self.execute_with_retry(request).await
    }

    // ========================================================================
    // OAuth 1.0 Signature Generation (RFC 5849)
    // ========================================================================

    fn generate_oauth_header(
        &self,
        method: &str,
        url: &str,
        query_params: &[(&str, String)],
    ) -> Result<String> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| NetSuiteError::AuthError("Failed to get timestamp".to_string()))?
            .as_secs()
            .to_string();

        let nonce = uuid::Uuid::new_v4().to_string().replace("-", "");

        let realm = self
            .config
            .realm
            .clone()
            .unwrap_or_else(|| self.config.account_id.clone());

        // OAuth parameters
        let mut oauth_params = vec![
            ("oauth_consumer_key", self.config.consumer_key.as_str()),
            ("oauth_token", self.config.token_id.as_str()),
            ("oauth_signature_method", "HMAC-SHA256"),
            ("oauth_timestamp", &timestamp),
            ("oauth_nonce", &nonce),
            ("oauth_version", "1.0"),
        ];

        // Generate signature
        let signature = self.generate_signature(method, url, &oauth_params, query_params)?;
        oauth_params.push(("oauth_signature", &signature));

        // Build Authorization header
        let header_value = format!(
            "OAuth realm=\"{}\",{}",
            realm,
            oauth_params
                .iter()
                .map(|(k, v)| format!("{}=\"{}\"", k, percent_encode(v)))
                .collect::<Vec<_>>()
                .join(",")
        );

        Ok(header_value)
    }

    fn generate_signature(
        &self,
        method: &str,
        url: &str,
        oauth_params: &[(&str, &str)],
        query_params: &[(&str, String)],
    ) -> Result<String> {
        // Step 1: Collect all parameters (OAuth + query)
        let mut all_params: Vec<(String, String)> = oauth_params
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        for (k, v) in query_params {
            all_params.push((k.to_string(), v.clone()));
        }

        // Step 2: Sort parameters alphabetically
        all_params.sort_by(|a, b| a.0.cmp(&b.0));

        // Step 3: Build parameter string
        let param_string = all_params
            .iter()
            .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        // Step 4: Build signature base string
        let signature_base = format!(
            "{}&{}&{}",
            percent_encode(method),
            percent_encode(url),
            percent_encode(&param_string)
        );

        // Step 5: Build signing key
        let signing_key = format!(
            "{}&{}",
            percent_encode(&self.config.consumer_secret),
            percent_encode(&self.config.token_secret)
        );

        // Step 6: Generate HMAC-SHA256 signature
        let mut mac = HmacSha256::new_from_slice(signing_key.as_bytes())
            .map_err(|_| NetSuiteError::AuthError("Failed to create HMAC".to_string()))?;
        mac.update(signature_base.as_bytes());
        let result = mac.finalize();
        let signature_bytes = result.into_bytes();

        // Step 7: Base64 encode
        Ok(base64::encode(&signature_bytes))
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    async fn execute_with_retry(&self, request: reqwest::RequestBuilder) -> Result<Response> {
        let mut attempts = 0;
        const MAX_RETRIES: u32 = 3;

        loop {
            attempts += 1;

            match request.try_clone() {
                Some(req) => match req.send().await {
                    Ok(response) => {
                        if response.status() == StatusCode::TOO_MANY_REQUESTS && attempts < MAX_RETRIES {
                            // Rate limited - exponential backoff
                            let delay = std::time::Duration::from_secs(2u64.pow(attempts));
                            tokio::time::sleep(delay).await;
                            continue;
                        }
                        return Ok(response);
                    }
                    Err(e) if attempts < MAX_RETRIES => {
                        // Network error - retry with exponential backoff
                        let delay = std::time::Duration::from_secs(2u64.pow(attempts));
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    Err(e) => return Err(NetSuiteError::NetworkError(e)),
                },
                None => {
                    return Err(NetSuiteError::AuthError(
                        "Failed to clone request for retry".to_string(),
                    ))
                }
            }
        }
    }

    async fn parse_response<T: serde::de::DeserializeOwned>(&self, response: Response) -> Result<T> {
        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return match status {
                StatusCode::NOT_FOUND => Err(NetSuiteError::NotFound(error_text)),
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                    Err(NetSuiteError::AuthError(error_text))
                }
                StatusCode::TOO_MANY_REQUESTS => Err(NetSuiteError::RateLimitExceeded),
                _ => Err(NetSuiteError::ApiError(status, error_text)),
            };
        }

        response.json().await.map_err(NetSuiteError::NetworkError)
    }

    async fn check_success(&self, response: Response) -> Result<()> {
        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return match status {
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                    Err(NetSuiteError::AuthError(error_text))
                }
                StatusCode::TOO_MANY_REQUESTS => Err(NetSuiteError::RateLimitExceeded),
                _ => Err(NetSuiteError::ApiError(status, error_text)),
            };
        }

        Ok(())
    }

    /// Test connection to NetSuite
    pub async fn test_connection(&self) -> Result<bool> {
        // Try to fetch a simple endpoint
        let url = format!("{}/inventoryItem", self.base_url);
        let params = vec![("limit", "1".to_string())];
        let response = self.get(&url, &params).await?;

        Ok(response.status().is_success())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn percent_encode(input: &str) -> String {
    utf8_percent_encode(input, NON_ALPHANUMERIC).to_string()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let config = NetSuiteConfig {
            account_id: "".to_string(),
            consumer_key: "test".to_string(),
            consumer_secret: "test".to_string(),
            token_id: "test".to_string(),
            token_secret: "test".to_string(),
            realm: None,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_percent_encode() {
        assert_eq!(percent_encode("hello world"), "hello%20world");
        assert_eq!(percent_encode("test@example.com"), "test%40example%2Ecom");
    }
}
