# Feature TODOs - Non-Security Items

This document tracks feature enhancements and non-critical improvements.

## Authentication & User Management

### Email Enumeration Prevention Enhancement
**File:** `src/services/auth_service.rs:52`
**Status:** Low Priority
**Description:** Send "account already exists" email when user tries to register with existing email

This enhances the email enumeration prevention by notifying legitimate users when someone tries to register with their email.

**Implementation:**
```rust
// TODO: Send "account already exists" email to user
// This notifies legitimate users while preventing enumeration
```

**Requires:**
- Email service integration (SMTP or SendGrid)
- Email templates
- Async email sending

---

## ERP Integration

### Webhook Event Processing
**Files:**
- `src/handlers/erp_integration.rs:833` (NetSuite)
- `src/handlers/erp_integration.rs:1033` (SAP)

**Status:** Medium Priority
**Description:** Implement webhook event processing for NetSuite and SAP

Currently webhooks are received, validated, and logged, but event processing is not implemented.

**Event Types to Handle:**

**NetSuite:**
- `inventory_updated`: Sync inventory quantities
- `item_created`: Create new pharmaceutical item
- `item_updated`: Update item details
- `order_status`: Update order status

**SAP:**
- `material_changed`: Update inventory quantities
- `material_created`: Create new pharmaceutical item
- `purchase_order_status`: Update order status

**Implementation Notes:**
- All security controls (HMAC verification, rate limiting) are already in place
- Event processing logic needs to be added
- Should use background job queue for async processing
- Add retry logic for failed events

---

## AI Import

### Track Discovery Jobs
**File:** `src/handlers/erp_ai_integration.rs` (various locations)
**Status:** Low Priority
**Description:** Track actual AI discovery jobs instead of returning placeholder data

Currently some AI endpoints return mock data. Should implement proper job tracking.

---

## Notes

All security-critical TODOs have been addressed in the security audit remediation. These remaining items are feature enhancements and can be implemented based on business priorities.

**Last Updated:** 2025-11-19
