#!/usr/bin/env python3
"""
Reset Whitebox Workflows QGIS plugin settings.

This script clears backend detection cache and other settings so they can be 
re-detected on next plugin load. Run this before testing backend auto-detection.
"""

import sys

try:
    from qgis.core import QSettings
except ImportError:
    print("Error: QGIS not found. Run this script from QGIS Python console or activate QGIS environment.")
    sys.exit(1)


def reset_plugin_settings():
    """Reset Whitebox Workflows plugin settings."""
    settings = QSettings()
    
    # Backend-related keys to reset (forces re-detection)
    backend_keys = [
        "whitebox_workflows/backend_wbw_path",
        "whitebox_workflows/backend_wbw_version",
        "whitebox_workflows/backend_wbw_override_path",
        "whitebox_workflows/installation_strategy",
    ]
    
    print("Resetting Whitebox Workflows plugin settings...")
    for key in backend_keys:
        if settings.contains(key):
            settings.remove(key)
            print(f"  ✓ Cleared: {key}")
        else:
            print(f"  - Not found: {key}")
    
    settings.sync()
    print("\nPlugin settings reset. Close and reopen QGIS to test backend auto-detection.")


if __name__ == "__main__":
    reset_plugin_settings()
