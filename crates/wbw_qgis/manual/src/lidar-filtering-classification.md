# Filtering and Classification


---

## Classify Buildings In LiDAR

**Function name:** `classify_buildings_in_lidar`


This tool can be used to assign the building class (classification value 6) to all points within an input LiDAR point cloud (`input`) that are contained within the polygons of an input buildings footprint vector (`buildings`). The tool performs a simple point-in-polygon operation to determine membership. The two inputs (i.e. the LAS file and vector) must share the same map projection. Furthermore, any error in the definition of the building footprints will result in misclassified points in the output LAS file (`output`). In particular, if the footprints extend slightly beyond the actual building, ground points situated adjacent to the building will be incorrectly classified. Thus, care must be taken in digitizing building footprint polygons. Furthermore, where there are tall trees that overlap significantly with the building footprint, these vegetation points will also be incorrectly assigned the building class value. 

### See Also

 

`filter_lidar_classes`, `lidar_ground_point_filter`, `clip_lidar_to_polygon` 

### Python API

```python
def classify_buildings_in_lidar(self, in_lidar: Lidar, building_footprints: Vector) -> Lidar:
```


---

## Classify LiDAR

**Function name:** `classify_lidar`


### Description

 

This tool provides a basic classification of a LiDAR point cloud into ground, building, and vegetation classes. The algorithm performs the classification based on point neighbourhood geometric properties, including planarity, linearity, and height above the ground. There is also a point segmentation involved in the classification process. 

 

The user may specify the names of the input and output LiDAR files (`input` and `output`). Note that if the user does not specify the optional input/output LiDAR files, the tool will search for all valid LiDAR (*.las, *.laz, *.zlidar) files contained within the current working directory. This feature can be useful for processing a large number of LiDAR files in batch mode. When this batch mode is applied, the output file names will be the same as the input file names but with a '_classified' suffix added to the end. 

The search distance (`radius`), defining the radius of the neighbourhood window surrounding each point, must also be specified. If this parameter is set to a value that is too large, areas of high surface curvature on the ground surface will be left unclassed and smaller buildings, e.g. sheds, will not be identified. If the parameter is set too small, areas of low point density may provide unsatisfactory classification values. The larger this search distance is, the longer the algorithm will take to processs a data set. For many airborne LiDAR data sets, a value between 1.0 - 3.0 meters is likely appropriate. 

The ground threshold parameter (`grd_threshold`) determines how far above the `tophat-transformed` surface a point must be to be excluded from the ground surface. This parameter also determines the maximum distance a point can be from a plane or line model fit to a neighbourhood of points to be considered part of the model geometry. Similarly the off-terrain object threshold parameter (`oto_threshold`) is used to determine how high above the ground surface a point must be to be considered either a vegetation or building point. The ground threshold must be smaller than the off-terrain object threshold. If you find that breaks-in-slope in areas of more complex ground topography are left unclassed (class = 1), this can be addressed by raising the ground threshold parameter. 

The planarity and linearity thresholds (`planarity_threshold` and `linearity_threshold`) describe the minimum proportion (0-1) of neighbouring points that must be part of a fitted model before the point is considered to be planar or linear. Both of these properties are used by the algorithm in a variety of ways to determine final class values. Planar and linear models are fit using a `RANSAC-like` algorithm, with the main user-specified parameter of the number of iterations (`iterations`). The larger the number of iterations the greater the processing time will be. 

The facade threshold (`facade_threshold`) is the last user-specified parameter, and determines the maximum horizontal distance that a point beneath a rooftop edge point may be to be considered part of the building facade (i.e. walls). The default value is 0.5 m, although this value will depend on a number of factors, such as whether or not the building has balconies. 

The algorithm generally does very well to identify deciduous (broad-leaf) trees but can at times struggle with incorrectly classifying dense coniferous (needle-leaf) trees as buildings. When this is the case, you may counter this tendency by lowering the planarity threshold parameter value. Similarly, the algorithm will generally leave overhead power lines as unclassified (class = 1), howevever, if you find that the algorithm misclassifies most such points as high vegetation (class = 5), this can be countered by lowering the linearity threshold value. 

Note that if the input file already contains class data, these data will be overwritten in the output file. 

### See Also

 

`colourize_based_on_class`, `filter_lidar`, `modify_lidar`, `sort_lidar`, `split_lidar` 

### Python API

```python
def classify_lidar(self, input_lidar: Optional[Lidar], search_radius: float = 2.5, grd_threshold: float = 0.1, oto_threshold: float = 1.0, linearity_threshold: float = 0.5, planarity_threshold: float = 0.85, num_iter: int = 30, facade_threshold: float = 0.5) -> Optional[Lidar]:
```


---

## Classify Overlap Points

**Function name:** `classify_overlap_points`


This tool can be used to flag points within an input LiDAR file (`input`) that overlap with other  nearby points from different flightlines, i.e. to identify overlap points. The flightline associated  with a LiDAR point is assumed to be contained within the point's `Point Source ID` (PSID) property. If the PSID property is not set, or has been lost, users may with to apply the `recover_flightline_info`  tool prior to running `flightline_overlap`. 

Areas of multiple flightline overlap tend to have point densities that are far greater than areas of single flightlines. This can produce suboptimal results for applications that assume regular point  distribution, e.g. in point classification operations. 

The tool works by applying a square grid over the extent of the input LiDAR file. The grid cell size is  determined by the user-defined `resolution` parameter.  Grid cells containing multiple PSIDs, i.e.  with more than one flightline, are then identified. Overlap points within these grid cells can then be  flagged on the basis of a user-defined `criterion`. The flagging options include the following:  CriterionOverlap Point Definition `max scan angle`All points that share the PSID of the point with the maximum absolute scan angle `not min point source ID`All points with a different PSID to that of the point with the lowest PSID `not min time`All points with a different PSID to that of the point with the minimum GPS time `multiple point source IDs`All points in grid cells with multiple PSIDs, i.e. all overlap points.   

Note that the `max scan angle` criterion may not be appropriate when more than two flightlines overlap,  since it will result in only flagging points from one of the multiple flightlines.  

It is important to set the `resolution` parameter appropriately, as setting this value too high will yield the filtering of points in non-overlap areas, and setting the resolution to low will result in fewer than expected points being flagged. An appropriate resolution size value may require experimentation, however a value that is 2-3 times the nominal point spacing has been previously recommended. The nominal point spacing can be determined using the `lidar_info` tool. 

By default, all flagged overlap points are reclassified in the output LiDAR file (`output`) to class  12. Alternatively, if the user specifies the `filter` parameter, then each overlap point will be  excluded from the output file. Classified overlap points may also be filtered from LiDAR point clouds using the `filter_lidar` tool. 

Note that this tool is intended to be applied to LiDAR tile data containing points that have been merged from multiple overlapping flightlines. It is commonly the case that airborne LiDAR data from each of the flightlines from a survey are merged and then tiled into 1 km2 tiles, which are the target dataset for this tool. 

### See Also

 

`flightline_overlap`, `recover_flightline_info`, `filter_lidar`, `lidar_info` 

### Python API

```python
def classify_overlap_points(self, in_lidar: Lidar, resolution: float = 1.0, overlap_criterion: str = "max scan angle", filter: bool = False) -> Lidar:
```


---

## Clip LiDAR To Polygon

**Function name:** `clip_lidar_to_polygon`


This tool can be used to isolate, or clip, all of the LiDAR points in a LAS file (`input`) contained within one or more vector polygon features. The user must specify the name of the input clip file (--polygons), which must be a vector of a Polygon base shape type. The clip file may contain multiple polygon features and polygon hole parts will be respected during clipping, i.e. LiDAR points within polygon holes will be removed from the output LAS file. 

Use the `erase_polygon_from_lidar` tool to perform the complementary operation of removing points from a LAS file that are contained within a set of polygons. 

### See Also

 

`erase_polygon_from_lidar`, `filter_lidar`, `clip`, `clip_raster_to_polygon` 

### Python API

```python
def clip_lidar_to_polygon(self, input: Lidar, polygons: Vector) -> Lidar:
```


---

## Erase Polygon From LiDAR

**Function name:** `erase_polygon_from_lidar`


This tool can be used to isolate, or clip, all of the LiDAR points in a LAS file (`input`) contained within one or more vector polygon features. The user must specify the name of the input clip file (--polygons), which must be a vector of a Polygon base shape type. The clip file may contain multiple polygon features and polygon hole parts will be respected during clipping, i.e. LiDAR points within polygon holes will be removed from the output LAS file. 

Use the `erase_polygon_from_lidar` tool to perform the complementary operation of removing points from a LAS file that are contained within a set of polygons. 

### See Also

 

`erase_polygon_from_lidar`, `filter_lidar`, `clip`, `clip_raster_to_polygon` 

### Python API

```python
def erase_polygon_from_lidar(self, input: Lidar, polygons: Vector) -> Lidar:
```


---

## Filter LiDAR

**Function name:** `filter_lidar`


### Description

 

The FilterLidar tool is a very powerful tool for filtering points within a LiDAR point cloud based on point properties. Complex filter statements (`statement`) can be used to include or exclude points in the output file (`output`). 

Note that if the user does not specify the optional input LiDAR file (`input`), the tool will search for all valid LiDAR (*.las, *.laz, *.zlidar) files contained within the current working directory. This feature can be useful for processing a large number of LiDAR files in batch mode. When this batch mode is applied, the output file names will be the same as the input file names but with a '_filtered' suffix added to the end. 

Points are either included or excluded from the output file by creating conditional filter statements. Statements must be valid Rust syntax and evaluate to a Boolean. Any of the following variables are acceptable within the filter statement:  Variable NameDescription xThe point x coordinate yThe point y coordinate zThe point z coordinate intensityThe point intensity value retThe point return number nretThe point number of returns is_onlyTrue if the point is an only return (i.e. ret == nret == 1), otherwise false is_multipleTrue if the point is a multiple return (i.e. nret > 1), otherwise false is_earlyTrue if the point is an early return (i.e. ret == 1), otherwise false is_intermediateTrue if the point is an intermediate return (i.e. ret > 1 && ret  is_lateTrue if the point is a late return (i.e. ret == nret), otherwise false is_firstTrue if the point is a first return (i.e. ret == 1 && nret > 1), otherwise false is_lastTrue if the point is a last return (i.e. ret == nret && nret > 1), otherwise false classThe class value in numeric form, e.g. 0 = Never classified, 1 = Unclassified, 2 = Ground, etc. is_noiseTrue if the point is classified noise (i.e. class == 7class == 18), otherwise false is_syntheticTrue if the point is synthetic, otherwise false is_keypointTrue if the point is a keypoint, otherwise false is_withheldTrue if the point is withheld, otherwise false is_overlapTrue if the point is an overlap point, otherwise false scan_angleThe point scan angle scan_directionTrue if the scanner is moving from the left towards the right, otherwise false is_flightline_edgeTrue if the point is situated along the filightline edge, otherwise false user_dataThe point user data point_source_idThe point source ID scanner_channelThe point scanner channel timeThe point GPS time, if it exists, otherwise 0 redThe point red value, if it exists, otherwise 0 greenThe point green value, if it exists, otherwise 0 blueThe point blue value, if it exists, otherwise 0 nirThe point near infrared value, if it exists, otherwise 0 pt_numThe point number within the input file n_ptsThe number of points within the file min_xThe file minimum x value mid_xThe file mid-point x value max_xThe file maximum x value min_yThe file minimum y value mid_yThe file mid-point y value max_yThe file maximum y value min_zThe file minimum z value mid_zThe file mid-point z value max_zThe file maximum z value dist_to_ptThe distance from the point to a specified xy or xyz point, e.g. dist_to_pt(562500, 4819500) or dist_to_pt(562500, 4819500, 320) dist_to_lineThe distance from the point to the line passing through two xy points, e.g. dist_to_line(562600, 4819500, 562750, 4819750) dist_to_line_segThe distance from the point to the line segment defined by two xy end-points, e.g. dist_to_line_seg(562600, 4819500, 562750, 4819750) within_rect1 if the point falls within the bounds of a 2D or 3D rectangle, otherwise 0. Bounds are defined as within_rect(ULX, ULY, LRX, LRY) or within_rect(ULX, ULY, ULZ, LRX, LRY, LRZ)   

In addition to the point properties defined above, if the user applies the `lidar_eigenvalue_features` tool on the input LiDAR file, the `filter_lidar` tool will automatically read in the additional *.eigen file, which include the eigenvalue-based point neighbourhood measures, such as `lambda1`, `lambda2`, `lambda3`, `linearity`, `planarity`, `sphericity`, `omnivariance`, `eigentropy`, `slope`, and `residual`. See the `lidar_eigenvalue_features` documentation for details on each of these metrics describing the structure and distribution of points within the neighbourhood surrounding each point in the LiDAR file. 

Statements can be as simple or complex as desired. For example, to filter out all points that are classified noise (i.e. class numbers 7 or 18): 

`!is_noise ` The following is a statement to retain only the late returns from the input file (i.e. both last and single returns): 

`ret == nret ` Notice that equality uses the `==` symbol an inequality uses the `!=` symbol. As an equivalent to the above statement, we could have used the `is_late` point property: 

`is_late ` If we want to remove all points outside of a range of xy values: 

`x >= 562000 && x <= 562500 && y >= 4819000 && y <= 4819500 ` Notice how we can combine multiple constraints using the `&&` (logical AND) and `||` (logical OR) operators. As an alternative to the above statement, we could have used the `within_rect` function: 

`within_rect(562000, 4819500, 562500, 4819000) ` If we want instead to exclude all of the points within this defined region, rather than to retain them, we simply use the `!` (logial NOT): 

`!(x >= 562000 && x <= 562500 && y >= 4819000 && y <= 4819500) ` or, simply: 

`!within_rect(562000, 4819500, 562500, 4819000) ` If we need to find all of the ground points within 150 m of (562000, 4819500), we could use: 

`class == 2 && dist_to_pt(562000, 4819500) <= 150.0 ` The following statement outputs all non-vegetation classed points in the upper-right quadrant: 

`!(class == 3 && class != 4 && class != 5) && x < min_x + (max_x - min_x) / 2.0 && y > max_y - (max_y - min_y) / 2.0 ` As demonstrated above, the `filter_lidar` tool provides an extremely flexible, powerful, and easy means for retaining and removing points from point clouds based on any of the common LiDAR point attributes. 

### See Also

 

`filter_lidar_classes`, `filter_lidar_scan_angles`, `modify_lidar`, `erase_polygon_from_lidar`, `clip_lidar_to_polygon`, `sort_lidar`, `lidar_eigenvalue_features` 

### Python API

```python
def filter_lidar(self, statement: str, input_lidar: Optional[Lidar]) -> Optional[Lidar]:
```


---

## Filter LiDAR By Percentile

**Function name:** `filter_lidar_by_percentile`


### Description

 

This tool can be used to extract a subset of points from an input LiDAR point cloud (`input_lidar`) that correspond  to a user-specified `percentile` of the points within the local neighbourhood. The algorithm works by overlaying a  grid of a specified size (`block_size`). The group of LiDAR points contained within each block in the superimposed  grid are identified and are sorted by elevation. The point with the elevation that corresponds most closely to the  specified percentile is then inserted into the output LiDAR point cloud. For example, if `percentile = 0.0`, the  lowest point within each block will be output, if `percentile = 100.0` the highest point will be output, and if `percentile = 50.0` the point that is nearest the median elevation will be output. Notice that the lower the number of points contained within a block, the more approximate the calculation will be. For example, if a block only contains three points, no single point occupies the 25th percentile. The equation that is used to identify the closest corresponding point (zero-based) from a list of `n` sorted by elevation values is: 

`point_num = ⌊percentile / 100.0 * (n - 1)⌉` 

Increasing the block size (default is 1.0 xy-units) will increase the average number of points within blocks,  allowing for a more accurate percentile calculation. 

Like many of the LiDAR functions, the input LiDAR point cloud (`input_lidar`) is optional. If an input LiDAR file  is not specified, the tool will search for all valid LiDAR (*.las, *.laz, *.zlidar) files contained within the current working directory. This feature can be very useful when you need to process a large number of LiDAR files contained within a directory. This batch processing mode enables the function to run in a more optimized parallel manner. When run in this batch mode, no output LiDAR object will be created. Instead the function will create an output file (saved to disc) with the same name as each input LiDAR file, but with the .tif extension. This can provide a very efficient means for processing extremely large LiDAR data sets. 

### See Also

 

`filter_lidar`, `lidar_block_minimum`, `lidar_block_maximum` 

### Python API

```python
def filter_lidar_by_percentile(self, input_lidar: Optional[Lidar],  percentile: float = 0.0, block_size: float = 1.0) -> Optional[Lidar]:
```


---

## Filter LiDAR By Reference Surface

**Function name:** `filter_lidar_by_reference_surface`


### Description

 

This tool can be used to extract a subset of points from an input LiDAR point cloud (`input_lidar`) that satisfy a `query` relation with a user-specified raster reference surface (`ref_surface`). For example, you may use this function to extract all of the points that are below (`query="<"` or `query="<="`) or above (`query=">"` or `query=">="`) a surface model. The default query mode is "within" (i.e. `query="within"`), which extracts all of the points that are within a specified absolute vertical distance (`threshold`) of the surface. Notice that the `threshold` parameter is ignored for query types other than "within". 

By default, the function will return a point cloud containing only the subset of points in the input dataset that satisfy the condition of the query. Setting the `classify` parameter to True modifies this behaviour such that the output point cloud will contain all of the points within the input dataset, but will have the classification value of the query-satifying points will be set to the `true_class_value` parameter (0-255) and points that do not satisfy the query  will be assigned the `false_class_value` (0-255). By setting the `preserve_classes` paramter to True, all points that do not satisfy the query will have unmodified class values from the input dataset. 

Unlike many of the LiDAR functions, this function does not have a batch mode and operates on single tiles only.  

### See Also

 

`filter_lidar` 

### Python API

```python
def filter_lidar_by_reference_surface(self, input_lidar: Lidar, ref_surface: Raster, query: str = "within", threshold: float = 0.0) -> Lidar:
```


---

## Filter LiDAR Classes

**Function name:** `filter_lidar_classes`


This tool can be used to remove points within a LAS LiDAR file that possess certain specified class values. The user must input the names of the input (`input`) and output (`output`) LAS files and the class values to be excluded (`exclude_cls`). Class values are specified by their numerical values, such that:  Classification ValueMeaning 0Created never classified 1Unclassified 2Ground 3Low Vegetation 4Medium Vegetation 5High Vegetation 6Building 7Low Point (noise) 8Reserved 9Water 10Rail 11Road Surface 12Reserved 13Wire – Guard (Shield) 14Wire – Conductor (Phase) 15Transmission Tower 16Wire-structure Connector (e.g. Insulator) 17Bridge Deck 18High noise   

Thus, to filter out low and high noise points from a point cloud, specify `exclude_cls='7,18'`. Class ranges may also be specified, e.g. `exclude_cls='3-5,7,18'`. Notice that usage of this tool assumes that the LAS file has underwent a comprehensive point classification, which not all point clouds have had. Use the `lidar_info` tool determine the distribution of various class values in your file. 

### See Also

 

`lidar_info` 

### Python API

```python
def filter_lidar_classes(self, input: Lidar, exclusion_classes: List[int]) -> Lidar:
```


---

## Filter LiDAR Noise

**Function name:** `filter_lidar_noise`


This function can be used to remove both low (class = 7) and high (class = 18) noise classed points within a LiDAR file.  The function is therefore equivalent to running the `filter_lidar_noise` function, specifying classes 7 and 18. Notice that usage of this tool assumes that the LAS file has underwent a comprehensive point classification, which not all point clouds have had. Use the `lidar_info` tool determine the distribution of various class values in your file. 

### See Also

 

`lidar_info`, `filter_lidar_classes` 

### Python API

```python
def filter_lidar_noise(self, input: Lidar) -> Lidar:
```


---

## Filter LiDAR Scan Angles

**Function name:** `filter_lidar_scan_angles`


### Python API

```python
def filter_lidar_scan_angles(self, in_lidar: Lidar, threshold: int) -> Lidar:
```


---

## Height Above Ground

**Function name:** `height_above_ground`


This tool normalizes an input LiDAR point cloud (`input`) such that point z-values in the output LAS file (`output`) are converted from elevations to heights above the ground, specifically the height above the nearest ground-classified point. The input LAS file must have ground-classified points, otherwise the tool will return an error. The `lidar_tophat_transform` tool can be used to perform the normalization if a ground classification is lacking. 

### See Also

 

`lidar_tophat_transform` 

### Python API

```python
def height_above_ground(self, input: Lidar) -> Lidar:
```


---

## Improved Ground Point Filter

**Function name:** `improved_ground_point_filter`


This function provides a faster alternative to the `lidar_ground_point_filter` algorithm, provided in the free version of Whitebox Workflows, for the extraction of ground points from within a LiDAR point cloud. The algorithm works by placing a grid overtop of the point cloud of a specified resolution (`block_size`, in xy-units) and identifying the subset of lidar points associated with the lowest position in each block. A raster surface is then created by  TINing these points. The surface is further processed by removing any off-terrain objects (OTOs), including buildings smaller than the `max_building_size` parameter (xy-units). Removing OTOs also requires the user to specify the value of a `slope_threshold`, in degrees. Finally, the algorithm then extracts all of the points in the input LiDAR point cloud  (`input`) that are within a specified absolute vertical distance (`elev_threshold`) of this surface model. 

Conceptually, this method of ground-point filtering is somewhat similar in concept to the cloth-simulation approach of  Zhang et al. (2016). The difference is that the cloth is first fitted to the minimum surface with infinite flexibility  and then the rigidity of the cloth is subsequently increased, via the identification and removal of OTOs from the minimal  surface. The `slope_threshold` parameter effectively controls the eventual rigidity of the fitted surface. 

By default, the tool will return a point cloud containing only the subset of points in the input dataset that coincide with the idenfitied ground points. Setting the `classify` parameter to True modifies this behaviour such that the output point cloud will contain all of the points within the input dataset, but will have the classification value of identified ground points set to '2' (i.e., the ground class value) and all other points will be set to '1' (i.e., the unclassified class value). By setting the `preserve_classes` paramter to True, all non-ground points in the output cloud will have the same classes as the corresponding point class values in the input dataset. 

Compared with the `lidar_ground_point_filter` algorithm, the `improved_ground_point_filter` algorithm is generally far faster and is able to more effectively remove points associated with larger buildings. Removing large buildings from point clouds with the  `lidar_ground_point_filter` algorithm requires use of very large search distances, which slows the operation considerably. 

As a comparison of the two available methods, one test tile of LiDAR containing numerous large buildings and abundant  vegetation required 600.5 seconds to process on the test system using the `lidar_ground_point_filter` algorithm  (removing all but the largest buildings) and 9.8 seconds to process using the `improved_ground_point_filter` algorithm  (with complete building removal), i.e., 61x faster. 

The original test LiDAR tile, containing abundant vegetation and buildings: 

 

The result of applying the `lidar_ground_point_filter` function, with a search radius of 25 m and max inter-point slope of  15 degrees: 

 

The result of applying the `improved_ground_point_filter` method, with `block_size` = 1.0 m, `max_building_size` = 150.0 m,  `slope_threshold` = 15.0 degrees, and `elev_threshold` = 0.15 m: 

 

### References:

 

Zhang, W., Qi, J., Wan, P., Wang, H., Xie, D., Wang, X., & Yan, G. (2016). An easy-to-use airborne LiDAR data filtering  method based on cloth simulation. Remote sensing, 8(6), 501. 

### See Also:

 

`lidar_ground_point_filter` 

### Python API

```python
def improved_ground_point_filter(self, input: Lidar, block_size = 1.0, max_building_size = 150.0, slope_threshold = 15.0, elev_threshold = 0.15, , classify = False, preserve_classes = False) -> Lidar:
```


---

## Individual Tree Segmentation

**Function name:** `individual_tree_segmentation`


Stable

Segment individual tree crowns from LiDAR using adaptive bandwidth and vegetation filtering.

### Parameters

NameDescriptionRequiredDefault
`input`Input LiDAR path or typed LiDAR object.Required—
`only_use_veg`If true, process only vegetation classes (default true).Optional—
`veg_classes`Vegetation classes as comma-delimited text or integer array (default '3,4,5').Optional—
`min_height`Minimum point height for segmentation (default 2.0).Optional—
`max_height`Optional maximum point height.Optional—
`bandwidth_min`Minimum horizontal bandwidth (default 1.0).Optional—
`bandwidth_max`Maximum horizontal bandwidth (default 6.0).Optional—
`adaptive_bandwidth`Estimate per-seed horizontal bandwidth from local crown geometry (default true).Optional—
`adaptive_neighbors`Neighbour count used for adaptive local density scale (default 24).Optional—
`adaptive_sector_count`Number of angular sectors for local crown-radius estimation (default 8).Optional—
`grid_acceleration`Use MeanShift++-style grid approximation for faster mode updates (default false).Optional—
`grid_cell_size`Grid cell size for accelerated mode updates (default 0.5).Optional—
`grid_refine_exact`Run short exact-neighbour refinement after grid acceleration (default false).Optional—
`grid_refine_iterations`Exact refinement iteration cap after grid mode updates (default 2).Optional—
`tile_size`Optional tile size for seed scheduling; Optional—
`tile_overlap`Tile overlap width for tiled seed scheduling (default 0.0).Optional—
`vertical_bandwidth`Vertical kernel bandwidth (default 5.0).Optional—
`max_iterations`Maximum mean-shift iterations per seed (default 30).Optional—
`convergence_tol`Convergence tolerance for shift magnitude (default 0.05).Optional—
`min_cluster_points`Minimum points per retained tree cluster (default 50).Optional—
`mode_merge_dist`Distance threshold for merging converged modes (default 0.8).Optional—
`threads`Thread count override (0 uses default Rayon pool).Optional—
`simd`Enable SIMD-assisted arithmetic in weighting loops (default true).Optional—
`output_id_mode`Output segment id encoding: rgb/user_data/point_source_id or combinations like rgb+user_data.Optional—
`output_sidecar_csv`If true, write point_index,segment_id CSV beside lidar output.Optional—
`seed`Deterministic seed for colour mapping (default 1).Optional—
`output`Optional output LiDAR path.Optional—


---

## LiDAR Classify Subset

**Function name:** `lidar_classify_subset`


This tool classifies points within a user-specified LiDAR point cloud (`base`) that correspond with points in a subset cloud (`subset`). The subset point cloud may have been derived by filtering the original point cloud. The user must specify the names of the two input LAS files (i.e. the full and subset clouds) and the class value (`subset_class`) to assign the matching points. This class value will be assigned to points in the base cloud, overwriting their input class values in the output LAS file (`output`). Class values should be numerical (integer valued) and should follow the LAS specifications below:  Classification ValueMeaning 0Created never classified 1Unclassified 2Ground 3Low Vegetation 4Medium Vegetation 5High Vegetation 6Building 7Low Point (noise) 8Reserved 9Water 10Rail 11Road Surface 12Reserved 13Wire – Guard (Shield) 14Wire – Conductor (Phase) 15Transmission Tower 16Wire-structure Connector (e.g. Insulator) 17Bridge Deck 18High noise   

The user may optionally specify a class value to be assigned to non-subset (i.e. non-matching) points (`nonsubset_class`) in the base file. If this parameter is not specified, output non-sutset points will have the same class value as the base file. 

### Python API

```python
def lidar_classify_subset(self, base_lidar: Lidar, subset_lidar: Lidar, subset_class_value: int, nonsubset_class_value: int) -> Lidar:
```


---

## LiDAR Elevation Slice

**Function name:** `lidar_elevation_slice`


This tool can be used to either extract or classify the elevation values (z) of LiDAR points within a specified elevation range (slice). In addition to the names of the input and output LiDAR files (`input` and `output`), the user must specify the lower (`minz`) and upper (`maxz`) bounds of the elevation range. By default, the tool will only output points within the elevation slice, filtering out all points lying outside of this range. If the `class` parameter is used, the tool will operate by assigning a class value (`inclassval`) to the classification bit of points within the slice and another class value (`outclassval`) to those points falling outside the range. 

### See Also

 

`lidar_remove_outliers`, `lidar_classify_subset` 

### Python API

```python
def lidar_elevation_slice(self, input: Lidar, minz: float = float('-inf'), maxz: float = float('inf'), classify: bool = False, in_class_value: int = 2, out_class_value: int = 1) -> Lidar:
```


---

## LiDAR Remove Outliers

**Function name:** `lidar_remove_outliers`


This tool will filter out points from a LiDAR point cloud if the absolute elevation difference between a point and the averge elevation of its neighbourhood, calculated without the point, exceeds a threshold (elev_diff). 

### Python API

```python
def lidar_remove_outliers(self, input: Lidar, search_radius: float = 2.0, elev_diff: float = 50.0, use_median: bool = False, classify: bool = False) -> Lidar:
```


---

## LiDAR Segmentation

**Function name:** `lidar_segmentation`


This tool can be used to segment a LiDAR point cloud based on differences in the orientation of fitted planar surfaces and point proximity. The algorithm begins by attempting to fit planar surfaces to all of the points within a user-specified radius (`radius`) of each point in the LiDAR data set. The planar equation is stored for each point for which a suitable planar model can be fit. A region-growing algorithm is then used to assign nearby points with similar planar models. Similarity is based on a maximum allowable angular difference (in degrees) between the two neighbouring points' plane normal vectors (`norm_diff`). The `norm_diff` parameter can therefore be thought of as a way of specifying the magnitude of edges mapped by the region-growing algorithm. By setting this value appropriately, it is possible to segment each facet of a building's roof. Segment edges for planar points may also be determined by a maximum allowable height difference (`maxzdiff`) between neighbouring points on the same plane. Points for which no suitable planar model can be fit are assigned to 'volume' (non-planar) segments (e.g. vegetation points) using a region-growing method that connects neighbouring points based solely on proximity (i.e. all volume points within `radius` distance are considered to belong to the same segment). 

The resulting point cloud will have both planar segments (largely ground surfaces and building roofs and walls) and volume segments (largely vegetation). Each segment is assigned a random red-green-blue (RGB) colour in the output LAS file. The largest segment in any airborne LiDAR dataset will usually belong to the ground surface. This largest segment will always be assigned a dark-green RGB of (25, 120, 0) by the tool. 

This tool uses the `random sample consensus (RANSAC)` method to identify points within a LiDAR point cloud that belong to planar surfaces. RANSAC is a common method used in the field of computer vision to identify a subset of inlier points in a noisy data set containing abundant outlier points. Because LiDAR point clouds often contain vegetation points that do not form planar surfaces, this tool can be used to largely strip vegetation points from the point cloud, leaving behind the ground returns, buildings, and other points belonging to planar surfaces. If the `classify` flag is used, non-planar points will not be removed but rather will be assigned a different class (1) than the planar points (0). 

The algorithm selects a random sample, of a specified size (`num_samples`) of the points from within the neighbourhood (`radius`) surrounding each LiDAR point. The sample is then used to parameterize a planar best-fit model. The distance between each neighbouring point and the plane is then evaluated; inliers are those neighbouring points within a user-specified distance threshold (`threshold`). Models with at least a minimum number of inlier points (`model_size`) are then accepted. This process of selecting models is iterated a number of user-specified times (`num_iter`). 

One of the challenges with identifying planar surfaces in LiDAR point clouds is that these data are usually collected along scan lines. Therefore, each scan line can potentially yield a vertical planar surface, which is one reason that some vegetation points may be assigned to planes during the RANSAC plane-fitting method. To cope with this problem, the tool allows the user to specify a maximum planar slope (`max_slope`) parameter. Planes that have slopes greater than this threshold are rejected by the algorithm. This has the side-effect of removing building walls however. 

 

### References

 

Fischler MA and Bolles RC. 1981. Random sample consensus: a paradigm for model fitting with applications to image analysis and automated cartography. Commun. ACM, 24(6):381–395. 

### See Also

 

`lidar_ransac_planes`, `lidar_ground_point_filter` 

### Python API

```python
def lidar_segmentation(self, in_lidar: Lidar, search_radius: float = 2.0, num_iterations: int = 50, num_samples: int = 10, inlier_threshold: float = 0.15, acceptable_model_size: int = 30, max_planar_slope: float = 75.0, norm_diff_threshold: float = 2.0, max_z_diff: float = 1.0, classes: bool = False, ground: bool = False) -> Lidar:
```


---

## LiDAR Segmentation Based Filter

**Function name:** `lidar_segmentation_based_filter`


### Python API

```python
def lidar_segmentation_based_filter(self, in_lidar: Lidar, search_radius: float = 5.0, norm_diff_threshold: float = 2.0, max_z_diff: float = 1.0, classify_points: bool = False) -> Lidar:
```


---

## Modify LiDAR

**Function name:** `modify_lidar`


### Description

 

The ModifyLidar tool can be used to alter the properties of points within a LiDAR point cloud. The user provides a statement (`statement`) containing one or more expressions, separated by semicolons (;). The expressions are evaluated for each point within the input LiDAR file (`input`). Expressions assign altered values to the properties of points in the output file (`output`), based on any mathematically defined expression that may include the various properties of individual points (e.g. coordinates, intensity, return attributes, etc) or some file-level properties (e.g. min/max coordinates). As a basic example, the following statement: 

`x = x + 1000.0 ` could be used to translate the point cloud 1000 x-units (note, the increment operator could be used as a simpler equivalent, `x += 1000.0`). 

Note that if the user does not specify the optional input LiDAR file, the tool will search for all valid LiDAR (*.las, *.laz, *.zlidar) files contained within the current working directory. This feature can be useful for processing a large number of LiDAR files in batch mode. When this batch mode is applied, the output file names will be the same as the input file names but with a '_modified' suffix added to the end. 

Expressions may contain any of the following point-level or file-level variables:  Variable NameDescriptionType **Point-level properties** xThe point x coordinatefloat yThe point y coordinatefloat zThe point z coordinatefloat xyAn x-y coordinate tuple, (x, y)(float, float) xyzAn x-y-z coordinate tuple, (x, y, z)(float, float, float) intensityThe point intensity valueint retThe point return numberint nretThe point number of returnsint is_onlyTrue if the point is an only return (i.e. ret == nret == 1), otherwise falseBoolean is_multipleTrue if the point is a multiple return (i.e. nret > 1), otherwise falseBoolean is_earlyTrue if the point is an early return (i.e. ret == 1), otherwise falseBoolean is_intermediateTrue if the point is an intermediate return (i.e. ret > 1 && ret Boolean is_lateTrue if the point is a late return (i.e. ret == nret), otherwise falseBoolean is_firstTrue if the point is a first return (i.e. ret == 1 && nret > 1), otherwise falseBoolean is_lastTrue if the point is a last return (i.e. ret == nret && nret > 1), otherwise falseBoolean classThe class value in numeric form, e.g. 0 = Never classified, 1 = Unclassified, 2 = Ground, etc.int is_noiseTrue if the point is classified noise (i.e. class == 7class == 18), otherwise falseBoolean is_syntheticTrue if the point is synthetic, otherwise falseBoolean is_keypointTrue if the point is a keypoint, otherwise falseBoolean is_withheldTrue if the point is withheld, otherwise falseBoolean is_overlapTrue if the point is an overlap point, otherwise falseBoolean scan_angleThe point scan angleint scan_directionTrue if the scanner is moving from the left towards the right, otherwise falseBoolean is_flightline_edgeTrue if the point is situated along the filightline edge, otherwise falseBoolean user_dataThe point user dataint point_source_idThe point source IDint scanner_channelThe point scanner channelint timeThe point GPS time, if it exists, otherwise 0float rgbA red-green-blue tuple (r, g, b) if it exists, otherwise (0,0,0)(int, int, int) nirThe point near infrared value, if it exists, otherwise 0int pt_numThe point number within the input fileint **File-level properties (invariant)** n_ptsThe number of points within the fileint min_xThe file minimum x valuefloat mid_xThe file mid-point x valuefloat max_xThe file maximum x valuefloat min_yThe file minimum y valuefloat mid_yThe file mid-point y valuefloat max_yThe file maximum y valuefloat min_zThe file minimum z valuefloat mid_zThe file mid-point z valuefloat max_zThe file maximum z valuefloat x_scale_factorThe file x scale factorfloat y_scale_factorThe file y scale factorfloat z_scale_factorThe file z scale factorfloat x_offsetThe file x offsetfloat y_offsetThe file y offsetfloat z_offsetThe file z offsetfloat   

Most of the point-level properties above are modifiable, however some are not. The complete list of modifiable point attributes include, x, y, z, xy, xyz, intensity, ret, nret, class, user_data, point_source_id, scanner_channel, scan_angle, time, rgb, nir, is_synthetic, is_keypoint, is_withheld, and is_overlap. The immutable properties include is_only, is_multiple, is_early, is_intermediate, is_late, is_first, is_last, is_noise, and pt_num. Of the file-level properties, the modifiable properties include the x_scale_factor, y_scale_factor, z_scale_factor, x_offset, y_offset, and z_offset. 

In addition to the point properties defined above, if the user applies the `lidar_eigenvalue_features` tool on the input LiDAR file, the `modify_lidar` tool will automatically read in the additional *.eigen file, which include the eigenvalue-based point neighbourhood measures, such as `lambda1`, `lambda2`, `lambda3`, `linearity`, `planarity`, `sphericity`, `omnivariance`, `eigentropy`, `slope`, and `residual`. See the `lidar_eigenvalue_features` documentation for details on each of these metrics describing the structure and distribution of points within the neighbourhood surrounding each point in the LiDAR file. 

Expressions may use any of the standard mathematical operators, +, -, *, /, % (modulo), ^ (exponentiation), comparison operators, <, >, <=, >=, == (equality), != (inequality), and logical operators, && (Boolean AND),  (Boolean OR). Expressions must evaluate to an assignment operation, where the variable that is assigned to must be a modifiable point-level property (see table above). That is, expressions should take the form  

`pt_variable = ...`. Other assignment operators are also possible (at least for numeric non-tuple properties), such as the increment (=+) operator (e.g. `x += 1000.0`) and the decrement (-=) operator (e.g. `y -= 1000.0`). Expressions may use a number of built-in mathematical functions, including:  Function NameDescriptionExample ifPerforms an if(CONDITION, TRUE, FALSE) operation, return either the value of TRUE or FALSE depending on CONDITION`ret = if(ret==0, 1, ret)` absReturns the absolute value of the argument`value = abs(x - mid_x)` minReturns the minimum of the arguments`value = min(x, y, z)` maxReturns the maximum of the arguments`value = max(x, y, z)` floorReturns the largest integer less than or equal to a number`x = floor(x)` roundReturns the nearest integer to a number. Rounds half-way cases away from 0.0`x = round(x)` ceilReturns the smallest integer greater than or equal to a number`x = ceil(x)` clampForces a value to fall within a specified range, defined by a minimum and maximum`z = clamp(min_z+10.0, z, max_z-20.0)` intReturns the integer equivalent of a number`intensity = int(z)` floatReturns the float equivalent of a number`z = float(intensity)` to_radiansConverts a number in degrees to radians`val = to_radians(scan_angle)` to_degreesConverts a number in radians to degrees`scan_angle = int(to_degrees(val))` distReturns the distance between two points defined by two n-length tuples`d = dist(xy, (mid_x, mid_y))` or `d = dist(xyz, (mid_x, mid_y, mid_z))` rotate_ptRotates an x-y point by a certain angle, in degrees`xy = rotate_pt(xy, 45.0)` or `orig_pt = (1000.0, 1000.0); xy = rotate_pt(xy, 45.0, orig_pt)` math::lnReturns the natural logarithm of the number`z = math::ln(z)` math::logReturns the logarithm of the number with respect to an arbitrary base`z = math::log(z, 10)` math::log2Returns the base 2 logarithm of the number`z = math::log2(z)` math::log10Returns the base 10 logarithm of the number`z = math::log10(z)` math::expReturns e^(number), (the exponential function)`z = math::exp(z)` math::powRaises a number to the power of the other number`z = math::pow(z, 2.0)` math::sqrtReturns the square root of a number. Returns NaN for a negative number`z = math::sqrt(z, 2.0)` math::cosComputes the cosine of a number (in radians)`z = math::cos(to_radians(z))` math::sinComputes the sine of a number (in radians)`z = math::sin(to_radians(z))` math::tanComputes the tangent of a number (in radians)`z = math::tan(to_radians(z))` math::acosComputes the arccosine of a number. The return value is in radians in the range [0, pi] or NaN if the number is outside the range [-1, 1]`z = math::acos(z)` math::asinComputes the arcsine of a number. The return value is in radians in the range [0, pi] or NaN if the number is outside the range [-1, 1]`z = math::asin(z)` math::atanComputes the arctangent of a number. The return value is in radians in the range [0, pi] or NaN if the number is outside the range [-1, 1]`z = math::atan(z)` randReturns a random value between 0 and 1, with an optional seed value`rgb = (int(255.0 * rand()), int(255.0 * rand()), int(255.0 * rand()))` helmert_transformationPerforms a Helmert transformation on a point using a 7-parameter transform`xyz = helmert_transformation(xyz, −446.448, 125.157, −542.06, 20.4894, −0.1502, −0.247, −0.8421 )`   

The hyperbolic trigonometric functions are also available for use in expression building, as is `math::atan2` and the mathematical constants `pi` and `e`. 

You may use `if` operations within statements to implement a conditional modification of point properties. For example, the following expression demonstrates how you could modify a point's RGB colour based on its classification, assign ground points (class 2) in the output file a green colour: 

`rgb = if(class==2, (0,255,0), rgb) ` To colour all points within 50 m of the tile mid-point red and all other points blue: 

`rgb = if(dist(xy, (mid_x, mid_y))<50.0, (255,0,0), (0,0,255)) ` `if` operations may also be nested to create more complex compound conditional point modification. For example, in the following statement, we assign first-return points red (255,0,0) and last-return points green (0,255,0) colours and white (255,255,255) to all other points (intermediate-returns and only-returns): 

`rgb = if(is_first, (255,0,0), if(is_last, (0,255,0), (255,255,255))) ` Here we use an `if` expression to re-classify points above an elevation of 1000.0 as high noise (class 18): 

`class = if(z>1000.0, 18, class) ` Expressions may be strung together within statements using semicolons (;), with each expression being evaluated individually. When this is the case, at least one of the expressions must assign a value to one of the variant point properties (see table above). The following statement demonstrates multi-expression statements, in this case to swap the x and y coordinates in a LiDAR file: 

`new_var = x; x = y; y = new_var ` The `rand` function, used with the seeding option, can be useful when assigning colours to points based on common point properties. For example, to assign a point a random RGB colour based on its `point_source_id` (Note, for many point clouds, this operation will assign each flightline a unique colour; if flightline information is not stored in the file's `point_source_id` attribute, one could use the `recover_flightline_info` tool to calculate this data.): 

`rgb=(int(255 * rand(point_source_id)), int(255 * rand(point_source_id+1)), int(255 * rand(point_source_id+2))) ` This expression-based approach to modifying point properties provides a great deal of flexibility and power to the processing of LiDAR point cloud data sets. 

### See Also

 

`filter_lidar`, `sort_lidar`, `lidar_eigenvalue_features` 

### Python API

```python
def modify_lidar(self, statement: str, input_lidar: Optional[Lidar]) -> Optional[Lidar]:
```


---

## Normalize LiDAR

**Function name:** `normalize_lidar`


This tool can be used to normalize a LiDAR point cloud. A normalized point cloud is one for which the point z-values represent height above the ground surface rather than raw elevation values. Thus, a point that falls on the ground  surface will have a z-value of zero and vegetation points, and points associated with other off-terrain objects,  have positive, non-zero z-values. Point cloud normalization is an essential pre-processing method for many forms of LiDAR data analysis, including the characterization of many forestry related metrics and individual tree mapping  (`IndividualTreeDetection`).  

This tool works by measuring the elevation difference of each point in an input LiDAR file (`input`) and the elevation of an input raster digital terrain model (`dtm`). A DTM is a bare-earth digital elevation model. Typically, the input DTM is creating using the same input LiDAR data by interpolating the ground surface using only ground-classified points. If the LiDAR point cloud does not contain ground-point classifications, you may wish to apply the `LidarGroundPointFilter`  or `ClassifyLidar`tools before interpolating the DTM. While ground-point classification works well to identify the ground  surface beneath vegetation cover, building points are sometimes left  It may also be necessary to remove other off-terrain  objects like buildings. The `RemoveOffTerrainObjects` tool can be useful for this purpose, creating a final bare-earth DTM. This tool outputs a normalized LiDAR point cloud (`output`). If the `no_negatives` parameter is True, any points that fall beneath the surface elevation defined by the DTM, will have their z-value set to zero. 

Note that the `LidarTophatTransform` tool similarly can be used to produce a type of normalized point cloud, although it  does not require an input raster DTM. Rather, it attempts to model the ground surface within the point cloud by identifying the lowest points within local neighbourhoods surrounding each point in the cloud. While this approach can produce satisfactory results in some cases, the `NormalizeLidar` tool likely works better under more rugged topography and in areas with  extensive building coverage, and provides greater control over the definition of the ground surface. 

### See Also

 

`lidar_tophat_transform`, `individual_tree_detection`, `lidar_ground_point_filter`, `classify_lidar` 

### Python API

```python
def normalize_lidar(self, input_lidar: Lidar, dtm: Raster) -> Lidar:
```


---

## Remove Duplicates

**Function name:** `remove_duplicates`


This tool removes duplicate points from a LiDAR data set. Duplicates are determined by their x, y, and optionally (`include_z`) z coordinates. 

### See Also

 

`eliminate_coincident_points` 

### Python API

```python
def remove_duplicates(self, input: Lidar, include_z: bool = False) -> Lidar:
```
