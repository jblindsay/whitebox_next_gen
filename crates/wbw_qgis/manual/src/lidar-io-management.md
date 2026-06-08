# I/O and Data Management


---

## Ascii To LAS

**Function name:** `ascii_to_las`


This tool can be used to convert one or more ASCII files, containing LiDAR point data, into LAS files. The user must specify the name(s) of the input ASCII file(s) (`inputs`). Each input file will have a correspondingly named output file with a `.las` file extension. The output point data, each on a separate line, will take the format: 

`x,y,z,intensity,class,return,num_returns" `  ValueInterpretation xx-coordinate yy-coordinate zelevation iintensity value cclassification rnreturn number nrnumber of returns timeGPS time sascan angle rred bblue ggreen   

The `x`, `y`, and `z` patterns must always be specified. If the `rn` pattern is used, the `nr` pattern must also be specified. Examples of valid pattern string include: 

`'x,y,z,i' 'x,y,z,i,rn,nr' 'x,y,z,i,c,rn,nr,sa' 'z,x,y,rn,nr' 'x,y,z,i,rn,nr,r,g,b' ` Use the `las_to_ascii` tool to convert a LAS file into a text file containing LiDAR point data. 

### See Also

 

`las_to_ascii` 

### Python API

```python
def ascii_to_las(self, input_ascii_files: List[str], pattern: str, epsg_code: int) -> None:
```


---

## LAS To Ascii

**Function name:** `las_to_ascii`


This tool can be used to convert one or more LAS file, containing LiDAR data, into ASCII files. The user must specify the name(s) of the input LAS file(s) (`inputs`). Each input file will have a correspondingly named output file with a `.csv` file extension. CSV files are comma separated value files and contain tabular data with each column corresponding to a field in the table and each row a point value. Fields are separated by commas in the ASCII formatted file. The output point data, each on a separate line, will take the format: 

`X,Y,Z,INTENSITY,CLASS,RETURN,NUM_RETURN,SCAN_ANGLE ` If the LAS file has a point format that contains RGB data, the final three columns will contain the RED, GREEN, and BLUE values respectively. Use the `ascii_to_las` tool to convert a text file containing LiDAR point data into a LAS file. 

### See Also

 

`ascii_to_las` 

### Python API

```python
def las_to_ascii(self, input_lidar: Optional[Lidar]) -> None:
```


---

## LAS To Shapefile

**Function name:** `las_to_shapefile`


This tool converts one or more LAS files into a POINT vector. When the input parameter is not specified, the tool grids all LAS files contained within the working directory. The attribute table of the output Shapefile will contain fields for the z-value, intensity, point class, return number, and number of return. 

This tool can be used in place of the `LasToMultipointShapefile` tool when the number of points are relatively low and when the desire is to represent more than simply the x,y,z position of points. Notice however that because each point in the input LAS file will be represented as a separate record in the output Shapefile, the output file will be many time larger than the equivalent output of the `LasToMultipointShapefile` tool. There is also a practical limit on the total number of records that can be held in a single Shapefile and large LAS files approach this limit. In these cases, the `LasToMultipointShapefile` tool should be preferred instead. 

### See Also

 

`LasToMultipointShapefile` 

### Python API

```python
def las_to_shapefile(self, input_lidar: Optional[Lidar], output_multipoint: bool = False) -> Vector:
```


---

## LiDAR Colourize

**Function name:** `lidar_colourize`


This tool can be used to add red-green-blue (RGB) colour values to the points contained within an input LAS file (`in_lidar`), based on the pixel values of an overlapping input colour image (`in_image`). Ideally, the image has been acquired at the same time as the LiDAR point cloud. If this is not the case, one may expect that transient objects (e.g. cars) in both input data sets will be incorrectly coloured. The input image should overlap in extent with the LiDAR data set and the two data sets should share the same projection. You may use the `lidar_tile_footprint` tool to determine the spatial extent of the LAS file. 

 

 

 

### See Also

 

`colourize_based_on_class`, `colourize_based_on_point_returns`, `lidar_tile_footprint` 

### Python API

```python
def lidar_colourize(self, in_lidar: Lidar, in_image: Raster) -> Lidar:
```


---

## LiDAR Join

**Function name:** `lidar_join`


This tool can be used to merge multiple LiDAR LAS files into a single output LAS file. Due to their large size, LiDAR data sets are often tiled into smaller, non-overlapping tiles. Sometimes it is more convenient to combine multiple tiles together for data processing and `lidar_join` can be used for this purpose. 

### See Also

 

`lidar_tile` 

### Python API

```python
def lidar_join(self, inputs: List[Lidar]) -> Lidar:
```


---

## LiDAR Shift

**Function name:** `lidar_shift`


This tool can be used to shift the x,y,z coordinates of points within a LiDAR file. The user must specify  the name of the input file (`input`) and the output file (`output`). Additionally, the user must specify the x,y,z shift values (`x_shift`, `y_shift`, `z_shift`). At least one non-zero shift value is needed to run the tool. Notice that shifting the x,y,z coordinates of LiDAR points is also possible using the  `modify_lidar` tool, which can also be used for more sophisticated point property manipulation (e.g. rotations). 

### See Also

 

`modify_lidar`, `lidar_elevation_slice`, `height_above_ground` 

### Python API

```python
def lidar_shift(self, input: Lidar, x_shift: float = 0.0, y_shift: float = 0.0, z_shift: float = 0.0) -> Lidar:
```


---

## LiDAR Tile

**Function name:** `lidar_tile`


single LAS file. The user must specify the parameter of the tile grid, including its origin (`origin_x` and `origin_y`) and the tile width and height (`width` and `height`). Tiles containing fewer points than specified in the `min_points` parameter will not be output. This can be useful when tiling terrestrial LiDAR datasets because the low point density at the edges of the point cloud (i.e. most distant from the scan station) can result in poorly populated tiles containing relatively few points. 

### See Also

 

`lidar_join`, `lidar_tile_footprint` 

### Python API

```python
def lidar_tile(self, input_lidar: Lidar, tile_width: float = 1000.0, tile_height: float = 1000.0, origin_x: float = 0.0, origin_y: float = 0.0, min_points_in_tile: int = 2, output_laz_format: bool = True) -> None:
```


---

## LiDAR Tophat Transform

**Function name:** `lidar_tophat_transform`


This tool performs a white `top-hat transform` on a LiDAR point cloud (`input`). A top-hat transform is a common digital image processing operation used for various tasks, such as feature extraction, background equalization, and image enhancement. When applied to a LiDAR point cloud, the white top-hat transform provides an estimate of *height above ground*, which is useful for modelling the vegetation canopy. 

As an example, notice that the input point cloud on the top of the image below has a substantial amount of topographic variability. After applying the top-hat transform (bottom point cloud), all of this topographic  variability has been removed and point elevations values effectively become height above ground. 

 

The white top-hat transform is defined as the difference between a point's original elevation and its `opening`. The opening operation can be thought of as the local neighbourhood maximum of a previous local minimum surface. The user must specify the size of the neighbourhood using the `radius` parameter. Setting this parameter can require some experimentation. Generally, it is appropriate to use a radius of a few meters in non-urban landscapes. However, in urban areas, the radius may need to be set much larger, reflective of the size of the largest building. 

If the input point cloud already has ground points classified, it may be better to use the `height_above_ground`, which simply measures the difference in height between each point and its nearest ground classified point within the search radius. 

### See Also

 

`height_above_ground`, `tophat_transform`, `closing`, `opening` 

### Python API

```python
def lidar_tophat_transform(self, input: Lidar, search_radius: float) -> Lidar:
```


---

## Recover Flightline Info

**Function name:** `recover_flightline_info`


### Description

 

Raw airborne LiDAR data are collected along flightlines and multiple flightlines are typically merged into square tiles to simplify data handling and processing. Commonly the *Point Source ID* attribute is used to store information about the origin flightline of each point. However, sometimes this information is lost (e.g. during data format conversion) or is omitted from some data sets. This tool can be used to identify groups of points within a LiDAR file (`input`) that belong to the same flightline. 

The tool works by sorting points based on their timestamp and then identifying points for which the time difference from the previous point is greater than a user-specified maximum time difference (`max_time_diff`), which are deemed to be the start of a different flightline. The operational assumption is that the time between consecutive points within a flightline is usually quite small (usually a fraction of a second), while the time between points in different flightlines is often relatively large (consider the aircraft turning time needed to take multiple passes of the study area). By default the maximum time difference is set to 5.0 seconds, although it may be necessary to increase this value depending on the characteristics of a particular data set. 

The tool works on individual LiDAR tiles and the flightline identifiers will range from 0 to the number of flightlines detected within the tile, minus one. Therefore, the flightline identifier created by this tool will not extend beyond the boundaries of the tile and into adjacent tiles. That is, a flightline that extends across multiple adjacent LiDAR tiles may have different flightline identifiers used in each tile. The identifiers are intended to discriminate between flighlines within a single file. The flightline identifier value can be optionally assigned to the *Point Source ID* point attribute (`pt_src_id`), the *User Data* point attribute (`user_data`), and the red-green-blue point colour data (`rgb`) within the output file (`output`). At least one of these output options must be selected and it is possible to select multiple output options. Notice that if the input file contains any information within the selected output fields, the original information will be over-written, and therefore lost--of course, it will remain unaltered within the input file, which this tool does not modify. If the input file does not contain RGB colour data and the `rgb` output option is selected, the output file point format will be altered from the input file to accommodate the addition of RGB colour data. Flightlines are assigned random colours. The LAS *User Data* point attribute is stored as a single byte and, therefore, if this output option is selected and the input file contains more than 256 flightlines, the tool will assign the same flightline identifier to more than one flightline. It is very rare for this condition to be the case in a typical 1 km-square tiles. The *Point Source ID* attribute is stored as a 16-bit integer and can therefore store 65,536 unique flightline identifiers. 

Outputting flightline information within the colour data point attribute can be useful for visualizing areas of flightline overlap within a file. This can be an important quality assurance/quality control (QA/QC) step after acquiring a new LiDAR data set. 

 

Please note that because this tool sorts points by their timestamps, the order of points in the output file may not match that of the input file. 

### See Also

 

`flightline_overlap`, `find_flightline_edge_points`, `LidarSortByTime` 

### Python API

```python
def recover_flightline_info(self, input: Lidar, max_time_diff: float = 5.0, pt_src_id: bool = False, user_data: bool = False, rgb: bool = False) -> Lidar:
```


---

## Select Tiles By Polygon

**Function name:** `select_tiles_by_polygon`


This tool copies LiDAR tiles overlapping with a polygon into an output directory. In actuality, the tool performs point-in-polygon operations, using the four corner points, the center point, and the four mid-edge points of each LiDAR tile bounding box and the polygons. This representation of overlapping geometry aids with performance. This approach generally works well when the polygon size is large relative to the LiDAR tiles. If, however, the input polygon is small relative to the tile size, this approach may miss some copying some tiles. It is advisable to buffer the polygon if this occurs. 

### See Also

 

`lidar_tile_footprint` 

### Python API

```python
def select_tiles_by_polygon(self, input_directory: str, output_directory: str, polygons: Vector) -> None:
```


---

## Sort LiDAR

**Function name:** `sort_lidar`


### Description

 

This tool can be used to sort the points in an input LiDAR file (`input`) based on their properties with respect to one or more sorting criteria (`criteria`). The sorting criteria may include: the x, y or z coordinates (`x`, `y`, `z`), the intensity data (`intensity`), the point class value (`class`), the point user data field (`user_data`), the return number (`ret_num`), the point source ID (`point_source_id`), the point scan angle data (`scan_angle`), the scanner channel (`scanner_channel`; LAS 1.4 datasets only), and the acquisition time (`time`). The following is an example of a complex sorting criteria statement that includes multiple criteria: 

`x 100.0, y 100.0, z 10.0, scan_angle` 

Criteria should be separated by a comma, semicolon, or pipe (|). Each criteria may have an associated bin value. In the example above, point `x` values are sorted into bins of 100 m, which are then sorted by `y` values into bins of 100 m, and sorted by point `z` values into bins of 10 m, and finally sorted by their `scan_angle`. 

Sorting point values can have a significant impact on the compression rate when using certain compressed LiDAR data formats (e.g. LAZ, zLidar). Sorting values can also improve the visualization speed in some rendering software. 

Note that if the user does not specify the optional input LiDAR file, the tool will search for all valid LiDAR (*.las, *.laz, *.zlidar) files contained within the current working directory. This feature can be useful for processing a large number of LiDAR files in batch mode. When this batch mode is applied, the output file names will be the same as the input file names but with a '_sorted' suffix added to the end. 

### Python API

```python
def sort_lidar(self, sort_criteria: str, input_lidar: Optional[Lidar]) -> Optional[Lidar]:
```


---

## Split LiDAR

**Function name:** `split_lidar`


### Description

 

This tool can be used to split an input LiDAR file (`input`) into a series of output files, placing points into each output based on their properties with respect to a grouping criterion (`criterion`). Points can be grouped based on a specified the number of points in the output file (`num_pts`; note the last file may contain fewer points), the x, y or z coordinates (`x`, `y`, `z`), the intensity data (`intensity`), the point class value (`class`), the point user data field (`user_data`), the point source ID (`point_source_id`), the point scan angle data (`scan_angle`), and the acquisition time (`time`). Points are binned into groupings based on a user-specified interval value (`interval`). For example, if an interval of 50.0 is used with the `z` criterion, a series of files will be output that are elevation bands of 50 m. The user may also optionally specify the minimum number of points needed before a particular grouping file is saved (`min_pts`). The interval value is not used for the `class` and `point_source_id` criteria. 

With this tool, a single input file can generate many output files. The names of the output files will be reflective of the point attribute used for the grouping and the bin. For example, running the tool with the on an input file named my_file.las using `intensity` criterion and with an interval of 1000 may produce the following files: 
 
- my_file_intensity0.las 
- my_file_intensity1000.las 
- my_file_intensity2000.las 
- my_file_intensity3000.las 
- my_file_intensity4000.las 
 

Where the number after the attribute (intensity, in this case) reflects the lower boundary of the bin. Thus, the first file contains all of the input points with intensity values from 0 to just less than 1000. 

Note that if the user does not specify the optional input LiDAR file, the tool will search for all valid LiDAR (*.las, *.laz, *.zlidar) files contained within the current working directory. This feature can be useful for processing a large number of LiDAR files in batch mode. When this batch mode is applied, the output file names will be the same as the input file names but with a suffix added to the end reflective of the split criterion and value (see above). 

### See Also

 

`sort_lidar`, `filter_lidar`, `modify_lidar`, `lidar_elevation_slice` 

### Python API

```python
def split_lidar(self, split_criterion: str, input_lidar: Optional[Lidar], interval: float = 5.0, min_pts: int = 5) -> None:
```
