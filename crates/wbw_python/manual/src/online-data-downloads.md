# Online Data Downloads

This chapter focuses on downloading vector data directly from online providers into Whitebox workflows. The initial implementation is OpenStreetMap (OSM) via Overpass API using `download_osm_vector`.

## Scope and Current Provider

Current tool:

- `wbe.vector.online_data.download_osm_vector(...)`

The tool downloads OSM features within a longitude/latitude bounding box (EPSG:4326), optionally filters by theme, and writes output in any supported vector format based on output extension.

## Quick Start

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()

roads = wbe.vector.online_data.download_osm_vector(
    west=-80.54,
    south=43.41,
    east=-80.47,
    north=43.47,
    filter_preset="roads",
    include_points=False,
    include_lines=True,
    include_polygons=False,
)

wbe.write_vector(roads, "kitchener_roads.geojson")
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

- `filter_key="amenity"`
- `filter_key_value="amenity=school"`

If custom filters are supplied, they take precedence over preset filtering.

## Geometry Controls

Use geometry toggles to reduce result size and parsing overhead:

- `include_points`
- `include_lines`
- `include_polygons`

Typical examples:

- Road centerlines only: points off, lines on, polygons off
- Building footprints only: points off, lines off, polygons on

Phase 2 options:

- `split_output_by_geometry=True` writes separate files with `_points`, `_lines`, and `_polygons` suffixes.

## Caching and Provenance

Use optional caching when iterating on the same AOI/filter query repeatedly:

- `cache_dir=".wbw_cache/osm"`
- `cache_ttl_hours=24` (set `0` to disable TTL checks)

Use `provenance_output` to write a JSON sidecar with endpoint, bbox, filters, feature counts, and cache usage metadata.

```python
roads = wbe.vector.online_data.download_osm_vector(
    west=-80.54,
    south=43.41,
    east=-80.47,
    north=43.47,
    filter_preset="trails",
    include_points=False,
    include_lines=True,
    include_polygons=False,
    split_output_by_geometry=True,
    cache_dir=".wbw_cache/osm",
    cache_ttl_hours=24,
    provenance_output="kitchener_trails_provenance.json",
    output="kitchener_trails.geojson",
)
```

## Projection and Output

Rules:

- Query extent is interpreted as EPSG:4326 (lon/lat).
- Set `input_extent_epsg` to provide west/south/east/north in another CRS
    (the bbox is transformed to EPSG:4326 before querying Overpass).
- Output stays EPSG:4326 unless `output_epsg` is provided.
- Output format is inferred from filename extension (`.shp`, `.gpkg`, `.geojson`, `.topojson`, ...).

Endpoint selection:

- `overpass_profile` supports: `main`, `kumi`, `fr`, `custom`.
- `overpass_url` overrides the selected profile URL when provided.

Large-AOI chunking:

- `chunk_large_aoi=True` (default) automatically tiles large query extents.
- `chunk_max_area_deg2=4.0` controls maximum area per chunk.
- `max_chunk_count=64` caps the number of generated chunk requests.
- `chunk_parallel_requests=1` (default) controls bounded parallel chunk fetch; set >1 to fetch chunks concurrently.

```python
buildings = wbe.vector.online_data.download_osm_vector(
    west=-80.54,
    south=43.41,
    east=-80.47,
    north=43.47,
    filter_preset="buildings",
    include_points=False,
    include_lines=False,
    include_polygons=True,
    output_epsg=32617,
)

wbe.write_vector(buildings, "kitchener_buildings_utm17n.gpkg")
```

## Operational Guidance

Overpass public endpoints enforce rate limits. Prefer smaller AOIs and bounded requests.

Recommended practice:

- Keep AOIs compact
- Use thematic filters
- Set `max_elements` defensively
- Increase `timeout_seconds` for denser urban queries

## Attribution and Licensing

OSM data are provided under ODbL. When distributing derived datasets or maps, ensure proper OpenStreetMap attribution and verify downstream licensing obligations for your use case.

## Companion Example

See:

- `crates/wbw_python/examples/osm_download_vector.py`
