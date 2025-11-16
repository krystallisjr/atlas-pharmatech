# ATLAS PHARMA - ERP INTEGRATION TECHNICAL PLAN
**Oracle NetSuite & SAP S/4HANA Bidirectional Sync**

**Target Onboarding Time**: 5 minutes
**Integration Model**: Bidirectional sync with hybrid authentication
**Sync Scope**: Stock levels, product master data, transactions, lot/batch tracking

---

## EXECUTIVE SUMMARY

This document outlines the technical architecture for seamless integration between Atlas Pharma and enterprise ERP systems (Oracle NetSuite and SAP S/4HANA). The solution enables pharmaceutical distributors to onboard in under 5 minutes using secure API key-based authentication, with real-time bidirectional inventory synchronization.

**Key Differentiators:**
- **5-Minute Onboarding**: Customer generates OAuth credentials in their ERP, pastes into Atlas dashboard, selects sync preferences → done
- **Zero IT Support Required**: Self-service setup with intelligent defaults and auto-discovery
- **Bidirectional Sync**: Real-time inventory updates flow both ways (ERP ↔ Atlas)
- **Security-First**: Customer retains full control of credentials; Atlas never accesses ERP admin accounts
- **Pharmaceutical-Specific**: NDC validation, lot/batch tracking, expiry date sync, regulatory compliance

---

## 1. AUTHENTICATION ARCHITECTURE

### A. Oracle NetSuite Integration

**Authentication Method**: Token-Based Authentication (TBA) - OAuth 1.0

#### Customer Setup (3 minutes)
1. **Enable TBA** (one-time setup per NetSuite account)
   - Navigate to: `Setup → Company → Enable Features → SuiteCloud tab`
   - Check: "Token-Based Authentication"
   - Save

2. **Create Integration Record**
   - Navigate to: `Setup → Integration → Manage Integrations → New`
   - Name: "Atlas Pharma Integration"
   - Check: "Token-Based Authentication"
   - Check: "TBA: Authorization Flow" (optional, for future OAuth2 upgrade)
   - **Save** → NetSuite displays:
     - **Consumer Key** (aka Client ID)
     - **Consumer Secret** (aka Client Secret)
   - ⚠️ **Customer must copy these immediately** (only shown once)

3. **Create Access Token**
   - Navigate to: `Setup → Users/Roles → Access Tokens → New`
   - **Application**: Select "Atlas Pharma Integration"
   - **User**: Select service account user (recommended) or admin
   - **Role**: Select role with permissions for inventory read/write
   - **Token Name**: "Atlas Pharma Sync"
   - **Save** → NetSuite displays:
     - **Token ID**
     - **Token Secret**
   - ⚠️ **Customer must copy these immediately** (only shown once)

#### Atlas Integration Points

**What Customer Provides to Atlas**:
```json
{
  "erp_type": "netsuite",
  "account_id": "1234567",  // NetSuite account ID (e.g., "1234567" from account.netsuite.com)
  "consumer_key": "abc123...",
  "consumer_secret": "def456...",
  "token_id": "ghi789...",
  "token_secret": "jkl012..."
}
```

**How Atlas Authenticates**:
- **Protocol**: OAuth 1.0a (NetSuite TBA standard)
- **Signature Method**: HMAC-SHA256
- **Base URL**: `https://{account_id}.suitetalk.api.netsuite.com/services/rest/record/v1/`
- **Request Headers**:
  ```
  Authorization: OAuth realm="{account_id}",
    oauth_consumer_key="{consumer_key}",
    oauth_token="{token_id}",
    oauth_signature_method="HMAC-SHA256",
    oauth_timestamp="{unix_timestamp}",
    oauth_nonce="{random_string}",
    oauth_version="1.0",
    oauth_signature="{calculated_signature}"
  ```

**OAuth 1.0 Signature Generation**:
```rust
// Rust implementation (crate: oauth-client)
use oauth_client::Token;

let consumer = Token::new(consumer_key, consumer_secret);
let access = Token::new(token_id, token_secret);

let signature = oauth_client::sign_hmac_sha256(
    "GET",
    &url,
    &params,
    &consumer,
    &access,
);
```

---

### B. SAP S/4HANA Integration

**Authentication Method**: OAuth 2.0 with Client Credentials Flow

#### Customer Setup (3 minutes)

1. **Create Communication System** (S/4HANA Cloud)
   - Navigate to: `Communication Management → Communication Systems → New`
   - **System ID**: "ATLAS_PHARMA"
   - **Host Name**: atlas.pharma (placeholder)
   - **OAuth 2.0 Settings**:
     - Check: "OAuth 2.0 Client Credentials"
     - **Client ID**: Auto-generated (copy this)
     - **Client Secret**: Auto-generated (copy this)
   - Save

2. **Create Communication Arrangement**
   - Navigate to: `Communication Management → Communication Arrangements → New`
   - **Scenario**: Select appropriate scenario:
     - `SAP_COM_0108` - Material Documents (Inventory Management)
     - `SAP_COM_0164` - Product Master Data
     - Custom scenario for Atlas integration
   - **Communication System**: Select "ATLAS_PHARMA"
   - **Inbound/Outbound**: Enable both
   - **Authentication**: OAuth 2.0
   - Save

3. **Assign User Permissions**
   - Create/use a technical communication user
   - Assign business roles:
     - `BR_EMPLOYEE` (minimum)
     - `BR_INVENTORY_MANAGER_EXT` (for inventory operations)
     - `BR_BUSINESS_USER` (for product master data)

#### Atlas Integration Points

**What Customer Provides to Atlas**:
```json
{
  "erp_type": "sap_s4hana",
  "environment": "cloud",  // or "on-premise"
  "base_url": "https://my12345.s4hana.cloud.sap",  // Customer's SAP URL
  "client_id": "abc123...",
  "client_secret": "def456...",
  "token_endpoint": "https://my12345.s4hana.cloud.sap/sap/bc/sec/oauth2/token",
  "scope": "API_MATERIAL_DOCUMENT_SRV_0001 API_PRODUCT_SRV_0001"  // Auto-filled by Atlas
}
```

**How Atlas Authenticates**:
- **Protocol**: OAuth 2.0 Client Credentials Flow
- **Token Request**:
  ```http
  POST {token_endpoint}
  Content-Type: application/x-www-form-urlencoded
  Authorization: Basic {base64(client_id:client_secret)}

  grant_type=client_credentials&scope={scope}
  ```

- **Response**:
  ```json
  {
    "access_token": "xyz789...",
    "token_type": "Bearer",
    "expires_in": 3600
  }
  ```

- **API Requests**:
  ```http
  GET {base_url}/sap/opu/odata/sap/API_MATERIAL_DOCUMENT_SRV/A_MaterialDocumentHeader
  Authorization: Bearer {access_token}
  Accept: application/json
  ```

**Token Refresh Strategy**:
- Atlas caches access token for 50 minutes (10-minute buffer before 1-hour expiry)
- Automatic refresh before expiration
- Retry logic if token expires mid-request

---

## 2. ATLAS BACKEND IMPLEMENTATION

### A. Database Schema

**New Tables**:

```sql
-- ERP Connection Configuration
CREATE TABLE erp_connections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    erp_type VARCHAR(50) NOT NULL,  -- 'netsuite' or 'sap_s4hana'

    -- NetSuite specific (encrypted)
    netsuite_account_id VARCHAR(50),
    netsuite_consumer_key TEXT,  -- Encrypted with AES-256-GCM
    netsuite_consumer_secret TEXT,  -- Encrypted
    netsuite_token_id TEXT,  -- Encrypted
    netsuite_token_secret TEXT,  -- Encrypted

    -- SAP specific (encrypted)
    sap_base_url VARCHAR(255),
    sap_client_id TEXT,  -- Encrypted
    sap_client_secret TEXT,  -- Encrypted
    sap_token_endpoint VARCHAR(255),
    sap_environment VARCHAR(20),  -- 'cloud' or 'on_premise'

    -- OAuth token cache (encrypted, short-lived)
    cached_access_token TEXT,  -- Encrypted, for SAP OAuth2
    token_expires_at TIMESTAMPTZ,

    -- Sync configuration
    sync_enabled BOOLEAN DEFAULT true,
    sync_frequency_minutes INTEGER DEFAULT 15,  -- Real-time via webhooks preferred
    last_sync_at TIMESTAMPTZ,
    last_sync_status VARCHAR(50),  -- 'success', 'failed', 'partial'
    last_sync_error TEXT,

    -- Feature flags
    sync_stock_levels BOOLEAN DEFAULT true,
    sync_product_master BOOLEAN DEFAULT true,
    sync_transactions BOOLEAN DEFAULT true,
    sync_lot_batch BOOLEAN DEFAULT true,

    -- Mapping preferences
    field_mappings JSONB,  -- Custom field mappings if needed

    -- Metadata
    connection_name VARCHAR(100),
    status VARCHAR(20) DEFAULT 'active',  -- 'active', 'paused', 'error'
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(user_id, erp_type, netsuite_account_id),
    UNIQUE(user_id, erp_type, sap_base_url)
);

CREATE INDEX idx_erp_connections_user ON erp_connections(user_id);
CREATE INDEX idx_erp_connections_status ON erp_connections(status) WHERE sync_enabled = true;


-- ERP Sync Mapping (links Atlas inventory to ERP items)
CREATE TABLE erp_inventory_mappings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    erp_connection_id UUID NOT NULL REFERENCES erp_connections(id) ON DELETE CASCADE,
    atlas_inventory_id UUID NOT NULL REFERENCES inventory(id) ON DELETE CASCADE,

    -- ERP-side identifiers
    erp_item_id VARCHAR(100) NOT NULL,  -- NetSuite internal ID or SAP material number
    erp_item_name VARCHAR(255),
    erp_location_id VARCHAR(100),  -- Warehouse/location in ERP

    -- Sync metadata
    last_synced_at TIMESTAMPTZ,
    sync_direction VARCHAR(20),  -- 'atlas_to_erp', 'erp_to_atlas', 'bidirectional'
    conflict_resolution VARCHAR(20) DEFAULT 'atlas_wins',  -- 'atlas_wins', 'erp_wins', 'manual'

    -- Field-level mapping
    quantity_field_mapping VARCHAR(100),  -- Custom field name in ERP for quantity
    expiry_field_mapping VARCHAR(100),
    lot_field_mapping VARCHAR(100),

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(erp_connection_id, atlas_inventory_id),
    UNIQUE(erp_connection_id, erp_item_id, erp_location_id)
);

CREATE INDEX idx_erp_mappings_connection ON erp_inventory_mappings(erp_connection_id);
CREATE INDEX idx_erp_mappings_inventory ON erp_inventory_mappings(atlas_inventory_id);


-- Sync History & Audit Trail
CREATE TABLE erp_sync_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    erp_connection_id UUID NOT NULL REFERENCES erp_connections(id) ON DELETE CASCADE,

    sync_type VARCHAR(50) NOT NULL,  -- 'full_sync', 'incremental', 'real_time', 'manual'
    sync_direction VARCHAR(20) NOT NULL,  -- 'atlas_to_erp', 'erp_to_atlas', 'bidirectional'

    -- Sync results
    status VARCHAR(20) NOT NULL,  -- 'running', 'success', 'failed', 'partial'
    items_synced INTEGER DEFAULT 0,
    items_failed INTEGER DEFAULT 0,
    items_skipped INTEGER DEFAULT 0,

    -- Error tracking
    error_message TEXT,
    error_details JSONB,  -- Detailed error per item

    -- Performance metrics
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    duration_seconds INTEGER,

    -- API metrics
    api_calls_made INTEGER DEFAULT 0,
    api_errors INTEGER DEFAULT 0,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_sync_logs_connection ON erp_sync_logs(erp_connection_id, created_at DESC);
CREATE INDEX idx_sync_logs_status ON erp_sync_logs(status, created_at DESC);


-- Webhook Configuration (for real-time updates from ERP)
CREATE TABLE erp_webhooks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    erp_connection_id UUID NOT NULL REFERENCES erp_connections(id) ON DELETE CASCADE,

    webhook_url VARCHAR(500) NOT NULL,  -- Atlas webhook endpoint
    webhook_secret VARCHAR(100) NOT NULL,  -- For signature verification

    -- ERP-specific webhook ID (for unsubscribing)
    erp_webhook_id VARCHAR(100),

    event_types JSONB NOT NULL,  -- ['inventory.updated', 'item.created', etc.]

    status VARCHAR(20) DEFAULT 'active',  -- 'active', 'paused', 'failed'
    last_received_at TIMESTAMPTZ,

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_webhooks_connection ON erp_webhooks(erp_connection_id);
```

---

### B. Rust Services Architecture

**Service Structure**:
```
src/services/erp/
├── mod.rs                         # Module exports
├── erp_connection_service.rs     # Manage connections, test connectivity
├── netsuite_client.rs             # NetSuite API client (OAuth 1.0)
├── sap_client.rs                  # SAP OData client (OAuth 2.0)
├── erp_sync_service.rs            # Orchestrate bidirectional sync
├── field_mapper.rs                # Map Atlas fields ↔ ERP fields
├── conflict_resolver.rs           # Handle sync conflicts
└── webhook_handler.rs             # Process incoming ERP webhooks
```

#### 1. **NetSuite Client** (`netsuite_client.rs`)

```rust
use oauth_client::{Token, sign_hmac_sha256};
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct NetSuiteClient {
    account_id: String,
    base_url: String,
    consumer: Token,
    access: Token,
    http_client: Client,
}

impl NetSuiteClient {
    pub fn new(config: NetSuiteConfig) -> Result<Self> {
        Ok(Self {
            account_id: config.account_id.clone(),
            base_url: format!(
                "https://{}.suitetalk.api.netsuite.com/services/rest/record/v1",
                config.account_id
            ),
            consumer: Token::new(config.consumer_key, config.consumer_secret),
            access: Token::new(config.token_id, config.token_secret),
            http_client: Client::new(),
        })
    }

    /// Get inventory item by internal ID
    pub async fn get_inventory_item(&self, item_id: &str) -> Result<NetSuiteInventoryItem> {
        let url = format!("{}/inventoryItem/{}", self.base_url, item_id);
        let auth_header = self.generate_oauth_header("GET", &url, &[])?;

        let response = self.http_client
            .get(&url)
            .header("Authorization", auth_header)
            .header("Accept", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(NetSuiteError::ApiError(response.status(), response.text().await?));
        }

        Ok(response.json().await?)
    }

    /// Update inventory item quantity
    pub async fn update_inventory_quantity(
        &self,
        item_id: &str,
        location_id: &str,
        new_quantity: i32,
    ) -> Result<()> {
        let url = format!("{}/inventoryItem/{}", self.base_url, item_id);

        let payload = json!({
            "locations": {
                "items": [{
                    "location": {"id": location_id},
                    "quantityOnHand": new_quantity
                }]
            }
        });

        let auth_header = self.generate_oauth_header("PATCH", &url, &[])?;

        let response = self.http_client
            .patch(&url)
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(NetSuiteError::ApiError(response.status(), response.text().await?));
        }

        Ok(())
    }

    /// Search inventory items (with filters)
    pub async fn search_inventory(
        &self,
        filters: &NetSuiteSearchFilters,
    ) -> Result<Vec<NetSuiteInventoryItem>> {
        let url = format!("{}/inventoryItem", self.base_url);
        let query_params = filters.to_query_params();

        let auth_header = self.generate_oauth_header("GET", &url, &query_params)?;

        let response = self.http_client
            .get(&url)
            .header("Authorization", auth_header)
            .query(&query_params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(NetSuiteError::ApiError(response.status(), response.text().await?));
        }

        let result: NetSuiteSearchResponse = response.json().await?;
        Ok(result.items)
    }

    /// Create purchase order from Atlas transaction
    pub async fn create_purchase_order(
        &self,
        order: &AtlasTransaction,
    ) -> Result<String> {
        // Map Atlas transaction to NetSuite PO format
        let payload = self.map_atlas_to_netsuite_po(order)?;

        let url = format!("{}/purchaseOrder", self.base_url);
        let auth_header = self.generate_oauth_header("POST", &url, &[])?;

        let response = self.http_client
            .post(&url)
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(NetSuiteError::ApiError(response.status(), response.text().await?));
        }

        let result: NetSuiteCreateResponse = response.json().await?;
        Ok(result.id)
    }

    /// Generate OAuth 1.0 Authorization header
    fn generate_oauth_header(
        &self,
        method: &str,
        url: &str,
        params: &[(String, String)],
    ) -> Result<String> {
        let realm = &self.account_id;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();
        let nonce = uuid::Uuid::new_v4().to_string();

        let mut oauth_params = vec![
            ("oauth_consumer_key", self.consumer.key.as_str()),
            ("oauth_token", self.access.key.as_str()),
            ("oauth_signature_method", "HMAC-SHA256"),
            ("oauth_timestamp", &timestamp),
            ("oauth_nonce", &nonce),
            ("oauth_version", "1.0"),
        ];

        let signature = sign_hmac_sha256(
            method,
            url,
            params,
            &self.consumer,
            &self.access,
        )?;

        oauth_params.push(("oauth_signature", &signature));

        Ok(format!(
            "OAuth realm=\"{}\",{}",
            realm,
            oauth_params.iter()
                .map(|(k, v)| format!("{}=\"{}\"", k, percent_encode(v)))
                .collect::<Vec<_>>()
                .join(",")
        ))
    }
}

#[derive(Debug, Deserialize)]
struct NetSuiteInventoryItem {
    id: String,
    #[serde(rename = "itemId")]
    item_id: String,
    #[serde(rename = "displayName")]
    display_name: String,
    #[serde(rename = "quantityOnHand")]
    quantity_on_hand: Option<i32>,
    locations: Option<NetSuiteLocations>,
    // Custom fields for pharma
    #[serde(rename = "custitem_ndc_code")]
    ndc_code: Option<String>,
    #[serde(rename = "custitem_lot_number")]
    lot_number: Option<String>,
    #[serde(rename = "custitem_expiry_date")]
    expiry_date: Option<String>,
}

#[derive(Debug, Serialize)]
struct NetSuiteSearchFilters {
    q: Option<String>,  // Search query
    limit: Option<i32>,  // Pagination
    offset: Option<i32>,
    // NetSuite-specific filters
    #[serde(rename = "fields")]
    fields: Vec<String>,  // Fields to return
}
```

#### 2. **SAP Client** (`sap_client.rs`)

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

pub struct SapClient {
    base_url: String,
    client_id: String,
    client_secret: String,
    token_endpoint: String,
    http_client: Client,

    // Cached access token
    cached_token: Option<String>,
    token_expires_at: Option<DateTime<Utc>>,
}

impl SapClient {
    pub fn new(config: SapConfig) -> Self {
        Self {
            base_url: config.base_url,
            client_id: config.client_id,
            client_secret: config.client_secret,
            token_endpoint: config.token_endpoint,
            http_client: Client::new(),
            cached_token: None,
            token_expires_at: None,
        }
    }

    /// Get valid access token (refresh if needed)
    async fn get_access_token(&mut self) -> Result<String> {
        // Check if cached token is still valid (with 10-minute buffer)
        if let (Some(token), Some(expires_at)) = (&self.cached_token, &self.token_expires_at) {
            let buffer = chrono::Duration::minutes(10);
            if Utc::now() + buffer < *expires_at {
                return Ok(token.clone());
            }
        }

        // Request new token
        let credentials = format!("{}:{}", self.client_id, self.client_secret);
        let auth_header = format!("Basic {}", base64::encode(credentials));

        let response = self.http_client
            .post(&self.token_endpoint)
            .header("Authorization", auth_header)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body("grant_type=client_credentials")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(SapError::AuthError(response.text().await?));
        }

        let token_response: SapTokenResponse = response.json().await?;

        // Cache token
        self.cached_token = Some(token_response.access_token.clone());
        self.token_expires_at = Some(
            Utc::now() + chrono::Duration::seconds(token_response.expires_in as i64)
        );

        Ok(token_response.access_token)
    }

    /// Get material document (inventory movement)
    pub async fn get_material_stock(
        &mut self,
        material_number: &str,
        plant: &str,
        storage_location: &str,
    ) -> Result<SapMaterialStock> {
        let token = self.get_access_token().await?;

        let url = format!(
            "{}/sap/opu/odata/sap/API_MATERIAL_STOCK_SRV/MaterialStock",
            self.base_url
        );

        let filter = format!(
            "Material eq '{}' and Plant eq '{}' and StorageLocation eq '{}'",
            material_number, plant, storage_location
        );

        let response = self.http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/json")
            .query(&[("$filter", filter.as_str())])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(SapError::ApiError(response.status(), response.text().await?));
        }

        let result: SapODataResponse<SapMaterialStock> = response.json().await?;
        result.d.results.get(0)
            .cloned()
            .ok_or(SapError::NotFound("Material not found".to_string()))
    }

    /// Post goods movement (update inventory)
    pub async fn post_goods_movement(
        &mut self,
        movement: &GoodsMovement,
    ) -> Result<String> {
        let token = self.get_access_token().await?;

        // First, get X-CSRF token
        let csrf_token = self.get_csrf_token(&token).await?;

        let url = format!(
            "{}/sap/opu/odata/sap/API_MATERIAL_DOCUMENT_SRV/A_MaterialDocumentHeader",
            self.base_url
        );

        let payload = json!({
            "GoodsMovementCode": "01",  // Goods receipt
            "PostingDate": movement.posting_date,
            "DocumentDate": movement.document_date,
            "to_MaterialDocumentItem": {
                "results": [{
                    "Material": movement.material_number,
                    "Plant": movement.plant,
                    "StorageLocation": movement.storage_location,
                    "GoodsMovementType": "501",  // Receipt without PO
                    "QuantityInEntryUnit": movement.quantity,
                    "EntryUnit": movement.unit,
                    "Batch": movement.batch_number,
                    // Custom fields for pharma
                    "YY1_ExpiryDate_MDI": movement.expiry_date,
                    "YY1_NDCCode_MDI": movement.ndc_code,
                }]
            }
        });

        let response = self.http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("X-CSRF-Token", csrf_token)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(SapError::ApiError(response.status(), response.text().await?));
        }

        let result: SapMaterialDocumentResponse = response.json().await?;
        Ok(result.d.MaterialDocument)
    }

    /// Get CSRF token (required for POST/PATCH/DELETE)
    async fn get_csrf_token(&self, access_token: &str) -> Result<String> {
        let response = self.http_client
            .get(&format!("{}/sap/opu/odata/sap/API_MATERIAL_DOCUMENT_SRV", self.base_url))
            .header("Authorization", format!("Bearer {}", access_token))
            .header("X-CSRF-Token", "Fetch")
            .send()
            .await?;

        response.headers()
            .get("X-CSRF-Token")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .ok_or(SapError::CsrfTokenError)
    }

    /// Get product master data
    pub async fn get_product(
        &mut self,
        material_number: &str,
    ) -> Result<SapProduct> {
        let token = self.get_access_token().await?;

        let url = format!(
            "{}/sap/opu/odata/sap/API_PRODUCT_SRV/A_Product('{}')",
            self.base_url, material_number
        );

        let response = self.http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(SapError::ApiError(response.status(), response.text().await?));
        }

        let result: SapODataSingleResponse<SapProduct> = response.json().await?;
        Ok(result.d)
    }
}

#[derive(Debug, Deserialize)]
struct SapTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct SapODataResponse<T> {
    d: SapODataResults<T>,
}

#[derive(Debug, Deserialize)]
struct SapODataResults<T> {
    results: Vec<T>,
}

#[derive(Debug, Clone, Deserialize)]
struct SapMaterialStock {
    #[serde(rename = "Material")]
    material: String,
    #[serde(rename = "Plant")]
    plant: String,
    #[serde(rename = "StorageLocation")]
    storage_location: String,
    #[serde(rename = "MatlWrhsStkQtyInMatlBaseUnit")]
    stock_quantity: String,  // SAP returns as string
    #[serde(rename = "MaterialBaseUnit")]
    base_unit: String,
}

#[derive(Debug, Serialize)]
struct GoodsMovement {
    material_number: String,
    plant: String,
    storage_location: String,
    quantity: f64,
    unit: String,
    batch_number: Option<String>,
    expiry_date: Option<String>,
    ndc_code: Option<String>,
    posting_date: String,  // YYYY-MM-DD
    document_date: String,
}
```

#### 3. **Sync Service** (`erp_sync_service.rs`)

```rust
use crate::services::erp::{NetSuiteClient, SapClient};
use crate::repositories::{InventoryRepository, ErpConnectionRepository};

pub struct ErpSyncService {
    db_pool: PgPool,
    inventory_repo: InventoryRepository,
    erp_connection_repo: ErpConnectionRepository,
}

impl ErpSyncService {
    /// Sync single inventory item to ERP
    pub async fn sync_inventory_to_erp(
        &self,
        inventory_id: Uuid,
    ) -> Result<()> {
        // 1. Get inventory item from Atlas
        let inventory = self.inventory_repo.get_by_id(inventory_id).await?;

        // 2. Get ERP connection for this user
        let connection = self.erp_connection_repo
            .get_active_connection_for_user(inventory.user_id)
            .await?;

        // 3. Get mapping (or create if auto-sync enabled)
        let mapping = self.get_or_create_mapping(&connection, &inventory).await?;

        // 4. Sync based on ERP type
        match connection.erp_type.as_str() {
            "netsuite" => self.sync_to_netsuite(&connection, &inventory, &mapping).await?,
            "sap_s4hana" => self.sync_to_sap(&connection, &inventory, &mapping).await?,
            _ => return Err(ErpError::UnsupportedErpType(connection.erp_type)),
        }

        // 5. Update last sync time
        self.update_mapping_sync_time(mapping.id).await?;

        Ok(())
    }

    /// Sync from ERP to Atlas (pull updates)
    pub async fn sync_from_erp_to_atlas(
        &self,
        connection_id: Uuid,
    ) -> Result<SyncResult> {
        let connection = self.erp_connection_repo.get_by_id(connection_id).await?;

        let sync_log = self.create_sync_log(&connection, "erp_to_atlas").await?;

        let result = match connection.erp_type.as_str() {
            "netsuite" => self.sync_from_netsuite(&connection).await,
            "sap_s4hana" => self.sync_from_sap(&connection).await,
            _ => Err(ErpError::UnsupportedErpType(connection.erp_type)),
        };

        self.complete_sync_log(sync_log.id, &result).await?;

        result
    }

    async fn sync_to_netsuite(
        &self,
        connection: &ErpConnection,
        inventory: &Inventory,
        mapping: &ErpInventoryMapping,
    ) -> Result<()> {
        // Decrypt NetSuite credentials
        let config = self.decrypt_netsuite_config(connection)?;
        let mut client = NetSuiteClient::new(config)?;

        // Update quantity in NetSuite
        client.update_inventory_quantity(
            &mapping.erp_item_id,
            &mapping.erp_location_id.as_ref().unwrap_or(&"1".to_string()),
            inventory.quantity,
        ).await?;

        // Update custom fields (lot, expiry)
        if connection.sync_lot_batch {
            client.update_custom_fields(
                &mapping.erp_item_id,
                &[
                    ("custitem_lot_number", &inventory.lot_number.clone().unwrap_or_default()),
                    ("custitem_expiry_date", &inventory.expiry_date.to_string()),
                ],
            ).await?;
        }

        Ok(())
    }

    async fn sync_to_sap(
        &self,
        connection: &ErpConnection,
        inventory: &Inventory,
        mapping: &ErpInventoryMapping,
    ) -> Result<()> {
        let config = self.decrypt_sap_config(connection)?;
        let mut client = SapClient::new(config)?;

        // Get current stock
        let current_stock = client.get_material_stock(
            &mapping.erp_item_id,
            &connection.sap_plant.as_ref().unwrap_or(&"1000".to_string()),
            &mapping.erp_location_id.as_ref().unwrap_or(&"0001".to_string()),
        ).await?;

        let current_qty = current_stock.stock_quantity.parse::<i32>().unwrap_or(0);
        let atlas_qty = inventory.quantity;

        // Post goods movement to adjust
        if current_qty != atlas_qty {
            let movement = GoodsMovement {
                material_number: mapping.erp_item_id.clone(),
                plant: connection.sap_plant.clone().unwrap_or("1000".to_string()),
                storage_location: mapping.erp_location_id.clone().unwrap_or("0001".to_string()),
                quantity: (atlas_qty - current_qty) as f64,
                unit: "PC".to_string(),
                batch_number: inventory.lot_number.clone(),
                expiry_date: Some(inventory.expiry_date.to_string()),
                ndc_code: inventory.pharmaceutical.as_ref()
                    .and_then(|p| p.ndc_code.clone()),
                posting_date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                document_date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            };

            client.post_goods_movement(&movement).await?;
        }

        Ok(())
    }

    async fn sync_from_netsuite(&self, connection: &ErpConnection) -> Result<SyncResult> {
        let config = self.decrypt_netsuite_config(connection)?;
        let client = NetSuiteClient::new(config)?;

        // Get all mapped items
        let mappings = self.erp_connection_repo
            .get_mappings_for_connection(connection.id)
            .await?;

        let mut synced = 0;
        let mut failed = 0;

        for mapping in mappings {
            match client.get_inventory_item(&mapping.erp_item_id).await {
                Ok(netsuite_item) => {
                    // Update Atlas inventory with NetSuite data
                    self.update_atlas_from_netsuite(&mapping, &netsuite_item).await?;
                    synced += 1;
                }
                Err(e) => {
                    tracing::error!("Failed to sync item {}: {}", mapping.erp_item_id, e);
                    failed += 1;
                }
            }
        }

        Ok(SyncResult { synced, failed, skipped: 0 })
    }

    /// Background job: Sync all active connections
    pub async fn run_scheduled_sync(&self) -> Result<()> {
        let connections = self.erp_connection_repo
            .get_all_active_connections()
            .await?;

        for connection in connections {
            // Check if it's time to sync (based on frequency)
            if self.should_sync_now(&connection) {
                match self.sync_bidirectional(connection.id).await {
                    Ok(_) => tracing::info!("Synced connection {}", connection.id),
                    Err(e) => tracing::error!("Sync failed for {}: {}", connection.id, e),
                }
            }
        }

        Ok(())
    }
}
```

---

## 3. FRONTEND IMPLEMENTATION

### A. ERP Connection Setup Wizard

**Page**: `/dashboard/settings/erp-integration`

**Steps**:
1. **Select ERP** → Oracle NetSuite or SAP S/4HANA
2. **Guided Credential Setup** → Instructions with screenshots
3. **Paste Credentials** → Form with validation
4. **Test Connection** → Live API test
5. **Configure Sync** → Select what to sync, frequency
6. **Map First Item** (optional) → Auto-discovery or manual mapping
7. **Activate** → Enable bidirectional sync

**UI Components**:
```typescript
// src/app/dashboard/settings/erp-integration/page.tsx

'use client';

import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';

export default function ErpIntegrationPage() {
  const [step, setStep] = useState(1);
  const [erpType, setErpType] = useState<'netsuite' | 'sap_s4hana' | null>(null);
  const [testing, setTesting] = useState(false);

  return (
    <div className="max-w-4xl mx-auto p-6">
      <h1 className="text-3xl font-bold mb-6">ERP Integration Setup</h1>

      {/* Step 1: Select ERP */}
      {step === 1 && (
        <div className="grid grid-cols-2 gap-6">
          <Card
            className="p-6 cursor-pointer hover:border-blue-500"
            onClick={() => { setErpType('netsuite'); setStep(2); }}
          >
            <h3 className="text-xl font-bold mb-2">Oracle NetSuite</h3>
            <p className="text-gray-600">
              Connect your NetSuite ERP for inventory sync
            </p>
          </Card>

          <Card
            className="p-6 cursor-pointer hover:border-blue-500"
            onClick={() => { setErpType('sap_s4hana'); setStep(2); }}
          >
            <h3 className="text-xl font-bold mb-2">SAP S/4HANA</h3>
            <p className="text-gray-600">
              Connect your SAP system for real-time sync
            </p>
          </Card>
        </div>
      )}

      {/* Step 2: Instructions */}
      {step === 2 && erpType === 'netsuite' && (
        <NetSuiteInstructionsStep onNext={() => setStep(3)} />
      )}

      {step === 2 && erpType === 'sap_s4hana' && (
        <SapInstructionsStep onNext={() => setStep(3)} />
      )}

      {/* Step 3: Enter Credentials */}
      {step === 3 && (
        <CredentialsForm
          erpType={erpType!}
          onTest={handleTestConnection}
          onNext={() => setStep(4)}
        />
      )}

      {/* Step 4: Configure Sync */}
      {step === 4 && (
        <SyncConfigurationForm onNext={() => setStep(5)} />
      )}

      {/* Step 5: Success */}
      {step === 5 && (
        <SuccessScreen erpType={erpType!} />
      )}
    </div>
  );
}

function NetSuiteInstructionsStep({ onNext }: { onNext: () => void }) {
  return (
    <Card className="p-6">
      <h2 className="text-2xl font-bold mb-4">NetSuite Setup Instructions</h2>

      <div className="space-y-4">
        <div className="bg-blue-50 dark:bg-blue-900/20 p-4 rounded-lg">
          <h3 className="font-semibold mb-2">Step 1: Enable Token-Based Authentication</h3>
          <ol className="list-decimal ml-5 space-y-2 text-sm">
            <li>Login to your NetSuite account</li>
            <li>Navigate to: <code className="bg-gray-200 px-2 py-1 rounded">Setup → Company → Enable Features</code></li>
            <li>Click the <strong>SuiteCloud</strong> tab</li>
            <li>Under "Manage Authentication", check <strong>Token-Based Authentication</strong></li>
            <li>Click <strong>Save</strong></li>
          </ol>
        </div>

        <div className="bg-green-50 dark:bg-green-900/20 p-4 rounded-lg">
          <h3 className="font-semibold mb-2">Step 2: Create Integration Record</h3>
          <ol className="list-decimal ml-5 space-y-2 text-sm">
            <li>Navigate to: <code className="bg-gray-200 px-2 py-1 rounded">Setup → Integration → Manage Integrations</code></li>
            <li>Click <strong>New</strong></li>
            <li>Enter Name: "Atlas Pharma Integration"</li>
            <li>Check <strong>Token-Based Authentication</strong></li>
            <li>Click <strong>Save</strong></li>
            <li>⚠️ <strong>Copy the Consumer Key and Consumer Secret</strong> (shown only once!)</li>
          </ol>
        </div>

        <div className="bg-purple-50 dark:bg-purple-900/20 p-4 rounded-lg">
          <h3 className="font-semibold mb-2">Step 3: Create Access Token</h3>
          <ol className="list-decimal ml-5 space-y-2 text-sm">
            <li>Navigate to: <code className="bg-gray-200 px-2 py-1 rounded">Setup → Users/Roles → Access Tokens</code></li>
            <li>Click <strong>New</strong></li>
            <li>Select Application: "Atlas Pharma Integration"</li>
            <li>Select User (service account recommended)</li>
            <li>Select Role (with inventory permissions)</li>
            <li>Click <strong>Save</strong></li>
            <li>⚠️ <strong>Copy the Token ID and Token Secret</strong> (shown only once!)</li>
          </ol>
        </div>
      </div>

      <div className="mt-6 flex justify-between">
        <Button variant="outline" onClick={() => window.history.back()}>
          Back
        </Button>
        <Button onClick={onNext}>
          I've Completed Setup →
        </Button>
      </div>
    </Card>
  );
}

function CredentialsForm({ erpType, onTest, onNext }: any) {
  const [credentials, setCredentials] = useState({
    // NetSuite fields
    accountId: '',
    consumerKey: '',
    consumerSecret: '',
    tokenId: '',
    tokenSecret: '',

    // SAP fields
    baseUrl: '',
    clientId: '',
    clientSecret: '',
  });

  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<'success' | 'error' | null>(null);

  const handleTestConnection = async () => {
    setTesting(true);
    setTestResult(null);

    try {
      const response = await fetch('/api/erp/test-connection', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ erpType, credentials }),
      });

      if (response.ok) {
        setTestResult('success');
      } else {
        setTestResult('error');
      }
    } catch (error) {
      setTestResult('error');
    } finally {
      setTesting(false);
    }
  };

  return (
    <Card className="p-6">
      <h2 className="text-2xl font-bold mb-4">Enter Your Credentials</h2>

      {erpType === 'netsuite' && (
        <div className="space-y-4">
          <div>
            <Label>NetSuite Account ID</Label>
            <Input
              placeholder="1234567"
              value={credentials.accountId}
              onChange={(e) => setCredentials({ ...credentials, accountId: e.target.value })}
            />
            <p className="text-sm text-gray-500 mt-1">
              Found in your NetSuite URL: <code>1234567.app.netsuite.com</code>
            </p>
          </div>

          <div>
            <Label>Consumer Key (Client ID)</Label>
            <Input
              placeholder="abc123..."
              type="password"
              value={credentials.consumerKey}
              onChange={(e) => setCredentials({ ...credentials, consumerKey: e.target.value })}
            />
          </div>

          <div>
            <Label>Consumer Secret (Client Secret)</Label>
            <Input
              placeholder="def456..."
              type="password"
              value={credentials.consumerSecret}
              onChange={(e) => setCredentials({ ...credentials, consumerSecret: e.target.value })}
            />
          </div>

          <div>
            <Label>Token ID</Label>
            <Input
              placeholder="ghi789..."
              type="password"
              value={credentials.tokenId}
              onChange={(e) => setCredentials({ ...credentials, tokenId: e.target.value })}
            />
          </div>

          <div>
            <Label>Token Secret</Label>
            <Input
              placeholder="jkl012..."
              type="password"
              value={credentials.tokenSecret}
              onChange={(e) => setCredentials({ ...credentials, tokenSecret: e.target.value })}
            />
          </div>
        </div>
      )}

      {erpType === 'sap_s4hana' && (
        <div className="space-y-4">
          <div>
            <Label>SAP Base URL</Label>
            <Input
              placeholder="https://my12345.s4hana.cloud.sap"
              value={credentials.baseUrl}
              onChange={(e) => setCredentials({ ...credentials, baseUrl: e.target.value })}
            />
          </div>

          <div>
            <Label>OAuth Client ID</Label>
            <Input
              placeholder="ABC123..."
              type="password"
              value={credentials.clientId}
              onChange={(e) => setCredentials({ ...credentials, clientId: e.target.value })}
            />
          </div>

          <div>
            <Label>OAuth Client Secret</Label>
            <Input
              placeholder="DEF456..."
              type="password"
              value={credentials.clientSecret}
              onChange={(e) => setCredentials({ ...credentials, clientSecret: e.target.value })}
            />
          </div>
        </div>
      )}

      <div className="mt-6 space-y-4">
        <Button
          onClick={handleTestConnection}
          disabled={testing}
          variant="outline"
          className="w-full"
        >
          {testing ? 'Testing Connection...' : 'Test Connection'}
        </Button>

        {testResult === 'success' && (
          <div className="bg-green-50 border border-green-200 p-4 rounded-lg">
            <p className="text-green-800 font-semibold">✓ Connection successful!</p>
            <p className="text-sm text-green-600">Your credentials are valid and Atlas can communicate with your ERP.</p>
          </div>
        )}

        {testResult === 'error' && (
          <div className="bg-red-50 border border-red-200 p-4 rounded-lg">
            <p className="text-red-800 font-semibold">✗ Connection failed</p>
            <p className="text-sm text-red-600">Please check your credentials and try again.</p>
          </div>
        )}

        <Button
          onClick={onNext}
          disabled={testResult !== 'success'}
          className="w-full"
        >
          Continue to Sync Configuration →
        </Button>
      </div>
    </Card>
  );
}
```

---

## 4. SYNC STRATEGIES

### A. Real-Time Sync (Webhooks - Preferred)

**NetSuite Webhooks**:
- NetSuite supports SuiteScript-based webhooks
- Customer creates a SuiteScript that triggers on inventory changes
- Script sends HTTP POST to Atlas webhook endpoint: `https://atlas.pharma/api/erp/webhooks/netsuite/{connection_id}`

**SAP Webhooks** (SAP Event Mesh):
- SAP S/4HANA Cloud supports event-driven architecture
- Events: MaterialChanged, StockLevelUpdated
- Atlas subscribes to events via SAP Event Mesh

**Atlas Webhook Endpoint**:
```rust
// src/handlers/erp_webhooks.rs

#[axum::debug_handler]
pub async fn netsuite_webhook(
    Path(connection_id): Path<Uuid>,
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<NetSuiteWebhookPayload>,
) -> Result<StatusCode> {
    // 1. Validate webhook signature
    verify_netsuite_signature(&payload)?;

    // 2. Process event
    match payload.event_type.as_str() {
        "inventory.updated" => {
            let service = ErpSyncService::new(pool);
            service.handle_netsuite_inventory_update(connection_id, &payload.data).await?;
        }
        "item.created" => {
            // Auto-create mapping if enabled
        }
        _ => {}
    }

    Ok(StatusCode::OK)
}
```

### B. Polling Sync (Fallback - 15-minute intervals)

For customers who can't set up webhooks:
```rust
// Background job runs every 15 minutes
pub async fn scheduled_sync_job(pool: PgPool) {
    let service = ErpSyncService::new(pool);

    loop {
        service.run_scheduled_sync().await.ok();
        tokio::time::sleep(Duration::from_secs(15 * 60)).await;
    }
}
```

### C. Manual Sync (On-Demand)

User clicks "Sync Now" button in dashboard:
```typescript
const handleManualSync = async () => {
  await fetch('/api/erp/connections/{id}/sync', {
    method: 'POST'
  });
};
```

---

## 5. FIELD MAPPING STRATEGY

### Automatic Mapping (Smart Discovery)

When user connects ERP, Atlas automatically:
1. Fetches all inventory items from ERP
2. Matches by NDC code (pharmaceutical-specific)
3. Creates mappings in `erp_inventory_mappings` table
4. Flags unmapped items for manual review

**Rust Logic**:
```rust
pub async fn auto_discover_mappings(&self, connection_id: Uuid) -> Result<Vec<ErpInventoryMapping>> {
    let connection = self.erp_connection_repo.get_by_id(connection_id).await?;
    let atlas_inventory = self.inventory_repo.get_by_user(connection.user_id).await?;

    let erp_items = match connection.erp_type.as_str() {
        "netsuite" => self.fetch_netsuite_items(&connection).await?,
        "sap_s4hana" => self.fetch_sap_items(&connection).await?,
        _ => vec![],
    };

    let mut mappings = vec![];

    for atlas_item in atlas_inventory {
        if let Some(ndc) = &atlas_item.pharmaceutical.as_ref().and_then(|p| p.ndc_code.clone()) {
            // Find matching ERP item by NDC code
            if let Some(erp_item) = erp_items.iter().find(|e| e.ndc_code.as_deref() == Some(ndc)) {
                let mapping = ErpInventoryMapping {
                    erp_connection_id: connection.id,
                    atlas_inventory_id: atlas_item.id,
                    erp_item_id: erp_item.id.clone(),
                    erp_item_name: Some(erp_item.name.clone()),
                    erp_location_id: erp_item.location_id.clone(),
                    sync_direction: "bidirectional".to_string(),
                    ..Default::default()
                };

                mappings.push(self.save_mapping(&mapping).await?);
            }
        }
    }

    Ok(mappings)
}
```

---

## 6. CONFLICT RESOLUTION

When ERP and Atlas have different values:

**Strategies**:
1. **Atlas Wins** (default for new integrations) - Atlas is source of truth
2. **ERP Wins** - ERP is source of truth (for existing ERP users)
3. **Manual Review** - Flag conflicts for user review

**Implementation**:
```rust
pub async fn resolve_conflict(
    &self,
    mapping: &ErpInventoryMapping,
    atlas_qty: i32,
    erp_qty: i32,
) -> Result<i32> {
    match mapping.conflict_resolution.as_str() {
        "atlas_wins" => Ok(atlas_qty),
        "erp_wins" => Ok(erp_qty),
        "manual" => {
            // Create notification for user
            self.create_conflict_notification(mapping, atlas_qty, erp_qty).await?;
            Ok(atlas_qty)  // Keep current until user resolves
        }
        _ => Ok(atlas_qty),
    }
}
```

---

## 7. ERROR HANDLING & MONITORING

### Comprehensive Error Logging

Every sync operation is logged:
```rust
#[derive(Debug, Serialize)]
struct SyncError {
    connection_id: Uuid,
    item_id: String,
    error_type: String,  // "auth_failed", "item_not_found", "network_error", etc.
    error_message: String,
    retry_count: i32,
    last_retry_at: DateTime<Utc>,
}
```

### User Notifications

- Email alerts for sync failures (3 consecutive failures)
- In-app notifications for conflicts
- Dashboard widget showing sync health

### Retry Logic

```rust
pub async fn sync_with_retry<F, T>(
    &self,
    operation: F,
    max_retries: u32,
) -> Result<T>
where
    F: Fn() -> BoxFuture<'static, Result<T>>,
{
    let mut attempts = 0;
    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if attempts < max_retries => {
                attempts += 1;
                let delay = std::time::Duration::from_secs(2u64.pow(attempts));  // Exponential backoff
                tokio::time::sleep(delay).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

---

## 8. SECURITY CONSIDERATIONS

### Credential Encryption

All ERP credentials encrypted with AES-256-GCM before storage:
```rust
use crate::services::encryption_service::EncryptionService;

pub async fn store_netsuite_credentials(
    &self,
    connection_id: Uuid,
    credentials: NetSuiteCredentials,
) -> Result<()> {
    let encryption = EncryptionService::new();

    let encrypted = EncryptedNetSuiteCredentials {
        consumer_key: encryption.encrypt(&credentials.consumer_key)?,
        consumer_secret: encryption.encrypt(&credentials.consumer_secret)?,
        token_id: encryption.encrypt(&credentials.token_id)?,
        token_secret: encryption.encrypt(&credentials.token_secret)?,
    };

    self.save_encrypted_credentials(connection_id, encrypted).await
}
```

### Webhook Signature Verification

Verify all incoming webhooks:
```rust
fn verify_netsuite_signature(payload: &NetSuiteWebhookPayload) -> Result<()> {
    let expected_signature = hmac_sha256(&payload.body, &webhook_secret);

    if payload.signature != expected_signature {
        return Err(ErpError::InvalidWebhookSignature);
    }

    Ok(())
}
```

### Audit Trail

Every sync operation logged with:
- User ID
- Connection ID
- Items synced
- Timestamp
- Source (manual, scheduled, webhook)
- Result (success/failure)

---

## 9. API ENDPOINTS

```
POST   /api/erp/connections                    # Create ERP connection
GET    /api/erp/connections                    # List user's connections
GET    /api/erp/connections/:id                # Get connection details
PUT    /api/erp/connections/:id                # Update connection
DELETE /api/erp/connections/:id                # Delete connection
POST   /api/erp/connections/:id/test           # Test connection
POST   /api/erp/connections/:id/sync           # Manual sync

GET    /api/erp/connections/:id/mappings       # List mappings
POST   /api/erp/connections/:id/mappings       # Create mapping
PUT    /api/erp/mappings/:id                   # Update mapping
DELETE /api/erp/mappings/:id                   # Delete mapping
POST   /api/erp/connections/:id/auto-discover  # Auto-discover mappings

GET    /api/erp/connections/:id/sync-logs      # Get sync history
GET    /api/erp/sync-logs/:id                  # Get log details

POST   /api/erp/webhooks/netsuite/:conn_id     # NetSuite webhook endpoint
POST   /api/erp/webhooks/sap/:conn_id          # SAP webhook endpoint
```

---

## 10. ONBOARDING FLOW SUMMARY

### 5-Minute Setup (User Perspective)

1. **Minute 1**: Navigate to Settings → ERP Integration
2. **Minute 2**: Click "Oracle NetSuite" → Read instructions
3. **Minute 3**: Open NetSuite in new tab → Create integration record → Copy keys
4. **Minute 4**: Paste keys into Atlas → Click "Test Connection" → Success
5. **Minute 5**: Enable sync options → Click "Auto-Discover Items" → **Done!**

**Total Time**: Under 5 minutes for technical users

---

## 11. COMPETITIVE ADVANTAGE

### Why Atlas Wins

| Feature | Atlas Pharma | Traditional Integrations |
|---------|-------------|--------------------------|
| **Setup Time** | 5 minutes | 2-4 weeks |
| **Technical Expertise** | None (self-service) | Requires IT team |
| **Cost** | Included | $5,000-$50,000 setup fee |
| **Customization** | Auto-configured | Manual field mapping |
| **Pharmaceutical-Specific** | NDC validation, lot tracking | Generic inventory |
| **Security** | End-to-end encryption | Varies |
| **Bidirectional Sync** | Real-time | Often one-way |

---

## 12. IMPLEMENTATION TIMELINE

### Phase 1: Core Integration (4 weeks)
- Week 1: Database schema + encryption
- Week 2: NetSuite client (OAuth 1.0 + API calls)
- Week 3: SAP client (OAuth 2.0 + OData)
- Week 4: Sync service + field mapping

### Phase 2: Frontend & Testing (3 weeks)
- Week 5: Setup wizard UI
- Week 6: Dashboard widgets + sync logs
- Week 7: End-to-end testing with test NetSuite/SAP accounts

### Phase 3: Webhooks & Optimization (2 weeks)
- Week 8: Webhook handlers + retry logic
- Week 9: Performance optimization + monitoring

### Phase 4: Production Launch (1 week)
- Week 10: Documentation, final QA, go-live

**Total**: 10 weeks (2.5 months)

---

## 13. DEPENDENCIES

### Rust Crates Needed
```toml
[dependencies]
# OAuth 1.0 for NetSuite
oauth-client = "0.5"

# Additional HTTP client features
reqwest = { version = "0.11", features = ["json", "cookies"] }

# HMAC for webhook verification
hmac = "0.12"
sha2 = "0.10"

# Base64 encoding
base64 = "0.21"

# Existing crates
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio", "uuid", "chrono"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
```

---

## CONCLUSION

This ERP integration strategy positions Atlas Pharma as the **easiest pharmaceutical B2B marketplace to onboard**. By leveraging industry-standard OAuth protocols, pharmaceutical-specific field mapping (NDC codes), and intelligent auto-discovery, we reduce integration time from weeks to minutes.

**Key Takeaways**:
1. **Customer generates credentials in their ERP** → Atlas never touches admin credentials
2. **5-minute self-service setup** → No IT team required
3. **Bidirectional real-time sync** → Atlas and ERP stay in perfect sync
4. **Pharmaceutical-specific mapping** → NDC codes, lot numbers, expiry dates
5. **Production-ready security** → AES-256 encryption, audit trails, webhook verification

This is a **massive competitive advantage** and a core pillar of your pitch deck. No competitor offers this level of integration ease for pharmaceutical distributors.

---

**Next Steps**:
1. Review and approve this plan
2. Begin Phase 1 implementation (database schema)
3. Create test NetSuite/SAP developer accounts
4. Build MVP for demo purposes

Let me know if you want me to dive deeper into any section or start building!
