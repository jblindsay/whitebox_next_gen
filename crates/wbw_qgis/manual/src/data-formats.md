# Supported Data Formats

This chapter documents format support exposed through WbW-QGIS.

Authoritative backend support comes from Whitebox core crates:
- Raster I/O: `wbraster`
- Vector I/O: `wbvector`
- LiDAR I/O: `wblidar`

The format tables below are aligned with those backend crates' README
"Supported Formats" sections.

---

## Raster Formats

Raster support in Whitebox is provided by `wbraster`.

| Format | Extension(s) | Read | Write | Notes |
|---|---|:---:|:---:|---|
| DTED | `.dt0`, `.dt1`, `.dt2` | Yes | Yes | DTED 0/1/2 elevation; WGS-84 geographic only |
| ENVI HDR Labelled | `.hdr` + sidecar data | Yes | Yes | Multi-band (`BSQ` / `BIL` / `BIP`) |
| ER Mapper | `.ers` + data | Yes | Yes | Hierarchical header |
| ERDAS IMAGINE (HFA) | `.img` | Yes | No | Read-only MVP; RLC compression supported |
| Esri ASCII Grid | `.asc`, `.grd` | Yes | Yes | Handles `xllcorner` and `xllcenter` |
| Esri Binary Grid | workspace dir / `.adf` | Yes | Yes | Single-band float32, big-endian |
| Esri Float Grid | `.flt`, `.hdr` | Yes | Yes | Single-band float grid with header |
| JPEG + World File | `.jpg`, `.jpeg` + `.jgw`/`.wld` | Yes | Yes | Non-rotated georeferencing |
| PNG + World File | `.png` + `.pgw`/`.wld` | Yes | Yes | Non-rotated georeferencing |
| GeoTIFF / BigTIFF / COG | `.tif`, `.tiff` | Yes | Yes | Stripped/tiled GeoTIFF, BigTIFF, COG |
| GeoPackage Raster (Phase 4) | `.gpkg` | Yes | Yes | Multi-band tiled raster |
| GRASS ASCII Raster | `.asc`, `.txt` | Yes | Yes | `north/south/east/west`, `rows/cols` headers |
| Idrisi/TerrSet Raster | `.rdc`, `.rst` | Yes | Yes | `byte`, `integer`, `real`, `RGB24` |
| JPEG2000 / GeoJP2 | `.jp2` | Yes | Yes | Pure-Rust reader and writer |
| PCRaster | `.map` | Yes | Yes | Value-scale aware writer |
| SAGA GIS Binary | `.sgrd`, `.sdat` | Yes | Yes | SAGA data types supported |
| Surfer GRD | `.grd` | Yes | Yes | DSAA and DSRB |
| Zarr v2/v3 | `.zarr` | Yes | Yes | 2D and 3D (`band,y,x`) chunked arrays |
| XYZ ASCII Grid | `.xyz` | Yes | Yes | Whitespace or comma-delimited X Y Z points |

Notes:
- Whitebox avoids runtime dependence on GDAL.
- In QGIS workflows, GeoTIFF remains the safest default interchange raster.

---

## Vector Formats

Vector support in Whitebox is provided by `wbvector`.

| Format | Read | Write | Notes |
|---|:---:|:---:|---|
| FlatGeobuf (`.fgb`) | Yes | Yes | High-performance binary interchange |
| GeoJSON (`.geojson`) | Yes | Yes | Web-friendly text format |
| TopoJSON (`.topojson`) | Yes | Yes | Topology-preserving JSON format |
| GeoPackage (`.gpkg`) | Yes | Yes | SQLite container; multi-layer workflows |
| GML (`.gml`) | Yes | Yes | Standards-based XML exchange |
| GPX (`.gpx`) | Yes | Yes | GPS tracks/routes/waypoints |
| KML (`.kml`) | Yes | Yes | Google Earth-style visualization |
| MapInfo Interchange (`.mif` + `.mid`) | Yes | Yes | Legacy MapInfo interoperability |
| ESRI Shapefile (`.shp` + sidecars) | Yes | Yes | Broad legacy compatibility |
| GeoParquet (`.parquet`) | Yes | Yes | Optional `geoparquet` feature |
| KMZ (`.kmz`) | Yes | Yes | Optional `kmz` feature |
| OSM PBF (`.osm.pbf`) | Yes | No | Read-only; optional `osmpbf` feature |

Feature-gated formats in `wbvector`:
- `geoparquet` for GeoParquet support
- `kmz` for KMZ support
- `osmpbf` for OSM PBF read support

In QGIS workflows, GeoPackage and FlatGeobuf are good modern interchange
choices; Shapefile remains a compatibility fallback.

---

## LiDAR / Point Cloud Formats

LiDAR support in Whitebox is provided by `wblidar`.

| Format | Read | Write | Notes |
|---|:---:|:---:|---|
| LAS | Yes | Yes | LAS 1.1-1.5, PDRF 0-15 |
| LAZ | Yes | Yes | Standards-compliant LASzip v2/v3 Point10/Point14 codecs |
| COPC | Yes | Yes | COPC 1.0 hierarchy with Point14-family payloads |
| PLY | Yes | Yes | ASCII, binary little-endian, binary big-endian |
| E57 | Yes | Yes | ASTM E2807 with CRC-32 page validation |

Optional features in `wblidar`:
- `copc-http` for HTTP range fetching of remote COPC
- `copc-parallel` for parallel COPC writing paths
- `laz-parallel` for optional parallel LAZ decode paths
- `parallel` umbrella feature (enables both parallel paths)

In QGIS workflows, `.copc.laz` is a strong default for large point-cloud
delivery and archive.

---

## QGIS Practical Defaults

For most WbW-QGIS production workflows:
- Raster default: GeoTIFF (`.tif`)
- Vector default: GeoPackage (`.gpkg`) or FlatGeobuf (`.fgb`)
- LiDAR default: COPC LAZ (`.copc.laz`) or LAZ (`.laz`)

These defaults balance compatibility, file size, and performance.

---

## Important Distinction

Backend format support means the Whitebox runtime can read/write those formats.
Specific QGIS tool dialogs may still constrain certain outputs or defaults
depending on parameter wiring and the tool category.

When in doubt:
1. Use the tool's default output extension in QGIS.
2. Re-open output and validate metadata.
3. Use QGIS conversion tools only when you need a different interchange format.

---

## Common Format Problems

| Problem | Likely cause | Fix |
|---|---|---|
| Output opens but schema is unexpected | Format-specific field/type constraints | Use GeoPackage or FlatGeobuf for richer schema |
| Shapefile field names truncated | 10-character DBF limit | Switch output to GeoPackage |
| Large cloud is slow to browse | Non-indexed point-cloud format | Use COPC LAZ for tiled access |
| Optional format not available | Feature not enabled in build | Use a non-optional format (for example GeoPackage/GeoJSON/Shapefile) |
| CRS appears missing in output | Sidecar or metadata issue | Confirm CRS in layer properties and re-export if needed |
