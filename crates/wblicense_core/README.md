# wblicense_core

**THIS CRATE IS CURRENTLY EXPERIMENTAL AND IS IN AN EARLY DEVELOPMENTAL STAGE. IT IS NOT INTENDED FOR PUBLIC USAGE AT PRESENT.**

Core licensing primitives for Whitebox Next Gen (OSS).

## Project Structure Context

Whitebox Next Gen uses an open-core model:

- Backend engine crates are open source.
- The majority of the 500+ tools are open source in `wbtools_oss`.
- A proprietary paid extension product exists outside this OSS workspace.

The purpose of this model is sustainability: commercial revenue supports continued OSS development, maintenance, and user support.

## Why this crate exists

wblicense_core provides open, auditable verification logic and capability evaluation for signed entitlement documents.

This crate is intentionally in the open-source monorepo so users can inspect and verify:

- how entitlement signatures are validated
- how license capabilities are interpreted
- how denial conditions are determined
- that no hidden proprietary logic is required for local verification

## Is Whitebox still OSS if this crate exists?

Yes.

This crate does not contain proprietary business logic. It contains common trust and policy plumbing that benefits from transparency.

Keeping verification in OSS improves:

- security reviewability (signature and policy behavior are inspectable)
- reproducibility (deterministic entitlement validation)
- interoperability (Python and R bindings can share one public contract)
- long-term maintainability (stable interfaces independent of commercial server internals)

## What wblicense_core does

- Defines stable entitlement and local-state data models:
  - EntitlementDocument
  - SignedEntitlement
  - EntitlementCapabilitiesDoc
  - LocalLicenseState
  - LeaseState
- Verifies signed entitlements using Ed25519 public-key cryptography.
- Evaluates capability access via a CapabilityProvider-compatible implementation.
- Enforces standard validity checks (algorithm, key id, signature, not-before, expiry).

## Contract and API stability (Stage 1)

The crate now defines a versioned entitlement contract with `schema_version` in `EntitlementDocument`.

- Current supported version: `ENTITLEMENT_SCHEMA_VERSION = 1`
- Unsupported schema versions are rejected during verification.

Primary verification entry points:

- `verify_signed_entitlement(...)` for already-parsed envelopes
- `verify_signed_entitlement_json(...)` for raw JSON envelopes
- `parse_signed_entitlement_json(...)` for strict envelope parsing

Canonical test fixtures live under `tests/fixtures/` and provide stable sample envelopes for contract testing.

## What wblicense_core does not do

- Does not issue or sign entitlements.
- Does not manage customer purchases, billing, activation workflows, or seat accounting.
- Does not include server endpoints.
- Does not embed private keys.
- Does not include proprietary provider code.

Those concerns live in private operational components (for example, server-side issuance and provider adapters).

## Ecosystem role

In the wider Whitebox Next Gen ecosystem, this crate is the trust boundary on the client/runtime side:

1. Private server/provider components issue signed entitlements.
2. wblicense_core verifies and interprets those entitlements locally.
3. Runtime bindings (Python, R, and other hosts) consume capability decisions through shared public interfaces.

This separation preserves backward compatibility and allows dual-stack operation:

- Legacy licensing flows can continue unchanged.
- New entitlement-based flows can be introduced incrementally.

## Security model (high level)

- Signing is private: only issuer infrastructure holds private signing keys.
- Verification is public: clients/runtime use public verification keys.
- Tampering invalidates signatures.
- Expired or not-yet-valid entitlements are rejected.

## Current scope and staging

This crate is Stage 1 of the licensing rollout:

- interface stabilization
- signed entitlement verification
- capability evaluation in OSS

Later stages add private provider/server features (activation, lease lifecycle, revocation distribution) without changing the OSS verification boundary.

## License

This crate follows the workspace licensing policy declared at the repository root.
