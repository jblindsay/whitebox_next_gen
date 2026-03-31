# Stream Network Analysis Parity Checklist

Reference legacy source:
- /Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis

Scope:
- New backend implementations in wbtools_oss + wbtools_pro

## Status Legend
- done: implemented and compiled in new backend
- partial: present but still behaviorally simplified vs legacy
- todo: not yet parity-complete

## Tool-by-Tool Status
- strahler_stream_order: done
- horton_stream_order: done
- hack_stream_order: done
- shreve_stream_magnitude: done
- topological_stream_order: done
- stream_link_identifier: done
- stream_link_class: done
- stream_link_length: done (now accepts legacy streams_id_raster semantics)
- stream_link_slope: done (now accepts legacy streams_id_raster semantics)
- stream_slope_continuous: done
- distance_to_outlet: done
- length_of_upstream_channels: done
- find_main_stem: done
- farthest_channel_head: done
- tributary_identifier: done
- remove_short_streams: done
- extract_streams: done
- extract_valleys: done (ported legacy variant/line_thin/filter_size handling)
- raster_streams_to_vector: done
- rasterize_streams: done
- long_profile: done
- long_profile_from_points: done
- repair_stream_vector_topology: done (intersection splitting + dangling arc correction added)
- vector_stream_network_analysis: done (now includes max_ridge_cutting_height and y-junction/intermediate key-point node routing for network traversal)
- prune_vector_streams: done (legacy-style priority-flood downstream assignment, squared-distance snapping semantics, y-junction key-point traversal, and TUCL-style tributary thresholding)
- river_centerlines: done (raster EDT/thinning, endpoint-join, braid-fix, and robust endpoint/loop vectorization workflow)

## Recent Changes Implemented
- wbtools_oss `extract_valleys`:
  - Added legacy-style variants (`lq`, `jandr`, `pandd`)
  - Added `line_thin` and `filter_size`
- wbtools_oss `vector_stream_network_analysis`:
  - Added `max_ridge_cutting_height` parameter support
  - Applied parameter during downstream link selection
  - Added y-junction/intermediate key-point node routing for network traversal
- wbtools_pro `prune_vector_streams`:
  - Added legacy parameter names and aliases: streams/input, dem/input_dem, threshold/magnitude_threshold, snap_distance/snap
  - Added DEM-informed orientation, priority-flood downstream assignment, squared-distance nearest-candidate dangling-outlet snapping, y-junction key-point traversal with precision matching, and TUCL-style tributary thresholding/pruning pass
- wbtools_pro `river_centerlines`:
  - Added legacy parameter names and aliases: raster/water_raster/input, min_length, search_radius/radius
  - Added raster EDT/thinning/endpoint-join/trace pipeline, braid reconnection heuristics, endpoint+loop vectorization, and segment merge/connect plus minimum-length filtering

## Validation Status
- `cargo check -p wbtools_oss -p wbtools_pro`: pass
- `cargo test -p wbtools_pro --lib stream_network_analysis -- --nocapture`: pass (5/5 tests)

## Remaining High-Value Parity Work
- Remaining gaps are now primarily validation depth (larger real-world regression corpus), rather than known missing algorithmic branches.
