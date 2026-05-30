# Typed Tool Parameter Schema Plan

## Goal

Replace frontend heuristics for tool parameter typing with a canonical,
machine-readable schema authored in the Rust tool metadata layer and shared
across Python, R, QGIS, and future frontends.

## Problem

Current runtime metadata exposes only coarse inferred fields:

1. `io_role`
2. `data_kind`

These are derived from parameter names and descriptions. That makes the
frontend contract lossy and brittle, especially for:

1. stream tools with generic parameter names
2. vector geometry restrictions
3. list inputs
4. mixed input modes such as raster-or-number
5. non-layer file inputs and outputs such as CSV and HTML

Legacy Whitebox solved this more reliably by exposing explicit parameter-type
metadata such as `ExistingFile`, `FileList`, `NewFile`, `OptionList`, and
nested data families like `Raster`, `Vector`, `Lidar`, and vector geometry
subtypes.

## Design Principles

1. Canonical parameter typing must be authored, not inferred.
2. Semantic type must be separated from frontend widget choice.
3. The shared schema must live in a frontend-neutral crate.
4. Backward compatibility must be preserved during migration.
5. Heuristics may remain only as a temporary fallback for untyped tools.

## Canonical Schema

The new schema should describe what a parameter is, not how QGIS happens to
render it.

Core dimensions:

1. Parameter role:
   input, output, argument
2. Value family:
   raster, vector, lidar, table, json, text, file, number, bool, string
3. Dataset specificity:
   vector geometry subtype, allowed file formats, mixed-family acceptance
4. Cardinality:
   single or multiple values
5. Coercion mode:
   pure dataset, dataset-or-number, dataset-or-string, etc.
6. Relationship metadata:
   field-parent binding, output loadability, memory compatibility

## Proposed Rust Model

The initial canonical model added in `wbcore` is centered on `ToolParamSchema`
and related enums.

Representative shape:

1. `ToolParamSchema`
   semantic classification for one parameter
2. `ToolDatasetSchema`
   raster, vector, lidar, table, json, text, file, or mixed
3. `ToolValueCardinality`
   single or multiple values
4. `ToolInputMode`
   existing dataset, dataset-or-number, dataset-or-string
5. `ToolOutputMode`
   new dataset, report file, sidecar file, or in-place update
6. `ToolVectorGeometry`
   point, line, polygon, line-or-polygon, any
7. `ToolScalarKind`
   integer or float

This is richer than the legacy QGIS-oriented schema because it preserves
semantics that multiple frontends can use differently.

## Compatibility Contract

For a transition period, `get_tool_metadata_json` should emit both:

1. Exact schema fields derived from `ToolParamSchema`
2. Legacy-compatible coarse fields:
   `io_role` and `data_kind`

The coarse fields should become derived convenience values, not the canonical
source of truth.

## Frontend Mapping

### QGIS

QGIS should map from exact schema first:

1. raster input -> raster layer selector
2. vector input with geometry restrictions -> feature source selector with
   geometry constraints
3. lidar/text/csv/html/json file inputs -> file selector
4. multiple datasets -> multiple-layer or file-list selector
5. output dataset schema -> destination widget with correct loadability rules
6. field schema -> attribute-field selector tied to parent vector input

Heuristic fallback should remain only for temporary compatibility with tools
that have not been migrated yet.

### Python and R

Bindings should use the exact schema for:

1. richer wrapper docs
2. argument validation
3. better generated signatures and parameter tables
4. future editor assistance and autocomplete metadata

## Migration Plan

1. Add canonical schema types to `wbcore`.
2. Emit exact schema plus compatibility fields in metadata JSON.
3. Update QGIS to consume exact schema first.
4. Add helper constructors/builders so tool authors do not hand-assemble enum
   trees repeatedly.
5. Migrate tools incrementally, starting with the most failure-prone families:
   hydrology, streams, vector, lidar.
6. Add validation so visible tools cannot silently ship without explicit schema
   once migration coverage is high enough.

## R Packaging Implications

`wbw_r` currently depends directly on workspace `wbcore` and stages that crate
into the R package build workspace. This design therefore fits the existing
architecture, provided that:

1. `wbcore` remains lightweight
2. schema types remain serialization-oriented and frontend-neutral
3. compatibility fields are preserved during rollout

No separate vendoring model is required for this change.

## Immediate Implementation Scope

This first implementation slice should do only two things:

1. define the canonical schema model in `wbcore`
2. wire metadata serialization so exact schema can be emitted alongside
   compatibility fields

Tool-by-tool migration and frontend adoption should follow in separate,
focused changes.