# Sampling and Gridding


---

## Construct Vector TIN

**Function name:** `construct_vector_tin`


This tool creates a vector triangular irregular network (TIN) for a set of vector points (`input`) using a 2D `Delaunay triangulation` algorithm. TIN vertex heights can be assigned based on either a field in the vector's attribute table (`field`), or alternatively, if the vector is of a z-dimension *VectorGeometryTypeDimension*, the point z-values may be used for vertex heights (`use_z`). For LiDAR points, use the `lidar_construct_vector_tin` tool instead. 

Triangulation often creates very long, narrow triangles near the edges of the data coverage, particularly in convex regions along the data boundary. To avoid these spurious triangles, the user may optionally specify the maximum allowable edge length of a triangular facet (`max_triangle_edge_length`). 

### See Also

 

`lidar_construct_vector_tin` 

### Python API

```python
def construct_vector_tin(self, input_points: Vector, field_name: str = "FID", use_z: bool = False, max_triangle_edge_length: float = float('inf')) -> Vector:
```


---

## Contours From Points

**Function name:** `contours_from_points`


This tool creates a contour coverage from a set of input points (`input`). The user must specify the contour interval (`interval`) and optionally, the base contour value (`base`). The degree to which contours are smoothed is controlled by the **Smoothing Filter Size** parameter (`smooth`). This value, which determines the size of a mean filter applied to the x-y position of vertices in each contour, should be an odd integer value, e.g. 3, 5, 7, 9, 11, etc. Larger values will result in smoother contour lines. 

### See Also

 

`contours_from_raster` 

### Python API

```python
def contours_from_points(self, input: Vector, field_name: str = "", use_z_values: bool = False, max_triangle_edge_length: float = float('inf'), contour_interval: float = 10.0, base_contour: float = 0.0, smoothing_filter_size: int = 9) -> Vector:
```


---

## Contours From Raster

**Function name:** `contours_from_raster`


This tool can be used to create a vector contour coverage from an input raster surface model (`input`), such as a digital elevation model (DEM). The user must specify the contour interval (`interval`) and optionally, the base contour value (`base`). The degree to which contours are smoothed is controlled by the **Smoothing Filter Size** parameter (`smooth`). This value, which determines the size of a mean filter applied to the x-y position of vertices in each contour, should be an odd integer value, e.g. 3, 5, 7, 9, 11, etc. Larger values will result in smoother contour lines. The tolerance parameter (`tolerance`) controls the amount of line generalization. That is, vertices in a contour line will be selectively removed from the line if they do not result in an angular deflection in the line's path of at least this threshold value. Increasing this value can significantly decrease the size of the output contour vector file, at the cost of generating straighter contour line segments. 

### See Also

 

`raster_to_vector_polygons` 

### Python API

```python
def contours_from_raster(self, raster_surface: Raster, contour_interval: float = 10.0, base_contour: float = 0.0, smoothing_filter_size: int = 9, deflection_tolerance: float = 10.0) -> Vector:
```


---

## Extract Nodes

**Function name:** `extract_nodes`


This tool converts vector lines or polygons into vertex points. The user must specify the name of the input vector, which must be of a polyline or polygon base shape type, and the name of the output point-type vector. 

### Python API

```python
def extract_nodes(self, input: Vector) -> Vector:
```


---

## Extract Raster Values At Points

**Function name:** `extract_raster_values_at_points`


This tool can be used to extract the values of one or more rasters (`inputs`) at the sites of a set of vector points. By default, the data is output to the attribute table of the input points (`points`) vector; however, if the `out_text` parameter is specified, the tool will additionally output point values as text data to standard output (*stdout*). Attribute fields will be added to the table of the points file, with field names, *VALUE1*, *VALUE2*, *VALUE3*, etc. each corresponding to the order of input rasters. 

If you need to plot a chart of values from a raster stack at a set of points, the `image_stack_profile` may be more suitable for this application. 

### See Also

 

`image_stack_profile`, `find_lowest_or_highest_points` 

### Python API

```python
def extract_raster_values_at_points(self, rasters: List[Raster], points: Vector) -> Tuple[Vector, str]:
```


---

## Find Lowest Or Highest Points

**Function name:** `find_lowest_or_highest_points`


This tool locates the lowest and/or highest cells in a raster and outputs these locations to a vector points file. The user must specify the name of the input raster (`input`) and the name of the output vector file (`output`). The user also has the option (`out_type`) to locate either the lowest value, highest value, or both values. The output vector's attribute table will contain fields for the points XY coordinates and their values. 

### See Also

 

`extract_raster_values_at_points` 

### Python API

```python
def find_lowest_or_highest_points(self, raster: Raster, output_type: str = "lowest") -> Vector:
```


---

## Hexagonal Grid From Raster Base

**Function name:** `hexagonal_grid_from_raster_base`


This tool can be used to create a hexagonal vector grid. The extent of the hexagonal grid is based on the extent of an input raster base file (`base`). The user must also  specify the hexagonal cell width (`width`) and whether the hexagonal orientation (`orientation`)  is `horizontal` or `vertical`. To use a vector base image instead of a raster, use the  `hexagonal_grid_from_vector_base` tool. 

### See Also

 

`hexagonal_grid_from_vector_base` 

### Python API

```python
def hexagonal_grid_from_raster_base(self, base: Raster, width: float, orientation: str = "h") -> Vector:
```


---

## Hexagonal Grid From Vector Base

**Function name:** `hexagonal_grid_from_vector_base`


This tool can be used to create a hexagonal vector grid. The extent of the hexagonal grid is based on the extent of an input vector base file (`base`). The user must also  specify the hexagonal cell width (`width`) and whether the hexagonal orientation (`orientation`)  is `horizontal` or `vertical`. To use a raster base image instead of a vector, use the  `hexagonal_grid_from_raster_base` tool. 

### See Also

 

`hexagonal_grid_from_raster_base` 

### Python API

```python
def hexagonal_grid_from_vector_base(self, base: Vector, width: float, orientation: str = "h") -> Vector:
```


---

## Layer Footprint Raster

**Function name:** `layer_footprint_raster`


This tool creates a vector polygon footprint of the area covered by an input raster grid (`input`). It will create a vector rectangle corresponding to the bounding box of the input raster. 

If input data are irregular shape (i.e. there a boundary of NoData cells) the resulting vector will still correspond to the full grid extent, ignoring the irregular boundary. If this is not  the desired effect, you may consider the `minimum_bounding_envelope` tool instead.  

### See Also

 

`layer_footprint_vector`, `minimum_bounding_envelope` 

### Python API

```python
def layer_footprint_raster(self, input: Raster) -> Vector:
```


---

## Layer Footprint Vector

**Function name:** `layer_footprint_vector`


This tool creates a vector polygon footprint of the area covered by a vector layer. It will create a vector rectangle corresponding to the bounding box. The user must specify the name of the input file (`input`). 

If input data are irregular shape the resulting vector will still correspond to the full grid extent, ignoring the irregular boundary. If this is not the desired effect, you should use the `minimum_bounding_envelope` tool instead.  

### See Also

 

`layer_footprint_raster`, `minimum_bounding_envelope` 

### Python API

```python
def layer_footprint_vector(self, input: Vector) -> Vector:
```


---

## Medoid

**Function name:** `medoid`


This tool calculates the medoid for a series of vector features contained in a shapefile. The medoid of a two-dimensional feature is conceptually similar its centroid, or mean position, but the medoid is always a members of the input feature data set. Thus, the medoid is a measure of central tendency that is robust in the presence of outliers. If the input vector is of a POLYLINE or POLYGON VectorGeometryType, the nodes of each feature will be used to estimate the feature medoid. If the input vector is of a POINT base VectorGeometryType, the medoid will be calculated for the collection of points. While there are more than one competing method of calculating the medoid, this tool uses an algorithm that works as follows: 
 
- The x-coordinate and y-coordinate of each point/node are placed into two arrays. 
- The x- and y-coordinate arrays are then sorted and the median x-coordinate (Med X) and median y-coordinate (Med Y) are calculated. 
- The point/node in the dataset that is nearest the point (Med X, Med Y) is identified as the medoid. 
 

### See Also

 

`centroid_vector` 

### Python API

```python
def medoid(self, input: Vector) -> Vector:
```


---

## Random Points In Polygon

**Function name:** `random_points_in_polygon`


Experimental

Generates random points uniformly within input polygon geometries.

vector sampling random

### Parameters

NameDescriptionRequiredDefault
`input`Input polygon layer.Required`polygons.shp`
`num_points`Number of random points to create.Required`100`
`seed`Optional RNG seed for reproducibility.Optional—
`output`Output vector path.Required—

### Examples

*Generates random sample points inside polygon boundaries.*
`wbe.random_points_in_polygon(input='polygons.shp', num_points=100, output='random_points.shp')`


---

## Rectangular Grid From Raster Base

**Function name:** `rectangular_grid_from_raster_base`


This tool can be used to create a rectangular vector grid. The extent of the rectangular grid is based on the extent of an input base raster (`base`). The user may also specify  the origin of the grid (`xorig` and `yorig`, defaults are 0.0) and the grid cell width  and height (`width` and `height`).  

### See Also

 

`rectangular_grid_from_vector_base`, `hexagonal_grid_from_raster` 

### Python API

```python
def rectangular_grid_from_raster_base(self, base: Raster, width: float, height: float, x_origin: float = 0.0, y_origin: float = 0.0) -> Vector:
```


---

## Rectangular Grid From Vector Base

**Function name:** `rectangular_grid_from_vector_base`


This tool can be used to create a rectangular vector grid. The extent of the rectangular grid is based on the extent of an input base vector (`base`). The user may also specify  the origin of the grid (`xorig` and `yorig`, defaults are 0.0) and the grid cell width  and height (`width` and `height`).  

### See Also

 

`rectangular_grid_from_raster_base`, `hexagonal_grid_from_vector` 

### Python API

```python
def rectangular_grid_from_vector_base(self, base: Vector, width: float, height: float, x_origin: float = 0.0, y_origin: float = 0.0) -> Vector:
```


---

## Vector Hex Binning

**Function name:** `vector_hex_binning`


The practice of binning point data to form a type of 2D histogram, density plot, or what is sometimes called a heatmap, is quite useful as an alternative for the cartographic display of of very dense points sets. This is particularly the case when the points experience significant overlap at the displayed scale. The `PointDensity` tool can be used to perform binning based on a regular grid (raster output). This tool, by comparison, bases the binning on a hexagonal grid. 

The tool is similar to the `CreateHexagonalVectorGrid` tool, however instead will create an output hexagonal grid in which each hexagonal cell possesses a `COUNT` attribute which specifies the number of points from an input points file (Shapefile vector) that are contained within the hexagonal cell. 

In addition to the names of the input points file and the output Shapefile, the user must also specify the desired hexagon width (w), which is the distance between opposing sides of each hexagon. The size (s) each side of the hexagon can then be calculated as, s = w / [2 x cos(PI / 6)]. The area of each hexagon (A) is, A = 3s(w / 2). The user must also specify the orientation of the grid with options of horizontal (pointy side up) and vertical (flat side up). 

### See Also

 

`LidarHexBinning`, `PointDensity`, `CreateHexagonalVectorGrid` 

### Python API

```python
def vector_hex_binning(self, vector_points: Vector, width: float, orientation: str = "h") -> Vector:
```


---

## Voronoi Diagram

**Function name:** `voronoi_diagram`


This tool creates a vector Voronoi diagram for a set of vector points. The Voronoi diagram is the dual graph of the Delaunay triangulation. The tool operates by first constructing the Delaunay triangulation and then connecting the circumcenters of each triangle. Each Voronoi cell contains one point of the input vector points. All locations within the cell are nearer to the contained point than any other input point. 

A dense frame of 'ghost' (hidden) points is inserted around the input point set to limit the spatial extent of the diagram. The frame is set back from the bounding box of the input points by 2 x the average point  spacing. The polygons of these ghost points are not output, however, points that are situated along the edges of the data will have somewhat rounded (paraboloic) exterior boundaries as a result of this edge condition. If this property is unacceptable for application, clipping the Voronoi diagram to the convex hull may be a better alternative. 

This tool works on vector input data only. If a Voronoi diagram is needed to tessellate regions associated with a set of raster points, use the `euclidean_allocation` tool instead. To use Voronoi diagrams for gridding data (i.e. raster interpolation), use the `NearestNeighbourGridding` tool. 

### See Also

 

`construct_vector_tin`, `euclidean_allocation`, `NearestNeighbourGridding` 

### Python API

```python
def voronoi_diagram(self, input_points: Vector) -> Vector:
```
