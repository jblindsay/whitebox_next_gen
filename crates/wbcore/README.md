# wbcore

Core runtime contracts and orchestration primitives for the Whitebox backend workspace.

## Purpose

`wbcore` defines the shared, language-agnostic tool execution model used by all tool packs and language bindings. It is the integration backbone that keeps `wbtools_oss`, `wbtools_pro`, Whitebox Workflows for Python, and Whitebox Workflows for R aligned.

## What this crate contains

- Core tool interfaces:
  - `Tool`
  - `ToolMetadata`
  - `ToolManifest`
  - `ToolArgs`
  - `ToolRunResult`
  - `ToolError`
- Runtime execution model:
  - `ExecuteRequest`
  - `ExecuteResponse`
  - `ToolRuntime`
  - `ToolRuntimeRegistry`
- Runtime composition and policy:
  - `RuntimeOptions`
  - `OwnedToolRuntime`
  - `ToolRuntimeBuilder`
  - `CapabilityProvider`
  - `MaxTierCapabilities`
- Progress and observability primitives:
  - `ProgressSink`
  - `RecordingProgressSink`
  - `ProgressEvent`
- Metadata-driven wrapper support:
  - `BindingTarget`
  - `generate_wrapper_stub`

## Design goals

- Keep execution contracts stable across crates and language bindings.
- Keep policy decisions (license tiers and visibility) centralized.
- Avoid duplicate orchestration logic in language-specific crates.
- Make tool metadata rich enough to generate wrappers and docs.

## Typical usage pattern

1. Implement tools in a tool-pack crate (`wbtools_oss` or `wbtools_pro`) using the `Tool` trait.
2. Implement `ToolRuntimeRegistry` in that crate's registry.
3. Build an owned runtime with `ToolRuntimeBuilder`.
4. Expose `list_visible_manifests` and `execute` from bindings.

## Testing

Run:

```bash
cargo test -p wbcore
```

This validates runtime execution behavior, manifest handling, capability filtering, and wrapper stub generation.
