# Raster-Vector Conversion


---

## Convert Nodata To Zero

**Function name:** `convert_nodata_to_zero`


### Description

 

This tool can be used to change the value within the grid cells of a raster (`input`) that contain NoData to zero. The most common reason for using this tool is to change the background region of a raster image such that it can be included in analysis since NoData values are usually ignored by by most tools. This change, however, will result in the background no longer displaying transparently in most GIS. This change can be reversed using the `set_nodata_value` tool. 

### See Also

 

`set_nodata_value`, `Raster.is_nodata` 

### Parameters

 

raster (Raster):     The input Raster object 

### Returns

 

Raster: the returning value 

### Python API

```python
def convert_nodata_to_zero(self, raster: Raster) -> Raster:
```


---

## Modify Nodata Value

**Function name:** `modify_nodata_value`


This tool can be used to modify the value of pixels containing the NoData value for an input raster image. This operation differs from the `set_nodata_value` tool, which sets the NoData value for an image in the image header without actually modifying pixel values. Also, `set_nodata_value` does not overwrite the input file, while the `modify_nodata_value` tool does. This tool cannot modify the input image data type, which is important to note since it may cause an unexpected behaviour if the new NoData value is negative and the input image data type is an unsigned integer type. 

### See Also

 

`set_nodata_value`, `convert_nodata_to_zero` 

### Python API

```python
def modify_nodata_value(self, input: Raster, new_value: float = -32768.0) :
```


---

## New Raster From Base Raster

**Function name:** `new_raster_from_base_raster`


This tool can be used to create a new raster with the same coordinates and dimensions (i.e. rows and columns) as an existing base image. The user must input a base file (`base`), the value that the new grid will be filled with (`out_val`; NoData if unspecified), and  the data type (`data_type` flag; options include 'double', 'float', and 'integer'). 

### See Also

 

`new_raster_from_base_vector`, `raster_cell_assignment` 

### Python API

```python
def new_raster_from_base_raster(self, base: Raster, out_val: float = float('nan'), data_type: str = "float") -> Raster:
```


---

## New Raster From Base Vector

**Function name:** `new_raster_from_base_vector`


This tool can be used to create a new raster with the same spatial extent as an input vector file (`base`). The user must specify the name of the base file, the value that the new grid will be filled with (`out_val`; NoData if unspecified), and the  data type (`data_type` flag; options include 'double', 'float', and 'integer'). It is also necessary to specify a value for the optional grid cell size (`cell_size`) input parameter. 

### See Also

 

`new_raster_from_base_raster`, `raster_cell_assignment` 

### Python API

```python
def new_raster_from_base_vector(self, base: Vector, cell_size: float, out_val: float = float('nan'), data_type: str = "float") -> Raster:
```


---

## Raster To Vector Lines

**Function name:** `raster_to_vector_lines`


This tool converts raster lines features into a vector of the POLYLINE VectorGeometryType. Grid cells associated with line features will contain non-zero, non-NoData cell values. The algorithm requires three passes of the raster. The first pass counts the number of line neighbours of each line cell; the second pass traces line segments starting from line ends (i.e. line cells with only one neighbouring line cell); lastly, the final pass traces any remaining line segments, which are likely forming closed loops (and therefore do not have line ends). 

If the line raster contains streams, it is preferable to use the `raster_streams_to_vector` instead. This tool will use knowledge of flow directions to ensure connections between stream segments at confluence sites, whereas `raster_to_vector_lines` will not. 

### See Also

 

`raster_to_vector_polygons`, `raster_to_vector_points`, `raster_streams_to_vector` 

### Python API

```python
def raster_to_vector_lines(self, raster: Raster) -> Vector:
```


---

## Raster To Vector Points

**Function name:** `raster_to_vector_points`


Converts a raster data set to a vector of the POINT VectorGeometryType. The user must specify the name of a raster file (`input`) and the name of the output vector (`output`). Points will correspond with grid cell centre points. All grid cells containing non-zero, non-NoData values will be considered a point. The vector's attribute table will contain a field called 'VALUE' that will contain the cell value for each point feature. 

### See Also

 

`raster_to_vector_polygons`, `raster_to_vector_lines` 

### Python API

```python
def raster_to_vector_points(self, raster: Raster) -> Vector:
```


---

## Raster To Vector Polygons

**Function name:** `raster_to_vector_polygons`


Converts a raster data set to a vector of the POLYGON geometry type. The user must specify the name of a raster file (`input`) and the name of the output (`output`) vector. All grid cells containing non-zero, non-NoData values will be considered part of a polygon feature. The vector's attribute table will contain a field called 'VALUE' that will contain the cell value for each polygon feature, in addition to the standard feature ID (FID) attribute. 

### See Also

 

`raster_to_vector_points`, `raster_to_vector_lines` 

### Python API

```python
def raster_to_vector_polygons(self, raster: Raster) -> Vector:
```


---

## Remove Raster Polygon Holes

**Function name:** `remove_raster_polygon_holes`


### Description

 

This tool can be used to remove holes in raster polygons. Holes are areas of background values (either zero or  NoData), completely surrounded by foreground values (any value other than zero or NoData). Therefore, this tool can somewhat be considered to be the raster equivalent to the vector-based `RemovePolygonHoles` tool. Users may  optionally remove holes less than a specified threshold size (`--threshold`), measured in grid cells. Hole size is determined using a clumping operation, similar to what is used by the `Clump` tool. Users may also optionally specify whether or not to included 8-cell diagonal connectedness during the clumping operation (`--use_diagonals`). 

`Some GIS professionals`  have previously used a `closing` operation to lessen the extent of polygon holes in raster data. A closing is a mathematical morphology operation that involves expanding the raster polygons using a dialation filter (`MaximumFilter`), followed by a dialation filter (`MinimumFilter`) on the resulting image. While this common image processing technique can be helpful for reducing the prevalance of polygon holes, it can also have considerable impact on non-hole features within the image. The `RemoveRasterPolygonHoles` tool, by comparison, will only affect hole features and does not impact the boundaries of other polygons at all. The following image compares the removal of polygon holes (islands in a lake polygon) using a closing operation (middle) calculated using an 11x11  convolution filter and the output of the `RemoveRasterPolygonHoles` tool. Notice how the convolution operation  impacts the edges of the polygon, particularly in convex regions, compared with the `RemoveRasterPolygonHoles`. 

 

`**Here**` is a video that demonstrates how to apply this tool to a classified  Sentinel-2 multi-spectral satellite imagery data set. 

### See Also

 

`closing`, `remove_polygon_holes`, `clump`, `generalize_classified_raster` 

### Python API

```python
def remove_raster_polygon_holes(self, input: Raster, threshold_size: int = sys.maxsize, use_diagonals: bool = False) -> Raster:
```


---

## Set Nodata Value

**Function name:** `set_nodata_value`


This tool will re-assign a user-defined background value in an input raster image the **NoData** value. More precisely, the NoData value will be changed to the specified background value and any existing grid cells containing the previous NoData value, if it had been defined, will be changed to this new value. Most WhiteboxTools tools recognize NoData grid cells and treat them specially. NoData grid cells are also often displayed transparently by GIS software. The user must specify the names of the input and output rasters and the background value. The default background value is zero, although any numeric value is possible. 

This tool differs from the `ModifyNoDataValue` tool in that it simply updates the NoData value in the raster header, without modifying pixel values. The `ModifyNoDataValue` tool will update the value in the header, and then modify each existing NoData pixel to contain this new value. Also, `set_nodata_value` does not overwrite the input file, while the `ModifyNoDataValue` tool does. 

This tool may result in a change in the data type of the output image compared with the input image, if  the background value is set to a negative value and the input image data type is an unsigned integer. In some cases, this may result in a doubling of the storage size of the output image. 

### See Also

 

`ModifyNoDataValue`, `convert_nodata_to_zero`, `IsNoData` 

### Python API

```python
def set_nodata_value(self, raster: Raster, back_value: float = 0.0) -> Raster:
```


---

## Vector Lines To Raster

**Function name:** `vector_lines_to_raster`


This tool can be used to convert a vector lines or polygon file into a raster grid of lines. If a vector of one of the polygon VectorGeometryTypes is selected, the resulting raster will outline the polygons without filling these features. Use the `VectorPolygonToRaster` tool if you need to fill the polygon features. 

The user must specify the name of the input vector (`input`) and the output raster file (`output`). The Field Name (`field`) is the field from the attributes table, from which the tool will retrieve the information to assign to grid cells in the output raster. Note that if this field contains numerical data with no decimals, the output raster data type will be INTEGER; if it contains decimals it will be of a FLOAT data type. The field must contain numerical data. If the user does not supply a Field Name parameter, each feature in the raster will be assigned the record number of the feature. The assignment operation determines how the situation of multiple points contained within the same grid cell is handled. The background value is the value that is assigned to grid cells in the output raster that do not correspond to the location of any points in the input vector. This value can be any numerical value (e.g. 0) or the string 'NoData', which is the default. 

If the user optionally specifies the `cell_size` parameter then the coordinates will be determined by the input vector (i.e. the bounding box) and the specified Cell Size. This will also determine the number of rows and columns in the output raster. If the user instead specifies the optional base raster file parameter (`base`), the output raster's coordinates (i.e. north, south, east, west) and row and column count will be the same as the base file. If the user does not specify either of these two optional parameters, the tool will determine the cell size automatically as the maximum of the north-south extent (determined from the shapefile's bounding box) or the east-west extent divided by 500. 

### See Also

 

`vector_points_to_raster`, `vector_polygons_to_raster` 

### Python API

```python
def vector_lines_to_raster(self, input: Vector, field_name: str = "FID", zero_background: bool = False, cell_size: float = 0.0, base_raster: Raster = None) -> Raster:
```


---

## Vector Points To Raster

**Function name:** `vector_points_to_raster`


This tool can be used to convert a vector points file into a raster grid. The user must specify the name of the input vector and the output raster file. The field name (`field`) is the field from the attributes table from which the tool will retrieve the information to assign to grid cells in the output raster. The field must contain numerical data. If the user does not supply a field name parameter, each feature in the raster will be assigned the record number of the feature. The assignment operation determines how the situation of multiple points contained within the same grid cell is handled. The background value is zero by default but can be set to `NoData` optionally using the `nodata` value. 

If the user optionally specifies the grid cell size parameter (`cell_size`) then the coordinates will be determined by the input vector (i.e. the bounding box) and the specified cell size. This will also determine the number of rows and columns in the output raster. If the user instead specifies the optional base raster file parameter (`base`), the output raster's coordinates (i.e. north, south, east, west) and row and column count will be the same as the base file. 

In the case that multiple points are contained within a single grid cell, the output can be assigned (`assign`) the first, last (default), min, max, sum, mean, or number of the contained points. 

### See Also

 

`vector_polygons_to_raster`, `vector_lines_to_raster` 

### Python API

```python
def vector_points_to_raster(self, input: Vector, field_name: str = "FID", assign_op: str = "last", zero_background: bool = False, cell_size: float = 0.0, base_raster: Raster = None) -> Raster:
```


---

## Vector Polygons To Raster

**Function name:** `vector_polygons_to_raster`


public constructor 

### Python API

```python
def vector_polygons_to_raster(self, input: Vector, field_name: str = "FID", zero_background: bool = False, cell_size: float = 0.0, base_raster: Raster = None) -> Raster:
```
