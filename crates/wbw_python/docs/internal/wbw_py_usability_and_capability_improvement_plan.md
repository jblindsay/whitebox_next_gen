# WbW-Py Usability and Capability Improvement Plan

## Purpose

This document turns the current high-level assessment of Whitebox Workflows for Python (WbW-Py) into a concrete improvement plan. The goal is to improve both day-to-day usability and the perceived maturity of the library without weakening the breadth and power of the existing backend.

The plan focuses on four areas:

1. Reduce the number of competing API idioms.
2. Make discovery nearly effortless from autocomplete and docs alone.
3. Tighten interoperability with the wider Python geospatial ecosystem.
4. Standardize a small number of obvious "happy path" workflows.

## Guiding Principles

1. Preserve backend power while simplifying the first 80 percent of the user experience.
2. Prefer additive migration paths before removing older entry points.
3. Optimize for discoverability in IDEs, not just completeness in docs.
4. Treat interoperability as a first-class product requirement, not a side feature.
5. Reduce choice overload by establishing one preferred pattern for common tasks.
6. Prioritize API clarity over backward compatibility while WbW-Py remains pre-release.
7. For public-facing API changes, evaluate whether a corresponding WbW-R change should be made.

## 1. Reduce Competing API Idioms

### Problem

WbW-Py currently exposes overlapping ways to accomplish similar tasks:

1. Legacy names and harmonized names coexist.
2. Tool execution can be approached through categories, environment helpers, and object methods.
3. Output control patterns are powerful but structurally dense.
4. Some features still reflect backend-driven terminology rather than Python-first naming.

This increases cognitive load, especially for new users trying to infer the "right" approach.

### Goal

Make one style clearly preferred and remove redundant paths aggressively while pre-release.

### Actions

1. Define a canonical API style guide for WbW-Py.
   - Establish preferred naming for object methods, environment helpers, namespaces, metadata access, and write operations.
   - Explicitly classify APIs as preferred, compatibility, or legacy.

2. Introduce "preferred API" markers in docs and stubs.
   - Mark compatibility aliases as temporary where they still exist.
   - Ensure examples always use the preferred path.

3. Audit overlapping methods and group them into migration buckets.
   - Keep: preferred methods that improve readability and consistency.
   - Remove now (pre-release): aliases that are redundant and add confusion.
   - Keep temporarily: only aliases needed to reduce active churn during in-flight work.

4. Rationalize output options into typed conceptual groups.
   - Separate general output controls, raster/GeoTIFF controls, vector controls, and lidar controls more explicitly.
   - Consider thin Python-side option objects or validation helpers instead of relying only on raw nested dictionaries.

5. Normalize object ergonomics.
   - Ensure Raster, Vector, and Lidar share consistent naming patterns wherever semantics match.
   - Prefer descriptive method names over backend-shaped or overly terse names.

### Deliverables

1. Internal style guide for canonical WbW-Py API design.
2. Compatibility inventory with keep/deprecate/remove recommendations.
3. Updated stubs and README examples that only use canonical patterns.
4. Follow-up RFC for any breaking cleanup after transition.

### Success Criteria

1. A new user can identify the preferred API path without reading migration notes.
2. The number of equally-valid ways to do a common task is visibly reduced.
3. Most examples use one coherent idiom end-to-end.

## Cross-Language Alignment (WbW-Py and WbW-R)

### Goal

Ensure major usability and naming improvements in WbW-Py are intentionally assessed for WbW-R alignment.

### Actions

1. Add a "WbW-R parity impact" note to each substantial WbW-Py API change proposal.
2. Tag changes as:
   - parallel now (should be implemented in WbW-R immediately),
   - parallel later (same direction, delayed implementation),
   - Python-only (intentionally language-specific ergonomics).
3. Maintain a lightweight parity ledger listing decisions and rationale.
4. Prioritize parallelization for naming, discovery, and core workflow conventions.

### Success Criteria

1. No major WbW-Py API UX change ships without an explicit WbW-R parity decision.
2. The two APIs trend toward conceptual consistency while still respecting language idioms.

## 2. Make Discovery Effortless

### Problem

WbW-Py already has significant capability, but users still need too much prior knowledge to discover it efficiently. This shows up in several ways:

1. Important capabilities are present but not obvious from the top-level environment.
2. Autocomplete is useful, but not yet sufficient as a self-guided exploration path.
3. The distinction between tools, utility namespaces, object methods, and interoperability helpers is not always immediately clear.

### Goal

A user in VS Code or a notebook should be able to discover major capabilities by typing and following autocomplete, docstrings, and a few obvious examples.

### Actions

1. Improve top-level environment discoverability.
   - Ensure major categories and utility namespaces are easy to see from `WbEnvironment`.
   - Keep category names semantically distinct from utility namespaces.

2. Expand and sharpen `.pyi` stub documentation.
   - Add concise docstrings for major classes, categories, namespaces, and frequently used methods.
   - Include short argument semantics where ambiguity is likely.

3. Build "entry point" docs around user intent.
   - Reading data.
   - Running terrain/hydrology tools.
   - Writing outputs.
   - Reprojection.
   - CRS and topology utilities.
   - Interop with NumPy, GeoPandas, Shapely, rasterio, xarray, and pyproj.

4. Create curated discovery pages.
   - "If you are coming from rasterio"
   - "If you are coming from GeoPandas/Shapely"
   - "If you are coming from whitebox_tools"
   - "Common first ten tasks in WbW-Py"

5. Add richer examples for the highest-value workflows.
   - Not just API coverage examples, but decision-making examples showing the intended path.

6. Consider a lightweight Python-side discovery helper.
   - For example: listing categories, key data object capabilities, or common interoperability conversions.
   - This should guide users without becoming a second documentation system.

### Deliverables

1. Expanded stubs/docstrings for primary public surface.
2. Intent-based documentation pages.
3. Curated example set for common first workflows.
4. Optional lightweight discovery helper API if justified.

### Success Criteria

1. Common tasks can be found from autocomplete alone.
2. Users no longer need to know backend terminology to locate Python features.
3. Questions of the form "do we support X?" decline because support is easier to find.

## 3. Tighten Interoperability

### Problem

WbW-Py is strongest when used as part of a wider Python geospatial workflow, but interoperability still risks feeling secondary instead of native.

### Goal

Make WbW-Py fit naturally into a workflow that also uses NumPy, rasterio, GeoPandas, Shapely, xarray/rioxarray, and pyproj.

### Actions

1. Standardize conversion APIs.
   - Keep conversion names predictable and object-centric.
   - Ensure round-trips are documented clearly, including lossiness or metadata caveats.

2. Improve metadata preservation rules across conversions.
   - CRS, nodata, band order, field schema, geometry type, and extent handling should be explicit.
   - Document what is preserved, inferred, or dropped.

3. Strengthen bridge examples.
   - Raster to NumPy and back.
   - Raster to rasterio/xarray/rioxarray workflows.
   - Vector to GeoPandas and Shapely.
   - CRS handoff to pyproj.

4. Reduce impedance mismatches.
   - Align band ordering, array shapes, and dtype expectations with common Python conventions where practical.
   - When divergence is necessary, provide explicit conversion helpers rather than leaving users to reshape data manually.

5. Add interop validation tests.
   - Smoke tests and round-trip tests against common Python ecosystem objects.
   - Include metadata fidelity checks and shape/dtype expectations.

6. Add explicit performance guidance.
   - Clarify when conversions are zero-copy, copy-based, metadata-only, or memory-expensive.

### Deliverables

1. Consolidated interoperability guide.
2. Interop round-trip tests for major pathways.
3. Consistent conversion API naming and behavior docs.
4. Metadata preservation matrix by conversion type.

### Success Criteria

1. WbW-Py objects can move into common Python geospatial workflows with minimal manual adaptation.
2. Users can predict array shape, CRS behavior, and metadata retention.
3. Interop examples become production-usable, not just illustrative.

## 4. Standardize Happy Path Workflows

### Problem

Too many valid choices can make a library feel harder than it is. WbW-Py needs a small set of clearly recommended end-to-end workflows that most users can follow without design decisions at every step.

### Goal

Define and document a handful of "golden path" workflows that represent the intended way to use WbW-Py today.

### Actions

1. Identify the primary workflow archetypes.
   - Raster analysis workflow.
   - Vector analysis workflow.
   - LiDAR workflow.
   - Mixed workflow using interop libraries.
   - Reprojection/CRS workflow.

2. Create one recommended pattern for each archetype.
   - Read input.
   - Inspect metadata.
   - Run tool or manipulation.
   - Write output with recommended options.
   - Convert to ecosystem objects when needed.

3. Standardize examples and tutorials around these patterns.
   - Prefer fewer, stronger examples over many partially overlapping ones.

4. Add "recommended vs advanced" framing.
   - Recommended path for most users.
   - Advanced path when explicit control is needed.

5. Formalize defaults.
   - Output formats.
   - Compression/layout recommendations.
   - Reprojection defaults.
   - Preferred metadata methods.

6. Use the happy paths to drive API cleanup.
   - If a common workflow cannot be written cleanly in a few lines, the API should be improved.

### Deliverables

1. Golden-path quickstart pages.
2. Example scripts organized by workflow archetype.
3. Consistent recommended defaults section in docs.
4. Checklist for evaluating whether new APIs improve or fragment the happy path.

### Success Criteria

1. A new user can complete common workflows without reading internal design docs.
2. Example code across README, docs, and tutorials feels consistent.
3. Advanced options remain available without obscuring the default path.

## Execution Plan

### Phase 1: Documentation and Surface Clarity

1. Remove stale or ambiguous terminology in docs.
2. Establish preferred API terminology in README and migration docs.
3. Expand stub/docstring coverage for the most important entry points.
4. Publish initial happy-path guides.

### Phase 2: API Consolidation and Discovery

1. Complete compatibility audit.
2. Mark preferred vs compatibility APIs in docs.
3. Improve namespace organization and top-level discoverability.
4. Add intent-driven documentation pages.

### Phase 3: Interop Hardening

1. Normalize major conversion APIs.
2. Add round-trip validation tests with Python ecosystem objects.
3. Document metadata preservation and performance characteristics.
4. Close major usability gaps discovered during example writing.

### Phase 4: Workflow Productization

1. Finalize golden-path workflows.
2. Align examples, docs, and stubs to those workflows.
3. Soft-deprecate the highest-confusion redundant APIs.
4. Reassess overall usability score and identify remaining blockers to a stronger public positioning.

## Prioritization

Highest-leverage work:

1. Clarify the preferred API style.
2. Improve autocomplete/docstring discoverability.
3. Publish canonical workflow examples.
4. Tighten interop guarantees.

These four items likely produce the largest user-perceived improvement without requiring major backend rework.

## Suggested Near-Term Milestones

### Milestone A

1. Canonical API style guide drafted.
2. README updated to consistently use preferred idioms.
3. Stub/docstring improvements for top-level environment and common object methods.

### Milestone B

1. Interop guide published.
2. Golden-path examples completed.
3. Conversion round-trip tests added.

### Milestone C

1. Compatibility alias audit completed.
2. First soft-deprecation recommendations prepared.
3. Public messaging updated to reflect improved usability maturity.

## Final Outcome Sought

WbW-Py should become a library that is not only powerful, but also obviously usable. The target state is:

1. Users can discover most capabilities from autocomplete and a few core docs.
2. Common workflows feel natural and consistent.
3. Interoperability with the Python geospatial stack feels deliberate and reliable.
4. The backend's breadth is no longer obscured by API complexity.