use std::env;
use std::path::{Path, PathBuf};

use image::imageops::FilterType;
use image::{Rgb, RgbImage};
use wbphotogrammetry::{
    FeatureMethod,
    ingest_image_set,
    MatchStats,
    run_feature_matching_with_method,
};

fn main() {
    let args: Vec<String> = env::args().collect();
    let images_dir = get_arg(&args, "--images-dir").unwrap_or_else(|| {
        eprintln!("missing --images-dir <path>");
        std::process::exit(2);
    });
    let left_name = get_arg(&args, "--left").unwrap_or_else(|| {
        eprintln!("missing --left <filename>");
        std::process::exit(2);
    });
    let right_name = get_arg(&args, "--right").unwrap_or_else(|| {
        eprintln!("missing --right <filename>");
        std::process::exit(2);
    });
    let profile = get_arg(&args, "--profile").unwrap_or_else(|| "balanced".to_string());
    let requested_method = get_arg(&args, "--method")
        .map(|value| value.parse::<FeatureMethod>())
        .transpose()
        .unwrap_or_else(|e| {
            eprintln!("invalid --method: {e}");
            std::process::exit(2);
        });
    let viz_dir = get_arg(&args, "--viz-dir").map(PathBuf::from);

    let frames = ingest_image_set(&images_dir).unwrap_or_else(|e| {
        eprintln!("failed to ingest image set: {e}");
        std::process::exit(1);
    });

    let left = frames
        .iter()
        .find(|f| f.path.ends_with(&left_name))
        .cloned()
        .unwrap_or_else(|| {
            eprintln!("left image not found in dataset: {left_name}");
            std::process::exit(2);
        });
    let right = frames
        .iter()
        .find(|f| f.path.ends_with(&right_name))
        .cloned()
        .unwrap_or_else(|| {
            eprintln!("right image not found in dataset: {right_name}");
            std::process::exit(2);
        });

    let pair = vec![left, right];

    println!("Feature Pair Probe");
    println!("images_dir: {images_dir}");
    println!("left: {left_name}");
    println!("right: {right_name}");
    println!("profile: {profile}");
    println!();

    let mut results = Vec::new();
    let methods: Vec<FeatureMethod> = if let Some(method) = requested_method {
        vec![method]
    } else {
        vec![
            FeatureMethod::Brief,
            FeatureMethod::Orb,
            FeatureMethod::Sift,
            FeatureMethod::RootSift,
        ]
    };

    for method in methods {
        let stats = run_feature_matching_with_method(&pair, &profile, method).unwrap_or_else(|e| {
            eprintln!("{} matcher failed: {e}", method.as_str());
            std::process::exit(1);
        });
        print_stats(&method.as_str().to_ascii_uppercase(), &stats);
        println!();
        results.push((method, stats));
    }

    if let Some(dir) = viz_dir {
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!("failed creating --viz-dir '{}': {e}", dir.display());
            std::process::exit(1);
        }

        let left_path = Path::new(&pair[0].path);
        let right_path = Path::new(&pair[1].path);
        let stem = format!(
            "{}_{}",
            sanitize_name(&left_name),
            sanitize_name(&right_name)
        );

        println!();
        println!("[visualizations]");
        for (method, stats) in &results {
            let output_path = dir.join(format!("{stem}_{}_matches.png", method.as_str()));
            write_match_visualization(left_path, right_path, stats, &output_path).unwrap_or_else(|e| {
                eprintln!("failed writing {} visualization '{}': {e}", method.as_str(), output_path.display());
                std::process::exit(1);
            });
            println!("  {}: {}", method.as_str().to_ascii_uppercase(), output_path.display());
        }
    }
}

fn write_match_visualization(
    left_path: &Path,
    right_path: &Path,
    stats: &MatchStats,
    output_path: &Path,
) -> Result<(), String> {
    const MAX_VIZ_IMAGE_DIM_PX: u32 = 1800;

    let left_dyn = image::open(left_path)
        .map_err(|e| format!("failed to read left image '{}': {e}", left_path.display()))?;
    let right_dyn = image::open(right_path)
        .map_err(|e| format!("failed to read right image '{}': {e}", right_path.display()))?;

    let (left_img, left_scale) = resize_for_visualization(left_dyn, MAX_VIZ_IMAGE_DIM_PX);
    let (right_img, right_scale) = resize_for_visualization(right_dyn, MAX_VIZ_IMAGE_DIM_PX);

    let mut canvas = compose_side_by_side(&left_img, &right_img);

    let Some(pair) = stats.pair_correspondences.first() else {
        return canvas
            .save(output_path)
            .map_err(|e| format!("failed to save '{}': {e}", output_path.display()));
    };

    let x_offset = left_img.width() as i32;
    for (idx, p) in pair.points.iter().enumerate() {
        let x0 = (p[0] * left_scale).round() as i32;
        let y0 = (p[1] * left_scale).round() as i32;
        let x1 = (p[2] * right_scale).round() as i32 + x_offset;
        let y1 = (p[3] * right_scale).round() as i32;

        let line_color = vivid_color_for_index(idx);
        draw_bold_line(&mut canvas, x0, y0, x1, y1, 2, line_color);
        draw_cross(&mut canvas, x0, y0, 2, Rgb([255, 255, 0]));
        draw_cross(&mut canvas, x1, y1, 2, Rgb([255, 255, 0]));
    }

    canvas
        .save(output_path)
        .map_err(|e| format!("failed to save '{}': {e}", output_path.display()))
}

fn resize_for_visualization(image: image::DynamicImage, max_dim: u32) -> (RgbImage, f64) {
    let src_w = image.width().max(1);
    let src_h = image.height().max(1);
    let largest = src_w.max(src_h);

    if largest <= max_dim {
        return (image.to_rgb8(), 1.0);
    }

    let scale = max_dim as f64 / largest as f64;
    let dst_w = ((src_w as f64) * scale).round().max(1.0) as u32;
    let dst_h = ((src_h as f64) * scale).round().max(1.0) as u32;
    let resized = image::imageops::resize(&image.to_rgb8(), dst_w, dst_h, FilterType::Triangle);
    (resized, scale)
}

fn compose_side_by_side(left: &RgbImage, right: &RgbImage) -> RgbImage {
    let width = left.width() + right.width();
    let height = left.height().max(right.height());
    let mut canvas = RgbImage::new(width, height);

    for y in 0..left.height() {
        for x in 0..left.width() {
            let px = *left.get_pixel(x, y);
            canvas.put_pixel(x, y, px);
        }
    }
    for y in 0..right.height() {
        for x in 0..right.width() {
            let px = *right.get_pixel(x, y);
            canvas.put_pixel(x + left.width(), y, px);
        }
    }

    canvas
}

fn draw_line(image: &mut RgbImage, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: Rgb<u8>) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        put_pixel_safe(image, x0, y0, color);
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

fn draw_bold_line(
    image: &mut RgbImage,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    half_width: i32,
    color: Rgb<u8>,
) {
    for dx in -half_width..=half_width {
        for dy in -half_width..=half_width {
            if (dx * dx + dy * dy) <= (half_width * half_width) {
                draw_line(image, x0 + dx, y0 + dy, x1 + dx, y1 + dy, color);
            }
        }
    }
}

fn draw_cross(image: &mut RgbImage, x: i32, y: i32, radius: i32, color: Rgb<u8>) {
    for delta in -radius..=radius {
        put_pixel_safe(image, x + delta, y, color);
        put_pixel_safe(image, x, y + delta, color);
    }
}

fn put_pixel_safe(image: &mut RgbImage, x: i32, y: i32, color: Rgb<u8>) {
    if x < 0 || y < 0 {
        return;
    }
    if x >= image.width() as i32 || y >= image.height() as i32 {
        return;
    }
    image.put_pixel(x as u32, y as u32, color);
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn vivid_color_for_index(index: usize) -> Rgb<u8> {
    let mut state = (index as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let mut next_unit = || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let r = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
        (r as f64) / (u64::MAX as f64)
    };

    let hue = next_unit() * 360.0;
    let sat = 0.78 + next_unit() * 0.20;
    let val = 0.82 + next_unit() * 0.16;
    hsv_to_rgb(hue, sat.min(1.0), val.min(1.0))
}

fn hsv_to_rgb(hue_deg: f64, saturation: f64, value: f64) -> Rgb<u8> {
    let c = value * saturation;
    let h_prime = (hue_deg / 60.0) % 6.0;
    let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());
    let (r1, g1, b1) = if h_prime < 1.0 {
        (c, x, 0.0)
    } else if h_prime < 2.0 {
        (x, c, 0.0)
    } else if h_prime < 3.0 {
        (0.0, c, x)
    } else if h_prime < 4.0 {
        (0.0, x, c)
    } else if h_prime < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let m = value - c;
    let r = ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    let g = ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    let b = ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Rgb([r, g, b])
}

fn print_stats(label: &str, stats: &MatchStats) {
    let (inlier_count, mean_weight, left_spread, right_spread) = pair_details(stats);
    println!("[{label}]");
    println!("  total_keypoints: {}", stats.total_keypoints);
    println!("  total_matches: {}", stats.total_matches);
    println!("  connectivity: {:.6}", stats.connectivity);
    println!("  mean_matches_per_pair: {:.6}", stats.mean_matches_per_pair);
    println!("  mean_parallax_px: {:.6}", stats.mean_parallax_px);
    println!("  pair_inliers: {inlier_count}");
    println!("  mean_conf_weight: {:.6}", mean_weight);
    println!("  left_spread_px: {:.2}", left_spread);
    println!("  right_spread_px: {:.2}", right_spread);
    if !stats.failure_codes.is_empty() {
        println!("  failure_codes: {}", stats.failure_codes.join(","));
    }
}

fn pair_details(stats: &MatchStats) -> (usize, f64, f64, f64) {
    let Some(pair) = stats.pair_correspondences.first() else {
        return (0, 0.0, 0.0, 0.0);
    };
    if pair.points.is_empty() {
        return (0, 0.0, 0.0, 0.0);
    }

    let mean_weight = if pair.confidence_weights.is_empty() {
        0.0
    } else {
        pair.confidence_weights.iter().sum::<f64>() / pair.confidence_weights.len() as f64
    };

    let mut min_lx = f64::INFINITY;
    let mut max_lx = f64::NEG_INFINITY;
    let mut min_ly = f64::INFINITY;
    let mut max_ly = f64::NEG_INFINITY;
    let mut min_rx = f64::INFINITY;
    let mut max_rx = f64::NEG_INFINITY;
    let mut min_ry = f64::INFINITY;
    let mut max_ry = f64::NEG_INFINITY;

    for p in &pair.points {
        min_lx = min_lx.min(p[0]);
        max_lx = max_lx.max(p[0]);
        min_ly = min_ly.min(p[1]);
        max_ly = max_ly.max(p[1]);
        min_rx = min_rx.min(p[2]);
        max_rx = max_rx.max(p[2]);
        min_ry = min_ry.min(p[3]);
        max_ry = max_ry.max(p[3]);
    }

    let left_spread = ((max_lx - min_lx).powi(2) + (max_ly - min_ly).powi(2)).sqrt();
    let right_spread = ((max_rx - min_rx).powi(2) + (max_ry - min_ry).powi(2)).sqrt();

    (pair.points.len(), mean_weight, left_spread, right_spread)
}

fn get_arg(args: &[String], flag: &str) -> Option<String> {
    let mut i = 0usize;
    while i < args.len() {
        if args[i] == flag {
            return args.get(i + 1).cloned();
        }
        i += 1;
    }
    None
}
