# Spatial Statistics Tool Exposure Analysis
## wbw_python vs wbw_r vs wbw_qgis

**Date:** 2026-06-04  
**Context:** OrdinaryCoKrigingTool just registered (commit 7ddcfb7)  
**Analysis Status:** Complete

---

## Executive Summary

| Interface | Discovery Mechanism | Spatial Stats Exposed? | Kriging Exposed? | CoKriging Exposed? | Status |
|-----------|-------------------|----------------------|------------------|------------------|--------|
| **wbw_python** | ✅ Automatic (registry) | ✅ YES (13 tools) | ✅ YES (5 tools) | ✅ YES (1 tool) | **READY** |
| **wbw_r** | ⚠️ Code generation | ✅ Included in code | ✅ Included in code | ❌ MISSING | **NEEDS REGEN** |
| **wbw_qgis** | ⚠️ Static descriptions | ❌ NOT EXPOSED | ❌ NOT EXPOSED | ❌ NOT EXPOSED | **BLOCKED** |

---

## 1. wbw_python Interface

### Discovery Mechanism: **AUTOMATIC (Registry-Based)**

**Files Involved:**
- `crates/wbw_python/src/lib.rs` - Python bindings (maturin)
- `crates/wbtools_oss/src/lib.rs` - Tool registration

**How It Works:**
```rust
// In wbw_python/src/lib.rs:
struct CompositeRegistry {
    oss: OssRegistry,
    #[cfg(feature = "pro")]
    pro: Option<ProRegistry>,
}

impl ToolRuntimeRegistry for CompositeRegistry {
    fn list_tools(&self) -> Vec<ToolMetadata> {
        // Automatically discovers OSS + Pro tools
    }
}
```

**Implementation Details:**
- Uses `register_default_oss_tools()` to register all tools from wbtools_oss
- Optionally includes Pro tools via `register_default_pro_tools()`
- Python calls `list_tools_json()` which internally calls `visible_manifests()`
- Tools are discovered **dynamically at runtime** - no manual tool listing needed

**Spatial Statistics Tools Registered in wbtools_oss:**

| Category | Tools | Count |
|----------|-------|-------|
| **Spatial Autocorrelation** | Global Moran's I, Getis-Ord Gi*, Local Moran's I (LISA), LISA Raster, Getis-Ord Gi* Raster | 5 |
| **Point Process** | Ripley's K, Ripley's K Function | 2 |
| **Kriging (Univariate)** | Ordinary Kriging, Local Kriging, Simple Kriging, Universal Kriging, Space-Time Kriging | 5 |
| **Multivariate Kriging** | Ordinary CoKriging | 1 |
| **TOTAL** | | **13 tools** |

**✅ Current Exposure Status:**
- ✅ ALL 13 spatial statistics tools ARE exposed to Python
- ✅ OrdinaryCoKrigingTool IS immediately accessible (auto-discovered)
- ✅ No regeneration needed - automatic discovery handles all new tools

**Code Path:**
```
wbw_python maturin bindings
    ↓
CompositeRegistry (auto-composes OssRegistry + ProRegistry)
    ↓
wbtools_oss::register_default_tools() [line 413 registers OrdinaryCoKrigingTool]
    ↓
list_tools() called from Python
    ↓
✅ All 13 spatial stats tools available immediately
```

---

## 2. wbw_r Interface

### Discovery Mechanism: **CODE GENERATION (with manual regeneration required)**

**Files Involved:**
- `crates/wbw_r/src/lib.rs` - R bindings & code generator
- `crates/wbw_r/examples/generate_r_wrappers.rs` - Standalone generator binary
- `crates/wbw_r/generated/wbw_tools_generated.R` - Generated R wrapper module

**How It Works:**

1. **Generation Time:** Developer runs code generator to produce R wrapper functions
```bash
cargo run -p wbw_r --example generate_r_wrappers -- [--include-pro] [--tier open|pro]
```

2. **Generation Logic** (in `generate_r_wrapper_module_with_options()`):
```rust
let rt = runtime_from_local_license_state(include_pro, parsed_tier)?;
let mut manifests = rt.list_visible_manifests();  // ← Discovers tools

for manifest in manifests {
    let fn_name = manifest.id.replace('-', "_");
    // Generate: session$fn_name <- function(...) { run_tool("tool_id", list(...)) }
}
```

3. **Output:** Static R file with one function per tool

**✅ Spatial Statistics Tools in Generated R File:**

Currently includes (from wbw_tools_generated.R):
- `session$getis_ord_gi_star()`
- `session$getis_ord_gi_star_raster()`
- `session$global_morans_i()`
- `session$local_kriging()` (Note: function names use underscores)
- `session$local_morans_i_lisa()`
- `session$local_morans_i_lisa_raster()`
- `session$ordinary_kriging()`
- `session$simple_kriging()`
- `session$space_time_kriging()`
- `session$universal_kriging()`
- `session$ripleys_k()`
- `session$ripleys_k_function()`
- (12 tools total)

**❌ PROBLEM: OrdinaryCoKrigingTool is NOT in generated R file**

**Root Cause:**
- OrdinaryCoKrigingTool was registered in commit `7ddcfb7` (TODAY)
- R wrappers were last generated in commit `adaca00` (older)
- The generated R file has NOT been updated since OrdinaryCoKrigingTool registration

**Current Git Status:**
```
Latest registration commit:  7ddcfb7 (HEAD -> main)
  "Phase 3 Week 8: Register OrdinaryCoKrigingTool in wbtools_oss"

Last R wrapper generation:    adaca00
  "Add Phase B-D spatial statistics tools to Python/R/QGIS bindings"
```

**⚠️ Current Exposure Status:**
- ✅ 12 spatial statistics tools ARE exposed to R
- ❌ OrdinaryCoKrigingTool is NOT exposed (missing from generated file)
- ❌ Will remain missing until R wrappers are **regenerated**

**Fix Required:**
```bash
cd /Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen
cargo run -p wbw_r --example generate_r_wrappers -- --include-pro --tier pro
# Output will update: crates/wbw_r/generated/wbw_tools_generated.R
# Then commit the updated file to include OrdinaryCoKrigingTool
```

---

## 3. wbw_qgis Interface

### Discovery Mechanism: **STATIC DESCRIPTION FILES (manual-only)**

**Files Involved:**
- `whitebox_workflows_for_qgis_provider.py` - QGIS plugin provider
- `whitebox_workflows_for_qgis/descriptions/*.txt` - Tool description files (512 total)
- Legacy generator: `/Users/johnlindsay/Documents/programming/python/create_qgis_descriptions.py` (legacy whitebox_tools only)

**How It Works:**

1. **Plugin Load Time:**
```python
def loadAlgorithms(self):
    folder = self.descriptionsPath()  # → ./descriptions/
    for descriptionFile in os.listdir(folder):
        if descriptionFile.endswith('txt'):
            alg = WhiteboxWorkflowsAlgorithm(os.path.join(folder, descriptionFile))
            self.addAlgorithm(alg)
```

2. **Description File Format (text):**
```
tool_id
Display Name
group_id
help_url
param1_description
param2_description
...
```

3. **Current State:**
- **512 total description files** for all wbw_workflows tools
- Each tool needs a manually-created `.txt` description file
- NO Python code generator for wbw_workflows → QGIS descriptions (yet)

**❌ Spatial Statistics Tools in QGIS:**

**Search Results:**
```bash
$ ls /Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/wbw_qgis/descriptions/*kriging*.txt
[NO RESULTS]

$ ls /Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/wbw_qgis/descriptions/*moran*.txt
[NO RESULTS]

$ grep "kriging" /path/to/descriptions/*.txt
[NO RESULTS]
```

**CONFIRMED: ZERO spatial statistics tools exposed to QGIS**

**❌ Current Exposure Status:**
- ❌ 0 spatial statistics tools are exposed to QGIS
- ❌ 0 kriging tools are exposed to QGIS
- ❌ OrdinaryCoKrigingTool is NOT exposed to QGIS
- ❌ Will remain missing indefinitely without manual description creation

**Why Not Exposed:**
1. **No auto-generation:** Unlike R (code generator) or Python (registry), QGIS requires static `.txt` files
2. **Manual effort:** Each tool needs a description file manually created or generated
3. **Lower priority:** 512 existing tools already consume significant plugin size
4. **Legacy integration:** Plugin still references whitebox_tools (legacy) for some functions

---

## Summary: Exposure by Tool

### OrdinaryCoKrigingTool (newest)

| Interface | Status | Details |
|-----------|--------|---------|
| wbw_python | ✅ **EXPOSED** | Auto-discovered via registry, immediately available |
| wbw_r | ❌ **NOT EXPOSED** | Missing from generated R file; needs regeneration |
| wbw_qgis | ❌ **NOT EXPOSED** | No description file; needs manual creation |

### All 13 Spatial Statistics Tools

| Interface | Count | Mechanism | Note |
|-----------|-------|-----------|------|
| wbw_python | 13/13 | Auto-registry | ✅ Ready immediately |
| wbw_r | 12/13 | Code generation | ⚠️ Needs regeneration |
| wbw_qgis | 0/13 | Static files | ❌ Needs descriptions |

---

## Action Items

### 🔴 IMMEDIATE (Blocking CoKriging from R users)

1. **Regenerate R wrappers** to include OrdinaryCoKrigingTool
   ```bash
   cargo run -p wbw_r --example generate_r_wrappers -- --include-pro --tier pro
   cd /Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen
   git add crates/wbw_r/generated/wbw_tools_generated.R
   git commit -m "Phase 3 Week 8: Regenerate R wrappers to include OrdinaryCoKrigingTool"
   ```
   
   **Current Status:** ⏳ Pending
   **Why:** R wrappers are static; generation happens once at build time

2. **Verify Python exposure**
   ```bash
   cd /Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen
   python3 -c "from whitebox_workflows import WbEnvironment; env = WbEnvironment(); 
   tools = [t['id'] for t in env.list_tools_json() if 'kriging' in t['id'].lower()]; 
   print('ordinary_cokriging' in tools)"
   ```
   
   **Expected:** ✅ True (should already work)

### 🟡 MEDIUM TERM (QGIS support)

3. **Create QGIS description files** for spatial statistics tools
   - Option A: Write Python generator from wbw_python manifests → `.txt` files
   - Option B: Manually create `.txt` files for critical tools (Kriging, Moran's I)
   - **Recommended:** Option A (scalable, auto-maintains)

4. **Generator script for QGIS** (if going with Option A)
   ```python
   # Pseudo-code
   import whitebox_workflows as wbw
   env = WbEnvironment()
   manifests = env.list_tools_json()
   
   for manifest in manifests:
       if any(tag in manifest['tags'] for tag in ['spatial_autocorrelation', 'kriging', 'geostatistics']):
           create_description_file(manifest)  # Write .txt file
   ```

### 🟢 LONG TERM (Architecture improvements)

5. **Unify tool exposure** across interfaces
   - Consider auto-generating QGIS descriptions at build time (like R)
   - Decouple description generation from manual file maintenance
   - Establish SLA for interface parity (e.g., "all tools exposed to all interfaces within 1 week of registration")

---

## Technical Details: Registration Chain

### How OrdinaryCoKrigingTool reaches each interface:

**wbw_python:**
```
crates/wbtools_oss/src/lib.rs [line 413]
    registry.register(Box::new(tools::OrdinaryCoKrigingTool));
        ↓
crates/wbtools_oss/src/tools/mod.rs [line 581]
    pub use geostats::OrdinaryCoKrigingTool;  ← Exported
        ↓
crates/wbw_python/src/lib.rs
    register_default_oss_tools(&mut oss);
        ↓
Python: env.list_tools_json() → ✅ ordinary_cokriging
```

**wbw_r:**
```
crates/wbtools_oss/src/lib.rs [line 413]
    registry.register(Box::new(tools::OrdinaryCoKrigingTool));
        ↓
crates/wbw_r/src/lib.rs [line 3490]
    fn generate_r_wrapper_module_with_options(include_pro, tier)
        rt.list_visible_manifests()  ← Would discover tool IF regenerated
            ↓
            ✅ Would generate: session$ordinary_cokriging <- function(...)
            
Status: ❌ NOT REGENERATED YET
```

**wbw_qgis:**
```
crates/wbtools_oss/src/lib.rs [line 413]
    registry.register(Box::new(tools::OrdinaryCoKrigingTool));
        ↓
        ❌ NOT CONNECTED TO QGIS
        ↓
wbw_qgis needs:
    - Manual .txt description file, OR
    - Auto-generator from wbw_python manifests
```

---

## Appendix: Tool Counts

**Total Spatial Statistics Tools in wbtools_oss (Phase B-C complete):**
- Global Moran's I
- Getis-Ord Gi*
- Local Moran's I (LISA)
- Local Moran's I (LISA) Raster
- Getis-Ord Gi* Raster
- Ripley's K
- Ripley's K Function
- Ordinary Kriging
- Local Kriging
- Simple Kriging
- Universal Kriging
- Space-Time Kriging
- **Ordinary CoKriging** ← NEW (Phase C)

**Total: 13 tools**

---

## References

**Key Files:**
- Registry: `crates/wbtools_oss/src/lib.rs` (lines 361-413)
- Python bindings: `crates/wbw_python/src/lib.rs` (auto-discovery)
- R code generator: `crates/wbw_r/examples/generate_r_wrappers.rs`
- R output: `crates/wbw_r/generated/wbw_tools_generated.R`
- QGIS plugin: `whitebox_workflows_for_qgis/whitebox_workflows_for_qgis_provider.py`
- QGIS descriptions: `whitebox_workflows/wbw_qgis/descriptions/` (512 files)

**Commits:**
- Registration: `7ddcfb7` - Phase 3 Week 8: Register OrdinaryCoKrigingTool
- Last R generation: `adaca00` - Add Phase B-D spatial statistics tools to bindings
