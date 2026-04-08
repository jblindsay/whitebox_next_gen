# wbw_python Pro Tool Visibility and Discoverability Spec

Status: Draft
Owner: wbw_python
Last updated: 2026-04-07

## 1. Purpose

Define a consistent API UX for OSS and Pro tools so users can:

- discover tools without confusion,
- see Pro status before execution,
- understand licensing failures immediately,
- and avoid accidental dead ends from IDE autocomplete.

This is an internal implementation spec and should not be mixed with tool reference docs.

## 2. Problem Summary

`include_pro=False` currently hides Pro tools from runtime listing and execution surfaces, but static Python method autocomplete may still show Pro wrapper methods in some IDEs because those methods are generated on the class surface.

Result: users can select a Pro method from autocomplete and only discover licensing constraints at runtime.

## 3. Design Goals

- Keep OSS and Pro tools in a unified domain model (no separate "Pro-only" API tree).
- Make Pro status obvious at selection time, not only at runtime.
- Preserve backward compatibility for existing API calls.
- Provide machine-readable metadata for downstream UIs and wrappers.

## 4. Required UX Signals

### 4.1 Autocomplete / Hover Signal

Every Pro-exposed wrapper method must include a leading docstring banner:

- Format: `[PRO] <one-line summary>`
- Example: `[PRO] Coregisters moving SAR to a reference SAR scene.`

For OSS methods, no tier badge is required in the first line.

### 4.2 Runtime License Error Signal

When a Pro tool is called without required visibility/tier/capability, the first error line must be explicit and stable:

- `This is a PRO tool: <tool_id>`

Then include:

- current runtime settings (`include_pro`, requested tier, effective tier),
- specific reason (not visible vs tier-gated vs entitlement failure),
- one actionable next step.

Preferred full template:

1. `This is a PRO tool: <tool_id>.`
2. `Current runtime: include_pro=<true|false>, tier=<open|pro|enterprise>, effective_tier=<...>.`
3. `Reason: <short reason>.`
4. `Action: enable include_pro=True and use a valid Pro/Enterprise entitlement.`

### 4.3 Discovery Metadata Signal

All list/search/describe payloads must expose tier metadata fields:

- `license_tier`: `open|pro|enterprise`
- `is_pro`: boolean
- `available_in_current_session`: boolean
- `availability_reason`: optional string when unavailable

Note: `available_in_current_session` is runtime-contextual and should be produced by session-aware listing APIs.

## 5. API Additions (Backward Compatible)

## 5.1 New Session-Aware Discovery APIs

Add functions (names can be finalized during implementation):

- `WbEnvironment.describe_tool(tool_id: str) -> dict`
- `WbEnvironment.search_tools(query: str, include_locked: bool = False) -> list[dict]`
- `WbEnvironment.list_tools_detailed(include_locked: bool = False) -> list[dict]`

Behavior:

- `include_locked=False` returns only available tools.
- `include_locked=True` includes unavailable tools and sets:
  - `available_in_current_session=false`
  - `availability_reason` explaining why.

## 5.2 Existing API Behavior

No breaking change to:

- `WbEnvironment(...)`
- `RuntimeSession(...)`
- `run_tool_*`
- `list_tools_*`

Existing calls remain valid.

## 6. Error Taxonomy for Pro Access

Use stable, parseable reasons in `availability_reason` and runtime errors:

- `pro_not_included` (`include_pro=False`)
- `build_without_pro` (binary not built with Pro feature)
- `tier_insufficient` (effective tier below required)
- `entitlement_missing_or_invalid`

## 7. Implementation Plan

### Phase 1: Fast UX Wins (No API Breaks)

- Add `[PRO]` banner to generated Pro wrapper docstrings.
- Standardize runtime error text for Pro access failures.
- Add optional `license_info()` guidance sentence in docs/examples showing how to self-check session state.

Acceptance checks:

- Hover/autocomplete on any Pro wrapper displays `[PRO]` prefix.
- Unlicensed Pro call returns standardized first-line message.

### Phase 2: Structured Discovery

- Add detailed discovery methods (`describe_tool`, `list_tools_detailed`, `search_tools`).
- Include `available_in_current_session` and `availability_reason`.

Acceptance checks:

- Users can discover both available and locked tools without ambiguity.
- Metadata can drive UI badges without string parsing.

### Phase 3: IDE Experience Improvements (Optional)

- Evaluate generating tier-specific `.pyi` stubs:
  - OSS-focused stub (default)
  - Pro-enabled stub (for licensed dev envs)
- If adopted, ship via optional generation command and docs.

Acceptance checks:

- OSS users can choose a stub path with minimal Pro autocomplete noise.

## 8. Documentation Placement

- Keep this file under `docs/internal/`.
- Keep user-facing tool docs (`TOOLS.md`, themed docs) focused on tool semantics and examples, not implementation policy.

## 9. Open Questions

- Should `[PRO]` be ASCII-only (`[PRO]`) or icon-based in docs? Recommendation: ASCII-only for portability.
- Should `include_locked=True` default to true in enterprise contexts? Recommendation: default false everywhere for least surprise.
- Do we expose OSS alternative suggestions in runtime errors now or defer to Phase 2 metadata matching?

## 10. Recommended Defaults

- Keep unified domain namespaces for OSS + Pro.
- Keep `include_pro=False` as default.
- Keep default discovery to available tools only.
- Make Pro status highly visible in docstrings and metadata.
