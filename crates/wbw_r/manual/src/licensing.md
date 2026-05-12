# Licensing

This chapter documents licensing modes, tier models, and interactive activation workflows in WbW-R.

Licensing mode is an operational dependency that should be explicit in scripts.
The objective is to make startup behavior predictable and auditable: define which
mode is expected, how fallback is handled, and what diagnostics are captured when
activation fails.

## License Tiers

Whitebox NG is available in two tiers:

- **Open Tier** (`open`): Governed by MIT/Apache 2.0 dual licensing.
  All Open-tier tools are free and open-source with no entitlement required.

- **Pro Tier** (`pro`): Proprietary commercial software.
  Pro-tier tools require activation with a valid license key and are subject to
  End-User License Agreement (EULA) terms.

## Open Mode

Use this mode for OSS-only workflows or environments without entitlement needs.
No activation required.

```r
library(whiteboxworkflows)

s <- wbw_session()
print(s)
```

## Interactive License Management

For end users and interactive workflows, activate and manage Pro licenses directly
within R. Activation persists local license state so subsequent runs automatically
load the license without re-entry.

### Activating a License

```r
library(whiteboxworkflows)

# User calls activate with their license key
result <- wbw_activate_license(
  key = 'YOUR_LICENSE_KEY',
  firstname = 'John',
  lastname = 'Smith',
  email = 'john@example.com',
  agree_to_license_terms = TRUE
  # Optional: provider_url, machine_id, customer_id
)
print(result)  # Activation confirmation message
```

**What happens:**
- Server validates the key and issues a signed entitlement.
- Entitlement is verified and persisted to `~/.whitebox/wbw_ng_license_state.json`.
- Subsequent `wbw_session()` calls automatically load local state.
- If local state is expired or invalid, fallback to open tier.

### Checking License Status

```r
library(whiteboxworkflows)

info <- wbw_license_info()
print(info)
# List with fields:
# $valid: TRUE
# $effective_tier: "pro"
# $seconds_remaining: 2592000
# $expires_at_unix: 1234567890
# $now_unix: 1234567890
```

### Transferring a License

To move a license to another machine, call transfer which returns a portable
payload and clears local state:

```r
library(whiteboxworkflows)

payload <- wbw_transfer_license()
print(payload)  # Contains activation credentials for the destination machine
# Local state is now cleared on this machine
```

### Deactivating a License

```r
library(whiteboxworkflows)

result <- wbw_deactivate_license()
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

### Signed Entitlement Mode

Use this for managed deployments with entitlement payload verification.

```r
library(whiteboxworkflows)

signed_entitlement_json <- '...'

s <- wbw_session(
  signed_entitlement_json = signed_entitlement_json,
  public_key_kid = 'k1',
  public_key_b64url = 'REPLACE_WITH_PROVIDER_KEY',
  include_pro = TRUE,
  tier = 'open'
)

print(s)
```

## Floating License Mode

Use this for centrally managed license allocation across users or machines.

```r
library(whiteboxworkflows)

s <- wbw_session(
  floating_license_id = 'fl_12345',
  include_pro = TRUE,
  tier = 'open',
  provider_url = 'https://license.example.com',
  machine_id = 'machine-01',
  customer_id = 'customer-abc'
)

print(s)
```

## Failure Handling Guidance

- Validate session creation at script startup and fail early.
- Capture and log runtime startup errors for entitlement/floating modes.
- Use open mode fallback only when policy allows.

## Security and Operations Notes

- Keep entitlement payloads and keys out of source control.
- Prefer environment-variable or secret-store injection.
- Record runtime version and startup mode for reproducibility.
