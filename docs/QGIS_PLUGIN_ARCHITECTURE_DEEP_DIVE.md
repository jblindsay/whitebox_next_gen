# Whitebox QGIS Plugin – Comprehensive Architecture Audit

**Date:** June 6, 2026  
**Status:** Critical Issues Identified  
**Scope:** bootstrap.py, bootstrap_v2.py, settings.py, settings_v2.py, plugin.py, algorithm.py, provider.py

---

## EXECUTIVE SUMMARY

The QGIS plugin has **two parallel bootstrap systems** in mid-migration:

- **bootstrap.py** (1900+ lines): Complex, heavyweight, multi-runtime support
- **bootstrap_v2.py** (350 lines): Simplified, single-design (QGIS bundled + override)

**CRITICAL FLAW:** algorithm.py imports from OLD bootstrap.py, while plugin.py uses NEW bootstrap_v2.py. This creates an **inconsistent runtime state** where tool discovery uses one system and tool execution uses another.

**OUTCOME:** Plugin may appear to load but tools fail at runtime with cryptic errors.

---

## PART 1: FILE STRUCTURE & PURPOSES

### bootstrap.py (1900 lines)
**Purpose:** Complex multi-mode Python runtime discovery and management

| Component | Lines | Purpose |
|-----------|-------|---------|
| `_subprocess_window_kwargs()` | 20 | Platform-safe subprocess kwargs |
| `set_runtime_preferences()` | 15 | Set runtime mode (auto/local/qgis) |
| `RuntimeBootstrapError` | 5 | Custom exception |
| `_parse_next_gen_capabilities()` | 30 | JSON capabilities parsing |
| `ExternalRuntimeSession` class | 450 | Subprocess-based external Python execution |
| `_discover_*_python()` (8 functions) | 300 | Multi-tier Python discovery |
| `_discover_external_python_candidates()` | 200 | Orchestrate all discovery methods |
| `load_whitebox_workflows()` | 8 | Import whitebox_workflows |
| `create_runtime_session()` | 120 | Select runtime (external/current) |
| `run_projection_wrapper()` | 100 | Handle reproject_* and assign_projection_* |

**Key Exports:**
- `create_runtime_session()` ← **USED BY algorithm.py**
- `run_projection_wrapper()` ← **USED BY algorithm.py**
- `load_whitebox_workflows()` ← **USED BY algorithm.py**
- `RuntimeBootstrapError` ← **USED BY algorithm.py**

### bootstrap_v2.py (350 lines)
**Purpose:** Simplified, practical backend detection for QGIS bundled Python

| Component | Lines | Purpose |
|-----------|-------|---------|
| `_subprocess_window_kwargs()` | 20 | Platform-safe subprocess kwargs (DUPLICATE) |
| `get_qgis_bundled_python()` | 15 | Get sys.executable |
| `get_wbw_python_to_use()` | 20 | Simple: override or QGIS bundled |
| `check_whitebox_workflows_installed()` | 15 | Subprocess check for package |
| `get_whitebox_workflows_version()` | 20 | Subprocess call to get version |
| `refresh_backend_status()` | 35 | Refresh backend and cache version |
| `get_backend_python_for_operations()` | 20 | Return cached Python + version |
| `attempt_install_to_qgis_python()` | 60 | Download wheel + pip install |
| `attempt_install_to_system_python()` | 50 | Find system Python + pip install |
| `install_whitebox_workflows()` | 40 | Orchestrate both strategies |

**Key Exports:**
- `get_wbw_python_to_use()` ← **USED BY plugin.py**
- `refresh_backend_status()` ← **USED BY plugin.py**
- `get_backend_python_for_operations()` ← **USED BY plugin.py**
- `install_whitebox_workflows()` ← **USED BY plugin.py**

### settings.py (280 lines)
**Purpose:** Complete settings for complex runtime modes

| Class | Fields | Purpose |
|-------|--------|---------|
| `WhiteboxPluginSettings` | 15 | Settings snapshot |
| `WhiteboxSettingsDialog` | — | Full Qt dialog with groups |

**Key Settings:**
- `runtime_mode` (auto/local/qgis)
- `local_python_path` (paired with runtime_mode)
- `backend_wbw_path` (read-only display)
- `backend_wbw_version` (read-only display)
- `backend_wbw_override_path`
- `installation_strategy` (which install method was used)
- `include_pro`, `tier`, `auto_install_backend`, `auto_check_backend_updates`

### settings_v2.py (250 lines)
**Purpose:** Simplified settings for practical backend configuration

| Class | Fields | Purpose |
|-------|--------|---------|
| `WhiteboxPluginSettings` | 12 | Settings snapshot |
| `WhiteboxSettingsDialog` | — | Simpler Qt dialog (fewer groups) |

**Key Settings:**
- `python_override_path` (replaces runtime_mode + local_python_path)
- `python_version_cached` (read-only display)
- `include_pro`, `tier`, `auto_install_backend`, `auto_check_backend_updates`

### provider.py (120 lines)
**Purpose:** QGIS Processing provider interface

| Function | Purpose |
|----------|---------|
| `id()` | Returns "whitebox_workflows" |
| `name()` | Returns "Whitebox Workflows" |
| `load()` | Calls `refresh_catalog()` |
| `loadAlgorithms()` | Builds algorithms from catalog |
| `refresh_catalog()` | Calls `discover_tool_catalog()` + generates help |

**Imports:**
- `from .algorithm import build_algorithms` – builds algorithm objects
- `from .bootstrap import load_whitebox_workflows` – **USES OLD bootstrap**
- `from .discovery import discover_tool_catalog` – discovers tools

### plugin.py (1000+ lines)
**Purpose:** Main plugin class, GUI management, settings persistence

| Section | Purpose |
|---------|---------|
| Imports | Uses bootstrap_v2 + settings_v2 (NEW system) |
| `__init__()` | Initialize 40+ settings keys |
| `initGui()` | Install panel, actions, ensure backend |
| `_load_*_preference()` | Load state from QSettings |
| `_install_panel()` | Register dock panel |
| `_install_actions()` | Register menu actions |
| `_ensure_backend_available()` | Check/install WbW-Py |
| `_refresh_catalog()` | Call provider.refresh_catalog() |
| Settings keys | 20+ separate QSettings keys for UI state |

**Settings Keys in plugin.py:**
```python
_settings_key_python_override_path = "whitebox_workflows/python_override_path"
_settings_key_python_version_cached = "whitebox_workflows/python_version_cached"
_settings_key_runtime_mode = "whitebox_workflows/runtime_mode"  # NOT USED in v2
_settings_key_local_python_path = "whitebox_workflows/local_python_path"  # NOT USED in v2
```

### algorithm.py (1000+ lines)
**Purpose:** QGIS Processing algorithm implementation for each tool

| Section | Purpose |
|----------|---------|
| Imports | Uses bootstrap (OLD) + settings | 
| `build_algorithms()` | Factory to create QgsProcessingAlgorithm instances |
| `WhiteboxProcessingAlgorithm` class | Per-tool algorithm implementation |
| `processAlgorithm()` | Execute tool: call `create_runtime_session()` |
| Parameter mapping | Convert QGIS parameters → tool args |
| Post-processors | Styling for colored rasters |

**Critical Functions Called:**
- `create_runtime_session()` – get runtime session object
- `run_projection_wrapper()` – handle reproject_* and assign_projection_*

---

## PART 2: PARALLEL SYSTEMS AUDIT

### Bootstrap Functions: What Exists Where?

| Function | bootstrap.py | bootstrap_v2.py | Used By |
|----------|--------------|-----------------|---------|
| `_subprocess_window_kwargs()` | ✓ (20 lines) | ✓ (20 lines) | Both | IDENTICAL – consolidate |
| `set_runtime_preferences()` | ✓ | ✗ | bootstrap only |
| `get_runtime_preferences()` | ✓ | ✗ | bootstrap only |
| `RuntimeBootstrapError` | ✓ | ✓ | Both |
| `ExternalRuntimeSession` | ✓ (450 lines) | ✗ | bootstrap only – complex |
| `_discover_external_python_candidates()` | ✓ (200 lines) | ✗ | bootstrap only – 8 tiers |
| `_discover_conda_installations()` | ✓ | ✗ | bootstrap only |
| `_discover_pyenv_installations()` | ✓ | ✗ | bootstrap only |
| `_discover_framework_pythons()` | ✓ | ✗ | bootstrap only |
| `_discover_deadsnakes_pythons()` | ✓ | ✗ | bootstrap only |
| `load_whitebox_workflows()` | ✓ | ✗ | bootstrap.py used by algorithm.py |
| `create_runtime_session()` | ✓ | ✗ | **algorithm.py CALLS THIS** |
| `run_projection_wrapper()` | ✓ | ✗ | **algorithm.py CALLS THIS** |
| `get_qgis_bundled_python()` | ✗ | ✓ | bootstrap_v2 only |
| `get_wbw_python_to_use()` | ✗ | ✓ | **plugin.py CALLS THIS** |
| `check_whitebox_workflows_installed()` | ✗ | ✓ | bootstrap_v2 only |
| `get_whitebox_workflows_version()` | ✗ | ✓ | bootstrap_v2 only |
| `refresh_backend_status()` | ✗ | ✓ | **plugin.py CALLS THIS** |
| `get_backend_python_for_operations()` | ✗ | ✓ | bootstrap_v2 only |
| `install_whitebox_workflows()` | ✗ | ✓ | bootstrap_v2 only |

### Settings Functions: What Exists Where?

| Setting | settings.py | settings_v2.py | plugin.py | Impact |
|---------|-------------|----------------|-----------|--------|
| `runtime_mode` | ✓ | ✗ | references but doesn't use | UNUSED in v2 |
| `local_python_path` | ✓ | ✗ | references but doesn't use | UNUSED in v2 |
| `backend_wbw_path` | ✓ | ✗ | doesn't reference | UNUSED in v2 |
| `backend_wbw_version` | ✓ | ✗ | doesn't reference | UNUSED in v2 |
| `backend_wbw_override_path` | ✓ | ✗ | doesn't reference | UNUSED in v2 |
| `installation_strategy` | ✓ | ✗ | doesn't reference | UNUSED in v2 |
| `python_override_path` | ✗ | ✓ | ✓ uses | NEW in v2 |
| `python_version_cached` | ✗ | ✓ | ✓ uses | NEW in v2 |

---

## PART 3: CRITICAL WEAKNESSES (Ordered by Impact)

### 1. 🔴 RUNTIME INCONSISTENCY – algorithm.py Uses OLD bootstrap.py (CRITICAL)

**Problem:**
- algorithm.py imports: `from .bootstrap import RuntimeBootstrapError, create_runtime_session, run_projection_wrapper`
- algorithm.py calls: `create_runtime_session()` to get runtime session
- **But:** bootstrap_v2.py has NO `create_runtime_session()` function
- **And:** plugin.py uses bootstrap_v2.py system (simplified)

**Flow Mismatch:**

```
plugin.py (bootstrap_v2):                algorithm.py (bootstrap):
  ├─ Uses QGIS bundled Python              ├─ Tries external Python first
  ├─ Simple override path                  ├─ Multi-tier discovery
  └─ Caches version                        └─ Falls back to QGIS Python

Catalog Discovery:  Simple         Tool Execution:  Complex
(inconsistent!)                    (inconsistent!)
```

**Consequence:**
- Plugin discovers tools using simple backend
- Tools try to execute using complex external Python discovery
- If external Python not found, uses current Python (which might have different WbW-Py version)
- Result: "Tool X not found" or "Tool metadata mismatch" errors

**Fix Required:** Migrate algorithm.py to use bootstrap_v2.py functions OR migrate plugin.py/provider.py back to bootstrap.py

---

### 2. 🔴 SETTINGS MIGRATION ISSUE – Old vs New Keys (CRITICAL)

**Problem:**

Old settings.py stored these keys:
```
whitebox_workflows/runtime_mode
whitebox_workflows/local_python_path
whitebox_workflows/backend_wbw_path
whitebox_workflows/backend_wbw_version
whitebox_workflows/backend_wbw_override_path
whitebox_workflows/installation_strategy
```

New settings_v2.py expects:
```
whitebox_workflows/python_override_path
whitebox_workflows/python_version_cached
```

plugin.py stores/loads:
```
_settings_key_python_override_path = "whitebox_workflows/python_override_path"
_settings_key_python_version_cached = "whitebox_workflows/python_version_cached"
```

**Consequence:**
- If user upgrades from old plugin → new plugin: OLD settings are ignored
- User's custom Python path override is lost
- User's cached version is reset
- Plugin reverts to defaults

**Fix Required:** Migration function to read old keys and write new keys

---

### 3. 🟠 SUBPROCESS TIMEOUT HAZARD – Hanging Processes (HIGH)

**In bootstrap.py:**
- `ExternalRuntimeSession._start_stream_worker()` – NO timeout on persistent worker
- `ExternalRuntimeSession._run_tool_json_stream_persistent()` – reads lines with NO timeout
- Can hang indefinitely if worker crashes or disconnects

**In bootstrap_v2.py:**
- `check_whitebox_workflows_installed()` – has timeout=3 ✓
- `get_whitebox_workflows_version()` – has timeout=3 ✓
- `install_whitebox_workflows()` – has timeout=60 for pip ✓

**Consequence:**
- If whitebox_workflows subprocess crashes, plugin freezes
- QGIS hangs waiting for output
- User must force-quit QGIS

**Fix Required:** Add global timeout to persistent stream worker, add readline() timeout

---

### 4. 🟠 ALGORITHM.PY IMPORT DEAD ENDS (HIGH)

**Problem:**
algorithm.py imports these functions from bootstrap.py:
```python
from .bootstrap import RuntimeBootstrapError, create_runtime_session, run_projection_wrapper
```

But nowhere does algorithm.py call:
- `load_whitebox_workflows()` – ONLY in bootstrap.py, not in bootstrap_v2.py
- `invoke_license_function()` – called by plugin.py's license methods

provider.py calls:
```python
from .bootstrap import load_whitebox_workflows
```

**Consequence:**
- If bootstrap.py is ever removed, algorithm.py and provider.py break
- These functions cannot migrate to bootstrap_v2.py without major rewrite

**Fix Required:** Either consolidate both files, or explicitly support both codepaths

---

### 5. 🟠 ERROR DETECTION STRING MATCHING (MEDIUM)

**Problem:**

bootstrap.py has fragile error detection:
```python
def _is_pro_unavailable_error(message: str) -> bool:
    text = str(message or "").lower()
    return all(marker in text for marker in (
        "include_pro=true requested",
        "does not include pro support",
    ))

def _is_legacy_runtime_error(message: str) -> bool:
    text = str(message or "").lower()
    checks = (
        "legacy whitebox_workflows runtime",
        "requires whitebox_workflows next gen",
        "unexpected keyword argument 'include_pro'",
        "unexpected keyword argument 'tier'",
        "has no attribute 'runtimesession'",
        "object has no attribute 'get_runtime_capabilities_json'",
        "unsupported method: get_runtime_capabilities_json",
    )
    return any(token in text for token in checks)
```

If error message text changes in whitebox_workflows, detection fails silently.

**Consequence:**
- Pro availability fallback may not trigger
- Legacy runtime detection may miss actual legacy runtimes
- Cryptic errors passed through instead

**Fix Required:** Use exception types instead of string matching, if possible

---

### 6. 🟠 UNUSED RUNTIME MODE SELECTOR (MEDIUM)

**Problem:**

settings.py has full runtime mode UI:
- "Auto" (discover external Python)
- "Force local" (use specific Python)
- "Force QGIS" (use bundled)

But plugin.py doesn't read these settings keys:
```python
_settings_key_runtime_mode = "whitebox_workflows/runtime_mode"  # DEFINED but NOT LOADED
_settings_key_local_python_path = "whitebox_workflows/local_python_path"  # DEFINED but NOT LOADED
```

bootstrap_v2.py has NO support for runtime_mode at all.

**Consequence:**
- User can't force external Python
- User can't force QGIS Python
- Runtime mode is locked to "auto" (QGIS bundled + override)

**Fix Required:** Either implement runtime mode selection, or remove UI

---

### 7. 🟡 DUPLICATE _Dummy Qt CLASSES (LOW)

**Problem:**

settings.py, settings_v2.py, and multiple other files each define identical `_Dummy` class:

```python
class _Dummy:  # type: ignore[override]
    def __init__(self, *_a, **_kw):
        pass
    def setChecked(self, *_a, **_kw):
        return None
    # ... 20+ dummy methods
```

**Consequence:**
- Code duplication
- Maintenance burden if dummy methods change

**Fix Required:** Move to shared utility module

---

### 8. 🟡 SETTINGS DIALOG COMPLEXITY (LOW)

**Problem:**

- settings.py dialog builds 16 widgets with complex enable/disable logic
- settings_v2.py dialog builds 12 widgets with simpler logic
- Both have identical style dictionaries

**Consequence:**
- Redundant dialog code
- Unclear which settings are "current" design

**Fix Required:** Consolidate to single settings module or clearly mark deprecated

---

## PART 4: UNNECESSARY COMPLEXITY

### 1. Multi-Tier Python Discovery (bootstrap.py – 300+ lines)

```
Priority 1: env var WBW_EXTERNAL_PYTHON
Priority 2: dev venvs (~/.venv, .venv-wbw, etc.)
Priority 3: PATH search (python3, python)
Priority 4: Windows official C:\Python3X\python.exe
Priority 5: Windows Anaconda (user or system)
Priority 6: Windows Store /AppData/Local/.../python3.exe
Priority 7: macOS framework /Library/Frameworks/Python.framework
Priority 8: macOS Homebrew /opt/homebrew, /usr/local
Priority 9: Linux system /usr/bin/python3
Priority 10: Linux deadsnakes /usr/bin/python3.11, 3.12, etc.
Priority 11: Conda/Miniconda (all platforms)
Priority 12: pyenv ~/.pyenv/versions
```

**But:** plugin.py (NEW design) doesn't use this at all. It uses:
- QGIS bundled Python (sys.executable)
- Optional user override path

**Verdict:** 300+ lines of complex code for a feature plugin.py doesn't use.

### 2. ExternalRuntimeSession Persistent Worker (bootstrap.py – 450 lines)

```python
class ExternalRuntimeSession:
    def _start_stream_worker(self) -> None:
        # Start persistent stdin/stdout process
        # Run tool via JSON command
        # Parse base64-encoded events on stdout
        # Handle __WBW_WORKER_EVENT__, __WBW_WORKER_RESULT__, __WBW_WORKER_ERROR__
```

**But:** algorithm.py never uses the persistent worker codepath. It calls:
```python
session = create_runtime_session()
response = session.run_tool_json_stream(tool_id, args_json, callback)
```

Which falls back to `_run_tool_json_stream_oneoff()` anyway (see line 576 in bootstrap.py).

**Verdict:** Persistent worker is dead code.

### 3. Runtime Mode Selector (settings.py, bootstrap.py)

settings.py offers 3 modes:
- Auto
- Force local
- Force QGIS

But plugin.py doesn't use it. bootstrap_v2.py doesn't support it.

**Verdict:** UI exists but no backend support.

### 4. Complex Fallback Chains (bootstrap.py – create_runtime_session)

```python
try:
    return _external_session()  # Try external
except RuntimeBootstrapError:
    try:
        wbw = load_whitebox_workflows()  # Try current
        if hasattr(wbw, "RuntimeSession"):
            session = wbw.RuntimeSession()
            try:
                session.get_runtime_capabilities_json()
                return session
            except Exception as exc:
                if include_pro and _is_pro_unavailable_error(str(exc)):
                    try:
                        downgraded = wbw.RuntimeSession(include_pro=False)
                        # ...
                        return downgraded
                    except Exception:
                        return _external_session()
                if _is_legacy_runtime_error(str(exc)):
                    try:
                        return _external_session()
                    except Exception:
                        raise RuntimeBootstrapError(...)
```

**Verdict:** 5-level nesting, hard to follow, unclear which path users actually hit.

---

## PART 5: ARCHITECTURE DECISION RECORD

### Current State: Hybrid (Broken)

```
┌─ plugin.py (GUI/settings)
│  ├─ Uses bootstrap_v2.py (simplified)
│  ├─ Uses settings_v2.py (simplified)
│  └─ Calls provider.refresh_catalog()
│
├─ provider.py (QGIS provider)
│  ├─ Uses bootstrap.py (imports load_whitebox_workflows)
│  └─ Calls algorithm.build_algorithms()
│
└─ algorithm.py (tool execution)
   ├─ Uses bootstrap.py (imports create_runtime_session, run_projection_wrapper)
   └─ Creates ExternalRuntimeSession via create_runtime_session()
```

**Problem:** bootstrap_v2.py (catalog discovery) vs bootstrap.py (tool execution) mismatch.

### Target State: ONE System

**Option A: "Keep Simple (bootstrap_v2.py)"**
```
Advantage:
  - Simpler codebase (350 lines vs 1900 lines)
  - Easier to understand and maintain
  - Matches user needs: QGIS bundled + optional override
  - No multi-tier discovery complexity

Disadvantage:
  - Loses advanced features: external Python priority tiers, persistent worker
  - May not support future requirements (multiple runtime environments)

Choice: Recommended if current users only need QGIS bundled + override
```

**Option B: "Keep Complex (bootstrap.py)"**
```
Advantage:
  - Preserves advanced features (multi-tier discovery, persistent worker)
  - Supports future use cases
  - Can optionally use external Python

Disadvantage:
  - 1900+ lines of code, hard to maintain
  - 300+ lines for Python discovery rarely used
  - 450 lines for persistent worker (dead code)
  - Fragile error string matching
  - Settings complexity

Choice: Recommended if future requirements demand multi-runtime support
```

**Option C: "Hybrid (Recommended)"**
```
Keep bootstrap_v2.py but:
  1. Add create_runtime_session() to bootstrap_v2.py that:
     - Returns a minimal RuntimeSession wrapper
     - Doesn't need ExternalRuntimeSession complexity
     - Just calls load_whitebox_workflows() and wraps it

  2. Move run_projection_wrapper() logic to bootstrap_v2.py

  3. Consolidate settings to single system (settings_v2.py pattern)

  4. Migrate algorithm.py to use bootstrap_v2.py

Result: Unified bootstrap_v2.py system, simplified codebase, no dead code

Time: ~4-6 hours (includes testing)
```

---

## PART 6: PRIORITY LIST OF WEAKNESSES

### CRITICAL (Must Fix)
1. algorithm.py uses bootstrap.py; plugin.py uses bootstrap_v2.py → inconsistent runtime
2. Settings migration: old keys lost on upgrade
3. bootstrap_v2.py missing create_runtime_session() that algorithm.py needs

### HIGH (Strongly Recommended)
4. Subprocess hanging risk: no timeout on persistent stream worker
5. Settings keys referenced but not loaded in plugin.py
6. bootstrap_v2.py missing run_projection_wrapper() implementation

### MEDIUM (Nice to Have)
7. Error detection string matching is fragile
8. Runtime mode selector UI exists but unsupported
9. Unused persistent worker code adds maintenance burden

### LOW (Nice to Clean Up)
10. Duplicate _Dummy Qt classes
11. Duplicate _subprocess_window_kwargs()
12. Duplicate style dictionaries (STATUS_STYLES, TIER_STYLES)

---

## PART 7: RECOMMENDED CLEANUP ORDER

### Phase 1: Fix Critical Runtime Inconsistency (4-6 hours)
1. Add minimal `create_runtime_session()` to bootstrap_v2.py
2. Add `run_projection_wrapper()` to bootstrap_v2.py
3. Add `load_whitebox_workflows()` to bootstrap_v2.py (wrapper)
4. Migrate algorithm.py imports: `from .bootstrap_v2 import ...`
5. Migrate provider.py imports: `from .bootstrap_v2 import ...`
6. Test tool execution end-to-end

### Phase 2: Settings Migration (2-3 hours)
1. Update plugin.py to read OLD settings keys if NEW keys don't exist
2. Write migration function: old→new
3. Test on clean install and upgrade scenarios
4. Verify no settings loss

### Phase 3: Add Subprocess Timeouts (1 hour)
1. Add readline() timeout to stream worker
2. Add process-level timeout to persistent worker initialization
3. Test with hung subprocess

### Phase 4: Consolidate Duplicate Code (2 hours)
1. Move _Dummy to shared utility
2. De-duplicate _subprocess_window_kwargs()
3. Consolidate style dictionaries

### Phase 5: Settings Cleanup (3-4 hours)
1. Remove unused runtime_mode UI from settings_v2.py
2. Remove unused local_python_path UI
3. Simplify settings keys to 5-6 core keys
4. Remove settings.py completely
5. Test settings persistence

### Phase 6: Documentation (1-2 hours)
1. Document simplified bootstrap flow
2. Add diagrams for plugin initialization
3. Document settings key names and migration

---

## PART 8: SUGGESTED MINIMAL VIABLE DESIGN

### Single Bootstrap Module (bootstrap_v2.py + functions from bootstrap.py)

```python
# Essential API
def get_qgis_bundled_python() -> str | None:
    """Get QGIS bundled Python (sys.executable)"""

def get_wbw_python_to_use(override_path: str = "") -> str:
    """Get Python to use: override or QGIS bundled"""

def check_whitebox_workflows_installed(interpreter: str) -> bool:
    """Check if whitebox_workflows is installed"""

def get_whitebox_workflows_version(interpreter: str) -> str:
    """Get installed whitebox_workflows version"""

def refresh_backend_status(override_path: str = "") -> dict:
    """Refresh backend detection and cache version"""

def get_backend_python_for_operations(override_path: str = "", 
                                       cached_version: str = "") -> tuple[str, str]:
    """Get Python + version for operations (no subprocess calls)"""

def install_whitebox_workflows(override_path: str = "", qgis_python: str = "") -> dict:
    """Install whitebox_workflows to QGIS Python or system Python"""

# NEW: Support algorithm.py
class RuntimeSession:
    """Minimal wrapper around whitebox_workflows runtime"""
    def __init__(self, python_executable: str, include_pro: bool = True, tier: str = "open"):
        self.interpreter = python_executable
        self.include_pro = include_pro
        self.tier = tier
    
    def run_tool_json_stream(self, tool_id: str, args_json: str, callback=None) -> str:
        """Run tool and stream progress to callback"""
        # ONE implementation: subprocess call, NO persistent worker
        # Has timeout
        # Parses output, calls callback for events

def create_runtime_session(include_pro: bool = True, tier: str = "open") -> RuntimeSession:
    """Create runtime session for tool execution"""
    python_exe = get_backend_python_for_operations()[0]
    return RuntimeSession(python_exe, include_pro, tier)

def run_projection_wrapper(tool_id: str, args: dict, include_pro: bool = True, tier: str = "open") -> dict:
    """Handle reproject_* and assign_projection_* tools"""
    # Implementation: direct WbEnvironment calls or subprocess as needed
```

### Single Settings Module (settings_v2.py basis)

```python
class WhiteboxPluginSettings:
    include_pro: bool
    tier: str
    quick_open_top_match: bool
    panel_show_available: bool
    panel_show_locked: bool
    panel_show_locked_recipes: bool
    panel_width: int
    auto_install_backend: bool
    auto_check_backend_updates: bool
    # SIMPLIFIED:
    python_override_path: str  # User override; empty = use QGIS bundled
    python_version_cached: str  # Read-only, displayed only

    # REMOVED: runtime_mode, local_python_path, backend_wbw_path, 
    #          backend_wbw_version, installation_strategy
```

### Settings Dialog (simplified)

```python
class WhiteboxSettingsDialog(QDialog):
    # Three groups:
    # 1. Backend Configuration
    #    - Current Python (display)
    #    - WbW-Py Version (display)
    #    - Override Python Path (edit)
    #    - Refresh Button
    # 2. License & Tier
    #    - Include Pro (checkbox)
    #    - Tier (combobox)
    #    - Auto-install (checkbox)
    #    - Check updates (checkbox)
    # 3. Panel UI
    #    - Quick open (checkbox)
    #    - Show available (checkbox)
    #    - Show locked (checkbox)
    #    - Panel width (spinbox)
```

### Plugin Initialization Flow (clear)

```
plugin.py initGui():
  1. Load preferences from QSettings
  2. Check backend available
     ├─ Yes → catalog discovery ready
     └─ No → offer to install
  3. Register provider
  4. Refresh catalog (calls provider.refresh_catalog())
  5. Start listening for panel events

provider.py refresh_catalog():
  1. Call discover_tool_catalog(include_pro, tier)
  2. Generate help files
  3. Return catalog

algorithm.py processAlgorithm():
  1. Get session = create_runtime_session(include_pro, tier)
  2. If tool is projection_* or assign_projection_*:
     → Call run_projection_wrapper()
  3. Else:
     → Call session.run_tool_json_stream(tool_id, args_json, callback)
  4. Callback updates QGIS progress bar
  5. Return result
```

---

## PART 9: DECISION POINTS FOR USER

### Question 1: Should We Support Multiple Runtime Modes?

**Current State:** settings.py UI offers 3 modes, but bootstrap_v2.py doesn't support them.

**Decision:**
- [ ] YES – Expand bootstrap_v2.py to support auto/local/qgis modes
  - Add back runtime mode selection to settings
  - Support external Python discovery
  - Estimated: 2-3 hours additional work
  
- [x] NO – Keep simplified design (QGIS bundled + override only)
  - Remove runtime mode selector from UI
  - Simpler, clearer behavior
  - Meets current user needs

**Recommendation:** NO (Keep Simple)  
*Reason: Current plugin.py design suggests single-mode approach. Multiple modes can be added later if needed.*

---

### Question 2: Should We Keep the Persistent Stream Worker?

**Current State:** bootstrap.py has 450-line persistent worker, but algorithm.py uses one-off subprocess anyway.

**Decision:**
- [ ] YES – Fix persistent worker and use it
  - Better performance for long-running tools
  - Estimated: 3-4 hours to fix timeouts and test
  
- [x] NO – Keep one-off subprocess approach
  - Simpler, clearer
  - Subprocess-per-tool is adequate performance
  - No timeout/hanging issues

**Recommendation:** NO (Keep One-Off)  
*Reason: Additional complexity not justified by performance gain for typical tool runs.*

---

### Question 3: Should We Support Both bootstrap.py AND bootstrap_v2.py?

**Current State:** Both exist, algorithm.py uses old, plugin.py uses new.

**Decision:**
- [ ] YES – Keep both and support both codepaths
  - Backward compatible
  - Estimated: 2-3 hours to unify APIs
  
- [x] NO – Migrate completely to bootstrap_v2.py
  - Single, clear codebase
  - Remove 1900 lines of dead code
  - Estimated: 4-6 hours migration

**Recommendation:** NO (Migrate to bootstrap_v2.py)  
*Reason: bootstrap_v2.py is simpler and sufficient for current needs.*

---

## PART 10: IMPLEMENTATION CHECKLIST

### Prerequisite: Define Migration Path
- [ ] User confirms: Drop advanced features from bootstrap.py?
- [ ] User confirms: Keep simple design (bootstrap_v2.py)?
- [ ] User confirms: One-off subprocess per tool is acceptable?

### Phase 1: Enhance bootstrap_v2.py (4-6 hours)
- [ ] Add minimal `RuntimeSession` class to bootstrap_v2.py
- [ ] Add `create_runtime_session()` function
- [ ] Add `run_projection_wrapper()` implementation
- [ ] Add `load_whitebox_workflows()` wrapper
- [ ] Test: Import from bootstrap_v2.py works
- [ ] Test: RuntimeSession.run_tool_json_stream() works

### Phase 2: Migrate Imports (2-3 hours)
- [ ] Update algorithm.py: `from .bootstrap_v2 import RuntimeBootstrapError, create_runtime_session, run_projection_wrapper`
- [ ] Update provider.py: `from .bootstrap_v2 import load_whitebox_workflows`
- [ ] Delete imports from bootstrap.py
- [ ] Test: algorithm.py builds without errors
- [ ] Test: provider.py loads
- [ ] cargo check / python -m py_compile

### Phase 3: Settings Migration (2-3 hours)
- [ ] Add migration function in plugin.py
- [ ] Load old keys if new keys don't exist
- [ ] Convert old → new and store
- [ ] Test: Fresh install works
- [ ] Test: Upgrade from old plugin preserves settings

### Phase 4: Subprocess Timeouts (1 hour)
- [ ] Add timeout to readline() in RuntimeSession
- [ ] Add timeout to process initialization
- [ ] Test: Hanging process is killed after timeout

### Phase 5: Consolidate (2 hours)
- [ ] Move _Dummy to shared utility
- [ ] De-duplicate _subprocess_window_kwargs()
- [ ] Remove settings.py completely
- [ ] Remove old bootstrap.py code not in bootstrap_v2.py

### Phase 6: Testing (3-4 hours)
- [ ] Fresh QGIS install + plugin load + tool execution
- [ ] Tool with progress callback
- [ ] Projection tool execution
- [ ] License activation/deactivation
- [ ] Settings persistence across restarts
- [ ] Backend install/refresh
- [ ] Backend update check

### Phase 7: Documentation (1-2 hours)
- [ ] Add architecture diagram to PLUGIN_ARCHITECTURE.md
- [ ] Document settings key names
- [ ] Document initialization flow
- [ ] Document bootstrap_v2.py API

---

## SUMMARY TABLE: Files to Keep/Modify/Delete

| File | Current | Action | Reason |
|------|---------|--------|--------|
| bootstrap.py | 1900 lines | DELETE | Consolidate into bootstrap_v2.py |
| bootstrap_v2.py | 350 lines | ENHANCE | Add missing functions, extend to 600+ lines |
| settings.py | 280 lines | DELETE | Consolidate into settings_v2.py |
| settings_v2.py | 250 lines | SIMPLIFY | Keep as single settings source |
| plugin.py | 1000+ lines | UPDATE | Fix settings key loads, handle migration |
| algorithm.py | 1000+ lines | UPDATE | Change imports to bootstrap_v2.py |
| provider.py | 120 lines | UPDATE | Change imports to bootstrap_v2.py |

---

## NEXT STEPS

1. **Confirm Decision:** Review Sections 9-10 with user
2. **Create Epic:** Multi-phase migration with checkpoints
3. **Phase 1 Implementation:** Enhance bootstrap_v2.py with RuntimeSession
4. **Incremental Testing:** Ensure each phase doesn't break existing functionality
5. **Document:** Update architecture docs after completion

---

