# LiDAR Processing

LiDAR (Light Detection and Ranging) produces dense 3D point clouds from
airborne or terrestrial laser scanning. Each point has a position (X, Y, Z),
return number, intensity, classification, and optionally colour, waveform, and
timestamp. Whitebox has one of the most comprehensive LiDAR processing
pipelines available in any open or commercial GIS platform, covering the
full chain from raw point cloud inspection through ground filtering,
gridding, structural analysis, and individual tree segmentation.

---

## LiDAR Data Model

A LiDAR file (LAS or LAZ) is a structured binary format with:

- **Header**: file-level metadata — bounding box, CRS, point count, record
  format version, generating software, and global statistics.
- **Point records**: per-point X/Y/Z, return metadata, intensity,
  classification (ground, low vegetation, buildings, water, etc.).
- **Variable length records (VLRs)**: CRS definitions, waveform lookups,
  extra byte descriptions, and custom metadata.

Modern point clouds can also be stored in **COPC** (Cloud-Optimized Point
Cloud) format — an indexed, HTTP-range-request-friendly LAS-in-COPC wrapper —
and in **E57** (the ASTM exchange format) or **PLY**. Whitebox supports all
of these through `wblidar`.

---

## Core Concepts

Before processing LiDAR data, understand these foundational terms:

- **Return number**: Which reflection from a single laser pulse. Pulse 1 is the first (often canopy top); pulse 2–5 capture midstory and ground returns.
- **Point classification**: ASPRS standard categories — ground (2), low veg (3), medium veg (4), high veg (5), buildings (6), noise (7), overlap (12), and others.
- **Intensity**: Reflectance value (0–65535) proportional to target brightness. Useful for vegetation density estimation and water detection.
- **Ground filtering**: Separating terrain points (classification 2) from vegetation and buildings; critical for accurate digital terrain models (DTMs).
- **Digital terrain model (DTM)**: Raster surface of bare earth, computed from ground returns only. Used for hydrology, geomorphometry, and flood modelling.
- **Digital surface model (DSM)**: Raster surface of highest returns (canopy top). Used for building detection and volume calculations.
- **Canopy height model (CHM)**: DSM minus DTM; represents vegetation height above ground. Standard input for tree detection and segmentation.
- **Point density**: Points per square unit (typically points/m²). Higher density enables finer segmentation; lower density requires smoothing.
- **Normalization**: Converting raw Z-values to height-above-ground by subtracting DTM, creating a normalized point cloud for structural analysis.

---

## Reading and Inspecting LiDAR Data

### Reading a Single File

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()

lidar = wbe.read_lidar('survey_tile.laz')
header = lidar.metadata()

print(f"Points:     {header.number_of_points}")
print(f"Bounds:     {header.min_x:.2f}, {header.min_y:.2f} to {header.max_x:.2f}, {header.max_y:.2f}")
print(f"Z range:    {header.min_z:.2f} – {header.max_z:.2f}")
print(f"Point fmt:  {header.point_format}")
print(f"CRS:        {header.wkt}")
```

### Inspecting Individual Point Records

You can iterate over point records for custom filtering or analysis. For
large files prefer the NumPy bridge or chunked streaming (see below).

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
lidar = wbe.read_lidar('tile.laz')

ground_count = 0
for i in range(lidar.header.number_of_points):
    pd, time, colour, waveform = lidar.get_point_record(i)
    if pd.classification == 2:   # 2 = ground in ASPRS classification
        ground_count += 1

print(f"Ground points: {ground_count}")
```

### NumPy Bridge

For vectorised analysis, convert the entire point cloud to a structured
NumPy array:

```python
import whitebox_workflows as wbw
import numpy as np

wbe = wbw.WbEnvironment()
lidar = wbe.read_lidar('tile.laz')

arr = lidar.to_numpy(['x', 'y', 'z', 'intensity', 'classification'])

# Z statistics for ground points
ground_z = arr['z'][arr['classification'] == 2]
print(f"Ground Z mean: {ground_z.mean():.2f}")
print(f"Ground Z std:  {ground_z.std():.2f}")
```

### Tile Footprints

When working with a tiled survey, produce a vector index of tile footprints
before processing:

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()

footprints = wbe.lidar_tile_footprint(
    input_directory='lidar_tiles/',
    output='tile_index.gpkg'
)
wbe.write_vector(footprints, 'tile_index.gpkg')
```

---

## Ground Point Filtering

Separating ground returns from vegetation, buildings, and other objects is the
most critical pre-processing step for DEM generation. Whitebox provides
several algorithms.

### Progressive Morphological Filter (PMF) / IMProved Ground Point Filter

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
lidar = wbe.read_lidar('raw_tile.laz')

# Classify ground returns using the improved ground point filter
lidar_classified = wbe.improved_ground_point_filter(
    lidar,
    radius=0.5,       # initial search radius in metres
    min_z_range=0.1,  # minimum Z variation to be non-ground
    max_z_range=30.0  # maximum above-ground feature height
)

wbe.write_lidar(lidar_classified, 'tile_classified.laz')
```

### Filtering by Percentile

Extract the subset of points that fall below a chosen height percentile
within each local neighbourhood — useful for identifying near-ground returns
without full classification:

```python
near_ground = wbe.filter_lidar_by_percentile(
    lidar,
    radius=2.0,
    percentile=5.0   # lowest 5% of heights locally
)
wbe.write_lidar(near_ground, 'near_ground_returns.laz')
```

### Filtering by Reference Surface

Remove points above (or below) a reference raster surface by a tolerance:

```python
ground_ref = wbe.read_raster('dtm_existing.tif')

filtered = wbe.filter_lidar_by_reference_surface(
    lidar,
    reference=ground_ref,
    threshold=0.5,   # metres above reference surface
    criterion='above'
)
wbe.write_lidar(filtered, 'below_reference.laz')
```

---

## Gridding: From Point Cloud to Raster

Gridding (or interpolation) converts the discrete point cloud into a
continuous raster surface.

### DTM — Digital Terrain Model

Interpolate from ground-classified returns to produce the bare-earth surface:

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
lidar = wbe.read_lidar('tile_classified.laz')

dtm = wbe.lidar_tin_gridding(
    lidar,
    parameter='elevation',
    returns_included='last',   # last returns best represent ground
    resolution=1.0,            # output cell size in metres
    exclude_cls='0,1,3,4,5,6,7,8,9'  # exclude all non-ground classes
)
wbe.write_raster(dtm, 'dtm_1m.tif')
```

TIN gridding triangulates the ground returns and interpolates elevations to
the output raster grid, preserving break-lines and reducing artefacts relative
to simple nearest-neighbour methods.

### DSM — Digital Surface Model

The DSM captures the highest return in each cell, representing the top of
the vegetation and building canopy:

```python
dsm = wbe.lidar_pointdensity(
    lidar,
    parameter='elevation',
    returns_included='first',
    resolution=1.0
)
# Or use the high-point gridding:
dsm = wbe.lidar_tin_gridding(
    lidar,
    parameter='elevation',
    returns_included='first',
    resolution=1.0
)
wbe.write_raster(dsm, 'dsm_1m.tif')
```

### Canopy Height Model (CHM)

CHM = DSM − DTM, expressing the height of above-ground objects:

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()

dtm = wbe.read_raster('dtm_1m.tif')
dsm = wbe.read_raster('dsm_1m.tif')

chm = dsm - dtm
chm = chm.max(0.0)    # Prevent negative CHM values from DEM mismatch

wbe.write_raster(chm, 'chm_1m.tif')
```

### Intensity Surface

Intensity records the strength of the laser return and is related to surface
reflectance. Gridding intensity produces a pseudo-imagery product useful for
feature detection:

```python
intensity = wbe.lidar_tin_gridding(
    lidar,
    parameter='intensity',
    returns_included='all',
    resolution=1.0
)
wbe.write_raster(intensity, 'intensity_1m.tif')
```

### Point Density

Understanding point density helps identify data quality issues (sparse areas,
flight overlap gaps) before gridding:

```python
density = wbe.lidar_pointdensity(lidar, resolution=1.0)
wbe.write_raster(density, 'point_density.tif')
```

---

## Multiple-Tile Workflows

Large surveys are tiled and must be processed together. Whitebox supports
batch processing and tile joining.

### Reading Multiple Tiles

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()

tiles = wbe.read_lidars([
    'tiles/tile_001.laz',
    'tiles/tile_002.laz',
    'tiles/tile_003.laz',
])
```

### Joining Tiles

```python
merged = wbe.lidar_join(tiles)
wbe.write_lidar(merged, 'merged.laz')
```

### Tiling Output

After processing a large cloud, re-tile for downstream storage:

```python
wbe.lidar_tile(
    lidar,
    width=500.0,   # tile width in map units
    height=500.0,
    origin_x=0.0,
    origin_y=0.0,
    output_directory='retiled/'
)
```

---

## Normalisation (Height Above Ground)

Normalised point clouds express each point's Z as height above the ground
surface rather than absolute elevation — essential for vegetation metrics.

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()

lidar = wbe.read_lidar('tile_classified.laz')
dtm   = wbe.read_raster('dtm_1m.tif')

normalised = wbe.normalise_lidar(lidar, dtm)
wbe.write_lidar(normalised, 'tile_normalised.laz')
```

After normalisation: ground returns are at Z ≈ 0, understorey vegetation at
1–5 m, mid-storey at 5–20 m, and canopy top at 20+ m (depending on forest type).

---

## Individual Tree Segmentation

Individual tree segmentation identifies and delineates individual tree crowns
from the CHM or normalised point cloud. This is used in forestry inventory,
carbon stock estimation, and urban tree management.

### CHM-Based Segmentation

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
chm = wbe.read_raster('chm_smooth.tif')

tree_polygons = wbe.individual_tree_detection(
    chm,
    min_search_radius=1.0,   # minimum crown radius in metres
    min_height=2.0,           # ignore vegetation below this height
    max_search_radius=10.0
)
wbe.write_vector(tree_polygons, 'tree_crowns.gpkg')
```

A smooth CHM substantially improves segmentation quality — apply
`feature_preserving_smoothing` or a moderate Gaussian to the raw CHM first:

```python
chm_raw    = wbe.read_raster('chm_1m.tif')
chm_smooth = wbe.gaussian_filter(chm_raw, sigma=0.75)
wbe.write_raster(chm_smooth, 'chm_smooth.tif')
```

### Point-Cloud-Based Segmentation

For higher-detail work, segment directly in 3D:

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
lidar_norm = wbe.read_lidar('tile_normalised.laz')

tree_clouds = wbe.lidar_segmentation(
    lidar_norm,
    radius=1.5,
    min_z_range=2.0
)
wbe.write_lidar(tree_clouds, 'trees_segmented.laz')
```

---

## Structural Metrics

### Canopy Cover and Closure

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
lidar = wbe.read_lidar('tile_normalised.laz')

# Canopy cover: fraction of cells with returns above height threshold
cover = wbe.lidar_canopy_cover(lidar, threshold=2.0, resolution=20.0)
wbe.write_raster(cover, 'canopy_cover_20m.tif')
```

### Height Percentiles and Mean Height

```python
p75 = wbe.lidar_elevation_slice(lidar, min_h=0.0, max_h=None, cls=1)
wbe.write_raster(p75, 'canopy_p75.tif')
```

### Building Detection

LiDAR height and planarity cues allow detection of building footprints:

```python
wbe = wbw.WbEnvironment()
lidar = wbe.read_lidar('urban_tile.laz')
dtm   = wbe.read_raster('dtm_urban.tif')

buildings = wbe.lidar_building_detection(
    lidar,
    dtm,
    min_height=3.0,
    min_area=25.0
)
wbe.write_vector(buildings, 'detected_buildings.gpkg')
```

---

## Chunked Lidar Streaming

For very large LiDAR datasets that exceed available RAM, WbW-Py supports
chunked streaming via the NumPy bridge. Each chunk is a fixed-size window
over the point array:

```python
import whitebox_workflows as wbw
import numpy as np

wbe = wbw.WbEnvironment()
lidar = wbe.read_lidar('large_survey.copc.laz')

first_return_z = []

for chunk in lidar.to_numpy_chunks(['x', 'y', 'z', 'return_number'], chunk_size=500_000):
    mask = chunk['return_number'] == 1
    first_return_z.append(chunk['z'][mask])

all_first_z = np.concatenate(first_return_z)
print(f"First-return Z mean: {all_first_z.mean():.2f}")
```

---

## Full Ground-Filter-to-DTM Pipeline

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
wbe.verbose = True

# --- 1. Read raw tile ---
lidar = wbe.read_lidar('raw_flight.laz')
print(lidar.metadata())

# --- 2. Remove outlier points ---
lidar_clean = wbe.lidar_elevation_slice(lidar, min_h=-2.0, max_h=3000.0)

# --- 3. Ground point filtering ---
lidar_classified = wbe.improved_ground_point_filter(lidar_clean, radius=0.5)
wbe.write_lidar(lidar_classified, 'classified.laz')

# --- 4. DTM ---
dtm = wbe.lidar_tin_gridding(
    lidar_classified,
    parameter='elevation',
    returns_included='last',
    resolution=0.5,
    exclude_cls='0,1,3,4,5,6,7,8,9'
)
dtm = wbe.fill_missing_data(dtm, filter_size=11)
wbe.write_raster(dtm, 'dtm_0.5m.tif')

# --- 5. DSM ---
dsm = wbe.lidar_tin_gridding(
    lidar_classified,
    parameter='elevation',
    returns_included='first',
    resolution=0.5
)
wbe.write_raster(dsm, 'dsm_0.5m.tif')

# --- 6. CHM ---
chm = (dsm - dtm).max(0.0)
wbe.write_raster(chm, 'chm_0.5m.tif')

# --- 7. Normalise ---
lidar_norm = wbe.normalise_lidar(lidar_classified, dtm)
wbe.write_lidar(lidar_norm, 'normalised.laz')

# --- 8. Tree segmentation ---
chm_smooth = wbe.gaussian_filter(chm, sigma=0.75)
trees = wbe.individual_tree_detection(chm_smooth, min_search_radius=1.0, min_height=2.0)
wbe.write_vector(trees, 'tree_crowns.gpkg')

print(f"Detected {trees.num_features()} tree crowns.")
```

---

## WbW-Pro Spotlight: LiDAR Change and Disturbance Analysis

- **Problem:** Compare repeat LiDAR epochs to detect disturbance in a
    repeatable way.
- **Tool:** `lidar_change_and_disturbance_analysis`
- **Typical inputs:** Baseline tile set, monitoring tile set, output
    resolution, minimum change threshold.
- **Typical outputs:** Change rasters plus summary metrics for affected area,
    hotspot intensity, and QA review.

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()

result = wbe.run_tool(
    'lidar_change_and_disturbance_analysis',
    {
        'baseline_tiles': '/data/lidar_epoch_2022/',
        'monitor_tiles':  '/data/lidar_epoch_2025/',
        'resolution': 1.0,
        'min_change_m': 0.5,
        'output_prefix': 'disturbance_2025_vs_2022'
    }
)
print(result)
```

> **Note:** This workflow requires a `WbEnvironment` initialized with a valid
> Pro licence.

---

## Summary

LiDAR processing in WbW-Py covers the full pipeline:

1. **Inspect** raw point cloud headers and per-file statistics.
2. **Filter** outliers and classify ground returns.
3. **Grid** ground, first returns, and intensity to produce DTM, DSM, and CHM.
4. **Normalise** for vegetation structure analysis.
5. **Segment** individual trees from the CHM or point cloud.
6. **Compute** structural metrics at plot or landscape scale.
7. **Stream** chunked data for very large surveys.

The high-performance `wblidar` backend supports LAS 1.0–1.5, LAZ, COPC, E57,
and PLY natively, with no external dependencies or codec wrappers.

---

## Tips

- **Choose your format wisely**: LAS is universal and compact; LAZ adds compression and is ideal for archival or transmission. COPC is cloud-optimized and best for remote HTTP range-request access. Use LAZ or LAS for terrestrial/airborne surveys; COPC for cloud-native workflows.
- **Always validate classifications**: Use `lidar_histogram()` and `lidar_info()` to inspect point distributions by return and classification. Misclassified ground points silently corrupt DTMs and downstream hydrology.
- **DTM vs. DSM vs. CHM**: Generate all three at the same resolution so derivatives align. A common pitfall is mixing DTM and DSM resolution.
- **Ground filtering is critical**: Outliers and noise (classification 7) should be excluded before gridding. Use `lidar_filter_for_ground()` to remove spikes and erratic points.
- **Normalization enables vegetation analysis**: Always normalize point clouds (subtract DTM) before individual tree detection or canopy structural metrics.
- **Monitor memory for large surveys**: Point clouds are memory-intensive. For datasets > 1 GB, use streaming APIs (`lidar_read_chunked()`) rather than loading the entire tile at once.
- **Coordinate reference systems matter**: LAS headers carry CRS as WKT. Verify WKT matches your project CRS before gridding or merging tiles.
- **Density and grid resolution**: If point density is < 0.5 pts/m², consider upsampling or smoothing the output grid to avoid isolated pits or peaks.
