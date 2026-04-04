# wbtools_oss

**THIS CRATE IS CURRENTLY EXPERIMENTAL AND IS IN AN EARLY DEVELOPMENTAL STAGE. IT IS NOT INTENDED FOR PUBLIC USAGE AT PRESENT.**

Open-source tool pack for the Whitebox backend runtime.

## Purpose

`wbtools_oss` hosts tools that are available under the Open license tier and are always safe to include in default language bindings.

## Current contents

- Registry implementation:
  - `ToolRegistry`
  - `register_default_tools`
- Sample open tools:
  - `add`, `subtract`, `multiply`, `divide` (legacy-style raster math port examples)

These sample tools include explicit manifests with defaults, examples, tags, and stability metadata.

## Legacy-style migration example

The raster binary math tools in `src/tools/raster/raster_add.rs` are direct
style-preserving ports of older Whitebox Workflows math operations to the new
plugin runtime model:

- Accepts two raster paths (`input1`, `input2`) and optional `output` path.
- Uses `wbraster` for I/O and per-cell arithmetic.
- Emits runtime progress/messages through `ctx.progress.info(...)` and
  `ctx.progress.progress(...)`.
- Returns a typed output envelope (`__wbw_type__ = "raster"`) so
  Whitebox Workflows for Python typed APIs can materialize a `Raster` object
  directly.

When `output` is omitted, these tools store the result in an in-memory raster
store and return a `memory://raster/<id>` handle. This allows tool chaining
with reduced disk I/O (output object from one tool can be passed directly to
the next tool).

## Integration role

- Implements `ToolRuntimeRegistry` from `wbcore`.
- Can be used directly in a runtime, or composed with `wbtools_pro` in a higher-level composite registry.
- Powers default open-mode behavior in Whitebox Workflows for Python and Whitebox Workflows for R.

## Usage

From this crate directly:

```bash
cargo run -p wbtools_oss --example run_tool -- list
cargo run -p wbtools_oss --example run_tool -- run add '{"input1":"a.tif","input2":"b.tif","output":"sum.tif"}'
```

## Testing

Run:

```bash
cargo test -p wbtools_oss
```

This validates tool execution, registry behavior, and manifest richness.
