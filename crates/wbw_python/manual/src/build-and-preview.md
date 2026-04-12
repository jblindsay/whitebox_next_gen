# Build and Preview

This manual is structured as an mdBook project.

Treat documentation builds as part of normal engineering hygiene. A clean build
confirms that chapter links, code fences, and chapter ordering remain coherent
as examples evolve. Live preview is especially useful when refining conceptual
text, because it helps ensure headings and narrative pacing stay readable.

## Install mdBook

```bash
cargo install mdbook
```

## Build the Manual

Run a full build when editing chapter structure, links, or long conceptual
sections.

From repository root:

```bash
cd crates/wbw_python/manual
mdbook build
```

Generated HTML is written to `book/`.

## Live Preview

Use live preview while refining wording and section flow in narrative-heavy
chapters.

```bash
cd crates/wbw_python/manual
mdbook serve --open
```

## Authoring Notes

- Keep chapter order aligned with `src/SUMMARY.md`.
- Prefer runnable snippets over pseudo-code.
- Link chapter recipes to concrete scripts listed in the script index.