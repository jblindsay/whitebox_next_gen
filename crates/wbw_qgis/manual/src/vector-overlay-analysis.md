# Overlay Analysis


---

## Clip

**Function name:** `clip`


This tool will extract all the features, or parts of features, that overlap with the features of the clip vector file. The clipping operation is one of the most common vector overlay operations in GIS and effectively imposes the boundary of the clip layer on a set of input vector features, or target features. The operation is sometimes likened to a 'cookie-cutter'. The input vector file can be of any feature type (i.e. points, lines, polygons), however, the clip vector must consist of polygons. 

### See Also

 

`erase` 

### Python API

```python
def clip(self, input: Vector, clip_layer: Vector) -> Vector:
```


---

## Difference

**Function name:** `difference`


This tool will remove all the overlapping features, or parts of overlapping features, between input and overlay vector files, outputting only the features that occur in one of the two inputs but not both. The *Symmetrical Difference* is related to the Boolean exclusive-or (**XOR**) operation in  set theory and is one of the common vector overlay operations in GIS. The user must specify  the names of the input and overlay vector files as well as the output vector file name. The tool operates on vector points, lines, or polygon, but both the input and overlay files must contain the same VectorGeometryType. 

The *Symmetrical Difference* can also be derived using a combination of other vector overlay operations, as either `(A union B) difference (A intersect B)`, or `(A difference B) union (B difference A)`. 

The attributes of the two input vectors will be merged in the output attribute table. Fields that are duplicated between the inputs will share a single attribute in the output. Fields that only exist in one of the two inputs will be populated by `null` in the output table. Multipoint VectorGeometryTypes however will simply contain a single output feature identifier (`FID`) attribute. Also, note that depending on the VectorGeometryType (polylines and polygons), `Measure` and `Z` ShapeDimension data will not be transferred to the output geometries. If the input attribute table contains fields that measure the geometric properties of their associated features (e.g. length or area), these fields will not be updated to reflect changes in geometry shape and size resulting from the overlay operation. 

### See Also

 

`intersect`, `difference`, `union`, `clip`, `erase` 

### Python API

```python
def difference(self, input: Vector, overlay: Vector) -> Vector:
```


---

## Dissolve

**Function name:** `dissolve`


This tool can be used to remove the interior, or shared, boundaries within a vector polygon coverage. You can either dissolve all interior boundaries or dissolve those boundaries along polygons with the same value of a user-specified attribute within the vector's attribute table. It may be desirable to use the `VectorCleaning` tool to correct any topological errors resulting from the slight misalignment of nodes along shared boundaries in the vector coverage before performing the `dissolve` operation. 

### See Also

 

`clip`, `erase`, `polygonize` 

### Python API

```python
def dissolve(self, input: Vector, dissolve_field: str = "", snap_tolerance: float = 2.220446049250313e-16) -> Vector:
```


---

## Erase

**Function name:** `erase`


This tool will remove all the features, or parts of features, that overlap with the features of the erase vector file. The erasing operation is one of the most common vector overlay operations in GIS and effectively imposes the boundary of the erase layer on a set of input vector features, or target features. 

### See Also

 

`clip` 

### Python API

```python
def erase(self, input: Vector, erase_layer: Vector) -> Vector:
```


---

## Identity

**Function name:** `identity`


*No help documentation available for this tool.*


---

## Intersect

**Function name:** `intersect`


The result of the `intersect` vector overlay operation includes all the feature parts that occur in both input layers, excluding all other parts. It is analogous to the **OR** logical operator and multiplication in arithmetic. This tool is one of the common vector overlay operations in GIS. The user must specify the names of the input and overlay vector files as well as the output vector file name. The tool operates on vector points, lines, or polygon, but both the input and overlay files must contain the same VectorGeometryType. 

The `intersect` tool is similar to the `clip` tool. The difference is that the overlay vector layer in a `clip` operation must always be polygons, regardless of whether the input layer consists of points or polylines. 

The attributes of the two input vectors will be merged in the output attribute table. Note, duplicate fields should not exist between the inputs layers, as they will share a single attribute in the output (assigned from the first layer). Multipoint VectorGeometryTypes will simply contain a single output feature identifier (`FID`) attribute. Also, note that depending on the VectorGeometryType (polylines and polygons), `Measure` and `Z` ShapeDimension data will not be transferred to the output geometries. If the input attribute table contains fields that measure the geometric properties of their associated features (e.g. length or area), these fields will not be updated to reflect changes in geometry shape and size resulting from the overlay operation. 

### See Also

 

`difference`, `union`, `symmetrical_difference`, `clip`, `erase` 

### Python API

```python
def intersect(self, input: Vector, overlay: Vector, snap_tolerance: float = 2.220446049250313e-16) -> Vector:
```


---

## Line Intersections

**Function name:** `line_intersections`


This tool identifies points where the features of two vector line/polygon layers intersect. The user must specify the names of two input vector line files and the output file. The output file will be a vector of POINT VectorGeometryType. If the input vectors intersect at a line segment, the beginning and end vertices of the segment will be present in the output file. A warning is issued if intersection line segments are identified during analysis. If no intersections are found between the input line files, the output file will not be saved and a warning will be issued. 

Each intersection point will contain `PARENT1` and `PARENT2` attribute fields, identifying the instersecting features in the first and second input line files respectively. Additionally, the output attribute table will contain all of the attributes (excluding `FID`s) of the two parent line features. 

### Python API

```python
def line_intersections(self, input1: Vector, input2: Vector) -> Vector:
```


---

## Line Polygon Clip

**Function name:** `line_polygon_clip`


Experimental

Clips line features to polygon interiors and outputs clipped line segments.

vector clip line

### Parameters

NameDescriptionRequiredDefault
`input`Input line layer.Required`lines.shp`
`clip`Clip polygon layer.Required`clip_polygons.shp`
`output`Output vector path.Required—

### Examples

*Returns clipped line segments inside clip polygons.*
`wbe.line_polygon_clip(clip='clip_polygons.shp', input='lines.shp', output='line_polygon_clip.shp')`


---

## Near

**Function name:** `near`


Experimental

Find nearest neighbor features and optionally compute distance. Efficient for proximity analysis and distance calculations.

vector nearest distance

### Parameters

NameDescriptionRequiredDefault
`input`Input vector layer.Required`input.shp`
`near`Near-feature vector layer.Required`near.shp`
`max_distance`Optional maximum search distance.Optional—
`output`Output vector path.Required—

### Examples

*Computes nearest feature IDs and distances.*
`wbe.near(input='input.shp', near='near.shp', output='near_output.shp')`


---

## Select By Location

**Function name:** `select_by_location`


Experimental

Extracts target features that satisfy a spatial relationship to query features.

vector query spatial

### Parameters

NameDescriptionRequiredDefault
`target`Target feature layer to filter.Required`target.shp`
`query`Query feature layer.Required`query.shp`
`predicate`Spatial predicate: intersects, within, contains, touches, crosses, overlaps, disjoint, within_distance.Required`intersects`
`distance`Distance threshold for within_distance predicate.Optional—
`output`Output vector path.Required—

### Examples

*Selects target features that intersect query features.*
`wbe.select_by_location(output='selected.shp', predicate='intersects', query='query.shp', target='target.shp')`


---

## Spatial Join

**Function name:** `spatial_join`


Experimental

Join attributes from one vector layer to another based on spatial relationship. Uses spatial indexing for efficient processing.

vector join spatial

### Parameters

NameDescriptionRequiredDefault
`target`Target layer receiving joined attributes.Required`target.shp`
`join`Join layer providing attributes.Required`join.shp`
`predicate`Spatial predicate: intersects, within, contains, touches, crosses, overlaps, within_distance.Required`intersects`
`distance`Distance threshold for within_distance predicate.Optional—
`strategy`Join strategy: first, last, count, sum, mean, min, max.Optional`first`
`prefix`Prefix for joined field names (default JOIN_).Optional`JOIN_`
`output`Output vector path.Required—

### Examples

*Transfers join-layer attributes where geometries intersect.*
`wbe.spatial_join(join='join.shp', output='spatial_join.shp', predicate='intersects', prefix='JOIN_', strategy='first', target='target.shp')`


---

## Symmetrical Difference

**Function name:** `symmetrical_difference`


This tool will remove all the overlapping features, or parts of overlapping features, between input and overlay vector files, outputting only the features that occur in one of the two inputs but not both. The *Symmetrical Difference* is related to the Boolean exclusive-or (**XOR**) operation in  set theory and is one of the common vector overlay operations in GIS. The user must specify  the names of the input and overlay vector files as well as the output vector file name. The tool operates on vector points, lines, or polygon, but both the input and overlay files must contain the same VectorGeometryType. 

The *Symmetrical Difference* can also be derived using a combination of other vector overlay operations, as either `(A union B) difference (A intersect B)`, or `(A difference B) union (B difference A)`. 

The attributes of the two input vectors will be merged in the output attribute table. Fields that are duplicated between the inputs will share a single attribute in the output. Fields that only exist in one of the two inputs will be populated by `null` in the output table. Multipoint VectorGeometryTypes however will simply contain a single output feature identifier (`FID`) attribute. Also, note that depending on the VectorGeometryType (polylines and polygons), `Measure` and `Z` ShapeDimension data will not be transferred to the output geometries. If the input attribute table contains fields that measure the geometric properties of their associated features (e.g. length or area), these fields will not be updated to reflect changes in geometry shape and size resulting from the overlay operation. 

### See Also

 

`intersect`, `difference`, `union`, `clip`, `erase` 

### Python API

```python
def symmetrical_difference(self, input: Vector, overlay: Vector, snap_tolerance: float = 2.220446049250313e-16) -> Vector:
```


---

## Union

**Function name:** `union`


This tool splits vector layers at their overlaps, creating a layer containing all the portions from both input and overlay layers. The *Union* is related to the Boolean **OR** operation in  set theory and is one of the common vector overlay operations in GIS. The user must specify  the names of the input and overlay vector files as well as the output vector file name. The tool operates on vector points, lines, or polygon, but both the input and overlay files must contain the same VectorGeometryType. 

The attributes of the two input vectors will be merged in the output attribute table. Fields that are duplicated between the inputs will share a single attribute in the output. Fields that only exist in one of the two inputs will be populated by `null` in the output table. Multipoint VectorGeometryTypes however will simply contain a single output feature identifier (`FID`) attribute. Also, note that depending on the VectorGeometryType (polylines and polygons), `Measure` and `Z` ShapeDimension data will not be transferred to the output geometries. If the input attribute table contains fields that measure the geometric properties of their associated features (e.g. length or area), these fields will not be updated to reflect changes in geometry shape and size resulting from the overlay operation. 

### See Also

 

`intersect`, `difference`, `symmetrical_difference`, `clip`, `erase` 

### Python API

```python
def union(self, input: Vector, overlay: Vector, snap_tolerance: float = 2.220446049250313e-16) -> Vector:
```


---

## Update

**Function name:** `update`


*No help documentation available for this tool.*
