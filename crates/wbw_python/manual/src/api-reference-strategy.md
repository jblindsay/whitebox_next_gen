# API Reference Strategy

Manual chapters provide narrative and recipes.

This split between narrative manual content and detailed tool references is
intentional. Conceptual chapters answer workflow questions (what to do and why),
while reference docs answer contract questions (exact parameters and return
shape). Keeping these concerns separate improves readability without sacrificing
precision.

Tool-by-tool parameter references remain in:
- `TOOLS.md`
- `docs/tools_*.md`

## Reference Boundaries

- This manual explains concepts, patterns, and end-to-end examples.
- Tool parameter completeness lives in `TOOLS.md` and category tool docs.
- Type signatures and IntelliSense details are reflected in package typing assets.

## Navigation

- Discovery and execution patterns: [Environment and Discovery](./environment-and-discovery.md)
- Data-object workflows: [Working with Rasters](./working-with-rasters.md), [Working with Vectors](./working-with-vectors.md), [Working with Lidar](./working-with-lidar.md)
- Concrete runnable scripts: [Script Index](./script-index.md)
