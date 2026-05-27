# API Reference Strategy

Manual chapters provide narrative and recipes, and now include a dedicated API
reference section near the end of the manual.

The split is intentional:

- conceptual chapters answer workflow questions (what to do and why)
- API chapters answer contract questions (exact parameters and return shape)

To reduce drift, API chapters source content from shared generated references.

## Reference Boundaries

- This manual explains concepts, patterns, and end-to-end examples.
- This manual also includes an API section for non-tool `WbEnvironment` methods
	and tool wrappers.
- Source-of-truth references remain in `TOOLS.md`, `docs/tools_*.md`, and
	package typing assets.

## Navigation

- Discovery and execution patterns: [Environment and Discovery](./environment-and-discovery.md)
- Data-object workflows: [Working with Rasters](./working-with-rasters.md), [Working with Vectors](./working-with-vectors.md), [Working with Lidar](./working-with-lidar.md)
- Concrete runnable scripts: [Script Index](./script-index.md)
- API-first lookup: [API Reference](./api-reference.md)
