# Whitebox Workflows for R

Whitebox Workflows for R is the R interface for the Whitebox backend.

## Purpose

Whitebox Workflows for R exposes the shared `wbcore` runtime model to R workflows using a JSON contract. It follows the same architecture as Whitebox Workflows for Python so behavior remains consistent across language APIs.

## Current API surface

- Runtime construction:
  - `RToolRuntime::new()`
  - `RToolRuntime::new_with_options(include_pro, max_tier)`
  - `RToolRuntime::new_with_entitlement_json(include_pro, fallback_tier, signed_entitlement_json, public_key_kid, public_key_b64url)`
  - `RToolRuntime::new_with_floating_license_id(include_pro, fallback_tier, floating_license_id, provider_url, machine_id, customer_id)`
  - `whitebox_tools(floating_license_id, include_pro, tier, provider_url, machine_id, customer_id)`
- Listing APIs:
  - `list_tools_json()`
  - `list_tools_json_with_options(include_pro, tier)`
  - `list_tools_json_with_entitlement_options(...)`
  - `list_tools_json_with_entitlement_file_options(...)`
  - `list_tools_json_with_floating_license_id_options(...)`
- Execution APIs:
  - `run_tool_json(tool_id, args_json)`
  - `run_tool_json_with_options(tool_id, args_json, include_pro, tier)`
  - `run_tool_json_with_entitlement_options(...)`
  - `run_tool_json_with_entitlement_file_options(...)`
  - `run_tool_json_with_floating_license_id_options(...)`
- Helper APIs:
  - `parse_tier(tier)`
  - `generate_wrapper_stubs_json_with_options(include_pro, tier, target)`
  - `generate_r_wrapper_module_with_options(include_pro, tier)`

## Runtime behavior

- Uses composed OSS + optional Pro registries.
- Filters listed manifests by effective capability tier.
- Applies the same license checks during execution as Python.

## Licensing and startup behavior (Pro builds)

The R runtime supports three licensing paths:

- Tier-only mode (`new_with_options`, `*_with_options` APIs)
- Signed-entitlement mode (`*_with_entitlement_options` APIs)
- Provider bootstrap mode (automatic)

Provider bootstrap is automatically attempted by
`RToolRuntime::new_with_options(include_pro=true, ...)` when
`WBW_LICENSE_PROVIDER_URL` is set.

Bootstrap sequence:

1. Load persisted local license state.
2. Refresh provider public keys.
3. Re-verify cached entitlement.
4. Attempt lease renew/acquire.
5. Resolve entitlement capabilities or fallback tier behavior.

Policy is controlled by `WBW_LICENSE_POLICY`:

- `fail_open` (default): provider/bootstrap failures fall back to tier runtime
  using the configured fallback tier (typically `open`).
- `fail_closed`: provider/bootstrap failures return a license-denied error.

A missing local persistence file is treated as a first-run condition and does
not crash startup; behavior follows the selected policy.

### Licensing environment variables

| Variable | Purpose |
|---|---|
| `WBW_LICENSE_PROVIDER_URL` | Enables provider bootstrap mode when set. |
| `WBW_LICENSE_POLICY` | `fail_open` (default) or `fail_closed`. |
| `WBW_LICENSE_LEASE_SECONDS` | Optional lease target duration in seconds. |
| `WBW_LICENSE_STATE_PATH` | Optional explicit local license-state file path. |

## Floating license behavior (current status)

"Floating license ID only" activation on a brand-new machine is now supported
when provider bootstrap is enabled and the provider exposes the floating
activation endpoint.

Current implementation supports:

- Lease renew/acquire after a valid entitlement already exists in local state
  (or is provided explicitly through entitlement APIs).
- Graceful first-run behavior when no persistence file exists (policy-driven
  fail-open/fail-closed).
- New-machine online activation by exchanging floating license id + machine id
  for a signed entitlement during bootstrap.

Requirements for floating activation:

- `WBW_LICENSE_PROVIDER_URL` must be configured.
- `WBW_FLOATING_LICENSE_ID` must be configured.
- Provider must expose `POST /api/v2/entitlements/activate-floating`.

## Operational runbook (online-first)

### Existing machine (has local state)

1. Set provider URL and policy environment variables.
2. Construct runtime with `include_pro=true`.
3. Runtime loads state, verifies entitlement, then renews/acquires lease.

### New machine (no local state file)

1. Set provider URL and policy environment variables.
2. Provide `WBW_FLOATING_LICENSE_ID` (and optional machine/customer id vars).
3. Runtime attempts floating activation, persists entitlement state, then
  proceeds with lease lifecycle.
4. If activation/bootstrap fails, behavior follows policy:
   - `fail_open`: fallback tier runtime (typically `open`)
   - `fail_closed`: license-denied error

## Troubleshooting

| Symptom | Likely cause | Current behavior | Action |
|---|---|---|---|
| Pro tools not visible on new machine | Floating activation failed (missing ID, unauthorized ID, provider unavailable) | Falls back to `open` in `fail_open`; denied in `fail_closed` | Set `WBW_FLOATING_LICENSE_ID`, verify provider endpoint and floating-ID allow-list |
| Runtime constructor returns license-denied | `WBW_LICENSE_POLICY=fail_closed` and bootstrap cannot establish valid entitlement | Constructor fails | Switch to `fail_open` for OSS fallback or provide valid entitlement state |
| Provider endpoint errors | Network outage, wrong URL, or provider downtime | `fail_open` fallback or `fail_closed` error | Verify `WBW_LICENSE_PROVIDER_URL`, network path, and provider service |
| Signature verification fails | Key mismatch (`kid`/public key) or stale signing material | Entitlement path rejected | Refresh keys and ensure provider signing key/kid is aligned with client verification |

Additional floating activation env vars:

- `WBW_FLOATING_LICENSE_ID` (required for floating-ID activation)
- `WBW_MACHINE_ID` (optional override; defaults to hostname when available)
- `WBW_CUSTOMER_ID` (optional customer id hint)

## Teaching/lab/notebook environments (no admin privileges)

You can now provide floating-license parameters directly in API calls instead
of relying on persistent machine environment-variable configuration. This is
better suited to locked-down labs, classroom images, and notebook sessions.

## Parity acceleration: generated R wrappers

To quickly expand callable R coverage while preserving a single tool registry,
`generate_r_wrapper_module_with_options(include_pro, tier)` emits a complete R
wrapper module string:

- Includes `wbw_make_session(...)` with `session$run_tool(...)` and
  `session$list_tools(...)` helpers.
- Supports floating-id startup directly in session construction via
  `floating_license_id`, `provider_url`, `machine_id`, and `customer_id`.
- Includes a shared `wbw_run_tool(...)` helper for non-session one-liners.
- Includes one R function per visible tool manifest.
- Uses tool id -> function name mapping by replacing `-` with `_`.

This is the recommended first stage for bringing R API coverage in step with
Python before ergonomic polish.

### Generate wrapper module file

Generate a full wrapper module to disk:

```bash
cargo run -p wbw_r --example generate_r_wrappers -- --tier open --output crates/wbw_r/generated/wbw_tools_generated.R
```

Include Pro-visible manifests (Pro builds):

```bash
cargo run -p wbw_r --features pro --example generate_r_wrappers -- --include-pro --tier pro --output crates/wbw_r/generated/wbw_tools_generated_pro.R
```

### Thin facade layer

Because `wbw_r` is not yet laid out as a full installed R package, the current
intermediate step is a sourceable facade file:

- [generated/wbw_tools_facade.R](generated/wbw_tools_facade.R)
- [examples/generated_session_example.R](examples/generated_session_example.R)

The facade exposes a stable small surface over the generated module:

- `whitebox_tools(...)` returns a session object
- `wbw_list_tools(...)` lists visible tools
- `wbw_run_tool(tool_id, args, ...)` executes any tool

Example:

```r
source("crates/wbw_r/generated/wbw_tools_facade.R")

session <- whitebox_tools()
tools <- session$list_tools()

# Floating-license classroom/notebook session
# session <- whitebox_tools(
#   floating_license_id = "FLOAT-ABC-123",
#   include_pro = TRUE,
#   tier = "open",
#   provider_url = "https://your-provider.example.com"
# )
```

### Package scaffold

An actual R package scaffold now exists at
[r-package/whiteboxworkflows](r-package/whiteboxworkflows).

Current contents:

- `DESCRIPTION` and `NAMESPACE`
- `R/bindings.R` registration bridge for native runtime functions
- `R/facade.R` stable user-facing entry points
- `R/zz_generated_wrappers.R` copied generated wrappers
- `inst/examples/generated_session_example.R`
- native Rust export layer in the `wbw_r` crate via extendr
- package `src/` build bridge that compiles the Rust static library during
  `R CMD INSTALL`

Current development workflow:

```bash
R CMD INSTALL crates/wbw_r/r-package/whiteboxworkflows
```

Optional build environment variables:

- `WBW_R_PACKAGE_PRO=true` builds the package against the Pro-enabled runtime.
- `WBW_R_PACKAGE_RELEASE=true` builds the Rust library in release mode.

The package now builds the `wbw_r` static library in `src/Makevars`, links it
into the package shared library via `src/entrypoint.c`, and relies on
`useDynLib(whiteboxworkflows, ...)` for package-native routine loading.

### Development sync workflow

The manual staging script is still useful when you want to refresh the package
sources without reinstalling the package:

```bash
bash crates/wbw_r/scripts/dev_r_package_sync.sh
```

This workflow:

- builds the `wbw_r` native library as a loadable dylib/shared object,
- copies it to `r-package/whiteboxworkflows/inst/libs/`,
- regenerates the wrapper module,
- refreshes `R/zz_generated_wrappers.R` in the package scaffold.

The package-native path is now the primary integration route. The staging
workflow remains a development convenience until package install/load testing is
fully hardened.

The package scaffold now also includes a minimal `testthat` suite under
`r-package/whiteboxworkflows/tests/testthat` for install/load/runtime smoke
coverage.

Current local prerequisite for the higher-level generated/session API:

- `jsonlite` must be installed in R (already declared in `DESCRIPTION`).

### Parity gate tests

The test suite includes a parity gate that verifies generated wrapper count and
wrapper names match the current visible manifest set.

```bash
cargo test -p wbw_r r_wrapper_module_generation_matches_manifest_count_and_names
```

## Script example: floating license id (online, new machine)

```r
Sys.setenv(WBW_LICENSE_POLICY = "fail_open")
Sys.setenv(WBW_LICENSE_LEASE_SECONDS = "3600")

# Replace with your package wrapper that maps to
# list_tools_json_with_floating_license_id_options.
# Example shape:
# tools_json <- whiteboxworkflows::list_tools_json_with_floating_license_id_options(
#   floating_license_id = "FLOAT-ABC-123",
#   include_pro = TRUE,
#   fallback_tier = "open",
#   provider_url = "https://your-provider.example.com",
#   machine_id = NULL,
#   customer_id = NULL
# )
# cat(tools_json)
```

Full script template: [examples/licensing_floating_online.R](examples/licensing_floating_online.R)

Legacy-style compatibility helper shape:

```r
# Replace with your package wrapper for wbw_r::whitebox_tools(...)
# Example shape:
# wbe <- whiteboxworkflows::whitebox_tools(
#   floating_license_id = "FLOAT-ABC-123",
#   include_pro = TRUE,
#   tier = "open",
#   provider_url = "https://your-provider.example.com",
#   machine_id = NULL,
#   customer_id = NULL
# )
```

## Script examples: offline mode

### A) Offline OSS fallback (no provider)

```r
Sys.unsetenv("WBW_LICENSE_PROVIDER_URL")
Sys.unsetenv("WBW_FLOATING_LICENSE_ID")

# Replace with your package wrapper.
# tools_json <- whiteboxworkflows::list_tools_json_with_options(include_pro = FALSE, tier = "open")
# cat(tools_json)
```

### B) Offline signed-entitlement mode (no provider call)

```r
# Replace with your package wrapper that maps to *_with_entitlement_options.
# tools_json <- whiteboxworkflows::list_tools_json_with_entitlement_options(
#   signed_entitlement_json = readChar("signed_entitlement.json", file.info("signed_entitlement.json")$size),
#   public_key_kid = "k1",
#   public_key_b64url = "REPLACE_WITH_PROVIDER_PUBLIC_KEY",
#   include_pro = TRUE,
#   fallback_tier = "open"
# )
# cat(tools_json)
```

Full script template: [examples/licensing_offline.R](examples/licensing_offline.R)

## Testing

Run:

```bash
cargo test -p wbw_r
```

Tests cover Open/Pro visibility, execution behavior, invalid tier handling, and wrapper stub generation.
