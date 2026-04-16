# wbw_qgis

QGIS 4 plugin frontend for Whitebox Workflows, intended to provide a QGIS-first user experience over the Whitebox backend runtime and the `whitebox_workflows` Python package.

The plugin is currently in active development and is installed locally from source. It is not yet published to the QGIS plugin portal, and `whitebox_workflows` is not yet published to PyPI, so the current workflow is source-based development and local installation only.

## Table of Contents

- [Mission](#mission)
- [The Whitebox Project](#the-whitebox-project)
- [Is wbw_qgis Only for Whitebox?](#is-wbw_qgis-only-for-whitebox)
- [What wbw_qgis Is Not](#what-wbw_qgis-is-not)
- [Current Capabilities](#current-capabilities)
- [Licensing Model in the QGIS Plugin](#licensing-model-in-the-qgis-plugin)
- [Installation and Local Setup](#installation-and-local-setup)
- [QGIS Version Support](#qgis-version-support)
- [Local Installation Workflow](#local-installation-workflow)
- [Development Notes](#development-notes)
- [Discovery Refresh Flow](#discovery-refresh-flow)
- [Known Limitations](#known-limitations)
- [License](#license)

## Mission

- Provide a QGIS-native frontend for Whitebox Workflows.
- Expose Whitebox tools through a dynamic QGIS Processing provider and plugin surface.
- Keep the plugin thin: orchestration, presentation, help, and QGIS integration should live here, while core geospatial computation remains in the Whitebox backend.
- Support a development workflow where the QGIS plugin tracks the current local `wbw_python` build without requiring packaging or portal publication.

## The Whitebox Project

[Whitebox](https://www.whiteboxgeo.com) is a suite of open-source geospatial data analysis software with roots at the [University of Guelph](https://geg.uoguelph.ca), Canada, where [Dr. John Lindsay](https://jblindsay.github.io/ghrg/index.html) began the project in 2009. Over more than fifteen years it has grown into a widely used platform for geomorphometry, spatial hydrology, LiDAR processing, and remote sensing research. In 2021 Dr. Lindsay and Anthony Francioni founded [Whitebox Geospatial Inc.](https://www.whiteboxgeo.com) to ensure the project's long-term, sustainable development. **Whitebox Next Gen** is the current major iteration of that work, and this crate is part of that larger effort.

Whitebox Next Gen is a ground-up redesign that improves on its predecessor in nearly every dimension:

- **CRS & reprojection** — Full read/write of coordinate reference system metadata across raster, vector, and LiDAR data, with multiple resampling methods for raster reprojection.
- **Raster I/O** — More robust GeoTIFF handling (including Cloud-Optimized GeoTIFFs), plus newly supported formats such as GeoPackage Raster and JPEG2000.
- **Vector I/O** — Expanded from Esri Shapefile-only to 11 formats, including GeoPackage, FlatGeobuf, GeoParquet, and other modern interchange formats.
- **Vector topology** — A new, dedicated topology engine (`wbtopology`) enabling robust overlay, buffering, and related operations.
- **LiDAR I/O** — Full support for LAS 1.0–1.5, LAZ, COPC, E57, and PLY via `wblidar`, a high-performance, modern LiDAR I/O engine.
- **Frontends** — Whitebox Workflows for Python (WbW-Python), Whitebox Workflows for R (WbW-R), and this QGIS 4-compliant plugin are in active development.

## Is wbw_qgis Only for Whitebox?

Yes, in practice. Unlike lower-level crates such as `wbraster`, `wbvector`, `wblidar`, or `wbtopology`, `wbw_qgis` is explicitly a Whitebox frontend.

- **Whitebox-specific**: it assumes the `whitebox_workflows` Python package and the Whitebox runtime discovery model.
- **QGIS-specific**: it targets QGIS plugin and Processing provider integration rather than a general Python UI framework.
- **Frontend-focused**: its role is to surface Whitebox capabilities inside QGIS, not to provide a reusable GIS plugin toolkit.

## What wbw_qgis Is Not

`wbw_qgis` is a QGIS integration layer. It is **not**:

- Not the core computational backend for Whitebox tools.
- Not a replacement for `wbw_python`; it depends on `whitebox_workflows` being installed into the Python environment used by QGIS.
- Not a generic QGIS plugin scaffolding framework.
- Not yet a complete packaged distribution story; portal publication and one-click installation are planned later.

## Current Capabilities

The current plugin implementation covers the core shell needed for QGIS integration and catalog-driven discovery.

- Plugin package structure and metadata for QGIS 4.
- Runtime bootstrap helpers for `whitebox_workflows`.
- Runtime capability discovery.
- Tool catalog discovery with locked-tool visibility metadata.
- Processing provider shell for catalog-driven algorithm exposure.
- Dock/panel and action wiring for the current plugin surface.
- Local help/documentation support with bundled and generated help paths.

Planned next steps include:

- Generate Processing algorithms more completely from tool-catalog entries.
- Expand plugin settings and entitlement bootstrap UI.
- Connect streamed progress and messages into QGIS task and log surfaces.
- Improve semantic styling, report loading, and richer result presentation.

## Licensing Model in the QGIS Plugin

Licensing is enforced by the Whitebox backend runtime exposed through `whitebox_workflows`; the QGIS plugin is primarily a discovery, presentation, and orchestration layer over that runtime.

For most readers of this repository, the practical assumption should be:

- If you are building the plugin yourself from the public source tree, you should expect an **open-tier** Whitebox Workflows environment.
- You should not assume that compiling the plugin from source gives access to Pro-tier tools.
- The plugin should therefore be understandable and usable as an open-tier frontend first, with Pro-aware behavior layered on top for licensed distributions.

In practice, the interaction works like this:

1. The plugin creates a `whitebox_workflows` runtime session and requests a tiered capability view.
2. The backend reports runtime capabilities, including the effective tier that was actually granted.
3. The plugin reads the tool catalog and uses backend metadata to distinguish available tools from locked tools.
4. The plugin presents those states in the QGIS UI rather than trying to implement a separate licensing authority in the frontend.

Important implications:

- **The plugin does not define licensing rules itself**: it reflects what the backend runtime says is available.
- **Open and Pro visibility are catalog-driven**: tools can be shown as available or locked based on backend capability and manifest metadata.
- **Requested tier and effective tier may differ**: the panel surface already exposes both, which is useful when a Pro-capable build falls back to open-mode behavior.
- **Locked tools can still be shown**: the plugin can display locked tools for discoverability, with the UI and help system marking that a Pro license is required.
- **Execution authority remains in the backend**: the reliable source of truth is the runtime session, not the QGIS UI state.

The current bootstrap path can request a Pro-capable runtime session, but it can also downgrade to open behavior if Pro support is unavailable in the active environment. This is intentional: the plugin should still load and provide open-tier workflows even when a Pro entitlement is absent or the local environment only has an open-only build. For external builders, this open-tier outcome should be treated as the normal expected case.

### Relationship between open-core and Pro-tier tools

The intended public release model is an **open-core plugin surface with Pro-aware discovery**.

That means:

- The public plugin should be installable and useful with the open Whitebox runtime alone.
- Open-core tools should appear and run normally with no Pro entitlement.
- Pro-tier tools may still be represented in the catalog for discoverability, but they should appear as locked or unavailable unless the backend runtime grants access.
- The plugin should not fork into separate open and Pro frontends; instead, one plugin should adapt its visible and executable tool surface to the effective runtime tier.

This is important both commercially and architecturally:

- It keeps the public plugin genuinely useful as an open geospatial frontend.
- It makes the upgrade path to Pro additive rather than requiring a different plugin.
- It keeps licensing logic centralized in the backend rather than split across multiple UI distributions.

As the licensing surfaces mature, the plugin is expected to remain thin and defer startup behavior to the same runtime patterns documented in `wbw_python`, including:

- Open mode.
- Signed entitlement mode.
- Floating license mode.

That keeps licensing behavior consistent across Python scripts, notebook workflows, and the QGIS plugin.

## Installation and Local Setup

This plugin currently supports **local installation from source only**.

There are two pieces to install:

1. `whitebox_workflows` must be installed into the same Python environment used by QGIS.
2. The plugin package directory `whitebox_workflows_qgis` must be present in the active QGIS profile's `python/plugins` directory.

The plugin metadata currently targets QGIS 4 (`qgisMinimumVersion=4.0`).

## QGIS Version Support

This plugin targets **QGIS 4.x**.

QGIS 3.x support is not planned.

The reasoning is practical rather than ideological:

- The plugin is being built for the current forward-looking Whitebox Next Gen frontend surface.
- By the expected release window, QGIS 3.x will be close to or past the point where it remains the long-term-maintenance target that matters for a new plugin launch.
- Supporting both QGIS 3.x and 4.x would increase testing, maintenance, and compatibility burden for a frontend that is still evolving quickly.
- For a small team, the correct tradeoff is to concentrate effort on one host-generation target and keep the plugin architecture clean.

Until public release, local development and testing should assume QGIS 4.x only.

## Local Installation Workflow

### 1. Install `whitebox_workflows` into the QGIS Python environment

From the workspace root:

```bash
./scripts/dev_python_install.sh
```

This is the normal path for public/open local development.

To build the Python package with Pro support compiled in:

```bash
./scripts/dev_python_install.sh --pro
```

This script runs `maturin develop --release` for the `wbw_python` crate and installs `whitebox_workflows` into the **current** Python environment. The important requirement is that this must be the same Python environment QGIS uses.

For most external builders, the `--pro` path should be considered non-default and possibly unavailable in practice, because access to Pro-tier tooling depends on private/commercial distribution inputs, not merely on compiling the plugin frontend.

If QGIS is using a dedicated Python environment, activate that environment first and then run the install script.

### 2. Install or symlink the QGIS plugin package into the QGIS profile

The plugin package to install is:

```text
crates/wbw_qgis/plugin/whitebox_workflows_qgis
```

The target is your active QGIS profile plugin directory:

```text
<QGIS settings dir>/python/plugins/whitebox_workflows_qgis
```

For local development, a symlink is preferred so edits in the repo are reflected immediately.

Typical pattern from the workspace root:

```bash
export QGIS_PLUGIN_DIR="<QGIS settings dir>/python/plugins"
mkdir -p "$QGIS_PLUGIN_DIR"
ln -snf "$PWD/crates/wbw_qgis/plugin/whitebox_workflows_qgis" \
	"$QGIS_PLUGIN_DIR/whitebox_workflows_qgis"
```

If symlinks are inconvenient on your platform, copy the directory instead.

Typical macOS example for a default local profile (adjust if your profile or QGIS version path differs):

```bash
export QGIS_PLUGIN_DIR="$HOME/Library/Application Support/QGIS/QGIS4/profiles/default/python/plugins"
mkdir -p "$QGIS_PLUGIN_DIR"
ln -snf "$PWD/crates/wbw_qgis/plugin/whitebox_workflows_qgis" \
	"$QGIS_PLUGIN_DIR/whitebox_workflows_qgis"
```

### 3. Start QGIS and enable the plugin

After the Python package and plugin directory are in place:

1. Start QGIS.
2. Open the Plugin Manager.
3. Enable **Whitebox Workflows**.
4. Confirm the Processing provider and plugin actions appear.
5. Trigger a catalog refresh if needed.

### 4. Verify the runtime import path

If the plugin loads but cannot discover tools, the first thing to verify is that QGIS can import `whitebox_workflows` from its active Python environment.

Minimal smoke test from the same environment used by QGIS:

```bash
python3 -c "import whitebox_workflows as wb; print(wb.__file__)"
```

## Development Notes

- The plugin is intentionally a thin orchestration and presentation layer over `wbw_python`.
- Local source installation is the expected development mode until portal and PyPI publication are in place.
- When `whitebox_workflows` changes, rerun `./scripts/dev_python_install.sh` in the Python environment QGIS actually uses.
- When plugin Python files change, a QGIS restart or plugin reload may be needed depending on the host state.
- The plugin package name on disk is `whitebox_workflows_qgis`, while the user-facing plugin name is **Whitebox Workflows**.

## Discovery Refresh Flow

The current plugin scaffold assumes this refresh sequence:

1. Import `whitebox_workflows` from the active QGIS Python environment.
2. Construct a runtime session with `include_pro=True` and the current tier.
3. Read runtime capability JSON for build and tier visibility state.
4. Read tool-catalog JSON for the complete manifest set, including locked tools.
5. Sort and partition the catalog into available and locked groups.
6. Rebuild Processing algorithms from the latest catalog snapshot.

This flow is intended to run:

- On plugin load.
- After a settings or entitlement change.
- After an explicit user-triggered refresh.
- After a detected `whitebox_workflows` upgrade in the QGIS environment.

## Known Limitations

- The plugin is not yet distributed through the QGIS plugin portal.
- `whitebox_workflows` is not yet distributed through PyPI, so local source installation is required.
- Public/source builds should be expected to operate in open-tier mode unless paired with a licensed Pro-capable backend runtime.
- The plugin targets QGIS 4.x only; QGIS 3.x compatibility is not a project goal.
- The plugin surface is still evolving and some workflows remain scaffold-level or partially implemented.
- Setup currently depends on aligning the QGIS Python environment with the environment used for `maturin develop`.
- The documented local-install steps are development-oriented rather than end-user one-click installation.

## License

This crate is part of the Whitebox Next Gen workspace. See the repository root for current licensing terms.