# Reprojection and CRS

A Coordinate Reference System (CRS) defines how coordinates in a dataset
map to real-world locations. CRS mismatches are one of the most common sources
of silent errors in GIS workflows: two layers may display correctly on screen
(because QGIS reprojects them on-the-fly for display) while producing wrong
results when passed to an analysis tool that expects matching CRS inputs.

This chapter explains how to identify, verify, and correct CRS issues in
WbW-QGIS workflows.

---

## Key Concepts

- **Geographic CRS (GCS)**: Coordinates in angular units (degrees of latitude
  and longitude). Common examples: WGS84 (EPSG:4326), NAD83 (EPSG:4269).
  **Not suitable as a working CRS for distance/area calculations.**
- **Projected CRS (PCS)**: Coordinates in linear units (metres or feet) on a
  flat map projection. Examples: UTM zones, Lambert Conformal Conic, Albers
  Equal Area.
- **EPSG code**: A numeric registry identifier for a CRS. EPSG:4326 = WGS84;
  EPSG:32617 = UTM Zone 17N (WGS84); EPSG:3978 = Canada Atlas Lambert.
- **On-the-fly reprojection**: QGIS displays all layers in the project CRS
  regardless of their native CRS. This is for **display only** — it does not
  change the file on disk.
- **Reproject (warp)**: Permanently transform raster or vector data to a new
  CRS, writing a new file. Required before passing data to analysis tools.
- **Z factor**: A unit-conversion factor applied when DEM horizontal units
  (metres) differ from vertical units (feet), or vice versa.

---

## Choosing a Working CRS

| Scenario | Recommended CRS type |
|----------|---------------------|
| Global or continental analysis | Geographic (WGS84 / EPSG:4326) for data exchange; Equal-Area projection for area measurements |
| Regional / national analysis | National projected CRS (e.g. Canada Atlas Lambert / EPSG:3978) |
| Local analysis (< 500 km extent) | UTM zone covering the study area |
| Terrain analysis, hydrology, LiDAR | Projected CRS in metres (UTM recommended) |
| Slope / distance calculations | **Always** use a projected CRS |

**Finding your UTM zone**: The UTM zone number equals ⌊(longitude + 180) / 6⌋ + 1.
For Ottawa, Canada (longitude ≈ –75.7°): zone 18, northern hemisphere →
EPSG:32618.

---

## Checking the CRS of a Layer

1. Right-click a layer in the Layers panel → **Properties**.
2. Select the **Information** tab.
3. Read the **CRS** field. Confirm:
   - Authority and code (e.g. `EPSG:32618`)
   - Unit (metres vs degrees)
   - Datum (WGS84, NAD83, etc.)

Or in the Python Console:

```python
from qgis.core import QgsProject

layer = QgsProject.instance().mapLayersByName('dem')[0]
crs = layer.crs()
print(crs.authid())        # e.g. "EPSG:32618"
print(crs.mapUnits())      # 0 = metres, 6 = degrees
print(crs.isGeographic())  # True if GCS
```

---

## Setting the Project CRS

The **project CRS** controls the display and the default output CRS for tools
that do not inherit CRS from their inputs.

**Project → Properties → CRS tab** → search by EPSG code or name → click OK.

Or use **View → Panels → CRS Status** at the bottom-right of the QGIS window
to set the project CRS from any loaded layer.

---

## Reprojecting a Raster

Use **Reproject Raster** to permanently transform a raster to a new CRS.
This is required before any terrain analysis on a DEM stored in geographic
(degree) coordinates.

**Processing Toolbox → Whitebox Workflows → Raster → `Reproject Raster`**

| Parameter | Recommended value |
|-----------|------------------|
| Input layer | `dem_wgs84.tif` |
| Target EPSG code | `32618` |
| Resampling method | `bilinear` (elevation surfaces) |
| Output | `dem_utm18n.tif` |

The **Resampling method** parameter accepts any of the methods supported by
WbW's raster engine:

| Method | Best for |
|--------|---------|
| `nearest` | Categorical / integer rasters (classification maps, stream grids) |
| `bilinear` | Continuous surfaces (DEMs, slope, TWI, reflectance) |
| `cubic` | High-quality continuous-surface resampling |
| `lanczos` | High-quality sinc-window resampling |
| `average` | 3×3 mean statistic |
| `min` / `max` | 3×3 extremum statistics |
| `mode` | 3×3 majority-class (smoothed categorical) |
| `median` | 3×3 median statistic |
| `stddev` | 3×3 standard deviation |

```python
import processing

processing.run('whitebox_workflows:reproject_raster', {
    'input': '/data/dem_wgs84.tif',
    'epsg': 32618,
    'resample': 'bilinear',
    'output': '/data/dem_utm18n.tif',
})
```

After running, load the output and confirm in Layer Properties → Information:
CRS shows `EPSG:32618` and the extent is in metres.

---

## Reprojecting a Vector Layer

Use **Reproject Vector** to transform a vector dataset to a new CRS.

**Processing Toolbox → Whitebox Workflows → Vector → `Reproject Vector`**

| Parameter | Recommended value |
|-----------|------------------|
| Input layer | `roads_wgs84.shp` |
| Target EPSG code | `32618` |
| Output | `roads_utm18n.shp` |

```python
import processing

processing.run('whitebox_workflows:reproject_vector', {
    'input': '/data/roads_wgs84.shp',
    'epsg': 32618,
    'output': '/data/roads_utm18n.shp',
})
```

---

## Reprojecting a LiDAR Dataset

Use **Reproject LiDAR** to transform a point cloud to a new CRS.

**Processing Toolbox → Whitebox Workflows → LiDAR → `Reproject LiDAR`**

| Parameter | Recommended value |
|-----------|------------------|
| Input LiDAR file | `cloud_wgs84.laz` |
| Target EPSG code | `32618` |
| Output | `cloud_utm18n.laz` |

```python
import processing

processing.run('whitebox_workflows:reproject_lidar', {
    'input': '/data/cloud_wgs84.laz',
    'epsg': 32618,
    'output': '/data/cloud_utm18n.laz',
})
```

---

## Assigning a Missing CRS

If a file has correct coordinates but missing or wrong CRS metadata (shown
as "Unknown CRS" in Layer Properties), use one of the three **Assign
Projection** tools to write the correct EPSG code into the file without
moving any coordinates.

> **Assign vs. Reproject:** Assigning a CRS only updates the metadata label.
> Use it when coordinates are already in the target system but the file has
> no CRS tag. If the coordinate values themselves need to change, use the
> Reproject tools above instead.

### Assign Projection to a Raster

**Processing Toolbox → Whitebox Workflows → Raster → `Assign Projection Raster`**

| Parameter | Value |
|-----------|-------|
| Input layer | `dem_no_crs.tif` |
| EPSG code to assign | `32618` |

```python
import processing

processing.run('whitebox_workflows:assign_projection_raster', {
    'input': '/data/dem_no_crs.tif',
    'epsg': 32618,
})
```

### Assign Projection to a Vector

**Processing Toolbox → Whitebox Workflows → Vector → `Assign Projection Vector`**

| Parameter | Value |
|-----------|-------|
| Input layer | `roads_no_crs.shp` |
| EPSG code to assign | `32618` |

```python
import processing

processing.run('whitebox_workflows:assign_projection_vector', {
    'input': '/data/roads_no_crs.shp',
    'epsg': 32618,
})
```

### Assign Projection to a LiDAR File

**Processing Toolbox -> Whitebox Workflows -> LiDAR -> `Assign Projection LiDAR`**

| Parameter | Value |
|-----------|-------|
| Input LiDAR file | `cloud_no_crs.laz` |
| EPSG code to assign | `32618` |

```python
import processing

processing.run('whitebox_workflows:assign_projection_lidar', {
  'input': '/data/cloud_no_crs.laz',
  'epsg': 32618,
})
```

---

## Georeferencing from Control Points

Use **Georeference Raster From Control Points** when the raster has no valid
georeferencing and you have control points that relate pixel coordinates to map
coordinates.

**Processing Toolbox -> Whitebox Workflows -> Raster -> `Georeference Raster From Control Points`**

| Parameter | Recommended value |
|-----------|-------------------|
| Input raster | `historical_scan.tif` |
| Control points CSV | `historical_scan_gcps.csv` |
| Destination EPSG code | `32618` |
| Resampling method | `bilinear` (continuous) or `nearest` (categorical) |
| Georeferenced raster output | `historical_scan_georef.tif` |
| Diagnostics report output | `historical_scan_georef_report.json` (optional) |

Control-points CSV fields must include source image coordinates and target map
coordinates:

- `source_col`
- `source_row`
- `target_x`
- `target_y`

```python
import processing

processing.run('whitebox_workflows:georeference_raster_from_control_points', {
  'input': '/data/historical_scan.tif',
  'control_points': '/data/historical_scan_gcps.csv',
  'epsg': 32618,
  'resample': 'bilinear',
  'output': '/data/historical_scan_georef.tif',
  'report': '/data/historical_scan_georef_report.json',
})
```

### Assign Projection to a LiDAR Dataset

**Processing Toolbox → Whitebox Workflows → LiDAR → `Assign Projection LiDAR`**

| Parameter | Value |
|-----------|-------|
| Input LiDAR file | `cloud_no_crs.laz` |
| EPSG code to assign | `32618` |

```python
import processing

processing.run('whitebox_workflows:assign_projection_lidar', {
    'input': '/data/cloud_no_crs.laz',
    'epsg': 32618,
})
```

---

## The Z Factor for Terrain Tools

When a DEM's horizontal units differ from its vertical units, slope and
curvature calculations are incorrect unless a Z factor is applied.

| Horizontal unit | Vertical unit | Z factor |
|----------------|--------------|---------|
| Metres | Metres | `1.0` (no conversion needed) |
| Metres | Feet | `0.3048` |
| Feet | Feet | `1.0` |
| Degrees (geographic) | Metres | Do **not** use — reproject first |

All WbW terrain tools that accept a Z factor parameter apply the conversion
as: `slope = atan(rise × z_factor / run)`.

> **Best practice**: Reproject the DEM to a projected CRS in metres before
> running any terrain analysis. Set Z factor to `1.0` after reprojection if
> vertical units are also metres.

---

## Batch Reprojection via Python Console

Reproject all GeoPackages in a folder to EPSG:32618 using the Whitebox
vector reprojection tool:

```python
import processing
from pathlib import Path

src_dir = Path('/data/raw_vectors')
out_dir = Path('/data/projected')
out_dir.mkdir(exist_ok=True)

for gpkg in src_dir.glob('*.gpkg'):
    out = out_dir / gpkg.name
    processing.run('whitebox_workflows:reproject_vector', {
        'input': str(gpkg),
        'epsg': 32618,
        'output': str(out),
    })
    print(f"Reprojected: {gpkg.name}")

print("Batch reprojection complete.")
```

Reproject a folder of LiDAR files in the same pattern:

```python
import processing
from pathlib import Path

src_dir = Path('/data/raw_lidar')
out_dir = Path('/data/projected_lidar')
out_dir.mkdir(exist_ok=True)

for las in src_dir.glob('*.laz'):
    out = out_dir / las.name
    processing.run('whitebox_workflows:reproject_lidar', {
        'input': str(las),
        'epsg': 32618,
        'output': str(out),
    })
    print(f"Reprojected: {las.name}")

print("Batch LiDAR reprojection complete.")
```

---

## Common CRS Problems

| Problem | Likely cause | Fix |
|---------|-------------|-----|
| Layers display in wrong location | Layer has incorrect assigned CRS | Assign correct CRS (do not reproject) |
| Slope values in hundreds of degrees | DEM in geographic CRS (degrees) — cell size << 1° | Reproject DEM to metres before running slope |
| Area calculations wildly wrong | Layer CRS is geographic (degrees) | Reproject to equal-area projected CRS |
| Watershed does not close properly | Raster and vector inputs in different CRS | Reproject all inputs to same CRS before processing |
| WbW tool silently returns NoData everywhere | CRS mismatch causes spatial extents not to overlap | Verify all inputs share the same CRS and extent |
| "Datum transform not found" warning in QGIS | Datum shift grid file not installed | Install `proj-data` package, or accept approximate transform |

---

## Validation Checklist

- [ ] Project CRS is set to the intended working CRS before analysis.
- [ ] All raster inputs share the same CRS, extent, and cell size.
- [ ] All vector inputs share the same CRS as the raster grid.
- [ ] DEM CRS is projected (linear units — metres or feet), not geographic.
- [ ] Z factor is set to `1.0` when both horizontal and vertical units are metres.
- [ ] Reprojected outputs have been inspected (extent in metres, CRS code confirmed).
- [ ] No layers show "Unknown CRS" in the Layers panel.
