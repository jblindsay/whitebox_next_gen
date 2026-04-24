# Licensing and Tiers

WbW-QGIS is a frontend layer. Licensing authority lives in the backend runtime.

## Core Principle

The plugin reflects backend capabilities; it does not define licensing rules.

## Practical Behavior

- Open-tier tools are expected to run in standard public builds.
- Pro-tier tools may be visible but locked without eligible runtime capability.
- Requested tier and effective tier can differ based on environment and
  entitlement state.

## Why This Matters

- One plugin surface can adapt to multiple capability tiers.
- Discovery remains consistent across Python, R, and QGIS frontends.
- Execution decisions remain centralized in backend runtime logic.

## Expected Local-Dev Outcome

For most source-based setups, assume open-tier behavior unless your runtime
environment is explicitly configured for Pro-enabled integration testing.
