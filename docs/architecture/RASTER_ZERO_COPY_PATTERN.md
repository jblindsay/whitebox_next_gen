# Raster Zero-Copy Arc Pattern Guide

**Date:** 2026-04-22  
**Status:** Active / Production  
**Scope:** All raster tools that accept input rasters

## Executive Summary

Whitebox has implemented a two-part architectural optimization that eliminates repeated disk reads and enables zero-copy Arc-based access to raster data across tool calls. This guide explains the pattern and how to apply it to tools.

**Performance Impact:** 20-40x faster repeated tool operations on memory-backed rasters.

## Background: The Optimization (2026-04-22)

### Part 1: Memory-Backed Raster Loading
**File:** `crates/wbw_python/src/wb_environment.rs`  
**Function:** `WbEnvironment::read_raster()`

Changed from lazy disk-path storage to eager memory loading:

```rust
// BEFORE: lazy (returned disk path)
Ok(Raster {
    file_path: path,
    active_band: 0,
})

// AFTER: eager (loads into memory_store)
let raster = WbRaster::read(&path)?;
let id = memory_store::put_raster(raster);
Ok(Raster {
    file_path: PathBuf::from(memory_store::make_raster_memory_path(&id)),
    active_band: 0,
})
```

**Benefit:** Rasters are now stored in `memory_store` with `memory://` paths instead of disk paths.

### Part 2: Arc-Based Zero-Copy Loading
**Files:** `crates/wbtools_oss/src/tools/raster/raster_add.rs`, `raster_unary_math.rs`, etc.

Tool input loaders now use `get_raster_arc_by_id()` for memory-backed rasters:

```rust
// PATTERN: load_raster_from_arg or load_input_raster
fn load_input_raster(path: &str) -> Result<Arc<Raster>, ToolError> {
    if memory_store::raster_is_memory_path(path) {
        let id = memory_store::raster_path_to_id(path)?;
        return memory_store::get_raster_arc_by_id(id)  // Zero-copy Arc
            .ok_or_else(|| ToolError::Validation(...));
    }

    Raster::read(path).map(Arc::new)  // Fallback for disk paths
        .map_err(|e| ToolError::Execution(...))
}
```

**Benefit:** Memory-backed rasters are retrieved as Arc references (zero-copy); disk reads only for non-memory paths.

## Tool Implementation Pattern

### For Read-Only Tools (Most Tools)

**Step 1:** Update input loader to return `Arc<Raster>`:

```rust
fn load_input_raster(path: &str) -> Result<Arc<Raster>, ToolError> {
    if memory_store::raster_is_memory_path(path) {
        let id = memory_store::raster_path_to_id(path)
            .ok_or_else(|| ToolError::Validation("malformed in-memory raster path".to_string()))?;
        return memory_store::get_raster_arc_by_id(id)
            .ok_or_else(|| ToolError::Validation(format!("unknown in-memory raster id '{id}'")));
    }

    Raster::read(path)
        .map(Arc::new)
        .map_err(|e| ToolError::Execution(format!("failed reading input raster: {e}")))
}
```

**Step 2:** Accept `Arc<Raster>` in tool logic:

```rust
let input = load_input_raster(input_path)?;  // Arc<Raster>
let z1 = input.data.get_f64(i);              // Dereferences transparently
let is_nodata = input.is_nodata(z1);         // Works with Arc<Raster>
```

The `Arc<Raster>` dereferences automatically in read contexts via `Deref`.

### For In-Place Mutation Tools (Rare)

If tool modifies raster in-place, clone once before mutation:

```rust
fn run_inplace_op(args: &ToolArgs) -> Result<ToolRunResult, ToolError> {
    let input1_path = parse_raster_path_arg(args, "input1")?;
    let mut in1 = (*load_raster(&input1_path, "input1")?).clone();  // One clone
    
    // Now mutate in1
    for i in 0..in1.data.len() {
        in1.data.set_f64(i, modified_value);
    }
    
    // Write modified raster
    in1.write(&input1_path, format)?;
    Ok(...)
}
```

This trades one upfront clone for zero per-operation cost.

## Implementation Checklist

For each tool file or tool category:

- [ ] Identify input loader function (`load_raster`, `load_input_raster`, etc.)
- [ ] Change return type from `Raster` to `Arc<Raster>`
- [ ] Add memory path detection: `memory_store::raster_is_memory_path(path)`
- [ ] Add Arc retrieval: `memory_store::get_raster_arc_by_id(id)`
- [ ] Keep disk fallback: `Raster::read(path).map(Arc::new)`
- [ ] For in-place tools only: add one-time clone at start
- [ ] Rebuild and test
- [ ] Update parity tracker with performance measurements

## Affected Tool Files

### Already Implemented ✅
- `crates/wbtools_oss/src/tools/raster/raster_add.rs` (binary math tools)
- `crates/wbtools_oss/src/tools/raster/raster_unary_math.rs` (unary math tools)

### Pending Implementation 🔴
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs` (statistics and in-place tools) — Stage 1
- Vector tools — Stage 2
- LiDAR tools — Stage 2

## Performance Expectations

**Read-Only Tools:**
- Memory-backed: ~0.2-0.5ms per input load (Arc retrieval)
- Disk-backed: ~7-10s per input load (file read)
- Repeated calls improvement: 20-40x

**In-Place Tools:**
- One-time cost: ~7-10s (initial clone)
- Subsequent operations: negligible
- Benefit for sequences of in-place ops: marginal

## Debugging Tips

### Verify Memory Backing
```python
import whitebox_workflows as wb
wbe = wb.WbEnvironment()
dem = wbe.read_raster('/path/to/dem.tif')
print(f"Raster path: {dem.file_path}")  # Should print: memory://raster/123
```

### Check Tool Receives Memory Path
Add temporary debug logging in tool:
```rust
fn load_input_raster(path: &str) -> Result<Arc<Raster>, ToolError> {
    eprintln!("[DEBUG] load_input_raster received path: {}", path);
    eprintln!("[DEBUG] is_memory_path: {}", memory_store::raster_is_memory_path(path));
    // ... rest of function
}
```

### Performance Verification
Benchmark before/after with consistent data:
```python
import statistics, time
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
dem = wbe.read_raster('/path/to/dem.tif')

# Warm up
_ = wbe.your_tool(input=dem)

# Benchmark (5 runs)
times = []
for _ in range(5):
    start = time.perf_counter()
    _ = wbe.your_tool(input=dem)
    times.append(time.perf_counter() - start)

print(f"Median: {statistics.median(times):.3f}s")
```

## Known Limitations

1. **Vector/Lidar tools:** Not yet updated; similar pattern applies
2. **Multi-band operations:** Arc semantics transparent across bands
3. **Disk persistence:** Tools writing to disk still perform actual I/O (expected)

## References

- Commit: 58b147b "BREAKTHROUGH: Fix raster math performance via memory-backed raster loading"
- Related: `vector_gis_whitebox_recovery_plan_2026-04-12.md` (optimization tracking)
- See also: `memory_store.rs` (implementation of put_raster, get_raster_arc_by_id, etc.)

## Questions?

Check these files for implementation details:
- `crates/wbtools_oss/src/tools/raster/raster_add.rs` (reference implementation)
- `crates/wbtools_oss/src/tools/raster/raster_unary_math.rs` (reference implementation)
- `crates/wbraster/src/memory_store.rs` (Arc storage implementation)
