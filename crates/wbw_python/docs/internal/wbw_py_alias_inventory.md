# WbW-Py Alias Inventory (Phase 1)

This inventory tracks canonical API names, removed aliases, and temporary retained aliases during pre-release cleanup.

## Canonical (Preferred)

- Raster metadata: `Raster.metadata()`
- Vector attribute reads: `Vector.schema()`, `Vector.attributes()`, `Vector.attribute()`
- Vector attribute writes: `Vector.update_attributes()`, `Vector.update_attribute()`, `Vector.add_field()`
- Category accessors: `wbe.raster`, `wbe.vector`, `wbe.lidar`, `wbe.remote_sensing`
- Utility namespaces: `wbe.projection.*`, `wbe.topology.*`
- Topology tool category: `wbe.topology_tools`

## Removed in Phase 1 (Pre-release)

- `Raster.configs()` -> use `Raster.metadata()`
- `Vector.get_attributes()` -> use `Vector.attributes()`
- `Vector.get_attribute()` -> use `Vector.attribute()`
- `Vector.set_attributes()` -> use `Vector.update_attributes()`
- `Vector.set_attribute()` -> use `Vector.update_attribute()`
- `Vector.add_attribute_field()` -> use `Vector.add_field()`
- `wbe.raster_tools` -> use `wbe.raster`
- `wbe.vector_tools` -> use `wbe.vector`
- `wbe.lidar_tools` -> use `wbe.lidar`
- `wbe.remote_sensing_tools` -> use `wbe.remote_sensing`

## Temporary Retained Aliases (Intentional)

These remain for now to reduce in-flight churn while docs/examples are being standardized:

- `WbEnvironment.category(name)` accepts normalized legacy tokens:
  - `raster_tools`, `vector_tools`, `lidar_tools`
  - `remote_sensing_tools`, `remotesensing`
  - `stream`, `stream_network`

## Follow-up

- Remove normalized legacy tokens from `category(name)` once docs/examples and downstream tests no longer reference them.
- Re-evaluate retention after Milestone B completion.
