# Build and Preview

This manual is structured as an mdBook project.

## Install mdBook

```bash
cargo install mdbook
```

## Build the Manual

From repository root:

```bash
cd crates/wbw_python/manual
mdbook build
```

Generated HTML is written to `book/`.

## Live Preview

```bash
cd crates/wbw_python/manual
mdbook serve --open
```

## Authoring Notes

- Keep chapter order aligned with `src/SUMMARY.md`.
- Prefer runnable snippets over pseudo-code.
- Link chapter recipes to concrete scripts listed in the script index.