# Licensing

This chapter documents open, signed entitlement, and floating license startup modes.

Licensing mode is a runtime dependency and should be treated like any other
infrastructure requirement. The objective is to make startup behavior explicit,
auditable, and resilient: scripts should clearly communicate which mode they
expect, how fallback is handled, and what operational evidence is logged when
activation succeeds or fails.

## Open Mode

Use this for OSS-only workflows or environments where entitlement is not
required.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
print('license type:', wbe.license_type())
```

## Signed Entitlement (JSON)

Use this in managed deployments where entitlement payloads are supplied at
runtime.

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

## Signed Entitlement (File)

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

## Floating License

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
