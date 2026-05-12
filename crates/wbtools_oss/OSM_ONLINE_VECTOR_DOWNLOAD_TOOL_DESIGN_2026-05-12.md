# OSM Online Vector Download Tool Design

Date: 2026-05-12
Status: Phase 2 feature-complete (relation/multipolygon + cache + split-output + provenance sidecar implemented)
Target crate: wbtools_oss

## Objective

Add a new wbtools_oss tool that downloads OpenStreetMap data from the Internet
for a user-specified spatial extent and filter, then writes output in any
wbvector-supported vector format with optional reprojection.

Proposed tool concept:
- Internal Rust type: DownloadOsmVectorTool
- User-facing nested namespace target:
  wbe.vector.online_data.download_osm_vector(...)

## Key Decision on Dependencies

Short answer: yes, this can be done without adding major dependencies to core
backend crates (wbvector, wbtopology, wbprojection, wbraster).

Recommended dependency boundary:
- Keep wbvector/wbtopology dependency set unchanged for this feature.
- Add Internet client dependency only in wbtools_oss.

Recommended HTTP client choice:
- Preferred: ureq (blocking, simple API, lighter integration burden).
- Alternative: reqwest blocking + rustls if consistency with existing patterns
  is desired.

Why this boundary works:
- wbvector already handles vector read/write and format dispatch.
- wbvector already has reprojection APIs.
- wbtopology is only needed if we add optional clipping/geometry repair.
- Network transport concerns belong in the tool layer, not data core crates.

## Feasibility Against Existing Capabilities

Existing capabilities already available:
- Vector write targets and format detect/write in wbvector.
- Vector reprojection in wbvector::reproject.
- Optional topology/clip operations in wbtopology if needed.
- OSM PBF parsing path exists in wbvector behind osmpbf feature, but this tool
  should use Overpass JSON for online retrieval in MVP.

Missing pieces to implement in wbtools_oss:
- Overpass query generation.
- HTTP fetch/retry/timeout and response-size guards.
- Overpass JSON parse to Layer.
- Tool parameters and validation.
- Frontend wiring/docs.

Conclusion:
- Feasible with moderate implementation effort.
- Low risk to core backend crates if dependency boundary is respected.

## MVP Scope

Input parameters:
- extent (bbox: min_x, min_y, max_x, max_y in EPSG:4326)
- filter_mode (preset or custom)
- include_keys (optional list)
- include_key_values (optional map/list of key=value)
- geometry_types (points, lines, polygons; default all)
- endpoint_url (default Overpass endpoint)
- timeout_seconds
- max_elements
- output (path)
- output_epsg (optional)
- clip_to_extent (optional, default true)

Output behavior:
- Build Layer in EPSG:4326 from OSM elements.
- Optionally clip to bbox for strict boundary behavior.
- Optionally reproject to output_epsg.
- Write with wbvector format inferred from output extension.

MVP filter presets:
- all (no preset constraint)
- roads (highway)
- buildings (building)
- water (waterway or natural=water)
- landuse (landuse)
- trails (highway path/footway/cycleway/bridleway)
- parks (leisure=park, boundary=national_park, recreation/nature reserve tags)
- rail (railway)
- amenities (amenity)
- boundaries (boundary)
- transit (public_transport, railway station, bus stop)
- poi (amenity, tourism, shop, leisure)

Custom filtering:
- key only: e.g., amenity
- key=value: e.g., amenity=school
- multiple filters combined with OR in query and post-filtered in tool.

## Overpass Query Strategy

MVP query style:
- Prefer way and node retrieval for bounded extent.
- Include relation retrieval for polygon-capable requests so multipolygon and boundary
  relations can be assembled from member ways.

Template shape (conceptual):
- [out:json][timeout:T];
- (
-   node[FILTER](S,W,N,E);
-   way[FILTER](S,W,N,E);
- );
- out body;
- >;
- out skel qt;

Notes:
- Parse returned element list into node map first, then construct way geometry.
- Polygon inference from closed ways with area-like tags can mirror existing
  wbvector osmpbf area heuristics.

## Data Model Mapping

Recommended output schema fields:
- osm_id (integer/text depending on storage constraints)
- osm_type (node/way)
- name
- class_key (primary classification key)
- class_value (primary classification value)
- osm_tags (JSON text)
- source_timestamp (optional)
- source_endpoint (optional)

Geometry handling:
- node -> Point
- way closed + area-like tags -> Polygon
- way otherwise -> LineString

Mixed geometry policy:
- Option A (default MVP): emit mixed geometry in one Layer.
- Option B (optional): split by geometry type into multiple outputs later.

## CRS and Projection

Proposed rules:
- Internet query extent is always EPSG:4326.
- Tool accepts output_epsg for final output reprojection.
- If output_epsg is omitted, output remains EPSG:4326.

Extent in projected CRS (future enhancement):
- Accept input_extent_epsg and internally transform bbox to EPSG:4326 before
  query.
- Not required for MVP if frontends already pass lon/lat bbox.

## Error Handling and Safeguards

Must-have guards:
- Reject invalid bbox or oversized bbox (configurable max area).
- Timeout and retry with backoff for 429/5xx.
- Response element count cap with clear error.
- Endpoint-unreachable and malformed response errors with diagnostics.

Operational constraints:
- Add user-facing note about Overpass public endpoint rate limits.
- Encourage local cache and smaller AOI queries.

## Caching Strategy (MVP+)

MVP:
- No persistent cache required.

Recommended next step:
- Optional file cache keyed by endpoint + query hash + date.
- Useful for reproducibility and reducing endpoint load.

## Licensing and Attribution

The tool should include explicit documentation for OSM/ODbL obligations:
- Data source attribution requirements.
- Share-alike considerations for certain use cases.
- Endpoint terms of use and fair-use/rate-limit behavior.

## Backend Dependency Policy Compliance

Policy-aligned implementation:
- Add HTTP dependency only to wbtools_oss.
- Keep wbvector/wbtopology/wbprojection APIs dependency-neutral.
- Use existing wbvector write/reproject APIs rather than duplicating logic.

## Frontend Integration Plan

wbw_python:
- Add wrapped tool function in generated/manual docs.
- Example: AOI roads download to GeoPackage and TopoJSON outputs.

wbw_r:
- Add mirrored wrapper and manual example.

wbw_qgis:
- Add toolbox tool metadata and parameter widgets.
- Add recipe entry for OSM AOI ingest + reproject + save.

## Proposed Tool Arguments (MVP)

- output: String
- west: f64
- south: f64
- east: f64
- north: f64
- filter_preset: String (all|roads|buildings|water|landuse|trails|parks|rail|amenities|boundaries|transit|poi)
- include_tags: String (semicolon-delimited keys; optional)
- include_key_values: String (semicolon-delimited k=v; optional)
- include_points: bool
- include_lines: bool
- include_polygons: bool
- overpass_url: String
- timeout_seconds: u32
- max_elements: u32
- output_epsg: i32 (optional; <=0 means none)
- clip_to_extent: bool
- split_output_by_geometry: bool
- cache_dir: String (optional)
- cache_ttl_hours: u64 (default 24; 0 disables TTL check)
- provenance_output: String (optional JSON sidecar path)
- input_extent_epsg: u32 (default 4326)
- overpass_profile: String (main|kumi|fr|custom)
- chunk_large_aoi: bool (default true)
- chunk_max_area_deg2: f64 (default 4.0)
- max_chunk_count: u32 (default 64)
- chunk_parallel_requests: u32 (default 1; >1 enables bounded parallel chunk fetch)

## Suggested Rollout Phases

Phase 1 (MVP):
- Overpass fetch for node/way.
- Preset + custom key/value filters.
- Geometry build, optional clip, optional reprojection.
- Single output file write.

Phase 2:
- Relation/multipolygon handling. (implemented)
- Optional cache. (implemented)
- Split-output mode by geometry type. (implemented)
- Better metadata/provenance sidecar. (implemented)

Phase 3:
- Projected extent input convenience. (implemented)
- Endpoint profiles and quota-aware defaults. (implemented: profile selection + URL override)
- Chunked fetch strategy for large AOIs. (implemented: auto-tiling + merge/dedup + optional bounded parallel chunk fetch)

## Testing Plan

Unit tests:
- Query builder coverage for presets/custom filters.
- Tag filter parser/validator.
- Geometry assembly from node/way samples.

Integration tests:
- Mock HTTP server responses to avoid live endpoint dependency.
- End-to-end output format checks (.gpkg, .geojson, .topojson).
- Reprojection output CRS verification.

Operational tests:
- Timeout behavior.
- 429/5xx retry policy.
- max_elements cut-off.

## Recommendation

Proceed with this tool.

Rationale:
- High practical user value.
- Feasible with current architecture.
- Can be implemented with strict dependency containment in wbtools_oss.
- Strong synergy with existing vector output and reprojection stack.
