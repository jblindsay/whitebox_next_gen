# Analysis and Metrics


---

## Colourize Based On Class

**Function name:** `colourize_based_on_class`


### Description

 

This tools sets the RGB colour values of an input LiDAR point cloud (`input`) based on the point classifications. Rendering a point cloud in this way can aid with the determination of point classification accuracy, by allowing you to determine if there are certain areas within a LiDAR tile, or certain classes, that are problematic during the point classification process. 

By default, the tool renders buildings in red (see table below). However, the tool also provides the option to render each building in a unique colour (`use_unique_clrs_for_buildings`), providing a visually stunning LiDAR-based map of built-up areas. When this option is selected, the user must also specify the `radius` parameter, which determines the search distance used during the building segmentation operation. The `radius` parameter is optional, and if unspecified (when the `use_unique_clrs_for_buildings` flag is used), a value of 2.0 will be used. 

 

The specific colours used to render each point class can optionally be set by the user with the `clr_str` parameter. The value of this parameter may list specific class values (0-18) and corresponding colour values in either a red-green-blue (RGB) colour triplet form (i.e. `(r, g, b)`), or or a hex-colour, of either form `#e6d6aa` or `0xe6d6aa` (note the `#` and `0x` prefixes used to indicate hexadecimal numbers; also either lowercase or capital letter values are acceptable). The following is an example of the a valid `clr_str` that sets the ground (class 2) and high vegetation (class 5) colours used for rendering: 

`2: (184, 167, 108); 5: #9ab86c` 

Notice that 1) each class is separated by a semicolon (';'), 2) class values and colour values are separated by colons (':'), and 3) either RGB and hex-colour forms are valid. 

If a `clr_str` parameter is not provided, the tool will use the default colours used for each class (see table below). 

Class values are assumed to follow the class designations listed in the LAS specification:  Classification ValueMeaningDefault Colour 0Created never classified 1Unclassified 2Ground 3Low Vegetation 4Medium Vegetation 5High Vegetation 6Building 7Low Point (noise) 8Reserved 9Water 10Rail 11Road Surface 12Reserved 13Wire – Guard (Shield) 14Wire – Conductor (Phase) 15Transmission Tower 16Wire-structure Connector (e.g. Insulator) 17Bridge Deck 18High noise   

The point RGB colour values can be blended with the intensity data to create a particularly effective visualization, further enhancing the visual interpretation of point return properties. The `intensity_blending` parameter value, which must range from 0% (no intensity blending) to 100% (all intensity), is used to set the degree of intensity/RGB blending. 

Because the output file contains RGB colour data, it is possible that it will be larger than the input file. If the input file does contain valid RGB data, the output will be similarly sized, but the input colour data will be replaced in the output file with the point-return colours. 

The output file can be visualized using any point cloud renderer capable of displaying point RGB information. We recommend the `plas.io` LiDAR renderer but many similar open-source options exist. 

### See Also

 

`colourize_based_on_point_returns`, `lidar_colourize` 

### Python API

```python
def colourize_based_on_class(self, input_lidar: Optional[Lidar], intensity_blending_amount: float = 50.0, clr_str: str = "", use_unique_clrs_for_buildings: bool = False, search_radius: float = 2.0) -> Optional[Lidar]:
```


---

## Colourize Based On Point Returns

**Function name:** `colourize_based_on_point_returns`


### Description

 

This tool sets the RGB colour values of a LiDAR point cloud (`input`) based on the point returns. It specifically renders only-return, first-return, intermediate-return, and last-return points in different colours, storing these data in the RGB colour data of the output LiDAR file (`output`). Colourizing the points in a LiDAR point cloud based on return properties can aid with the visual inspection of point distributions, and therefore, the quality assurance/quality control (QA/QC) of LiDAR data tiles. For example, this visualization process can help to determine if there are areas of vegetation where there is insufficient coverage of ground points, perhaps due to acquisition of the data during leaf-on conditions. There is often an assumption in LiDAR data processing that the ground surface can be modelled using a subset of the only-return and last-return points (beige and blue in the image below). However, under heavy forest cover, and in particular if the data were collected during leaf-on conditions or if there is significant coverage of conifer trees, the only-return and last-return points may be poor approximations of the ground surface. This tool can help to determine the extent to which this is the case for a particular data set. 

 

The specific colours used to render each return type can be set by the user with the `only`, `first`, `intermediate`, and `last` parameters. Each parameter takes either a red-green-blue (RGB) colour triplet, of the form `(r,g,b)`, or a hex-colour, of either form `#e6d6aa` or `0xe6d6aa` (note the `#` and `0x` prefixes used to indicate hexadecimal numbers; also either lowercase or capital letter values are acceptable). 

The point RGB colour values can be blended with the intensity data to create a particularly effective visualization, further enhancing the visual interpretation of point return properties. The `intensity_blending` parameter value, which must range from 0% (no intensity blending) to 100% (all intensity), is used to set the degree of intensity/RGB blending. 

Because the output file contains RGB colour data, it is possible that it will be larger than the input file. If the input file does contain valid RGB data, the output will be similarly sized, but the input colour data will be replaced in the output file with the point-return colours. 

The output file can be visualized using any point cloud renderer capable of displaying point RGB information. We recommend the `plas.io` LiDAR renderer but many similar open-source options exist. 

This tool is a convenience function and can alternatively be achieved using the `modify_lidar` tool with the statement: 

`rgb=if(is_only, (230,214,170), if(is_last, (0,0,255), if(is_first, (0,255,0), (255,0,255))))` 

The `colourize_based_on_point_returns` tool is however significantly faster for this operation than the `modify_lidar` tool because the expression above must be executed dynamically for each point. 

### See Also

 

`modify_lidar`, `lidar_colourize` 

### Python API

```python
def colourize_based_on_point_returns(self, input_lidar: Optional[Lidar], intensity_blending_amount: float = 50.0, only_ret_colour: str = "(230,214,170)", first_ret_colour:str = "(0,140,0)", intermediate_ret_colour: str = "(255,0,255)", last_ret_colour: str = "(0,0,255)") -> Optional[Lidar]:
```


---

## Find Flightline Edge Points

**Function name:** `find_flightline_edge_points`


### Python API

```python
def find_flightline_edge_points(self, in_lidar: Lidar) -> Lidar:
```


---

## Individual Tree Detection

**Function name:** `individual_tree_detection`


This tool can be used to identify points in a LiDAR point cloud that are associated with the tops of individual trees. The tool takes a LiDAR point cloud as an input (`input_lidar`) and it is best if the input file has been normalized using the `lidar_tophat_transform` function, such that points record height above the ground surface. Note that the `input_lidar`  parameter is optional and if left unspecified the tool will search for all valid LiDAR (*.las, *.laz, *.zlidar) files  contained within the current working directory. This 'batch mode' operation is common among many of the LiDAR processing  tools. Output vectors are saved to disc automatically for each processed LiDAR file when operating in batch mode and the function returns `None`. When an individual `input_lidar` Lidar object is specified, the tool will return a `Vector` object, containing the tree top points. 

The tool will evaluate the points within a local neighbourhood around each point in the input point cloud and determine if it is the highest point within the neighbourhood. If a point is the highest local point, it will be entered into the output vector file. The neighbourhood size can vary, with higher canopy positions generally associated with larger neighbourhoods. The user specifies the `min_search_radius` and `min_height` parameters, which default to 1 m and 0 m  respectively. If the `min_height` parameter is greater than zero, all points that are less than this value above the  ground (assuming the input point cloud measures this height parameter) are ignored, which can be a useful mechanism for removing shorter trees and other vegetation from the analysis. If the user specifies the `max_search_radius` and `max_height` parameters, the search radius will be determined by linearly interpolation based on point height and the min/max search radius and height parameter values. Points that are above the `max_height` parameter will be processed with search neighbourhoods sized `max_search_radius`. If the max radius and height parameters are unspecified, they are set to the same values as the minimum radius and height parameters, i.e., the neighbourhood size does not increase with canopy height. 

If the point cloud contains point classifications, it may be useful to exclude all non-vegetation points. To do this simply set the `only_use_veg` parameter to True. This parameter should only be set to True when you know that the input file contains point classifications, otherwise the tool may generate an empty output vector file. 

### See Also

 

`lidar_tophat_transform` 

### Python API

```python
def individual_tree_detection(self, input_lidar: Lidar, min_search_radius: float = 1.0, min_height: float = 0.0, max_search_radius: Optional[float] = None, max_height: Optional[float] = None, only_use_veg = False) -> Optional[Vector]:
```


---

## LiDAR Eigenvalue Features

**Function name:** `lidar_eigenvalue_features`


### Description

 

This tool can be used to measure eigenvalue-based features that describe the characteristics of the local neighbourhood surrounding each point in an input LiDAR file (`input`). These features can then be used in point classification applications, or as the basis for point filtering (`filter_lidar`) or modifying point properties (`modify_lidar`). 

The algorithm begins by using the x, y, z coordinates of the points within a local spherical neighbourhood to calculate a `covariance matrix`. The three `eigenvalues` λ1, λ2, λ3 are then derived from the covariance matrix decomposition such that λ1 > λ2 > λ3. The eigenvalues are then used to describe the extent to which the neighbouring points can be characterized by a linear, planar, or volumetric distribution, by calculating the following three features: 

linearity = (λ1 - λ2) / λ1 

planarity = (λ2 - λ3) / λ1 

sphericity = λ3 / λ1 

In the case of a neighbourhood containing a 1-dimensional line, the first of the three components will possess most of data variance, with very little contained within λ2 and λ3, and linearity will be nearly equal to 1.0. If the local neighbourhood contains a 2-dimensional plane, the first two components will possess most of the variance, with little variance within λ3, and planarity will be nearly equal to 1.0. Lastly, in the case of a 3-dimensional, random  volumetric point distribution, each of the three components will be nearly equal in magnitude and sphericity will be nearly equal to 1.0. 

Researchers in the field of LiDAR point classification also frequently define two additional eigenvalue-based features, the omnivariance (low values correspond to planar and linear regions and higher values occur for areas with a volumetric point distribution, such as vegetation), and the eigentropy, which is related to the Shannon entropy and is a measure of the unpredictability of the distribution of points in the neighbourhood: 

omnivariance = (λ1 &#8901; λ2 &#8901; λ3)1/3 

eigentropy = -*e*1 &#8901; ln*e*1 - *e*2 &#8901; ln*e*2 - *e*3 &#8901; ln*e*3 

where *e*1, *e*2, and *e*3 are the normalized eigenvalues. 

In addition to the eigenvalues, the eigendecomposition of the symmetric covariance matrix also yields the three eigenvectors, which describe the transformation coefficients of the principal components. The first two eigenvectors represent the `basis of the plane` resulting from the orthogonal regression analysis, while the third eigenvector represents the plane normal. From this normal, it is possible to calculate the slope of the plane, as well as the orthogonal distance between each point and the neighbourhood fitted plane, i.e. the point residual. 

This tool outputs a binary file (*.eigen; `output`) that contains a total of 10 features for each point in the input file, including the `point_num` (for reference), `lambda1`, `lambda2`, `lambda3`, `linearity`, `planarity`, `sphericity`, `omnivariance`, `eigentropy`, `slope`, and `residual`. Users should bear in mind that most of these features describe the properties of the distribution of points within a spherical neighbourhood surrounding each point in the input file, rather than a characteristic of the point itself. The only one of the ten features that is a point property is the `residual`. Points for which the `planarity` value is high and the `residual` value is low may be assumed to be part of the plane that dominate the structure of their neighbourhoods. In addition to the binary data *.eigen file, the tool will also output a `sidecar file`, with a *.eigen.json extension, which describes the structure of the raw binary data file. 

Local neighbourhoods are spherical in shape and the size of each neighbourhood is characterized by the `num_neighbours` and `radius` parameters. If the optional `num_neighbours` parameter is specified, the size of the neighbourhood will vary by point, increasing or decreasing to encompass the specified number of neighbours (notice that this value does not include the point itself). If the optional `radius` parameter is specified in addition to a number of neighbours, the specified radius value will serve as a upper-bound and neighbouring points that are beyond this radial distance to the centre point will be excluded. If a radius search distance is specified but the `num_neighbours` parameter is not, then a constant search distance will be used for each point in the input file, resulting in varying number of points within local neighbourhoods, depending on local point densities. If point density varies significantly in the input file, then use of the `num_neighbours` parameter may be advisable. Notice that at least one of the two parameters must be specified. In cases where the number of neighbouring points is fewer than eight, each of the output feature values will be set to 0.0. 

Note that if the user does not specify the optional input LiDAR file, the tool will search for all valid LiDAR (*.las, *.laz, *.zlidar) files contained within the current working directory. This feature can be useful for processing a large number of LiDAR files in batch mode. 

The binary data file (*.eigen) can be used directly by the `filter_lidar` and `modify_lidar` tools, and will be automatically read by the tools when the *.eigen and *.eigen.json files are present in the same folder as the accompanying source LiDAR file. This allows users to apply data filters, or to modify point properties, using these point neighbourhood features. For example, the statement, `rgb=(int(linearity*255), int(planarity*255), int(sphericity*255))`, used with the `modify_lidar` tool, can render the point RGB colour values based on some of the eigenvalue features, allowing users to visually identify linear features (red), planar features (green), and volumetric regions (blue). 

 

Additionally, these features data can also be readily incorporated into a Python-based point analysis or classification. As an example, the following script reads in a *.eigen binary data file for direct manipulation and analysis: 

`import numpy as np 

dt = np.dtype([ ('point_num', '<u8'), ('lambda1', '<f4'), ('lambda2', '<f4'), ('lambda3', '<f4'), ('linearity', '<f4'), ('planarity', '<f4'), ('sphericity', '<f4'), ('omnivariance', '<f4'), ('eigentropy', '<f4'), ('slope', '<f4'), ('resid', '<f4') ]) 

with open('/Users/johnlindsay/Documents/data/aaa2.eigen', 'rb') as f: b = f.read() 

pt_features = np.frombuffer(b, dt) 

### Print the first 100 point features to demonstrate

 

for i in range(100): print(f"{pt_features['point_num'][i]} {pt_features['linearity'][i]} {pt_features['planarity'][i]} {pt_features['sphericity'][i]}") 

print("Done!") ` 

### References

 

Chehata, N., Guo, L., & Mallet, C. (2009). Airborne lidar feature selection for urban classification using random forests. In Laser Scanning IAPRS, Vol. XXXVIII, Part 3/W8 – Paris, France, September 1-2, 2009. 

Gross, H., Jutzi, B., & Thoennessen, U. (2007). Segmentation of tree regions using data of a full-waveform laser. International Archives of Photogrammetry, Remote Sensing and Spatial Information Sciences, 36(part 3), W49A. 

Niemeyer, J., Mallet, C., Rottensteiner, F., & Sörgel, U. (2012). Conditional Random Fields for the Classification of LIDAR Point Clouds. In XXII ISPRS Congress at Melbourne, ISPRS Annals of the Photogrammetry, Remote Sensing and Spatial Information Sciences (Vol. 3). 

West, K. F., Webb, B. N., Lersch, J. R., Pothier, S., Triscari, J. M., & Iverson, A. E. (2004). Context-driven automated target detection in 3D data. In Automatic Target Recognition XIV (Vol. 5426, pp. 133-143). SPIE. 

### See Also

 

`filter_lidar`, `modify_lidar`, `sort_lidar`, `split_lidar` 

### Python API

```python
def lidar_eigenvalue_features(self, input_lidar: Optional[Lidar], num_neighbours: Optional[int], search_radius: Optional[float]) -> None:
```


---

## LiDAR Histogram

**Function name:** `lidar_histogram`


This tool can be used to plot a histogram of data derived from a LiDAR file. The user must specify the name of the input LAS file (`input`), the name of the output HTML file (`output`), the parameter (`parameter`) to be plotted, and the amount (in percent) to clip the upper and lower tails of the f requency distribution (`clip`). The LiDAR parameters that can be plotted using `lidar_histogram` include the point elevations, intensity values, scan angles, and class values. 

Use the `lidar_point_stats` tool instead to examine the spatial distribution of LiDAR points. 

 

### See Also

 

`lidar_point_stats` 

### Python API

```python
def lidar_histogram(self, input_lidar: Lidar, output_html_file: str, parameter: str = "elevation", clip_percent: float = 1.0) -> None:
```


---

## LiDAR Info

**Function name:** `lidar_info`


This tool can be used to print basic information about the data contained within a LAS file, used to store LiDAR data. The reported information will include including data on the header, point return frequency, and classification data and information about the variable length records (VLRs) and geokeys. If the `output_html_file` is specified, the function will write the output information as a HTML file that will be automatically displayed. If this parameter is unspecified, the function will return a string containing the information instead. 

### Python API

```python
def lidar_info(self, input_lidar: Lidar, output_html_file: str = None, show_point_density: bool = True, show_vlrs: bool = True, show_geokeys: bool = True) -> str:
```


---

## LiDAR Kappa

**Function name:** `lidar_kappa`


This tool performs a kappa index of agreement (KIA) analysis on the classification values of two LiDAR (LAS) files. The output report HTML file should be displayed automatically but can also be displayed afterwards in any web browser. As a measure of overall classification accuracy, the KIA is more robust than the percent agreement calculation because it takes into account the agreement occurring by random chance. In addition to the KIA, the tool will output the producer's and user's accuracy, the overall accuracy, and the error matrix. The KIA is often used as a means of assessing the accuracy of an image classification analysis; however the `LidarKappaIndex` tool performs the analysis on a point-to-point basis, comparing the class values of the points in one input LAS file with the corresponding nearest points in the second input LAS file. 

The user must also specify the name and resolution of an output raster file, which is used to show the spatial distribution of class accuracy. Each grid cell contains the overall accuracy, i.e. the points correctly classified divided by the total number of points contained within the cell, expressed as a percentage. 

### Python API

```python
def lidar_kappa(self, input_lidar1: Lidar, input_lidar2: Lidar, output_html_file: str, cell_size: float = 1.0, output_class_accuracy: bool = False) -> Raster:
```


---

## LiDAR Point Density

**Function name:** `lidar_point_density`


### Python API

```python
def lidar_point_density(self, input_lidar: Optional[Lidar], returns_included: str = "all", cell_size: float = 1.0, search_radius: float = 2.5, excluded_classes: List[int] = None, min_elev: float = float('-inf'), max_elev: float = float('inf')) -> Raster:
```


---

## LiDAR Point Return Analysis

**Function name:** `lidar_point_return_analysis`


### Description

 

This performs a quality control check on the return values of points in a LiDAR file. In particular, the tool will search for missing point returns, duplicate point returns, and points for which the return number (*r*) is larger than the encoded number of returns (*n*), all of which may be indicative of processing or encoding errors in the input file. 

The user must specify the name of the input LiDAR file (`input`), and may optionally specify an output LiDAR file (`output`). If no output file is specified, only the text report is generated by the tool. If an output is specified, the tool will create an output LiDAR file for which missing returns are assigned class 13, duplicate returns are assigned class 14, points that are both, part of a missing series and are duplicate returns, are classed 15, and all other non-problemmatic points are assigned class 1. Note, those points designated as missing in the output image are clearly not so much missing as they are part of a sequence of points that contain missing returns. Missing points are apparent when the first point in a series does not have *r* = 1, when the last point does not have *r* = *n*, or the series is non-sequential (e.g. 1/3, 3/3, but no 2/3). This condition may occur because returns are split between tiles. However, when sequences with missing points are not located near the edges of tiles, it is usually an indication that either point filtering has taken place during pre-processing or that there is been some kind of processing or encoding error. 

Duplicate points are defined as points that share the same time, scanner channel, *r*, and *n*. Note that these points may have different x, y, z coordinates. Duplicate points are always an indication of a processing or encoding error. For example, it may indicate that the scanner channel information from a multi-channel LiDAR sensor has not been encoded when creating the file or has been lost. 

No point should have *r* > *n*. This always indicates some kind of processing or encoding error when it occurs. 

The following is a sample output report generated by this tool: 

`*************************************** * Welcome to LidarPointReturnAnalysis *  

The Global Encoding for this file indicates that the point returns are not synthetic. 

Missing Returns: 2441636 (16.336 percent) points are missing  rnMissing Pts 121127770 22817 13823240 23569 33718 14285695 24142890 34142 44213 1529772 2519848 359928 4518 5516    

Duplicate Returns: 4311021 (28.844 percent) points are duplicates  rnDuplicates 112707083 12332028 22663717 1370619 23211834 33282348 142856 248568 3414280 4417136 1523 2569 35115 45161 55184    

Return Greater Than Num. Returns: 0 (0.000 percent) points have r > n 

Writing output LAS file... Complete! Elapsed Time (including I/O): 1.959s ` 

### Python API

```python
def lidar_point_return_analysis(self, input: Lidar, create_output: bool = False) -> Optional[Lidar]:
```


---

## LiDAR Point Stats

**Function name:** `lidar_point_stats`


This tool creates several rasters summarizing the distribution of LiDAR points in a LAS data file. The user must specify the name of an input LAS file (`input`) and the output raster grid resolution (`resolution`). Additionally, the user must specify one or more of the possible output rasters to create using the various available flags, which include:  FlagMeaning `num_points`Number of points (returns) in each grid cell `num_pulses`Number of pulses in each grid cell `avg_points_per_pulse`Average number of points per pulse in each grid cells `z_range`Elevation range within each grid cell `intensity_range`Intensity range within each grid cell `predom_class`Predominant class value within each grid cell   

If no output raster flags are specified, all of the output rasters will be created. All output rasters will have the same base name as the input LAS file but will have a suffix that reflects the statistic type (e.g. _num_pnts, _num_pulses, _avg_points_per_pulse, etc.). Output files will be in the GeoTIFF (*.tif) file format. 

When the input/output parameters are not specified, the tool works on all LAS files contained within the working directory. 

**Notes**: 1. The num_pulses output is actually the number of pulses with at least one return; specifically it is    the sum of the early returns (first and only) in a grid cell. In areas of low reflectance, such as    over water surfaces, the system may have emitted a significantly higher pulse rate but far fewer    returns are observed. 2. The memory requirement of this tool is high, particulalry if the grid resolution is fine and    the spatial extent is large. 

### See Also

 

`lidar_block_minimum`, `lidar_block_maximum` 

### Python API

```python
def lidar_point_stats(self, input_lidar: Optional[Lidar], cell_size: float = 1.0, num_points: bool = False, num_pulses: bool = False, avg_points_per_pulse: bool = False, z_range: bool = False, intensity_range: bool = False, predominant_class: bool = False) :
```


---

## LiDAR Ransac Planes

**Function name:** `lidar_ransac_planes`


This tool uses the `random sample consensus (RANSAC)` method to identify points within a LiDAR point cloud that belong to planar surfaces. RANSAC is a common method used in the field of computer vision to identify a subset of inlier points in a noisy data set containing abundant outlier points. Because LiDAR point clouds often contain vegetation points that do not form planar surfaces, this tool can be used to largely strip vegetation points from the point cloud, leaving behind the ground returns, buildings, and other points belonging to planar surfaces. If the `classify` flag is used, non-planar points will not be removed but rather will be assigned a different class (1) than the planar points (0). 

The algorithm selects a random sample, of a specified size (`num_samples`) of the points from within the neighbourhood (`radius`) surrounding each LiDAR point. The sample is then used to parameterize a planar best-fit model. The distance between each neighbouring point and the plane is then evaluated; inliers are those neighbouring points within a user-specified distance threshold (`threshold`). Models with at least a minimum number of inlier points (`model_size`) are then accepted. This process of selecting models is iterated a number of user-specified times (`num_iter`). 

One of the challenges with identifying planar surfaces in LiDAR point clouds is that these data are usually collected along scan lines. Therefore, each scan line can potentially yield a vertical planar surface, which is one reason that some vegetation points remain after applying the RANSAC plane-fitting method. To cope with this problem, the tool allows the user to specify a maximum planar slope (`max_slope`) parameter. Planes that have slopes greater than this threshold are rejected by the algorithm. This has the side-effect of removing building walls however. 

### References

 

Fischler MA and Bolles RC. 1981. Random sample consensus: a paradigm for model fitting with applications to image analysis and automated cartography. Commun. ACM, 24(6):381–395. 

### See Also

 

`lidar_segmentation`, `lidar_ground_point_filter` 

### Python API

```python
def lidar_ransac_planes(self, in_lidar: Lidar, search_radius: float = 2.0, num_iterations: int = 50, num_samples: int = 10, inlier_threshold: float = 0.15, acceptable_model_size: int = 30, max_planar_slope: float = 75.0, classify: bool = False, only_last_returns: bool = False) -> Lidar:
```


---

## LiDAR Rooftop Analysis

**Function name:** `lidar_rooftop_analysis`


This tool can be used to identify roof segments in a LiDAR point cloud. 

### See Also

 

`classify_buildings_in_lidar`, `clip_lidar_to_polygon` 

### Python API

```python
def lidar_rooftop_analysis(self, lidar_inputs: List[Lidar], building_footprints: Vector, search_radius: float = 2.0, num_iterations: int = 50, num_samples: int = 10, inlier_threshold: float = 0.15, acceptable_model_size: int = 30, max_planar_slope: float = 75.0, norm_diff_threshold: float = 2.0, azimuth: float = 180.0, altitude: float = 30.0) -> Vector:
```


---

## Normal Vectors

**Function name:** `normal_vectors`


Calculates normal vectors for points within a LAS file and stores these data (XYZ vector components) in the RGB field. 

### Python API

```python
def normal_vectors(self, input: Lidar, search_radius: float = -1.0) -> Lidar:
```
