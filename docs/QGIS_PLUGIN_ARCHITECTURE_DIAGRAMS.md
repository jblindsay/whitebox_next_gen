# QGIS Plugin Architecture - Visual Summary

## CURRENT STATE (Broken – Hybrid System)

```
┌─────────────────────────────────────────────────────────────────────────┐
│ QGIS Plugin Initialization                                              │
└────────────────────────────┬────────────────────────────────────────────┘
                             │
                ┌────────────▼────────────┐
                │  plugin.py              │
                │  ─────────────────      │
                │  Uses bootstrap_v2 ✓   │
                │  Uses settings_v2 ✓    │
                │  Calls:                 │
                │  • get_wbw_python_to_use
                │  • refresh_backend_status
                │  • install_whitebox_workflows
                └────────────┬────────────┘
                             │
        ┌────────────────────┴────────────────────┐
        │                                         │
        ▼                                         ▼
┌──────────────────────┐              ┌──────────────────────┐
│ provider.py          │              │ algorithm.py         │
│ ─────────────────    │              │ ─────────────────    │
│ Uses bootstrap.py ✗  │              │ Uses bootstrap.py ✗  │
│ (imports:            │              │ (imports:            │
│  load_whitebox_      │              │  create_runtime_     │
│  workflows)          │              │  session, run_proj   │
│                      │              │  ection_wrapper)     │
│ PROBLEM: Calls       │              │                      │
│ discover_tool_       │              │ PROBLEM: Calls       │
│ catalog() from       │              │ create_runtime_      │
│ bootstrap.py flow    │              │ session() which      │
└──────────────────────┘              │ expects external     │
        │                             │ Python discovery     │
        ▼                             │ (bootstrap.py)       │
   Catalog Discovery:                 └──────────────────────┘
   SIMPLE (QGIS bundled                      │
   + override)                               ▼
        │                            Tool Execution:
        │                            COMPLEX (tries external
        │                            Python first, then
        │                            fallback to QGIS)
        │                                   │
        └───────────────────┬───────────────┘
                            │
                            ▼
                    INCONSISTENT!
                    
                    Catalog says tools exist
                    Execution can't find them
                    
                    OR
                    
                    Execution uses different Python
                    than what was discovered
```

---

## TARGET STATE (Fixed – Unified System)

```
┌─────────────────────────────────────────────────────────────────────────┐
│ QGIS Plugin Initialization                                              │
└────────────────────────────┬────────────────────────────────────────────┘
                             │
                ┌────────────▼────────────┐
                │  plugin.py              │
                │  ─────────────────      │
                │  Uses bootstrap_v2 ✓   │
                │  Uses settings_v2 ✓    │
                │  Calls:                 │
                │  • get_wbw_python_to_use
                │  • refresh_backend_status
                │  • install_whitebox_workflows
                │  • create_runtime_session (NEW)
                └────────────┬────────────┘
                             │
        ┌────────────────────┴────────────────────┐
        │                                         │
        ▼                                         ▼
┌──────────────────────┐              ┌──────────────────────┐
│ provider.py          │              │ algorithm.py         │
│ ─────────────────    │              │ ─────────────────    │
│ Uses bootstrap_v2 ✓  │              │ Uses bootstrap_v2 ✓  │
│ (imports:            │              │ (imports:            │
│  load_whitebox_      │              │  create_runtime_     │
│  workflows from v2)  │              │  session, run_proj   │
│                      │              │  ection_wrapper      │
│ GOOD: Calls          │              │  from v2)            │
│ discover_tool_       │              │                      │
│ catalog() from       │              │ GOOD: Calls          │
│ bootstrap_v2.py flow │              │ create_runtime_      │
└──────────────────────┘              │ session() from       │
        │                             │ bootstrap_v2.py      │
        ▼                             │ (same as discovery)  │
   Catalog Discovery:                 └──────────────────────┘
   UNIFIED (QGIS bundled                     │
   + override)                               ▼
        │                            Tool Execution:
        │                            UNIFIED (QGIS bundled
        │                            + override)
        │                                   │
        └───────────────────┬───────────────┘
                            │
                            ▼
                    CONSISTENT!
                    
                    Catalog and execution use
                    same Python interpreter
                    
                    Same backend version
                    Same tool metadata
```

---

## DEPENDENCY GRAPH: CURRENT STATE

```
bootstrap_v2.py (NEW, Simplified)           bootstrap.py (OLD, Complex)
  350 lines                                   1900 lines
  ├─ get_qgis_bundled_python()               ├─ set_runtime_preferences()
  ├─ get_wbw_python_to_use()                 ├─ ExternalRuntimeSession (450 lines)
  ├─ check_whitebox_workflows_installed()    ├─ _discover_external_python_candidates() (200 lines)
  ├─ get_whitebox_workflows_version()        ├─ _discover_conda/pyenv/framework/deadsnakes()
  ├─ refresh_backend_status()                ├─ load_whitebox_workflows()
  ├─ install_whitebox_workflows()            ├─ create_runtime_session()  ◄── USED BY algorithm.py
  └─ get_backend_python_for_operations()     ├─ run_projection_wrapper()  ◄── USED BY algorithm.py
                                             └─ (many helper functions)

settings_v2.py (NEW, Simplified)            settings.py (OLD, Complex)
  250 lines                                   280 lines
  ├─ WhiteboxPluginSettings (12 fields)      ├─ WhiteboxPluginSettings (15 fields)
  │   └─ python_override_path                 │   ├─ runtime_mode
  │   └─ python_version_cached                │   ├─ local_python_path
  └─ WhiteboxSettingsDialog (simpler UI)     │   ├─ backend_wbw_path
                                             │   ├─ backend_wbw_version
                                             │   ├─ backend_wbw_override_path
                                             │   ├─ installation_strategy
                                             └─ WhiteboxSettingsDialog (complex UI)

plugin.py
  └─ Imports bootstrap_v2.py ✓
  └─ Imports settings_v2.py ✓

provider.py
  └─ Imports bootstrap.py ✗ (load_whitebox_workflows)

algorithm.py
  └─ Imports bootstrap.py ✗ (create_runtime_session, run_projection_wrapper)

RESULT: TWO systems in use → Inconsistency
```

---

## DEPENDENCY GRAPH: TARGET STATE

```
bootstrap_v2.py (ENHANCED, Unified)
  600 lines (expanded from 350)
  ├─ get_qgis_bundled_python()
  ├─ get_wbw_python_to_use()
  ├─ check_whitebox_workflows_installed()
  ├─ get_whitebox_workflows_version()
  ├─ refresh_backend_status()
  ├─ install_whitebox_workflows()
  ├─ get_backend_python_for_operations()
  ├─ load_whitebox_workflows()          ◄── FROM bootstrap.py
  ├─ create_runtime_session()           ◄── FROM bootstrap.py (SIMPLIFIED)
  ├─ run_projection_wrapper()           ◄── FROM bootstrap.py (SIMPLIFIED)
  ├─ RuntimeSession class (NEW)
  └─ (helper functions)

settings_v2.py (FINAL, Unified)
  250 lines
  ├─ WhiteboxPluginSettings (12 fields)
  │   └─ python_override_path
  │   └─ python_version_cached
  └─ WhiteboxSettingsDialog (simple UI)

plugin.py
  └─ Imports bootstrap_v2.py ✓
  └─ Imports settings_v2.py ✓

provider.py
  └─ Imports bootstrap_v2.py ✓ (load_whitebox_workflows)

algorithm.py
  └─ Imports bootstrap_v2.py ✓ (create_runtime_session, run_projection_wrapper)

RESULT: ONE system in use → Consistency
```

---

## SETTINGS KEYS: MIGRATION PATH

### Current State (OLD)
```
whitebox_workflows/runtime_mode → "auto" | "local" | "qgis"
whitebox_workflows/local_python_path → "/path/to/python" (if mode == "local")
whitebox_workflows/backend_wbw_path → "/path/where/wbw/lives" (display only)
whitebox_workflows/backend_wbw_version → "2.0.3" (display only)
whitebox_workflows/backend_wbw_override_path → "/path/override" (if specified)
whitebox_workflows/installation_strategy → "whiteboxgeo_wheel" | "pip_system_python"
whitebox_workflows/include_pro → true | false
whitebox_workflows/requested_tier → "open" | "pro" | "enterprise"
```

### Target State (NEW)
```
whitebox_workflows/python_override_path → "/path/to/python" (empty = use QGIS bundled)
whitebox_workflows/python_version_cached → "2.0.3" (display only, read-only)
whitebox_workflows/include_pro → true | false
whitebox_workflows/tier → "open" | "pro" | "enterprise"
whitebox_workflows/auto_install_backend → true | false
whitebox_workflows/auto_check_backend_updates → true | false
```

### Migration Logic (in plugin.py)
```python
def _migrate_settings_if_needed():
    """Upgrade OLD settings keys to NEW keys if needed."""
    old_mode = QSettings().value("whitebox_workflows/runtime_mode", "")
    new_override = QSettings().value("whitebox_workflows/python_override_path", "")
    
    # If NEW key doesn't exist but OLD keys do, migrate
    if not new_override and old_mode:
        if old_mode == "local":
            old_path = QSettings().value("whitebox_workflows/local_python_path", "")
            if old_path:
                QSettings().setValue("whitebox_workflows/python_override_path", old_path)
        # Else: modes "auto" and "qgis" map to empty override_path
        
    # Clean up OLD keys
    QSettings().remove("whitebox_workflows/runtime_mode")
    QSettings().remove("whitebox_workflows/local_python_path")
    # etc.
```

---

## PYTHON DISCOVERY FLOW

### Current (bootstrap.py – 8 tiers, complex)
```
Start
  ├─ Environment var WBW_EXTERNAL_PYTHON
  ├─ Dev venvs (~/.venv, .venv-wbw)
  ├─ PATH (python3, python)
  ├─ Windows C:\Python3X\python.exe
  ├─ Windows Anaconda
  ├─ Windows Store
  ├─ macOS framework
  ├─ macOS Homebrew
  ├─ Linux system
  ├─ Linux deadsnakes
  ├─ Conda/Miniconda
  └─ pyenv
```

### Target (bootstrap_v2.py – simple, 2-tier)
```
Start
  ├─ IF override_path specified AND valid → Use it
  ├─ ELSE → Use QGIS bundled (sys.executable)
  └─ IF neither available → Error
```

---

## ERROR HANDLING: CURRENT vs TARGET

### Current (bootstrap.py – string matching)
```python
if "include_pro=true requested" in error.lower() and \
   "does not include pro support" in error.lower():
    # Try downgraded session
    
elif any(token in error.lower() for token in [
    "legacy whitebox_workflows runtime",
    "requires whitebox_workflows next gen",
    "unexpected keyword argument 'include_pro'",
    ...
]):
    # Try external Python fallback
```

### Target (bootstrap_v2.py – simpler)
```python
try:
    session = RuntimeSession(python_exe, include_pro, tier)
except RuntimeBootstrapError as exc:
    if "Pro" in str(exc) and include_pro:
        # Try without Pro
        session = RuntimeSession(python_exe, include_pro=False, tier)
    else:
        raise
```

---

## TOOL EXECUTION FLOW: CURRENT vs TARGET

### Current (algorithm.py using bootstrap.py)
```
processAlgorithm()
  ├─ Try external Python discovery
  ├─ If not found, try current Python
  ├─ If Pro not available, downgrade to OSS
  ├─ If legacy detected, try external again
  └─ Create ExternalRuntimeSession
       └─ Try persistent worker
            └─ If fails, one-off subprocess
```

### Target (algorithm.py using bootstrap_v2.py)
```
processAlgorithm()
  ├─ Get Python to use (override or QGIS bundled)
  ├─ Create RuntimeSession
  │   └─ Calls subprocess: import wbw; run tool
  │   └─ Streams progress via callback
  └─ Return result
```

---

## TESTING MATRIX: VERIFY MIGRATION SUCCESSFUL

| Test Case | Current Expected | Target Expected | Status |
|-----------|------------------|-----------------|--------|
| Fresh install, QGIS bundled has WbW-Py | ✓ Load | ✓ Load | ✓ |
| Fresh install, QGIS bundled missing WbW-Py | Offer install | Offer install | ✓ |
| Upgrade with OLD settings keys | Settings lost | Settings migrated | ✓ |
| User sets override path | Complex (runtime_mode + local_python_path) | Simple (python_override_path) | ✓ |
| Execute tool (simple) | Complex path, may use external | Simple path, QGIS bundled | ✓ |
| Execute tool (projection) | Tries external first | Uses QGIS bundled | ✓ |
| Hung subprocess | Plugin freezes | Plugin kills after timeout | ✓ |
| License activation | Works | Works | ✓ |
| Settings persist restart | Works | Works | ✓ |

---

## CODE METRICS: BEFORE vs AFTER

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| bootstrap.py lines | 1900 | — | -1900 (DELETE) |
| bootstrap_v2.py lines | 350 | 600 | +250 (ENHANCE) |
| settings.py lines | 280 | — | -280 (DELETE) |
| settings_v2.py lines | 250 | 250 | — (KEEP) |
| Total bootstrap lines | 2250 | 600 | -1650 (-73%) |
| Total settings lines | 530 | 250 | -280 (-53%) |
| Cyclomatic complexity | HIGH (7+ levels) | LOW (2-3 levels) | REDUCED |
| Dead code (persistent worker) | 450 lines | — | REMOVED |
| Unused discovery tiers | 200 lines | — | REMOVED |
| Duplicate functions | 100 lines | — | CONSOLIDATED |
| **Total Reduction** | — | — | **-2210 lines** |

---

## DECISION TREE: WHICH APPROACH?

```
START: "Plugin has two bootstrap systems"
  │
  ├─ Approach A: "Keep both, make them coexist"
  │  Cost: Medium (2-3 hours API bridging)
  │  Benefit: Backward compatible, no deletions
  │  Risk: Hidden bugs, maintenance burden
  │  Verdict: NOT RECOMMENDED
  │
  ├─ Approach B: "Keep old, deprecate new"
  │  Cost: High (migrate plugin.py back to old system)
  │  Benefit: Advanced features available
  │  Risk: Complex codebase hard to maintain
  │  Verdict: NOT RECOMMENDED
  │
  └─ Approach C: "Migrate to new, consolidate" ◄── RECOMMENDED
     Cost: Medium (4-6 hours)
     Benefit: Simple, clear codebase, matches user needs
     Risk: Requires testing all scenarios
     Verdict: RECOMMENDED
        │
        ├─ Migrate algorithm.py to bootstrap_v2.py imports
        ├─ Migrate provider.py to bootstrap_v2.py imports
        ├─ Enhance bootstrap_v2.py with missing functions
        ├─ Add settings migration
        ├─ Delete bootstrap.py completely
        ├─ Delete settings.py completely
        └─ Test end-to-end
```

---

