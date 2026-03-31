use crate::raster::ShapeRecord;
use crate::spatial_index::KdTree;
use crate::vector::Shapefile;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use wblidar::{LasFile, PointData, Transform3D};

/// Individual Tree Detection Tool
/// 
/// Identifies tree tops (local maxima) in a LiDAR point cloud.
/// Points are determined to be tree tops if they are the highest point
/// within a height-dependent search radius.
pub struct IndividualTreeDetectionTool {
    pub name: String,
    pub description: String,
}

impl Default for IndividualTreeDetectionTool {
    fn default() -> Self {
        Self::new()
    }
}

impl IndividualTreeDetectionTool {
    pub fn new() -> Self {
        IndividualTreeDetectionTool {
            name: "individual_tree_detection".to_string(),
            description: "Identifies tree top points in a LiDAR cloud using local maxima detection."
                .to_string(),
        }
    }

    /// Run the individual tree detection algorithm.
    ///
    /// # Parameters
    ///
    /// - `input`: Input LAS/LAZ file
    /// - `min_search_radius`: Minimum search radius in map units (default 1.0)
    /// - `min_height`: Minimum height to consider points (default 0.0)
    /// - `max_search_radius`: Maximum search radius (if None, uses min_search_radius)
    /// - `max_height`: Maximum height (if None, uses min_height)
    /// - `only_use_veg`: Only process vegetation class points (default true)
    ///
    /// # Returns
    ///
    /// A vector of tree top points with attributes:
    /// - FID: original point index + 1
    /// - Z: height value
    pub fn run(
        &self,
        input_path: &str,
        min_search_radius: f64,
        min_height: f64,
        max_search_radius: Option<f64>,
        max_height: Option<f64>,
        only_use_veg: bool,
    ) -> Result<Shapefile, String> {
        let max_search_radius = max_search_radius.unwrap_or(min_search_radius);
        let max_height = max_height.unwrap_or(min_height);

        if min_search_radius <= 0.0 || max_search_radius <= 0.0 {
            return Err("Search radius parameters must be larger than zero.".to_string());
        }

        let radius_range = max_search_radius - min_search_radius;
        let height_range = max_height - min_height;

        // Read input LiDAR
        let mut input = LasFile::read(input_path)
            .map_err(|e| format!("Failed to read input LiDAR: {}", e))?;

        let n_points = input.header.number_of_point_records as usize;

        // Filter and build KdTree on eligible points
        let mut points: Vec<(usize, f64, f64)> = Vec::new();
        let transform = input.get_transform();

        for i in 0..n_points {
            let point = input.get_point_record(i);

            // Check eligibility
            if point.withheld_flag() || point.synthetic_flag() || point.is_noise_classification() {
                continue;
            }

            if only_use_veg {
                // LAS vegetation classes: 3 (low vegetation), 4 (medium vegetation), 5 (high vegetation)
                if !matches!(point.classification(), 3 | 4 | 5) {
                    continue;
                }
            }

            let coords = transform.point_to_3d(point);
            points.push((i, coords.x, coords.y));
        }

        if points.is_empty() {
            return Err(if only_use_veg {
                "No vegetation points found. Try setting only_use_veg=false.".to_string()
            } else {
                "No eligible points found.".to_string()
            });
        }

        // Build KdTree with (x, y) coordinates for 2D spatial queries
        let kdtree = KdTree::new(&points);

        // Create output shapefile
        let mut output = Shapefile::new(
            "",
            crate::vector::ShapefileGeometryType::Point,
        ).map_err(|e| format!("Failed to create output shapefile: {}", e))?;

        output.projection = input.get_projection_wkt();

        // Add attributes
        output.add_field(
            "FID",
            crate::vector::FieldDataType::Integer,
            7,
            0,
        ).map_err(|e| format!("Failed to add FID field: {}", e))?;

        output.add_field(
            "Z",
            crate::vector::FieldDataType::Real,
            12,
            5,
        ).map_err(|e| format!("Failed to add Z field: {}", e))?;

        // Identify tree tops
        let num_procs = num_cpus::get();
        let input = Arc::new(input);
        let kdtree = Arc::new(kdtree);
        let (tx, rx) = mpsc::channel();

        for tid in 0..num_procs {
            let input = Arc::clone(&input);
            let kdtree = Arc::clone(&kdtree);
            let tx = tx.clone();
            let points_clone = points.clone();

            thread::spawn(move || {
                for (idx, &(point_idx, x, y)) in points_clone
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| i % num_procs == tid)
                {
                    let point = input.get_point_record(point_idx);
                    let transform = input.get_transform();
                    let coords = transform.point_to_3d(point);

                    if coords.z < min_height {
                        tx.send((idx, point_idx, false, coords.z)).unwrap();
                        continue;
                    }

                    // Calculate search radius based on height
                    let radius = if coords.z < min_height {
                        min_search_radius
                    } else if coords.z > max_height {
                        max_search_radius
                    } else if height_range > 0.0 {
                        min_search_radius + (coords.z - min_height) / height_range * radius_range
                    } else {
                        min_search_radius
                    };

                    // Find neighbors within search radius
                    let neighbors = kdtree.within_radius(x, y, radius);

                    // Check if this point is the highest in its neighborhood
                    let mut is_highest = true;
                    for &(_, neighbor_idx, _, _) in &neighbors {
                        if neighbor_idx != point_idx {
                            let neighbor = input.get_point_record(neighbor_idx);
                            let neighbor_coords = transform.point_to_3d(neighbor);
                            if neighbor_coords.z > coords.z {
                                is_highest = false;
                                break;
                            }
                        }
                    }

                    tx.send((idx, point_idx, is_highest, coords.z)).unwrap();
                }
            });
        }

        drop(tx);

        // Collect results
        let mut tree_tops: Vec<(usize, usize, f64)> = Vec::new();
        for (idx, point_idx, is_highest, z) in rx {
            if is_highest {
                tree_tops.push((idx, point_idx, z));
            }
        }

        // Sort by original point index for consistent ordering
        tree_tops.sort_by_key(|t| t.1);

        // Add tree top records to output
        for (_, point_idx, z) in tree_tops {
            let point = input.get_point_record(point_idx);
            let transform = input.get_transform();
            let coords = transform.point_to_3d(point);

            // Add point to output
            let shape_record = ShapeRecord::Point {
                x: coords.x,
                y: coords.y,
            };

            output
                .add_shape(shape_record)
                .map_err(|e| format!("Failed to add shape: {}", e))?;

            // Add attributes: FID (1-based index) and Z
            output
                .add_record(vec![
                    crate::vector::FieldValue::Integer(point_idx as i32 + 1),
                    crate::vector::FieldValue::Real(z),
                ])
                .map_err(|e| format!("Failed to add record: {}", e))?;
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_individual_tree_detection_creation() {
        let tool = IndividualTreeDetectionTool::new();
        assert_eq!(tool.name, "individual_tree_detection");
    }
}
