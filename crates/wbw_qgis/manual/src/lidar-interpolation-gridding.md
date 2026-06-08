# Interpolation and Gridding


---

## Flightline Overlap

**Function name:** `flightline_overlap`


This tool can be used to map areas of overlapping flightlines in an input LiDAR (LAS) file (`input`).  The output raster file (`output`) will contain the number of different flightlines that are contained within each grid cell. The user must specify the desired cell size (`resolution`). The flightline  associated with a LiDAR point is assumed to be contained within the point's `Point Source ID` property. Thus, the tool essentially counts the number of different Point Source ID values among the points contained within each grid cell. If the Point Source ID property is not set, or has been lost, users may with to apply the `recover_flightline_info` tool prior to running `flightline_overlap`. 

It is important to set the `resolution` parameter appropriately, as setting this value too high will yield the mis-characterization of non-overlap areas, and setting the resolution to low will result in fewer than expected overlap areas. An appropriate resolution size value may require experimentation, however a value that is 2-3 times the nominal point spacing has been previously recommended. The nominal point spacing can be determined using the `lidar_info` tool. 

Note that this tool is intended to be applied to LiDAR tile data containing points that have been merged from multiple overlapping flightlines. It is commonly the case that airborne LiDAR data from each of the flightlines from a survey are merged and then tiled into 1 km2 tiles, which are the target dataset for this tool. 

Like many of the LiDAR related tools, the input and output file parameters are optional. If left unspecified, the tool will locate all valid LiDAR files within the current Whitebox working directory and use these for calculation (specifying the output raster file name based on the associated input LiDAR file). This can be a helpful way to run the tool on a batch of user inputs within a specific directory. 

### See Also

 

`classify_overlap_points`, `recover_flightline_info`, `lidar_info` 

### Python API

```python
def flightline_overlap(self, input_lidar: Lidar, resolution: float = 1.0) -> Raster:
```


---

## LiDAR Block Maximum

**Function name:** `lidar_block_maximum`


This function superimposes a raster grid overtop of an input LiDAR point cloud (`input_lidar`) of a user-specified resolution (`cell_size`) and identifies the highest point in each block. The output raster therefore appoximates a digital surface model (DSM), representing the elevation of the ground surface in open areas and the elevations of off-terrain objects (OTOs), such as buildings and vegetation. While this function will be faster, it is recommended that if you use the `lidar_digital_surface_model` instead if you are trying to create a DSM. This method will  generally produce better results. 

Like many of the LiDAR functions, the input LiDAR point cloud (`input_lidar`) is optional. If an input LiDAR file  is not specified, the tool will search for all valid LiDAR (*.las, *.laz, *.zlidar) files contained within the current working directory. This feature can be very useful when you need to process a large number of LiDAR files contained within a directory. This batch processing mode enables the function to run in a more optimized parallel manner. When run in this batch mode, no output LiDAR object will be created. Instead the function will create an output file (saved to disc) with the same name as each input LiDAR file, but with the .tif extension. This can provide a very efficient means for processing extremely large LiDAR data sets. 

### See Also

 

`lidar_block_minimum`, `lidar_digital_surface_model`, `filterfilter_lidar_by_percentile_lidar` 

### Python API

```python
def lidar_block_maximum(self, input_lidar: Optional[Lidar], cell_size: float = 1.0) -> Raster:
```


---

## LiDAR Block Minimum

**Function name:** `lidar_block_minimum`


This function superimposes a raster grid overtop of an input LiDAR point cloud (`input_lidar`) of a user-specified resolution (`cell_size`) and identifies the lowest point in each block. The output raster therefore appoximates a bare-earth digital elevation model (DEM), or a digital terrain model (DTM), although it is likely to contain several off-terrain objects (OTOs), such as buildings. Under heavier forest cover, the minimum-surface will also very likely contain some blocks that are not coinincident with the ground surface, but rather will represent the elevation of the lower position of tree trunks and low vegetation. 

Like many of the LiDAR functions, the input LiDAR point cloud (`input_lidar`) is optional. If an input LiDAR file  is not specified, the tool will search for all valid LiDAR (*.las, *.laz, *.zlidar) files contained within the current working directory. This feature can be very useful when you need to process a large number of LiDAR files contained within a directory. This batch processing mode enables the function to run in a more optimized parallel manner. When run in this batch mode, no output LiDAR object will be created. Instead the function will create an output file (saved to disc) with the same name as each input LiDAR file, but with the .tif extension. This can provide a very efficient means for processing extremely large LiDAR data sets. 

### See Also

 

`lidar_block_maximum`, `filterfilter_lidar_by_percentile_lidar` 

### Python API

```python
def lidar_block_minimum(self, input_lidar: Optional[Lidar], cell_size: float = 1.0) -> Raster:
```


---

## LiDAR Construct Vector TIN

**Function name:** `lidar_construct_vector_tin`


This tool creates a vector triangular irregular network (TIN) for a set of LiDAR points (`input`) using a 2D `Delaunay triangulation` algorithm. LiDAR points may be excluded from the triangulation operation based on a number of criteria, include the point return number (`returns`), point classification value (`exclude_cls`), or a minimum (`minz`) or maximum (`maxz`) elevation. 

For vector points, use the `construct_vector_tin` tool instead. 

### See Also

 

`construct_vector_tin` 

### Python API

```python
def lidar_construct_vector_tin(self, input_lidar: Optional[Lidar], returns_included: str = "all", excluded_classes: List[int] = None, min_elev: float = float('-inf'), max_elev: float = float('inf'), max_triangle_edge_length: float = float('inf')) -> Vector:
```


---

## LiDAR Contour

**Function name:** `lidar_contour`


### Description

 

This tool can be used to create a `contour` (i.e. isolines of elevation values) vector coverage from an input LiDAR points data set (`input`). The tool works by first creating a `triangulation` of the input LiDAR points. The user must specify the contour interval (`interval`), or vertical spacing between contour lines. The `smooth` parameter can be used to increase or decrease the degree to which contours are smoothed. This parameter should be an odd integer value (0, 1, 3, 5...), with 0 indicating no smoothing. The tool can interpolate contours based on the LiDAR point elevation values, intensity data, or the user data field (`parameter`), with 'elevation' as the default parameter. LiDAR points may be excluded from the contouring process based on a number of criteria, including their return value (`returns`, which may be 'all', 'last', 'first'), their class value (`exclude_cls`), and whether they fall outside of a user-specified elevation range (`minz` and `maxz`). The optional `max_triangle_edge_length` parameter can be used to exclude the output of contours within areas that are sparsely populated areas of the data set, where the triangles formed by the Delaunay triangulation are too large. This is often the case within bodies of water; long and narrow triangular facets can also occur within the concave portions of the hull, or polygon enclosing, the points, when the data have an irregular shaped extent. Setting this parameter can help alleviate the problem of contouring beyond the data footprint. 

Like many of the LiDAR tools, both the `input` and `output` parameters are optional. If these parameters are not specified by the user, the tool will search for all LAS files contained within the current WhiteboxTools working directory. This feature can be useful when you need to contour a large number of LiDAR tiles. This batch processing mode enables the tool to enable parallel data processing, which can significantly improve the efficiency of data conversion for datasets with many LiDAR tiles. When run in this batch mode, the output file (`output`) also need not be specified; the tool will instead create an output file with the same name as each input LiDAR file, but with the `.shp` extension. 

It is important to note that contouring is better suited to well-defined surfaces (e.g. the ground surface or building heights), rather than volume features, such as vegetation, which tend to produce extremely complex contour sets. It is advisable to use this tool with last-returns and/or ground-classified point returns. If the input data set does not contain ground classification, consider pre-processing with the `lidar_ground_point_filter` tool. 

 

 

### See Also

 

`contours_from_points`, `contours_from_raster`, `lidar_ground_point_filter` 

### Python API

```python
def lidar_contour(self, input_lidar: Optional[Lidar], contour_interval: float = 10.0, base_contour: float = 0.0, smooth: int = 5, interpolation_parameter: str = "elevation", returns_included: str = "all",  excluded_classes: Optional[List[int]] = None, min_elev: float = float('-inf'), max_elev: float = float('inf'), tile_overlap: float = 0.0, max_triangle_edge_length: float = float('inf')) -> Optional[Vector]:
```


---

## LiDAR Digital Surface Model

**Function name:** `lidar_digital_surface_model`


This tool creates a digital surface model (DSM) from a LiDAR point cloud. A DSM reflects the elevation of the tops of all off-terrain objects (i.e. non-ground features) contained within the data set. For example, a DSM will model the canopy top as well as building roofs. This is in stark contrast to a bare-earth digital elevation model (DEM), which models the ground surface without off-terrain objects present. Bare-earth DEMs can be derived from LiDAR data by interpolating last-return points using one of the other LiDAR interpolators (e.g. `lidar_tin_gridding`). The algorithm used for interpolation in this tool is based on gridding a triangulation (TIN) fit to top-level points in the input LiDAR point cloud. All points in the input LiDAR data set that are below other neighbouring points, within a specified search radius (`radius`), and that have a large inter-point slope, are filtered out. Thus, this tool will remove the ground surface beneath as well as any intermediate points within a forest canopy, leaving only the canopy top surface to be interpolated. Similarly, building wall points and any ground points beneath roof overhangs will also be remove prior to interpolation. Note that because the ground points beneath overhead wires and utility lines are filtered out by this operation, these features tend to be appear as 'walls' in the output DSM. If these points are classified in the input LiDAR file, you may wish to filter them out before using this tool (`filter_lidar_classes`). 

The following images show the differences between creating a DSM using the `lidar_digital_surface_model` and by interpolating first-return points only using the `lidar_tin_gridding` tool respectively. Note, the images show `time_in_daylight`, which is a more effective way of hillshading DSMs than the traditional `hillshade` method. Compare how the DSM created `lidar_digital_surface_model` tool (above) has far less variability in areas of tree-cover, more effectively capturing the canopy top. As well, notice how building rooftops are more extensive and straighter in the `lidar_digital_surface_model` DSM image. This is because this method eliminates ground returns beneath roof overhangs before the triangulation operation. 

 

 

The user must specify the grid resolution of the output raster (`resolution`), and optionally, the name of the input LiDAR file (`input`) and output raster (`output`). Note that if an input LiDAR file (`input`) is not specified by the user, the tool will search for all valid LiDAR (*.las, *.laz, *.zlidar) files contained within the current working directory. This feature can be very useful when you need to interpolate a DSM for a large number of LiDAR files. Not only does this batch processing mode enable the tool to run in a more optimized parallel manner, but it will also allow the tool to include a small buffer of points extending into adjacent tiles when interpolating an individual file. This can significantly reduce edge-effects when the output tiles are later mosaicked together. When run in this batch mode, the output file (`output`) also need not be specified; the tool will instead create an output file with the same name as each input LiDAR file, but with the .tif extension. This can provide a very efficient means for processing extremely large LiDAR data sets. 

Users may also exclude points from the interpolation if they fall below or above the minimum (`minz`) or maximum (`maxz`) thresholds respectively. This can be a useful means of excluding anomalously high or low points. Note that points that are classified as low points (LAS class 7) or high noise (LAS class 18) are automatically excluded from the interpolation operation. 

Triangulation will generally completely fill the convex hull containing the input point data. This can sometimes result in very long and narrow triangles at the edges of the data or connecting vertices on either side of void areas. In LiDAR data, these void areas are often associated with larger waterbodies, and triangulation can result in very unnatural interpolated patterns within these areas. To avoid this problem, the user may specify a the maximum allowable triangle edge length (`max_triangle_edge_length`) and all grid cells within triangular facets with edges larger than this threshold are simply assigned the NoData values in the output DSM. These NoData areas can later be better dealt with using the `fill_missing_data` tool after interpolation. 

### See Also

 

`lidar_tin_gridding`, `filter_lidar_classes`, `fill_missing_data`, `time_in_daylight` 

### Python API

```python
def lidar_digital_surface_model(self, input_lidar: Optional[Lidar], cell_size: float = 1.0, search_radius: float = 0.5, min_elev: float = float('-inf'), max_elev: float = float('inf'), max_triangle_edge_length: float = float('inf')) -> Raster:
```


---

## LiDAR Hex Bin

**Function name:** `lidar_hex_bin`


The practice of binning point data to form a type of 2D histogram, density plot, or what is sometimes called a heatmap, is quite useful as an alternative for the cartographic display of of very dense points sets. This is particularly the case when the points experience significant overlap at the displayed scale. The `lidar_point_density` tool can be used to perform binning based on a regular grid (raster output). This tool, by comparison, bases the binning on a hexagonal grid. 

The tool is similar to the `CreateHexagonalVectorGrid` tool, however instead will create an output hexagonal grid in which each hexagonal cell possesses a `COUNT` attribute which specifies the number of points from an input points file (LAS file) that are contained within the hexagonal cell. The tool will also calculate the minimum and maximum elevations and intensity values and outputs these data to the attribute table. 

In addition to the names of the input points file and the output Shapefile, the user must also specify the desired hexagon width (w), which is the distance between opposing sides of each hexagon. The size (s) each side of the hexagon can then be calculated as, s = w / [2 x cos(PI / 6)]. The area of each hexagon (A) is, A = 3s(w / 2). The user must also specify the orientation of the grid with options of horizontal (pointy side up) and vertical (flat side up). 

### See Also

 

`vector_hex_binning`, `lidar_point_density`, `CreateHexagonalVectorGrid` 

### Python API

```python
def lidar_hex_bin(self, input_lidar: Lidar, width: float, orientation: str = "h") -> Vector:
```


---

## LiDAR Hillshade

**Function name:** `lidar_hillshade`


### Python API

```python
def lidar_hillshade(self, input: Lidar, search_radius: float = -1.0, azimuth: float = 315.0, altitude: float = 30.0) -> Lidar:
```


---

## LiDAR IDW Interpolation

**Function name:** `lidar_idw_interpolation`


This tool interpolates LiDAR files using `inverse-distance weighting` (IDW) scheme. The user must specify the value of the IDW weight parameter (`weight`). The output grid can be based on any of the stored LiDAR point parameters (`parameter`), including elevation (in which case the output grid is a digital elevation model, DEM), intensity, class, return number, number of returns, scan angle, RGB (colour) values, and user data values. Similarly, the user may specify which point return values (`returns`) to include in the interpolation, including all points, last returns (including single return points), and first returns (including single return points). 

The user must specify the grid resolution of the output raster (`resolution`), and optionally, the name of the input LiDAR file (`input`) and output raster (`output`). Note that if an input LiDAR file (`input`) is not specified by the user, the tool will search for all valid LiDAR (*.las, *.laz, *.zlidar) files contained within the current working directory. This feature can be very useful when you need to interpolate a DEM for a large number of LiDAR files. Not only does this batch processing mode enable the tool to run in a more optimized parallel manner, but it will also allow the tool to include a small buffer of points extending into adjacent tiles when interpolating an individual file. This can significantly reduce edge-effects when the output tiles are later mosaicked together. When run in this batch mode, the output file (`output`) also need not be specified; the tool will instead create an output file with the same name as each input LiDAR file, but with the .tif extension. This can provide a very efficient means for processing extremely large LiDAR data sets. 

Users may excluded points from the interpolation based on point classification values, which follow the LAS classification scheme. Excluded classes are specified using the `exclude_cls` parameter. For example, to exclude all vegetation and building classified points from the interpolation, use --exclude_cls='3,4,5,6'. Users may also exclude points from the interpolation if they fall below or above the minimum (`minz`) or maximum (`maxz`) thresholds respectively. This can be a useful means of excluding anomalously high or low points. Note that points that are classified as low points (LAS class 7) or high noise (LAS class 18) are automatically excluded from the interpolation operation. 

The tool will search for the nearest input LiDAR point to each grid cell centre, up to a maximum search distance (`radius`). If a grid cell does not have a LiDAR point within this search distance, it will be assigned the NoData value in the output raster. In LiDAR data, these void areas are often associated with larger waterbodies. These NoData areas can later be better dealt with using the `fill_missing_data` tool after interpolation. 

### See Also

 

`lidar_tin_gridding`, `lidar_nearest_neighbour_gridding`, `lidar_sibson_interpolation` 

### Python API

```python
def lidar_idw_interpolation(self, input_lidar: Optional[Lidar], interpolation_parameter: str = "elevation", returns_included: str = "all", cell_size: float = 1.0, idw_weight: float = 1.0, search_radius: float = 2.5, excluded_classes: List[int] = None, min_elev: float = float('-inf'), max_elev: float = float('inf')) -> Raster:
```


---

## LiDAR Nearest Neighbour Gridding

**Function name:** `lidar_nearest_neighbour_gridding`


This tool grids LiDAR files using nearest-neighbour (NN) scheme, that is, each grid cell in the output image will be assigned the parameter value of the point nearest the grid cell centre. This method should not be confused for the similarly named `natural-neighbour interpolation` (a.k.a Sibson's method). Nearest neighbour gridding is generally regarded as a poor way of interpolating surfaces from low-density point sets and results in the creation of a `Voronoi diagram`. However, this method has several advantages when applied to LiDAR data. NN gridding is one of the fastest methods for generating raster surfaces from large LiDAR data sets. NN gridding is one of the few interpolation methods, along with triangulation, that will preserve vertical breaks-in-slope, such as occur at the edges of building. This characteristic can be important when using some post-processing methods, such as the `remove_off_terrain_objects` tool. Furthermore, because most LiDAR data sets have remarkably high point densities compared with other types of geographic data, this approach does often produce a satisfactory result; this is particularly true when the point density is high enough that there are multiple points in the majority of grid cells. 

The output grid can be based on any of the stored LiDAR point parameters (`parameter`), including elevation (in which case the output grid is a digital elevation model, DEM), intensity, class, return number, number of returns, scan angle, RGB (colour) values, time, and user data values. Similarly, the user may specify which point return values (`returns`) to include in the interpolation, including all points, last returns (including single return points), and first returns (including single return points). 

The user must specify the grid resolution of the output raster (`resolution`), and optionally, the name of the input LiDAR file (`input`) and output raster (`output`). Note that if an input LiDAR file (`input`) is not specified by the user, the tool will search for all valid LiDAR (*.las, *.laz, *.zlidar) files contained within the current working directory. This feature can be very useful when you need to interpolate a DEM for a large number of LiDAR files. Not only does this batch processing mode enable the tool to run in a more optimized parallel manner, but it will also allow the tool to include a small buffer of points extending into adjacent tiles when interpolating an individual file. This can significantly reduce edge-effects when the output tiles are later mosaicked together. When run in this batch mode, the output file (`output`) also need not be specified; the tool will instead create an output file with the same name as each input LiDAR file, but with the .tif extension. This can provide a very efficient means for processing extremely large LiDAR data sets. 

Users may excluded points from the interpolation based on point classification values, which follow the LAS classification scheme. Excluded classes are specified using the `exclude_cls` parameter. For example, to exclude all vegetation and building classified points from the interpolation, use --exclude_cls='3,4,5,6'. Users may also exclude points from the interpolation if they fall below or above the minimum (`minz`) or maximum (`maxz`) thresholds respectively. This can be a useful means of excluding anomalously high or low points. Note that points that are classified as low points (LAS class 7) or high noise (LAS class 18) are automatically excluded from the interpolation operation. 

The tool will search for the nearest input LiDAR point to each grid cell centre, up to a maximum search distance (`radius`). If a grid cell does not have a LiDAR point within this search distance, it will be assigned the NoData value in the output raster. In LiDAR data, these void areas are often associated with larger waterbodies. These NoData areas can later be better dealt with using the `fill_missing_data` tool after interpolation. 

### See Also

 

`lidar_tin_gridding`, `lidar_idw_interpolation`, `lidar_tin_gridding`, `remove_off_terrain_objects`, `fill_missing_data` 

### Python API

```python
def lidar_nearest_neighbour_gridding(self, input_lidar: Optional[Lidar], interpolation_parameter: str = "elevation", returns_included: str = "all", cell_size: float = 1.0, search_radius: float = 2.5, excluded_classes: List[int] = None, min_elev: float = float('-inf'), max_elev: float = float('inf')) -> Raster:
```


---

## LiDAR Radial Basis Function Interpolation

**Function name:** `lidar_radial_basis_function_interpolation`


### Python API

```python
def lidar_radial_basis_function_interpolation(self, input_lidar: Optional[Lidar], interpolation_parameter: str = "elevation", returns_included: str = "all", cell_size: float = 1.0, num_points: int = 15, excluded_classes: List[int] = None, min_elev: float = float('-inf'), max_elev: float = float('inf'), func_type: str = "thinplatespline", poly_order: str = "none", weight: float = 0.1) -> Raster:
```


---

## LiDAR Sibson Interpolation

**Function name:** `lidar_sibson_interpolation`


### Description

 

This tool interpolates LiDAR files using `Sibson's interpolation method`, sometimes referred to as natural-neighbour interpolation (not to be confused with nearest-neighbour interpolation, `lidar_nearest_neighbour_gridding`). Sibon's method is based on assigning weight to points for which inserting a grid point would result in captured areas of the `Voronoi tessellation` of the input point set. The larger the captured area, the higher the weight assigned to the associated point. One of the main advantages of this natural neighbour approach to interpolation over similar techniques, such as inverse-distance weighting (IDW `lidar_idw_interpolation`), is that there is no need to specify a search distance or other interpolation weighting parameters. Sibson's approach frequently provides a very suitable interpolation for LiDAR data. The method requires the calculation of a Delaunay triangulation, from which the Voronoi tessellation is calculated. 

The user must specify the value of the IDW weight parameter (`weight`). The output grid can be based on any of the stored LiDAR point parameters (`parameter`), including elevation (in which case the output grid is a digital elevation model, DEM), intensity, class, return number, number of returns, scan angle values, and user data values. Similarly, the user may specify which point return values (`returns`) to include in the interpolation, including all points, last returns (including single return points), and first returns (including single return points). 

The user must specify the grid resolution of the output raster (`resolution`), and optionally, the name of the input LiDAR file (`input`) and output raster (`output`). Note that if an input LiDAR file (`input`) is not specified by the user, the tool will search for all valid LiDAR (*.las, *.laz, *.zlidar) files contained within the current working directory. This feature can be useful when you need to interpolate a DEM for a large number of LiDAR files. This batch processing mode enables the tool to include a small buffer of points extending into adjacent tiles when interpolating an individual file. This can significantly reduce edge-effects when the output tiles are later mosaicked together. When run in this batch mode, the output file (`output`) also need not be specified; the tool will instead create an output file with the same name as each input LiDAR file, but with the .tif extension. This can provide a very efficient means for processing extremely large LiDAR data sets. 

Users may excluded points from the interpolation based on point classification values, which follow the LAS classification scheme. Excluded classes are specified using the `exclude_cls` parameter. For example, to exclude all vegetation and building classified points from the interpolation, use --exclude_cls='3,4,5,6'. Users may also exclude points from the interpolation if they fall below or above the minimum (`minz`) or maximum (`maxz`) thresholds respectively. This can be a useful means of excluding anomalously high or low points. Note that points that are classified as low points (LAS class 7) or high noise (LAS class 18) are automatically excluded from the interpolation operation. 

### See Also

 

`lidar_tin_gridding`, `lidar_nearest_neighbour_gridding`, `lidar_idw_interpolation` 

### Python API

```python
def lidar_sibson_interpolation(self, input_lidar: Optional[Lidar], interpolation_parameter: str = "elevation", resolution: float = 1.0, returns_included: str = "all", excluded_classes: Optional[List[int]] = None, min_elev: float = float('-inf'), max_elev: float = float('inf')) -> Optional[Raster]:
```


---

## LiDAR Thin

**Function name:** `lidar_thin`


Thins a LiDAR point cloud, reducing point density. 

### Python API

```python
def lidar_thin(self, input: Lidar, resolution: float = 1.0, selection_method: str = "first", save_filtered: bool = False) -> Tuple[Lidar, Union[Lidar, None]]:
```


---

## LiDAR Thin High Density

**Function name:** `lidar_thin_high_density`


Thins points from high density areas within a LiDAR point cloud. 

### Python API

```python
def lidar_thin_high_density(self, input: Lidar, density: float, resolution: float = 1.0, save_filtered: bool = False) -> Tuple[Lidar, Union[Lidar, None]]:
```


---

## LiDAR Tile Footprint

**Function name:** `lidar_tile_footprint`


This tool can be used to create a vector polygon of the bounding box or convex hull of a LiDAR point cloud (i.e. LAS file). If the user specified an input file (`input`) and output file (`output`), the tool will calculate the footprint, containing all of the data points, and output this feature to a vector polygon file. If the `input` and `output` parameters are left unspecified, the tool will calculate the footprint of every LAS file contained within the working directory and output these features to a single vector polygon file. If this is the desired mode of operation, it is important to specify the working directory (`wd`) containing the group of LAS files; do not specify the optional `input` and `output` parameters in this case. Each polygon in the output vector will contain a `LAS_NM` field, specifying the source LAS file name, a `NUM_PNTS` field, containing the number of points within the source file, and Z_MIN and Z_MAX fields, containing the minimum and maximum elevations. This output can therefore be useful to create an index map of a large tiled LiDAR dataset. 

By default, this tool identifies the axis-aligned minimum rectangular hull, or bounding box, containing the points in each of the input tiles. If the user specifies the `hull` flag, the tool will identify the `minimum convex hull` instead of the bounding box. This option is considerably more computationally intensive and will be a far longer running operation if many tiles are specified as inputs. 

**A note on LAZ file inputs:** While WhiteboxTools does not currently support the reading and writing of the compressed LiDAR format `LAZ`, it is able to read `LAZ` file headers. This tool, when run in in the bounding box mode (rather than the convex hull mode), is able to take `LAZ` input files. 

`lidar_tile`, `LayerFootprint`, `minimum_bounding_box`, `minimum_convex_hull` 

### Python API

```python
def lidar_tile_footprint(self, input_lidar: Optional[Lidar], output_hulls: bool = False) -> Vector:
```


---

## LiDAR TIN Gridding

**Function name:** `lidar_tin_gridding`


This tool creates a raster grid based on a Delaunay triangular irregular network (TIN) fitted to LiDAR points. The output grid can be based on any of the stored LiDAR point parameters (`parameter`), including elevation (in which case the output grid is a digital elevation model, DEM), intensity, class, return number, number of returns, scan angle, RGB (colour) values, and user data values. Similarly, the user may specify which point return values (`returns`) to include in the interpolation, including all points, last returns (including single return points), and first returns (including single return points). 

The user must specify the grid resolution of the output raster (`resolution`), and optionally, the name of the input LiDAR file (`input`) and output raster (`output`). Note that if an input LiDAR file (`input`) is not specified by the user, the tool will search for all valid LiDAR (*.las, *.laz, *.zlidar) files contained within the current working directory. This feature can be very useful when you need to interpolate a DEM for a large number of LiDAR files. Not only does this batch processing mode enable the tool to run in a more optimized parallel manner, but it will also allow the tool to include a small buffer of points extending into adjacent tiles when interpolating an individual file. This can significantly reduce edge-effects when the output tiles are later mosaicked together. When run in this batch mode, the output file (`output`) also need not be specified; the tool will instead create an output file with the same name as each input LiDAR file, but with the .tif extension. This can provide a very efficient means for processing extremely large LiDAR data sets. 

Users may excluded points from the interpolation based on point classification values, which follow the LAS classification scheme. Excluded classes are specified using the `exclude_cls` parameter. For example, to exclude all vegetation and building classified points from the interpolation, use --exclude_cls='3,4,5,6'. Users may also exclude points from the interpolation if they fall below or above the minimum (`minz`) or maximum (`maxz`) thresholds respectively. This can be a useful means of excluding anomalously high or low points. Note that points that are classified as low points (LAS class 7) or high noise (LAS class 18) are automatically excluded from the interpolation operation. 

Triangulation will generally completely fill the convex hull containing the input point data. This can sometimes result in very long and narrow triangles at the edges of the data or connecting vertices on either side of void areas. In LiDAR data, these void areas are often associated with larger waterbodies, and triangulation can result in very unnatural interpolated patterns within these areas. To avoid this problem, the user may specify a the maximum allowable triangle edge length (`max_triangle_edge_length`) and all grid cells within triangular facets with edges larger than this threshold are simply assigned the NoData values in the output DSM. These NoData areas can later be better dealt with using the `fill_missing_data` tool after interpolation. 

### See Also

 

`lidar_idw_interpolation`, `lidar_nearest_neighbour_gridding`, `lidar_tin_gridding`, `filter_lidar_classes`, `fill_missing_data` 

### Python API

```python
def lidar_tin_gridding(self, input_lidar: Optional[Lidar], interpolation_parameter: str = "elevation", returns_included: str = "all", cell_size: float = 1.0, excluded_classes: List[int] = None, min_elev: float = float('-inf'), max_elev: float = float('inf'), max_triangle_edge_length: float = float('inf')) -> Raster:
```
