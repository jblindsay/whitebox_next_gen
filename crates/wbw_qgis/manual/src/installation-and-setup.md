# Installation and Setup

This chapter covers local installation of WbW-QGIS from source.

## Prerequisites

- QGIS 4.x
- A Python environment used by QGIS
- Local checkout of whitebox_next_gen

## Install the Whitebox Python Runtime

From the repository root, install whitebox_workflows into the same Python
environment used by QGIS:

```bash
./scripts/dev_python_install.sh
```

If you are working in an environment that supports Pro-enabled integration
builds, use:

```bash
./scripts/dev_python_install.sh --pro
```

## Install or Symlink the Plugin

Plugin source directory:

- crates/wbw_qgis/plugin/whitebox_workflows_qgis

Target QGIS plugins directory:

- <QGIS settings dir>/python/plugins/whitebox_workflows_qgis

Typical local workflow from repository root:

```bash
export QGIS_PLUGIN_DIR="<QGIS settings dir>/python/plugins"
mkdir -p "$QGIS_PLUGIN_DIR"
ln -snf "$PWD/crates/wbw_qgis/plugin/whitebox_workflows_qgis" \
  "$QGIS_PLUGIN_DIR/whitebox_workflows_qgis"
```

## Verify Python Import Path

Before opening QGIS, verify whitebox_workflows import in the same Python
environment QGIS uses:

```bash
python3 -c "import whitebox_workflows as wb; print(wb.__file__)"
```

If this fails, fix environment alignment before continuing.
