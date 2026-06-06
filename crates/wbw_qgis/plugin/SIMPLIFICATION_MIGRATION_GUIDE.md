# QGIS Plugin Simplification – Migration Guide

## Summary

The plugin has been redesigned around a **single, clear rule**:

```
If user set override path in settings → use that
Otherwise → use QGIS bundled Python
Cache the version, reload from cache during normal operation
Only call subprocess during install/refresh (manual actions)
```

## Files Created

### 1. `bootstrap_v2.py` (NEW)

**Functions exported:**
- `get_qgis_bundled_python()` → Get QGIS Python exe
- `get_wbw_python_to_use(override_path)` → Get Python to use (priority: override → QGIS)
- `check_whitebox_workflows_installed(interpreter)` → Check if WbW-Py is in this Python
- `get_whitebox_workflows_version(interpreter)` → Get version (subprocess call—only on demand)
- `refresh_backend_status(override_path)` → Refresh cache (subprocess call—only manual action)
- `get_backend_python_for_operations(override_path, cached_version)` → Get Python + version for tool ops (no subprocess—uses cache)
- `install_whitebox_workflows(override_path, qgis_python)` → Install with fallback strategy

**Key design:**
- `refresh_backend_status()` is the ONLY function that queries versions
- `get_backend_python_for_operations()` uses CACHED values (no subprocess)
- Installation tries QGIS first, then system Python, then fails gracefully

### 2. `settings_v2.py` (NEW)

**Settings class now has only:**
- `python_override_path` – User-provided path (empty = use QGIS Python)
- `python_version_cached` – Read-only display of last-known version

**Removed:**
- `runtime_mode` (auto/local/qgis)
- `local_python_path`
- `backend_wbw_path` (detected, not stored)
- `backend_wbw_version` (only cached version stored)
- `installation_strategy` (not needed)
- All complex decision logic

**UI shows:**
- Current Python: "[path]" or "(Using QGIS bundled Python)"
- WbW-Py Version: "[version]" or "(version unknown)"
- Input field: "Override Python Path" with tooltip
- Button: "Refresh Backend Status" (triggers manual refresh)

### 3. What changes in `plugin.py`

**Remove these settings keys:**
```python
_settings_key_runtime_mode = "whitebox_workflows/runtime_mode"
_settings_key_runtime_local_python = "whitebox_workflows/runtime_local_python"
_settings_key_installation_strategy = "whitebox_workflows/installation_strategy"
_settings_key_backend_wbw_path = "whitebox_workflows/backend_wbw_path"
_settings_key_backend_wbw_version = "whitebox_workflows/backend_wbw_version"
_settings_key_backend_wbw_override_path = "whitebox_workflows/backend_wbw_override_path"
```

**Replace with:**
```python
_settings_key_python_override_path = "whitebox_workflows/python_override_path"
_settings_key_python_version_cached = "whitebox_workflows/python_version_cached"
```

**Remove these instance variables:**
```python
_runtime_mode
_runtime_local_python
_auto_install_backend
_installation_strategy
_backend_wbw_path
_backend_wbw_version
_backend_wbw_override_path
```

**Replace with:**
```python
_python_override_path = ""
_python_version_cached = ""
```

**In `__init__`:**
- Load only the two new settings keys
- Call `refresh_backend_status()` once on plugin load to populate version cache

**In `_show_settings()`:**
- Create `WhiteboxSettingsDialog` with new simple settings
- After user clicks OK, save `python_override_path` to QSettings
- Call `refresh_backend_status()` to update cache if user changed the path

**In tool execution (`_run_tool()`):**
- Replace all the complex runtime mode logic with:
  ```python
  interpreter, version = get_backend_python_for_operations(
      self._python_override_path, 
      self._python_version_cached
  )
  # Use interpreter for tool execution
  ```

**In installation:**
- Replace entire install flow with:
  ```python
  result = install_whitebox_workflows(
      self._python_override_path,
      get_qgis_bundled_python()
  )
  if result["installed"]:
      self._python_version_cached = result["version"]
      # Save to settings
  else:
      # Show error dialog with result["error"]
  ```

## Key Behavioral Changes

### Old Behavior
- Plugin load: Called subprocess to detect Python candidates and check versions
- Tool run: Complex decision tree (runtime_mode → local_python_path or qgis or auto)
- Settings: Three overlapping path mechanisms (runtime_local_python, backend_wbw_override_path, backend_wbw_path)
- Many potential points of failure

### New Behavior
- Plugin load: Quick check (no subprocess), load cached version if available
- Tool run: Get cached Python path, use it—done
- Settings: Single override field, single cache field
- Clear fallback: override path → QGIS Python → error

## Migration Checklist

- [ ] Backup existing bootstrap.py and settings.py
- [ ] Copy bootstrap_v2.py → bootstrap.py (or rename, update import in plugin.py)
- [ ] Copy settings_v2.py → settings.py (or rename, update import in plugin.py)
- [ ] Update plugin.py:
  - [ ] Remove old settings keys
  - [ ] Add new settings keys
  - [ ] Remove old instance variables
  - [ ] Add new instance variables
  - [ ] Update `__init__()` to load new settings only
  - [ ] Update `_load_backend_preferences()` → rename to `_initialize_backend()` (just loads cache)
  - [ ] Update `_show_settings()` to create new dialog
  - [ ] Update `_show_settings()` to refresh after user changes override path
  - [ ] Update all tool execution to use `get_backend_python_for_operations()`
  - [ ] Update install flow to use new `install_whitebox_workflows()`
  - [ ] Update diagnostics to show current Python path and cached version only
- [ ] Remove/disable old helper functions (if any)
- [ ] Test plugin load and setting preservation
- [ ] Test tool execution (should use QGIS Python)
- [ ] Test override path setting (switch to different Python)
- [ ] Test installation (should try QGIS, then system Python, then show helpful error)
- [ ] Clean up any remaining subprocess calls outside of bootstrap_v2.py

## Testing Script (for terminal)

```bash
# After changes, validate syntax:
python3 -m py_compile bootstrap_v2.py settings_v2.py

# Check imports (after updating plugin.py):
python3 -c "from bootstrap_v2 import get_qgis_bundled_python; print('OK')"
```

## Rollback Plan

If issues arise:
1. Restore original bootstrap.py, settings.py
2. Restore plugin.py to previous state
3. Restart QGIS
4. Delete settings via direct QGIS4.ini edit if needed

---

## Why This Works

1. **Single code path** – Less complexity, fewer bugs
2. **Cache-based operation** – No subprocess calls during tool use (fast, safe)
3. **Manual refresh only** – Subprocess only when user explicitly asks or during install
4. **Clear fallback chain** – User override → QGIS Python → error (no ambiguity)
5. **Same experience for everyone** – User has same simple control as developer (no special runtime modes)
