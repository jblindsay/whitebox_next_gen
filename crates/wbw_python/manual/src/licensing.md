# Licensing

This chapter documents licensing modes, tier models, and interactive activation workflows.

Licensing mode is a runtime dependency and should be treated like any other
infrastructure requirement. The objective is to make startup behavior explicit,
auditable, and resilient: scripts should clearly communicate which mode they
expect, how fallback is handled, and what operational evidence is logged when
activation succeeds or fails.

## License Tiers

Whitebox NG is available in two tiers:

- **Open Tier** (`open`): Governed by MIT/Apache 2.0 dual licensing.
  All Open-tier tools are free and open-source with no entitlement required.

- **Pro Tier** (`pro`): Proprietary commercial software.
  Pro-tier tools require activation with a valid license key and are subject to
  End-User License Agreement (EULA) terms.

## Open Mode

Use this for OSS-only workflows or environments where entitlement is not
required. No activation needed.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
print('license type:', wbe.license_type())  # open
```

## Interactive License Management

For end users and interactive workflows, activate and manage Pro licenses directly
within Python. Activation persists local license state so subsequent runs
automatically load the license without re-entry.

### Activating a License

```python
import whitebox_workflows as wb

# User calls activate with their license key
result = wb.activate_license(
    key='YOUR_LICENSE_KEY',
    firstname='John',
    lastname='Smith',
    email='john@example.com',
    agree_to_license_terms=True,
    # Optional: provider_url, machine_id, customer_id
)
print(result)  # Activation confirmation message
```

**What happens:**
- Server validates the key and issues a signed entitlement.
- Entitlement is verified and persisted to `~/.whitebox/wbw_ng_license_state.json`.
- Subsequent `WbEnvironment()` calls automatically load local state.
- If local state is expired or invalid, fallback to open tier.

### Checking License Status

```python
import whitebox_workflows as wb

info = wb.license_info()
print(info)
# {
#   "valid": true,
#   "effective_tier": "pro",
#   "seconds_remaining": 2592000,
#   "expires_at_unix": 1234567890,
#   "now_unix": 1234567890
# }
```

### Transferring a License

To move a license to another machine, call transfer which returns a portable
payload and clears local state:

```python
import whitebox_workflows as wb

payload = wb.transfer_license()
print(payload)  # Contains activation credentials for the destination machine
# Local state is now cleared on this machine
```

### Deactivating a License

```python
import whitebox_workflows as wb

result = wb.deactivate_license()
print(result)  # Confirmation message
# Local state is cleared; future runs fall back to open tier
```

### Local State Persistence

Active license state is stored at `~/.whitebox/wbw_ng_license_state.json` (or
override via `WBW_LICENSE_STATE_PATH` environment variable). On each startup:
- If local state exists and is valid, it is automatically loaded.
- If local state is expired or missing, the runtime falls back to open tier.
- You do not need to re-authenticate on every run once activated.

## Programmatic Modes: Signed Entitlement and Floating License

For automated or managed deployments, use programmatic licensing modes where
entitlements are supplied directly at runtime.

### Signed Entitlement (JSON)

Use this in managed deployments where entitlement payloads are supplied at runtime.

```python
import whitebox_workflows as wb

signed_entitlement_json = '...'
public_key_kid = 'k1'
public_key_b64url = 'REPLACE_WITH_PROVIDER_KEY'

wbe = wb.WbEnvironment.from_signed_entitlement_json(
    signed_entitlement_json=signed_entitlement_json,
    public_key_kid=public_key_kid,
    public_key_b64url=public_key_b64url,
    include_pro=True,
    fallback_tier='open',
)

print(wbe.license_type())
```

### Signed Entitlement (File)

Use this when entitlement bundles are provisioned as files by platform tooling.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment.from_signed_entitlement_file(
    entitlement_file='entitlement.json',
    public_key_kid='k1',
    public_key_b64url='REPLACE_WITH_PROVIDER_KEY',
    include_pro=True,
    fallback_tier='open',
)
```

### Floating License

Use this when license allocation is centrally managed and leased per machine or
session.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment.from_floating_license_id(
    floating_license_id='fl_12345',
    include_pro=True,
    fallback_tier='open',
    provider_url='https://license.example.com',
    machine_id='machine-01',
    customer_id='customer-abc',
)

print(wbe.license_info())
```

## Failure Handling Guidance

- Validate startup at script entry and fail early on licensing errors.
- For entitlement/floating modes, keep diagnostics from thrown exceptions.
- Use open mode fallback only where policy permits.

## Security and Operations Notes

- Do not hardcode real license secrets in source control.
- Prefer environment variables or secure secret stores.
- Log license mode and runtime version for reproducibility.
