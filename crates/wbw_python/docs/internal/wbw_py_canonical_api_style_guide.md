# WbW-Py Canonical API Style Guide (Pre-release)

This guide defines the preferred API style for WbW-Py while the API is still pre-release.

## Scope

Applies to:
- Python-facing API surface in runtime bindings
- Stub signatures and guidance comments
- README examples and discovery docs
- New examples and smoke tests

## Core policy

- Pre-release clarity over backward compatibility.
- One preferred path for common tasks.
- Keep naming explicit and descriptive.
- Keep utility namespaces distinct from tool categories.
- Mirror conceptual patterns in WbW-R where practical.

## Canonical naming rules

1. Metadata access
- Use `metadata()` for data objects:
  - `Raster.metadata()`
  - `Vector.metadata()`
  - `Lidar.metadata()`

2. Vector attribute APIs
- Reads:
  - `schema()`
  - `attributes(feature_index)`
  - `attribute(feature_index, field_name)`
- Writes:
  - `update_attributes(feature_index, values)`
  - `update_attribute(feature_index, field_name, value)`
  - `add_field(...)`

3. Category accessors
- Use direct category properties on `WbEnvironment`:
  - `wbe.raster`, `wbe.vector`, `wbe.lidar`, `wbe.remote_sensing`, etc.
- Avoid `*_tools` property aliases in new APIs/docs/examples.

4. Utility namespaces and disambiguation
- CRS/projection helpers: `wbe.projection.*`
- Topology/geometry helpers: `wbe.topology.*`
- Topology tool category: `wbe.topology_tools`

5. Workflow ordering
- Prefer object-first chain:
  - `read_*` -> inspect `metadata()` -> run category tools -> `write_*`

## Documentation and example rules

- README examples must use canonical methods only.
- New examples should include brief comments only where intent is non-obvious.
- When showing interoperability, explicitly call out copy boundaries.
- Keep headings stable for future manual generation.

## Stub guidance rules

- Stubs should include short comments for top-level namespace intent.
- Avoid documenting removed aliases as current options.
- Keep comments concise and user-oriented.

## WbW-R parity process

For each substantial WbW-Py API change:
- Record parity decision in `wbw_py_wbw_r_parity_ledger.md` as one of:
  - `parallel now`
  - `parallel later`
  - `Python-only`
- Include rationale and proposed WbW-R action.

## Acceptance checklist for new API changes

- Is there exactly one preferred user path?
- Are names descriptive and harmonized across object types?
- Does the change keep utility-vs-category boundaries clear?
- Are README/stubs/examples updated consistently?
- Is WbW-R parity decision recorded?
