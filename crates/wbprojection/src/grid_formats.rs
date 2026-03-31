//! Grid file format loaders for datum transformations.
//!
//! Supported loaders:
//! - NTv2 binary (`.gsb`) single-subgrid extraction
//! - NADCON-style ASCII shift pair (`.los`/`.las`) regular grids

use std::fs;
use std::path::Path;
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use crate::error::{ProjectionError, Result};
use crate::grid_shift::{GridShiftGrid, GridShiftSample, register_grid};

const NTV2_REC_LEN: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Endian {
    Le,
    Be,
}

fn key_from_record(rec: &[u8]) -> String {
    let key_bytes = &rec[..8];
    String::from_utf8_lossy(key_bytes)
        .trim_matches(char::from(0))
        .trim()
        .to_string()
}

fn read_u32(rec: &[u8], endian: Endian) -> u32 {
    let b = [rec[8], rec[9], rec[10], rec[11]];
    match endian {
        Endian::Le => u32::from_le_bytes(b),
        Endian::Be => u32::from_be_bytes(b),
    }
}

fn read_f64(rec: &[u8], endian: Endian) -> f64 {
    let b = [rec[8], rec[9], rec[10], rec[11], rec[12], rec[13], rec[14], rec[15]];
    match endian {
        Endian::Le => f64::from_le_bytes(b),
        Endian::Be => f64::from_be_bytes(b),
    }
}

fn read_label_value(rec: &[u8]) -> String {
    String::from_utf8_lossy(&rec[8..16])
        .trim_matches(char::from(0))
        .trim()
        .to_string()
}

fn detect_ntv2_endian(data: &[u8]) -> Result<Endian> {
    if data.len() < NTV2_REC_LEN {
        return Err(ProjectionError::DatumError(
            "NTv2 file too short to contain header".to_string(),
        ));
    }
    let key = key_from_record(&data[..NTV2_REC_LEN]);
    if key != "NUM_OREC" {
        return Err(ProjectionError::DatumError(format!(
            "invalid NTv2 header first key: expected NUM_OREC, found {key}"
        )));
    }
    let le = read_u32(&data[..NTV2_REC_LEN], Endian::Le);
    let be = read_u32(&data[..NTV2_REC_LEN], Endian::Be);

    if (1..=64).contains(&le) {
        Ok(Endian::Le)
    } else if (1..=64).contains(&be) {
        Ok(Endian::Be)
    } else {
        Err(ProjectionError::DatumError(
            "unable to determine NTv2 endianness".to_string(),
        ))
    }
}

fn parse_ntv2_fields(recs: &[&[u8]], endian: Endian) -> Result<std::collections::HashMap<String, f64>> {
    let mut m = std::collections::HashMap::new();
    for rec in recs {
        let key = key_from_record(rec);
        if key.is_empty() {
            continue;
        }
        m.insert(key, read_f64(rec, endian));
    }
    Ok(m)
}

#[derive(Debug, Clone)]
struct Ntv2SubgridDescriptor {
    name: String,
    parent: String,
    s_lat: f64,
    n_lat: f64,
    e_lon: f64,
    w_lon: f64,
    lat_inc: f64,
    lon_inc: f64,
    width: usize,
    height: usize,
    gs_count: usize,
    shifts_start: usize,
}

fn parse_ntv2_subgrids(data: &[u8], endian: Endian) -> Result<Vec<Ntv2SubgridDescriptor>> {
    if data.len() < NTV2_REC_LEN * 11 {
        return Err(ProjectionError::DatumError(
            "NTv2 file too short for overview header".to_string(),
        ));
    }

    let mut overview = Vec::with_capacity(11);
    for i in 0..11 {
        let s = i * NTV2_REC_LEN;
        overview.push(&data[s..s + NTV2_REC_LEN]);
    }

    let num_file = overview
        .iter()
        .find(|r| key_from_record(r) == "NUM_FILE")
        .map(|r| read_u32(r, endian) as usize)
        .unwrap_or(0);

    if num_file == 0 {
        return Err(ProjectionError::DatumError(
            "NTv2 overview header reports zero subgrids".to_string(),
        ));
    }

    let mut out = Vec::with_capacity(num_file);
    let mut offset = NTV2_REC_LEN * 11;

    for i in 0..num_file {
        let sg_hdr_start = offset;
        let sg_hdr_end = sg_hdr_start + NTV2_REC_LEN * 11;
        if data.len() < sg_hdr_end {
            return Err(ProjectionError::DatumError(
                "NTv2 file too short for subgrid header".to_string(),
            ));
        }

        let mut recs = Vec::with_capacity(11);
        for j in 0..11 {
            let s = sg_hdr_start + j * NTV2_REC_LEN;
            recs.push(&data[s..s + NTV2_REC_LEN]);
        }

        let fields = parse_ntv2_fields(&recs, endian)?;
        let get = |k: &str| -> Result<f64> {
            fields.get(k).copied().ok_or_else(|| {
                ProjectionError::DatumError(format!("NTv2 subgrid missing required field '{k}'"))
            })
        };

        let sub_name = recs
            .iter()
            .find(|r| key_from_record(r) == "SUB_NAME")
            .map(|r| read_label_value(r))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("SUBGRID_{}", i + 1));
        let parent = recs
            .iter()
            .find(|r| key_from_record(r) == "PARENT")
            .map(|r| read_label_value(r))
            .unwrap_or_default();

        let s_lat = get("S_LAT")?;
        let n_lat = get("N_LAT")?;
        let e_lon = get("E_LONG")?;
        let w_lon = get("W_LONG")?;
        let lat_inc = get("LAT_INC")?;
        let lon_inc = get("LONG_INC")?;
        let gs_count = get("GS_COUNT")? as usize;

        if lat_inc <= 0.0 || lon_inc <= 0.0 {
            return Err(ProjectionError::DatumError(
                "NTv2 increments must be positive".to_string(),
            ));
        }

        let width = ((w_lon - e_lon) / lon_inc).round() as isize + 1;
        let height = ((n_lat - s_lat) / lat_inc).round() as isize + 1;
        if width < 2 || height < 2 {
            return Err(ProjectionError::DatumError(
                "NTv2 subgrid dimensions are invalid".to_string(),
            ));
        }
        let width = width as usize;
        let height = height as usize;

        if width * height != gs_count {
            return Err(ProjectionError::DatumError(format!(
                "NTv2 GS_COUNT mismatch: header {gs_count}, computed {}",
                width * height
            )));
        }

        let shifts_start = sg_hdr_end;
        let shifts_end = shifts_start + gs_count * NTV2_REC_LEN;
        if data.len() < shifts_end {
            return Err(ProjectionError::DatumError(
                "NTv2 file too short for shift records".to_string(),
            ));
        }

        out.push(Ntv2SubgridDescriptor {
            name: sub_name,
            parent,
            s_lat,
            n_lat,
            e_lon,
            w_lon,
            lat_inc,
            lon_inc,
            width,
            height,
            gs_count,
            shifts_start,
        });

        offset = shifts_end;
    }

    Ok(out)
}

#[derive(Debug, Clone)]
struct Ntv2HierarchyEntry {
    grid_name: String,
    subgrid_name_norm: String,
    parent_name_norm: Option<String>,
    lon_min_deg: f64,
    lon_max_deg: f64,
    lat_min_deg: f64,
    lat_max_deg: f64,
    area_deg2: f64,
}

impl Ntv2HierarchyEntry {
    fn contains(&self, lon_deg: f64, lat_deg: f64) -> bool {
        lon_deg >= self.lon_min_deg
            && lon_deg <= self.lon_max_deg
            && lat_deg >= self.lat_min_deg
            && lat_deg <= self.lat_max_deg
    }
}

fn normalize_subgrid_name(name: &str) -> String {
    let n = name.trim();
    if n.is_empty() {
        String::new()
    } else {
        n.to_ascii_uppercase()
    }
}

static NTV2_HIERARCHY_REGISTRY: OnceLock<RwLock<HashMap<String, Vec<Ntv2HierarchyEntry>>>> =
    OnceLock::new();

fn hierarchy_registry() -> &'static RwLock<HashMap<String, Vec<Ntv2HierarchyEntry>>> {
    NTV2_HIERARCHY_REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

fn register_ntv2_hierarchy_entries(dataset_name: &str, entries: Vec<Ntv2HierarchyEntry>) -> Result<()> {
    let mut m = hierarchy_registry().write().map_err(|_| {
        ProjectionError::DatumError("NTv2 hierarchy registry lock poisoned".to_string())
    })?;
    m.insert(dataset_name.to_string(), entries);
    Ok(())
}

pub(crate) fn resolve_ntv2_hierarchy_grid(dataset_name: &str, lon_deg: f64, lat_deg: f64) -> Result<Option<String>> {
    let m = hierarchy_registry().read().map_err(|_| {
        ProjectionError::DatumError("NTv2 hierarchy registry lock poisoned".to_string())
    })?;

    let Some(entries) = m.get(dataset_name) else {
        return Ok(None);
    };

    let roots: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            e.parent_name_norm.is_none()
                && e.contains(lon_deg, lat_deg)
        })
        .map(|(i, _)| i)
        .collect();

    if roots.is_empty() {
        let fallback = entries
            .iter()
            .filter(|e| e.contains(lon_deg, lat_deg))
            .min_by(|a, b| {
                a.area_deg2
                    .partial_cmp(&b.area_deg2)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        return Ok(fallback.map(|e| e.grid_name.clone()));
    }

    let mut current_idx = *roots
        .iter()
        .min_by(|&&ia, &&ib| {
            entries[ia]
                .area_deg2
                .partial_cmp(&entries[ib].area_deg2)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap();

    loop {
        let parent_name = entries[current_idx].subgrid_name_norm.clone();
        let child = entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.parent_name_norm.as_deref() == Some(parent_name.as_str())
                    && e.contains(lon_deg, lat_deg)
            })
            .min_by(|(_, a), (_, b)| {
                a.area_deg2
                    .partial_cmp(&b.area_deg2)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i);

        match child {
            Some(i) => current_idx = i,
            None => break,
        }
    }

    Ok(Some(entries[current_idx].grid_name.clone()))
}

/// Resolve the registered grid name selected by NTv2 hierarchy logic
/// for a coordinate in geographic degrees.
///
/// Returns `None` when no registered hierarchy dataset or no covering subgrid exists.
pub fn resolve_ntv2_hierarchy_grid_name(
    dataset_name: &str,
    lon_deg: f64,
    lat_deg: f64,
) -> Result<Option<String>> {
    resolve_ntv2_hierarchy_grid(dataset_name, lon_deg, lat_deg)
}

/// Resolve the selected NTv2 subgrid name (not namespaced grid key)
/// for a coordinate in geographic degrees.
///
/// Returns `None` when no registered hierarchy dataset or no covering subgrid exists.
pub fn resolve_ntv2_hierarchy_subgrid(
    dataset_name: &str,
    lon_deg: f64,
    lat_deg: f64,
) -> Result<Option<String>> {
    let selected = resolve_ntv2_hierarchy_grid(dataset_name, lon_deg, lat_deg)?;
    Ok(selected.map(|name| {
        if let Some((_, sub)) = name.rsplit_once("::") {
            sub.to_string()
        } else {
            name
        }
    }))
}

fn build_grid_from_descriptor(
    data: &[u8],
    endian: Endian,
    descriptor: &Ntv2SubgridDescriptor,
    grid_name: String,
) -> Result<GridShiftGrid> {
    let mut samples = Vec::with_capacity(descriptor.gs_count);
    for i in 0..descriptor.gs_count {
        let s = descriptor.shifts_start + i * NTV2_REC_LEN;
        let rec = &data[s..s + NTV2_REC_LEN];

        let dlat = {
            let b = [rec[0], rec[1], rec[2], rec[3]];
            match endian {
                Endian::Le => f32::from_le_bytes(b) as f64,
                Endian::Be => f32::from_be_bytes(b) as f64,
            }
        };
        let dlon_west = {
            let b = [rec[4], rec[5], rec[6], rec[7]];
            match endian {
                Endian::Le => f32::from_le_bytes(b) as f64,
                Endian::Be => f32::from_be_bytes(b) as f64,
            }
        };

        samples.push(GridShiftSample::new(-dlon_west, dlat));
    }

    let lon_min_deg = (-descriptor.w_lon) / 3600.0;
    let lat_min_deg = descriptor.s_lat / 3600.0;
    let lon_step_deg = descriptor.lon_inc / 3600.0;
    let lat_step_deg = descriptor.lat_inc / 3600.0;

    GridShiftGrid::new(
        grid_name,
        lon_min_deg,
        lat_min_deg,
        lon_step_deg,
        lat_step_deg,
        descriptor.width,
        descriptor.height,
        samples,
    )
}

/// Load an NTv2 `.gsb` file and build a grid-shift model.
///
/// This loads the first subgrid found in the file.
/// Use [`load_ntv2_gsb_subgrid`] to target a specific subgrid by name.
pub fn load_ntv2_gsb(path: impl AsRef<Path>, grid_name: impl Into<String>) -> Result<GridShiftGrid> {
    let data = fs::read(path.as_ref()).map_err(|e| {
        ProjectionError::DatumError(format!("failed to read NTv2 file '{}': {e}", path.as_ref().display()))
    })?;

    let endian = detect_ntv2_endian(&data)?;
    let subgrids = parse_ntv2_subgrids(&data, endian)?;
    let first = subgrids.first().ok_or_else(|| {
        ProjectionError::DatumError("NTv2 file contains no subgrids".to_string())
    })?;
    build_grid_from_descriptor(&data, endian, first, grid_name.into())
}

/// List available NTv2 subgrid names in file order.
pub fn list_ntv2_subgrids(path: impl AsRef<Path>) -> Result<Vec<String>> {
    let data = fs::read(path.as_ref()).map_err(|e| {
        ProjectionError::DatumError(format!("failed to read NTv2 file '{}': {e}", path.as_ref().display()))
    })?;
    let endian = detect_ntv2_endian(&data)?;
    let subgrids = parse_ntv2_subgrids(&data, endian)?;
    Ok(subgrids.into_iter().map(|s| s.name).collect())
}

/// Load a specific NTv2 subgrid by name.
pub fn load_ntv2_gsb_subgrid(
    path: impl AsRef<Path>,
    grid_name: impl Into<String>,
    subgrid_name: &str,
) -> Result<GridShiftGrid> {
    let data = fs::read(path.as_ref()).map_err(|e| {
        ProjectionError::DatumError(format!("failed to read NTv2 file '{}': {e}", path.as_ref().display()))
    })?;
    let endian = detect_ntv2_endian(&data)?;
    let subgrids = parse_ntv2_subgrids(&data, endian)?;

    let descriptor = subgrids
        .iter()
        .find(|s| s.name.eq_ignore_ascii_case(subgrid_name))
        .ok_or_else(|| {
            ProjectionError::DatumError(format!(
                "NTv2 subgrid '{subgrid_name}' not found"
            ))
        })?;

    build_grid_from_descriptor(&data, endian, descriptor, grid_name.into())
}

/// Load and register an NTv2 `.gsb` file.
pub fn register_ntv2_gsb(path: impl AsRef<Path>, grid_name: impl Into<String>) -> Result<()> {
    let grid = load_ntv2_gsb(path, grid_name)?;
    register_grid(grid)
}

/// Load and register a specific NTv2 subgrid by name.
pub fn register_ntv2_gsb_subgrid(
    path: impl AsRef<Path>,
    grid_name: impl Into<String>,
    subgrid_name: &str,
) -> Result<()> {
    let grid = load_ntv2_gsb_subgrid(path, grid_name, subgrid_name)?;
    register_grid(grid)
}

/// Load all subgrids from an NTv2 file, register each regular grid, and register
/// a hierarchy dataset for runtime coordinate-based subgrid selection.
///
/// Registered grid names are namespaced as `"{dataset_name}::{subgrid_name}"`.
pub fn register_ntv2_gsb_hierarchy(
    path: impl AsRef<Path>,
    dataset_name: &str,
) -> Result<Vec<String>> {
    let data = fs::read(path.as_ref()).map_err(|e| {
        ProjectionError::DatumError(format!("failed to read NTv2 file '{}': {e}", path.as_ref().display()))
    })?;
    let endian = detect_ntv2_endian(&data)?;
    let descriptors = parse_ntv2_subgrids(&data, endian)?;

    let mut registered_names = Vec::with_capacity(descriptors.len());
    let mut entries = Vec::with_capacity(descriptors.len());

    for descriptor in descriptors {
        let grid_name = format!("{}::{}", dataset_name, descriptor.name);
        let grid = build_grid_from_descriptor(&data, endian, &descriptor, grid_name.clone())?;
        register_grid(grid)?;

        let lon_min_deg = (-descriptor.w_lon) / 3600.0;
        let lon_max_deg = (-descriptor.e_lon) / 3600.0;
        let lat_min_deg = descriptor.s_lat / 3600.0;
        let lat_max_deg = descriptor.n_lat / 3600.0;
        let area_deg2 = (lon_max_deg - lon_min_deg).abs() * (lat_max_deg - lat_min_deg).abs();

        let subgrid_name_norm = normalize_subgrid_name(&descriptor.name);
        let parent_norm_raw = normalize_subgrid_name(&descriptor.parent);
        let parent_name_norm = if parent_norm_raw.is_empty()
            || parent_norm_raw == "NONE"
            || parent_norm_raw == subgrid_name_norm
        {
            None
        } else {
            Some(parent_norm_raw)
        };

        entries.push(Ntv2HierarchyEntry {
            grid_name: grid_name.clone(),
            subgrid_name_norm,
            parent_name_norm,
            lon_min_deg,
            lon_max_deg,
            lat_min_deg,
            lat_max_deg,
            area_deg2,
        });
        registered_names.push(grid_name);
    }

    register_ntv2_hierarchy_entries(dataset_name, entries)?;
    Ok(registered_names)
}

fn parse_nadcon_ascii(path: &Path) -> Result<(usize, usize, f64, f64, f64, f64, Vec<f64>)> {
    let txt = fs::read_to_string(path).map_err(|e| {
        ProjectionError::DatumError(format!("failed to read NADCON ascii file '{}': {e}", path.display()))
    })?;

    let mut lines = txt.lines().filter(|l| !l.trim().is_empty());
    let header = lines.next().ok_or_else(|| {
        ProjectionError::DatumError("NADCON ascii file missing header line".to_string())
    })?;
    let parts: Vec<&str> = header.split_whitespace().collect();
    if parts.len() < 6 {
        return Err(ProjectionError::DatumError(
            "NADCON ascii header must contain: lon_min lat_min lon_step lat_step width height"
                .to_string(),
        ));
    }

    let lon_min: f64 = parts[0].parse().map_err(|_| ProjectionError::DatumError("invalid lon_min".to_string()))?;
    let lat_min: f64 = parts[1].parse().map_err(|_| ProjectionError::DatumError("invalid lat_min".to_string()))?;
    let lon_step: f64 = parts[2].parse().map_err(|_| ProjectionError::DatumError("invalid lon_step".to_string()))?;
    let lat_step: f64 = parts[3].parse().map_err(|_| ProjectionError::DatumError("invalid lat_step".to_string()))?;
    let width: usize = parts[4].parse().map_err(|_| ProjectionError::DatumError("invalid width".to_string()))?;
    let height: usize = parts[5].parse().map_err(|_| ProjectionError::DatumError("invalid height".to_string()))?;

    let mut vals = Vec::with_capacity(width * height);
    for l in lines {
        for tok in l.split_whitespace() {
            let v: f64 = tok
                .parse()
                .map_err(|_| ProjectionError::DatumError(format!("invalid numeric value '{tok}'")))?;
            vals.push(v);
        }
    }

    if vals.len() != width * height {
        return Err(ProjectionError::DatumError(format!(
            "NADCON ascii value count mismatch: expected {}, got {}",
            width * height,
            vals.len()
        )));
    }

    Ok((width, height, lon_min, lat_min, lon_step, lat_step, vals))
}

/// Load NADCON ASCII longitude/latitude shift grids (arc-seconds) into one model.
///
/// Expected simple ASCII format for each file:
/// first line: `lon_min lat_min lon_step lat_step width height`
/// remaining lines: `width*height` shift values in arc-seconds.
pub fn load_nadcon_ascii_pair(
    lon_shift_path: impl AsRef<Path>,
    lat_shift_path: impl AsRef<Path>,
    grid_name: impl Into<String>,
) -> Result<GridShiftGrid> {
    let (w1, h1, lon_min1, lat_min1, lon_step1, lat_step1, lon_vals) =
        parse_nadcon_ascii(lon_shift_path.as_ref())?;
    let (w2, h2, lon_min2, lat_min2, lon_step2, lat_step2, lat_vals) =
        parse_nadcon_ascii(lat_shift_path.as_ref())?;

    if (w1, h1) != (w2, h2)
        || (lon_min1 - lon_min2).abs() > 1e-12
        || (lat_min1 - lat_min2).abs() > 1e-12
        || (lon_step1 - lon_step2).abs() > 1e-12
        || (lat_step1 - lat_step2).abs() > 1e-12
    {
        return Err(ProjectionError::DatumError(
            "NADCON lon/lat grids are not aligned".to_string(),
        ));
    }

    let mut samples = Vec::with_capacity(w1 * h1);
    for (dlon, dlat) in lon_vals.into_iter().zip(lat_vals.into_iter()) {
        samples.push(GridShiftSample::new(dlon, dlat));
    }

    GridShiftGrid::new(
        grid_name,
        lon_min1,
        lat_min1,
        lon_step1,
        lat_step1,
        w1,
        h1,
        samples,
    )
}

/// Load and register NADCON ASCII pair grids.
pub fn register_nadcon_ascii_pair(
    lon_shift_path: impl AsRef<Path>,
    lat_shift_path: impl AsRef<Path>,
    grid_name: impl Into<String>,
) -> Result<()> {
    let grid = load_nadcon_ascii_pair(lon_shift_path, lat_shift_path, grid_name)?;
    register_grid(grid)
}

#[cfg(test)]
mod tests {
    use super::{
        list_ntv2_subgrids, load_nadcon_ascii_pair, load_ntv2_gsb, load_ntv2_gsb_subgrid,
    };
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        p.push(format!("wbproj_{name}_{t}"));
        p
    }

    #[test]
    fn parse_nadcon_ascii_pair() {
        let lon_path = temp_path("lon.asc");
        let lat_path = temp_path("lat.asc");

        let lon_txt = "0 0 1 1 2 2\n1 1\n1 1\n";
        let lat_txt = "0 0 1 1 2 2\n-2 -2\n-2 -2\n";

        fs::write(&lon_path, lon_txt).unwrap();
        fs::write(&lat_path, lat_txt).unwrap();

        let grid = load_nadcon_ascii_pair(&lon_path, &lat_path, "TEST_NADCON").unwrap();
        let (dlon, dlat) = grid.sample_shift_degrees(0.5, 0.5).unwrap();
        assert!((dlon - (1.0 / 3600.0)).abs() < 1e-12);
        assert!((dlat - (-2.0 / 3600.0)).abs() < 1e-12);

        let _ = fs::remove_file(&lon_path);
        let _ = fs::remove_file(&lat_path);
    }

    #[test]
    fn parse_minimal_ntv2_le() {
        let path = temp_path("test.gsb");

        fn rec_key_u32(key: &str, v: u32) -> [u8; 16] {
            let mut r = [0u8; 16];
            let kb = key.as_bytes();
            r[..kb.len().min(8)].copy_from_slice(&kb[..kb.len().min(8)]);
            r[8..12].copy_from_slice(&v.to_le_bytes());
            r
        }
        fn rec_key_f64(key: &str, v: f64) -> [u8; 16] {
            let mut r = [0u8; 16];
            let kb = key.as_bytes();
            r[..kb.len().min(8)].copy_from_slice(&kb[..kb.len().min(8)]);
            r[8..16].copy_from_slice(&v.to_le_bytes());
            r
        }
        fn shift_rec(dlat: f32, dlon_west: f32) -> [u8; 16] {
            let mut r = [0u8; 16];
            r[0..4].copy_from_slice(&dlat.to_le_bytes());
            r[4..8].copy_from_slice(&dlon_west.to_le_bytes());
            r
        }

        let mut bytes = Vec::new();

        // Overview header (11 records)
        bytes.extend_from_slice(&rec_key_u32("NUM_OREC", 11));
        bytes.extend_from_slice(&rec_key_u32("NUM_SREC", 11));
        bytes.extend_from_slice(&rec_key_u32("NUM_FILE", 1));
        bytes.extend_from_slice(&rec_key_f64("GS_TYPE", 0.0));
        bytes.extend_from_slice(&rec_key_f64("VERSION", 1.0));
        bytes.extend_from_slice(&rec_key_f64("SYSTEM_F", 0.0));
        bytes.extend_from_slice(&rec_key_f64("SYSTEM_T", 0.0));
        bytes.extend_from_slice(&rec_key_f64("MAJOR_F", 0.0));
        bytes.extend_from_slice(&rec_key_f64("MINOR_F", 0.0));
        bytes.extend_from_slice(&rec_key_f64("MAJOR_T", 0.0));
        bytes.extend_from_slice(&rec_key_f64("MINOR_T", 0.0));

        // Subgrid header (11 records)
        bytes.extend_from_slice(&rec_key_f64("SUB_NAME", 0.0));
        bytes.extend_from_slice(&rec_key_f64("PARENT", 0.0));
        bytes.extend_from_slice(&rec_key_f64("CREATED", 0.0));
        bytes.extend_from_slice(&rec_key_f64("UPDATED", 0.0));
        bytes.extend_from_slice(&rec_key_f64("S_LAT", 0.0));
        bytes.extend_from_slice(&rec_key_f64("N_LAT", 3600.0));
        bytes.extend_from_slice(&rec_key_f64("E_LONG", -3600.0));
        bytes.extend_from_slice(&rec_key_f64("W_LONG", 0.0));
        bytes.extend_from_slice(&rec_key_f64("LAT_INC", 3600.0));
        bytes.extend_from_slice(&rec_key_f64("LONG_INC", 3600.0));
        bytes.extend_from_slice(&rec_key_f64("GS_COUNT", 4.0));

        // 2x2 shifts: +1 arcsec east, -2 arcsec lat
        for _ in 0..4 {
            bytes.extend_from_slice(&shift_rec(-2.0, -1.0));
        }

        fs::write(&path, bytes).unwrap();

        let grid = load_ntv2_gsb(&path, "TEST_NTV2").unwrap();
        let (dlon, dlat) = grid.sample_shift_degrees(0.5, 0.5).unwrap();
        assert!((dlon - (1.0 / 3600.0)).abs() < 1e-12);
        assert!((dlat - (-2.0 / 3600.0)).abs() < 1e-12);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn list_and_load_specific_ntv2_subgrid() {
        let path = temp_path("test_multi.gsb");

        fn rec_key_u32(key: &str, v: u32) -> [u8; 16] {
            let mut r = [0u8; 16];
            let kb = key.as_bytes();
            r[..kb.len().min(8)].copy_from_slice(&kb[..kb.len().min(8)]);
            r[8..12].copy_from_slice(&v.to_le_bytes());
            r
        }
        fn rec_key_f64(key: &str, v: f64) -> [u8; 16] {
            let mut r = [0u8; 16];
            let kb = key.as_bytes();
            r[..kb.len().min(8)].copy_from_slice(&kb[..kb.len().min(8)]);
            r[8..16].copy_from_slice(&v.to_le_bytes());
            r
        }
        fn rec_key_label(key: &str, value: &str) -> [u8; 16] {
            let mut r = [0u8; 16];
            let kb = key.as_bytes();
            r[..kb.len().min(8)].copy_from_slice(&kb[..kb.len().min(8)]);
            let vb = value.as_bytes();
            let n = vb.len().min(8);
            r[8..8 + n].copy_from_slice(&vb[..n]);
            r
        }
        fn shift_rec(dlat: f32, dlon_west: f32) -> [u8; 16] {
            let mut r = [0u8; 16];
            r[0..4].copy_from_slice(&dlat.to_le_bytes());
            r[4..8].copy_from_slice(&dlon_west.to_le_bytes());
            r
        }

        let mut bytes = Vec::new();

        // Overview header (11 records), with NUM_FILE=2.
        bytes.extend_from_slice(&rec_key_u32("NUM_OREC", 11));
        bytes.extend_from_slice(&rec_key_u32("NUM_SREC", 11));
        bytes.extend_from_slice(&rec_key_u32("NUM_FILE", 2));
        bytes.extend_from_slice(&rec_key_f64("GS_TYPE", 0.0));
        bytes.extend_from_slice(&rec_key_f64("VERSION", 1.0));
        bytes.extend_from_slice(&rec_key_f64("SYSTEM_F", 0.0));
        bytes.extend_from_slice(&rec_key_f64("SYSTEM_T", 0.0));
        bytes.extend_from_slice(&rec_key_f64("MAJOR_F", 0.0));
        bytes.extend_from_slice(&rec_key_f64("MINOR_F", 0.0));
        bytes.extend_from_slice(&rec_key_f64("MAJOR_T", 0.0));
        bytes.extend_from_slice(&rec_key_f64("MINOR_T", 0.0));

        // Subgrid A (2x2), +1" lon east, -2" lat
        bytes.extend_from_slice(&rec_key_label("SUB_NAME", "SUBA"));
        bytes.extend_from_slice(&rec_key_label("PARENT", "NONE"));
        bytes.extend_from_slice(&rec_key_label("CREATED", "20260313"));
        bytes.extend_from_slice(&rec_key_label("UPDATED", "20260313"));
        bytes.extend_from_slice(&rec_key_f64("S_LAT", 0.0));
        bytes.extend_from_slice(&rec_key_f64("N_LAT", 3600.0));
        bytes.extend_from_slice(&rec_key_f64("E_LONG", -3600.0));
        bytes.extend_from_slice(&rec_key_f64("W_LONG", 0.0));
        bytes.extend_from_slice(&rec_key_f64("LAT_INC", 3600.0));
        bytes.extend_from_slice(&rec_key_f64("LONG_INC", 3600.0));
        bytes.extend_from_slice(&rec_key_f64("GS_COUNT", 4.0));
        for _ in 0..4 {
            bytes.extend_from_slice(&shift_rec(-2.0, -1.0));
        }

        // Subgrid B (2x2), +3" lon east, +4" lat
        bytes.extend_from_slice(&rec_key_label("SUB_NAME", "SUBB"));
        bytes.extend_from_slice(&rec_key_label("PARENT", "NONE"));
        bytes.extend_from_slice(&rec_key_label("CREATED", "20260313"));
        bytes.extend_from_slice(&rec_key_label("UPDATED", "20260313"));
        bytes.extend_from_slice(&rec_key_f64("S_LAT", 0.0));
        bytes.extend_from_slice(&rec_key_f64("N_LAT", 3600.0));
        bytes.extend_from_slice(&rec_key_f64("E_LONG", -3600.0));
        bytes.extend_from_slice(&rec_key_f64("W_LONG", 0.0));
        bytes.extend_from_slice(&rec_key_f64("LAT_INC", 3600.0));
        bytes.extend_from_slice(&rec_key_f64("LONG_INC", 3600.0));
        bytes.extend_from_slice(&rec_key_f64("GS_COUNT", 4.0));
        for _ in 0..4 {
            bytes.extend_from_slice(&shift_rec(4.0, -3.0));
        }

        fs::write(&path, bytes).unwrap();

        let names = list_ntv2_subgrids(&path).unwrap();
        assert_eq!(names, vec!["SUBA".to_string(), "SUBB".to_string()]);

        let grid_b = load_ntv2_gsb_subgrid(&path, "TEST_NTV2_B", "SUBB").unwrap();
        let (dlon_b, dlat_b) = grid_b.sample_shift_degrees(0.5, 0.5).unwrap();
        assert!((dlon_b - (3.0 / 3600.0)).abs() < 1e-12);
        assert!((dlat_b - (4.0 / 3600.0)).abs() < 1e-12);

        let _ = fs::remove_file(&path);
    }
}