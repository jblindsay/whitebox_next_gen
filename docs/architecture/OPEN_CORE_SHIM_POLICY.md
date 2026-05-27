# Open-Core Shim Policy (Public Safe Build Graph)

## Goal

Ensure a clean clone of the public monorepo can build OSS binaries and bindings
without any private repositories present, while preserving private Pro
integration in a separate build overlay.

## Non-Negotiable Contract

1. Public manifests in this repository must not require private path
dependencies to resolve.
2. The public shim must contain no proprietary tool logic.
3. Private Pro logic must live only in the private repository.
4. Public CI must prove clean-clone OSS buildability.
5. Private CI must prove Pro integration using private override wiring.

## Shim Scope (Allowed)

The shim can contain only:

1. Public trait/interface definitions.
2. Registration hook stubs and no-op implementations.
3. Capability and tier metadata constants.
4. Error messages indicating Pro-only functionality is unavailable in OSS.

## Shim Scope (Forbidden)

The shim must never contain:

1. Any proprietary tool algorithm implementation.
2. Any embedded Pro lookup tables, coefficients, or datasets.
3. Any private licensing bypass or hidden unlock logic.
4. Any copied code from the private repository.

## Naming Decision

Use the public crate package name `wbtools_pro` for compatibility, but place it
in a clearly public folder path:

1. Folder: `crates/wbtools_pro_shim`
2. Cargo package name in that crate: `wbtools_pro`

This keeps existing dependency names stable for `wbw_python` and `wbw_r` while
making repository intent explicit.

## GitHub Repository Naming Constraint

Within one GitHub owner (user/org), you cannot have two repositories with the
same name (public and private).

Recommended:

1. Keep public monorepo as `whitebox_next_gen`.
2. Keep private repo as `wbtools_pro` (or rename to `whitebox_pro_tools`).
3. Do not create a second public repo named `wbtools_pro`.

## Workspace Placement

Recommended local layout:

1. Public monorepo:
	`/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen`
2. Private Pro repo:
	`/Users/johnlindsay/Documents/programming/Rust/wbtools_pro`

Public build uses only monorepo contents.
Private build uses a Cargo override to map `wbtools_pro` to the private repo.

## CI Gates

### Gate A (Public Safety Gate)

Run on every PR to public default branch:

1. Clean clone in isolated container (no sibling private repos).
2. Build `wbw_python` OSS mode.
3. Build `wbw_r` OSS mode.
4. Fail if any path dependency points outside repo root.
5. Fail if shim contains forbidden patterns (see Gate C).

### Gate B (Private Integration Gate)

Run in private CI only:

1. Inject Cargo override mapping `wbtools_pro` to private repo.
2. Build Pro-enabled `wbw_python` and `wbw_r`.
3. Run Pro smoke tests.

### Gate C (Leak Guard)

Run on public CI for shim paths:

1. Block proprietary code keywords or module namespace markers.
2. Block large algorithmic function bodies in shim folders.
3. Block references to private repo URLs/paths.

## Release Gate Checklist

Before any public release:

1. Public safety gate green.
2. Leak guard green.
3. Clean-clone manual spot check from a fresh temp directory.
4. Confirm no public manifest references to private sibling paths.

## Migration Plan (Low-Risk Sequence)

1. Add shim crate folder and no-op API-compatible surface.
2. Repoint `wbw_python` and `wbw_r` dependency to in-repo shim path.
3. Add public safety gate and leak guard gate.
4. Add private CI override wiring.
5. Validate R-Universe open-source build path.

## Ownership and Review Controls

1. Add CODEOWNERS rule for shim paths requiring explicit maintainer review.
2. Require two approvals for shim changes.
3. No direct pushes on protected branch for shim paths.

