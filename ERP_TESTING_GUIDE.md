# ERP Integration Testing Guide

Complete guide for testing Oracle NetSuite and SAP S/4HANA integration without live systems.

## 🎯 Testing Strategy Overview

Since you don't have access to real SAP or NetSuite systems, we provide multiple testing approaches:

1. **Mock ERP Servers** - Local servers that simulate ERP APIs
2. **Unit Tests** - Test individual components in isolation
3. **Integration Tests** - Test end-to-end workflows with mocked responses
4. **Manual Testing** - Use Postman/curl to test API endpoints
5. **Docker Test Environment** - Optional containerized test setup

---

## 📦 Option 1: Mock ERP Servers (Recommended)

### Quick Start

Run the mock servers in the background:

```bash
# Run mock server tests
cargo test --test erp_mock_server

# Or run continuously for manual testing
cargo test --test erp_mock_server -- --nocapture
```

### What This Provides

The mock servers simulate:
- ✅ **NetSuite REST API** - Inventory CRUD operations
- ✅ **SAP OData API** - Material stock management
- ✅ **OAuth tokens** - Authentication flows
- ✅ **Error scenarios** - 401, 404, rate limits
- ✅ **Data persistence** - In-memory state during tests

### Mock Server Endpoints

**NetSuite Mock (`http://localhost:RANDOM_PORT`):**
```
GET    /services/rest/record/v1/inventoryItem/:id
POST   /services/rest/record/v1/inventoryItem/:id
POST   /services/rest/query/v1/search
```

**SAP Mock (`http://localhost:RANDOM_PORT`):**
```
POST   /oauth/token
GET    /sap/opu/odata/sap/API_MATERIAL_STOCK_SRV/MaterialStock
GET    /sap/opu/odata/sap/API_PRODUCT_SRV/A_Product(:id)
POST   /sap/opu/odata/sap/API_MATERIAL_DOCUMENT_SRV/A_MaterialDocumentHeader
```

---

## 🧪 Option 2: Unit Tests with Mocked HTTP Responses

Create unit tests using `wiremock` or `mockito`:

```bash
# Add to Cargo.toml [dev-dependencies]
wiremock = "0.6"
tokio-test = "0.4"
```

Example test:

```rust
#[tokio::test]
async fn test_netsuite_client_get_item() {
    // Start mock server
    let mock_server = wiremock::MockServer::start().await;

    // Setup mock response
    wiremock::Mock::given(method("GET"))
        .and(path("/services/rest/record/v1/inventoryItem/TEST123"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "id": "TEST123",
                "quantityOnHand": 100.0
            })))
        .mount(&mock_server)
        .await;

    // Test the client
    let config = NetSuiteConfig {
        account_id: "test".to_string(),
        // ... other fields
    };

    let client = NetSuiteClient::new_with_base_url(
        config,
        mock_server.uri()
    ).unwrap();

    let item = client.get_inventory_item("TEST123").await.unwrap();
    assert_eq!(item.id, "TEST123");
    assert_eq!(item.quantity_on_hand.unwrap(), 100.0);
}
```

---

## 🔄 Option 3: Integration Tests (End-to-End)

Test the full Atlas → ERP flow:

```bash
# Create integration test file
touch tests/erp_integration_test.rs
```

```rust
use sqlx::PgPool;
use atlas_pharma::services::erp::*;

#[sqlx::test]
async fn test_atlas_to_netsuite_sync(pool: PgPool) {
    // 1. Setup: Create user, inventory, and ERP connection
    let user_id = create_test_user(&pool).await;
    let inventory_id = create_test_inventory(&pool, user_id).await;

    // 2. Point to mock server
    let (netsuite_url, _, _) = erp_mock_server::start_mock_servers().await;

    let connection = create_erp_connection(
        &pool,
        user_id,
        "netsuite",
        netsuite_url
    ).await;

    // 3. Trigger sync
    let sync_service = ErpSyncService::new(pool.clone());
    let result = sync_service.sync_atlas_to_erp(connection.id).await.unwrap();

    // 4. Verify
    assert_eq!(result.items_synced, 1);
    assert_eq!(result.items_failed, 0);
}
```

---

## 🌐 Option 4: Manual API Testing with Postman/curl

### Step 1: Start Atlas Server

```bash
# Make sure database is running
docker-compose up -d postgres

# Run migrations
psql postgres://postgres:postgres@localhost:5432/atlas_pharma -f migrations/010_erp_integration_system.sql

# Start server
cargo run
```

### Step 2: Get Auth Token

```bash
# Register user
curl -X POST http://localhost:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "Test123!@#",
    "full_name": "Test User"
  }'

# Login
TOKEN=$(curl -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "Test123!@#"
  }' | jq -r '.token')

echo "Token: $TOKEN"
```

### Step 3: Create Mock ERP Connection

Since we can't connect to real NetSuite, we'll create a connection pointing to our mock server:

```bash
# First, run mock server in another terminal
cargo test --test erp_mock_server -- --nocapture

# Note the port from the test output, then create connection:
curl -X POST http://localhost:8080/api/erp/connections \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "connection_name": "Mock NetSuite",
    "erp_type": "netsuite",
    "netsuite_account_id": "MOCK123",
    "netsuite_consumer_key": "mock_key",
    "netsuite_consumer_secret": "mock_secret",
    "netsuite_token_id": "mock_token",
    "netsuite_token_secret": "mock_token_secret",
    "netsuite_realm": "MOCK",
    "sync_enabled": true,
    "sync_frequency_minutes": 60,
    "sync_stock_levels": true,
    "sync_product_master": true
  }'
```

### Step 4: Test Endpoints

```bash
# List connections
curl http://localhost:8080/api/erp/connections \
  -H "Authorization: Bearer $TOKEN"

# Get specific connection
CONNECTION_ID="<id-from-response>"
curl http://localhost:8080/api/erp/connections/$CONNECTION_ID \
  -H "Authorization: Bearer $TOKEN"

# Test connection (will fail without real credentials, but tests validation)
curl -X POST http://localhost:8080/api/erp/connections/$CONNECTION_ID/test \
  -H "Authorization: Bearer $TOKEN"

# Trigger manual sync
curl -X POST "http://localhost:8080/api/erp/connections/$CONNECTION_ID/sync?direction=atlas_to_erp" \
  -H "Authorization: Bearer $TOKEN"

# View sync logs
curl http://localhost:8080/api/erp/connections/$CONNECTION_ID/sync-logs \
  -H "Authorization: Bearer $TOKEN"

# View mappings
curl http://localhost:8080/api/erp/connections/$CONNECTION_ID/mappings \
  -H "Authorization: Bearer $TOKEN"
```

---

## 🐳 Option 5: Docker Test Environment (Advanced)

Create a complete test environment with mock services:

```yaml
# docker-compose.test.yml
version: '3.8'

services:
  # Your Atlas application
  atlas:
    build: .
    environment:
      DATABASE_URL: postgres://postgres:postgres@postgres:5432/atlas_pharma
      NETSUITE_BASE_URL: http://netsuite-mock:3001
      SAP_BASE_URL: http://sap-mock:3002
    depends_on:
      - postgres
      - netsuite-mock
      - sap-mock

  # Mock NetSuite server
  netsuite-mock:
    build:
      context: .
      dockerfile: Dockerfile.netsuite-mock
    ports:
      - "3001:3001"

  # Mock SAP server
  sap-mock:
    build:
      context: .
      dockerfile: Dockerfile.sap-mock
    ports:
      - "3002:3002"

  postgres:
    image: pgvector/pgvector:pg16
    environment:
      POSTGRES_DB: atlas_pharma
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: postgres
    ports:
      - "5432:5432"
```

Run:
```bash
docker-compose -f docker-compose.test.yml up
```

---

## 🎬 Option 6: Scenario-Based Testing

Create specific test scenarios:

### Scenario 1: NetSuite Connection & Sync

```bash
# File: tests/scenarios/netsuite_sync.sh

#!/bin/bash
set -e

echo "🧪 Testing NetSuite Integration Flow"

# 1. Create user and get token
TOKEN=$(./scripts/create_test_user.sh)

# 2. Create inventory item
INVENTORY_ID=$(curl -X POST http://localhost:8080/api/inventory \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "pharmaceutical_id": "...",
    "batch_number": "BATCH001",
    "quantity": 100,
    "expiry_date": "2025-12-31"
  }' | jq -r '.id')

echo "✅ Created inventory: $INVENTORY_ID"

# 3. Create NetSuite connection (pointing to mock)
CONNECTION_ID=$(curl -X POST http://localhost:8080/api/erp/connections \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d @test_data/netsuite_connection.json | jq -r '.id')

echo "✅ Created connection: $CONNECTION_ID"

# 4. Trigger sync
curl -X POST "http://localhost:8080/api/erp/connections/$CONNECTION_ID/sync?direction=atlas_to_erp" \
  -H "Authorization: Bearer $TOKEN"

echo "✅ Sync triggered"

# 5. Check sync logs
sleep 2
curl http://localhost:8080/api/erp/connections/$CONNECTION_ID/sync-logs \
  -H "Authorization: Bearer $TOKEN" | jq '.[0]'

echo "✅ Test complete!"
```

### Scenario 2: Bidirectional Sync with Conflict Resolution

```rust
#[tokio::test]
async fn test_conflict_resolution_erp_wins() {
    let (pool, mock_servers) = setup_test_environment().await;

    // 1. Create inventory with quantity 100
    let inventory = create_test_inventory(&pool, 100).await;

    // 2. Update ERP to quantity 150
    update_mock_erp_quantity(&mock_servers, "ITEM001", 150).await;

    // 3. Update Atlas to quantity 120 (conflict!)
    update_atlas_quantity(&pool, inventory.id, 120).await;

    // 4. Sync with ERP_WINS strategy
    let connection = create_connection_with_strategy(
        &pool,
        ConflictResolution::ErpWins
    ).await;

    sync_bidirectional(&pool, connection.id).await;

    // 5. Verify ERP value won
    let final_inventory = get_inventory(&pool, inventory.id).await;
    assert_eq!(final_inventory.quantity, 150);
}
```

---

## 📊 Test Coverage Goals

Aim for these coverage targets:

- ✅ **Connection Management**: 100%
  - Create, read, update, delete connections
  - Test connection validation
  - Credential encryption/decryption

- ✅ **Sync Operations**: 90%+
  - Atlas → ERP sync
  - ERP → Atlas sync
  - Bidirectional sync
  - Conflict resolution (all strategies)

- ✅ **Error Handling**: 100%
  - Network failures
  - Invalid credentials
  - Rate limiting
  - Invalid data formats

- ✅ **Mapping Management**: 85%+
  - Auto-discovery
  - Manual mapping
  - Mapping updates

---

## 🔍 Debugging Tips

### Enable Detailed Logging

```bash
RUST_LOG=debug,atlas_pharma::services::erp=trace cargo run
```

### Inspect Network Requests

Use a proxy like `mitmproxy`:

```bash
# Install mitmproxy
brew install mitmproxy  # or pip install mitmproxy

# Run Atlas with proxy
HTTP_PROXY=http://localhost:8080 cargo run

# In another terminal
mitmproxy -p 8080
```

### Database Inspection

```sql
-- View ERP connections
SELECT id, connection_name, erp_type, status, last_sync_at
FROM erp_connections;

-- View sync logs
SELECT * FROM erp_sync_logs
ORDER BY created_at DESC
LIMIT 10;

-- View mappings
SELECT * FROM erp_inventory_mappings;
```

---

## 🚀 Running All Tests

```bash
# Run all tests
cargo test

# Run only ERP tests
cargo test erp

# Run with output
cargo test erp -- --nocapture

# Run specific test
cargo test test_netsuite_sync

# Run with coverage
cargo tarpaulin --out Html --output-dir coverage
```

---

## 📝 Test Checklist

Before deploying:

- [ ] All unit tests pass
- [ ] Integration tests with mock servers pass
- [ ] Manual API testing completed
- [ ] Error scenarios tested (401, 404, 500, rate limits)
- [ ] Conflict resolution strategies tested
- [ ] Audit logging verified
- [ ] Encryption/decryption tested
- [ ] Connection validation works
- [ ] Sync logs are created correctly
- [ ] Webhooks receive data correctly

---

## 🎓 Next Steps for Production

When you get access to real ERP systems:

1. **Sandbox Environments**
   - Request NetSuite sandbox account
   - Request SAP S/4HANA test system

2. **Gradual Rollout**
   - Test with 1 inventory item
   - Test with 10 items
   - Test with full inventory

3. **Monitoring**
   - Setup alerts for sync failures
   - Monitor API rate limits
   - Track sync performance

4. **Data Validation**
   - Compare Atlas vs ERP quantities
   - Verify all mappings are correct
   - Audit sync logs

---

## 💡 Tips

- Start with mock servers for rapid development
- Use integration tests for CI/CD pipeline
- Keep mock data realistic (use actual NDC codes, etc.)
- Test both success and failure paths
- Document any ERP-specific quirks you discover

---

## 🆘 Troubleshooting

**Mock server won't start:**
```bash
# Check if ports are in use
lsof -i :3001
lsof -i :3002

# Kill processes if needed
kill -9 <PID>
```

**Tests timeout:**
```bash
# Increase timeout
cargo test -- --test-threads=1 --nocapture
```

**Can't connect to database:**
```bash
# Reset database
docker-compose down -v
docker-compose up -d
cargo sqlx migrate run
```

---

Happy Testing! 🎉
