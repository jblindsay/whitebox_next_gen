# Key Generation & License Issuance Process

**Whitebox NG Pro Edition**

---

## Overview

License keys are the user-facing credential for Pro tier activation. The flow is:

1. **Customer purchases** Pro tier via sales/e-commerce
2. **Key is generated** (manually or automated)
3. **Customer receives key** via email
4. **Customer activates** using the key in Python/R/QGIS
5. **Activation endpoint** validates key and issues a signed entitlement
6. **Entitlement persists** locally for future runs

This document describes internal key generation procedures.

---

## Key Generation Mechanisms

### Automated (Post-Sale)

When a customer completes a Pro tier purchase through the e-commerce or CRM system:

1. **Trigger**: Sales/billing system creates new order for Whitebox NG Pro
2. **Signal**: Webhook or scheduled job detects new Pro subscription
3. **Generation**: Activation service (radiant-garden) generates and stores a new key
4. **Delivery**: Key is emailed to customer's registered email address
5. **Activation URL**: Optional quick-start link that pre-populates the key in the activation dialog

**Current system**: Handled by radiant-garden Go service  
**Endpoint**: Likely `/api/v2/entitlements/generate-key` or similar (admin-only)

### Manual (Admin Tool)

For special cases (reseller keys, evaluation keys, internal testing):

1. **Tool**: `whitebox_workflows_activation_key` Rust app (legacy)
2. **Flow**: Admin runs tool, specifies customer name/email/tier/expiration
3. **Output**: Generated key displayed and stored in database
4. **Email**: Admin manually emails key or batch-processes email delivery

This tool may be superseded by Go service admin endpoints.

---

## Key Format and Structure

Keys should follow a standard pattern for readability and validation:

**Proposed Format**: `WBW-XXXX-XXXX-XXXX` (8 characters, dash-separated)

Examples:
- `WBW-2026-05A1-BX3T` (12-month evaluation, issued May 2026)
- `WBW-PROD-AB12-XY4Z` (production customer)

**Metadata embedded or stored separately**:
- Issue date
- Expiration date (typically +12 months from issue)
- Tier (open, pro)
- Geographic/field restrictions (if any)
- Max simultaneous machines
- Customer name/ID (from sales system)

---

## Key Validation Flow (Server-Side)

When a user calls `activate_license(key=...)`:

1. **Client sends key** to activation endpoint
2. **Server validates key**:
   - Format check (matches pattern)
   - Exists in database and is active
   - Not yet expired
   - Not already activated on max machines (if limit enforced)
   - Geographic/field restrictions permit this activation
3. **Server issues entitlement**:
   - Fetches key metadata
   - Creates signed EntitlementDocument with tier, expiration, allowed tools
   - Signs with private key (EdDSA)
   - Returns signed entitlement + public key kid + public key b64url
4. **Client verifies entitlement**:
   - Checks signature using provided public key
   - Verifies expiration and issue dates
   - Persists to local state if valid
5. **Runtime loads** entitlement from local state on next run

---

## Current Implementation Status

### radiant-garden Go Service

The current activation service supports:

- `POST /api/v2/entitlements/issue` (admin-only, requires WBW_ADMIN_SECRET)
  - Directly issues signed entitlements (no key needed)
  - Parameters: customer_id, product, tier, expires_at_unix, allowed_tool_ids, etc.
  - Returns: signed, ready-to-use EntitlementDocument
  - Use case: Internal/admin entitlement generation

- `GET /api/v2/public-keys` → returns public keys for verification

- `POST /api/v2/entitlements/activate-floating` → handles floating license leases

**Missing endpoints for key-based activation**:
- `POST /api/v2/entitlements/activate` (customer-facing)
  - Input: license key
  - Output: signed entitlement or error
  - This endpoint **MUST be implemented** for the customer activation flow to work

### Key Activation Endpoint (To Be Implemented)

The missing piece for the current implementation is a customer-facing endpoint that:

```
POST /api/v2/entitlements/activate
Content-Type: application/json

{
  "key": "WBW-XXXX-XXXX-XXXX",
  "customer_id": "cust_abc123",
  "machine_id": "optional-machine-uuid"
}

→ 200 OK
{
  "alg": "EdDSA",
  "kid": "k1",
  "payload": {
    "entitlement_id": "ent_...",
    "customer_id": "cust_abc123",
    "product": "whitebox-ng",
    "tier": "pro",
    ...
  },
  "signature_b64url": "..."
}
```

This endpoint should:
1. Validate the key format and existence
2. Check key hasn't expired or been revoked
3. Fetch associated customer and entitlement data
4. Call the internal `/api/v2/entitlements/issue` endpoint with admin secret
5. Return the signed entitlement to the client

### Admin Entitlement Issuance (Available Now)

The `/api/v2/entitlements/issue` endpoint is already implemented and can be used for:
- Test entitlements
- Manual admin-issued entitlements
- Internal testing and QA

Requires `WBW_ADMIN_SECRET` environment variable to be set.

---

## Integration Points

### 1. Sales/E-Commerce System

When a Pro order is placed:
- Webhook triggers radiant-garden key generation endpoint
- Key is stored in database
- Email sent to customer with activation instructions

### 2. Python/R/QGIS Activation

User calls:
```python
wb.activate_license(
    key='WBW-XXXX-XXXX-XXXX',
    firstname='...',
    lastname='...',
    email='...',
    agree_to_license_terms=True
)
```

Activation handler:
1. Sends key to radiant-garden `/api/v2/entitlements/activate` endpoint
2. Server responds with signed entitlement
3. Client verifies and persists locally

### 3. Admin Dashboard (Future)

May need:
- View active keys and their status
- Manual key generation interface
- Revoke/suspend keys
- View activation history per key

---

## Security Considerations

1. **Key confidentiality**: Keys are like passwords; treat as secrets
   - Never log keys in plain text
   - Always use HTTPS for transmission
   - Encourage users not to share keys

2. **Key validation**: Server must validate thoroughly
   - Check against database of issued keys
   - Verify not blacklisted or revoked
   - Rate-limit activation attempts to prevent brute-force

3. **Entitlement signing**: Signatures must be cryptographically sound
   - Use EdDSA (current implementation)
   - Rotate keys periodically
   - Publicly distribute public keys securely

4. **Transfer restrictions**: Limit machine transfers per key
   - Track activation count per key
   - Optional: require approval for transfers
   - Revoke on excessive transfers

---

## Next Steps

1. **Verify** radiant-garden has key generation capability or extend it
2. **Set up** automated key delivery in sales/CRM workflow
3. **Test** end-to-end: purchase → key generation → activation
4. **Document** for support team: how to manually issue keys if needed
5. **Monitor** for abuse patterns (excessive transfers, concurrent activations)

---

**Last Updated**: May 12, 2026
