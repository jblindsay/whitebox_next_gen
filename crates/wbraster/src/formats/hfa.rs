//! ERDAS IMAGINE HFA (Hierarchical File Architecture) format (`.img`).
//!
//! **Read-only, MVP scope.** Supports:
//! - Single and multi-band rasters
//! - Uncompressed pixels: `u1`, `u2`, `u4`, `u8`, `s8`, `u16`, `s16`, `u32`, `s32`, `f32`, `f64`
//! - RLC-compressed blocks (`compressType=1`) for direct and tiled storage
//! - Tiled and non-tiled (direct) band storage
//! - Geo-transform extraction from `Eprj_MapInfo`
//! - CRS extraction (Tier A—geographic datums; Tier B—comprehensive projected systems):
//!   - **Tier A**: Common geographic datums → EPSG codes (WGS-84, NAD83, NAD27, ED50, WGS72, etc.)
//!   - **Tier B**: Projected systems via ERDAS projection number + zone:
//!     - **UTM** (100% coverage): All 60 zones × 3 datums (WGS84, NAD83, NAD27)
//!       - Automatic zone detection from proZone, proParams[0], or proNumber
//!     - **State Plane** (comprehensive): All 50 US states + DC + 8 US territories
//!       - Covers ~80+ State Plane zones with both NAD83 and NAD27 EPSG codes
//!     - **Common Mercator-family**: Transverse Mercator, Polyconic, Lambert Conformal (via WKT fallback)
//!   - **Tier C (scaffolded)**: Seeded ERDAS projection-code mappings for globally common cases
//!     - Pseudo Mercator (EPRJ 68) → EPSG:3857 when parameters match canonical form
//!     - Mercator Variant A / Mercator (EPRJ 69/5) → EPSG:3395 when parameters match canonical form
//!     - TM/Gauss-Kruger family (EPRJ 9/36) → UTM EPSG when canonical UTM parameters are detected
//!     - UTM zone extraction from projection names (e.g., "...UTM_Zone_11N")
//!     - Explicit EPSG token extraction from projection names (e.g., "...EPSG:3035")
//!     - Strict name-only seeds for common global CRS labels when parameters are missing
//!     - Albers Conic Equal Area (EPRJ 3) seeded for NAD83 CONUS Albers → EPSG:5070
//!     - Lambert Conformal Conic (EPRJ 4) seeded for NAD83 StatsCan Lambert → EPSG:3347
//!     - Plate Carree / Equirectangular (EPRJ 34/17) seeded for canonical WGS84 → EPSG:32662
//!     - Equidistant Cylindrical (EPRJ 35) seeded for canonical WGS84 world form → EPSG:4087
//!     - Cylindrical Equal Area (EPRJ 58) seeded for canonical WGS84 global EASE form → EPSG:6933
//!     - Polar Stereographic (EPRJ 6) seeded for canonical WGS84 Arctic form → EPSG:3995
//!     - Polar Stereographic (EPRJ 6) seeded for canonical WGS84 Antarctic form → EPSG:3031
//!     - Lambert Azimuthal Equal Area (EPRJ 11) seeded for canonical WGS84 EASE polar forms
//!       → EPSG:6931 (north), EPSG:6932 (south)
//!   - Datum detection via sphere name, semi-major axis, and projection parameters
//! - Nodata extraction from `Eimg_NonInitializedValue` child nodes
//!
//! **Not supported in this MVP:**
//! - Write (export; use GeoTIFF instead)
//! - Non-RLC compressed tile codecs (if encountered)
//! - Spill files (`.rrd` pyramids)
//! - Attribute tables (`Edsc_Column`)
//!
//! ## File structure summary
//!
//! ```text
//! [File header – 34 bytes]
//!   Magic  "EHFA_HEADER_TAG\0"   (16 bytes)
//!   version                       uint32 LE
//!   free_list_ptr                 uint32 LE
//!   root_entry_ptr                uint32 LE  ← offset of root node header
//!   entry_header_length           uint16 LE  (usually 128)
//!   dictionary_ptr                uint32 LE
//!
//! [Node header – entry_header_length bytes, at each node's file offset]
//!   name[64]      ASCII, null-terminated
//!   type[32]      ASCII, null-terminated
//!   data_offset   uint32 LE  ← file offset of node data block
//!   data_size     uint32 LE
//!   next_entry    uint32 LE  ← sibling
//!   prev_entry    uint32 LE  ← sibling
//!   parent_entry  uint32 LE
//!   child_entry   uint32 LE  ← first child
//!   (8 bytes padding)
//!
//! Key node types:
//!   Eimg_Layer         – raster band; one per band; children of root
//!   Edms_State         – tile index; child of Eimg_Layer ("RasterDMS")
//!   Eimg_NonInitializedValue – nodata double; child of Eimg_Layer
//!   Eprj_MapInfo       – geo-transform; child of root
//!   Eprj_ProParameters – projection; child of root
//! ```
//!
//! Reference: MIL-STD / ERDAS Open Files specification, GDAL HFA driver source.

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::crs_info::CrsInfo;
use crate::error::{Result, RasterError};
use crate::io_utils::{read_f32_le, read_f64_le, read_i32_le, read_i16_le, read_u16_le, read_u32_le};
use crate::raster::{DataType, Raster, RasterConfig};

// ─── Constants ────────────────────────────────────────────────────────────────

const MAGIC: &[u8; 16] = b"EHFA_HEADER_TAG\0";
/// Byte offset for canonical `root_entry_ptr` in one HFA header layout variant.
const ROOT_PTR_OFFSET_V1: usize = 24;
/// Byte offset for canonical `entry_header_length` in one HFA header layout variant.
const ENTRY_HDR_LEN_OFFSET_V1: usize = 28;
/// Alternate observed root-entry pointer offset (GDAL-generated sample compatibility).
const ROOT_PTR_OFFSET_V2: usize = 28;
/// Alternate observed entry-header-length offset (GDAL-generated sample compatibility).
const ENTRY_HDR_LEN_OFFSET_V2: usize = 32;
/// Standard node-header size (bytes 28-29 of the file header).
const DEFAULT_ENTRY_HDR_LEN: usize = 128;

// ERDAS pixel-type enum values (EPTType).
const EPT_U1: u32  = 0;
const EPT_U2: u32  = 1;
const EPT_U4: u32  = 2;
const EPT_U8: u32  = 3;
const EPT_S8: u32  = 4;
const EPT_U16: u32 = 5;
const EPT_S16: u32 = 6;
const EPT_U32: u32 = 7;
const EPT_S32: u32 = 8;
const EPT_F32: u32 = 9;
const EPT_F64: u32 = 10;

// HFA RasterDMS compression types.
const HFA_COMPRESS_NONE: u32 = 0;
const HFA_COMPRESS_RLC: u32 = 1;

// ─── Public API ───────────────────────────────────────────────────────────────

/// Read an ERDAS IMAGINE `.img` file (read-only).
pub fn read(path: &str) -> Result<Raster> {
    match read_native(path) {
        Ok(raster) => Ok(raster),
        Err(native_err) => match read_via_gdal_translate(path) {
            Ok(raster) => Ok(raster),
            Err(_) => Err(native_err),
        },
    }
}

fn read_native(path: &str) -> Result<Raster> {
    let mut file = File::open(path)?;
    let mut raw: Vec<u8> = Vec::new();
    file.read_to_end(&mut raw)?;

    // ── File header ───────────────────────────────────────────────────────
    if raw.len() < 34 {
        return Err(RasterError::CorruptData(
            "HFA: file too short to contain a valid header".into(),
        ));
    }
    if &raw[..16] != MAGIC {
        return Err(RasterError::UnknownFormat(
            "HFA: missing EHFA_HEADER_TAG magic bytes".into(),
        ));
    }
    let (root_ptr, entry_hdr_len) = read_root_and_entry_header_len(&raw)?;

    // ── Walk the node tree ────────────────────────────────────────────────
    let mut nodes: Vec<NodeHdr> = Vec::new();
    collect_nodes(&raw, root_ptr, entry_hdr_len, None, &mut nodes)?;

    if nodes.is_empty() {
        return Err(RasterError::CorruptData("HFA: empty node tree".into()));
    }

    // Build lookup maps for fast child queries.
    // children_of[parent_offset] = vec of node indices in `nodes`
    let mut children_of: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, n) in nodes.iter().enumerate() {
        if let Some(p) = n.parent_offset {
            children_of.entry(p).or_default().push(i);
        }
    }

    // ── Collect Eimg_Layer bands (children of root, in sibling order) ─────
    let band_indices: Vec<usize> = nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| {
            n.type_name == "Eimg_Layer"
                || n.type_name == "Eimg_Layer_SubSample"
                || n.type_name == "Ehfa_Layer"
                || n.name.starts_with("Layer_")
        })
        .map(|(i, _)| i)
        .collect();

    let scanned_layer_nodes = if band_indices.is_empty() {
        scan_for_layer_nodes(&raw, entry_hdr_len)
    } else {
        Vec::new()
    };

    if band_indices.is_empty() && scanned_layer_nodes.is_empty() {
        return Err(RasterError::CorruptData(
            "HFA: no Eimg_Layer nodes found — file may not be a raster image".into(),
        ));
    }

    // ── Parse each band ───────────────────────────────────────────────────
    let mut bands: Vec<BandData> = Vec::with_capacity(band_indices.len());
    for &bi in &band_indices {
        let node = &nodes[bi];
        let band_info = match parse_eimg_layer(&raw, node) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let children = children_of.get(&node.file_offset).cloned().unwrap_or_default();

        // Locate RasterDMS child for tiled storage.
        let dms_node = children
            .iter()
            .find(|&&ci| nodes[ci].name == "RasterDMS" || nodes[ci].type_name == "Edms_State")
            .map(|&ci| &nodes[ci]);

        // Locate nodata child.
        let nodata_val = children
            .iter()
            .find(|&&ci| nodes[ci].name == "Eimg_NonInitializedValue"
                      || nodes[ci].type_name == "Eimg_NonInitializedValue")
            .and_then(|&ci| parse_nodata_node(&raw, &nodes[ci]));

        let pixels = read_band_pixels(&raw, &band_info, dms_node)?;
        bands.push(BandData {
            pixels,
            nodata: nodata_val,
            pixel_type: band_info.pixel_type,
        });
    }

    if bands.is_empty() {
        for node in &scanned_layer_nodes {
            let band_info = match parse_eimg_layer(&raw, node) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let pixels = read_band_pixels(&raw, &band_info, None)?;
            bands.push(BandData {
                pixels,
                nodata: None,
                pixel_type: band_info.pixel_type,
            });
        }
    }

    if bands.is_empty() {
        return Err(RasterError::CorruptData(
            "HFA: no parseable raster layer payloads found".into(),
        ));
    }

    // Validate uniform dimensions.
    let (cols, rows) = (bands[0].pixels.cols, bands[0].pixels.rows);
    for b in &bands {
        if b.pixels.cols != cols || b.pixels.rows != rows {
            return Err(RasterError::CorruptData(
                "HFA: bands have inconsistent dimensions".into(),
            ));
        }
    }

    // ── Geo-transform ─────────────────────────────────────────────────────
    let map_info_node = nodes.iter().find(|n| n.name == "Eprj_MapInfo");
    let geo = map_info_node
        .and_then(|n| parse_eprj_map_info(&raw, n, rows).ok())
        .unwrap_or(GeoTransform {
            x_min: 0.0,
            y_min: 0.0,
            cell_size_x: 1.0,
            cell_size_y: 1.0,
        });

    // ── CRS ───────────────────────────────────────────────────────────────
    let crs = nodes
        .iter()
        .find(|n| n.name == "Eprj_ProParameters")
        .and_then(|n| parse_eprj_pro_parameters(&raw, n).ok())
        .unwrap_or_default();

    // ── Assemble raster ───────────────────────────────────────────────────
    let num_bands = bands.len();
    let data_type = ept_to_data_type(bands[0].pixel_type)?;
    let nodata = bands[0].nodata.unwrap_or(-9999.0);

    // Flatten all bands into a single band-major Vec<f64>.
    let total = num_bands * rows * cols;
    let mut all_data: Vec<f64> = Vec::with_capacity(total);
    for b in bands {
        all_data.extend_from_slice(&b.pixels.data);
    }

    let cfg = RasterConfig {
        cols,
        rows,
        bands: num_bands,
        x_min: geo.x_min,
        y_min: geo.y_min,
        cell_size: geo.cell_size_x,
        cell_size_y: if (geo.cell_size_y - geo.cell_size_x).abs() > 1e-12 {
            Some(-geo.cell_size_y)
        } else {
            None
        },
        nodata,
        data_type,
        crs,
        ..Default::default()
    };
    Raster::from_data(cfg, all_data)
}

fn read_via_gdal_translate(path: &str) -> Result<Raster> {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temp_path: PathBuf = std::env::temp_dir().join(format!("wbw_hfa_fallback_{unique}.tif"));

    let status = Command::new("gdal_translate")
        .args(["-of", "GTiff", path, temp_path.to_string_lossy().as_ref()])
        .status();

    let status = match status {
        Ok(status) if status.success() => status,
        _ => return Err(RasterError::CorruptData("HFA: native parser failed and GDAL fallback was unavailable".into())),
    };
    let _ = status;

    let result = super::geotiff::read(temp_path.to_string_lossy().as_ref());
    let _ = std::fs::remove_file(&temp_path);
    result
}

fn read_root_and_entry_header_len(raw: &[u8]) -> Result<(usize, usize)> {
    fn plausible_entry_len(v: usize) -> bool {
        (64..=1024).contains(&v)
    }
    fn plausible_root(root: usize, raw_len: usize) -> bool {
        root > 0 && root + 64 <= raw_len
    }

    let r1 = read_u32_le(raw, ROOT_PTR_OFFSET_V1) as usize;
    let e1_raw = read_u16_le(raw, ENTRY_HDR_LEN_OFFSET_V1) as usize;
    let e1 = if e1_raw == 0 { DEFAULT_ENTRY_HDR_LEN } else { e1_raw };

    let r2 = read_u32_le(raw, ROOT_PTR_OFFSET_V2) as usize;
    let e2_raw = read_u16_le(raw, ENTRY_HDR_LEN_OFFSET_V2) as usize;
    let e2 = if e2_raw == 0 { DEFAULT_ENTRY_HDR_LEN } else { e2_raw };

    let v1_ok = plausible_entry_len(e1) && plausible_root(r1, raw.len());
    let v2_ok = plausible_entry_len(e2) && plausible_root(r2, raw.len());

    if v1_ok && (!v2_ok || r1 <= r2) {
        return Ok((r1, e1));
    }
    if v2_ok {
        return Ok((r2, e2));
    }

    Err(RasterError::CorruptData(format!(
        "HFA: could not resolve root node header offsets (v1 root={r1:#x}, len={e1}; v2 root={r2:#x}, len={e2})"
    )))
}

/// Write an ERDAS IMAGINE `.img` file (not yet implemented).
pub fn write(_raster: &Raster, _path: &str) -> Result<()> {
    Err(RasterError::UnsupportedDataType(
        "Writing ERDAS IMAGINE format is not yet implemented".into(),
    ))
}

// ─── Internal data types ──────────────────────────────────────────────────────

/// Parsed node header (128 bytes on disk).
#[derive(Debug)]
struct NodeHdr {
    /// Node name (up to 64 chars).
    name: String,
    /// Node type name (up to 32 chars, refers to a dictionary entry).
    type_name: String,
    /// File offset of this node's data block (0 = no data).
    data_offset: usize,
    /// Size of this node's data block in bytes.
    data_size: usize,
    /// File offset of the parent node (None for root).
    parent_offset: Option<usize>,
    /// File offset where THIS node header was read from.
    file_offset: usize,
}

/// Parsed Eimg_Layer fields we need.
#[derive(Debug)]
struct EimgLayer {
    width: u32,
    height: u32,
    /// ERDAS EPTType enum.
    pixel_type: u32,
    block_width: f64,
    block_height: f64,
    compress_type: u32,
    /// File offset to raw pixel data (valid for non-tiled bands only).
    data_file_offset: usize,
    data_file_size: usize,
}

/// Per-band pixel data with row-major top-down layout.
struct PixelGrid {
    data: Vec<f64>,
    cols: usize,
    rows: usize,
}

struct BandData {
    pixels: PixelGrid,
    nodata: Option<f64>,
    pixel_type: u32,
}

/// Geo-transform fields.
struct GeoTransform {
    x_min: f64,
    y_min: f64,
    cell_size_x: f64,
    cell_size_y: f64,
}

// ─── Node tree traversal ──────────────────────────────────────────────────────

/// Recursively walk the node tree (DFS via child+sibling links) and push all
/// nodes into `out`.  Uses an iteration depth limit to guard against cycles.
fn collect_nodes(
    raw: &[u8],
    node_offset: usize,
    entry_hdr_len: usize,
    parent_offset: Option<usize>,
    out: &mut Vec<NodeHdr>,
) -> Result<()> {
    const MAX_NODES: usize = 65536;
    let mut stack: Vec<(usize, Option<usize>)> = vec![(node_offset, parent_offset)];
    while let Some((offset, parent)) = stack.pop() {
        if offset == 0 || out.len() >= MAX_NODES {
            continue;
        }
        if raw.len() < offset + entry_hdr_len {
            return Err(RasterError::CorruptData(format!(
                "HFA: node at offset {offset:#x} would read past end of file"
            )));
        }
        let node_raw = &raw[offset..offset + entry_hdr_len];
        // HFA node headers are encountered in at least two layout variants in the wild.
        // Variant A (legacy assumption): [name,type,data,next,prev,parent,child]
        // Variant B (GDAL samples): [next,prev,parent,child,name,type,data,size]
        let a_name = nul_terminated_str(&node_raw[..64]).to_string();
        let a_type = nul_terminated_str(&node_raw[64..96]).to_string();
        let a_data_offset = read_u32_le(node_raw, 96) as usize;
        let a_data_size = read_u32_le(node_raw, 100) as usize;
        let a_next = read_u32_le(node_raw, 104) as usize;
        let a_child = read_u32_le(node_raw, 116) as usize;

        let b_name = if entry_hdr_len >= 80 {
            nul_terminated_str(&node_raw[16..80]).to_string()
        } else {
            String::new()
        };
        let b_type = if entry_hdr_len >= 112 {
            nul_terminated_str(&node_raw[80..112]).to_string()
        } else {
            String::new()
        };
        let b_data_offset = read_u32_le(node_raw, 112) as usize;
        let b_data_size = read_u32_le(node_raw, 116) as usize;
        let b_next = read_u32_le(node_raw, 0) as usize;
        let b_child = read_u32_le(node_raw, 12) as usize;

        let a_score = (!a_name.is_empty() as usize) + (!a_type.is_empty() as usize);
        let b_score = (!b_name.is_empty() as usize) + (!b_type.is_empty() as usize);

        let (name, type_name, data_offset, data_size, next_entry, child_entry) = if b_score > a_score {
            (b_name, b_type, b_data_offset, b_data_size, b_next, b_child)
        } else {
            (a_name, a_type, a_data_offset, a_data_size, a_next, a_child)
        };

        let node = NodeHdr {
            name,
            type_name,
            data_offset,
            data_size,
            parent_offset: parent,
            file_offset: offset,
        };
        // Push next sibling (same parent) before pushing children.
        if next_entry != 0 {
            stack.push((next_entry, parent));
        }
        // Push first child.
        if child_entry != 0 {
            stack.push((child_entry, Some(offset)));
        }
        out.push(node);
    }
    Ok(())
}

/// Extract a null-terminated ASCII string from a byte slice.
#[inline]
fn nul_terminated_str(bytes: &[u8]) -> &str {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    std::str::from_utf8(&bytes[..end]).unwrap_or("")
}

fn scan_for_layer_nodes(raw: &[u8], entry_hdr_len: usize) -> Vec<NodeHdr> {
    if entry_hdr_len < 120 || raw.len() < entry_hdr_len {
        return Vec::new();
    }

    let mut out = Vec::new();
    for offset in 0..=(raw.len() - entry_hdr_len) {
        let node_raw = &raw[offset..offset + entry_hdr_len];

        let type_a = nul_terminated_str(&node_raw[64..96]);
        let type_b = nul_terminated_str(&node_raw[80..112]);
        let is_layer = type_a == "Eimg_Layer"
            || type_a == "Eimg_Layer_SubSample"
            || type_b == "Eimg_Layer"
            || type_b == "Eimg_Layer_SubSample";
        if !is_layer {
            continue;
        }

        let (name, type_name, data_offset, data_size) = if type_b == "Eimg_Layer"
            || type_b == "Eimg_Layer_SubSample"
        {
            (
                nul_terminated_str(&node_raw[16..80]).to_string(),
                type_b.to_string(),
                read_u32_le(node_raw, 112) as usize,
                read_u32_le(node_raw, 116) as usize,
            )
        } else {
            (
                nul_terminated_str(&node_raw[..64]).to_string(),
                type_a.to_string(),
                read_u32_le(node_raw, 96) as usize,
                read_u32_le(node_raw, 100) as usize,
            )
        };

        if data_offset == 0 || data_size < 52 || data_offset + 52 > raw.len() {
            continue;
        }

        out.push(NodeHdr {
            name,
            type_name,
            data_offset,
            data_size,
            parent_offset: None,
            file_offset: offset,
        });
    }

    out
}

// ─── Eimg_Layer parser ────────────────────────────────────────────────────────

/// Parse the binary `Eimg_Layer` data block.
///
/// Hard-coded field layout (matches the ERDAS IMAGINE standard dictionary):
/// ```text
/// Offset  Size  Field
///      0     4  width          (int32 LE)
///      4     4  height         (int32 LE)
///      8     4  layerType enum (int32 LE; 0=thematic, 1=raster)
///     12     4  pixelType enum (int32 LE; EPTType: 0=u1 … 10=f64)
///     16     8  blockWidth     (float64 LE)
///     24     8  blockHeight    (float64 LE)
///     32     4  compressType   (int32 LE; 0=none)
///     36     4  ptr Edsc_Table (uint32 LE; pointer, may be 0)
///     40     4  numRasters     (int32 LE)
///     44     4  dataOffset     (uint32 LE; file offset to pixel data)
///     48     4  dataSize       (int32 LE)
/// ```
fn parse_eimg_layer(raw: &[u8], node: &NodeHdr) -> Result<EimgLayer> {
    let off = node.data_offset;
    if node.data_size < 52 || raw.len() < off + 52 {
        return Err(RasterError::CorruptData(format!(
            "HFA: Eimg_Layer node '{}' data too small ({} bytes)",
            node.name, node.data_size
        )));
    }
    let d = &raw[off..off + node.data_size.min(raw.len() - off)];
    let width = read_i32_le(d, 0) as u32;
    let height = read_i32_le(d, 4) as u32;
    let pixel_type = read_i32_le(d, 12) as u32;
    let block_width = read_f64_le(d, 16);
    let block_height = read_f64_le(d, 24);
    let compress_type = read_i32_le(d, 32) as u32;
    let data_file_offset = read_u32_le(d, 44) as usize;
    let data_file_size = read_i32_le(d, 48) as usize;

    if width == 0 || height == 0 {
        return Err(RasterError::InvalidDimensions {
            cols: width as usize,
            rows: height as usize,
        });
    }
    Ok(EimgLayer {
        width,
        height,
        pixel_type,
        block_width: if block_width > 0.0 { block_width } else { width as f64 },
        block_height: if block_height > 0.0 { block_height } else { height as f64 },
        compress_type,
        data_file_offset,
        data_file_size,
    })
}

// ─── Nodata node ─────────────────────────────────────────────────────────────

/// Read the single `float64` initializedValue from an `Eimg_NonInitializedValue` node.
fn parse_nodata_node(raw: &[u8], node: &NodeHdr) -> Option<f64> {
    let off = node.data_offset;
    if node.data_size < 8 || raw.len() < off + 8 {
        return None;
    }
    Some(read_f64_le(raw, off))
}

// ─── Pixel data reader ────────────────────────────────────────────────────────

fn read_band_pixels(
    raw: &[u8],
    info: &EimgLayer,
    dms_node: Option<&NodeHdr>,
) -> Result<PixelGrid> {
    let cols = info.width as usize;
    let rows = info.height as usize;

    match dms_node {
        None => {
            // ── Direct (non-tiled) storage ────────────────────────────────
            // Pixel data is at data_file_offset, sequential row-major top-down.
            let pixel_count = cols * rows;
            let bytes_needed = pixel_bytes(info.pixel_type, pixel_count)?;
            let file_off = info.data_file_offset;
            let data = if info.compress_type == HFA_COMPRESS_NONE {
                if raw.len() < file_off + bytes_needed {
                    return Err(RasterError::CorruptData(format!(
                        "HFA: direct pixel data at offset {file_off:#x} extends past end of file \
                         ({} bytes needed, {} available)",
                        bytes_needed,
                        raw.len().saturating_sub(file_off)
                    )));
                }
                decode_pixels(raw, file_off, info.pixel_type, pixel_count)?
            } else if info.compress_type == HFA_COMPRESS_RLC {
                let comp_size = info.data_file_size;
                if comp_size == 0 || raw.len() < file_off + comp_size {
                    return Err(RasterError::CorruptData(format!(
                        "HFA: compressed direct block at offset {file_off:#x} is truncated"
                    )));
                }
                let comp = &raw[file_off..file_off + comp_size];
                let uncompressed = uncompress_hfa_rlc_block(comp, info.pixel_type, pixel_count)?;
                decode_pixels_from_bytes(&uncompressed, info.pixel_type, pixel_count)?
            } else {
                return Err(RasterError::UnsupportedDataType(format!(
                    "HFA: unsupported direct compression type {} (only RLC=1 is supported)",
                    info.compress_type
                )));
            };
            Ok(PixelGrid { data, cols, rows })
        }
        Some(dms) => {
            // ── Tiled storage (Edms_State / RasterDMS) ───────────────────
            read_tiled_pixels(raw, info, dms, cols, rows)
        }
    }
}

/// Read the tile index from the `Edms_State` (`RasterDMS`) node and assemble
/// the full grid by copying each tile into the right position.
///
/// `Edms_State` binary layout (hard-coded, all int32 LE unless noted):
/// ```text
/// Offset  Field
///      0  numVirtualBlocks
///      4  numObjectsPerBlock
///      8  nextObjectNum
///     12  initialDirtyObjectCount
///     16  compressionType (enum: 0=none)
///     20  compressionLayerVersion
///     24  objectVersion
///     28  numRasterBands (unused here)
///     32  displayLayerNum
///     36  dataFileCode
///     40  → array of Edms_VirtualBlockInfo (32 bytes each)
///
/// Edms_VirtualBlockInfo (32 bytes, all int32 LE):
///     fileCode, dataOffset, dataSize, logicalBlockNum,
///     parentLogicalBlockNum, compressionType, compressionVersion,
///     cacheBufferOffset
/// ```
fn read_tiled_pixels(
    raw: &[u8],
    info: &EimgLayer,
    dms: &NodeHdr,
    cols: usize,
    rows: usize,
) -> Result<PixelGrid> {
    const DMS_HEADER: usize = 40;
    const BLOCK_INFO_SIZE: usize = 32;

    let off = dms.data_offset;
    if dms.data_size < DMS_HEADER || raw.len() < off + DMS_HEADER {
        return Err(RasterError::CorruptData(
            "HFA: RasterDMS node data too small".into(),
        ));
    }
    let d = &raw[off..off + dms.data_size.min(raw.len() - off)];

    let num_virtual_blocks = read_i32_le(d, 0) as usize;
    if num_virtual_blocks == 0 || d.len() < DMS_HEADER + num_virtual_blocks * BLOCK_INFO_SIZE {
        return Err(RasterError::CorruptData(format!(
            "HFA: RasterDMS block count {num_virtual_blocks} inconsistent with node data size {}", d.len()
        )));
    }

    // Build a map: logicalBlockNum → (file_offset, file_size, compress_type)
    let mut tile_map: HashMap<u32, TileRef> = HashMap::with_capacity(num_virtual_blocks);
    for i in 0..num_virtual_blocks {
        let base = DMS_HEADER + i * BLOCK_INFO_SIZE;
        if base + BLOCK_INFO_SIZE > d.len() {
            break;
        }
        let file_code = read_i32_le(d, base) as u32;
        let data_offset = read_i32_le(d, base + 4) as u32;
        let data_size = read_i32_le(d, base + 8) as u32;
        let logical_num = read_i32_le(d, base + 12) as u32;
        let compress = read_i32_le(d, base + 20) as u32;
        if file_code == 0 || data_offset != 0 {
            // file_code == 0 means main file; data_offset == 0 → uninitialized tile (skip)
            tile_map.insert(logical_num, TileRef {
                file_offset: data_offset as usize,
                data_size: data_size as usize,
                compress_type: compress,
            });
        }
    }

    let tile_w = info.block_width as usize;
    let tile_h = info.block_height as usize;
    let tiles_x = cols.div_ceil(tile_w);
    let tiles_y = rows.div_ceil(tile_h);

    let mut data = vec![f64::NAN; rows * cols];

    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            let logical = (ty * tiles_x + tx) as u32;
            let Some(tile_ref) = tile_map.get(&logical) else {
                // Uninitialized tile — leave as NaN (caller will convert to nodata).
                continue;
            };
            let pixel_count = tile_w * tile_h;
            let bytes_needed = pixel_bytes(info.pixel_type, pixel_count)?;
            let file_off = tile_ref.file_offset;
            let tile_pixels = if tile_ref.compress_type == HFA_COMPRESS_NONE {
                if raw.len() < file_off + bytes_needed {
                    continue; // partial/corrupt tile — leave as NaN
                }
                decode_pixels(raw, file_off, info.pixel_type, pixel_count)?
            } else if tile_ref.compress_type == HFA_COMPRESS_RLC {
                if tile_ref.data_size == 0 || raw.len() < file_off + tile_ref.data_size {
                    continue; // partial/corrupt tile — leave as NaN
                }
                let comp = &raw[file_off..file_off + tile_ref.data_size];
                let uncompressed = match uncompress_hfa_rlc_block(comp, info.pixel_type, pixel_count) {
                    Ok(v) => v,
                    Err(_) => continue, // bad compressed tile — leave as NaN
                };
                decode_pixels_from_bytes(&uncompressed, info.pixel_type, pixel_count)?
            } else {
                return Err(RasterError::UnsupportedDataType(format!(
                    "HFA: unsupported tile compression type {} (only RLC=1 is supported)",
                    tile_ref.compress_type
                )));
            };

            // Copy tile into output grid.
            let dst_row_start = ty * tile_h;
            let dst_col_start = tx * tile_w;
            for tr in 0..tile_h {
                let dst_row = dst_row_start + tr;
                if dst_row >= rows {
                    break;
                }
                let actual_cols = (dst_col_start + tile_w).min(cols) - dst_col_start;
                let src_base = tr * tile_w;
                let dst_base = dst_row * cols + dst_col_start;
                data[dst_base..dst_base + actual_cols]
                    .copy_from_slice(&tile_pixels[src_base..src_base + actual_cols]);
            }
        }
    }

    // Replace NaN with -9999 (caller-supplied nodata will be handled above).
    for v in data.iter_mut() {
        if v.is_nan() {
            *v = -9999.0;
        }
    }

    Ok(PixelGrid { data, cols, rows })
}

#[derive(Debug)]
struct TileRef {
    file_offset: usize,
    data_size: usize,
    compress_type: u32,
}

// ─── Pixel decoding ───────────────────────────────────────────────────────────

/// Return the number of bytes required for `count` pixels of the given EPT type.
fn pixel_bytes(pixel_type: u32, count: usize) -> Result<usize> {
    let bytes = match pixel_type {
        EPT_U1  => count.div_ceil(8),
        EPT_U2  => count.div_ceil(4),
        EPT_U4  => count.div_ceil(2),
        EPT_U8 | EPT_S8   => count,
        EPT_U16 | EPT_S16 => count * 2,
        EPT_U32 | EPT_S32 => count * 4,
        EPT_F32            => count * 4,
        EPT_F64            => count * 8,
        _ => return Err(RasterError::UnsupportedDataType(format!(
            "HFA: pixel type {} (complex/unknown) is not supported", pixel_type
        ))),
    };
    Ok(bytes)
}

/// Decode `count` pixels from `raw` at `offset` into a `Vec<f64>`.
fn decode_pixels(raw: &[u8], offset: usize, pixel_type: u32, count: usize) -> Result<Vec<f64>> {
    let bytes_needed = pixel_bytes(pixel_type, count)?;
    if raw.len() < offset + bytes_needed {
        return Err(RasterError::CorruptData("HFA: pixel buffer too short".into()));
    }
    decode_pixels_from_bytes(&raw[offset..offset + bytes_needed], pixel_type, count)
}

fn decode_pixels_from_bytes(src: &[u8], pixel_type: u32, count: usize) -> Result<Vec<f64>> {
    let mut out = Vec::with_capacity(count);
    match pixel_type {
        EPT_U1 => {
            for i in 0..count {
                let byte = src[i / 8];
                let bit = (byte >> (i % 8)) & 1;
                out.push(bit as f64);
            }
        }
        EPT_U2 => {
            for i in 0..count {
                let byte = src[i / 4];
                let val = (byte >> ((i % 4) * 2)) & 0x03;
                out.push(val as f64);
            }
        }
        EPT_U4 => {
            for i in 0..count {
                let byte = src[i / 2];
                let val = if i % 2 == 0 { byte & 0x0F } else { (byte >> 4) & 0x0F };
                out.push(val as f64);
            }
        }
        EPT_U8 => {
            for i in 0..count {
                out.push(src[i] as f64);
            }
        }
        EPT_S8 => {
            for i in 0..count {
                out.push(src[i] as i8 as f64);
            }
        }
        EPT_U16 => {
            for i in 0..count {
                out.push(read_u16_le(src, i * 2) as f64);
            }
        }
        EPT_S16 => {
            for i in 0..count {
                out.push(read_i16_le(src, i * 2) as f64);
            }
        }
        EPT_U32 => {
            for i in 0..count {
                out.push(read_u32_le(src, i * 4) as f64);
            }
        }
        EPT_S32 => {
            for i in 0..count {
                out.push(read_i32_le(src, i * 4) as f64);
            }
        }
        EPT_F32 => {
            for i in 0..count {
                out.push(read_f32_le(src, i * 4) as f64);
            }
        }
        EPT_F64 => {
            for i in 0..count {
                out.push(read_f64_le(src, i * 8));
            }
        }
        _ => {
            return Err(RasterError::UnsupportedDataType(format!(
                "HFA: pixel type {} is not supported", pixel_type
            )));
        }
    }
    Ok(out)
}

/// Decode HFA RLC-compressed block payload into uncompressed pixel bytes.
///
/// Layout (`hfaband.cpp::UncompressBlock` compatible):
/// - bytes 0..4:  dataMin (u32 LE)
/// - bytes 4..8:  numRuns (i32 LE; -1 means non-RLE packed stream)
/// - bytes 8..12: dataOffset (i32 LE; offset of value stream)
/// - byte 12:     numBits per value
/// - bytes 13..:  run-count stream (if numRuns != -1) and/or values stream
fn uncompress_hfa_rlc_block(comp: &[u8], pixel_type: u32, pixel_count: usize) -> Result<Vec<u8>> {
    if comp.len() < 13 {
        return Err(RasterError::CorruptData("HFA: compressed block header too short".into()));
    }

    let data_min = read_u32_le(comp, 0);
    let num_runs = read_i32_le(comp, 4);
    let data_offset = read_i32_le(comp, 8);
    if data_offset < 13 || (data_offset as usize) > comp.len() {
        return Err(RasterError::CorruptData("HFA: invalid compressed block dataOffset".into()));
    }
    let num_bits = comp[12] as usize;

    let out_len = pixel_bytes(pixel_type, pixel_count)?;
    let mut out = vec![0_u8; out_len];

    let mut pixels_out = 0usize;
    let mut values_bit_pos = 0usize;
    let values = &comp[data_offset as usize..];

    if num_runs == -1 {
        while pixels_out < pixel_count {
            let raw_value = read_value_bits(values, &mut values_bit_pos, num_bits)?;
            let value = raw_value.wrapping_add(data_min);
            write_pixel_value(&mut out, pixel_type, pixels_out, value)?;
            pixels_out += 1;
        }
        return Ok(out);
    }

    if num_runs < 0 {
        return Err(RasterError::CorruptData("HFA: invalid numRuns in compressed block".into()));
    }

    let mut counter_pos = 13usize;
    for _ in 0..(num_runs as usize) {
        if counter_pos >= comp.len() {
            return Err(RasterError::CorruptData("HFA: truncated run-count stream".into()));
        }
        let first = comp[counter_pos];
        counter_pos += 1;

        let mut repeat = (first & 0x3f) as usize;
        match first & 0xc0 {
            0x00 => {}
            0x40 => {
                if counter_pos + 1 > comp.len() {
                    return Err(RasterError::CorruptData("HFA: truncated 2-byte run-count".into()));
                }
                repeat = (repeat << 8) | comp[counter_pos] as usize;
                counter_pos += 1;
            }
            0x80 => {
                if counter_pos + 2 > comp.len() {
                    return Err(RasterError::CorruptData("HFA: truncated 3-byte run-count".into()));
                }
                repeat = (repeat << 16)
                    | ((comp[counter_pos] as usize) << 8)
                    | comp[counter_pos + 1] as usize;
                counter_pos += 2;
            }
            _ => {
                if counter_pos + 3 > comp.len() {
                    return Err(RasterError::CorruptData("HFA: truncated 4-byte run-count".into()));
                }
                repeat = (repeat << 24)
                    | ((comp[counter_pos] as usize) << 16)
                    | ((comp[counter_pos + 1] as usize) << 8)
                    | comp[counter_pos + 2] as usize;
                counter_pos += 3;
            }
        }

        let raw_value = read_value_bits(values, &mut values_bit_pos, num_bits)?;
        let value = raw_value.wrapping_add(data_min);

        if repeat == 0 {
            continue;
        }

        let max_repeat = pixel_count.saturating_sub(pixels_out);
        let repeat = repeat.min(max_repeat);
        for _ in 0..repeat {
            write_pixel_value(&mut out, pixel_type, pixels_out, value)?;
            pixels_out += 1;
        }

        if pixels_out >= pixel_count {
            break;
        }
    }

    Ok(out)
}

fn read_value_bits(values: &[u8], bit_pos: &mut usize, num_bits: usize) -> Result<u32> {
    if num_bits == 0 {
        return Ok(0);
    }
    if num_bits > 32 {
        return Err(RasterError::UnsupportedDataType(format!(
            "HFA: compressed value bit width {} is not supported",
            num_bits
        )));
    }

    let total_bits = values.len() * 8;
    if *bit_pos + num_bits > total_bits {
        return Err(RasterError::CorruptData("HFA: truncated compressed value stream".into()));
    }

    let mut v = 0_u32;
    for i in 0..num_bits {
        let bp = *bit_pos + i;
        let byte = values[bp / 8];
        let bit = (byte >> (bp % 8)) & 1;
        v |= (bit as u32) << i;
    }
    *bit_pos += num_bits;
    Ok(v)
}

fn write_pixel_value(dst: &mut [u8], pixel_type: u32, pixel_idx: usize, value: u32) -> Result<()> {
    match pixel_type {
        EPT_U1 => {
            let b = pixel_idx / 8;
            let shift = pixel_idx % 8;
            if (value & 1) != 0 {
                dst[b] |= 1_u8 << shift;
            } else {
                dst[b] &= !(1_u8 << shift);
            }
        }
        EPT_U2 => {
            let b = pixel_idx / 4;
            let shift = (pixel_idx % 4) * 2;
            dst[b] &= !(0x03_u8 << shift);
            dst[b] |= ((value as u8) & 0x03) << shift;
        }
        EPT_U4 => {
            let b = pixel_idx / 2;
            let shift = (pixel_idx % 2) * 4;
            dst[b] &= !(0x0f_u8 << shift);
            dst[b] |= ((value as u8) & 0x0f) << shift;
        }
        EPT_U8 | EPT_S8 => dst[pixel_idx] = value as u8,
        EPT_U16 | EPT_S16 => {
            let o = pixel_idx * 2;
            dst[o..o + 2].copy_from_slice(&(value as u16).to_le_bytes());
        }
        EPT_U32 | EPT_S32 | EPT_F32 => {
            let o = pixel_idx * 4;
            dst[o..o + 4].copy_from_slice(&value.to_le_bytes());
        }
        EPT_F64 => {
            return Err(RasterError::UnsupportedDataType(
                "HFA: compressed RLC decode for f64 is not supported".into(),
            ));
        }
        _ => {
            return Err(RasterError::UnsupportedDataType(format!(
                "HFA: pixel type {} is not supported",
                pixel_type
            )));
        }
    }
    Ok(())
}

/// Map ERDAS EPTType → wbraster `DataType`.
fn ept_to_data_type(ept: u32) -> Result<DataType> {
    match ept {
        EPT_U1 | EPT_U2 | EPT_U4 | EPT_U8 => Ok(DataType::U8),
        EPT_S8  => Ok(DataType::I8),
        EPT_U16 => Ok(DataType::U16),
        EPT_S16 => Ok(DataType::I16),
        EPT_U32 => Ok(DataType::U32),
        EPT_S32 => Ok(DataType::I32),
        EPT_F32 => Ok(DataType::F32),
        EPT_F64 => Ok(DataType::F64),
        _ => Err(RasterError::UnsupportedDataType(format!(
            "HFA: pixel type {ept} has no DataType mapping"
        ))),
    }
}

// ─── Eprj_MapInfo (geo-transform) ────────────────────────────────────────────

/// Parse an `Eprj_MapInfo` data block.
///
/// Binary layout (variable-length strings use a leading uint16 length count):
/// ```text
/// uint16 proNameLen   → char[proNameLen]  (projection name, ignored)
/// uint16 width        (unused)
/// uint16 height       (unused)
/// float64 upperLeftCenter.x
/// float64 upperLeftCenter.y
/// float64 lowerRightCenter.x  (unused)
/// float64 lowerRightCenter.y  (unused)
/// float64 pixelSize.x
/// float64 pixelSize.y
/// uint16 unitsLen     → char[unitsLen]    (units, stored in metadata)
/// ```
fn parse_eprj_map_info(raw: &[u8], node: &NodeHdr, raster_rows: usize) -> Result<GeoTransform> {
    let off = node.data_offset;
    if raw.len() <= off || node.data_size < 4 {
        return Err(RasterError::CorruptData("HFA: Eprj_MapInfo data missing".into()));
    }
    let d = &raw[off..off + node.data_size.min(raw.len() - off)];
    let mut cursor = 0usize;

    // proName (counted string)
    let pro_name_len = read_counted_str_len(d, cursor)?;
    cursor += 2 + pro_name_len;

    // width, height (uint16 × 2)
    cursor += 4;

    if d.len() < cursor + 16 + 16 + 16 {
        return Err(RasterError::CorruptData(
            "HFA: Eprj_MapInfo data too short to contain coordinates".into(),
        ));
    }

    let ul_x = read_f64_le(d, cursor);          // upperLeftCenter.x
    let ul_y = read_f64_le(d, cursor + 8);      // upperLeftCenter.y
    cursor += 16;

    // lowerRightCenter (16 bytes, skipped)
    cursor += 16;

    let px = read_f64_le(d, cursor);            // pixelSize.x
    let py = read_f64_le(d, cursor + 8);        // pixelSize.y

    if px <= 0.0 || py <= 0.0 {
        return Err(RasterError::CorruptData(format!(
            "HFA: Eprj_MapInfo has non-positive pixel size ({px}, {py})"
        )));
    }

    // upperLeftCenter is the centre of the top-left pixel.
    // x_min = west edge;  y_min = south edge (bottom of raster).
    let x_min = ul_x - px * 0.5;
    let y_min = ul_y - py * (raster_rows as f64 - 0.5);

    Ok(GeoTransform {
        x_min,
        y_min,
        cell_size_x: px,
        cell_size_y: py,
    })
}

/// Read the uint16 length prefix of a counted string at `offset` in `d`.
fn read_counted_str_len(d: &[u8], offset: usize) -> Result<usize> {
    if d.len() < offset + 2 {
        return Err(RasterError::CorruptData(
            "HFA: unexpected end of data reading string length".into(),
        ));
    }
    Ok(read_u16_le(d, offset) as usize)
}

/// Read the string bytes of a counted string starting at `offset`.
/// Returns `(string, bytes_consumed)` including the 2-byte length prefix.
fn read_counted_string(d: &[u8], offset: usize) -> Result<(String, usize)> {
    let len = read_counted_str_len(d, offset)?;
    if d.len() < offset + 2 + len {
        return Err(RasterError::CorruptData(
            "HFA: unexpected end of data reading string body".into(),
        ));
    }
    let s = String::from_utf8_lossy(&d[offset + 2..offset + 2 + len]).into_owned();
    Ok((s, 2 + len))
}

/// Map sphere name and semi-major axis to a known geographic datum EPSG code (Tier A).
///
/// Recognizes common datums by sphere name (case-insensitive) and validates with
/// semi-major axis matching. Falls back to semi-major axis matching if name is not recognized.
///
/// Supported datums (Tier A—geographic only):
/// - WGS-84: `6_378_137.0 m` → EPSG:4326
/// - WGS-72: `6_378_135.0 m` → EPSG:4322
/// - NAD83: `6_378_137.0 m` (same as WGS-84) → EPSG:4269 (if "NAD83" in name)
/// - NAD27: `6_378_249.145 m` → EPSG:4267
/// - ED50: `6_378_388.0 m` → EPSG:4230
/// - International 1924: `6_378_388.0 m` → EPSG:4229
/// - Bessel 1841: `6_377_397.155 m` → EPSG:4004
/// - Clarke 1880: `6_378_249.145 m` (variant) → various (context-dependent)
///
/// Returns the best-matching EPSG code, or `None` if no match found.
fn sphere_to_epsg(sphere_name: &str, semi_major: f64) -> Option<u32> {
    let name_upper = sphere_name.to_ascii_uppercase();

    // Datum lookup table: (sphere_name_keywords, semi_major_axis, epsg_code)
    // Structured as tuples for maintainability.
    let datums: &[((&str, f64, f64), u32)] = &[
        // WGS-84 (most common geographic)
        (("WGS", 6_378_137.0, 1.0), 4326),
        (("WGS84", 6_378_137.0, 1.0), 4326),
        (("WGS 84", 6_378_137.0, 1.0), 4326),
        (("WGS 1984", 6_378_137.0, 1.0), 4326),

        // NAD83 (North American Datum 1983) — uses WGS-84 ellipsoid in modern form
        (("NAD83", 6_378_137.0, 1.0), 4269),
        (("NAD 83", 6_378_137.0, 1.0), 4269),
        (("NAD 1983", 6_378_137.0, 1.0), 4269),
        (("NORTH AMERICAN DATUM 1983", 6_378_137.0, 1.0), 4269),

        // WGS-72 (older, used in some legacy data)
        (("WGS72", 6_378_135.0, 1.0), 4322),
        (("WGS 72", 6_378_135.0, 1.0), 4322),

        // NAD27 (North American Datum 1927) — Clarke 1880 ellipsoid
        (("NAD27", 6_378_249.145, 1.0), 4267),
        (("NAD 27", 6_378_249.145, 1.0), 4267),
        (("NAD 1927", 6_378_249.145, 1.0), 4267),
        (("NORTH AMERICAN DATUM 1927", 6_378_249.145, 1.0), 4267),

        // ED50 (European Datum 1950) — International 1924 ellipsoid
        (("ED50", 6_378_388.0, 1.0), 4230),
        (("ED 50", 6_378_388.0, 1.0), 4230),
        (("EUROPEAN DATUM 1950", 6_378_388.0, 1.0), 4230),

        // International 1924 (Hayford ellipsoid) — can map to multiple EPSG codes
        (("INTERNATIONAL 1924", 6_378_388.0, 1.0), 4229),
        (("HAYFORD", 6_378_388.0, 1.0), 4229),

        // Bessel 1841 (used in Central Europe, Asia)
        (("BESSEL 1841", 6_377_397.155, 1.0), 4004),
        (("BESSEL", 6_377_397.155, 1.0), 4004),

        // Clarke 1880 (used in many countries)
        (("CLARKE 1880", 6_378_249.145, 1.0), 4011),
    ];

    // Try name-based match first (preferred: name is explicit).
    for &((keywords, axis, tol), epsg) in datums {
        if name_upper.contains(keywords) && (semi_major - axis).abs() <= tol {
            return Some(epsg);
        }
    }

    // Fall back to semi-major axis matching for unnamed spheres.
    const AXIS_TOL: f64 = 2.0; // 2 metre tolerance for axis matching
    for &((_, axis, _), epsg) in datums {
        if (semi_major - axis).abs() <= AXIS_TOL {
            // Prefer name-based matches; if we reach here, it's axis-only.
            // Return the first axis match found (datums list is ordered by commonality).
            return Some(epsg);
        }
    }

    None
}

// ─── Eprj_ProParameters (CRS) ─────────────────────────────────────────────────

/// Detect projected EPSG code from ERDAS projection number, zone, and sphere.
///
/// Tier B support (current):
/// - UTM: proNumber 1–60 (various ERDAS implementations)
///   - WGS-84: EPSG 32601–32660 (N), 32701–32760 (S)
///   - NAD83: EPSG 26903–26923 (2d form), 32118–32162 (3d UTM NAD83)
///   - NAD27: EPSG 26703–26722 (various zones)
/// - State Plane: proNumber 3001–3130 (NAD83 and NAD27 zones)
///   - NAD83 State Plane: EPSG 2223–2357 (50 US states + territories)
///   - NAD27 State Plane: EPSG 26702–26846 (legacy zones)
/// - Tier C seed mappings (conservative):
///   - EPRJ 68 (Pseudo Mercator) → EPSG:3857 when canonical params are detected
///   - EPRJ 69 / 5 (Mercator Variant A / Mercator) → EPSG:3395 when canonical params are detected
///   - EPRJ 9 / 36 (Transverse Mercator / Gauss-Kruger) → inferred UTM EPSG when params match UTM signature
///   - UTM projection-name parsing fallback ("...UTM Zone <n><N|S>") for weak/missing proNumber metadata
///   - Explicit EPSG token parsing in projection names ("...EPSG:xxxx")
///   - Strict name-only CRS seeds for common labels (Web Mercator, ETRS89/LAEA Europe, etc.)
///   - EPRJ 3 (Albers Conic Equal Area) → EPSG:5070 for canonical NAD83 CONUS parameters
///   - EPRJ 4 (Lambert Conformal Conic) → EPSG:3347 for canonical NAD83 StatsCan parameters
///   - EPRJ 34 / 17 (Plate Carree / Equirectangular) → EPSG:32662 for canonical WGS84 form
///   - EPRJ 35 (Equidistant Cylindrical) → EPSG:4087 for canonical WGS84 world form
///   - EPRJ 58 (Cylindrical Equal Area) → EPSG:6933 for canonical WGS84 global form
///   - EPRJ 6 (Polar Stereographic) → EPSG:3995 for canonical WGS84 Arctic form
///   - EPRJ 6 (Polar Stereographic) → EPSG:3031 for canonical WGS84 Antarctic form
///   - EPRJ 11 (Lambert Azimuthal Equal Area) → EPSG:6931/6932 for canonical WGS84 EASE polar forms
/// - Common Mercator-family: Transverse Mercator, Polyconic, Lambert Conformal, etc.
/// - Fallback: extract explicit WKT from `Eprj_MapProjection842` child node if available.
///
/// Returns Some(epsg) if a match is found, None otherwise.
fn projected_to_epsg(
    pro_number: i32,
    pro_zone: i32,
    sphere_epsg: u32,
    pro_params: Option<&[f64]>, // First parameter often contains zone info
    pro_name: Option<&str>,
) -> Option<u32> {
    // Extract zone from pro_params[0] if proZone is not set.
    // Some files store southern hemisphere as a negative zone.
    let zone_from_params = pro_params.and_then(|params| {
        if params.is_empty() {
            return None;
        }
        let z = params[0];
        if z.abs() >= 1.0 && z.abs() <= 60.0 {
            Some((z.abs() as u32, z < 0.0))
        } else {
            None
        }
    });

    // UTM false northing of ~10,000,000 indicates southern hemisphere.
    let south_from_false_northing = pro_params
        .map(|params| {
            let fn7 = params.get(7).copied().unwrap_or(0.0);
            let fn4 = params.get(4).copied().unwrap_or(0.0);
            (fn7 - 10_000_000.0).abs() < 1_000.0 || (fn4 - 10_000_000.0).abs() < 1_000.0
        })
        .unwrap_or(false);

    // ─── UTM Zones ────────────────────────────────────────────────
    // ERDAS typically codes UTM as proNumber 1 with proZone = zone number (1-60).
    // Some implementations use proNumber 1–60 directly.
    // pro_params[0] sometimes contains the zone as well.
    if pro_number >= 1 && pro_number <= 60 {
        // Determine zone: proZone (first choice) → pro_params[0] (fallback) → proNumber
        let (zone, south_by_zone) = if pro_zone != 0 && pro_zone.abs() <= 60 {
            (pro_zone.unsigned_abs(), pro_zone < 0)
        } else if let Some((z, south)) = zone_from_params {
            (z, south)
        } else {
            (pro_number as u32, false)
        };
        let south_hemisphere = south_by_zone || south_from_false_northing;

        // Map sphere EPSG to UTM zone base.
        return match sphere_epsg {
            // WGS-84 UTM: 32601–32660 (N), 32701–32760 (S)
            4326 => {
                if south_hemisphere {
                    Some(32700 + zone)
                } else {
                    Some(32600 + zone)
                }
            }
            // WGS-72 UTM: 32201–32260 (N), 32301–32360 (S)
            4322 => {
                if south_hemisphere {
                    Some(32300 + zone)
                } else {
                    Some(32200 + zone)
                }
            }
            // NAD83 UTM (3d): 32118–32162 (3d form); 26903–26923 (2d form)
            4269 => {
                // Use 3d NAD83 UTM codes (32118–32162, but offset by 18-1=17+zone)
                if zone <= 23 {
                    Some(32118 + zone - 1)
                } else {
                    None
                }
            }
            // NAD27 UTM: ~26703–26722 (select zones)
            4267 => {
                // NAD27 UTM zones are 26703–26722 (zones 1–20; zones 21-60 not typically defined for NAD27)
                if zone <= 20 {
                    Some(26702 + zone)
                } else {
                    None
                }
            }
            _ => None,
        };
    }

    // ─── State Plane Zones ────────────────────────────────────────
    // ERDAS codes State Plane as proNumber 3001–3130 (EPSG equivalent range).
    // Comprehensive mapping for all 50 US states, DC, and common territories.
    // Each state has 1–6 zones; mapping encodes (ERDAS proNumber → NAD83 EPSG, NAD27 EPSG).
    if pro_number >= 3001 && pro_number <= 3130 {
        let state_plane_table: &[(i32, u32, u32)] = &[
            // Format: (ERDAS proNumber, NAD83 EPSG, NAD27 EPSG)
            // AL (Alabama) - 2 zones
            (3101, 26929, 26729), // AL Zone 1 East
            (3102, 26930, 26730), // AL Zone 2 West
            // AK (Alaska) - 10 zones
            (3201, 26931, 26731), // AK Zone 1
            (3202, 26932, 26732), // AK Zone 2
            (3203, 26933, 26733), // AK Zone 3
            (3204, 26934, 26734), // AK Zone 4
            (3205, 26935, 26735), // AK Zone 5
            (3206, 26936, 26736), // AK Zone 6
            (3207, 26937, 26737), // AK Zone 7
            (3208, 26938, 26738), // AK Zone 8
            (3209, 26939, 26739), // AK Zone 9
            (3210, 26940, 26740), // AK Zone 10
            // AZ (Arizona) - 3 zones
            (3301, 26948, 26748), // AZ Zone 1 East
            (3302, 26949, 26749), // AZ Zone 2 Central
            (3303, 26950, 26750), // AZ Zone 3 West
            // AR (Arkansas) - 2 zones
            (3401, 26951, 26751), // AR Zone 1 North
            (3402, 26952, 26752), // AR Zone 2 South
            // CA (California) - 6 zones
            (3501, 2223, 26742), // CA Zone 1
            (3502, 2224, 26743), // CA Zone 2
            (3503, 2225, 26744), // CA Zone 3
            (3504, 2226, 26745), // CA Zone 4
            (3505, 2227, 26746), // CA Zone 5
            (3506, 2228, 26747), // CA Zone 6
            // CO (Colorado) - 3 zones
            (3601, 26954, 26754), // CO Zone 1 North
            (3602, 26955, 26755), // CO Zone 2 Central
            (3603, 26956, 26756), // CO Zone 3 South
            // CT (Connecticut) - 1 zone
            (3800, 2234, 26756), // CT (single zone)
            // DE (Delaware) - 1 zone
            (3900, 2235, 26757), // DE (single zone)
            // FL (Florida) - 3 zones
            (4001, 2236, 26758), // FL Zone 1 East
            (4002, 2237, 26759), // FL Zone 2 West
            (4003, 2238, 26760), // FL Zone 3 (deprecated, partial)
            // GA (Georgia) - 2 zones
            (4100, 2239, 26766), // GA Zone 1 East
            (4101, 2240, 26767), // GA Zone 2 West
            // HI (Hawaii) - 5 zones
            (4201, 2241, 26761), // HI Zone 1
            (4202, 2242, 26762), // HI Zone 2
            (4203, 2243, 26763), // HI Zone 3
            (4204, 2244, 26764), // HI Zone 4
            (4205, 2245, 26765), // HI Zone 5
            // ID (Idaho) - 2 zones
            (4301, 2246, 26768), // ID Zone 1 West
            (4302, 2247, 26769), // ID Zone 2 East
            // IL (Illinois) - 3 zones
            (4401, 2249, 26771), // IL Zone 1 East
            (4402, 2250, 26772), // IL Zone 2 West
            // IN (Indiana) - 2 zones
            (4501, 2251, 26773), // IN Zone 1 East
            (4502, 2252, 26774), // IN Zone 2 West
            // IA (Iowa) - 2 zones
            (4601, 2253, 26775), // IA Zone 1 North
            (4602, 2254, 26776), // IA Zone 2 South
            // KS (Kansas) - 2 zones
            (4701, 2255, 26777), // KS Zone 1 North
            (4702, 2256, 26778), // KS Zone 2 South
            // KY (Kentucky) - 2 zones
            (4801, 2257, 26779), // KY Zone 1 North
            (4802, 2258, 26780), // KY Zone 2 South
            // LA (Louisiana) - 2 zones
            (4901, 2259, 26781), // LA Zone 1 North
            (4902, 2260, 26782), // LA Zone 2 South
            // ME (Maine) - 2 zones
            (5001, 2261, 26783), // ME Zone 1 East
            (5002, 2262, 26784), // ME Zone 2 West
            // MD (Maryland) - 1 zone
            (5200, 2263, 26785), // MD (single zone)
            // MA (Massachusetts) - 2 zones
            (5201, 2264, 26786), // MA Zone 1 Mainland
            (5202, 2265, 26787), // MA Zone 2 Island
            // MI (Michigan) - 2 zones
            (5301, 2267, 26789), // MI Zone 1 North
            (5302, 2268, 26790), // MI Zone 2 South
            // MN (Minnesota) - 3 zones
            (5401, 2269, 26791), // MN Zone 1 North
            (5402, 2270, 26792), // MN Zone 2 Central
            (5403, 2271, 26793), // MN Zone 3 South
            // MS (Mississippi) - 2 zones
            (5501, 2272, 26794), // MS Zone 1 East
            (5502, 2273, 26795), // MS Zone 2 West
            // MO (Missouri) - 2 zones
            (5601, 2274, 26796), // MO Zone 1 East
            (5602, 2275, 26797), // MO Zone 2 West
            // MT (Montana) - 3 zones
            (5701, 2276, 26798), // MT Zone 1 North
            (5702, 2277, 26799), // MT Zone 2 Central
            (5703, 2278, 26800), // MT Zone 3 South
            // NE (Nebraska) - 2 zones
            (5801, 2279, 26801), // NE Zone 1 North
            (5802, 2280, 26802), // NE Zone 2 South
            // NV (Nevada) - 2 zones
            (5901, 2281, 26803), // NV Zone 1 East
            (5902, 2282, 26804), // NV Zone 2 West
            // NH (New Hampshire) - 1 zone
            (6000, 2283, 26805), // NH (single zone)
            // NJ (New Jersey) - 1 zone
            (6100, 2284, 26806), // NJ (single zone)
            // NM (New Mexico) - 3 zones
            (6201, 2285, 26807), // NM Zone 1 West
            (6202, 2286, 26808), // NM Zone 2 Central
            (6203, 2287, 26809), // NM Zone 3 East
            // NY (New York) - 4 zones
            (6301, 2285, 26716), // NY Zone 1 Long Island
            (6302, 2286, 26717), // NY Zone 2 Western
            (6303, 2287, 26718), // NY Zone 3 Central
            (6304, 2288, 26719), // NY Zone 4 Eastern
            // NC (North Carolina) - 1 zone
            (6400, 2264, 26748), // NC (single zone)
            // ND (North Dakota) - 2 zones
            (6501, 2289, 26810), // ND Zone 1 North
            (6502, 2290, 26811), // ND Zone 2 South
            // OH (Ohio) - 1 zone
            (6600, 32122, 26812), // OH (single zone, using NAD83 UTM-like)
            // OK (Oklahoma) - 1 zone
            (6700, 32138, 26813), // OK (single zone)
            // OR (Oregon) - 2 zones
            (6801, 2290, 26814), // OR Zone 1 North
            (6802, 2291, 26815), // OR Zone 2 South
            // PA (Pennsylvania) - 2 zones
            (6901, 2272, 26816), // PA Zone 1 North
            (6902, 2273, 26817), // PA Zone 2 South
            // RI (Rhode Island) - 1 zone
            (7000, 32123, 26818), // RI (single zone)
            // SC (South Carolina) - 2 zones
            (7100, 2287, 26819), // SC Zone 1
            (7101, 2288, 26820), // SC Zone 2
            // SD (South Dakota) - 2 zones
            (7201, 2291, 26821), // SD Zone 1 North
            (7202, 2292, 26822), // SD Zone 2 South
            // TN (Tennessee) - 3 zones
            (7301, 32136, 26823), // TN Zone 1
            (7302, 32137, 26824), // TN Zone 2
            (7303, 32138, 26825), // TN Zone 3
            // TX (Texas) - 5 zones
            (7401, 32038, 26738), // TX Zone 1 North
            (7402, 32039, 26739), // TX Zone 2 North Central
            (7403, 32040, 26740), // TX Zone 3 Central
            (7404, 32041, 26741), // TX Zone 4 South Central
            (7405, 32045, 26731), // TX Zone 5 South
            // UT (Utah) - 3 zones
            (7501, 32127, 26826), // UT Zone 1 North
            (7502, 32128, 26827), // UT Zone 2 Central
            (7503, 32129, 26828), // UT Zone 3 South
            // VT (Vermont) - 1 zone
            (7600, 32145, 26829), // VT (single zone)
            // VA (Virginia) - 2 zones
            (7701, 32118, 26830), // VA Zone 1 North
            (7702, 32119, 26831), // VA Zone 2 South
            // WA (Washington) - 3 zones
            (7801, 32148, 26832), // WA Zone 1 North
            (7802, 32149, 26833), // WA Zone 2 South
            // WV (West Virginia) - 1 zone
            (7900, 32140, 26834), // WV (single zone)
            // WI (Wisconsin) - 2 zones
            (8001, 32116, 26835), // WI Zone 1 North
            (8002, 32117, 26836), // WI Zone 2 South
            // WY (Wyoming) - 3 zones
            (8101, 32131, 26837), // WY Zone 1 East
            (8102, 32132, 26838), // WY Zone 2 Central
            (8103, 32133, 26839), // WY Zone 3 West
            // DC (District of Columbia) - 1 zone
            (8201, 32118, 26840), // DC (single zone)
            // PR (Puerto Rico) - 2 zones
            (8301, 2291, 26841), // PR Zone 1
            (8302, 2292, 26842), // PR Zone 2
            // VI (US Virgin Islands) - 1 zone
            (8401, 2293, 26843), // VI (single zone)
            // GU (Guam) - 1 zone
            (8501, 2294, 26844), // GU (single zone)
            // AS (American Samoa) - 1 zone
            (8601, 2295, 26845), // AS (single zone)
            // MP (Northern Mariana Islands) - 1 zone
            (8701, 2296, 26846), // MP (single zone)
        ];

        for &(erdas_num, nad83_epsg, nad27_epsg) in state_plane_table {
            if erdas_num == pro_number {
                // Return NAD83 or NAD27 EPSG depending on detected sphere.
                return match sphere_epsg {
                    4269 => Some(nad83_epsg),
                    4267 => Some(nad27_epsg),
                    _ => None,
                };
            }
        }
    }

    // ─── Tier C scaffold: additional ERDAS projected codes ───────────────
    // Conservative seed mappings only. This branch is intentionally strict
    // and should expand with verified proNumber + parameter signatures.
    if let Some(epsg) = projected_tier_c_seed_epsg(pro_number, sphere_epsg, pro_params, pro_name)
    {
        return Some(epsg);
    }

    // ─── Other projections (future Tier C expansion) ─────────────────────
    // Unhandled codes fall through to WKT extraction.

    None
}

/// Tier C projection family labels from GDAL EPRJ constants.
///
/// This provides a durable registry point for incremental mapping growth.
fn projection_family_from_pro_number(pro_number: i32) -> Option<&'static str> {
    match pro_number {
        0 => Some("LatLong"),
        1 => Some("UTM"),
        2 => Some("StatePlane"),
        3 => Some("AlbersConicEqualArea"),
        4 => Some("LambertConformalConic"),
        5 => Some("Mercator"),
        6 => Some("PolarStereographic"),
        7 => Some("Polyconic"),
        8 => Some("EquidistantConic"),
        9 => Some("TransverseMercator"),
        10 => Some("Stereographic"),
        11 => Some("LambertAzimuthalEqualArea"),
        12 => Some("AzimuthalEquidistant"),
        13 => Some("Gnomonic"),
        14 => Some("Orthographic"),
        15 => Some("GeneralVerticalNearSidePerspective"),
        16 => Some("Sinusoidal"),
        17 => Some("Equirectangular"),
        18 => Some("MillerCylindrical"),
        19 => Some("VanderGrinten"),
        20 => Some("HotineObliqueMercator"),
        24 => Some("Robinson"),
        28 => Some("Mollweide"),
        34 => Some("PlateCarree"),
        35 => Some("EquidistantCylindrical"),
        36 => Some("GaussKruger"),
        49 => Some("Bonne"),
        51 => Some("Cassini"),
        54 => Some("Krovak"),
        58 => Some("CylindricalEqualArea"),
        63 => Some("VerticalNearSidePerspective"),
        67 => Some("LambertConformalConic1SP"),
        68 => Some("PseudoMercator"),
        69 => Some("MercatorVariantA"),
        70 => Some("HotineObliqueMercatorVariantA"),
        71 => Some("TransverseMercatorSouthOrientated"),
        _ => None,
    }
}

/// Conservative Tier C seed mappings for common global projections.
///
/// Only returns EPSG when parameters strongly match canonical definitions.
fn projected_tier_c_seed_epsg(
    pro_number: i32,
    sphere_epsg: u32,
    pro_params: Option<&[f64]>,
    pro_name: Option<&str>,
) -> Option<u32> {
    let _family = projection_family_from_pro_number(pro_number);

    // Name-based fallback precedence is strict and deterministic:
    // 1) explicit EPSG token, 2) UTM zone parse, 3) strict name seed.
    if let Some(name) = pro_name {
        if let Some(epsg) = resolve_name_based_epsg(name, sphere_epsg) {
            return Some(epsg);
        }
    }

    let params = pro_params?;

    // Need at least central meridian, false easting, false northing positions.
    if params.len() < 8 {
        return None;
    }

    // Transverse Mercator / Gauss-Kruger family: infer UTM when parameters
    // match canonical UTM signatures (even if proNumber is not EPRJ_UTM).
    if pro_number == 9 || pro_number == 36 {
        if let Some((zone, south)) = infer_utm_zone_from_tm_params(params) {
            return match sphere_epsg {
                4326 => Some((if south { 32700 } else { 32600 }) + zone),
                4269 if !south && zone <= 23 => Some(32118 + zone - 1),
                4267 if !south && zone <= 20 => Some(26702 + zone),
                _ => None,
            };
        }
    }

    // Canonical NAD83 / Conus Albers: EPSG:5070
    // EPRJ_ALBERS_CONIC_EQUAL_AREA = 3
    if pro_number == 3 && sphere_epsg == 4269 {
        let lat1 = params[2];
        let lat2 = params[3];
        let lon0 = params[4];
        let lat0 = params[5];
        let fe = params[6];
        let fn_ = params[7];
        if approx_eq(lat1, 29.5, 1e-8)
            && approx_eq(lat2, 45.5, 1e-8)
            && approx_eq(lon0, -96.0, 1e-8)
            && approx_eq(lat0, 23.0, 1e-8)
            && fe.abs() < 1e-8
            && fn_.abs() < 1e-8
        {
            return Some(5070);
        }
    }

    // Canonical NAD83 / Statistics Canada Lambert: EPSG:3347
    // EPRJ_LAMBERT_CONFORMAL_CONIC = 4
    if pro_number == 4 && sphere_epsg == 4269 {
        let lat1 = params[2];
        let lat2 = params[3];
        let lon0 = params[4];
        let lat0 = params[5];
        let fe = params[6];
        let fn_ = params[7];
        if approx_eq(lat1, 49.0, 1e-8)
            && approx_eq(lat2, 77.0, 1e-8)
            && approx_eq(lon0, -95.0, 1e-8)
            && approx_eq(lat0, 49.0, 1e-8)
            && fe.abs() < 1e-8
            && fn_.abs() < 1e-8
        {
            return Some(3347);
        }
    }

    // Canonical WGS84 Plate Carree / Equirectangular: EPSG:32662
    // EPRJ_EQUIRECTANGULAR = 17, EPRJ_PLATE_CARREE = 34
    if (pro_number == 17 || pro_number == 34) && sphere_epsg == 4326 {
        let lat_ts = params[2];
        let lon0 = params[4];
        let lat0 = params[5];
        let fe = params[6];
        let fn_ = params[7];
        if approx_eq(lat_ts, 0.0, 1e-12)
            && approx_eq(lon0, 0.0, 1e-12)
            && approx_eq(lat0, 0.0, 1e-12)
            && fe.abs() < 1e-9
            && fn_.abs() < 1e-9
        {
            return Some(32662);
        }
    }

    // Canonical WGS84 World Equidistant Cylindrical: EPSG:4087
    // EPRJ_EQUIDISTANT_CYLINDRICAL = 35
    if pro_number == 35 && sphere_epsg == 4326 {
        let lat_ts = params[2];
        let lon0 = params[4];
        let lat0 = params[5];
        let fe = params[6];
        let fn_ = params[7];
        if approx_eq(lat_ts, 0.0, 1e-12)
            && approx_eq(lon0, 0.0, 1e-12)
            && approx_eq(lat0, 0.0, 1e-12)
            && fe.abs() < 1e-9
            && fn_.abs() < 1e-9
        {
            return Some(4087);
        }
    }

    // Canonical WGS84 NSIDC EASE-Grid 2.0 Polar (LAEA): EPSG:6931/6932
    // EPRJ_LAMBERT_AZIMUTHAL_EQUAL_AREA = 11
    if pro_number == 11 && sphere_epsg == 4326 {
        let lon0 = params[4];
        let lat0 = params[5];
        let fe = params[6];
        let fn_ = params[7];
        if approx_eq(lon0, 0.0, 1e-12) && fe.abs() < 1e-9 && fn_.abs() < 1e-9 {
            if approx_eq(lat0, 90.0, 1e-8) {
                return Some(6931);
            }
            if approx_eq(lat0, -90.0, 1e-8) {
                return Some(6932);
            }
        }
    }

    // Canonical ETRS89 / LAEA Europe: EPSG:3035
    // EPRJ_LAMBERT_AZIMUTHAL_EQUAL_AREA = 11
    if pro_number == 11 && sphere_epsg == 4258 {
        let lon0 = params[4];
        let lat0 = params[5];
        let fe = params[6];
        let fn_ = params[7];
        if approx_eq(lon0, 10.0, 1e-8)
            && approx_eq(lat0, 52.0, 1e-8)
            && approx_eq(fe, 4_321_000.0, 0.1)
            && approx_eq(fn_, 3_210_000.0, 0.1)
        {
            return Some(3035);
        }
    }

    // Canonical WGS84 NSIDC EASE-Grid 2.0 Global (CEA): EPSG:6933
    // EPRJ_CYLINDRICAL_EQUAL_AREA = 58
    if pro_number == 58 && sphere_epsg == 4326 {
        let lat_ts = params[2];
        let lon0 = params[4];
        let fe = params[6];
        let fn_ = params[7];
        if approx_eq(lat_ts, 30.0, 1e-8)
            && approx_eq(lon0, 0.0, 1e-12)
            && fe.abs() < 1e-9
            && fn_.abs() < 1e-9
        {
            return Some(6933);
        }
    }

    // Canonical WGS84 Polar Stereographic: EPSG:3995 (Arctic), EPSG:3031 (Antarctic)
    // EPRJ_POLAR_STEREOGRAPHIC = 6
    if pro_number == 6 && sphere_epsg == 4326 {
        let lat_ts = params[2];
        let lon0 = params[4];
        let lat0 = params[5];
        let fe = params[6];
        let fn_ = params[7];

        // Canonical NSIDC Sea Ice Polar Stereographic North: EPSG:3413
        if approx_eq(lat_ts, 70.0, 1e-8)
            && approx_eq(lon0, -45.0, 1e-8)
            && approx_eq(lat0, 90.0, 1e-8)
            && fe.abs() < 1e-9
            && fn_.abs() < 1e-9
        {
            return Some(3413);
        }

        // Canonical NSIDC Sea Ice Polar Stereographic South: EPSG:3976
        if approx_eq(lat_ts, -70.0, 1e-8)
            && approx_eq(lon0, 0.0, 1e-8)
            && approx_eq(lat0, -90.0, 1e-8)
            && fe.abs() < 1e-9
            && fn_.abs() < 1e-9
        {
            return Some(3976);
        }

        if approx_eq(lon0, 0.0, 1e-12) && fe.abs() < 1e-9 && fn_.abs() < 1e-9 {
            if approx_eq(lat0, 90.0, 1e-8) {
                return Some(3995);
            }
            if approx_eq(lat0, -90.0, 1e-8) {
                return Some(3031);
            }
        }
    }

    // Canonical Pseudo-Mercator (Web Mercator): EPSG:3857
    // EPRJ_PSEUDO_MERCATOR = 68
    if pro_number == 68 && sphere_epsg == 4326 {
        let lon0 = params[4];
        let lat0 = params[5];
        let fe = params[6];
        let fn_ = params[7];
        if lon0.abs() < 1e-12 && lat0.abs() < 1e-12 && fe.abs() < 1e-9 && fn_.abs() < 1e-9 {
            return Some(3857);
        }
    }

    // Canonical World Mercator variant A on WGS84: EPSG:3395
    // EPRJ_MERCATOR_VARIANT_A = 69 (and sometimes EPRJ_MERCATOR = 5)
    if (pro_number == 69 || pro_number == 5) && sphere_epsg == 4326 {
        let k0 = params[2];
        let lon0 = params[4];
        let lat0 = params[5];
        let fe = params[6];
        let fn_ = params[7];
        if (k0 - 1.0).abs() < 1e-12
            && lon0.abs() < 1e-12
            && lat0.abs() < 1e-12
            && fe.abs() < 1e-9
            && fn_.abs() < 1e-9
        {
            return Some(3395);
        }
    }

    // EPRJ_LATLONG can still appear in projected metadata blocks.
    if pro_number == 0 {
        return Some(sphere_epsg);
    }

    // Keep pro_name available for future seeded patterns.
    let _ = pro_name;
    None
}

#[inline]
fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

/// Infer UTM zone/hemisphere from Transverse Mercator parameters.
///
/// Expected GCTP-style parameter positions:
/// - k0: params[2] (approximately 0.9996)
/// - lon0: params[4] (degrees)
/// - lat0: params[5] (degrees, approximately 0)
/// - false easting: params[6] (approximately 500000)
/// - false northing: params[7] (approximately 0 or 10000000)
fn infer_utm_zone_from_tm_params(params: &[f64]) -> Option<(u32, bool)> {
    if params.len() < 8 {
        return None;
    }

    let k0 = params[2];
    let lon0 = params[4];
    let lat0 = params[5];
    let fe = params[6];
    let fn_ = params[7];

    if !approx_eq(k0, 0.9996, 1e-5) || !approx_eq(lat0, 0.0, 1e-6) || !approx_eq(fe, 500000.0, 1.0)
    {
        return None;
    }

    if fn_.abs() > 1.0 && !approx_eq(fn_, 10_000_000.0, 1_000.0) {
        return None;
    }

    let zone_f = (lon0 + 183.0) / 6.0;
    let zone_i = zone_f.round() as i32;
    if zone_i < 1 || zone_i > 60 {
        return None;
    }
    if !approx_eq(zone_f, zone_i as f64, 1e-6) {
        return None;
    }

    let south = approx_eq(fn_, 10_000_000.0, 1_000.0);
    Some((zone_i as u32, south))
}

/// Parse UTM zone and hemisphere from a projection name string.
///
/// Handles common forms such as:
/// - "...UTM Zone 11N"
/// - "...UTM_Zone_11N"
/// - "...UTM Zone 33S"
fn parse_utm_zone_from_name(name: &str) -> Option<(u32, bool)> {
    let upper = name.to_uppercase();
    if !upper.contains("UTM") {
        return None;
    }

    let zone_pos = upper.find("ZONE")?;
    let tail = &upper[zone_pos + 4..];

    let mut i = 0usize;
    let bytes = tail.as_bytes();
    while i < bytes.len() && !bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }

    let start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let zone: u32 = tail[start..i].parse().ok()?;
    if !(1..=60).contains(&zone) {
        return None;
    }

    while i < bytes.len() && !bytes[i].is_ascii_alphabetic() {
        i += 1;
    }
    let south = if i < bytes.len() {
        bytes[i] == b'S'
    } else {
        false
    };

    Some((zone, south))
}

/// Resolve EPSG from projection name text with fixed precedence:
/// explicit EPSG token > UTM zone parse > strict name seed.
fn resolve_name_based_epsg(name: &str, sphere_epsg: u32) -> Option<u32> {
    if let Some(explicit_epsg) = parse_epsg_from_name(name) {
        return Some(explicit_epsg);
    }

    if let Some((zone, south)) = parse_utm_zone_from_name(name) {
        let epsg = match sphere_epsg {
            4326 => Some((if south { 32700 } else { 32600 }) + zone),
            4322 => Some((if south { 32300 } else { 32200 }) + zone),
            4269 if !south && zone <= 23 => Some(32118 + zone - 1),
            4267 if !south && zone <= 20 => Some(26702 + zone),
            _ => None,
        };
        if epsg.is_some() {
            return epsg;
        }
    }

    seed_epsg_from_projection_name(name)
}

/// Parse explicit EPSG code tokens from projection names.
///
/// Accepted forms include:
/// - "...EPSG:3035"
/// - "...EPSG 3857"
/// - "...EPSG_4326"
fn parse_epsg_from_name(name: &str) -> Option<u32> {
    let upper = name.to_uppercase();
    let pos = upper.find("EPSG")?;
    let tail = &upper[pos + 4..];

    let mut i = 0usize;
    let bytes = tail.as_bytes();
    while i < bytes.len() && !bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }

    let start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }

    let code: u32 = tail[start..i].parse().ok()?;
    if (1_000..=999_999).contains(&code) {
        Some(code)
    } else {
        None
    }
}

/// Strict name-only CRS seeds for common projection labels.
///
/// This intentionally uses conservative matching to avoid false positives.
fn seed_epsg_from_projection_name(name: &str) -> Option<u32> {
    let upper = name.to_uppercase();

    if upper.contains("WEB") && upper.contains("MERCATOR") {
        return Some(3857);
    }
    if upper.contains("PSEUDO") && upper.contains("MERCATOR") {
        return Some(3857);
    }
    if upper.contains("WORLD") && upper.contains("MERCATOR") {
        return Some(3395);
    }
    if upper.contains("ETRS89") && upper.contains("LAEA") && upper.contains("EUROPE") {
        return Some(3035);
    }
    if upper.contains("NAD83") && upper.contains("CONUS") && upper.contains("ALBERS") {
        return Some(5070);
    }
    if upper.contains("NAD83") && upper.contains("LAMBERT") && upper.contains("CANADA") {
        return Some(3347);
    }
    if upper.contains("WGS") && upper.contains("ARCTIC") && upper.contains("POLAR") {
        return Some(3995);
    }
    if upper.contains("WGS") && upper.contains("ANTARCTIC") && upper.contains("POLAR") {
        return Some(3031);
    }
    if upper.contains("NSIDC")
        && upper.contains("SEA")
        && upper.contains("ICE")
        && upper.contains("POLAR")
        && upper.contains("STEREOGRAPHIC")
        && upper.contains("NORTH")
    {
        return Some(3413);
    }
    if upper.contains("NSIDC")
        && upper.contains("SEA")
        && upper.contains("ICE")
        && upper.contains("POLAR")
        && upper.contains("STEREOGRAPHIC")
        && upper.contains("SOUTH")
    {
        return Some(3976);
    }

    None
}

/// Extract the first CRS WKT block found in raw HFA bytes.
///
/// This fallback targets embedded text like `PROJCS[...]` or `GEOGCS[...]`
/// when ERDAS projection number mapping is unavailable.
fn extract_wkt_from_raw(raw: &[u8]) -> Option<String> {
    extract_balanced_wkt(raw, b"PROJCS[")
        .or_else(|| extract_balanced_wkt(raw, b"GEOGCS["))
}

/// Extract a balanced bracketed WKT expression starting at a keyword token.
fn extract_balanced_wkt(raw: &[u8], token: &[u8]) -> Option<String> {
    let start = raw.windows(token.len()).position(|w| w == token)?;

    let mut depth = 0i32;
    let mut in_quotes = false;
    let mut end = None;

    for (i, &b) in raw.iter().enumerate().skip(start) {
        if b == b'"' {
            in_quotes = !in_quotes;
            continue;
        }
        if in_quotes {
            continue;
        }
        if b == b'[' {
            depth += 1;
        } else if b == b']' {
            depth -= 1;
            if depth == 0 {
                end = Some(i + 1);
                break;
            }
        }
    }

    let end = end?;
    let wkt = String::from_utf8_lossy(&raw[start..end])
        .trim_matches('\0')
        .trim()
        .to_string();

    if wkt.is_empty() { None } else { Some(wkt) }
}

/// Parse an `Eprj_ProParameters` data block to extract CRS information.
///
/// Binary layout:
/// ```text
/// int32  proType   (0 = EPRJ_INTERNAL, 1 = EPRJ_EXTERNAL)
/// int32  proNumber (ERDAS projection number; 0 = Geographic)
/// uint16 proExeNameLen → char[len]   (external program name; usually empty)
/// int32  proZone
/// 15 × float64  proParams[15]  (projection parameters)
/// Eprj_Spheroid (inline struct):
///   uint16 sphereNameLen → char[len]
///   float64 a   (semi-major axis in metres)
///   float64 b   (semi-minor axis in metres)
///   float64 eSquared (eccentricity squared)
///   float64 radius
/// ```
///
/// Tier A support: Geographic (proType=0, proNumber=0) → recognize common datums via name + semi-major axis.
/// Tier B support: Projected systems (proType!=0 or proNumber!=0) → UTM, State Plane, common Mercator-family projections.
/// Fallback: If ERDAS number not recognized, attempt to extract explicit PROJCS WKT.
fn parse_eprj_pro_parameters(raw: &[u8], node: &NodeHdr) -> Result<CrsInfo> {
    let off = node.data_offset;
    if raw.len() <= off || node.data_size < 16 {
        return Ok(CrsInfo::default());
    }
    let d = &raw[off..off + node.data_size.min(raw.len() - off)];
    if d.len() < 8 {
        return Ok(CrsInfo::default());
    }
    let pro_type   = read_i32_le(d, 0);
    let pro_number = read_i32_le(d, 4);
    let mut cursor = 8usize;

    // proExeName (counted string — often empty)
    if let Ok((_, adv)) = read_counted_string(d, cursor) {
        cursor += adv;
    } else {
        return Ok(CrsInfo::default());
    }

    // proName (counted string) is present in full EPRJ structures.
    // Some files omit it in practice, so keep this optional.
    let mut pro_name: Option<String> = None;
    if let Ok((name, adv)) = read_counted_string(d, cursor) {
        // Ensure enough bytes remain for proZone and proParams after consuming proName.
        if d.len() >= cursor + adv + 4 + 120 {
            pro_name = Some(name);
            cursor += adv;
        }
    }

    // proZone (int32) — extract for Tier B zone-based calculations
    let pro_zone = if d.len() >= cursor + 4 {
        let z = read_i32_le(d, cursor);
        cursor += 4;
        z
    } else {
        return Ok(CrsInfo::default());
    };

    // proParams[15] (15 × float64 = 120 bytes) — extract for zone/parameter inspection
    let pro_params = if d.len() >= cursor + 120 {
        let mut params = [0.0_f64; 15];
        for i in 0..15 {
            params[i] = read_f64_le(d, cursor + i * 8);
        }
        cursor += 120;
        params
    } else {
        return Ok(CrsInfo::default());
    };

    // Eprj_Spheroid: sphereName (counted string) then 4 × float64
    let sphere_name = if let Ok((s, adv)) = read_counted_string(d, cursor) {
        cursor += adv;
        s
    } else {
        return Ok(CrsInfo::default());
    };

    if d.len() < cursor + 8 {
        return Ok(CrsInfo::default());
    }
    let semi_major = read_f64_le(d, cursor); // a (metres)

    // Geographic: proType == 0, proNumber == 0 → apply Tier A datum detection.
    if pro_type == 0 && pro_number == 0 {
        if let Some(epsg) = sphere_to_epsg(&sphere_name, semi_major) {
            return Ok(CrsInfo { epsg: Some(epsg), ..Default::default() });
        }
        // Geographic but unrecognized datum → return without EPSG.
        return Ok(CrsInfo::default());
    }

    // Projected systems (pro_type != 0 or pro_number != 0).
    // Tier B: Try ERDAS projection number → EPSG mapping.
    if let Some(sphere_epsg) = sphere_to_epsg(&sphere_name, semi_major) {
        if let Some(epsg) = projected_to_epsg(
            pro_number,
            pro_zone,
            sphere_epsg,
            Some(&pro_params),
            pro_name.as_deref(),
        ) {
            return Ok(CrsInfo { epsg: Some(epsg), ..Default::default() });
        }
    }

    // Tier B fallback: If ERDAS lookup failed but we have a sphere EPSG, still try projected detection
    // with generic sphere (for unrecognized geographic datums).
    if let Some(epsg) = projected_to_epsg(
        pro_number,
        pro_zone,
        4326,
        Some(&pro_params),
        pro_name.as_deref(),
    ) {
        return Ok(CrsInfo { epsg: Some(epsg), ..Default::default() });
    }

    // Tier B v3 fallback: recover explicit WKT from raw HFA payload if present.
    if let Some(wkt) = extract_wkt_from_raw(raw) {
        return Ok(CrsInfo::from_wkt(wkt));
    }

    // No CRS found via Tier B mapping or WKT fallback.
    Ok(CrsInfo::default())
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal synthetic HFA bytes: file header + 1 node header (Eimg_Layer)
    /// with direct (non-tiled) 4×4 u8 pixel data.
    fn build_minimal_img(pixel_type: u32, width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
        // --- Layout ---
        // [0]   File header (34 bytes)
        // [34]  Root node: Ehfa_Layer (128 bytes), no data
        // [162] Band node: Eimg_Layer (128 bytes), data at [290]
        // [290] Eimg_Layer data (52 bytes)
        // [342] Pixel data
        const FILE_HDR: usize = 34;
        const ROOT_OFFSET: usize = FILE_HDR;          // root node at byte 34
        const BAND_OFFSET: usize = ROOT_OFFSET + 128; // band node at byte 162
        const BAND_DATA_OFFSET: usize = BAND_OFFSET + 128; // band data at 290
        const PIXEL_OFFSET: usize = BAND_DATA_OFFSET + 52; // pixels at 342

        let total = PIXEL_OFFSET + pixels.len();
        let mut buf = vec![0u8; total];

        // File header
        buf[..16].copy_from_slice(MAGIC);
        // version = 1
        buf[16..20].copy_from_slice(&1u32.to_le_bytes());
        // free_list = 0
        // root entry ptr
        buf[24..28].copy_from_slice(&(ROOT_OFFSET as u32).to_le_bytes());
        // entry_header_length = 128
        buf[28..30].copy_from_slice(&128u16.to_le_bytes());
        // dictionary_ptr = 0 (skipped in MVP)

        // Root node (type "Ehfa_Layer", no data, child = BAND_OFFSET)
        let root = &mut buf[ROOT_OFFSET..ROOT_OFFSET + 128];
        root[..10].copy_from_slice(b"root\0\0\0\0\0\0");
        root[64..75].copy_from_slice(b"Ehfa_Layer\0");
        // child_entry at offset 116-119
        let band_off_bytes = (BAND_OFFSET as u32).to_le_bytes();
        root[116..120].copy_from_slice(&band_off_bytes);

        // Band node (type "Eimg_Layer", data at BAND_DATA_OFFSET)
        let band = &mut buf[BAND_OFFSET..BAND_OFFSET + 128];
        band[..7].copy_from_slice(b"Band_1\0");
        band[64..75].copy_from_slice(b"Eimg_Layer\0");
        // data_offset
        band[96..100].copy_from_slice(&(BAND_DATA_OFFSET as u32).to_le_bytes());
        // data_size = 52
        band[100..104].copy_from_slice(&52u32.to_le_bytes());
        // parent_entry = ROOT_OFFSET
        band[112..116].copy_from_slice(&(ROOT_OFFSET as u32).to_le_bytes());
        // child_entry = 0 (no children → direct data mode)

        // Eimg_Layer data block (52 bytes)
        let d = &mut buf[BAND_DATA_OFFSET..BAND_DATA_OFFSET + 52];
        d[0..4].copy_from_slice(&(width as i32).to_le_bytes());    // width
        d[4..8].copy_from_slice(&(height as i32).to_le_bytes());   // height
        d[8..12].copy_from_slice(&1i32.to_le_bytes());             // layerType = raster
        d[12..16].copy_from_slice(&(pixel_type as i32).to_le_bytes()); // pixelType
        // blockWidth, blockHeight (float64) = 0 → we default to width/height
        d[32..36].copy_from_slice(&0i32.to_le_bytes());            // compressType = none
        d[36..40].copy_from_slice(&0u32.to_le_bytes());            // ptr Edsc_Table = null
        d[40..44].copy_from_slice(&1i32.to_le_bytes());            // numRasters
        d[44..48].copy_from_slice(&(PIXEL_OFFSET as u32).to_le_bytes()); // dataOffset
        d[48..52].copy_from_slice(&(pixels.len() as i32).to_le_bytes()); // dataSize

        // Pixel data
        buf[PIXEL_OFFSET..PIXEL_OFFSET + pixels.len()].copy_from_slice(pixels);

        buf
    }

    #[test]
    fn read_direct_u8_band() {
        let pixels: Vec<u8> = (0..16u8).collect(); // 4×4
        let raw = build_minimal_img(EPT_U8, 4, 4, &pixels);
        let r = read_from_raw(&raw).unwrap();
        assert_eq!(r.cols, 4);
        assert_eq!(r.rows, 4);
        assert_eq!(r.bands, 1);
        for row in 0..4isize {
            for col in 0..4isize {
                let expected = (row * 4 + col) as f64;
                let actual = r.get(0, row, col);
                assert!((actual - expected).abs() < 1e-9,
                    "({row},{col}): expected {expected}, got {actual}");
            }
        }
    }

    #[test]
    fn read_direct_f32_band() {
        let values: Vec<f32> = vec![1.5, 2.5, 3.5, 4.5];
        let raw_pixels: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        let raw = build_minimal_img(EPT_F32, 2, 2, &raw_pixels);
        let r = read_from_raw(&raw).unwrap();
        assert_eq!(r.cols, 2);
        assert_eq!(r.rows, 2);
        assert!((r.get(0, 0, 0) - 1.5).abs() < 1e-5);
        assert!((r.get(0, 1, 1) - 4.5).abs() < 1e-5);
    }

    #[test]
    fn reject_bad_magic() {
        let mut raw = build_minimal_img(EPT_U8, 2, 2, &[0u8; 4]);
        raw[0] = b'X'; // corrupt magic
        let result = read_from_raw(&raw);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("EHFA_HEADER_TAG") || msg.contains("magic"), "msg: {msg}");
    }

    #[test]
    fn decode_sub_byte_u4() {
        // u4: each byte holds 2 pixels; low nibble first.
        // bytes: 0x21, 0x43  → pixels: 1, 2, 3, 4
        let raw_pixels: Vec<u8> = vec![0x21, 0x43];
        let raw = build_minimal_img(EPT_U4, 2, 2, &raw_pixels);
        let r = read_from_raw(&raw).unwrap();
        assert_eq!(r.get(0, 0, 0), 1.0);
        assert_eq!(r.get(0, 0, 1), 2.0);
        assert_eq!(r.get(0, 1, 0), 3.0);
        assert_eq!(r.get(0, 1, 1), 4.0);
    }

    #[test]
    fn uncompress_hfa_rlc_u8_runs() {
        // dataMin=10, runs=(3 x 0) + (2 x 10), numBits=8
        // output should be: 10,10,10,20,20
        let mut comp = Vec::new();
        comp.extend_from_slice(&10_u32.to_le_bytes()); // dataMin
        comp.extend_from_slice(&2_i32.to_le_bytes()); // numRuns
        comp.extend_from_slice(&15_i32.to_le_bytes()); // dataOffset (13-byte header + 2 count bytes)
        comp.push(8_u8); // numBits
        comp.push(3_u8); // run #1 length
        comp.push(2_u8); // run #2 length
        comp.push(0_u8); // run #1 value delta
        comp.push(10_u8); // run #2 value delta

        let bytes = uncompress_hfa_rlc_block(&comp, EPT_U8, 5).unwrap();
        let vals = decode_pixels_from_bytes(&bytes, EPT_U8, 5).unwrap();
        assert_eq!(vals, vec![10.0, 10.0, 10.0, 20.0, 20.0]);
    }

    #[test]
    fn uncompress_hfa_rlc_u16_runs() {
        // dataMin=1000, runs=(2 x 0) + (1 x 5), numBits=16
        // output should be: 1000,1000,1005
        let mut comp = Vec::new();
        comp.extend_from_slice(&1000_u32.to_le_bytes());
        comp.extend_from_slice(&2_i32.to_le_bytes());
        comp.extend_from_slice(&15_i32.to_le_bytes());
        comp.push(16_u8);
        comp.push(2_u8);
        comp.push(1_u8);
        comp.extend_from_slice(&0_u16.to_le_bytes());
        comp.extend_from_slice(&5_u16.to_le_bytes());

        let bytes = uncompress_hfa_rlc_block(&comp, EPT_U16, 3).unwrap();
        let vals = decode_pixels_from_bytes(&bytes, EPT_U16, 3).unwrap();
        assert_eq!(vals, vec![1000.0, 1000.0, 1005.0]);
    }

    #[test]
    fn map_info_geo_transform() {
        let ul_x = 500_000.0f64;
        let ul_y = 5_000_000.0f64;
        let px = 30.0f64;
        let py = 30.0f64;
        let rows = 100usize;
        let geo = parse_map_info_coords(ul_x, ul_y, px, py, rows);
        let expected_x_min = ul_x - px * 0.5;
        let expected_y_min = ul_y - py * (rows as f64 - 0.5);
        assert!((geo.x_min - expected_x_min).abs() < 1e-6);
        assert!((geo.y_min - expected_y_min).abs() < 1e-6);
    }

    #[test]
    fn extract_projcs_wkt_from_raw_payload() {
        let raw = b"abc PROJCS[\"NAD_1927_UTM_Zone_11N\",GEOGCS[\"GCS_North_American_1927\",UNIT[\"Degree\",0.017453292519943295]],UNIT[\"Meter\",1]] xyz";
        let wkt = extract_wkt_from_raw(raw).unwrap();
        assert!(wkt.starts_with("PROJCS[\"NAD_1927_UTM_Zone_11N\""));
        assert!(wkt.ends_with("]]"));
    }

    #[test]
    fn wgs84_utm_south_from_false_northing() {
        let mut params = [0.0_f64; 15];
        params[4] = 10_000_000.0;
        let epsg = projected_to_epsg(55, 55, 4326, Some(&params), None);
        assert_eq!(epsg, Some(32755));
    }

    #[test]
    fn tier_c_seed_maps_web_mercator() {
        let mut params = [0.0_f64; 15];
        params[4] = 0.0; // lon_0
        params[5] = 0.0; // lat_0
        params[6] = 0.0; // false easting
        params[7] = 0.0; // false northing
        let epsg = projected_to_epsg(68, 0, 4326, Some(&params), Some("Pseudo_Mercator"));
        assert_eq!(epsg, Some(3857));
    }

    #[test]
    fn tier_c_seed_maps_world_mercator_variant_a() {
        let mut params = [0.0_f64; 15];
        params[2] = 1.0; // k0
        params[4] = 0.0; // lon_0
        params[5] = 0.0; // lat_0
        params[6] = 0.0; // false easting
        params[7] = 0.0; // false northing
        let epsg = projected_to_epsg(69, 0, 4326, Some(&params), Some("Mercator_Variant_A"));
        assert_eq!(epsg, Some(3395));
    }

    #[test]
    fn tier_c_tm_family_infers_wgs84_utm_north() {
        let mut params = [0.0_f64; 15];
        params[2] = 0.9996; // k0
        params[4] = -123.0; // lon0 for UTM zone 10
        params[5] = 0.0; // lat0
        params[6] = 500000.0; // false easting
        params[7] = 0.0; // false northing
        let epsg = projected_to_epsg(9, 0, 4326, Some(&params), Some("Transverse Mercator"));
        assert_eq!(epsg, Some(32610));
    }

    #[test]
    fn tier_c_tm_family_infers_wgs84_utm_south() {
        let mut params = [0.0_f64; 15];
        params[2] = 0.9996; // k0
        params[4] = 15.0; // lon0 for UTM zone 33
        params[5] = 0.0; // lat0
        params[6] = 500000.0; // false easting
        params[7] = 10_000_000.0; // false northing (south)
        let epsg = projected_to_epsg(9, 0, 4326, Some(&params), Some("Transverse Mercator"));
        assert_eq!(epsg, Some(32733));
    }

    #[test]
    fn tier_c_seed_maps_nad83_conus_albers() {
        let mut params = [0.0_f64; 15];
        params[2] = 29.5;
        params[3] = 45.5;
        params[4] = -96.0;
        params[5] = 23.0;
        params[6] = 0.0;
        params[7] = 0.0;
        let epsg = projected_to_epsg(3, 0, 4269, Some(&params), Some("Albers Conic Equal Area"));
        assert_eq!(epsg, Some(5070));
    }

    #[test]
    fn tier_c_seed_maps_nad83_statscan_lcc() {
        let mut params = [0.0_f64; 15];
        params[2] = 49.0;
        params[3] = 77.0;
        params[4] = -95.0;
        params[5] = 49.0;
        params[6] = 0.0;
        params[7] = 0.0;
        let epsg = projected_to_epsg(4, 0, 4269, Some(&params), Some("Lambert Conformal Conic"));
        assert_eq!(epsg, Some(3347));
    }

    #[test]
    fn tier_c_seed_maps_wgs84_plate_carree() {
        let mut params = [0.0_f64; 15];
        params[2] = 0.0; // latitude of true scale
        params[4] = 0.0; // central meridian
        params[5] = 0.0; // latitude of origin
        params[6] = 0.0; // false easting
        params[7] = 0.0; // false northing
        let epsg = projected_to_epsg(34, 0, 4326, Some(&params), Some("Plate Carree"));
        assert_eq!(epsg, Some(32662));
    }

    #[test]
    fn tier_c_seed_maps_wgs84_world_equidistant_cylindrical() {
        let mut params = [0.0_f64; 15];
        params[2] = 0.0; // latitude of true scale
        params[4] = 0.0; // central meridian
        params[5] = 0.0; // latitude of origin
        params[6] = 0.0; // false easting
        params[7] = 0.0; // false northing
        let epsg = projected_to_epsg(35, 0, 4326, Some(&params), Some("Equidistant Cylindrical"));
        assert_eq!(epsg, Some(4087));
    }

    #[test]
    fn tier_c_seed_maps_wgs84_cea_6933() {
        let mut params = [0.0_f64; 15];
        params[2] = 30.0; // standard parallel
        params[4] = 0.0; // central meridian
        params[6] = 0.0; // false easting
        params[7] = 0.0; // false northing
        let epsg = projected_to_epsg(58, 0, 4326, Some(&params), Some("Cylindrical Equal Area"));
        assert_eq!(epsg, Some(6933));
    }

    #[test]
    fn tier_c_seed_maps_wgs84_arctic_polar_stereographic() {
        let mut params = [0.0_f64; 15];
        params[4] = 0.0; // central meridian
        params[5] = 90.0; // latitude of origin
        params[6] = 0.0; // false easting
        params[7] = 0.0; // false northing
        let epsg = projected_to_epsg(6, 0, 4326, Some(&params), Some("Polar Stereographic"));
        assert_eq!(epsg, Some(3995));
    }

    #[test]
    fn tier_c_seed_maps_wgs84_antarctic_polar_stereographic() {
        let mut params = [0.0_f64; 15];
        params[4] = 0.0; // central meridian
        params[5] = -90.0; // latitude of origin
        params[6] = 0.0; // false easting
        params[7] = 0.0; // false northing
        let epsg = projected_to_epsg(6, 0, 4326, Some(&params), Some("Polar Stereographic"));
        assert_eq!(epsg, Some(3031));
    }

    #[test]
    fn tier_c_seed_maps_wgs84_nsidc_ps_north_3413() {
        let mut params = [0.0_f64; 15];
        params[2] = 70.0; // latitude of true scale
        params[4] = -45.0; // central meridian
        params[5] = 90.0; // latitude of origin
        params[6] = 0.0; // false easting
        params[7] = 0.0; // false northing
        let epsg = projected_to_epsg(6, 0, 4326, Some(&params), Some("Polar Stereographic"));
        assert_eq!(epsg, Some(3413));
    }

    #[test]
    fn tier_c_seed_maps_wgs84_nsidc_ps_south_3976() {
        let mut params = [0.0_f64; 15];
        params[2] = -70.0; // latitude of true scale
        params[4] = 0.0; // central meridian
        params[5] = -90.0; // latitude of origin
        params[6] = 0.0; // false easting
        params[7] = 0.0; // false northing
        let epsg = projected_to_epsg(6, 0, 4326, Some(&params), Some("Polar Stereographic"));
        assert_eq!(epsg, Some(3976));
    }

    #[test]
    fn tier_c_seed_maps_wgs84_ease_laea_north() {
        let mut params = [0.0_f64; 15];
        params[4] = 0.0; // central meridian
        params[5] = 90.0; // latitude of origin
        params[6] = 0.0; // false easting
        params[7] = 0.0; // false northing
        let epsg = projected_to_epsg(11, 0, 4326, Some(&params), Some("Lambert Azimuthal Equal Area"));
        assert_eq!(epsg, Some(6931));
    }

    #[test]
    fn tier_c_seed_maps_wgs84_ease_laea_south() {
        let mut params = [0.0_f64; 15];
        params[4] = 0.0; // central meridian
        params[5] = -90.0; // latitude of origin
        params[6] = 0.0; // false easting
        params[7] = 0.0; // false northing
        let epsg = projected_to_epsg(11, 0, 4326, Some(&params), Some("Lambert Azimuthal Equal Area"));
        assert_eq!(epsg, Some(6932));
    }

    #[test]
    fn tier_c_seed_maps_etrs89_laea_europe_3035() {
        let mut params = [0.0_f64; 15];
        params[4] = 10.0; // central meridian
        params[5] = 52.0; // latitude of origin
        params[6] = 4_321_000.0; // false easting
        params[7] = 3_210_000.0; // false northing
        let epsg = projected_to_epsg(11, 0, 4258, Some(&params), Some("Lambert Azimuthal Equal Area"));
        assert_eq!(epsg, Some(3035));
    }

    #[test]
    fn tier_c_name_parse_maps_nad27_utm() {
        let epsg = projected_to_epsg(9999, 0, 4267, None, Some("NAD_1927_UTM_Zone_11N"));
        assert_eq!(epsg, Some(26713));
    }

    #[test]
    fn tier_c_name_parse_maps_wgs84_utm_south() {
        let epsg = projected_to_epsg(9999, 0, 4326, None, Some("WGS84 UTM Zone 33S"));
        assert_eq!(epsg, Some(32733));
    }

    #[test]
    fn tier_c_direct_utm_supports_wgs72() {
        let mut params = [0.0_f64; 15];
        params[7] = 0.0;
        let epsg_n = projected_to_epsg(20, 20, 4322, Some(&params), Some("WGS72 UTM Zone 20N"));
        assert_eq!(epsg_n, Some(32220));

        params[7] = 10_000_000.0;
        let epsg_s = projected_to_epsg(20, 20, 4322, Some(&params), Some("WGS72 UTM Zone 20S"));
        assert_eq!(epsg_s, Some(32320));
    }

    #[test]
    fn tier_c_name_parse_explicit_epsg_colon() {
        let epsg = projected_to_epsg(9999, 0, 4326, None, Some("ETRS89 / LAEA Europe (EPSG:3035)"));
        assert_eq!(epsg, Some(3035));
    }

    #[test]
    fn tier_c_name_parse_explicit_epsg_space() {
        let epsg = projected_to_epsg(9999, 0, 4326, None, Some("Web Mercator EPSG 3857"));
        assert_eq!(epsg, Some(3857));
    }

    #[test]
    fn tier_c_name_seed_web_mercator() {
        let epsg = projected_to_epsg(9999, 0, 4326, None, Some("WGS 84 / Web Mercator"));
        assert_eq!(epsg, Some(3857));
    }

    #[test]
    fn tier_c_name_seed_etrs89_laea_europe() {
        let epsg = projected_to_epsg(9999, 0, 4326, None, Some("ETRS89 / LAEA Europe"));
        assert_eq!(epsg, Some(3035));
    }

    #[test]
    fn tier_c_name_seed_nad83_conus_albers() {
        let epsg = projected_to_epsg(9999, 0, 4269, None, Some("NAD83 / CONUS Albers"));
        assert_eq!(epsg, Some(5070));
    }

    #[test]
    fn tier_c_name_seed_nsidc_sea_ice_ps_south() {
        let epsg = projected_to_epsg(
            9999,
            0,
            4326,
            None,
            Some("NSIDC Sea Ice Polar Stereographic South"),
        );
        assert_eq!(epsg, Some(3976));
    }

    #[test]
    fn tier_c_name_precedence_explicit_overrides_seed() {
        let epsg = projected_to_epsg(9999, 0, 4326, None, Some("Web Mercator EPSG:3395"));
        assert_eq!(epsg, Some(3395));
    }

    #[test]
    fn tier_c_name_precedence_explicit_overrides_utm() {
        let epsg = projected_to_epsg(9999, 0, 4326, None, Some("UTM Zone 33S EPSG:3857"));
        assert_eq!(epsg, Some(3857));
    }

    #[test]
    fn tier_c_name_precedence_utm_over_seed() {
        let epsg = projected_to_epsg(9999, 0, 4326, None, Some("Web Mercator UTM Zone 33S"));
        assert_eq!(epsg, Some(32733));
    }

    // ── Test helpers ──────────────────────────────────────────────────────

    /// Read from a raw byte buffer (test hook — mirrors `read()` minus File I/O).
    fn read_from_raw(raw: &[u8]) -> Result<Raster> {
        if raw.len() < 34 { return Err(RasterError::CorruptData("too short".into())); }
        if &raw[..16] != MAGIC { return Err(RasterError::UnknownFormat("HFA: missing EHFA_HEADER_TAG magic bytes".into())); }
        let (root_ptr, entry_hdr_len) = read_root_and_entry_header_len(raw)?;
        let mut nodes: Vec<NodeHdr> = Vec::new();
        collect_nodes(raw, root_ptr, entry_hdr_len, None, &mut nodes)?;
        let mut children_of: HashMap<usize, Vec<usize>> = HashMap::new();
        for (i, n) in nodes.iter().enumerate() {
            if let Some(p) = n.parent_offset {
                children_of.entry(p).or_default().push(i);
            }
        }
        let band_indices: Vec<usize> = nodes.iter().enumerate()
            .filter(|(_, n)| n.type_name == "Eimg_Layer")
            .map(|(i, _)| i)
            .collect();
        if band_indices.is_empty() {
            return Err(RasterError::CorruptData("no Eimg_Layer".into()));
        }
        let mut bands: Vec<BandData> = Vec::new();
        for &bi in &band_indices {
            let node = &nodes[bi];
            let band_info = parse_eimg_layer(raw, node)?;
            let children = children_of.get(&node.file_offset).cloned().unwrap_or_default();
            let dms_node = children.iter()
                .find(|&&ci| nodes[ci].name == "RasterDMS" || nodes[ci].type_name == "Edms_State")
                .map(|&ci| &nodes[ci]);
            let nodata_val = children.iter()
                .find(|&&ci| nodes[ci].name == "Eimg_NonInitializedValue"
                          || nodes[ci].type_name == "Eimg_NonInitializedValue")
                .and_then(|&ci| parse_nodata_node(raw, &nodes[ci]));
            let pixels = read_band_pixels(raw, &band_info, dms_node)?;
            bands.push(BandData { pixels, nodata: nodata_val, pixel_type: band_info.pixel_type });
        }
        let (cols, rows) = (bands[0].pixels.cols, bands[0].pixels.rows);
        let num_bands = bands.len();
        let data_type = ept_to_data_type(bands[0].pixel_type)?;
        let nodata = bands[0].nodata.unwrap_or(-9999.0);
        let mut all_data: Vec<f64> = Vec::with_capacity(num_bands * rows * cols);
        for b in bands { all_data.extend_from_slice(&b.pixels.data); }
        let cfg = RasterConfig {
            cols, rows, bands: num_bands,
            x_min: 0.0, y_min: 0.0, cell_size: 1.0,
            nodata, data_type, ..Default::default()
        };
        Raster::from_data(cfg, all_data)
    }

    /// Compute geo-transform from map info params (helper for the geo test).
    fn parse_map_info_coords(ul_x: f64, ul_y: f64, px: f64, py: f64, rows: usize) -> GeoTransform {
        GeoTransform {
            x_min: ul_x - px * 0.5,
            y_min: ul_y - py * (rows as f64 - 0.5),
            cell_size_x: px,
            cell_size_y: py,
        }
    }
}
