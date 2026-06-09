# Installation and Setup

This chapter covers installing the Whitebox Workflows QGIS plugin and its
Python backend.

## System Requirements

- **QGIS 4.0 or later** (3.28+ may work but is not officially supported)
- **Internet connection** (required for backend installation via pip)
- **macOS, Linux, or Windows**

## Install the QGIS Plugin

### From QGIS Plugin Repository (Recommended)

1. Open QGIS
2. Go to **Plugins → Manage and Install Plugins**
3. Search for "Whitebox Workflows"
4. Click **Install Plugin**
5. Restart QGIS

### From File (Manual Installation)

If you have a plugin `.zip` file:

1. Download or obtain the `whitebox_workflows_for_qgis-*.zip` file
2. Extract the zip to your QGIS plugins directory:
   - **macOS**: `~/Library/Application Support/QGIS/QGIS4/profiles/default/python/plugins/`
   - **Linux**: `~/.local/share/QGIS/QGIS4/profiles/default/python/plugins/`
   - **Windows**: `%APPDATA%\QGIS\QGIS4\profiles\default\python\plugins\`
3. Ensure the directory structure is `plugins/whitebox_workflows_qgis/` with 
   `__init__.py` and `metadata.txt` directly inside
4. Restart QGIS

## Install the Whitebox Workflows Backend

After installing the plugin, restart QGIS. The plugin will check for the
`whitebox-workflows` Python package.

### Option A: Install via Plugin Dialog (Easiest)

1. Open QGIS
2. If the backend is not installed, a dialog appears: 
   **"⚠️ Action Required — Install Whitebox Workflows Backend"**
3. Read the installation instructions (or copy the command to clipboard)
```python
exec("import runpy,sys\ntry:\n    import pip\nexcept ImportError:\n    import ensurepip\n    ensurepip.bootstrap(upgrade=True)\nsys.argv=['pip','install','--user','whitebox-workflows']\ntry:\n    runpy.run_module('pip',run_name='__main__',alter_sys=True)\nexcept SystemExit:\n    pass\nimport qgis.utils\nqgis.utils.unloadPlugin('whitebox_workflows_qgis')\nqgis.utils.loadPlugin('whitebox_workflows_qgis')\nqgis.utils.startPlugin('whitebox_workflows_qgis')\nprint('Whitebox Workflows backend installed and plugin reloaded.')")
```
4. Click **Install** to automatically download and install the backend using 
   QGIS's bundled Python and pip
5. Wait for the installation to complete
6. The plugin automatically reloads with full access to all tools

### Option B: Install via Command Line

If you prefer manual installation, run:

```bash
pip install whitebox-workflows
```

Or, if you have the QGIS Python environment:

```bash
# macOS/Linux (find the QGIS Python executable)
$(which python3) -m pip install whitebox-workflows

# Or explicitly use QGIS's Python if installed locally:
/Applications/QGIS.app/Contents/MacOS/Python/bin/python3 -m pip install whitebox-workflows
```

After installation, restart QGIS.

### Verify Installation

Once the backend is installed, you should see:
- The Processing Toolbox populates with 700+ Whitebox tools
- No error messages in the QGIS message log
- Whitebox tools appear in **Processing → Toolbox → Whitebox Workflows**

## For Development and Local Testing

If you are a developer working with the source repository:

1. Checkout the `whitebox_next_gen` repository
2. Install the plugin locally by symlinking it into your QGIS plugins folder:
   ```bash
   export QGIS_PLUGIN_DIR="<QGIS settings dir>/python/plugins"
   mkdir -p "$QGIS_PLUGIN_DIR"
   ln -snf "$PWD/crates/wbw_qgis/plugin/whitebox_workflows_qgis" \
     "$QGIS_PLUGIN_DIR/whitebox_workflows_qgis"
   ```
3. Install the backend using the automated installer or `pip install whitebox-workflows`
4. Changes to the plugin source are reflected immediately on QGIS restart

## Troubleshooting

### "Cannot find __init__.py or metadata.txt"

This error means the plugin zip was extracted incorrectly. Ensure your plugins 
directory contains:
```
plugins/
  whitebox_workflows_qgis/
    __init__.py
    metadata.txt
    bootstrap.py
    plugin.py
    ... (other files)
```

Not:
```
plugins/
  whitebox_workflows_qgis/
    whitebox_workflows_qgis/     ← Extra nesting
      __init__.py
```

### Backend Installation Fails

#### `ImportError: No module named pip` (Windows / OSGeo4W)

This is a known issue on Windows when QGIS is installed via the **OSGeo4W
installer**. Unlike the standalone QGIS installer, OSGeo4W does not include
`pip` in its bundled Python by default. The symptom is an error like:

```
ImportError: No module named pip
```

or:

```
C:\OSGeo4W\bin\python.exe: No module named pip
```

**Fix:** Bootstrap `pip` into the OSGeo4W Python environment first, then
re-run the backend installation.

1. Open the **OSGeo4W Shell** (search for it in the Windows Start menu)
2. Run the following commands to install pip:
   ```bat
   python -m ensurepip
   python -m pip install --upgrade pip
   ```
   If `ensurepip` is unavailable, download and run the pip bootstrap script
   directly:
   ```bat
   curl https://bootstrap.pypa.io/get-pip.py -o get-pip.py
   python get-pip.py
   ```
   Alternatively, follow the official OSGeo4W instructions for external
   Python packages at: https://trac.osgeo.org/osgeo4w/wiki/ExternalPythonPackages

3. Once pip is installed, run the Whitebox backend installation from the
   OSGeo4W Shell:
   ```bat
   python -m pip install whitebox-workflows
   ```
4. Restart QGIS and the tools should populate normally.

> **Note:** The standalone QGIS installer from qgis.org includes pip by default
> and does not require this step. If you encounter this issue frequently,
> consider switching to the standalone installer.

#### Other Backend Installation Issues

- **"Permission denied"** — Try `pip install --user whitebox-workflows`
- **Network issues** — Check your internet connection and proxy settings
- **For help** — See [Troubleshooting](troubleshooting.md) or contact 
  support@whiteboxgeo.com
