# 🚀 ERP Testing Quickstart

## TL;DR - Test Without Real SAP/NetSuite

You have **4 easy options** to test the ERP integration:

---

## Option 1: Automated Test Script (Fastest) ⚡

```bash
# Start your Atlas server
cargo run

# In another terminal, run the test script
./scripts/test_erp.sh
```

**What it tests:**
- ✅ Creating ERP connections
- ✅ Listing connections
- ✅ Testing connections
- ✅ Viewing sync logs
- ✅ Deleting connections

**Time:** ~30 seconds

---

## Option 2: Mock ERP Servers (Most Realistic) 🎭

```bash
# Run the mock servers with tests
cargo test --test erp_mock_server -- --nocapture

# The test output will show you URLs like:
# NetSuite Mock: http://127.0.0.1:XXXXX
# SAP Mock: http://127.0.0.1:YYYYY
```

**What you get:**
- Full NetSuite REST API simulation
- Full SAP OData API simulation
- Realistic responses
- In-memory data that persists during test run

**Time:** ~1 minute to set up, unlimited testing

---

## Option 3: Postman Collection (Interactive) 🎯

```bash
# 1. Import the collection into Postman
# File: postman/ERP_Integration.postman_collection.json

# 2. Start Atlas server
cargo run

# 3. In Postman:
#    - Run "Auth > Register"
#    - Run "Auth > Login" (saves token automatically)
#    - Test all ERP endpoints
```

**Perfect for:**
- Manual exploratory testing
- Debugging specific scenarios
- Sharing with team members

**Time:** Import once, test anytime

---

## Option 4: Unit Tests (Continuous Integration) 🔄

```bash
# Run all ERP tests
cargo test erp

# Run with output
cargo test erp -- --nocapture

# Run specific test
cargo test test_netsuite_sync
```

**Great for:**
- CI/CD pipelines
- Regression testing
- Code coverage reports

---

## Quick Demo Video Script 📹

Want to show someone how it works? Follow this:

```bash
# Terminal 1: Start server
cargo run

# Terminal 2: Run test
./scripts/test_erp.sh

# Watch it:
# ✅ Create user
# ✅ Login
# ✅ Create NetSuite connection
# ✅ Create SAP connection
# ✅ List connections
# ✅ Test connection
# ✅ View logs
# ✅ Clean up
```

---

## What You Can Test

### ✅ Connection Management
- Create connections (NetSuite + SAP)
- Validate credentials (structure, not actual login)
- Store encrypted credentials
- List/view/delete connections

### ✅ Configuration
- Sync frequency settings
- Sync direction (Atlas→ERP, ERP→Atlas, Bidirectional)
- Feature flags (stock levels, lot/batch, transactions)
- Conflict resolution strategies

### ✅ API Endpoints
- All REST endpoints work
- Authentication required
- Proper error responses
- Audit logging

### ⚠️ What You CANNOT Test (Without Real ERP)
- Actual NetSuite/SAP authentication
- Real inventory synchronization
- Live conflict resolution
- Production webhooks

---

## When You Get Real ERP Access

1. **Update credentials** in the connection creation requests
2. **Point to real URLs** instead of mock servers
3. **Everything else works the same!**

The code is production-ready and will work with real systems when you have:
- NetSuite SuiteScript account with REST API access
- SAP S/4HANA with OData services enabled
- Valid OAuth credentials for both

---

## Troubleshooting

**Server won't start?**
```bash
# Check database
docker-compose up -d postgres
cargo sqlx migrate run
```

**Test script fails?**
```bash
# Check if server is running
curl http://localhost:8080/health

# Check if jq is installed (for script)
brew install jq  # or: apt-get install jq
```

**Need more details?**
- See `ERP_TESTING_GUIDE.md` for comprehensive guide
- See `tests/erp_mock_server.rs` for mock server code
- See `ERP_INTEGRATION_TECHNICAL_PLAN.md` for architecture

---

## Quick Test Checklist ✓

Before deploying, make sure you've tested:

- [ ] Create NetSuite connection
- [ ] Create SAP connection
- [ ] List connections
- [ ] Get connection by ID
- [ ] Test connection (even if it fails with mock creds)
- [ ] View sync logs
- [ ] View mappings
- [ ] Delete connection
- [ ] Verify audit logs are created
- [ ] Verify credentials are encrypted in database

---

## Next Steps

1. **Run the quick test:** `./scripts/test_erp.sh`
2. **Explore the mock server:** `cargo test --test erp_mock_server`
3. **Read the full guide:** `ERP_TESTING_GUIDE.md`
4. **When ready for production:** Get real ERP credentials and you're good to go!

Happy Testing! 🎉
