# Pre-Publish Checklist — Python & R APIs

Items that must be completed before the `whitebox_workflows` Python package and the
equivalent R package are published publicly.

---

## Licensing Enforcement

### [ ] Restore floating-license verification in `wbw_python`

**File:** `crates/wbw_python/src/lib.rs`  
**Functions:** `PythonToolRuntime::new_with_options` (Pro path) and
`PythonToolRuntime::new_with_floating_license_id`

**Current state:** The `from_floating_license_id` constructor on `WbEnvironment`
accepts license parameters (`floating_license_id`, `provider_url`, `machine_id`,
`customer_id`) but silently discards them:

```rust
let _ = (floating_license_id, provider_url, machine_id, customer_id);
```

The runtime is then built using only the `fallback_tier` value the caller supplies,
with **no actual verification** against a license server.

**Why it was left this way:** `wbtools_pro::licensing` became a placeholder stub
(single comment line, no real exports) during an earlier refactor, so all imports of
`bootstrap_runtime_license_offline`, `LicensingProviderClient`,
`RuntimeLicenseBootstrapConfig`, `RuntimeLicensePolicy`, `RuntimeLicenseResolution`,
and `ResolvedCapabilities` had to be removed to keep the build green.

**What needs to happen before publish:**

1. Implement (or restore) the real exports in `wbtools_pro/src/licensing.rs`:
   - `LicensingProviderClient`
   - `RuntimeLicenseBootstrapConfig`
   - `RuntimeLicensePolicy`
   - `RuntimeLicenseResolution`
   - `ResolvedCapabilities`
   - `bootstrap_runtime_license_offline`

2. Re-introduce the provider-bootstrap call path in `new_with_floating_license_id`
   so that the supplied license credentials are actually verified and a
   `ResolvedCapabilities` / tier is returned before building the runtime.

3. Re-introduce the equivalent path in `new_with_options` (Pro feature flag) where
   applicable.

4. Add an integration test that confirms a bogus license key is **rejected** and does
   not grant Pro-tier access.

---

## (Add further items here as development continues)
