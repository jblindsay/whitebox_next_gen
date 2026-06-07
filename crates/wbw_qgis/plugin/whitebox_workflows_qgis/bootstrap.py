"""Bootstrap whitebox_workflows backend for QGIS plugin.

Simplified approach:
- Only works with QGIS bundled Python
- User installs WbW-Py via: pip install whitebox-workflows (published) or
                             pip install /path/to/wheel (development)
- No mode switching or environment juggling
- Direct import from QGIS Python site-packages
"""

from __future__ import annotations

import importlib
import json
import os
import subprocess
from pathlib import Path


class RuntimeBootstrapError(RuntimeError):
    pass


def load_whitebox_workflows():
    """Import and return the whitebox_workflows module.
    
    Works only with QGIS bundled Python.  Assumes whitebox-workflows is installed
    in QGIS Python via: pip install whitebox-workflows (or local wheel).
    """
    try:
        wbw = importlib.import_module("whitebox_workflows")
        return wbw
    except ImportError as e:
        raise RuntimeBootstrapError(
            "BACKEND_NOT_INSTALLED: whitebox-workflows is not installed in QGIS Python. "
            "Install it using: Plugins → Whitebox Workflows → Plugin Settings → Install Backend"
        ) from e


def is_backend_not_installed_error(exc: Exception) -> bool:
    """Return True if exc is the specific 'backend not installed' sentinel."""
    return str(exc).startswith("BACKEND_NOT_INSTALLED:")


def get_loaded_backend_info() -> dict:
    """Return diagnostic info about the currently loaded whitebox_workflows.
    
    Returns a dict with:
    - version: version string (e.g., "2.0.3")
    - file_path: location of the whitebox_workflows package
    """
    try:
        wbw = importlib.import_module("whitebox_workflows")
        
        version = "unknown"
        try:
            import importlib.metadata
            version = importlib.metadata.version("whitebox_workflows")
        except Exception:
            if hasattr(wbw, "__version__"):
                version = str(wbw.__version__)
        
        file_path = "unknown"
        if hasattr(wbw, "__file__") and wbw.__file__:
            file_path = str(wbw.__file__)
        
        return {
            "version": version,
            "file_path": file_path,
        }
    except Exception as exc:
        return {
            "version": "error",
            "file_path": str(exc),
        }


def install_whitebox_workflows_via_pip(target_dir: str = "", use_wheel: str = "") -> None:
    """Install whitebox-workflows in QGIS Python.
    
    Args:
        target_dir: Optional --target directory for pip install. If empty, installs
                   to default QGIS Python site-packages.
        use_wheel: Optional path to local .whl file. If provided, installs from wheel
                  instead of PyPI published version.
    """
    import sys
    
    cmd = [sys.executable, "-m", "pip", "install", "--upgrade"]
    
    if target_dir:
        cmd.extend(["--target", target_dir])
    
    if use_wheel:
        # Install from local wheel file
        wheel_path = str(use_wheel).strip()
        if not Path(wheel_path).exists():
            raise RuntimeBootstrapError(f"Wheel file not found: {wheel_path}")
        cmd.append(wheel_path)
    else:
        # Install published version from PyPI
        cmd.append("whitebox-workflows")
    
    result = subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        check=False,
        env=dict(os.environ),  # Keep full environment for PYTHONHOME etc.
    )
    
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or "unknown error"
        raise RuntimeBootstrapError(
            f"pip install failed: {detail}"
        )


def get_install_script() -> str:
    """Return Python code to run in QGIS console to install whitebox-workflows.
    
    User runs this in Plugins → Python Console.
    """
    return '''# Copy-paste this into QGIS Python Console to install whitebox-workflows
# For published version (recommended for users):
import subprocess, sys
subprocess.run([sys.executable, "-m", "pip", "install", "--upgrade", "whitebox-workflows"])

# For development version from local wheel:
# subprocess.run([sys.executable, "-m", "pip", "install", "--upgrade", "/path/to/whitebox_workflows-2.x.x-*.whl"])

print("Installation complete. Refresh the Whitebox Workflows catalog: Plugins → Whitebox Workflows → Refresh Catalog")
'''
