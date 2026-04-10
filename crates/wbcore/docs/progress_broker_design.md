# Progress Broker Design (Language-Agnostic)

## Objectives

- Keep parallel compute fast by avoiding callback work on worker threads.
- Provide smooth user-visible progress for long-running tools.
- Ensure identical behavior across Python and R bindings.
- Cap callback progress event volume to avoid waste.

## Core Guarantees

1. Worker threads do not invoke user callbacks.
2. Progress callbacks are emitted by one reporter path only.
3. Progress percent events are integer-bucket coalesced.
4. For any run, progress callback events are capped at 101 buckets (0..100), with at most one event per bucket.
5. Message events are rate-limited and optional.

Notes:
- The 101 cap is for percent buckets only.
- If tools choose to skip explicit 0, cap becomes 100.
- Final 100% is always emitted exactly once on successful completion.

## High-Level Architecture

### 1) Worker-side accounting

Workers only update cheap shared counters:

- `phase_completed_units: AtomicU64`
- optional `phase_messages_dropped: AtomicU64`

No worker performs callback dispatch or JSON conversion.

### 2) Reporter-side emission

A single reporter (main thread or a dedicated lightweight reporter thread) periodically samples counters and emits events via `ProgressSink`.

Emission policy:

- Emit when integer percent bucket advances.
- Optionally apply min cadence (e.g. 100-250 ms) for UI smoothness.
- Coalesce multiple updates into latest bucket.

### 3) Phase-weighted mapping

Tools declare weighted phases:

- `compute` (e.g. 0.0..0.85)
- `materialize` (e.g. 0.85..0.95)
- `write` (e.g. 0.95..1.0)

Progress percent is derived from weighted phase completion.

## Proposed wbcore API Surface

```rust
pub struct ProgressPlan {
    pub phases: Vec<ProgressPhase>,
}

pub struct ProgressPhase {
    pub id: &'static str,
    pub weight: f64,
    pub total_units: u64,
}

pub struct ProgressBroker<'a> {
    sink: &'a dyn ProgressSink,
    // internal atomics/state
}

impl<'a> ProgressBroker<'a> {
    pub fn new(sink: &'a dyn ProgressSink, plan: ProgressPlan) -> Self;

    // Called by orchestrator/tool setup.
    pub fn start(&self);

    // Returns a handle that workers can cheaply update.
    pub fn phase_counter(&self, phase_id: &'static str) -> ProgressCounter;

    // Called by reporter loop or orchestrator checkpoints.
    pub fn flush(&self);

    // Ensures final 100% exactly once.
    pub fn finish_ok(&self);

    // Optional terminal state for errors/cancel.
    pub fn finish_err(&self, msg: &str);
}

#[derive(Clone)]
pub struct ProgressCounter {
    // cheap worker-side handle
}

impl ProgressCounter {
    #[inline]
    pub fn worked(&self, units: u64);
}
```

Optional helper for tools that do not need continuous reporting:

```rust
pub struct PercentCoalescer {
    // guarantees one emit per bucket
}

impl PercentCoalescer {
    pub fn emit_fraction(&self, sink: &dyn ProgressSink, pct01: f64);
}
```

## Event Volume Characteristics

With integer-bucket coalescing:

- Max percent events: 101 (`0..100`) or 100 (`1..100`) depending on start semantics.
- This is independent of raster row count and chunk count.
- A 20,000-row raster and a 2,000-row raster can produce the same capped progress event count.

## Tool Integration Pattern

For a chunked parallel raster tool:

1. Define plan: `compute=0.85`, `copy=0.10`, `write=0.05`.
2. `compute` total units = number of chunks.
3. Worker loop calls `compute_counter.worked(1)` after each completed chunk.
4. Reporter loop calls `broker.flush()` at fixed cadence (or orchestrator checkpoints).
5. After each serial phase, update that phase counter to completion and flush.
6. Call `finish_ok()` once.

## Why This Works Better

- Removes high-frequency callback work from worker threads.
- Prevents per-row/per-chunk callback spam.
- Keeps UI responsive with bounded event volume.
- Works uniformly for Python and R because transport format is no longer the bottleneck.

## Migration Plan

1. Implement broker in wbcore with tests:
   - monotonic percent
   - cap enforcement
   - final 100 once
2. Integrate one representative raster tool (abs/unary math).
3. Benchmark before/after on large rasters.
4. Roll out to other long-running tools in batches.

## Acceptance Criteria

- For a stress run with millions of cells and many chunks:
  - worker throughput regression <= 2% vs no-progress baseline
  - progress percent events <= 101
  - final event is 100%
  - no duplicate 100% events
