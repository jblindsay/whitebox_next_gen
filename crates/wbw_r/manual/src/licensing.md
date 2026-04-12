# Licensing

This chapter documents open, signed entitlement, and floating license startup patterns in WbW-R.

Licensing mode is an operational dependency that should be explicit in scripts.
The objective is to make startup behavior predictable and auditable: define which
mode is expected, how fallback is handled, and what diagnostics are captured when
activation fails.

## Open Mode

Use this mode for OSS-only workflows or environments without entitlement needs.

```r
library(whiteboxworkflows)

s <- wbw_session()
print(s)
```

## Signed Entitlement Mode

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
