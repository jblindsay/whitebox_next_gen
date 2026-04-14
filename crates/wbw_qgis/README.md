# WbW-QGIS

Initial QGIS plugin scaffold for Whitebox Workflows.

This package is intentionally minimal in the first implementation slice. It
establishes the package layout, plugin metadata, runtime bootstrap helpers, and
tool-catalog discovery flow that will later drive dynamic Processing algorithm
registration.

Current scope:

- plugin package structure for QGIS
- WbW-Py bootstrap and session construction helpers
- runtime capability discovery
- tool catalog discovery with locked-tool visibility metadata
- Processing provider shell for future algorithm registration

Planned next steps:

- generate Processing algorithms from tool-catalog entries
- wire plugin settings and entitlement bootstrap UI
- connect streamed progress/message events into QGIS task and log surfaces
- add semantic styling and report-loading helpers

## Discovery refresh flow

The initial plugin scaffold assumes this refresh sequence:

1. Import `whitebox_workflows` from the active QGIS Python environment.
2. Construct a runtime session with `include_pro=True` and the current tier.
3. Read runtime capability JSON for build/tier visibility state.
4. Read tool-catalog JSON for the complete manifest set, including locked tools.
5. Sort and partition the catalog into available and locked groups.
6. Rebuild Processing algorithms from the latest catalog snapshot.

This flow is meant to run:

- on plugin load
- after a settings or entitlement change
- after an explicit user-triggered refresh
- after a detected WbW-Py upgrade in the QGIS environment