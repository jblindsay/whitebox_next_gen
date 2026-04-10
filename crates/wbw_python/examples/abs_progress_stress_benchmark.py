import statistics
import time

import whitebox_workflows as wb


WORKING_DIR = "/Users/johnlindsay/Documents/data/UofWaterloo_lidar/"
INPUT_RASTER = "DEM.tif"
ITERATIONS = 10


def run_abs(wbe, dem, use_callback):
    event_count = 0
    progress_values = []

    callback = None
    if use_callback:
        def cb(event):
            nonlocal event_count, progress_values
            event_count += 1
            if isinstance(event, dict):
                event_type = event.get("type")
                pct = event.get("percent")
                if event_type == "progress" and pct is not None:
                    progress_values.append(float(pct))
        callback = cb

    start = time.perf_counter()
    _ = wbe.raster.abs(input=dem, callback=callback)
    elapsed = time.perf_counter() - start

    unique_progress = len({round(v, 6) for v in progress_values})
    return elapsed, event_count, unique_progress


def summarize(label, elapsed_values):
    print(
        f"{label}: mean={statistics.mean(elapsed_values):.6f}s, "
        f"min={min(elapsed_values):.6f}s, max={max(elapsed_values):.6f}s"
    )


def main():
    wbe = wb.WbEnvironment(include_pro=True, tier="pro")
    wbe.working_directory = WORKING_DIR
    dem = wbe.read_raster(INPUT_RASTER)

    # Warm up runtime caches/JIT paths.
    _ = wbe.raster.abs(input=dem)

    no_cb_elapsed = []
    cb_elapsed = []
    cb_event_counts = []
    cb_unique_progress = []

    for _ in range(ITERATIONS):
        elapsed, _, _ = run_abs(wbe, dem, use_callback=False)
        no_cb_elapsed.append(elapsed)

    for _ in range(ITERATIONS):
        elapsed, event_count, unique_progress = run_abs(wbe, dem, use_callback=True)
        cb_elapsed.append(elapsed)
        cb_event_counts.append(event_count)
        cb_unique_progress.append(unique_progress)

    print(f"iterations={ITERATIONS}")
    summarize("no_callback", no_cb_elapsed)
    summarize("with_callback", cb_elapsed)

    print(
        "callback_events: "
        f"mean={statistics.mean(cb_event_counts):.2f}, "
        f"min={min(cb_event_counts)}, max={max(cb_event_counts)}"
    )
    print(
        "unique_progress_values: "
        f"mean={statistics.mean(cb_unique_progress):.2f}, "
        f"min={min(cb_unique_progress)}, max={max(cb_unique_progress)}"
    )

    slowdown = statistics.mean(cb_elapsed) / statistics.mean(no_cb_elapsed)
    print(f"callback_slowdown_factor={slowdown:.3f}")


if __name__ == "__main__":
    main()
