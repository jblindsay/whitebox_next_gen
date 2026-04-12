# Installation and Setup

Installation is intentionally separated from workflow chapters so failures are
detected early and diagnosed in isolation. The smoke tests here are not just
"does import work" checks; they confirm that bindings, core runtime components,
and representative interop pathways are all healthy on your machine.

For team environments, treat this chapter as a baseline validation checklist
before onboarding scripts or CI jobs.

## Development Install

Use this path for local development or source-based testing. It installs the
package and links the Python layer with the current workspace backend.

```bash
./scripts/dev_python_install.sh
```

## Pro Build

Use this path when testing environments that include optional pro-enabled
capabilities.

```bash
./scripts/dev_python_install.sh --pro
```

## Smoke Tests

Run both smoke tests before starting deeper debugging. The first validates
import and startup; the second checks an interop roundtrip so I/O boundaries are
confirmed.

```bash
python3 crates/wbw_python/examples/python_import_smoke_test.py
python3 crates/wbw_python/examples/interop_roundtrip_smoke_test.py
```
