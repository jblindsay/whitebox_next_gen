# wbtools_pro shim (public OSS)

This crate is a public compatibility shim that preserves the `wbtools_pro` API surface required by bindings.

## Design contract

- Keep this crate small.
- Do not implement Pro tool logic here.
- Expose only the minimal registry API expected by consumers:
  - `ToolRegistry`
  - `register_default_tools`
- In public OSS builds, `register_default_tools` is intentionally a no-op.
- Real Pro implementations are injected only in private CI overlay workflows.

## Why this exists

Public builds must compile from a clean clone without any private sibling repository.
This shim allows `wbw_python` and `wbw_r` to compile in both default and `pro` feature modes while keeping private code out of the public manifest graph.
