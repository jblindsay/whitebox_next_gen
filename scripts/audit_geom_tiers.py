from pathlib import Path

mismatches = [
    "accumulation_curvature",
    "assess_route",
    "average_horizon_distance",
    "breakline_mapping",
    "curvedness",
    "dem_void_filling",
    "difference_curvature",
    "generating_function",
    "horizon_area",
    "horizontal_excess_curvature",
    "local_hypsometric_analysis",
    "low_points_on_headwater_divides",
    "multiscale_curvatures",
    "openness",
    "pennock_landform_classification",
    "ring_curvature",
    "rotor",
    "shadow_animation",
    "shadow_image",
    "shape_index",
    "sky_view_factor",
    "skyline_analysis",
    "slope_vs_aspect_plot",
    "smooth_vegetation_residual",
    "topo_render",
    "topographic_hachures",
    "topographic_position_animation",
    "unsphericity",
    "vertical_excess_curvature",
]

roots = [
    Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src"),
    Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_pro/src"),
]

for tool_id in mismatches:
    print(f"## {tool_id}")
    for root in roots:
        hits = []
        for path in root.rglob("*.rs"):
            text = path.read_text(errors="ignore")
            if tool_id in text:
                hits.append(path)
        label = "oss" if "wbtools_oss" in str(root) else "pro"
        print(f"  {label}: {len(hits)}")
        for hit in hits[:12]:
            print(f"   - {hit}")
