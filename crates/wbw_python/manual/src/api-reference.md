# API Reference

This section provides API-first documentation for:

- Tool wrappers (every available tool by theme).
- Non-tool methods on `WbEnvironment`.

The goal is to make API discovery reliable without depending on autocomplete.

## Sections

- [Non-Tool `WbEnvironment` API](./api-non-tool-wbenvironment.md)
- [Tool API Reference (All Tools)](./api-tools-reference.md)

## Source of Truth

- Typed API surface: `whitebox_workflows/whitebox_workflows.pyi`
- Shared tool docs: `docs/tools_*.md`
- Broad API reference: `TOOLS.md`

When updating wrapper signatures or runtime contracts, regenerate/update those
sources first, then refresh these manual chapters.
