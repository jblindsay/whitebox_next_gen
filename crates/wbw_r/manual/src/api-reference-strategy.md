# API Reference Strategy

Manual chapters provide narrative and recipes, and now include a dedicated API
reference section near the end of the manual.

This separation remains intentional:

- workflow chapters focus on design and usage patterns
- API chapters focus on argument contracts and callable surfaces

Detailed tool-level contracts are sourced from shared generated tool docs to
minimize drift.

## Reference Boundaries

- This manual focuses on concepts, workflow patterns, and runnable examples.
- This manual now also includes API chapters for non-tool facade/session APIs
	and tool wrappers.
- Source-of-truth tool contracts remain in shared generated tool docs.

## Navigation

- Discovery and execution patterns: [Session and Discovery](./session-and-discovery.md)
- Data-object workflows: [Working with Rasters](./working-with-rasters.md), [Working with Vectors](./working-with-vectors.md), [Working with Lidar](./working-with-lidar.md)
- Concrete runnable scripts: [Script Index](./script-index.md)
- API-first lookup: [API Reference](./api-reference.md)
