# Licensing

This chapter documents open, signed entitlement, and floating license startup modes.

## Open Mode

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
print('license type:', wbe.license_type())
```

## Signed Entitlement (JSON)

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
