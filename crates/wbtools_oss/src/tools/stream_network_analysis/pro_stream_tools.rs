/// Professional stream network analysis tools
///
/// This module contains premium tools for advanced stream analysis:
/// - Prune vector streams
/// - River centerlines extraction
/// - Ridge and valley vectors

use serde_json::json;
use std::collections::HashMap;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use wbcore::{
    parse_optional_output_path, parse_raster_path_arg, parse_vector_path_arg, LicenseTier, Tool, ToolArgs, ToolCategory,
    ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata,
    ToolParamDescriptor, ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::Raster;
use wbvector::{Coord, Feature, Geometry, GeometryType, Layer, VectorFormat};
use crate::{
    memory_store,
    tools::{ElevationPercentileTool, SlopeTool, RemoveRasterPolygonHolesTool, ClosingTool},
};

pub struct PruneVectorStreamsTool;
pub struct RiverCenterlinesTool;
pub struct RidgeAndValleyVectorsTool;

fn build_result(path: String) -> ToolRunResult {
    let mut outputs = std::collections::BTreeMap::new();
    outputs.insert("path".to_string(), json!(path));
    ToolRunResult {
        outputs,
        ..Default::default()
    }
}

fn detect_vector_format(path: &str) -> Result<VectorFormat, ToolError> {
    VectorFormat::detect(path)
        .map_err(|e| ToolError::Validation(format!("could not determine vector format for '{}': {}", path, e)))
}

fn coord_distance(a: &Coord, b: &Coord) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

fn coord_distance_sq(a: &Coord, b: &Coord) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

fn line_length(line: &[Coord]) -> f64 {
    line.windows(2).map(|seg| coord_distance(&seg[0], &seg[1])).sum()
}

fn endpoint_key(coord: &Coord, snap_dist: f64) -> (i64, i64) {
    if snap_dist <= 0.0 {
        ((coord.x * 1.0e9).round() as i64, (coord.y * 1.0e9).round() as i64)
    } else {
        (
            (coord.x / snap_dist).round() as i64,
            (coord.y / snap_dist).round() as i64,
        )
    }
}

fn point_to_row_col(r: &Raster, x: f64, y: f64) -> Option<(isize, isize)> {
    let col = ((x - r.x_min) / r.cell_size_x).floor() as isize;
    let row = ((r.y_max() - y) / r.cell_size_y.abs()).floor() as isize;
    if row < 0 || col < 0 || row >= r.rows as isize || col >= r.cols as isize {
        None
    } else {
        Some((row, col))
    }
}

fn sample_dem_at_coord(dem: &Raster, coord: &Coord) -> Option<f64> {
    let (row, col) = point_to_row_col(dem, coord.x, coord.y)?;
    let z = dem.get(0, row, col);
    if dem.is_nodata(z) {
        None
    } else {
        Some(z)
    }
}

fn line_geometries(layer: &Layer) -> Vec<Vec<Coord>> {
    let mut lines = Vec::new();
    for feat in &layer.features {
        if let Some(geom) = &feat.geometry {
            match geom {
                Geometry::LineString(coords) => {
                    if coords.len() >= 2 {
                        lines.push(coords.clone());
                    }
                }
                Geometry::MultiLineString(parts) => {
                    for part in parts {
                        if part.len() >= 2 {
                            lines.push(part.clone());
                        }
                    }
                }
                _ => {}
            }
        }
    }
    lines
}

fn merge_centerline_segments(mut lines: Vec<Vec<Coord>>, connect_dist: f64) -> Vec<Vec<Coord>> {
    if lines.len() < 2 {
        return lines;
    }

    let mut changed = true;
    while changed {
        changed = false;
        'outer: for i in 0..lines.len() {
            if lines[i].len() < 2 {
                continue;
            }
            let a_start = lines[i].first().cloned().unwrap();
            let a_end = lines[i].last().cloned().unwrap();

            for j in (i + 1)..lines.len() {
                if lines[j].len() < 2 {
                    continue;
                }
                let b_start = lines[j].first().cloned().unwrap();
                let b_end = lines[j].last().cloned().unwrap();

                let d_es = coord_distance(&a_end, &b_start);
                let d_ee = coord_distance(&a_end, &b_end);
                let d_ss = coord_distance(&a_start, &b_start);
                let d_se = coord_distance(&a_start, &b_end);

                let mut best = (d_es, 0usize);
                if d_ee < best.0 {
                    best = (d_ee, 1);
                }
                if d_ss < best.0 {
                    best = (d_ss, 2);
                }
                if d_se < best.0 {
                    best = (d_se, 3);
                }

                if best.0 > connect_dist {
                    continue;
                }

                let mut left = lines[i].clone();
                let mut right = lines[j].clone();
                match best.1 {
                    0 => {}
                    1 => right.reverse(),
                    2 => {
                        left.reverse();
                    }
                    _ => {
                        left.reverse();
                        right.reverse();
                    }
                }

                if let (Some(l), Some(r)) = (left.last().cloned(), right.first().cloned()) {
                    if coord_distance(&l, &r) > 0.0 {
                        left.push(r.clone());
                    }
                }
                left.extend(right.into_iter().skip(1));

                lines[i] = left;
                lines.remove(j);
                changed = true;
                break 'outer;
            }
        }
    }

    lines
}

fn rc_in_bounds(row: isize, col: isize, rows: usize, cols: usize) -> bool {
    row >= 0 && col >= 0 && (row as usize) < rows && (col as usize) < cols
}

fn rc_index(row: usize, col: usize, cols: usize) -> usize {
    row * cols + col
}

fn rc_draw_line(mask: &mut [u8], rows: usize, cols: usize, r1: isize, c1: isize, r2: isize, c2: isize) {
    let mut x0 = c1;
    let mut y0 = r1;
    let x1 = c2;
    let y1 = r2;
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        if rc_in_bounds(y0, x0, rows, cols) {
            let i = rc_index(y0 as usize, x0 as usize, cols);
            mask[i] = 1;
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

fn rc_neighbor_offsets() -> [(isize, isize); 8] {
    [
        (0, 1), (-1, 1), (-1, 0), (-1, -1),
        (0, -1), (1, -1), (1, 0), (1, 1),
    ]
}

fn snap_line_endpoints(lines: &mut [Vec<Coord>], snap_dist: f64) {
    if snap_dist <= 0.0 {
        return;
    }
    let mut endpoints = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        endpoints.push((idx, true, line.first().cloned().unwrap()));
        endpoints.push((idx, false, line.last().cloned().unwrap()));
    }
    for i in 0..endpoints.len() {
        let (line_idx, is_start, coord) = endpoints[i].clone();
        let mut best = None;
        let mut best_dist = snap_dist;
        for (other_idx, _other_is_start, other) in &endpoints {
            if *other_idx == line_idx {
                continue;
            }
            let dist = coord_distance(&coord, other);
            if dist < best_dist {
                best_dist = dist;
                best = Some(other.clone());
            }
        }
        if let Some(snapped) = best {
            if is_start {
                lines[line_idx][0] = snapped;
            } else {
                let last = lines[line_idx].len() - 1;
                lines[line_idx][last] = snapped;
            }
        }
    }
}

fn collect_link_key_nodes(lines: &[Vec<Coord>], snap_dist: f64, precision_sq: f64) -> Vec<Vec<(i64, i64)>> {
    let mut endpoints = Vec::<(Coord, usize)>::new();
    for (i, line) in lines.iter().enumerate() {
        if line.len() < 2 {
            continue;
        }
        endpoints.push((line[0].clone(), i));
        endpoints.push((line.last().unwrap().clone(), i));
    }

    let mut key_nodes = Vec::with_capacity(lines.len());
    for (i, line) in lines.iter().enumerate() {
        if line.len() < 2 {
            key_nodes.push(Vec::new());
            continue;
        }
        let mut nodes = vec![
            endpoint_key(&line[0], snap_dist),
            endpoint_key(line.last().unwrap(), snap_dist),
        ];
        for p in line.iter().skip(1).take(line.len().saturating_sub(2)) {
            let mut match_other = false;
            for (ep, ep_id) in &endpoints {
                if *ep_id != i && coord_distance_sq(p, ep) <= precision_sq {
                    match_other = true;
                    break;
                }
            }
            if match_other {
                let k = endpoint_key(p, snap_dist);
                if !nodes.contains(&k) {
                    nodes.push(k);
                }
            }
        }
        key_nodes.push(nodes);
    }

    key_nodes
}

impl Tool for PruneVectorStreamsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "prune_vector_streams",
            display_name: "Prune Vector Streams",
            summary: "Prunes vector stream network based on Shreve magnitude.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "streams",
                    description: "Input vector stream network",
                    required: true,
                },
                ToolParamSpec {
                    name: "dem",
                    description: "Input DEM raster used to determine flow direction",
                    required: true,
                },
                ToolParamSpec {
                    name: "threshold",
                    description: "Minimum Shreve magnitude to retain",
                    required: true,
                },
                ToolParamSpec {
                    name: "snap_distance",
                    description: "Maximum snapping distance for endpoint connectivity",
                    required: false,
                },
                ToolParamSpec {
                    name: "max_ridge_cutting_height",
                    description: "Maximum elevation rise allowed when connecting downstream links",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output vector path",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("threshold".to_string(), json!(2.0));
        defaults.insert("snap_distance".to_string(), json!(0.001));
        defaults.insert("max_ridge_cutting_height".to_string(), json!(10.0));

        ToolManifest {
            id: "prune_vector_streams".to_string(),
            display_name: "Prune Vector Streams".to_string(),
            summary: "Prunes vector stream network based on Shreve magnitude.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![ToolExample {
                name: "prune_example".to_string(),
                description: "Prune streams with magnitude < 2".to_string(),
                args: ToolArgs::new(),
            }],
            tags: vec![
                "stream_network".to_string(),
                "vector".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        parse_vector_path_arg(args, "streams")
            .or_else(|_| parse_vector_path_arg(args, "input"))?;
        parse_raster_path_arg(args, "dem")
            .or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input = parse_vector_path_arg(args, "streams")
            .or_else(|_| parse_vector_path_arg(args, "input"))?;
        let dem_path = parse_raster_path_arg(args, "dem")
            .or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
        let threshold = args
            .get("threshold")
            .or_else(|| args.get("magnitude_threshold"))
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0);
        let snap_distance = args
            .get("snap_distance")
            .or_else(|| args.get("snap"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.001)
            .max(0.0);
        let max_ridge_cutting_height = args
            .get("max_ridge_cutting_height")
            .and_then(|v| v.as_f64())
            .unwrap_or(10.0)
            .max(0.0);
        let output_path = parse_optional_output_path(args, "output")?
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("{}_pruned.geojson", input));

        let src = wbvector::read(&input)
            .map_err(|e| ToolError::Execution(format!("failed reading input vector: {}", e)))?;
        let dem = Raster::read(&dem_path)
            .map_err(|e| ToolError::Execution(format!("failed reading DEM: {}", e)))?;

        #[derive(Clone)]
        struct LinkInfo {
            geom: Vec<Coord>,
            up_key: (i64, i64),
            down_key: (i64, i64),
            up_pt: Coord,
            down_pt: Coord,
            min_elev: f64,
            length: f64,
            crosses_nodata: bool,
            is_beyond_edge: bool,
        }

        #[derive(Clone, Copy, Debug)]
        struct QueueItem {
            index: usize,
            priority: f64,
        }

        impl PartialEq for QueueItem {
            fn eq(&self, other: &Self) -> bool {
                self.index == other.index && self.priority.to_bits() == other.priority.to_bits()
            }
        }

        impl Eq for QueueItem {}

        impl PartialOrd for QueueItem {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                // Reverse for min-heap behavior over BinaryHeap.
                other.priority.partial_cmp(&self.priority)
            }
        }

        impl Ord for QueueItem {
            fn cmp(&self, other: &Self) -> Ordering {
                self.partial_cmp(other).unwrap_or(Ordering::Equal)
            }
        }

        let mut links = Vec::<LinkInfo>::new();
        let mut working_lines = line_geometries(&src);
        snap_line_endpoints(&mut working_lines, snap_distance);
        working_lines = merge_centerline_segments(working_lines, snap_distance);

        for mut line in working_lines {
            let z_start = sample_dem_at_coord(&dem, &line[0]).unwrap_or(0.0);
            let z_end = sample_dem_at_coord(&dem, line.last().unwrap()).unwrap_or(0.0);
            if z_end > z_start {
                line.reverse();
            }
            let mut min_elev = f64::INFINITY;
            let mut has_data = false;
            let mut crosses_nodata = false;
            for c in &line {
                if let Some(z) = sample_dem_at_coord(&dem, c) {
                    has_data = true;
                    min_elev = min_elev.min(z);
                } else {
                    crosses_nodata = true;
                }
            }
            let is_beyond_edge = !has_data;
            if !min_elev.is_finite() {
                min_elev = z_start.min(z_end);
            }
            let length = line_length(&line);
            let up_pt = line[0].clone();
            let down_pt = line.last().unwrap().clone();
            let up_key = endpoint_key(&line[0], snap_distance);
            let down_key = endpoint_key(line.last().unwrap(), snap_distance);
            links.push(LinkInfo {
                geom: line,
                up_key,
                down_key,
                up_pt,
                down_pt,
                min_elev,
                length,
                crosses_nodata,
                is_beyond_edge,
            });
        }

        let geom_lines: Vec<Vec<Coord>> = links.iter().map(|l| l.geom.clone()).collect();
        let precision_sq = (f64::EPSILON * 10.0) * (f64::EPSILON * 10.0);
        let link_key_nodes = collect_link_key_nodes(&geom_lines, snap_distance, precision_sq);

        let mut endpoint_links: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
        let mut node_links: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
        let mut downstream_of: Vec<Option<usize>> = vec![None; links.len()];
        let mut upstream_of: Vec<Vec<usize>> = vec![Vec::new(); links.len()];

        for (idx, link) in links.iter().enumerate() {
            endpoint_links.entry(link.up_key).or_default().push(idx);
            endpoint_links.entry(link.down_key).or_default().push(idx);
            for node in &link_key_nodes[idx] {
                node_links.entry(*node).or_default().push(idx);
            }
        }

        let mut is_exterior_link = vec![false; links.len()];
        let mut is_outlet_link = vec![false; links.len()];
        let mut have_entered_queue = vec![false; links.len()];
        let mut have_visited = vec![false; links.len()];

        let mut queue: BinaryHeap<QueueItem> = BinaryHeap::new();

        for i in 0..links.len() {
            if links[i].is_beyond_edge {
                continue;
            }
            let mut is_exterior = false;
            let mut z_neigh = f64::INFINITY;
            let e1 = endpoint_links.get(&links[i].up_key).cloned().unwrap_or_default();
            let e2 = endpoint_links.get(&links[i].down_key).cloned().unwrap_or_default();

            let mut n1 = 0usize;
            for id in e1 {
                if id != i && !links[id].is_beyond_edge {
                    n1 += 1;
                    if links[id].min_elev < z_neigh {
                        z_neigh = links[id].min_elev;
                    }
                }
            }
            if n1 == 0 {
                is_exterior = true;
            }

            let mut n2 = 0usize;
            for id in e2 {
                if id != i && !links[id].is_beyond_edge {
                    n2 += 1;
                    if links[id].min_elev < z_neigh {
                        z_neigh = links[id].min_elev;
                    }
                }
            }
            if n2 == 0 {
                is_exterior = true;
            }

            if is_exterior || links[i].crosses_nodata {
                is_exterior_link[i] = true;
                if links[i].crosses_nodata || links[i].min_elev <= z_neigh {
                    queue.push(QueueItem {
                        index: i,
                        priority: links[i].min_elev + max_ridge_cutting_height,
                    });
                    have_entered_queue[i] = true;
                }
            }
        }

        let mut outlet_num_of = vec![0usize; links.len()];
        let mut current_max_outlet_num = 0usize;

        while let Some(item) = queue.pop() {
            let link = item.index;
            if have_visited[link] {
                continue;
            }
            have_visited[link] = true;

            if downstream_of[link].is_none() {
                if !links[link].crosses_nodata {
                    // Determine likely downstream endpoint for dangling links.
                    let mut up_open = 0usize;
                    if let Some(nbrs) = endpoint_links.get(&links[link].up_key) {
                        for id in nbrs {
                            if *id != link && !links[*id].is_beyond_edge && !have_visited[*id] && !is_outlet_link[*id] {
                                up_open += 1;
                            }
                        }
                    }
                    let down_pt = if up_open > 0 {
                        links[link].down_pt.clone()
                    } else {
                        let mut down_open = 0usize;
                        if let Some(nbrs) = endpoint_links.get(&links[link].down_key) {
                            for id in nbrs {
                                if *id != link && !links[*id].is_beyond_edge && !have_visited[*id] && !is_outlet_link[*id] {
                                    down_open += 1;
                                }
                            }
                        }
                        if down_open > 0 {
                            links[link].up_pt.clone()
                        } else {
                            let z1 = sample_dem_at_coord(&dem, &links[link].up_pt).unwrap_or(f64::INFINITY);
                            let z2 = sample_dem_at_coord(&dem, &links[link].down_pt).unwrap_or(f64::INFINITY);
                            if z1 <= z2 {
                                links[link].up_pt.clone()
                            } else {
                                links[link].down_pt.clone()
                            }
                        }
                    };

                    // Attempt snapping this dangling outlet to previously discovered exterior outlets.
                    let mut candidates: Vec<(usize, f64)> = Vec::new();
                    let snap_sq = snap_distance * snap_distance;
                    for id in 0..links.len() {
                        if !links[id].is_beyond_edge && have_visited[id] && is_exterior_link[id] && id != link {
                            let d1 = coord_distance_sq(&down_pt, &links[id].up_pt);
                            let d2 = coord_distance_sq(&down_pt, &links[id].down_pt);
                            let d = d1.min(d2);
                            candidates.push((id, d));
                        }
                    }

                    candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
                    let mut snapped: Option<usize> = None;
                    for (id, d) in candidates.into_iter().take(3) {
                        if d < snap_sq {
                            snapped = Some(id);
                            break;
                        }
                    }

                    if let Some(dsn) = snapped {
                        downstream_of[link] = Some(dsn);
                        outlet_num_of[link] = outlet_num_of[dsn];
                    } else {
                        current_max_outlet_num += 1;
                        outlet_num_of[link] = current_max_outlet_num;
                        is_outlet_link[link] = true;
                    }
                } else {
                    current_max_outlet_num += 1;
                    outlet_num_of[link] = current_max_outlet_num;
                    is_outlet_link[link] = true;
                }

                if outlet_num_of[link] == 0 {
                    current_max_outlet_num += 1;
                    outlet_num_of[link] = current_max_outlet_num;
                    is_outlet_link[link] = true;
                }
            }

            for node in &link_key_nodes[link] {
                let neighbors = node_links.get(node).cloned().unwrap_or_default();
                let mut num_unentered = 0usize;
                for id in &neighbors {
                    if !links[*id].is_beyond_edge && !have_entered_queue[*id] {
                        num_unentered += 1;
                    }
                }
                let _is_confluence = num_unentered > 1;
                for id in neighbors {
                    if links[id].is_beyond_edge || have_entered_queue[id] {
                        continue;
                    }
                    queue.push(QueueItem {
                        index: id,
                        priority: links[id].min_elev,
                    });
                    have_entered_queue[id] = true;
                    downstream_of[id] = Some(link);
                    outlet_num_of[id] = outlet_num_of[link];
                }
            }
        }

        // Fallback for disconnected non-beyond-edge links not reached by flood.
        for i in 0..links.len() {
            if links[i].is_beyond_edge || have_visited[i] {
                continue;
            }
            if downstream_of[i].is_none() {
                let mut best: Option<(usize, f64)> = None;
                let mut candidates = Vec::new();
                for node in &link_key_nodes[i] {
                    if let Some(v) = node_links.get(node) {
                        candidates.extend(v.iter().copied());
                    }
                }
                for c in candidates {
                    if c == i || links[c].is_beyond_edge {
                        continue;
                    }
                    if links[c].min_elev > links[i].min_elev + max_ridge_cutting_height {
                        continue;
                    }
                    let z = links[c].min_elev;
                    if let Some((_, bz)) = best {
                        if z < bz {
                            best = Some((c, z));
                        }
                    } else {
                        best = Some((c, z));
                    }
                }
                downstream_of[i] = best.map(|v| v.0);
            }
        }

        for (idx, ds) in downstream_of.iter().enumerate() {
            if let Some(d) = ds {
                upstream_of[*d].push(idx);
            }
        }

        // Legacy-style upstream accumulation used for pruning (TUCL-like).
        let mut link_mag = vec![0.0f64; links.len()];
        let mut max_upstream_length = vec![0.0f64; links.len()];
        let mut pending: Vec<usize> = upstream_of.iter().map(|u| u.len()).collect();
        let mut stack: Vec<usize> = (0..links.len()).filter(|&i| pending[i] == 0).collect();
        while let Some(idx) = stack.pop() {
            link_mag[idx] += links[idx].length;
            max_upstream_length[idx] += links[idx].length;
            if let Some(ds) = downstream_of[idx] {
                link_mag[ds] += link_mag[idx];
                if max_upstream_length[idx] > max_upstream_length[ds] {
                    max_upstream_length[ds] = max_upstream_length[idx];
                }
                pending[ds] -= 1;
                if pending[ds] == 0 {
                    stack.push(ds);
                }
            }
        }

        // Legacy-style tributary ID assignment, preserving main-stem branch by max link_mag.
        let mut trib_num = vec![0usize; links.len()];
        let outlets: Vec<usize> = (0..links.len()).filter(|&i| downstream_of[i].is_none()).collect();
        let mut current_trib = 0usize;
        let mut stack: Vec<usize> = Vec::new();
        for outlet in outlets {
            current_trib += 1;
            trib_num[outlet] = current_trib;
            stack.push(outlet);
        }
        while let Some(i) = stack.pop() {
            if upstream_of[i].is_empty() {
                continue;
            }
            let max_link = upstream_of[i]
                .iter()
                .copied()
                .max_by(|a, b| link_mag[*a].partial_cmp(&link_mag[*b]).unwrap_or(std::cmp::Ordering::Equal));
            for up in &upstream_of[i] {
                stack.push(*up);
                if Some(*up) == max_link {
                    trib_num[*up] = trib_num[i];
                } else {
                    current_trib += 1;
                    trib_num[*up] = current_trib;
                }
            }
        }

        let mut trib_tucl = vec![0.0f64; current_trib + 1];
        for i in 0..links.len() {
            let t = trib_num[i];
            if t > 0 && link_mag[i] > trib_tucl[t] {
                trib_tucl[t] = link_mag[i];
            }
        }

        let mut dst = Layer::new(src.name.clone()).with_geom_type(GeometryType::LineString);
        if let Some(epsg) = src.crs_epsg() {
            dst = dst.with_epsg(epsg);
        }
        let mut fid: u64 = 1;
        for (i, link) in links.into_iter().enumerate() {
            let t = trib_num[i];
            let score = if t > 0 { trib_tucl[t] } else { 0.0 };
            if score >= threshold {
                let mut f = Feature::with_geometry(fid, Geometry::line_string(link.geom), 0);
                f.fid = fid;
                dst.push(f);
                fid += 1;
            }
        }

        let fmt = detect_vector_format(&output_path)?;
        wbvector::write(&dst, &output_path, fmt)
            .map_err(|e| ToolError::Execution(format!("failed writing output vector: {}", e)))?;

        Ok(build_result(output_path))
    }
}

impl Tool for RiverCenterlinesTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "river_centerlines",
            display_name: "River Centerlines",
            summary: "Extracts river centerlines from water raster using medial axis.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "raster",
                    description: "Binary water/non-water raster",
                    required: true,
                },
                ToolParamSpec {
                    name: "min_length",
                    description: "Minimum centerline length in raster cells",
                    required: false,
                },
                ToolParamSpec {
                    name: "search_radius",
                    description: "Search radius (cells) used to connect nearby endpoints",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output centerline vector path",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "river_centerlines".to_string(),
            display_name: "River Centerlines".to_string(),
            summary: "Extracts river centerlines from water raster using medial axis.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults: ToolArgs::new(),
            examples: vec![],
            tags: vec!["stream_network".to_string(), "vector".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        parse_raster_path_arg(args, "raster")
            .or_else(|_| parse_raster_path_arg(args, "water_raster"))
            .or_else(|_| parse_raster_path_arg(args, "input"))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let water_path = parse_raster_path_arg(args, "raster")
            .or_else(|_| parse_raster_path_arg(args, "water_raster"))
            .or_else(|_| parse_raster_path_arg(args, "input"))?;
        let min_length = args
            .get("min_length")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(3)
            .max(2);
        let search_radius = args
            .get("search_radius")
            .or_else(|| args.get("radius"))
            .and_then(|v| v.as_i64())
            .unwrap_or(9)
            .max(1) as f64;
        let output_path = parse_optional_output_path(args, "output")?
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("{}_centerlines.geojson", water_path));

        let water = Raster::read(&water_path)
            .map_err(|e| ToolError::Execution(format!("failed reading water raster: {}", e)))?;

        let rows = water.rows;
        let cols = water.cols;
        let n = rows * cols;
        let nodata = water.nodata as f32;
        let mut dist = vec![nodata; n];
        let mut rx = vec![0.0f32; n];
        let mut ry = vec![0.0f32; n];

        for row in 0..rows {
            for col in 0..cols {
                let z = water.get(0, row as isize, col as isize);
                let i = rc_index(row, col, cols);
                dist[i] = if water.is_nodata(z) || z <= 0.0 { 0.0 } else { f32::INFINITY };
            }
        }

        let dx = [-1isize, -1, 0, 1, 1, 1, 0, -1];
        let dy = [0isize, -1, -1, -1, 0, 1, 1, 1];
        let gx = [1.0f32, 1.0, 0.0, 1.0, 1.0, 1.0, 0.0, 1.0];
        let gy = [0.0f32, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0];

        for row in 0..rows {
            for col in 0..cols {
                let i = rc_index(row, col, cols);
                let z = dist[i];
                if z == 0.0 || !z.is_finite() {
                    continue;
                }
                let mut z_min = f32::INFINITY;
                let mut which = 0usize;
                for k in 0..4 {
                    let rn = row as isize + dy[k];
                    let cn = col as isize + dx[k];
                    if !rc_in_bounds(rn, cn, rows, cols) {
                        continue;
                    }
                    let j = rc_index(rn as usize, cn as usize, cols);
                    let z2 = dist[j];
                    if z2 == nodata {
                        continue;
                    }
                    let h = match k {
                        0 => 2.0 * rx[j] + 1.0,
                        1 => 2.0 * (rx[j] + ry[j] + 1.0),
                        2 => 2.0 * ry[j] + 1.0,
                        _ => 2.0 * (rx[j] + ry[j] + 1.0),
                    };
                    let v = z2 + h;
                    if v < z_min {
                        z_min = v;
                        which = k;
                    }
                }
                if z_min < z {
                    dist[i] = z_min;
                    let rn = row as isize + dy[which];
                    let cn = col as isize + dx[which];
                    let j = rc_index(rn as usize, cn as usize, cols);
                    rx[i] = rx[j] + gx[which];
                    ry[i] = ry[j] + gy[which];
                }
            }
        }

        for row in (0..rows).rev() {
            for col in (0..cols).rev() {
                let i = rc_index(row, col, cols);
                let z = dist[i];
                if z == 0.0 || !z.is_finite() {
                    continue;
                }
                let mut z_min = f32::INFINITY;
                let mut which = 4usize;
                for k in 4..8 {
                    let rn = row as isize + dy[k];
                    let cn = col as isize + dx[k];
                    if !rc_in_bounds(rn, cn, rows, cols) {
                        continue;
                    }
                    let j = rc_index(rn as usize, cn as usize, cols);
                    let z2 = dist[j];
                    if z2 == nodata {
                        continue;
                    }
                    let h = match k {
                        4 => 2.0 * rx[j] + 1.0,
                        5 => 2.0 * (rx[j] + ry[j] + 1.0),
                        6 => 2.0 * ry[j] + 1.0,
                        _ => 2.0 * (rx[j] + ry[j] + 1.0),
                    };
                    let v = z2 + h;
                    if v < z_min {
                        z_min = v;
                        which = k;
                    }
                }
                if z_min < z {
                    dist[i] = z_min;
                    let rn = row as isize + dy[which];
                    let cn = col as isize + dx[which];
                    let j = rc_index(rn as usize, cn as usize, cols);
                    rx[i] = rx[j] + gx[which];
                    ry[i] = ry[j] + gy[which];
                }
            }
        }

        // Ridge-preserving centerline seed extraction from local distance maxima.
        let mut thin = vec![0u8; n];
        for row in 0..rows {
            for col in 0..cols {
                let i = rc_index(row, col, cols);
                let z = dist[i];
                if z <= 0.0 || !z.is_finite() {
                    continue;
                }
                let mut keep = true;
                for k in 0..8 {
                    let rn = row as isize + dy[k];
                    let cn = col as isize + dx[k];
                    if !rc_in_bounds(rn, cn, rows, cols) {
                        continue;
                    }
                    let zn = dist[rc_index(rn as usize, cn as usize, cols)];
                    if zn > z {
                        keep = false;
                        break;
                    }
                }
                if keep {
                    thin[i] = 1;
                }
            }
        }

        // Traditional iterative line thinning.
        let elements1 = [[6usize, 7, 0, 4, 3, 2], [0, 1, 2, 4, 5, 6], [2, 3, 4, 6, 7, 0], [4, 5, 6, 0, 1, 2]];
        let elements2 = [[7usize, 0, 1, 3, 5], [1, 2, 3, 5, 7], [3, 4, 5, 7, 1], [5, 6, 7, 1, 3]];
        let vals1 = [0u8, 0, 0, 1, 1, 1];
        let vals2 = [0u8, 0, 0, 1, 1];
        let nbs = rc_neighbor_offsets();
        let mut changed = true;
        while changed {
            changed = false;
            for a in 0..4 {
                let mut to_zero = Vec::new();
                for row in 0..rows {
                    for col in 0..cols {
                        let i = rc_index(row, col, cols);
                        if thin[i] != 1 {
                            continue;
                        }
                        let mut neigh = [0u8; 8];
                        for k in 0..8 {
                            let rn = row as isize + nbs[k].0;
                            let cn = col as isize + nbs[k].1;
                            if rc_in_bounds(rn, cn, rows, cols) {
                                neigh[k] = thin[rc_index(rn as usize, cn as usize, cols)];
                            }
                        }
                        let mut m1 = true;
                        for k in 0..6 {
                            if neigh[elements1[a][k]] != vals1[k] {
                                m1 = false;
                                break;
                            }
                        }
                        let mut m2 = true;
                        for k in 0..5 {
                            if neigh[elements2[a][k]] != vals2[k] {
                                m2 = false;
                                break;
                            }
                        }
                        if m1 || m2 {
                            to_zero.push(i);
                        }
                    }
                }
                if !to_zero.is_empty() {
                    changed = true;
                    for i in to_zero {
                        thin[i] = 0;
                    }
                }
            }
        }

        // Label components and remove tiny fragments.
        let mut label = vec![0u32; n];
        let mut next_label = 1u32;
        let mut comp_size = HashMap::<u32, usize>::new();
        for row in 0..rows {
            for col in 0..cols {
                let i = rc_index(row, col, cols);
                if thin[i] == 0 || label[i] != 0 {
                    continue;
                }
                let mut stack = vec![(row as isize, col as isize)];
                label[i] = next_label;
                let mut size = 0usize;
                while let Some((r, c)) = stack.pop() {
                    size += 1;
                    for (dr, dc) in &nbs {
                        let rn = r + *dr;
                        let cn = c + *dc;
                        if !rc_in_bounds(rn, cn, rows, cols) {
                            continue;
                        }
                        let j = rc_index(rn as usize, cn as usize, cols);
                        if thin[j] == 1 && label[j] == 0 {
                            label[j] = next_label;
                            stack.push((rn, cn));
                        }
                    }
                }
                comp_size.insert(next_label, size);
                next_label += 1;
            }
        }
        for i in 0..n {
            if thin[i] == 1 {
                let lbl = label[i];
                if comp_size.get(&lbl).copied().unwrap_or(0) < min_length {
                    thin[i] = 0;
                    label[i] = 0;
                }
            }
        }

        // Legacy-like braid fix: connect nearby disconnected branch tips and remove isolated artifacts.
        let braid_offsets: [(isize, isize); 16] = [
            (-2, -2), (-1, -2), (0, -2), (1, -2), (2, -2),
            (-2, -1),                             (2, -1),
            (-2, 0),                              (2, 0),
            (-2, 1),                              (2, 1),
            (-2, 2), (-1, 2), (0, 2), (1, 2), (2, 2),
        ];
        let braid_connector: [usize; 16] = [
            6, 7, 7, 7, 0,
            5,       1,
            5,       1,
            5,       1,
            4, 3, 3, 3, 2,
        ];
        for row in 0..rows {
            for col in 0..cols {
                let i = rc_index(row, col, cols);
                if thin[i] == 0 {
                    continue;
                }
                let z = label[i];
                if z == 0 {
                    continue;
                }
                let mut same = 0usize;
                let mut other = 0usize;
                for k in 0..8 {
                    let rn = row as isize + dy[k];
                    let cn = col as isize + dx[k];
                    if !rc_in_bounds(rn, cn, rows, cols) {
                        continue;
                    }
                    let j = rc_index(rn as usize, cn as usize, cols);
                    if thin[j] == 0 {
                        continue;
                    }
                    if label[j] == z {
                        same += 1;
                    } else {
                        other += 1;
                    }
                }

                if same == 1 && other == 0 {
                    for n in 0..braid_offsets.len() {
                        let rn = row as isize + braid_offsets[n].1;
                        let cn = col as isize + braid_offsets[n].0;
                        if !rc_in_bounds(rn, cn, rows, cols) {
                            continue;
                        }
                        let j = rc_index(rn as usize, cn as usize, cols);
                        if thin[j] == 1 && label[j] != z {
                            let br = row as isize + dy[braid_connector[n]];
                            let bc = col as isize + dx[braid_connector[n]];
                            if rc_in_bounds(br, bc, rows, cols) {
                                thin[rc_index(br as usize, bc as usize, cols)] = 1;
                            }
                            break;
                        }
                    }
                } else if same == 0 {
                    thin[i] = 0;
                }
            }
        }

        // Relabel after braid fix and endpoint connections.
        label.fill(0);
        comp_size.clear();
        next_label = 1;
        for row in 0..rows {
            for col in 0..cols {
                let i = rc_index(row, col, cols);
                if thin[i] == 0 || label[i] != 0 {
                    continue;
                }
                let mut stack = vec![(row as isize, col as isize)];
                label[i] = next_label;
                let mut size = 0usize;
                while let Some((r, c)) = stack.pop() {
                    size += 1;
                    for (dr, dc) in &nbs {
                        let rn = r + *dr;
                        let cn = c + *dc;
                        if !rc_in_bounds(rn, cn, rows, cols) {
                            continue;
                        }
                        let j = rc_index(rn as usize, cn as usize, cols);
                        if thin[j] == 1 && label[j] == 0 {
                            label[j] = next_label;
                            stack.push((rn, cn));
                        }
                    }
                }
                comp_size.insert(next_label, size);
                next_label += 1;
            }
        }

        // Connect nearby component endpoints within search radius.
        let search_cells = search_radius as isize;
        let mut endpoints = Vec::<(isize, isize, u32)>::new();
        for row in 0..rows {
            for col in 0..cols {
                let i = rc_index(row, col, cols);
                if thin[i] == 0 {
                    continue;
                }
                let mut deg = 0;
                for (dr, dc) in &nbs {
                    let rn = row as isize + *dr;
                    let cn = col as isize + *dc;
                    if rc_in_bounds(rn, cn, rows, cols) && thin[rc_index(rn as usize, cn as usize, cols)] == 1 {
                        deg += 1;
                    }
                }
                if deg == 1 {
                    endpoints.push((row as isize, col as isize, label[i]));
                }
            }
        }
        let mut used_ep = vec![false; endpoints.len()];
        for i in 0..endpoints.len() {
            if used_ep[i] {
                continue;
            }
            let (r1, c1, l1) = endpoints[i];
            let mut best: Option<(usize, f64)> = None;
            for j in (i + 1)..endpoints.len() {
                if used_ep[j] {
                    continue;
                }
                let (r2, c2, l2) = endpoints[j];
                if l1 == l2 {
                    continue;
                }
                let dr = r2 - r1;
                let dc = c2 - c1;
                if dr.abs() > search_cells || dc.abs() > search_cells {
                    continue;
                }
                let d = ((dr * dr + dc * dc) as f64).sqrt();
                if d > search_radius {
                    continue;
                }
                if let Some((_, bd)) = best {
                    if d < bd {
                        best = Some((j, d));
                    }
                } else {
                    best = Some((j, d));
                }
            }
            if let Some((j, _)) = best {
                let (r2, c2, _) = endpoints[j];
                rc_draw_line(&mut thin, rows, cols, r1, c1, r2, c2);
                used_ep[i] = true;
                used_ep[j] = true;
            }
        }

        // Vectorize thinned centerlines by tracing from endpoints.
        let mut visited = vec![false; n];
        let mut lines = Vec::<Vec<Coord>>::new();
        let get_neighbors = |r: isize, c: isize, thin: &[u8]| -> Vec<(isize, isize)> {
            let mut v = Vec::new();
            for (dr, dc) in &nbs {
                let rn = r + *dr;
                let cn = c + *dc;
                if rc_in_bounds(rn, cn, rows, cols) && thin[rc_index(rn as usize, cn as usize, cols)] == 1 {
                    v.push((rn, cn));
                }
            }
            v
        };

        for row in 0..rows {
            for col in 0..cols {
                let i = rc_index(row, col, cols);
                if thin[i] == 0 || visited[i] {
                    continue;
                }
                let neigh = get_neighbors(row as isize, col as isize, &thin);
                if neigh.len() != 1 {
                    continue;
                }

                let mut coords = Vec::<Coord>::new();
                let mut cur = (row as isize, col as isize);
                let mut prev: Option<(isize, isize)> = None;
                loop {
                    let ci = rc_index(cur.0 as usize, cur.1 as usize, cols);
                    if visited[ci] {
                        break;
                    }
                    visited[ci] = true;
                    coords.push(Coord::xy(water.col_center_x(cur.1), water.row_center_y(cur.0)));

                    let mut nexts = get_neighbors(cur.0, cur.1, &thin);
                    if let Some(p) = prev {
                        nexts.retain(|n| *n != p);
                    }
                    if nexts.len() != 1 {
                        break;
                    }
                    prev = Some(cur);
                    cur = nexts[0];
                }
                if coords.len() >= min_length {
                    lines.push(coords);
                }
            }
        }

        // Trace remaining unvisited cells (e.g., loops or braided fragments without endpoints).
        for row in 0..rows {
            for col in 0..cols {
                let i = rc_index(row, col, cols);
                if thin[i] == 0 || visited[i] {
                    continue;
                }
                let mut coords = Vec::<Coord>::new();
                let mut cur = (row as isize, col as isize);
                let mut prev: Option<(isize, isize)> = None;
                let start = cur;
                loop {
                    let ci = rc_index(cur.0 as usize, cur.1 as usize, cols);
                    if visited[ci] {
                        break;
                    }
                    visited[ci] = true;
                    coords.push(Coord::xy(water.col_center_x(cur.1), water.row_center_y(cur.0)));

                    let mut nexts = get_neighbors(cur.0, cur.1, &thin);
                    if let Some(p) = prev {
                        nexts.retain(|n| *n != p);
                    }
                    if nexts.is_empty() {
                        break;
                    }
                    prev = Some(cur);
                    cur = nexts[0];
                    if cur == start {
                        let si = rc_index(start.0 as usize, start.1 as usize, cols);
                        if !visited[si] {
                            coords.push(Coord::xy(water.col_center_x(start.1), water.row_center_y(start.0)));
                            visited[si] = true;
                        }
                        break;
                    }
                }
                if coords.len() >= min_length {
                    lines.push(coords);
                }
            }
        }

        let connect_dist = search_radius * water.cell_size_x.abs().max(water.cell_size_y.abs());
        lines = merge_centerline_segments(lines, connect_dist);

        let mut layer = Layer::new("river_centerlines").with_geom_type(GeometryType::LineString);
        if let Some(epsg) = water.crs.epsg {
            layer = layer.with_epsg(epsg);
        }
        let mut fid: u64 = 1;
        for coords in lines {
            if coords.len() < min_length {
                continue;
            }
            let mut f = Feature::with_geometry(fid, Geometry::line_string(coords), 0);
            f.fid = fid;
            layer.push(f);
            fid += 1;
        }

        if let Some(parent) = std::path::Path::new(&output_path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| ToolError::Execution(format!("failed creating output directory: {}", e)))?;
            }
        }

        let fmt = detect_vector_format(&output_path)?;
        wbvector::write(&layer, &output_path, fmt)
            .map_err(|e| ToolError::Execution(format!("failed writing centerline vector: {}", e)))?;

        Ok(build_result(output_path))
    }
}

// ── RidgeAndValleyVectorsTool ─────────────────────────────────────────────────

fn get_path_from_result(result: &ToolRunResult) -> Result<String, ToolError> {
    result
        .outputs
        .get("path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| ToolError::Execution("sub-tool returned no path".to_string()))
}

fn raster_mem(r: Raster) -> String {
    let id = memory_store::put_raster(r);
    memory_store::make_raster_memory_path(&id)
}

fn load_raster_mem(path: &str, label: &str) -> Result<Raster, ToolError> {
    if memory_store::raster_is_memory_path(path) {
        let id = memory_store::raster_path_to_id(path)
            .ok_or_else(|| ToolError::Execution(format!("invalid memory path for '{label}'")))?;
        memory_store::get_raster_by_id(id)
            .ok_or_else(|| ToolError::Execution(format!("memory raster not found for '{label}'")))
    } else {
        Raster::read(std::path::Path::new(path))
            .map_err(|e| ToolError::Execution(format!("failed reading '{label}': {e}")))
    }
}

impl Tool for RidgeAndValleyVectorsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "ridge_and_valley_vectors",
            display_name: "Ridge and Valley Vectors",
            summary: "Extracts ridge and valley centreline vector layers from a DEM using elevation percentile and slope thresholding.",
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "dem", description: "Input DEM raster.", required: true },
                ToolParamSpec { name: "filter_size", description: "Odd neighbourhood size in cells for elevation percentile calculation (default 11).", required: false },
                ToolParamSpec { name: "ep_threshold", description: "Elevation percentile threshold in [5, 50]. Ridges are cells > (100 - threshold), valleys are cells < threshold (default 30.0).", required: false },
                ToolParamSpec { name: "slope_threshold", description: "Minimum slope in degrees; cells below this are excluded (default 0.0).", required: false },
                ToolParamSpec { name: "min_length", description: "Minimum centreline segment length in cells (default 20).", required: false },
                ToolParamSpec { name: "output_ridges", description: "Optional output path for ridge centrelines (GeoJSON). If omitted, auto-named.", required: false },
                ToolParamSpec { name: "output_valleys", description: "Optional output path for valley centrelines (GeoJSON). If omitted, auto-named.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("dem".to_string(), json!("dem.tif"));
        defaults.insert("filter_size".to_string(), json!(11));
        defaults.insert("ep_threshold".to_string(), json!(30.0));
        defaults.insert("slope_threshold".to_string(), json!(0.0));
        defaults.insert("min_length".to_string(), json!(20));

        let mut example_args = defaults.clone();
        example_args.insert("output_ridges".to_string(), json!("ridges.geojson"));
        example_args.insert("output_valleys".to_string(), json!("valleys.geojson"));

        ToolManifest {
            id: "ridge_and_valley_vectors".to_string(),
            display_name: "Ridge and Valley Vectors".to_string(),
            summary: "Extracts ridge and valley centreline vectors from a DEM.".to_string(),
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "dem".to_string(), description: "Input DEM raster.".to_string(), required: true },
                ToolParamDescriptor { name: "filter_size".to_string(), description: "Neighbourhood size for elevation percentile.".to_string(), required: false },
                ToolParamDescriptor { name: "ep_threshold".to_string(), description: "Elevation percentile threshold [5, 50].".to_string(), required: false },
                ToolParamDescriptor { name: "slope_threshold".to_string(), description: "Minimum slope threshold in degrees.".to_string(), required: false },
                ToolParamDescriptor { name: "min_length".to_string(), description: "Minimum centreline length in cells.".to_string(), required: false },
                ToolParamDescriptor { name: "output_ridges".to_string(), description: "Output ridge centrelines path.".to_string(), required: false },
                ToolParamDescriptor { name: "output_valleys".to_string(), description: "Output valley centrelines path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "ridge_valley_basic".to_string(),
                description: "Extract ridge and valley vectors from a DEM.".to_string(),
                args: example_args,
            }],
            tags: vec!["geomorphometry".to_string(), "ridges".to_string(), "valleys".to_string(), "vector".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        parse_raster_path_arg(args, "dem")
            .or_else(|_| parse_raster_path_arg(args, "input"))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let dem_path = parse_raster_path_arg(args, "dem")
            .or_else(|_| parse_raster_path_arg(args, "input"))?;
        let filter_size = args.get("filter_size").and_then(|v| v.as_u64()).unwrap_or(11) as usize;
        let mut ep_threshold = args.get("ep_threshold").and_then(|v| v.as_f64()).unwrap_or(30.0);
        ep_threshold = ep_threshold.clamp(5.0, 50.0);
        let slope_threshold = args.get("slope_threshold").and_then(|v| v.as_f64()).unwrap_or(0.0).max(0.0);
        let min_length = args.get("min_length").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
        let ridges_path = args.get("output_ridges").and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{}_ridges.geojson", dem_path));
        let valleys_path = args.get("output_valleys").and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{}_valleys.geojson", dem_path));

        // Step 1: Compute elevation percentile
        ctx.progress.info("ridge_and_valley_vectors: computing elevation percentile");
        let mut a = ToolArgs::new();
        a.insert("input".to_string(), json!(dem_path));
        a.insert("filter_size_x".to_string(), json!(filter_size));
        a.insert("filter_size_y".to_string(), json!(filter_size));
        a.insert("sig_digits".to_string(), json!(2));
        let r = ElevationPercentileTool.run(&a, ctx)?;
        let ep_path = get_path_from_result(&r)?;

        // Step 2: Compute slope
        ctx.progress.info("ridge_and_valley_vectors: computing slope");
        let mut a = ToolArgs::new();
        a.insert("input".to_string(), json!(dem_path));
        let r = SlopeTool.run(&a, ctx)?;
        let slope_path = get_path_from_result(&r)?;

        // Step 3: Build ridge and valley binary rasters
        ctx.progress.info("ridge_and_valley_vectors: building ridge/valley masks");
        let ep = load_raster_mem(&ep_path, "ep")?;
        let slope = load_raster_mem(&slope_path, "slope")?;
        let rows = ep.rows;
        let cols = ep.cols;
        let band_stride = rows * cols;

        let ridge_threshold = 100.0 - ep_threshold;
        let mut ridge_raster = ep.clone();
        let mut valley_raster = ep.clone();
        ridge_raster.nodata = -1.0;
        valley_raster.nodata = -1.0;

        for i in 0..band_stride {
            let ep_v = ep.data.get_f64(i);
            let sl_v = slope.data.get_f64(i);
            if ep.is_nodata(ep_v) || slope.is_nodata(sl_v) {
                ridge_raster.data.set_f64(i, -1.0);
                valley_raster.data.set_f64(i, -1.0);
                continue;
            }
            ridge_raster.data.set_f64(i, if ep_v > ridge_threshold && sl_v > slope_threshold { 1.0 } else { 0.0 });
            valley_raster.data.set_f64(i, if ep_v < ep_threshold && sl_v > slope_threshold { 1.0 } else { 0.0 });
        }

        let ridge_mem = raster_mem(ridge_raster);
        let valley_mem = raster_mem(valley_raster);

        // Step 4: Remove small holes, then morphological closing, then centrelines — for ridges
        ctx.progress.info("ridge_and_valley_vectors: extracting ridge centrelines");
        let ridge_vec_path = run_ridge_valley_pipeline(ctx, &ridge_mem, min_length, &ridges_path)?;

        // Step 5: Same pipeline for valleys
        ctx.progress.info("ridge_and_valley_vectors: extracting valley centrelines");
        let valley_vec_path = run_ridge_valley_pipeline(ctx, &valley_mem, min_length, &valleys_path)?;

        let mut outputs = std::collections::BTreeMap::new();
        outputs.insert("ridges_path".to_string(), json!(ridge_vec_path));
        outputs.insert("valleys_path".to_string(), json!(valley_vec_path));
        // Also expose as "path" (ridges) for single-output consumers
        outputs.insert("path".to_string(), json!(ridges_path));
        Ok(ToolRunResult { outputs, ..Default::default() })
    }
}

fn run_ridge_valley_pipeline(
    ctx: &ToolContext,
    binary_mem: &str,
    min_length: usize,
    output_path: &str,
) -> Result<String, ToolError> {
    // Remove small holes
    let mut a = ToolArgs::new();
    a.insert("input".to_string(), json!(binary_mem));
    a.insert("threshold".to_string(), json!(10));
    a.insert("use_diagonals".to_string(), json!(true));
    let r = RemoveRasterPolygonHolesTool.run(&a, ctx)?;
    let no_holes_path = r.outputs.get("path")
        .and_then(|v: &serde_json::Value| v.as_str())
        .ok_or_else(|| ToolError::Execution("remove_holes returned no path".to_string()))?.to_string();

    // Morphological closing (simplify shapes)
    let mut a = ToolArgs::new();
    a.insert("input".to_string(), json!(no_holes_path));
    a.insert("filter_size_x".to_string(), json!(5));
    a.insert("filter_size_y".to_string(), json!(5));
    let r = ClosingTool.run(&a, ctx)?;
    let closed_path = r.outputs.get("path")
        .and_then(|v: &serde_json::Value| v.as_str())
        .ok_or_else(|| ToolError::Execution("closing returned no path".to_string()))?.to_string();

    // Extract centrelines via RiverCenterlinesTool
    let mut a = ToolArgs::new();
    a.insert("raster".to_string(), json!(closed_path));
    a.insert("min_length".to_string(), json!(min_length));
    a.insert("search_radius".to_string(), json!(9));
    a.insert("output".to_string(), json!(output_path));
    let r = RiverCenterlinesTool.run(&a, ctx)?;
    r.outputs.get("path").and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| ToolError::Execution("river_centerlines returned no path".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_centerline_segments_connects_nearby_ends() {
        let a = vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0)];
        let b = vec![Coord::xy(1.0005, 0.0), Coord::xy(2.0, 0.0)];
        let merged = merge_centerline_segments(vec![a, b], 0.01);
        assert_eq!(merged.len(), 1);
        assert!(merged[0].len() >= 3);
    }

    #[test]
    fn endpoint_key_respects_snap_quantization() {
        let p1 = Coord::xy(10.004, 20.004);
        let p2 = Coord::xy(10.0049, 20.0049);
        let k1 = endpoint_key(&p1, 0.01);
        let k2 = endpoint_key(&p2, 0.01);
        assert_eq!(k1, k2);
    }

    #[test]
    fn rc_draw_line_marks_expected_cells() {
        let rows = 5usize;
        let cols = 5usize;
        let mut mask = vec![0u8; rows * cols];
        rc_draw_line(&mut mask, rows, cols, 1, 1, 3, 3);
        assert_eq!(mask[rc_index(1, 1, cols)], 1);
        assert_eq!(mask[rc_index(2, 2, cols)], 1);
        assert_eq!(mask[rc_index(3, 3, cols)], 1);
    }

    #[test]
    fn collect_link_key_nodes_detects_y_junction_interior_vertex() {
        let trunk = vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(1.0, 0.0),
            Coord::xy(2.0, 0.0),
        ];
        let tributary = vec![Coord::xy(1.0, 0.0), Coord::xy(1.0, 1.0)];
        let nodes = collect_link_key_nodes(&[trunk, tributary], 0.001, 1.0e-12);
        let j_key = endpoint_key(&Coord::xy(1.0, 0.0), 0.001);

        assert!(nodes[0].contains(&j_key));
        assert!(nodes[1].contains(&j_key));
    }

    #[test]
    fn collect_link_key_nodes_uses_precision_threshold() {
        let trunk = vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(1.0 + 1.0e-8, 0.0),
            Coord::xy(2.0, 0.0),
        ];
        let tributary = vec![Coord::xy(1.0, 0.0), Coord::xy(1.0, 1.0)];
        let nodes = collect_link_key_nodes(&[trunk, tributary], 0.001, 1.0e-12);
        let j_key = endpoint_key(&Coord::xy(1.0 + 1.0e-8, 0.0), 0.001);

        assert!(nodes[0].contains(&j_key));
    }
}
