# Geometry and Topology


---

## Fix Dangling Arcs

**Function name:** `fix_dangling_arcs`


### Description

 

This tool can be used to fix undershot and overshot arcs, two common topological errors, in an input vector lines file (`input`). In addition to the input lines vector, the user must also specify the output vector (`output`) and the snap distance (`snap`). All dangling arcs that are within this threshold snap distance of another line feature will be connected to the neighbouring feature. If the input lines network is a vector stream network, users are advised to apply the `repair_stream_vector_topology` tool instead. 

 

### See Also

 

`repair_stream_vector_topology`, `clean_vector` 

### Python API

```python
def fix_dangling_arcs(self, input: Vector, snap_dist: float) -> Vector:
```


---

## Lines To Polygons

**Function name:** `lines_to_polygons`


This tool converts vector polylines into polygons. Note that this tool will close polygons that are open and will ensure that the first part of an input line is interpreted as the polygon hull and subsequent parts are considered holes. The tool does not examine input lines for line crossings (self intersections), which are topological errors. 

### See Also

 

`polygons_to_lines` 

### Python API

```python
def lines_to_polygons(self, input: Vector) -> Vector:
```


---

## Multipart To Singlepart

**Function name:** `multipart_to_singlepart`


This tool can be used to convert a vector file containing multi-part features into a vector containing only single-part features. Any multi-part polygons or lines within the input vector file will be split into separate features in the output file, each possessing their own entry in the associated attribute file. For polygon-type vectors, the user may optionally choose to exclude hole-parts from being separated from their containing polygons. That is, with the `exclude_holes` parameter, hole parts in the input vector will continue to belong to their enclosing polygon in the output vector. The tool will also convert MultiPoint Shapefiles into single Point vectors. 

### See Also

 

`single_part_to_multipart` 

### Python API

```python
def multipart_to_singlepart(self, input: Vector, exclude_holes: bool = False) -> Vector:
```


---

## Polygons To Lines

**Function name:** `polygons_to_lines`


This tool converts vector polygons into polylines, simply by modifying the Shapefile geometry type. 

### See Also

 

`lines_to_polygons` 

### Python API

```python
def polygons_to_lines(self, input: Vector) -> Vector:
```


---

## Remove Polygon Holes

**Function name:** `remove_polygon_holes`


This tool can be used to remove holes from the features within a vector polygon file. The user must specify the name of the input vector file, which must be of a polygon VectorGeometryType, and the name of the output file. 

### Python API

```python
def remove_polygon_holes(self, input: Vector) -> Vector:
```


---

## Singlepart To Multipart

**Function name:** `singlepart_to_multipart`


This tool can be used to convert a vector file containing single-part features into a vector containing multi-part features. The user has the option to either group features based on an ID Field (`field` flag), which is a categorical field within the vector's attribute table. The ID Field should either be of String (text) or Integer type. Fields containing decimal values are not good candidates for the ID Field. **If no `field` flag is specified, all features will be grouped together into one large multi-part vector**. 

This tool works for vectors containing either point, line, or polygon features. Since vectors of a POINT VectorGeometryType cannot represent multi-part features, the VectorGeometryType of the output file will be modified to a MULTIPOINT VectorGeometryType if the input file is of a POINT VectorGeometryType. If the input vector is of a POLYGON VectorGeometryType, the user can optionally set the algorithm to search for polygons that should be represented as hole parts. In the case of grouping based on an ID Field, hole parts are polygon features contained within larger polygons of the same ID Field value. Please note that searching for polygon holes may significantly increase processing time for larger polygon coverages. 

### See Also

 

`MultiPartToSinglePart` 

### Python API

```python
def singlepart_to_multipart(self, input: Vector, field_name: str) -> Vector:
```


---

## Topology Rule Autofix

**Function name:** `topology_rule_autofix`


Experimental

Automatically applies safe, auditable fixes to topology violations detected by topology_rule_validate.

data-tools vector topology fix quality

### Parameters

NameDescriptionRequiredDefault
`input`Input vector path.Required`input.gpkg`
`rule_set`Rule configuration as JSON array/object, CSV string, or file path. Applies fixes for supported rules only: line_endpoints_must_snap_within_tolerance, point_must_be_covered_by_line, polygon_must_not_have_gaps, line_must_not_have_dangles.Optional`['line_endpoints_must_snap_within_tolerance', 'point_must_be_covered_by_line', 'polygon_must_not_have_gaps', 'line_must_not_have_dangles']`
`snap_tolerance`Tolerance for snapping operations in coordinate units. Defaults to 0.01.Optional`0.01`
`dry_run`If true (default), emits change report without modifying input. If false, applies changes.Optional`True`
`output`Output vector path for fixed features. If omitted, derived beside input.Optional`topology_fixed.gpkg`
`change_report`Optional JSON change audit-trail report path.Optional—

### Examples

*Preview endpoint snapping fixes in dry-run mode with change audit trail.*
`wbe.topology_rule_autofix(change_report='network_changes.json', dry_run=False, input='network_violations.gpkg', output='network_fixed.gpkg', rule_set=['line_endpoints_must_snap_within_tolerance'], snap_tolerance=0.01)`


---

## Topology Rule Validate

**Function name:** `topology_rule_validate`


Experimental

Validates vector topology against rule-set checks (self-intersection, overlap, gaps, dangles, point coverage, endpoint snapping) and emits feature-level violations.

data-tools vector topology rules qa

### Parameters

NameDescriptionRequiredDefault
`input`Input vector path.Required`input.gpkg`
`rule_set`Rule configuration as JSON array/object, CSV string, or file path. Defaults to all 6 MVP rules. Supported: line_must_not_self_intersect, polygon_must_not_overlap, polygon_must_not_have_gaps, line_must_not_have_dangles, point_must_be_covered_by_line, line_endpoints_must_snap_within_tolerance.Optional`['line_must_not_self_intersect', 'polygon_must_not_overlap', 'polygon_must_not_have_gaps', 'line_must_not_have_dangles', 'point_must_be_covered_by_line', 'line_endpoints_must_snap_within_tolerance']`
`snap_tolerance`Tolerance for line_endpoints_must_snap_within_tolerance rule in coordinate units. Defaults to 1.0.Optional`1.0`
`output`Output vector path for violations. If omitted, derived beside input.Optional`topology_rule_violations.gpkg`
`report`Optional JSON summary report path.Optional—

### Examples

*Validate network topology including self-intersections, dangles, and endpoint snapping.*
`wbe.topology_rule_validate(input='network.gpkg', output='network_topology_violations.gpkg', report='network_topology_violations.json', rule_set=['line_must_not_self_intersect', 'line_endpoints_must_snap_within_tolerance'], snap_tolerance=0.5)`


---

## Topology Validation Report

**Function name:** `topology_validation_report`


Experimental

Audits a vector layer for topology issues and writes a per-feature CSV report.

data-tools vector topology qa

### Parameters

NameDescriptionRequiredDefault
`input`Input vector path.Required`input.gpkg`
`output`Output CSV path. If omitted, a CSV path is derived beside the input.Optional`topology_report.csv`

### Examples

*Generate a CSV report of topology issues for a vector layer.*
`wbe.topology_validation_report(input='parcels.gpkg', output='parcels_topology_report.csv')`
