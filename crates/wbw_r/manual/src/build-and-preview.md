# Build and Preview

This manual is structured as an mdBook project.

Documentation build steps are part of quality control. Running these commands
early and often helps catch broken links, malformed markdown, and chapter order
drift before those issues accumulate. Live preview is especially useful when
iterating on explanatory text and section hierarchy.

## Install mdBook

```bash
cargo install mdbook
```

## Build the Manual

Run this when changing chapter content, links, or section ordering.

From repository root:

```bash
cd crates/wbw_r/manual
mdbook build
```

Generated HTML is written to `book/`.

## Live Preview

Use live preview while refining long explanatory sections and chapter flow.

```bash
cd crates/wbw_r/manual
mdbook serve --open
```

## Authoring Notes

- Keep chapter order aligned with `src/SUMMARY.md`.
- Prefer runnable snippets over pseudo-code.
- Link chapter recipes to concrete scripts listed in the script index.