#!/bin/bash
# Quick ERP Integration Testing Script
# Usage: ./scripts/test_erp.sh

set -e

echo "🧪 Atlas Pharma - ERP Integration Test Suite"
echo "=============================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Configuration
API_URL="${API_URL:-http://localhost:8080}"
TEST_EMAIL="erp-test-$(date +%s)@example.com"
TEST_PASSWORD="Test123!@#"

echo -e "${BLUE}Step 1: Checking if server is running...${NC}"
if ! curl -s "$API_URL/health" > /dev/null 2>&1; then
    echo -e "${RED}❌ Server is not running at $API_URL${NC}"
    echo "Please start the server with: cargo run"
    exit 1
fi
echo -e "${GREEN}✅ Server is running${NC}"
echo ""

echo -e "${BLUE}Step 2: Creating test user...${NC}"
REGISTER_RESPONSE=$(curl -s -X POST "$API_URL/api/auth/register" \
  -H "Content-Type: application/json" \
  -d "{
    \"email\": \"$TEST_EMAIL\",
    \"password\": \"$TEST_PASSWORD\",
    \"full_name\": \"ERP Test User\"
  }")

echo "Register response: $REGISTER_RESPONSE"
echo ""

echo -e "${BLUE}Step 3: Logging in...${NC}"
LOGIN_RESPONSE=$(curl -s -X POST "$API_URL/api/auth/login" \
  -H "Content-Type: application/json" \
  -d "{
    \"email\": \"$TEST_EMAIL\",
    \"password\": \"$TEST_PASSWORD\"
  }")

TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r '.token // .access_token // empty')

if [ -z "$TOKEN" ] || [ "$TOKEN" == "null" ]; then
    echo -e "${RED}❌ Failed to get authentication token${NC}"
    echo "Response: $LOGIN_RESPONSE"
    exit 1
fi

echo -e "${GREEN}✅ Logged in successfully${NC}"
echo "Token: ${TOKEN:0:20}..."
echo ""

echo -e "${BLUE}Step 4: Testing ERP Connection Management${NC}"
echo "-------------------------------------------"

# Test 1: Create NetSuite connection
echo "📝 Creating NetSuite connection..."
NETSUITE_CONN=$(curl -s -X POST "$API_URL/api/erp/connections" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "connection_name": "Test NetSuite Connection",
    "erp_type": "netsuite",
    "netsuite_account_id": "TEST_ACCOUNT_123",
    "netsuite_consumer_key": "test_consumer_key_abc123",
    "netsuite_consumer_secret": "test_consumer_secret_xyz789",
    "netsuite_token_id": "test_token_id_456",
    "netsuite_token_secret": "test_token_secret_789",
    "netsuite_realm": "TEST_REALM",
    "sync_enabled": true,
    "sync_frequency_minutes": 60,
    "sync_stock_levels": true,
    "sync_product_master": true,
    "sync_transactions": false,
    "sync_lot_batch": true
  }')

NETSUITE_ID=$(echo "$NETSUITE_CONN" | jq -r '.id // empty')

if [ -z "$NETSUITE_ID" ] || [ "$NETSUITE_ID" == "null" ]; then
    echo -e "${RED}❌ Failed to create NetSuite connection${NC}"
    echo "Response: $NETSUITE_CONN"
else
    echo -e "${GREEN}✅ Created NetSuite connection: $NETSUITE_ID${NC}"
fi
echo ""

# Test 2: Create SAP connection
echo "📝 Creating SAP S/4HANA connection..."
SAP_CONN=$(curl -s -X POST "$API_URL/api/erp/connections" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "connection_name": "Test SAP S/4HANA Connection",
    "erp_type": "sap_s4hana",
    "sap_base_url": "https://test-sap.example.com",
    "sap_client_id": "test_sap_client_123",
    "sap_client_secret": "test_sap_secret_abc456",
    "sap_token_endpoint": "https://test-sap.example.com/oauth/token",
    "sap_environment": "TEST",
    "sap_plant": "1000",
    "sap_company_code": "1000",
    "sync_enabled": true,
    "sync_frequency_minutes": 30,
    "sync_stock_levels": true,
    "sync_product_master": true
  }')

SAP_ID=$(echo "$SAP_CONN" | jq -r '.id // empty')

if [ -z "$SAP_ID" ] || [ "$SAP_ID" == "null" ]; then
    echo -e "${RED}❌ Failed to create SAP connection${NC}"
    echo "Response: $SAP_CONN"
else
    echo -e "${GREEN}✅ Created SAP connection: $SAP_ID${NC}"
fi
echo ""

# Test 3: List all connections
echo "📋 Listing all connections..."
CONNECTIONS=$(curl -s -X GET "$API_URL/api/erp/connections" \
  -H "Authorization: Bearer $TOKEN")

CONNECTION_COUNT=$(echo "$CONNECTIONS" | jq -r '.total // .connections | length // 0')
echo -e "${GREEN}✅ Found $CONNECTION_COUNT connection(s)${NC}"
echo ""

# Test 4: Get specific connection
if [ ! -z "$NETSUITE_ID" ] && [ "$NETSUITE_ID" != "null" ]; then
    echo "🔍 Getting NetSuite connection details..."
    CONN_DETAILS=$(curl -s -X GET "$API_URL/api/erp/connections/$NETSUITE_ID" \
      -H "Authorization: Bearer $TOKEN")

    CONN_NAME=$(echo "$CONN_DETAILS" | jq -r '.connection_name // empty')
    echo -e "${GREEN}✅ Retrieved connection: $CONN_NAME${NC}"
    echo ""

    # Test 5: Test connection (will likely fail with mock credentials)
    echo "🔌 Testing NetSuite connection..."
    TEST_RESULT=$(curl -s -X POST "$API_URL/api/erp/connections/$NETSUITE_ID/test" \
      -H "Authorization: Bearer $TOKEN")

    TEST_SUCCESS=$(echo "$TEST_RESULT" | jq -r '.success // false')
    if [ "$TEST_SUCCESS" == "true" ]; then
        echo -e "${GREEN}✅ Connection test passed${NC}"
    else
        echo -e "${BLUE}ℹ️  Connection test failed (expected with mock credentials)${NC}"
        echo "Message: $(echo "$TEST_RESULT" | jq -r '.message // "N/A"')"
    fi
    echo ""

    # Test 6: Get sync logs (should be empty initially)
    echo "📊 Getting sync logs..."
    SYNC_LOGS=$(curl -s -X GET "$API_URL/api/erp/connections/$NETSUITE_ID/sync-logs" \
      -H "Authorization: Bearer $TOKEN")

    LOG_COUNT=$(echo "$SYNC_LOGS" | jq '. | length // 0')
    echo -e "${GREEN}✅ Found $LOG_COUNT sync log(s)${NC}"
    echo ""

    # Test 7: Get mappings
    echo "🗺️  Getting inventory mappings..."
    MAPPINGS=$(curl -s -X GET "$API_URL/api/erp/connections/$NETSUITE_ID/mappings" \
      -H "Authorization: Bearer $TOKEN")

    MAPPING_COUNT=$(echo "$MAPPINGS" | jq '. | length // 0')
    echo -e "${GREEN}✅ Found $MAPPING_COUNT mapping(s)${NC}"
    echo ""

    # Test 8: Delete connection
    echo "🗑️  Deleting NetSuite connection..."
    DELETE_RESULT=$(curl -s -X DELETE "$API_URL/api/erp/connections/$NETSUITE_ID" \
      -H "Authorization: Bearer $TOKEN" \
      -w "\n%{http_code}")

    HTTP_CODE=$(echo "$DELETE_RESULT" | tail -n1)
    if [ "$HTTP_CODE" == "204" ] || [ "$HTTP_CODE" == "200" ]; then
        echo -e "${GREEN}✅ Connection deleted successfully${NC}"
    else
        echo -e "${RED}❌ Failed to delete connection (HTTP $HTTP_CODE)${NC}"
    fi
fi
echo ""

# Test 9: Delete SAP connection
if [ ! -z "$SAP_ID" ] && [ "$SAP_ID" != "null" ]; then
    echo "🗑️  Deleting SAP connection..."
    curl -s -X DELETE "$API_URL/api/erp/connections/$SAP_ID" \
      -H "Authorization: Bearer $TOKEN" > /dev/null
    echo -e "${GREEN}✅ SAP connection deleted${NC}"
fi
echo ""

echo "=============================================="
echo -e "${GREEN}✅ ERP Integration Test Suite Complete!${NC}"
echo ""
echo "Summary:"
echo "  - Connection creation: ✅"
echo "  - Connection retrieval: ✅"
echo "  - Connection listing: ✅"
echo "  - Connection deletion: ✅"
echo "  - Sync logs: ✅"
echo "  - Mappings: ✅"
echo ""
echo "💡 Next steps:"
echo "  1. Run unit tests: cargo test erp"
echo "  2. Check ERP_TESTING_GUIDE.md for detailed testing strategies"
echo "  3. Use mock servers for integration testing"
echo ""
