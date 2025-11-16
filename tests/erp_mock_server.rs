// Mock ERP Server for Testing
// Simulates NetSuite and SAP S/4HANA API responses
// Run with: cargo test --test erp_mock_server

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// ============================================================================
// Mock Data Structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockInventoryItem {
    pub id: String,
    pub name: String,
    pub quantity: f64,
    pub ndc_code: Option<String>,
    pub lot_number: Option<String>,
}

#[derive(Debug, Default)]
pub struct MockErpState {
    pub netsuite_items: Vec<MockInventoryItem>,
    pub sap_materials: Vec<MockInventoryItem>,
    pub netsuite_token_valid: bool,
    pub sap_token_valid: bool,
}

type SharedState = Arc<RwLock<MockErpState>>;

// ============================================================================
// NetSuite Mock Endpoints
// ============================================================================

async fn netsuite_get_inventory(
    State(state): State<SharedState>,
    Path(item_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let state = state.read().await;

    if !state.netsuite_token_valid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let item = state.netsuite_items
        .iter()
        .find(|i| i.id == item_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(json!({
        "id": item.id,
        "itemId": item.id,
        "displayName": item.name,
        "quantityOnHand": item.quantity,
        "customFields": {
            "custitem_ndc_code": item.ndc_code,
            "custitem_lot_number": item.lot_number,
        },
        "locations": {
            "items": [{
                "locationId": "1",
                "quantityOnHand": item.quantity,
            }]
        }
    })))
}

async fn netsuite_update_inventory(
    State(state): State<SharedState>,
    Path(item_id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut state = state.write().await;

    if !state.netsuite_token_valid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let item = state.netsuite_items
        .iter_mut()
        .find(|i| i.id == item_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(qty) = payload.get("quantityOnHand").and_then(|v| v.as_f64()) {
        item.quantity = qty;
    }

    Ok(Json(json!({
        "id": item.id,
        "success": true
    })))
}

async fn netsuite_search_inventory(
    State(state): State<SharedState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let state = state.read().await;

    if !state.netsuite_token_valid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let items: Vec<serde_json::Value> = state.netsuite_items
        .iter()
        .map(|item| json!({
            "id": item.id,
            "itemId": item.id,
            "displayName": item.name,
            "quantityOnHand": item.quantity,
        }))
        .collect();

    Ok(Json(json!({
        "items": items,
        "totalResults": items.len(),
        "offset": 0,
        "count": items.len()
    })))
}

// ============================================================================
// SAP Mock Endpoints
// ============================================================================

async fn sap_token(
    State(state): State<SharedState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let state = state.read().await;

    if !state.sap_token_valid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(Json(json!({
        "access_token": "mock_sap_token_12345",
        "token_type": "Bearer",
        "expires_in": 3600
    })))
}

async fn sap_get_material(
    State(state): State<SharedState>,
    Path(material_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let state = state.read().await;

    let material = state.sap_materials
        .iter()
        .find(|m| m.id == material_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(json!({
        "d": {
            "Material": material.id,
            "MaterialDescription": material.name,
            "Plant": "1000",
            "StorageLocation": "0001",
            "MaterialBaseUnit": "PC",
            "to_MaterialStock": {
                "results": [{
                    "Material": material.id,
                    "Plant": "1000",
                    "StorageLocation": "0001",
                    "StockQuantity": material.quantity.to_string(),
                    "MaterialBaseUnit": "PC"
                }]
            }
        }
    })))
}

async fn sap_get_stock(
    State(state): State<SharedState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let state = state.read().await;

    let results: Vec<serde_json::Value> = state.sap_materials
        .iter()
        .map(|material| json!({
            "Material": material.id,
            "Plant": "1000",
            "StorageLocation": "0001",
            "StockQuantity": material.quantity.to_string(),
            "MaterialBaseUnit": "PC"
        }))
        .collect();

    Ok(Json(json!({
        "d": {
            "results": results
        }
    })))
}

async fn sap_adjust_inventory(
    State(state): State<SharedState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut state = state.write().await;

    let material_id = payload.get("Material")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;

    let quantity_delta = payload.get("QuantityInEntryUnit")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .ok_or(StatusCode::BAD_REQUEST)?;

    let material = state.sap_materials
        .iter_mut()
        .find(|m| m.id == material_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    material.quantity += quantity_delta;

    Ok(Json(json!({
        "d": {
            "Material": material.id,
            "Plant": "1000",
            "PostingDate": "2024-01-15",
            "DocumentDate": "2024-01-15",
            "MaterialDocumentYear": "2024",
            "MaterialDocument": format!("MD{}", Uuid::new_v4())
        }
    })))
}

// ============================================================================
// Server Setup
// ============================================================================

pub fn create_netsuite_mock_server(state: SharedState) -> Router {
    Router::new()
        .route("/services/rest/record/v1/inventoryItem/:id", get(netsuite_get_inventory))
        .route("/services/rest/record/v1/inventoryItem/:id", post(netsuite_update_inventory))
        .route("/services/rest/query/v1/search", post(netsuite_search_inventory))
        .with_state(state)
}

pub fn create_sap_mock_server(state: SharedState) -> Router {
    Router::new()
        .route("/oauth/token", post(sap_token))
        .route("/sap/opu/odata/sap/API_MATERIAL_STOCK_SRV/MaterialStock", get(sap_get_stock))
        .route("/sap/opu/odata/sap/API_PRODUCT_SRV/A_Product(:id)", get(sap_get_material))
        .route("/sap/opu/odata/sap/API_MATERIAL_DOCUMENT_SRV/A_MaterialDocumentHeader", post(sap_adjust_inventory))
        .with_state(state)
}

// ============================================================================
// Test Helper Functions
// ============================================================================

pub async fn start_mock_servers() -> (String, String, SharedState) {
    let state = Arc::new(RwLock::new(MockErpState {
        netsuite_items: vec![
            MockInventoryItem {
                id: "ITEM001".to_string(),
                name: "Test Pharmaceutical A".to_string(),
                quantity: 100.0,
                ndc_code: Some("12345-678-90".to_string()),
                lot_number: Some("LOT123".to_string()),
            },
            MockInventoryItem {
                id: "ITEM002".to_string(),
                name: "Test Pharmaceutical B".to_string(),
                quantity: 50.0,
                ndc_code: Some("98765-432-10".to_string()),
                lot_number: Some("LOT456".to_string()),
            },
        ],
        sap_materials: vec![
            MockInventoryItem {
                id: "MAT001".to_string(),
                name: "SAP Material A".to_string(),
                quantity: 200.0,
                ndc_code: Some("11111-222-33".to_string()),
                lot_number: Some("SAPLOT001".to_string()),
            },
            MockInventoryItem {
                id: "MAT002".to_string(),
                name: "SAP Material B".to_string(),
                quantity: 150.0,
                ndc_code: Some("44444-555-66".to_string()),
                lot_number: Some("SAPLOT002".to_string()),
            },
        ],
        netsuite_token_valid: true,
        sap_token_valid: true,
    }));

    let netsuite_app = create_netsuite_mock_server(state.clone());
    let sap_app = create_sap_mock_server(state.clone());

    // Start NetSuite mock server on random port
    let netsuite_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let netsuite_addr = netsuite_listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(netsuite_listener, netsuite_app).await.unwrap();
    });

    // Start SAP mock server on random port
    let sap_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let sap_addr = sap_listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(sap_listener, sap_app).await.unwrap();
    });

    // Give servers time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let netsuite_url = format!("http://{}", netsuite_addr);
    let sap_url = format!("http://{}", sap_addr);

    (netsuite_url, sap_url, state)
}

// ============================================================================
// Integration Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_netsuite_mock_server() {
        let (netsuite_url, _, _state) = start_mock_servers().await;

        let client = reqwest::Client::new();

        // Test getting inventory
        let response = client
            .get(format!("{}/services/rest/record/v1/inventoryItem/ITEM001", netsuite_url))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let data: serde_json::Value = response.json().await.unwrap();
        assert_eq!(data["id"], "ITEM001");
        assert_eq!(data["quantityOnHand"], 100.0);
    }

    #[tokio::test]
    async fn test_sap_mock_server() {
        let (_netsuite_url, sap_url, _state) = start_mock_servers().await;

        let client = reqwest::Client::new();

        // Test getting token
        let response = client
            .post(format!("{}/oauth/token", sap_url))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let data: serde_json::Value = response.json().await.unwrap();
        assert!(data["access_token"].is_string());
    }

    #[tokio::test]
    async fn test_netsuite_update_inventory() {
        let (netsuite_url, _, state) = start_mock_servers().await;

        let client = reqwest::Client::new();

        // Update quantity
        let response = client
            .post(format!("{}/services/rest/record/v1/inventoryItem/ITEM001", netsuite_url))
            .json(&json!({
                "quantityOnHand": 150.0
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);

        // Verify update
        let state = state.read().await;
        let item = state.netsuite_items.iter().find(|i| i.id == "ITEM001").unwrap();
        assert_eq!(item.quantity, 150.0);
    }

    #[tokio::test]
    async fn test_sap_adjust_inventory() {
        let (_netsuite_url, sap_url, state) = start_mock_servers().await;

        let client = reqwest::Client::new();

        // Adjust quantity
        let response = client
            .post(format!("{}/sap/opu/odata/sap/API_MATERIAL_DOCUMENT_SRV/A_MaterialDocumentHeader", sap_url))
            .json(&json!({
                "Material": "MAT001",
                "QuantityInEntryUnit": "25.0"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);

        // Verify adjustment
        let state = state.read().await;
        let material = state.sap_materials.iter().find(|m| m.id == "MAT001").unwrap();
        assert_eq!(material.quantity, 225.0); // 200 + 25
    }
}
