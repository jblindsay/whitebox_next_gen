# Geometry Processing


---

## Centroid Vector

**Function name:** `centroid_vector`


This can be used to identify the centroid point of a vector polyline or polygon feature or a group of vector points. The output is a vector shapefile of points. For multi-part polyline or polygon features, the user can optionally specify whether to identify the centroid of each part. The default is to treat multi-part features a single entity. 

For raster features, use the `Centroid` tool instead. 

### See Also

 

`Centroid`, `medoid` 

### Python API

```python
def centroid_vector(self, input: Vector) -> Vector:
```


---

## Concave Hull

**Function name:** `concave_hull`


Experimental

Creates concave hull polygons around all input feature coordinates.

vector hull boundary

### Parameters

NameDescriptionRequiredDefault
`input`Input vector layer.Required`input.shp`
`max_edge_length`Maximum edge length controlling hull detail.Required`50.0`
`epsilon`Robustness epsilon (default 1e-9).Optional`1e-09`
`output`Output vector path.Required—

### Examples

*Builds a concave hull from all input coordinates.*
`wbe.concave_hull(epsilon=1e-09, input='input.shp', max_edge_length=50.0, output='concave_hull.shp')`


---

## Densify Features

**Function name:** `densify_features`


Experimental

Add intermediate vertices to geometries at regular intervals. Improves accuracy for curved features or when reprojecting.

vector densify vertices

### Parameters

NameDescriptionRequiredDefault
`input`Input vector layer.Required`input.shp`
`spacing`Maximum spacing between adjacent vertices.Required`25.0`
`output`Output vector path.Required—

### Examples

*Adds regularly spaced vertices along geometry segments.*
`wbe.densify_features(input='input.shp', output='densified.shp', spacing=25.0)`


---

## Eliminate Coincident Points

**Function name:** `eliminate_coincident_points`


This tool can be used to remove any coincident, or nearly coincident, points from a vector points file. The user must specify the name of the input file, which must be of a POINTS VectorGeometryType, the output file name, and the tolerance distance. All points that are within the specified tolerance distance will be eliminated from the output file. A tolerance distance of 0.0 indicates that points must be exactly coincident to be removed. 

### See Also

 

`LidarRemoveDuplicates` 

### Python API

```python
def eliminate_coincident_points(self, input: Vector, tolerance_dist: float) -> Vector:
```


---

## Extend Vector Lines

**Function name:** `extend_vector_lines`


This tool can be used to extend vector lines by a specified distance. The user must input the names of the input and output shapefiles, the distance to extend features by, and whether to extend both ends, line starts, or line ends. The input shapefile must be of a POLYLINE base shape type and should be in a projected coordinate system. 

### Python API

```python
def extend_vector_lines(self, input: Vector, distance: float, extend_direction: str = "both") -> Vector:
```


---

## Merge Line Segments

**Function name:** `merge_line_segments`


Vector lines can sometimes contain two features that are connected by a shared end vertex. This tool identifies connected line features in an input vector file (`input`) and merges them in the output file (`output`). Two line features are merged if their ends are coincident, and are not coincident with any other feature (i.e. a bifurcation junction). End vertices are considered to be coincident if they are within the specified snap distance (`snap`). 

### See Also

 

`split_with_lines` 

### Python API

```python
def merge_line_segments(self, input: Vector, snap_tolerance: float = 2.220446049250313e-16) -> Vector:
```


---

## Minimum Bounding Box

**Function name:** `minimum_bounding_box`


This tool delineates the minimum bounding box (MBB) for a group of vectors. The MBB is the smallest box to completely enclose a feature. The algorithm works by rotating the feature, calculating the axis-aligned bounding box for each rotation, and finding the box with the smallest area, length, width, or perimeter. The MBB is needed to compute several shape indices, such as the Elongation Ratio. The `MinimumBoundingEnvelop` tool can be used to calculate the axis-aligned bounding rectangle around each feature in a vector file. 

### See Also

 

`minimum_bounding_circle`, `minimum_bounding_envelope`, `minimum_convex_hull` 

### Python API

```python
def minimum_bounding_box(self, input: Vector, min_criteria: str = "area", individual_feature_hulls: bool = True) -> Vector:
```


---

## Minimum Bounding Circle

**Function name:** `minimum_bounding_circle`


This tool delineates the minimum bounding circle (MBC) for a group of vectors. The MBC is the smallest enclosing circle to completely enclose a feature. 

### See Also

 

`minimum_bounding_box`, `minimum_bounding_envelope`, `minimum_convex_hull` 

### Python API

```python
def minimum_bounding_circle(self, input: Vector, individual_feature_hulls: bool = True) -> Vector:
```


---

## Minimum Bounding Envelope

**Function name:** `minimum_bounding_envelope`


This tool delineates the minimum bounding axis-aligned box for a group of vector features. The is the smallest rectangle to completely enclose a feature, in which the sides of the envelope are aligned with the x and y axis of the coordinate system. The `minimum_bounding_box` can be used instead to find the smallest possible non-axis aligned rectangular envelope. 

### See Also

 

`minimum_bounding_box`, `minimum_bounding_circle`, `minimum_convex_hull` 

### Python API

```python
def minimum_bounding_envelope(self, input: Vector, individual_feature_hulls: bool = True) -> Vector:
```


---

## Minimum Convex Hull

**Function name:** `minimum_convex_hull`


This tool creates a vector convex polygon around vector features. The convex hull is a convex closure of a set of points or polygon vertices and can be may be conceptualized as the shape enclosed by a rubber band stretched around the point set. The convex hull has many applications and is most notably used in various shape indices. The Delaunay triangulation of a point set and its dual, the Voronoi diagram, are mathematically related to convex hulls. 

### See Also

 

`minimum_bounding_box`, `minimum_bounding_circle`, `minimum_bounding_envelope` 

### Python API

```python
def minimum_convex_hull(self, input: Vector, individual_feature_hulls: bool = True) -> Vector:
```


---

## Polygonize

**Function name:** `polygonize`


This tool outputs a vector polygon layer from two or more intersecting line features contained in one or more input vector line files. Each space enclosed by the intersecting line set is converted to polygon added to the output layer. This tool should not be confused with the `lines_to_polygons` tool, which can be used to convert a vector file of polylines into a set of polygons, simply by closing each line feature. The `lines_to_polygons` tool does not deal with line intersection in the same way that the `polygonize` tool does. 

### See Also

 

`lines_to_polygons` 

### Python API

```python
def polygonize(self, input_layers: List[Vector]) -> Vector:
```


---

## Representative Point Vector

**Function name:** `representative_point_vector`


*No help documentation available for this tool.*


---

## Simplify Features

**Function name:** `simplify_features`


Experimental

Simplify geometries by removing detail while preserving shape. Reduces complexity for visualization or processing speed.

vector simplify generalization

### Parameters

NameDescriptionRequiredDefault
`input`Input vector layer.Required`input.shp`
`tolerance`Simplification tolerance in map units.Required`5.0`
`output`Output vector path.Required—

### Examples

*Simplifies geometry complexity while retaining shape.*
`wbe.simplify_features(input='input.shp', output='simplified.shp', tolerance=5.0)`


---

## Smooth Vectors

**Function name:** `smooth_vectors`


This tool smooths a vector coverage of either a POLYLINE or POLYGON base VectorGeometryType. The algorithm uses a simple moving average method for smoothing, where the size of the averaging window is specified by the user. The default filter size is 3 and can be any odd integer larger than or equal to 3. The larger the averaging window, the greater the degree of line smoothing. 

### Python API

```python
def smooth_vectors(self, input: Vector, filter_size: int = 3) -> Vector:
```


---

## Snap Endnodes

**Function name:** `snap_endnodes`


Experimental

Snaps nearby polyline endpoints to a shared location within a tolerance.

vector gis linework legacy-port

### Parameters

NameDescriptionRequiredDefault
`input`Input polyline vector layer.Required`input_lines.shp`
`snap_tolerance`Endpoint snapping tolerance in map units.Optional`2.220446049250313e-16`
`output`Output snapped polyline vector path.Required—

### Examples

*Snaps adjacent line endpoints by a tolerance threshold.*
`wbe.snap_endnodes(input='input_lines.shp', output='snapped_endnodes.shp', snap_tolerance=2.220446049250313e-16)`


---

## Split Vector Lines

**Function name:** `split_vector_lines`


This tool can be used to divide longer vector lines (`input`) into segments of a maximum specified length (`length`). 

### See Also

 

`assess_route` 

### Python API

```python
def split_vector_lines(self, input: Vector, segment_length: float) -> Vector:
```


---

## Split With Lines

**Function name:** `split_with_lines`


This tool splits the lines or polygons in one layer using the lines in another layer to define the breaking points. Intersection points between geometries in both layers are considered as split points. The input layer (`input`) can be of either POLYLINE or POLYGON VectorGeometryType and the output file will share this geometry type. The user must also specify an split layer (`split`), of POLYLINE VectorGeometryType, used to bisect the input geometries. 

Each split geometry's attribute record will contain `FID` and `PARENT_FID` values and all of the attributes (excluding `FID`'s) of the input layer. 

### See Also

 

'MergeLineSegments' 

### Python API

```python
def split_with_lines(self, input: Vector, split_vector: Vector) -> Vector:
```
