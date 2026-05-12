# Online Data Downloads

This chapter covers downloading vector data directly from online providers into WbW-R workflows. The first provider is OpenStreetMap (OSM) through the Overpass API using tool ID `download_osm_vector`.

## Scope and Current Provider

Current online-data tool:

- `download_osm_vector`

In R, use `wbw_run_tool(...)` with explicit argument lists.

## Quick Start

```r
library(whiteboxworkflows)

session <- wbw_session()

result <- wbw_run_tool(
  "download_osm_vector",
  args = list(
    west = -80.54,
    south = 43.41,
    east = -80.47,
    north = 43.47,
    filter_preset = "roads",
    include_points = FALSE,
    include_lines = TRUE,
    include_polygons = FALSE,
    output = "kitchener_roads.geojson"
  ),
  session = session
)

print(result$outputs$path)
```

## Presets and Filters

Preset classes:

- `all`
- `roads`
- `buildings`
- `water`
- `landuse`
- `trails`
- `parks`
- `rail`
- `amenities`
- `boundaries`
- `transit`
- `poi`

Optional custom filters:

- `filter_key = "amenity"`
- `filter_key_value = "amenity=school"`

When custom filters are provided, they are used instead of preset filtering.

## Geometry Controls

Use geometry toggles to reduce payload and output complexity:

- `include_points`
- `include_lines`
- `include_polygons`

Examples:

- Streets only: points off, lines on, polygons off
- Building footprints: points off, lines off, polygons on

Phase 2 options:

- `split_output_by_geometry = TRUE` writes separate files with `_points`, `_lines`, and `_polygons` suffixes.

## Caching and Provenance

Use optional caching when running repeated AOI/filter requests:

- `cache_dir = ".wbw_cache/osm"`
- `cache_ttl_hours = 24` (set `0` to disable TTL checks)

Use `provenance_output` to write a JSON sidecar with endpoint, bbox, filters, feature counts, and cache usage metadata.

```r
wbw_run_tool(
  "download_osm_vector",
  args = list(
    west = -80.54,
    south = 43.41,
    east = -80.47,
    north = 43.47,
    filter_preset = "trails",
    include_points = FALSE,
    include_lines = TRUE,
    include_polygons = FALSE,
    split_output_by_geometry = TRUE,
    cache_dir = ".wbw_cache/osm",
    cache_ttl_hours = 24,
    provenance_output = "kitchener_trails_provenance.json",
    output = "kitchener_trails.geojson"
  ),
  session = session
)
```

## Projection and Output

Rules:

- Query extent is in EPSG:4326 (longitude/latitude).
- Set `input_extent_epsg` to provide west/south/east/north in another CRS
  (the bbox is transformed to EPSG:4326 before querying Overpass).
- Use `output_epsg` to reproject output.
- Output format is inferred from output extension.

Endpoint selection:

- `overpass_profile` supports: `main`, `kumi`, `fr`, `custom`.
- `overpass_url` overrides the selected profile URL when provided.

Large-AOI chunking:

- `chunk_large_aoi = TRUE` (default) automatically tiles large query extents.
- `chunk_max_area_deg2 = 4.0` controls maximum area per chunk.
- `max_chunk_count = 64` caps the number of generated chunk requests.
- `chunk_parallel_requests = 1` (default) controls bounded parallel chunk fetch; set >1 to fetch chunks concurrently.

```r
wbw_run_tool(
  "download_osm_vector",
  args = list(
    west = -80.54,
    south = 43.41,
    east = -80.47,
    north = 43.47,
    filter_preset = "buildings",
    include_points = FALSE,
    include_lines = FALSE,
    include_polygons = TRUE,
    output_epsg = 32617,
    output = "kitchener_buildings_utm17n.gpkg"
  ),
  session = session
)
```

## Operational Guidance

Overpass public endpoints have rate limits and shared infrastructure constraints.

Recommended practice:

- Use smaller AOIs
- Prefer thematic filters
- Cap responses with `max_elements`
- Increase `timeout_seconds` only when needed

## Attribution and Licensing

OSM data are licensed under ODbL. Ensure appropriate OpenStreetMap attribution and verify obligations for redistributed outputs.

## Companion Example

See:

- `crates/wbw_r/examples/osm_download_vector.R`
