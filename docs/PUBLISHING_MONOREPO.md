# Whitebox Next Gen Monorepo Publishing Guide

This runbook describes:

1. How to publish the backend crates from this monorepo.
2. How to publish a single individual crate.
3. The checks to run before and after publishing.

This guide applies only to crates intended for crates.io publication. Not every crate in this workspace should ever be published to crates.io.

## 1. Scope and Current Publish Order

Current backend crates covered by this guide:

- wbgeotiff
- wbprojection
- wbraster
- wbvector
- wbtopology
- wblidar

Required publish order:

1. wbgeotiff
2. wbprojection
3. wbraster
4. wbvector
5. wbtopology
6. wblidar

Why order matters:

- Several crates depend on sibling crates that must already exist on crates.io.
- A dependency crate must be published before dependent crates can pass package verification.

Crates in this workspace that are intentionally excluded from crates.io publishing:

- wbcore
- wblicense_core
- wbtools_oss
- wbphotogrammetry
- wbw_python
- wbw_r

These crates are frontend plumbing, internal support crates, or product-specific integration layers. They are GitHub-only crates and must keep `publish = false` in their Cargo.toml manifests.

## 2. One-Time Setup (First Time Only)

### 2.1 Accounts and credentials

1. Create or confirm crates.io account.
2. Generate a crates.io API token.
3. Log in locally:

```bash
cargo login <CRATES_IO_TOKEN>
```

### 2.2 Repository metadata consistency

Ensure backend Cargo.toml metadata points to the canonical repo URL.

Use the helper script:

```bash
bash scripts/set_backend_repo_metadata.sh https://github.com/jblindsay/whitebox_next_gen
```

### 2.3 Build sanity

```bash
cargo check -p wbgeotiff -p wbprojection -p wbraster -p wbvector -p wbtopology -p wblidar
```

## 3. Pre-Publish Checks (Run Every Release)

### 3.1 Backend readiness and strict package surface policy

```bash
bash scripts/check_backend_publish_readiness.sh
```

This script currently checks:

- Required package metadata fields.
- Non-publishable crates retain `publish = false`.
- Strict no-internal-files package policy.
- Dry packaging behavior for each backend crate.

### 3.2 Publish-order dry run

Stop at first blocked crate:

```bash
bash scripts/publish_backend_dry_run.sh
```

Report all blocked crates:

```bash
bash scripts/publish_backend_dry_run.sh --continue
```

### 3.3 Public-boundary guard (required)

Before any push to mainline branches, run:

```bash
bash scripts/check_public_boundary.sh
```

This blocks changes to restricted public-sensitive paths by default:

- crates/wbcore/
- crates/wblicense_core/
- crates/wbtools_oss/
- crates/wbw_python/
- crates/wbw_r/

Install the local pre-push hook once per clone:

```bash
bash scripts/install_boundary_pre_push_hook.sh
```

CI also enforces this with:

- .github/workflows/public-boundary-guard.yml

If an emergency exception is truly required, use explicit override:

```bash
PUBLIC_BOUNDARY_OVERRIDE=I_UNDERSTAND_THIS_IS_PUBLIC bash scripts/check_public_boundary.sh
```

Any override usage should be considered a release-governance event and reviewed.

## 4. Publishing the Full Backend Set (Monorepo Release)

Use this when releasing all backend crates in sequence.

Do not use this workflow for wbcore, wblicense_core, wbtools_oss, wbphotogrammetry, wbw_python, or wbw_r. Those crates are intentionally excluded from crates.io publication.

### 4.1 Publish wbgeotiff

```bash
cargo publish -p wbgeotiff
```

Wait until crates.io and index propagation complete. Then verify:

```bash
cargo search wbgeotiff --limit 1
```

### 4.2 Publish wbprojection

```bash
cargo publish -p wbprojection
```

Verify:

```bash
cargo search wbprojection --limit 1
```

### 4.3 Publish wbraster

```bash
cargo publish -p wbraster
```

### 4.4 Publish wbvector

```bash
cargo publish -p wbvector
```

### 4.5 Publish wbtopology

```bash
cargo publish -p wbtopology
```

### 4.6 Publish wblidar

```bash
cargo publish -p wblidar
```

### 4.7 Post-publish verification

1. Confirm each crate appears on crates.io at expected version.
2. In a clean temporary project, add dependencies and run cargo check.
3. Confirm docs.rs build status for each crate.

## 5. Publishing One Individual Crate

Yes, there are separate steps when publishing just one crate.

Use this workflow:

1. Choose target crate and target version.
2. Confirm the target crate is one of the six backend crates covered by this runbook.
3. Update that crate version in its Cargo.toml.
4. If sibling dependency versions changed, update dependent version requirements.
4. Run targeted checks:

```bash
cargo check -p <crate_name>
cargo package -p <crate_name> --allow-dirty --no-verify
```

5. If dependency crate is not yet on crates.io at the required version, publish dependency first.
6. Publish target crate:

```bash
cargo publish -p <crate_name>
```

7. Verify via cargo search and crates.io page.

Example, publish only wbvector:

1. Ensure wbprojection required version already exists on crates.io.
2. Run cargo check -p wbvector.
3. Run cargo package -p wbvector --allow-dirty --no-verify.
4. Run cargo publish -p wbvector.

If the target crate has `publish = false`, stop. That crate is outside the crates.io publishing scope of this repository.

## 6. Versioning Guidance

Current backend crates are at 0.1.0.

Suggested approach while APIs are still evolving:

- Use 0.x minor bumps for breaking changes.
- Use patch bumps for bug fixes and docs-only corrections.

Example progression:

- 0.1.0 -> 0.1.1 for non-breaking fixes.
- 0.1.0 -> 0.2.0 for breaking changes.

## 7. Common Failure Modes and Fixes

### Error: no matching package named <crate> found

Cause:

- Publish order not satisfied, or crates.io index has not propagated yet.

Fix:

1. Publish missing dependency crate first.
2. Wait and retry after propagation delay.

### Error: all dependencies must have a version requirement specified

Cause:

- Path dependency missing version field.

Fix:

- Use path + version for local sibling dependencies, for example:

```toml
wbprojection = { path = "../wbprojection", version = "0.1.0" }
```

### Internal/development files leaking into package

Cause:

- Files under internal locations are included by manifest/package settings.

Fix:

1. Move maintainer assets under excluded paths such as dev or docs/internal.
2. Re-run strict readiness script.

### Error: package is marked as unpublishable

Cause:

- The crate manifest contains `publish = false`.

Fix:

- Do not override this for internal/frontend crates. This is the expected guardrail for crates that are GitHub-only and not part of the crates.io release set.

## 8. Maintainer-Only Workflows

Maintainer-only utilities are kept under excluded dev paths.

List available maintainer commands:

```bash
bash scripts/run_maintainer_workflows.sh list
```

List projection internal tools:

```bash
bash scripts/run_maintainer_workflows.sh projection-list-tools
```

List topology internal tools:

```bash
bash scripts/run_maintainer_workflows.sh topology-list-tools
```

Run topology perf gate:

```bash
bash scripts/run_maintainer_workflows.sh topology-perf-gate
```

## 9. Suggested Release Checklist

1. Run metadata and strict package checks.
2. Run publish dry-run in order.
3. Confirm no non-publishable crate has been added to the crates.io release plan.
4. Publish in required sequence.
5. Verify crates.io and docs.rs.
6. Tag release in git and update changelog/release notes.
