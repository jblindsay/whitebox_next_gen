# QGIS Plugin Architecture – Quick Reference

## Problem in One Sentence
**The plugin has two parallel bootstrap systems (old complex + new simple) but they're only partially integrated, causing algorithm.py and provider.py to use different runtime detection logic than plugin.py.**

---

## Files at a Glance

### Bootstrap Files
| File | Size | Status | Purpose |
|------|------|--------|---------|
| **bootstrap.py** | 1900 L | OLD/COMPLEX | Multi-runtime external Python discovery, ExternalRuntimeSession |
| **bootstrap_v2.py** | 350 L | NEW/SIMPLE | QGIS bundled + optional override, installation helpers |

**Problem:** algorithm.py imports from bootstrap.py; plugin.py uses bootstrap_v2.py

### Settings Files
| File | Size | Status | Purpose |
|------|------|--------|---------|
| **settings.py** | 280 L | OLD/COMPLEX | Runtime mode selector, complex backend tracking |
| **settings_v2.py** | 250 L | NEW/SIMPLE | Single python_override_path, simplified UI |

**Problem:** plugin.py uses settings_v2.py; old settings keys lost on upgrade

### Integration Files
| File | Size | Imports | Purpose |
|------|------|---------|---------|
| **plugin.py** | 1000+ L | bootstrap_v2, settings_v2 | Main plugin, settings persistence |
| **provider.py** | 120 L | bootstrap (OLD) | QGIS provider interface |
| **algorithm.py** | 1000+ L | bootstrap (OLD) | Tool execution |

---

## What's Broken

1. **Runtime Inconsistency**: Catalog discovery (plugin.py → bootstrap_v2) uses QGIS bundled Python. Tool execution (algorithm.py → bootstrap) tries external Python first. If external Python has different WbW-Py version → tools fail.

2. **Settings Migration**: Upgrading from old plugin loses settings (runtime_mode, local_python_path, backend_wbw_path, etc.) because new keys don't match old keys.

3. **Missing Functions**: bootstrap_v2.py lacks `create_runtime_session()` and `run_projection_wrapper()` that algorithm.py needs.

4. **Subprocess Hanging**: ExternalRuntimeSession persistent worker has no readline() timeout, can hang indefinitely.

5. **Dead Code**: 450 lines of persistent worker code (not used), 300 lines of multi-tier Python discovery (not used).

---

## The Fix: Option Recommended

**Consolidate to bootstrap_v2.py system**

1. Enhance bootstrap_v2.py to add:
   - `create_runtime_session()`
   - `run_projection_wrapper()`
   - `RuntimeSession` class

2. Migrate imports:
   - algorithm.py: `from .bootstrap_v2 import ...`
   - provider.py: `from .bootstrap_v2 import ...`

3. Add settings migration in plugin.py

4. Delete bootstrap.py and settings.py

**Time:** 4-6 hours + testing

**Benefit:** Single clear system, -1650 lines of code

---

## Critical Numbers

| Metric | Value |
|--------|-------|
| bootstrap.py lines | 1900 |
| bootstrap_v2.py lines | 350 |
| Total bootstrap code | 2250 |
| Can be reduced to | 600 |
| Reduction % | -73% |
| Dead code (persistent worker) | 450 lines |
| Unused discovery tiers | 200 lines |
| Files to delete | 2 (bootstrap.py, settings.py) |
| Files to enhance | 2 (bootstrap_v2.py, algorithm.py, provider.py) |
| Estimated fix time | 4-6 hours |

---

## Decision Points

### 1. Support Multiple Runtime Modes?
- **Old design** offers: auto/local/qgis mode selector
- **New design** offers: just QGIS bundled + override
- **Recommendation:** Keep simple (no multiple modes)

### 2. Use Persistent Stream Worker?
- **Current:** 450 lines, but algorithm.py doesn't use it
- **New:** One-off subprocess is simpler and adequate
- **Recommendation:** Remove persistent worker

### 3. Support External Python Discovery?
- **Current:** 8-tier discovery (conda, pyenv, framework, etc.)
- **New:** Just QGIS bundled + user override
- **Recommendation:** Keep simple (user can manually set override path)

---

## Migration Checklist

- [ ] Phase 1: Enhance bootstrap_v2.py (4 hours)
  - [ ] Add RuntimeSession class
  - [ ] Add create_runtime_session()
  - [ ] Add run_projection_wrapper()
  - [ ] Add load_whitebox_workflows()
  
- [ ] Phase 2: Migrate imports (2 hours)
  - [ ] algorithm.py: import from bootstrap_v2
  - [ ] provider.py: import from bootstrap_v2
  - [ ] Test: cargo check, python -m py_compile
  
- [ ] Phase 3: Settings migration (2 hours)
  - [ ] Add migration function in plugin.py
  - [ ] Test: fresh install + upgrade scenarios
  
- [ ] Phase 4: Subprocess timeouts (1 hour)
  - [ ] Add readline() timeout
  - [ ] Test: hung subprocess is killed
  
- [ ] Phase 5: Cleanup (2 hours)
  - [ ] Delete bootstrap.py
  - [ ] Delete settings.py
  - [ ] Move _Dummy to shared utility
  
- [ ] Phase 6: Testing (3-4 hours)
  - [ ] Full end-to-end testing

---

## Key Functions to Port from bootstrap.py → bootstrap_v2.py

```python
# MUST PORT
create_runtime_session(include_pro, tier)
run_projection_wrapper(tool_id, args, ...)

# MUST WRAP
load_whitebox_workflows()

# MUST ADD
class RuntimeSession:
    def run_tool_json_stream(tool_id, args_json, callback)

# CAN DELETE
ExternalRuntimeSession (450 lines)
_discover_*_python() functions (200 lines)
_discover_external_python_candidates() (100 lines)
set_runtime_preferences() (not used)
```

---

## Settings Keys: Old → New Mapping

| Old Key | Old Purpose | New Key | New Purpose |
|---------|-------------|---------|-------------|
| runtime_mode | auto/local/qgis | — | REMOVED (always auto) |
| local_python_path | paired with runtime_mode | — | REMOVED |
| backend_wbw_path | display only | — | REMOVED |
| backend_wbw_version | display only | — | REMOVED |
| backend_wbw_override_path | path override | python_override_path | path override |
| installation_strategy | which install method | — | REMOVED |
| include_pro | license setting | include_pro | license setting |
| requested_tier | license tier | tier | license tier |

---

## Testing Scenarios

| Scenario | Expected Result | Notes |
|----------|-----------------|-------|
| Fresh install, WbW-Py in QGIS | Plugin loads, catalog visible | Baseline |
| Fresh install, no WbW-Py | Offer to install | Bootstrap kicks in |
| User sets override path | Uses override Python | Override takes precedence |
| Execute simple tool | Tool runs successfully | Uses same Python as discovery |
| Execute projection tool | Tool runs, output file created | Calls run_projection_wrapper |
| Upgrade from old plugin | Settings preserved | Migration function works |
| Hung subprocess | Process killed after timeout | Timeout+cleanup |
| License activation | Works, catalog refreshes | Calls invoke_license_function |
| Panel visible on restart | Panel remembered from settings | Settings persistence |

---

## Dependency Graph: Imports to Update

```
algorithm.py
  OLD: from .bootstrap import RuntimeBootstrapError, create_runtime_session, run_projection_wrapper
  NEW: from .bootstrap_v2 import RuntimeBootstrapError, create_runtime_session, run_projection_wrapper

provider.py
  OLD: from .bootstrap import load_whitebox_workflows
  NEW: from .bootstrap_v2 import load_whitebox_workflows

plugin.py
  KEEP: from .bootstrap_v2 import (...)
  KEEP: from .settings_v2 import (...)
```

---

## Code Metrics Summary

| Aspect | Old | New | Impact |
|--------|-----|-----|--------|
| Python discovery tiers | 8 | 1 | Simpler |
| Bootstrap modules | 2 (1900 + 350) | 1 (600) | Consolidated |
| Settings modules | 2 (280 + 250) | 1 (250) | Consolidated |
| Cyclomatic complexity | HIGH (7 levels) | LOW (2-3 levels) | Maintainable |
| Dead code lines | 450 + 200 = 650 | 0 | Cleaner |
| Settings keys | 8 | 6 | Simplified |

---

## Risk Assessment

| Risk | Probability | Severity | Mitigation |
|------|-------------|----------|------------|
| Tool execution breaks | HIGH (if not migrated) | CRITICAL | Phase 2 migration + testing |
| Settings lost on upgrade | HIGH (if migration missing) | HIGH | Phase 3 migration + verify |
| Hung subprocess | MEDIUM (if no timeout) | HIGH | Phase 4 timeout addition |
| Regression on simple tools | MEDIUM | HIGH | Comprehensive testing |
| Regression on projection tools | LOW | HIGH | Specific test for projection tools |

---

## Communication Points

**To User:**
> "The plugin has two partial implementations that need to be unified. This fixes tool execution failures and simplifies the codebase by 73%. It's a 4-6 hour project with clear phases."

**To Developers:**
> "After migration: simpler bootstrap_v2 with ~600 lines, one settings module, clear algorithm.py → bootstrap_v2 import chain. Dead code and complex discovery logic removed."

**To QA:**
> "Test: fresh install, upgrade, tool execution, projection tools, license functions, settings persistence across restarts."

---

## Success Criteria

- [ ] algorithm.py imports from bootstrap_v2 successfully
- [ ] provider.py imports from bootstrap_v2 successfully
- [ ] Tool execution end-to-end test passes
- [ ] Projection tool (reproject_raster) works
- [ ] Settings upgrade preserves user python_override_path
- [ ] Subprocess timeout prevents hangs
- [ ] bootstrap.py and settings.py deleted
- [ ] All tests green

---

## Recommended Reading Order

1. **This document** – Overview (5 min)
2. [QGIS_PLUGIN_ARCHITECTURE_DEEP_DIVE.md](QGIS_PLUGIN_ARCHITECTURE_DEEP_DIVE.md) – Detailed analysis (30 min)
3. [QGIS_PLUGIN_ARCHITECTURE_DIAGRAMS.md](QGIS_PLUGIN_ARCHITECTURE_DIAGRAMS.md) – Visual reference (10 min)
4. **Code review** – bootstrap_v2.py, algorithm.py (30 min)

---

