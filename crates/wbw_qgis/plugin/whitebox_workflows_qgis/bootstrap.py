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


def backend_install_status() -> dict:
    """Check if whitebox-workflows is installed.
    
    Returns dict with:
    - installed: bool
    - version: str or None
    - error: str or None
    """
    try:
        info = get_loaded_backend_info()
        version = str(info.get("version", "unknown")).strip()
        if version and version not in ("unknown", "error"):
            return {
                "installed": True,
                "version": version,
                "error": None,
            }
    except Exception:
        pass
    
    return {
        "installed": False,
        "version": None,
        "error": "whitebox-workflows is not installed",
    }


def backend_update_status() -> dict:
    """Check for available updates to whitebox-workflows.
    
    Returns dict with:
    - update_available: bool
    - installed_version: str
    - latest_version: str
    - interpreter: str
    - error: str or None
    """
    try:
        installed_info = backend_install_status()
        if not installed_info.get("installed"):
            return {
                "update_available": False,
                "installed_version": None,
                "latest_version": None,
                "interpreter": None,
                "error": "Backend not installed",
            }
        
        installed_version = str(installed_info.get("version", "unknown"))
        
        # Try to get latest version from PyPI
        import sys
        result = subprocess.run(
            [sys.executable, "-m", "pip", "index", "versions", "whitebox-workflows"],
            capture_output=True,
            text=True,
            check=False,
            timeout=5,
        )
        
        latest_version = "unknown"
        if result.returncode == 0:
            # Parse output to find latest version
            for line in result.stdout.split('\n'):
                if 'Available versions:' in line:
                    # Next line contains versions
                    parts = line.split(':')
                    if len(parts) > 1:
                        versions = parts[1].strip().split(',')
                        if versions:
                            latest_version = versions[0].strip()
                    break
        
        if latest_version == "unknown":
            # Fallback: try pip-audit or similar
            latest_version = installed_version
        
        # Compare versions (simplified string comparison)
        update_available = latest_version > installed_version if latest_version != "unknown" else False
        
        return {
            "update_available": update_available,
            "installed_version": installed_version,
            "latest_version": latest_version,
            "interpreter": str(subprocess.sys.executable),
            "error": None,
        }
    except subprocess.TimeoutExpired:
        return {
            "update_available": False,
            "installed_version": "unknown",
            "latest_version": "unknown",
            "interpreter": str(subprocess.sys.executable),
            "error": "Update check timed out",
        }
    except Exception as exc:
        return {
            "update_available": False,
            "installed_version": "unknown",
            "latest_version": "unknown",
            "interpreter": "unknown",
            "error": str(exc),
        }


def install_or_upgrade_whitebox_workflows(use_wheel: str = "") -> dict:
    """Install or upgrade whitebox-workflows in QGIS Python.
    
    Args:
        use_wheel: Optional path to local .whl file.
    
    Returns dict with:
    - success: bool
    - message: str
    - installed_version: str or None
    - error: str or None
    """
    try:
        install_whitebox_workflows_via_pip(use_wheel=use_wheel)
        info = get_loaded_backend_info()
        version = str(info.get("version", "unknown"))
        return {
            "success": True,
            "message": f"Installation complete. Installed version: {version}",
            "installed_version": version,
            "error": None,
        }
    except Exception as exc:
        return {
            "success": False,
            "message": f"Installation failed: {exc}",
            "installed_version": None,
            "error": str(exc),
        }
