# WbW-Py <-> WbW-R Parity Ledger (Initial)

This ledger records parity decisions for substantial WbW-Py API changes.

Status values:
- `parallel now`: implement equivalent change in WbW-R immediately.
- `parallel later`: same direction, scheduled for later phase.
- `Python-only`: intentionally language-specific ergonomics.

## Entries

1. Remove `Raster.configs()` in favor of `Raster.metadata()`
- WbW-Py decision: removed in pre-release Phase 1.
- WbW-R parity: `parallel now`.
- Rationale: harmonized metadata naming across object types improves discoverability.
- Proposed WbW-R action: ensure canonical metadata accessor naming is consistent in docs/stubs and remove redundant aliases if present.

2. Remove vector `get_*`/`set_*`/`add_attribute_field` aliases in favor of canonical methods
- WbW-Py decision: removed in pre-release Phase 1.
- WbW-R parity: `parallel now`.
- Rationale: old verb-heavy names duplicate functionality and increase surface-area confusion.
- Proposed WbW-R action: align vector attribute API around one read/write naming family.

3. Remove `wbe.*_tools` property aliases (`raster_tools`, `vector_tools`, `lidar_tools`, `remote_sensing_tools`)
- WbW-Py decision: removed in pre-release Phase 1.
- WbW-R parity: `parallel later`.
- Rationale: direct category names are clearer; R-facing ergonomics may prefer a slightly different naming pattern.
- Proposed WbW-R action: evaluate whether direct category naming can replace or supersede legacy category aliases.

4. Keep `wbe.topology_tools` alongside `wbe.topology` utility namespace
- WbW-Py decision: retained as canonical disambiguation.
- WbW-R parity: `parallel now`.
- Rationale: explicit split between utility namespace and tool category avoids type confusion.
- Proposed WbW-R action: preserve equivalent utility/category disambiguation in R API docs and surface naming.

5. Keep `category(name)` normalized legacy tokens temporarily
- WbW-Py decision: retained temporary compatibility shim.
- WbW-R parity: `Python-only`.
- Rationale: this is an implementation detail for transition convenience and not part of preferred user-facing style.
- Proposed WbW-R action: none required unless an equivalent transition shim is needed.

6. Add interoperability behavior matrix and copy-boundary guidance to user-facing docs
- WbW-Py decision: added README matrix plus internal detailed matrix source.
- WbW-R parity: `parallel now`.
- Rationale: shared conceptual documentation structure will reduce cross-language confusion and support a unified manual-generation workflow.
- Proposed WbW-R action: add an R-facing interoperability matrix with equivalent bridge categories and preservation/drift semantics.
