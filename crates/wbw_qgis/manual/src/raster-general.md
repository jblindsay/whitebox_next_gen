# General Tools


---

## Abs

**Function name:** `abs`


Experimental

Calculates the absolute value of each raster cell.

raster math abs

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply abs transform to each non-nodata cell.*
`wbe.abs(input='dem.tif', output='abs_dem.tif')`


---

## Aggregate Raster

**Function name:** `aggregate_raster`


This tool can be used to reduce the grid resolution of a raster by a user specified amount. For example, using  an aggregation factor (`agg_factor`) of 2 would result in a raster with half the number of rows and columns.  The grid cell values (`type`) in the output image will consist of the mean, sum, maximum, minimum, or range  of the overlapping grid cells in the input raster (four cells in the case of an aggregation factor of 2).  

### See Also

 

`resample` 

### Python API

```python
def aggregate_raster(self, raster: Raster, aggregation_factor: int = 2, aggregation_type: str = "mean") -> Raster:
```


---

## Anova

**Function name:** `anova`


This tool performs an `Analysis of variance` (ANOVA) test on the distribution of values in a raster (`input`) among a group of features (`features`). The ANOVA report is written to an output HTML report (`output`). 

### Python API

```python
def anova(self, input_raster: Raster, features_raster: Raster, output_html_file: str) -> None:
```


---

## Arccos

**Function name:** `arccos`


Experimental

Computes the inverse cosine (arccos) of each raster cell.

raster math arccos

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply arccos transform to each non-nodata cell.*
`wbe.arccos(input='dem.tif', output='arccos_dem.tif')`


---

## Arcosh

**Function name:** `arcosh`


Experimental

Computes the inverse hyperbolic cosine of each raster cell.

raster math arcosh

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply arcosh transform to each non-nodata cell.*
`wbe.arcosh(input='dem.tif', output='arcosh_dem.tif')`


---

## Arcsin

**Function name:** `arcsin`


Experimental

Computes the inverse sine (arcsin) of each raster cell.

raster math arcsin

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply arcsin transform to each non-nodata cell.*
`wbe.arcsin(input='dem.tif', output='arcsin_dem.tif')`


---

## Arctan

**Function name:** `arctan`


Experimental

Computes the inverse tangent (arctan) of each raster cell.

raster math arctan

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply arctan transform to each non-nodata cell.*
`wbe.arctan(input='dem.tif', output='arctan_dem.tif')`


---

## Arsinh

**Function name:** `arsinh`


Experimental

Computes the inverse hyperbolic sine of each raster cell.

raster math arsinh

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply arsinh transform to each non-nodata cell.*
`wbe.arsinh(input='dem.tif', output='arsinh_dem.tif')`


---

## Artanh

**Function name:** `artanh`


Experimental

Computes the inverse hyperbolic tangent of each raster cell.

raster math artanh

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply artanh transform to each non-nodata cell.*
`wbe.artanh(input='dem.tif', output='artanh_dem.tif')`


---

## Atan2

**Function name:** `atan2`


Experimental

Computes the four-quadrant inverse tangent using two rasters on a cell-by-cell basis.

raster math atan2 legacy-port

### Parameters

NameDescriptionRequiredDefault
`input1`First input raster (path string or typed raster object).Required`input1.tif`
`input2`Second input raster (path string or typed raster object).Required`input2.tif`
`output`Optional output raster file path. If omitted, output remains in memory and is returned as a memory:// raster handle.Optional—

### Examples

*Runs atan2 on two DEM rasters and writes the result to dem_atan2.tif.*
`wbe.atan2(input1='dem_a.tif', input2='dem_b.tif', output='dem_atan2.tif')`


---

## Block Maximum

**Function name:** `block_maximum`


Creates a raster grid based on a set of vector points and assigns grid values using a block maximum scheme. 

### Python API

```python
def block_maximum(self, points: Vector, field_name: str = "FID", use_z: bool = False, cell_size: float = 0.0, base_raster: Raster = None) -> Raster:
```


---

## Block Minimum

**Function name:** `block_minimum`


Creates a raster grid based on a set of vector points and assigns grid values using a block minimum scheme. 

### Python API

```python
def block_minimum(self, points: Vector, field_name: str = "FID", use_z: bool = False, cell_size: float = 0.0, base_raster: Raster = None) -> Raster:
```


---

## Boundary Shape Complexity

**Function name:** `boundary_shape_complexity`


This tools calculates a type of shape complexity index for raster objects, focused on the complexity of the boundary of polygons. The index uses the `line_thinning` tool to estimate a skeletonized network for each input raster polygon. The Boundary Shape Complexity (BSC) index is then calculated as the percentage of the skeletonized network belonging to exterior links. Polygons with more complex boundaries will possess more branching skeletonized networks, with each spur in the boundary possessing a short exterior branch. The two longest exterior links in the network are considered to be part of the main network.  Therefore, polygons of complex shaped boundaries will have a higher percentage of their skeleton networks consisting of exterior links. It is expected that simple convex hulls should have relatively low BSC index values. 

Objects in the input raster (`input`) are designated by their unique identifiers. Identifier values should be positive, non-zero whole numbers. 

### See Also

 

`shape_complexity_index_raster`, `line_thinning` 

### Python API

```python
def boundary_shape_complexity(self, raster: Raster) -> Raster:
```


---

## Ceil

**Function name:** `ceil`


Experimental

Rounds each raster cell upward to the nearest integer.

raster math ceil

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply ceil transform to each non-nodata cell.*
`wbe.ceil(input='dem.tif', output='ceil_dem.tif')`


---

## Centroid Raster

**Function name:** `centroid_raster`


This tool calculates the centroid, or average location, of raster polygon objects. For vector features, use the `centroid_vector` tool instead. 

### See Also

 

`centroid_vector` 

### Python API

```python
def centroid_raster(self, input: Raster) -> Tuple[Raster, str]:
```


---

## Clip Raster To Polygon

**Function name:** `clip_raster_to_polygon`


This tool can be used to clip an input raster (`input`) to the extent of a vector polygon (shapefile). The user must specify the name of the input clip file (`polygons`), which must be a vector of a Polygon base shape type. The clip file may contain multiple polygon features. Polygon hole parts will be respected during clipping, i.e. polygon holes will be removed from the output raster by setting them to a NoData background value. Raster grid cells that fall outside of a polygons in the clip file will be assigned the NoData background value in the output file. By default, the output raster will be cropped to the spatial extent of the clip file, unless the `maintain_dimensions` parameter is used, in which case the output grid extent will match that of the input raster. The grid resolution of output raster is the same as the input raster. 

It is very important that the input raster and the input vector polygon file share the same projection. The result is unlikely to be satisfactory otherwise. 

### See Also

 

`erase_polygon_from_raster` 

### Python API

```python
def clip_raster_to_polygon(self, raster: Raster, polygons: Vector, maintain_dimensions: bool = False) -> Raster:
```


---

## Clump

**Function name:** `clump`


This tool re-categorizes data in a raster image by grouping cells that form discrete, contiguous areas into unique categories. Essentially this will produce a patch map from an input categorical raster, assigning each feature unique identifiers. The input raster should either be Boolean (1's and 0's) or categorical. The input raster could be created using the `reclass` tool or one of the comparison operators (`GreaterThan`, `LessThan`, `EqualTo`, `NotEqualTo`). Use the *treat zeros as background cells* options (`zero_back`) if you would like to only assigned contiguous groups of non-zero values in the raster unique identifiers. Additionally, inter-cell connectivity can optionally include diagonally neighbouring cells if the `diag` flag is specified. 

### See Also

 

`reclass`, `GreaterThan`, `LessThan`, `EqualTo`, `NotEqualTo` 

### Python API

```python
def clump(self, raster: Raster, diag: bool = False, zero_background: bool = False) -> Raster:
```


---

## Cos

**Function name:** `cos`


Experimental

Computes the cosine of each raster cell value.

raster math cos

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply cos transform to each non-nodata cell.*
`wbe.cos(input='dem.tif', output='cos_dem.tif')`


---

## Cosh

**Function name:** `cosh`


Experimental

Computes the hyperbolic cosine of each raster cell.

raster math cosh

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply cosh transform to each non-nodata cell.*
`wbe.cosh(input='dem.tif', output='cosh_dem.tif')`


---

## Create Plane

**Function name:** `create_plane`


This tool can be used to create a new raster with values that are determined by the equation of a simple plane. The user must specify the name of a base raster (`base`) from which the output raster coordinate and dimensional information will be taken. In addition the user must specify the values of the planar slope gradient (S; `gradient`; `aspect`) in degrees, the planar slope direction or aspect (A; 0 to 360 degrees), and an constant value (k; `constant`). The equation of the plane is as follows:  

Z = tan(S) × sin(A - 180) × X + tan(S) × cos(A - 180) × Y + k  

where X and Y are the X and Y coordinates of each grid cell in the grid. Notice that A is the direction, or azimuth, that the plane is facing 

### Python API

```python
def create_plane(self, base_file: Raster, gradient: float, aspect: float, constant: float) -> Raster:
```


---

## Crispness Index

**Function name:** `crispness_index`


The Crispness Index (*C*) provides a means of quantifying the crispness, or fuzziness, of a membership probability (MP) image. MP images describe the probability of each grid cell belonging to some feature or class. MP images contain values ranging from 0 to 1. 

The index, as described by Lindsay (2006), is the ratio between the sum of the squared differences (from the image mean) in the MP image divided by the sum of the squared differences for the Boolean case in which the total probability, summed for the image, is arranged crisply. 

*C* is closely related to a family of relative variation coefficients that measure variation in an MP image relative to the maximum possible variation (i.e. when the total probability is arranged such that grid cells contain only 1s or 0s). Notice that 0 < *C* < 1 and a low *C*-value indicates a nearly uniform spatial distribution of any probability value, and *C* = 1 indicates a crisp spatial probability distribution, containing only 1's and 0's. 

*C* is calculated as follows:  

C = SS_mp ∕ SS_B = [∑(pij − p-bar)^2] ∕ [ ∑pij(1 − p-bar)^2 + p2(RC − ∑pij)]  

Note that there is an error in the original published equation. Specifically, the denominator read:  

∑pij(1 - p_bar)^2 + p_bar^2 (RC - ∑pij)  

instead of the original:  

∑pij(1 - p_bar^2) - p_bar^2 (RC - ∑pij)  

### References

 

Lindsay, J. B. (2006). Sensitivity of channel mapping techniques to uncertainty in digital elevation data. International Journal of Geographical Information Science, 20(6), 669-692. 

### Python API

```python
def crispness_index(self, raster: Raster, output_html_file: str) -> None:
```


---

## Cross Tabulation

**Function name:** `cross_tabulation`


This tool can be used to perform a cross-tabulation on two input raster images (`i1` and `i2`) containing categorical data, i.e. classes. It will output a `contingency table` in HTML format (`output`). A contingency table, also known as a cross tabulation or crosstab, is a type of table that displays the multivariate frequency distribution of the variables. These tables provide a basic picture of the interrelation between two categorical variables and can help find interactions between them. `cross_tabulation` can provide useful information about the nature of land-use/land-cover (LULC) changes between two dates of classified multi-spectral satellite imagery. For example, the extent of urban expansion could be described using the information about the extent of pixels in an 'urban' class in Date 2 that were previously assigned to other classes (e.g. agricultural LULC categories) in the Date 1 imagery. 

Both input images must share the same grid, as the analysis requires a comparison of a pair of images on a cell-by-cell basis. If a grid cell contains a **NoData** value in either of the input images, the cell will be excluded from the analysis. 

### Python API

```python
def cross_tabulation(self, raster1: Raster, raster2: Raster, output_html_file: str) -> None:
```


---

## Cumulative Distribution

**Function name:** `cumulative_distribution`


This tool converts the values in an input image (`input`) into a `cumulative distribution function`. Therefore, the output raster (`output`) will contain the cumulative probability value (0-1) of of values equal to or less than the value in the corresponding grid cell in the input image. NoData values in the input image are not considered during the transformation and remain NoData values in the output image. 

### See Also

 

`z_scores` 

### Python API

```python
def cumulative_distribution(self, raster: Raster) -> Raster:
```


---

## Dbscan

**Function name:** `dbscan`


### Description

 

This tool performs an unsupervised `DBSCAN` clustering operation, based on a series of input rasters (`inputs`). Each grid cell defines a stack of feature values (one value for each input raster), which serves as a point within the multi-dimensional feature space. The DBSCAN algorithm identifies clusters in feature space by identifying regions of high density (core points) and the set of points connected to these high-density areas. Points in feature space that are not connected to high-density regions are labeled by the DBSCAN algorithm as 'noise' and the associated grid cell in the output raster (`output`) is assigned the nodata value. Areas of high density (i.e. core points) are defined as those points for which the number of neighbouring points within a search distance (`search_dist`) is greater than some user-defined minimum threshold (`min_points`). 

The main advantages of the DBSCAN algorithm over other clustering methods, such as *k*-means (`k_means_clustering`), is that 1) you do not need to specify the number of clusters *a priori*, and 2) that the method does not make assumptions about the shape of the cluster (spherical in the *k*-means method). However, DBSCAN does assume that the density of every cluster in the data is approximately equal, which may not be a valid assumption. DBSCAN may also produce unsatisfactory results if there is significant overlap among clusters, as it will aggregate the clusters. Finding search distance and minimum core-point density thresholds that apply globally to the entire data set may be very challenging or impossible for certain applications. 

The DBSCAN algorithm is based on the calculation of distances in multi-dimensional space. Feature scaling is essential to the application of DBSCAN clustering, especially when the ranges of the features are different, for example, if they are measured in different units. Without scaling, features with larger ranges will have greater influence in computing the distances between points. The tool offers three options for feature-scaling (`scaling`), including 'None', 'Normalize', and 'Standardize'. Normalization simply rescales each of the features onto a 0-1 range. This is a good option for most applications, but it is highly sensitive to outliers because it is determined by the range of the minimum and maximum values. Standardization rescales predictors using their means and standard deviations, transforming the data into z-scores. This is a better option than normalization when you know that the data contain outlier values; however, it does does assume that the feature data are somewhat normally distributed, or are at least symmetrical in distribution. 

One should keep the impact of feature scaling in mind when setting the `search_dist` parameter. For example, if applying normalization, the entire range of values for each dimension of feature space will be bound within the 0-1 range, meaning that the search distance should be smaller than 1.0, and likely significantly smaller. If standardization is used instead, features space is technically infinite, although the vast majority of the data are likely to be contained within the range -2.5 to 2.5. 

Because the DBSCAN algorithm calculates distances in feature-space, like many other related algorithms, it suffers from the `curse of dimensionality`. Distances become less meaningful in high-dimensional space because the vastness of these spaces means that distances between points are less significant (more similar). As such, if the predictor list includes insignificant or highly correlated variables, it is advisable to exclude these features during the model-building phase, or to use a dimension reduction technique such as `principal_component_analysis` to transform the features into a smaller set of uncorrelated predictors. 

### Memory Usage

 

The peak memory usage of this tool is approximately 8 bytes per grid cell &times; # predictors. 

### See Also

 

`k_means_clustering`, `modified_k_means_clustering`, `principal_component_analysis` 

### Python API

```python
def dbscan(self, input_rasters: List[Raster], scaling_method: str = "none", search_distance: float = 1.0, min_points: int = 5) -> Raster:
```


---

## Decrement

**Function name:** `decrement`


Experimental

Subtracts 1 from each non-nodata raster cell.

raster math decrement

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply decrement transform to each non-nodata cell.*
`wbe.decrement(input='dem.tif', output='decrement_dem.tif')`


---

## Edge Proportion

**Function name:** `edge_proportion`


This tool will measure the edge proportion, i.e. the proportion of grid cells in a patch that are located along the patch's boundary, for an input raster image (`input`). Edge proportion is an indicator of polygon shape complexity and elongation. The user must specify the name of the output raster file (`output`), which will be raster layer containing the input features assigned the edge proportion. The user may also optionally choose to output text data for easy input to a spreadsheet or database. 

Objects in the input raster are designated by their unique identifiers. Identifier values must be positive, non-zero whole numbers. 

### See Also

 

`shape_complexity_index_raster`, `linearity_index`, `elongation_ratio` 

### Python API

```python
def edge_proportion(self, raster: Raster) -> Tuple[Raster, str]:
```


---

## Equal To

**Function name:** `equal_to`


Experimental

Tests whether two rasters are equal on a cell-by-cell basis.

raster math equal_to legacy-port

### Parameters

NameDescriptionRequiredDefault
`input1`First input raster (path string or typed raster object).Required`input1.tif`
`input2`Second input raster (path string or typed raster object).Required`input2.tif`
`output`Optional output raster file path. If omitted, output remains in memory and is returned as a memory:// raster handle.Optional—

### Examples

*Runs equal_to on two DEM rasters and writes the result to dem_equal_to.tif.*
`wbe.equal_to(input1='dem_a.tif', input2='dem_b.tif', output='dem_equal_to.tif')`


---

## Erase Polygon From Raster

**Function name:** `erase_polygon_from_raster`


This tool can be used to set values an input raster (`input`) to a NoData background value with a vector erasing polygon (`polygons`). The input erase polygon file must be a vector of a Polygon base shape type. The erase file may contain multiple polygon features. Polygon hole parts will be respected during clipping, i.e. polygon holes will not be removed from the output raster. Raster grid cells that fall inside of a polygons in the erase file will be assigned the NoData background value in the output file. 

### See Also

 

`clip_raster_to_polygon` 

### Python API

```python
def erase_polygon_from_raster(self, raster: Raster, polygons: Vector) -> Raster:
```


---

## Exp

**Function name:** `exp`


Experimental

Computes e raised to the power of each raster cell.

raster math exp

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply exp transform to each non-nodata cell.*
`wbe.exp(input='dem.tif', output='exp_dem.tif')`


---

## Exp2

**Function name:** `exp2`


Experimental

Computes 2 raised to the power of each raster cell.

raster math exp2

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply exp2 transform to each non-nodata cell.*
`wbe.exp2(input='dem.tif', output='exp2_dem.tif')`


---

## FFT Random Field

**Function name:** `fft_random_field`


*No help documentation available for this tool.*


---

## Filter Raster Features By Area

**Function name:** `filter_raster_features_by_area`


This tool takes an input raster (`input`) containing integer-labelled features, such as the output of the `clump` tool, and removes all features that are smaller than a user-specified size (`threshold`), measured in grid cells. The user must specify the replacement value for removed features using the `background` parameter, which can be either `zero` or `nodata`. 

### See Also

 

`clump` 

### Python API

```python
def filter_raster_features_by_area(self, input: Raster, threshold: int, zero_background: bool = False) -> Raster:
```


---

## Find Patch Edge Cells

**Function name:** `find_patch_edge_cells`


This tool will identify all grid cells situated along the edges of patches or class features within an input raster (`input`). Edge cells in the output raster (`output`) will have the patch identifier value assigned in the corresponding grid cell. All non-edge cells will be assigned zero in the output raster. Patches (or classes) are designated by positive, non-zero values in the input image. Zero-valued and NoData-valued grid cells are interpreted as background cells by the tool. 

### See Also

 

`edge_proportion` 

### Python API

```python
def find_patch_edge_cells(self, raster: Raster) -> Raster:
```


---

## Floor

**Function name:** `floor`


Experimental

Rounds each raster cell downward to the nearest integer.

raster math floor

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply floor transform to each non-nodata cell.*
`wbe.floor(input='dem.tif', output='floor_dem.tif')`


---

## Greater Than

**Function name:** `greater_than`


Experimental

Tests whether the first raster is greater than the second on a cell-by-cell basis.

raster math greater_than legacy-port

### Parameters

NameDescriptionRequiredDefault
`input1`First input raster (path string or typed raster object).Required`input1.tif`
`input2`Second input raster (path string or typed raster object).Required`input2.tif`
`output`Optional output raster file path. If omitted, output remains in memory and is returned as a memory:// raster handle.Optional—

### Examples

*Runs greater_than on two DEM rasters and writes the result to dem_greater_than.tif.*
`wbe.greater_than(input1='dem_a.tif', input2='dem_b.tif', output='dem_greater_than.tif')`


---

## Heat Map

**Function name:** `heat_map`


This tool is used to generate a raster heat map, or `kernel density estimation` surface raster from a set of vector points (`input`). Heat mapping is a visualization and modelling technique used to create the continuous density surface associated with the occurrences of a point phenomenon. Heat maps can  therefore be used to identify point clusters by mapping the concentration of event occurrence. For example, heat maps have been used extensively to map the spatial distributions of crime events (i.e. crime mapping) or disease cases. 

By default, the tool maps the density of raw occurrence events, however, the user may optionally specify an associated weights field (`weights`) from the point file's attribute table. When a weights field is specified, these values  are simply multiplied by each of the individual components of the density estimate. Weights must be numeric. 

The bandwidth parameter (--bandwidth) determines the radius of the `kernel`  used in calculation of the density surface. There are `guidelines`  that statisticians use in determining an appropriate bandwidth for a particular population and data set, but often  this parameter is determined through experimentation. The bandwidth of the kernel is a free parameter which exhibits  a strong influence on the resulting estimate.  

The user must specify the kernel `function type`  (`kernel`). Options include 'uniform', 'triangular', 'epanechnikov', 'quartic', 'triweight', 'tricube', 'gaussian', 'cosine',  'logistic', 'sigmoid', and 'silverman'; 'quartic' is the default kernel type. Descriptions of each function can be found at the  link above. 

The characteristics of the output raster (resolution and extent) are determined by one of two optional parameters, `cell_size` and `base`. If the user optionally specifies the output grid cell size parameter (`cell_size`)  then the coordinates of the output raster extent are determined by the input vector (i.e. the bounding box) and  the specified cell size determines the number of rows and columns. If the user instead specifies the optional  base raster file parameter (`base`), the output raster's coordinates (i.e. north, south, east, west) and row  and column count, and therefore, resolution, will be the same as the base file. 

### Reference

 

Geomatics (2017) QGIS Heatmap Using Kernel Density Estimation Explained, online resource: `https://www.geodose.com/2017/11/qgis-heatmap-using-kernel-density.html` visited 02/06/2022. 

### Python API

```python
def heat_map(self, points: Vector, field_name: str, bandwidth: float = 0.0, cell_size: float = 0.0, base_raster: Raster = None, kernel_function: str = "quartic") -> Raster:
```


---

## IDW Interpolation

**Function name:** `idw_interpolation`


points or a fixed neighbourhood size. This tool is currently configured to perform the later only, using a FixedRadiusSearch structure. Using a fixed number of neighbours will require use of a KD-tree structure. I've been testing one Rust KD-tree library but its performance does not appear to be satisfactory compared to the FixedRadiusSearch. I will need to explore other options here. 

Another change that will need to be implemented is the use of a nodal function. The original Whitebox GAT tool allows for use of a constant or a quadratic. This tool only allows the former. 

### Python API

```python
def idw_interpolation(self, points: Vector, field_name: str = "FID", use_z: bool = False, weight: float = 2.0, radius: float = 0.0, min_points: int = 0, cell_size: float = 0.0, base_raster: Raster = None) -> Raster:
```


---

## Image Autocorrelation

**Function name:** `image_autocorrelation`


Spatial autocorrelation describes the extent to which a variable is either dispersed or clustered through space. In the case of a raster image, spatial autocorrelation refers to the similarity in the values of nearby grid cells. This tool measures the spatial autocorrelation of a raster image using the global Moran's *I* statistic. Moran's *I* varies from -1 to 1, where *I* = -1 indicates a dispersed, checkerboard type pattern and *I* = 1 indicates a clustered (smooth) surface. *I* = 0 occurs for a random distribution of values. `image_autocorrelation` computes Moran's *I* for the first lag only, meaning that it only takes into account the variability among the immediate neighbors of each grid cell. 

The user must specify the names of one or more input raster images. In addition, the user must specify the contiguity type (`contiguity`; Rook's, King's, or Bishop's), which describes which neighboring grid cells are examined for the analysis. The following figure describes the available cases: 

Rook's contiguity  ... 010 1X1 010   

Kings's contiguity  ... 111 1X1 111   

Bishops's contiguity  ... 101 0X0 101   

The tool outputs an HTML report (`output`) which, for each input image (`input`), reports the Moran's *I* value and the variance, z-score, and p-value (significance) under normal and randomization sampling assumptions. 

Use the `image_correlation` tool instead when there is need to determine the correlation among multiple raster inputs. 

**NoData **values in the input image are ignored during the analysis. 

### See Also

 

`image_correlation`, `image_correlation_neighbourhood_analysis` 

### Python API

```python
def image_autocorrelation(self, rasters: List[Raster], output_html_file: str, contiguity_type: str = "bishop") -> None:
```


---

## Image Correlation

**Function name:** `image_correlation`


This tool can be used to estimate the Pearson product-moment correlation coefficient (*r*) between two or more input images (`inputs`). The *r*-value is a measure of the linear association in the variation of the input variables (images, in this case). The coefficient ranges from -1.0, indicated a perfect negative linear association, to 1.0, indicated a perfect positive linear association. An *r*-value of 0.0 indicates no correlation between the test variables. 

Note that this index is a measure of the linear association; two variables may be strongly related by a non-linear association (e.g. a power function curve) which will lead to an apparent weak association based on the Pearson coefficient. In fact, non-linear associations are very common among spatial variables, e.g. terrain indices such as slope and contributing area. In such cases, it is advisable that the input images are transformed prior to the estimation of the Pearson coefficient, or that an alternative, non-parametric statistic be used, e.g. the Spearman rank correlation coefficient. 

The user must specify the names of two or more input images (`inputs`). All input images must share the same grid, as the coefficient requires a comparison of a pair of images on a grid-cell-by-grid-cell basis. If more than two image names are selected, the correlation coefficient will be calculated for each pair of images and reported in the HTML output report (`output`) as a correlation matrix. Caution must be exercised when attempted to estimate the significance of a correlation coefficient derived from image data. The very high *N*-value (essentially the number of pixels in the image pair) means that even small correlation coefficients can be found to be statistically significant, despite being practically insignificant. 

**NoData** values in either of the two input images are ignored during the calculation of the correlation between images. 

### See Also

 

`image_correlation_neighbourhood_analysis`, `image_regression`, `image_autocorrelation` 

### Python API

```python
def image_correlation(self, rasters: List[Raster], output_html_file: str) -> None:
```


---

## Image Regression

**Function name:** `image_regression`


This tool performs a bivariate linear regression analysis on two input raster images. The first image (`i1`) is considered to be the independent variable while the second image (`i2`) is considered to be the dependent variable in the analysis. Both input images must share the same grid, as the coefficient requires a comparison of a pair of images on a grid-cell-by-grid-cell basis. The tool will output an HTML report (`output`) summarizing the regression model, an Analysis of Variance (ANOVA), and the significance of the regression coefficients. The regression residuals can optionally be output as a new raster image (`out_residuals`) and the user can also optionally specify to standardize the residuals (`standardize`). 

Note that the analysis performs a linear regression; two variables may be strongly related by a non-linear association (e.g. a power function curve) which will lead to an apparently weak fitting regression model. In fact, non-linear relations are very common among spatial variables, e.g. terrain indices such as slope and contributing area. In such cases, it is advisable that the input images are transformed prior to the analysis. 

**NoData** values in either of the two input images are ignored during the calculation of the correlation between images. 

### Example usage

 

import whitebox_workflow 

### See Also

 

`image_correlation`, `image_correlation_neighbourhood_analysis` 

### Python API

```python
def image_regression(self, independent_variable: Raster, dependent_variable: Raster, output_html_file: str, standardize_residuals: bool = False, output_scattergram: bool = False, num_samples: int = 1000) -> Raster:
```


---

## Increment

**Function name:** `increment`


Experimental

Adds 1 to each non-nodata raster cell.

raster math increment

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply increment transform to each non-nodata cell.*
`wbe.increment(input='dem.tif', output='increment_dem.tif')`


---

## Inplace Add

**Function name:** `inplace_add`


Experimental

Performs an in-place addition operation (input1 += input2).

raster math legacy-port

### Parameters

NameDescriptionRequiredDefault
`input1`Input raster to modify.Required`in1.tif`
`input2`Input raster path or numeric constant.Required`in2.tif`

### Examples

*Modify input1 by adding input2.*
`wbe.inplace_add(input1='in1.tif', input2=10.5)`


---

## Inplace Divide

**Function name:** `inplace_divide`


Experimental

Performs an in-place division operation (input1 /= input2).

raster math legacy-port

### Parameters

NameDescriptionRequiredDefault
`input1`Input raster to modify.Required`in1.tif`
`input2`Input raster path or non-zero numeric constant.Required`in2.tif`

### Examples

*Modify input1 by dividing by input2.*
`wbe.inplace_divide(input1='in1.tif', input2=10.5)`


---

## Inplace Multiply

**Function name:** `inplace_multiply`


Experimental

Performs an in-place multiplication operation (input1 *= input2).

raster math legacy-port

### Parameters

NameDescriptionRequiredDefault
`input1`Input raster to modify.Required`in1.tif`
`input2`Input raster path or numeric constant.Required`in2.tif`

### Examples

*Modify input1 by multiplying with input2.*
`wbe.inplace_multiply(input1='in1.tif', input2=10.5)`


---

## Inplace Subtract

**Function name:** `inplace_subtract`


Experimental

Performs an in-place subtraction operation (input1 -= input2).

raster math legacy-port

### Parameters

NameDescriptionRequiredDefault
`input1`Input raster to modify.Required`in1.tif`
`input2`Input raster path or numeric constant.Required`in2.tif`

### Examples

*Modify input1 by subtracting input2.*
`wbe.inplace_subtract(input1='in1.tif', input2=10.5)`


---

## Integer Division

**Function name:** `integer_division`


Experimental

Divides two rasters and truncates each result toward zero.

raster math integer_division legacy-port

### Parameters

NameDescriptionRequiredDefault
`input1`First input raster (path string or typed raster object).Required`input1.tif`
`input2`Second input raster (path string or typed raster object).Required`input2.tif`
`output`Optional output raster file path. If omitted, output remains in memory and is returned as a memory:// raster handle.Optional—

### Examples

*Runs integer_division on two DEM rasters and writes the result to dem_integer_division.tif.*
`wbe.integer_division(input1='dem_a.tif', input2='dem_b.tif', output='dem_integer_division.tif')`


---

## Inverse PCA

**Function name:** `inverse_pca`


### Description

 

This tool takes a two or more component images (`inputs`), and the `principal component analysis` (PCA) report derived using the `principal_component_analysis` tool, and performs the inverse PCA transform to derive the original series of input images. This inverse transform is frequently performed to reduce noise within a multi-spectral image data set. With a typical PCA transform, high-frequency noise will commonly map onto the higher component images. By excluding one or more higher-valued component images from the input component list, the inverse transform can produce a set of images in the original coordinate system that exclude the information contained within component images excluded from the input list. Note that the number of output images will also equal the number of original images input to the `principal_component_analysis` tool. The output images will be named automatically with a "inv_PCA_image" suffix. 

### See Also

 

`principal_component_analysis` 

### Python API

```python
def inverse_pca(self, rasters: List[Raster], pca_report_file: str) -> List[Raster]:
```


---

## Is Nodata

**Function name:** `is_nodata`


Experimental

Outputs 1 for nodata cells and 0 for all valid cells.

raster math is_nodata

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Identify nodata cells.*
`wbe.is_nodata(input='dem.tif', output='dem_is_nodata.tif')`


---

## Kappa Index

**Function name:** `kappa_index`


This tool calculates the `Kappa index of agreement` (KIA), or Cohen's Kappa, for two categorical input raster images (`input1` and `input2`). The KIA is a measure of inter-rater reliability (i.e. classification accuracy) and is widely applied in many fields, notably remote sensing. For example, The KIA is often used as a means of assessing the accuracy of an image classification analysis. The KIA can be interpreted as the percentage improvement that the underlying classification has over and above a random classifier (i.e. random assignment to categories). The user must specify the output HTML file (`output`). The input images must be of a categorical data type, i.e. contain classes. As a measure of classification accuracy, the KIA is more robust than the *overall percent agreement* because it takes into account the agreement occurring by chance. A KIA of 0 would indicate that the classifier is no better than random class assignment. In addition to the KIA, this tool will also output the `producer's and user's accuracy`, the overall accuracy, and the error matrix. 

### See Also

 

`cross_tabulation` 

### Python API

```python
def kappa_index(self, class_raster: Raster, reference_raster: Raster, output_html_file: str = "") -> None:
```


---

## KS Normality Test

**Function name:** `ks_normality_test`


This tool will perform a Kolmogorov-Smirnov (K-S) test for normality to evaluate whether the frequency distribution of values within a raster image are drawn from a Gaussian (normal) distribution. The user must specify the name of the raster image. The test can be performed optionally on the entire image or on a random sub-sample of pixel values of a user-specified size. In evaluating the significance of the test, it is important to keep in mind that given a sufficiently large sample, extremely small and non-notable differences can be found to be statistically significant. Furthermore statistical significance says nothing about the practical significance of a difference. 

### See Also

 

`two_sample_ks_test` 

### Python API

```python
def ks_normality_test(self, raster: Raster, output_html_file: str, num_samples: int) -> None:
```


---

## Less Than

**Function name:** `less_than`


Experimental

Tests whether the first raster is less than the second on a cell-by-cell basis.

raster math less_than legacy-port

### Parameters

NameDescriptionRequiredDefault
`input1`First input raster (path string or typed raster object).Required`input1.tif`
`input2`Second input raster (path string or typed raster object).Required`input2.tif`
`output`Optional output raster file path. If omitted, output remains in memory and is returned as a memory:// raster handle.Optional—

### Examples

*Runs less_than on two DEM rasters and writes the result to dem_less_than.tif.*
`wbe.less_than(input1='dem_a.tif', input2='dem_b.tif', output='dem_less_than.tif')`


---

## List Unique Values Raster

**Function name:** `list_unique_values_raster`


This function can be used to list each of the unique values contained within a categorical raster (`raster`). The tool  outputs string containing a comma-seperated variable (CSV) table of the unique values and their frequency of  occurrence within the data. The input raster *should not contain continuous floating-point numerical data*, because the number of categories will likely equal the number of pixels, which may be quite large. 

### See Also

 

`list_unique_values` 

### Python API

```python
def list_unique_values_raster(self, raster: Raster) -> str:
```


---

## Ln

**Function name:** `ln`


Experimental

Computes the natural logarithm of each raster cell.

raster math ln

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply ln transform to each non-nodata cell.*
`wbe.ln(input='dem.tif', output='ln_dem.tif')`


---

## Log10

**Function name:** `log10`


Experimental

Computes the base-10 logarithm of each raster cell.

raster math log10

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply log10 transform to each non-nodata cell.*
`wbe.log10(input='dem.tif', output='log10_dem.tif')`


---

## Log2

**Function name:** `log2`


Experimental

Computes the base-2 logarithm of each raster cell.

raster math log2

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply log2 transform to each non-nodata cell.*
`wbe.log2(input='dem.tif', output='log2_dem.tif')`


---

## Map Features

**Function name:** `map_features`


Experimental

Maps discrete elevated terrain features from a raster using descending-priority region growth.

raster gis features legacy-port

### Parameters

NameDescriptionRequiredDefault
`input`Input raster.Required`input.tif`
`min_feature_height`Minimum vertical separation for independent features.Required`1.0`
`min_feature_size`Minimum feature size in cells.Required`1`
`output`Optional output raster path.Optional—

### Examples

*Labels terrain features using descending-elevation region growth.*
`wbe.map_features(input='input.tif', min_feature_height=1.0, min_feature_size=1, output='map_features.tif')`


---

## Max

**Function name:** `max`


Experimental

Performs a MAX operation on two rasters or a raster and a constant value.

raster math max legacy-port

### Parameters

NameDescriptionRequiredDefault
`input1`First raster path or numeric constant.Required`in1.tif`
`input2`Second raster path or numeric constant.Required`in2.tif`
`output`Optional output raster path.Optional—

### Examples

*Compute cellwise maximum between a raster and a constant.*
`wbe.max(input1='in1.tif', input2='15.0', output='max_output.tif')`


---

## Min

**Function name:** `min`


Experimental

Performs a MIN operation on two rasters or a raster and a constant value.

raster math min legacy-port

### Parameters

NameDescriptionRequiredDefault
`input1`First raster path or numeric constant.Required`in1.tif`
`input2`Second raster path or numeric constant.Required`in2.tif`
`output`Optional output raster path.Optional—

### Examples

*Compute cellwise minimum between a raster and a constant.*
`wbe.min(input1='in1.tif', input2='15.0', output='min_output.tif')`


---

## Modified Shepard Interpolation

**Function name:** `modified_shepard_interpolation`


This tool interpolates vector points into a raster surface using a radial basis function (RBF) scheme. 

### Python API

```python
def radial_basis_function_interpolation(self, points: Vector, field_name: str = "FID", use_z: bool = False, radius: float = 0.0, min_points: int = 0, cell_size: float = 0.0, base_raster: Raster = None, func_type: str = "thinplatespline", poly_order: str = "none", weight: float = 0.1) -> Raster:
```


---

## Narrowness Index

**Function name:** `narrowness_index`


This tools calculates a type of shape narrowness index (*NI*) for raster objects. The index is equal to:  

*NI* = *A* / (&#960;*MD*2)  

where *A* is the patch area and *MD* is the maximum distance-to-edge of the patch. Circular-shaped patches will have a narrowness index near 1.0, while more narrow patch shapes will have higher index values. The index may be conceptualized as the ratio of the patch area to the area of the largest contained circle, although in practice the circle defined by the radius of the maximum distance-to-edge will often fall outside the patch boundaries. 

Objects in the input raster (`input`) are designated by their unique identifiers. Identifier values must be positive, non-zero whole numbers. It is quite common for identifiers to be set using the `clump` tool applied to some kind of thresholded raster. 

### See Also

 

`linearity_index`, `elongation_ratio`, `clump` 

### Python API

```python
def narrowness_index(self, raster: Raster) -> Raster:
```


---

## Negate

**Function name:** `negate`


Experimental

Negates each non-nodata raster cell value.

raster math negate

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply negate transform to each non-nodata cell.*
`wbe.negate(input='dem.tif', output='negate_dem.tif')`


---

## Nibble

**Function name:** `nibble`


The nibble function assigns areas within an input class map raster that are coincident with a mask the value  of their nearest neighbour. Nibble is typically used to replace erroneous sections in a class map. Cells in the mask raster that are either NoData or zero values will be replaced in the input image with their nearest non-masked value. All input raster values in non-mask areas will be unmodified. 

There are two input parameters that are related to how NoData cells in the input raster are handled during the nibble operation. The use_nodata Boolean determines whether or not input NoData cells, not contained within masked areas, are treated as ordinary values during the nibble. It is False by default, meaning that NoData cells  in the input raster do not extend into nibbled areas. When the nibble_nodata parameter is True, any NoData cells in the input raster that are within the masked area are also NoData in the output raster; when nibble_nodata is False these cells will be nibbled. 

### See Also:

 

`sieve` 

### Python API

```python
def nibble(self, input_raster: Raster, mask: Raster, use_nodata: bool = False, nibble_nodata: bool = True) -> Raster:
```


---

## Not Equal To

**Function name:** `not_equal_to`


Experimental

Tests whether two rasters are not equal on a cell-by-cell basis.

raster math not_equal_to legacy-port

### Parameters

NameDescriptionRequiredDefault
`input1`First input raster (path string or typed raster object).Required`input1.tif`
`input2`Second input raster (path string or typed raster object).Required`input2.tif`
`output`Optional output raster file path. If omitted, output remains in memory and is returned as a memory:// raster handle.Optional—

### Examples

*Runs not_equal_to on two DEM rasters and writes the result to dem_not_equal_to.tif.*
`wbe.not_equal_to(input1='dem_a.tif', input2='dem_b.tif', output='dem_not_equal_to.tif')`


---

## Paired Sample T Test

**Function name:** `paired_sample_t_test`


This tool will perform a paired-sample *t*-test to evaluate whether a significant statistical difference exists between the two rasters. The null hypothesis is that the difference between the paired population means is equal to zero. The paired-samples *t*-test makes an assumption that the differences between related samples follows a Gaussian distribution. The tool will output a cumulative probability distribution, with a fitted Gaussian, to help users evaluate whether this assumption is violated by the data. If this is the case, the `wilcoxon_signed_rank_test` should be used instead. 

The user must specify the name of the two input raster images (`input1` and `input2`) and the output report HTML file (`output`). The test can be performed optionally on the entire image or on a random sub-sample of pixel values of a user-specified size (`num_samples`). In evaluating the significance of the test, it is important to keep in mind that given a sufficiently large sample, extremely small and non-notable differences can be found to be statistically significant. Furthermore statistical significance says nothing about the practical significance of a difference. 

### See Also

 

`two_sample_ks_test`, `wilcoxon_signed_rank_test` 

### Python API

```python
def paired_sample_t_test(self, raster1: Raster, raster2: Raster, output_html_file: str, num_samples: int) -> None:
```


---

## Phi Coefficient

**Function name:** `phi_coefficient`


### Description

 

This tool performs a binary classification accuracy assessment, using the `Phi coefficient`. The Phi coefficient is a measure of association for two binary variables. Introduced by Karl Pearson, this measure is similar to the Pearson correlation coefficient in its interpretation and is related to the chi-squared statistic for a 2×2 contingency table. The user must specify the names of two input images (`input1` and `input2`), containing categorical data. 

### Python API

```python
def phi_coefficient(self, raster1: Raster, raster2: Raster, output_html_file: str) -> None:
```


---

## Principal Component Analysis

**Function name:** `principal_component_analysis`


Principal component analysis (PCA) is a common data reduction technique that is used to reduce the dimensionality of multi-dimensional space. In the field of remote sensing, PCA is often used to reduce the number of bands of multi-spectral, or hyper-spectral, imagery. Image correlation analysis often reveals a substantial level of correlation among bands of multi-spectral imagery. This correlation represents data redundancy, i.e. fewer images than the number of bands are required to represent the same information, where the information is related to variation within the imagery. PCA transforms the original data set of *n* bands into *n* 'component' images, where each component image is uncorrelated with all other components. The technique works by transforming the axes of the multi-spectral space such that it coincides with the directions of greatest correlation. Each of these new axes are orthogonal to one another, i.e. they are at right angles. PCA is therefore a type of coordinate system transformation. The PCA component images are arranged such that the greatest amount of variance (or information) within the original data set, is contained within the first component and the amount of variance decreases with each component. It is often the case that the majority of the information contained in a multi-spectral data set can be represented by the first three or four PCA components. The higher-order components are often associated with noise in the original data set. 

The user must specify the names of the multiple input images (`inputs`). Additionally, the user must specify whether to perform a standardized PCA (`standardized`) and the number of output components (`num_comp`) to generate (all components will be output unless otherwise specified). A standardized PCA is performed using the correlation matrix rather than the variance-covariance matrix. This is appropriate when the variances in the input images differ substantially, such as would be the case if they contained values that were recorded in different units (e.g. feet and meters) or on different scales (e.g. 8-bit vs. 16 bit). 

Several outputs will be generated when the tool has completed. The PCA report will be embedded within an output (`output`) HTML file, which should be automatically displayed after the tool has completed. This report contains useful data summarizing the results of the PCA, including the explained variances of each factor, the Eigenvalues and Eigenvectors associated with factors, the factor loadings, and a scree plot. The first table that is in the PCA report lists the amount of explained variance (in non-cumulative and cumulative form), the Eigenvalue, and the Eigenvector for each component. Each of the PCA components refer to the newly created, transformed images that are created by running the tool. The amount of explained variance associated with each component can be thought of as a measure of how much information content within the original multi-spectral data set that a component has. The higher this value is, the more important the component is. This same information is presented in graphical form in the *scree plot*, found at the bottom of the PCA report. The Eigenvalue is another measure of the information content of a component and the eigenvector describes the mathematical transformation (rotation coordinates) that correspond to a particular component image. 

Factor loadings are also output in a table within the PCA text report (second table). These loading values describe the correlation (i.e. *r* values) between each of the PCA components (columns) and the original images (rows). These values show you how the information contained in an image is spread among the components. An analysis of factor loadings can be reveal useful information about the data set. For example, it can help to identify groups of similar images. 

PCA is used to reduce the number of band images necessary for classification (i.e. as a data reduction technique), for noise reduction, and for change detection applications. When used as a change detection technique, the major PCA components tend to be associated with stable elements of the data set while variance due to land-cover change tend to manifest in the high-order, 'change components'. When used as a noise reduction technique, an inverse PCA is generally performed, leaving out one or more of the high-order PCA components, which account for noise variance. 

Note: the current implementation reads every raster into memory at one time. This is because of the calculation of the co-variances. As such, if the entire image stack cannot fit in memory, the tool will likely experience an out-of-memory error. This tool should be run using the `wd` flag to specify the working directory into which the component images will be written. 

### Python API

```python
def principal_component_analysis(self, rasters: List[Raster], output_html_file: str, num_components: int = 2, standardized: bool = False) -> List[Raster]:
```


---

## Print Geotiff Tags

**Function name:** `print_geotiff_tags`


This tool can be used to view the tags contained within a GeoTiff file. Viewing the tags of a GeoTiff file can be useful when trying to import the GeoTiff to different software environments. The user must specify the name of a GeoTiff file and the tag information will be output to the StdOut output stream (e.g. console). Note that tags that contain greater than 100 values will be truncated in the output. GeoKeys will also be interpreted as per the GeoTIFF specification. 

### Python API

```python
def print_geotiff_tags(self, file_name: str) :
```


---

## Quantiles

**Function name:** `quantiles`


This tool transforms values in an input raster (`input`) into quantiles. In statistics, quantiles are cut points dividing the range of a probability distribution into continuous intervals with equal probabilities, or dividing the observations in a sample in a same way. There is one fewer quantile than the number of groups created. Thus quartiles are the three cut points that will divide a dataset into four equal-sized groups. Common quantiles have special names: for instance quartile (4-quantile), quintiles (5-quantiles), decile (10-quantile), percentile (100-quantile).   

The user must specify the desired number of quantiles, q (`num_quantiles`), in the output raster (`output`). The output raster will contain q equal-sized groups with values 1 to q, indicating which quantile group each grid cell belongs to. 

### See Also

 

`histogram_equalization` 

### Python API

```python
def quantiles(self, raster: Raster, num_quantiles: int = 5) -> Raster:
```


---

## Radial Basis Function Interpolation

**Function name:** `radial_basis_function_interpolation`


This tool interpolates vector points into a raster surface using a radial basis function (RBF) scheme. 

### Python API

```python
def radial_basis_function_interpolation(self, points: Vector, field_name: str = "FID", use_z: bool = False, radius: float = 0.0, min_points: int = 0, cell_size: float = 0.0, base_raster: Raster = None, func_type: str = "thinplatespline", poly_order: str = "none", weight: float = 0.1) -> Raster:
```


---

## Radius Of Gyration

**Function name:** `radius_of_gyration`


This can be used to calculate the radius of gyration (RoG) for the polygon features within a raster image. RoG measures how far across the landscape a polygon extends its reach on average, given by the mean distance between cells in a patch (Mcgarigal et al. 2002). The radius of gyration can be considered a measure of the average distance an organism can move within a patch before encountering the patch boundary from a random starting point (Mcgarigal et al. 2002). The input raster grid should contain polygons with unique identifiers greater than zero. The user must also specify the name of the output raster file (where the radius of gyration will be assigned to each feature in the input file) and the specified option of outputting text data. 

### Python API

```python
def radius_of_gyration(self, raster: Raster) -> Tuple[Raster, str]:
```


---

## Random Field

**Function name:** `random_field`


This tool can be used to a raster image filled with random values drawn from a standard normal distribution. The values range from approximately -4.0 to 4.0, with a mean of 0 and a standard deviation of 1.0. The dimensions and georeferencing of the output random field (`output`) are based on an existing, user-specified raster grid (`base`). Note that the output field will not possess any spatial autocorrelation. If spatially autocorrelated random fields are desired, the `turning_bands_simulation` tool is more appropriate, or alternatively, the `fast_almost_gaussian_filter` tool may be used to force spatial autocorrelation onto the distribution of the `random_field` tool. 

### See Also

 

`turning_bands_simulation`, `fast_almost_gaussian_filter` 

### Python API

```python
def random_field(self, base_raster: Raster = None) -> Raster:
```


---

## Random Forest Classification Fit

**Function name:** `random_forest_classification_fit`


### Description

 

This tool performs a supervised `random forest (RF) classification` using multiple predictor rasters (`inputs`), or features, and training data (`training`). It can be used to model the spatial distribution of class data, such as land-cover type, soil class, or vegetation type. The training data take the form of an input vector Shapefile containing a set of points or polygons, for which the known class information is contained within a field (`class_field_name`) of the attribute table. Each grid cell defines a stack of feature values (one value for each input raster), which serves as a point within the multi-dimensional feature space. Random forest is an ensemble learning method that works by creating a large number (`n_trees`) of decision trees and using a majority vote to determine estimated class values. Individual trees are created using a random sub-set of predictors. This ensemble approach overcomes the tendency of individual decision trees to overfit the training data. As such, the RF method is a widely and successfully applied machine-learning method in many domains.  

Note that this function is part of a set of two tools, including `random_forest_classification_fit` and `random_forest_classification_prdict`. The  **random_forest_classificaiton_fit** tool should be used first to create the RF model and the **random_forest_classification_predict** can then be used to apply that model for prediction. The output of the *fit* tool is a byte array that is a  binary representation of the RF model. This model can then be used as the input to the *predict* tool, along with a list of input raster predictors, **which must be in the same order as those used in the *fit* tool**. The output of the *predict* tool is a classified raster. The reason that the RF workflow is split in this way is that often it is the case that you need to experiment with various input predictor sets and parameter values to create an adequate model. There is no need to generate an output classified raster during this experimentation stage, and because prediction can often be the slowest part of the RF modelling process, it is generally only performed after the final model has been identified. The binary representation of the RF-based model can be serialized (i.e., saved to a file) and then later read back into memory to serve as the input for the prediction  step of the workflow (see code example below). 

Also note that this tool is for RF-based classification. There is a similar set of *fit* and *predict tools available for performing RF-based regression, including `random_forest_regression_fit` and `random_forest_regression_predict`.  These tools are more appropriately applied to the modelling of continuous data, rather than categorical data. 

The user must specify the splitting criteria (`split_criterion`) used in training the decision trees. Options for this parameter include 'Gini', 'Entropy', and 'ClassificationError'. The model can also be adjusted based on each of the number of trees (`n_trees`), the minimum number of samples required to be at a leaf node (`min_samples_leaf`), and the minimum number of samples required to split an internal node (`min_samples_split`) parameters. 

The tool splits the training data into two sets, one for training the classifier and one for testing the model. These test data are used to calculate the overall accuracy and Cohen's kappa index of agreement, as well as to estimate the variable importance. The `test_proportion` parameter is used to set the proportion of the input training data used in model testing. For example, if `test_proportion = 0.2`, 20% of the training data will be set aside for testing, and this subset will be selected randomly. As a result of this random selection of test data, and the random selection of features used in decision tree creation, the tool is inherently stochastic, and will result in a different model each time it is run. 

Like all supervised classification methods, this technique relies heavily on proper selection of training data. Training sites are exemplar areas/points of known and representative class value (e.g. land cover type). The algorithm determines the feature signatures of the pixels within each training area. In selecting training sites, care should be taken to ensure that they cover the full range of variability within each class. Otherwise the classification accuracy will be impacted. If possible, multiple training sites should be selected for each class. It is also advisable to avoid areas near the edges of class objects (e.g. land-cover patches), where mixed pixels may impact the purity of training site values. 

After selecting training sites, the feature value distributions of each class type can be assessed using the `evaluate_training_sites` tool. In particular, the distribution of class values should ideally be non-overlapping in at least one feature dimension. 

RF, like decision trees, does not require feature scaling. That is, unlike the *k*-NN algorithm and other methods that are based on the calculation of distances in multi-dimensional space, there is no need to rescale the predictors onto a common scale prior to RF analysis. Because individual trees do not use the full set of predictors, RF is also more robust against the `curse of dimensionality` than many other machine learning methods. Nonetheless, there is still debate about whether or not it is advisable to use a large number of predictors with RF analysis and it may be better to exclude predictors that are highly correlated with others, or that do not contribute significantly to the model during the model-building phase. A dimension reduction technique such as `principal_component_analysis` can be used to transform the features into a smaller set of uncorrelated predictors. 

### Example Code

 

`import os from whitebox_workflows import WbEnvironment 

license_id = 'floating-license-id' wbe = WbEnvironment(license_id) 

try:     wbe.verbose = True     wbe.working_directory = "/path/to/data" # Read the input raster files into memory images = wbe.read_rasters(     'LC09_L1TP_018030_20220614_20220615_02_T1_B2.TIF',     'LC09_L1TP_018030_20220614_20220615_02_T1_B3.TIF',     'LC09_L1TP_018030_20220614_20220615_02_T1_B4.TIF',     'LC09_L1TP_018030_20220614_20220615_02_T1_B5.TIF' )  # Read the input training polygons into memory training_data = wbe.read_vector('training_data.shp')  # Train the model model = wbe.random_forest_classification_fit(     images,      training_data,      class_field_name = 'CLASS',      split_criterion = "Gini",      n_trees = 50,       min_samples_leaf = 1,      min_samples_split = 2,      test_proportion = 0.2 )  # Example of how to serialize the model, i.e., save the model, which is just binary data print('Saving the model to file...') file_path = os.path.join(wbe.working_directory, "rf_model.bin") with open(file_path, "wb") as file:     file.write(bytearray(model))  # Example of how to deserialize the model, i.e. read the model model = [] with open(file_path, mode='rb') as file:     model = list(file.read())  # Use the model to predict rf_class_image = wbe.random_forest_classification_predict(images, model)  wbe.write_raster(rf_class_image, 'rf_classification.tif', compress=True)  print('All done!') ` 

except Exception as e:     print("The error raised is: ", e) finally:     wbe.check_in_license(license_id) 

 

### See Also

 

`random_forest_classification_predict`, `random_forest_regression_fit`, `random_forest_regression_predict`, `knn_classification`, `svm_classification`, `parallelepiped_classification`, `evaluate_training_sites` 

### Python API

```python
def random_forest_classification_fit(self, input_rasters: List[Raster], training_data: Vector, class_field_name: str, split_criterion: str = "gini", n_trees: int = 500, min_samples_leaf: int = 1, min_samples_split: int = 2, test_proportion: float = 0.2) -> List[int]:
```


---

## Random Forest Classification Predict

**Function name:** `random_forest_classification_predict`


Note this tool is part of a `WhiteboxTools extension product`. Please visit  `Whitebox Geospatial Inc.` for information about purchasing a license  activation key (`https://www.whiteboxgeo.com/extension-pricing/`).  

This tool applies a pre-built `random forest (RF) classification`  model trained using multiple predictor rasters (`input_rasters`), or features, and training data to predict a spatial distribution. This function is part of a set of two tools, including  `random_forest_classification_fit` and `random_forest_classification_prdict`. The  **random_forest_classification_fit** tool should be used first to create the RF model and the **random_forest_classification_predict** can then be used to apply that model for prediction. The output of the *fit* tool is a byte array that is a  binary representation of the RF model. This model can then be used as the input to the *predict* tool, along with a list of input raster predictors, which must be in the same order as those used in the *fit* tool (see below). The output of the *predict* tool is a classified raster. The reason that the RF workflow is split in this way is that often it is the case that you need to experiment with various input predictor sets and parameter values to create an adequate model. There is no need to generate an output classified raster during this experimentation stage, and because prediction can often be the slowest part of the RF modelling process, it is generally only performed after the final model has been identified. The binary representation of the RF-based model can be serialized (i.e., saved to a file) and then later read back into memory to serve as the input for the prediction  step of the workflow (see code example below).  

**Note**: it is very important that the order of feature rasters is the same for both fitting the model and using the model for prediction. It is possible to use a model fitted to one data set to  make preditions for another data set, however, the set of feature reasters specified to the prediction tool must be input in the same sequence used for building the model. For example, one may  train a RF classifer on one set of multi-spectral satellite imagery and then apply that model to classify  a different imagery scene, but the image band sequence must be the same for the Fit/Predict tools otherwise inaccurate predictions will result.  

### Example Code

 

`import os from whitebox_workflows import WbEnvironment 

license_id = 'floating-license-id' wbe = WbEnvironment(license_id) 

try:     wbe.verbose = True     wbe.working_directory = "/path/to/data" # Read the input raster files into memory images = wbe.read_rasters(     'LC09_L1TP_018030_20220614_20220615_02_T1_B2.TIF',     'LC09_L1TP_018030_20220614_20220615_02_T1_B3.TIF',     'LC09_L1TP_018030_20220614_20220615_02_T1_B4.TIF',     'LC09_L1TP_018030_20220614_20220615_02_T1_B5.TIF' )  # Read the input training polygons into memory training_data = wbe.read_vector('training_data.shp')  # Train the model model = wbe.random_forest_classification_fit(     images,      training_data,      class_field_name = 'CLASS',      split_criterion = "Gini",      n_trees = 50,       min_samples_leaf = 1,      min_samples_split = 2,      test_proportion = 0.2 )  # Example of how to serialize the model, i.e., save the model, which is just binary data print('Saving the model to file...') file_path = os.path.join(wbe.working_directory, "rf_model.bin") with open(file_path, "wb") as file:     file.write(bytearray(model))  # Example of how to deserialize the model, i.e. read the model model = [] with open(file_path, mode='rb') as file:     model = list(file.read())  # Use the model to predict rf_class_image = wbe.random_forest_classification_predict(images, model)  wbe.write_raster(rf_class_image, 'rf_classification.tif', compress=True)  print('All done!') ` 

except Exception as e:     print("The error raised is: ", e) finally:     wbe.check_in_license(license_id) 

 

### See Also

 

`random_forest_classification_fit`, `random_forest_regression_fit`, `random_forest_regression_predict`, `knn_classification`, `svm_classification`, `parallelepiped_classification`, `evaluate_training_sites` 

### Python API

```python
def random_forest_classification_predict(self, input_rasters: List[Raster], model_bytes: List[int]) -> Raster:
```


---

## Random Forest Regression Fit

**Function name:** `random_forest_regression_fit`


### Description

 

This function performs a supervised `random forest (RF) regression analysis`  using multiple predictor rasters (`input_rasters`), or features, and training data (`training_data`).  The training data take the form of an input vector Shapefile containing a set of points, for  which the known outcome information is contained within a field (`field_name`) of the attribute table. Each  grid cell defines a stack of feature values (one value for each input raster), which serves as a point  within the multi-dimensional feature space. 

Note that this function is part of a set of two tools, including `random_forest_regression_fit` and `random_forest_regression_prdict`. The  **random_forest_classificaiton_fit** tool should be used first to create the RF model and the **random_forest_regression_predict** can then be used to apply that model for prediction. The output of the *fit* tool is a byte array that is a  binary representation of the RF model. This model can then be used as the input to the *predict* tool, along with a list of input raster predictors, **which must be in the same order as those used in the *fit* tool**. The output of the *predict* tool is a continous raster. The reason that the RF workflow is split in this way is that often it is the case that you need to experiment with various input predictor sets and parameter values to create an adequate model. There is no need to generate an output raster during this experimentation stage. Because prediction can often be the slowest part of the RF modelling process, it is generally only performed after the final model has been identified. The binary representation of the RF-based model can be serialized (i.e., saved to a file) and then later read back into memory to serve as the input for the prediction  step of the workflow (see code example below). 

Also note that this tool is for RF-based regression analysis. There is a similar set of *fit* and *predict tools available for performing RF-based classification, including `random_forest_classification_fit` and `random_forest_classification_predict`.  These tools are more appropriately applied to the modelling of categorical data, rather than continuous data.  

**Note**: it is very important that the order of feature rasters is the same for both fitting the model and using the model for prediction. It is possible to use a model fitted to one data set to  make preditions for another data set, however, the set of feature reasters specified to the prediction tool must be input in the same sequence used for building the model. For example, one may  train a RF regressor on one set of land-surface parameters and then apply that model to predict the spatial  distribution of a soil property on a land-surface parameter stack derived for a different landscape, but the  image band sequence must be the same for the Fit/Predict tools otherwise inaccurate predictions will result.  

Random forest is an ensemble learning method that works by  creating a large number (`n_trees`) of decision trees and using an averaging of each tree to determine estimated  outcome values. Individual trees are created using a random sub-set of predictors. This ensemble approach  overcomes the tendency of individual decision trees to overfit the training data. As such, the RF method is a widely and successfully applied machine-learning method in many domains. 

Users must specify the number of trees (`n_trees`), the minimum number of samples required to  be at a leaf node (`min_samples_leaf`), and the minimum number of samples required to split an internal  node (`min_samples_split`) parameters, which determine the characteristics of the resulting model. 

The function splits the training data into two sets, one for training the model and one for testing the prediction. These test data are used to calculate the regression accuracy statistics, as  well as to estimate the variable importance. The `test_proportion` parameter is used to set the proportion of the input training data used in model testing. For example, if  `test_proportion = 0.2`, 20% of the training data will be set aside for testing, and this subset will be selected randomly. As a result of this random selection of test data, as well as the  randomness involved in establishing the individual decision trees, the tool in inherently stochastic, and will result in a different model each time it is run. 

RF, like decision trees, does not require feature scaling. That is, unlike the *k*-NN algorithm and other methods that are based on the calculation of distances in multi-dimensional space, there is no need to rescale the predictors onto a common scale prior to RF analysis. Because individual trees do not use the full set of predictors, RF is also more robust against the `curse of dimensionality` than many other machine learning methods. Nonetheless, there is still debate about whether or not it is advisable to use a large number of predictors with RF analysis and it may be better to exclude  predictors that are highly correlated with others, or that do not contribute significantly to the model during the model-building phase. A dimension reduction technique such as  `principal_component_analysis` can be used to transform the features into a smaller set of uncorrelated predictors. 

For a video tutorial on how to use the `RandomForestRegression` tool, see  `this YouTube video`. 

### Code Example

 

`import os from whitebox_workflows import WbEnvironment 

license_id = 'floating-license-id' wbe = WbEnvironment(license_id) 

try:     wbe.verbose = True     wbe.working_directory = "/path/to/data" # Read the input raster files into memory images = wbe.read_rasters(     'DEV.tif',     'profile_curv.tif',     'tan_curv.tif',     'slope.tif' )  # Read the input training polygons into memory training_data = wbe.read_vector('Ottawa_soils_data.shp')  # Train the model model = wbe.random_forest_regression_fit(     images,      training_data,      field_name = 'Sand',     n_trees = 50,       min_samples_leaf = 1,      min_samples_split = 2,      test_proportion = 0.2 )  # Example of how to serialize the model, i.e., save the model, which is just binary data print('Saving the model to file...') file_path = os.path.join(wbe.working_directory, "rf_model.bin") with open(file_path, "wb") as file:     file.write(bytearray(model))  # Example of how to deserialize the model, i.e. read the model model = [] with open(file_path, mode='rb') as file:     model = list(file.read())  # Use the model to predict rf_image = wbe.random_forest_regression_predict(images, model)  wbe.write_raster(rf_image, 'rf_regression.tif', compress=True)  print('All done!') ` 

except Exception as e:     print("The error raised is: ", e) finally:     wbe.check_in_license(license_id) 

 

### See Also

 

`random_forest_regression_predict`, `random_forest_classification_fit`, `random_forest_classification_predict`, `knn_classification`, `svm_classification`, `parallelepiped_classification`, `evaluate_training_sites` 

### Python API

```python
def random_forest_regression_fit(self, input_rasters: List[Raster], training_data: Vector, field_name: str, n_trees: int = 500, min_samples_leaf: int = 1, min_samples_split: int = 2, test_proportion: float = 0.2) -> List[int]:
```


---

## Random Forest Regression Predict

**Function name:** `random_forest_regression_predict`


Note this tool is part of a `WhiteboxTools extension product`. Please visit  `Whitebox Geospatial Inc.` for information about purchasing a license  activation key (`https://www.whiteboxgeo.com/extension-pricing/`).  

This tool applies a pre-built `random forest (RF) regression`  model trained using multiple predictor rasters, or features (`input_rasters`), and training data to predict a spatial distribution. This function is part of a set of two tools, including  `random_forest_regression_fit` and `random_forest_regression_prdict`. The  **random_forest_regression_fit** function should be used first to create the RF model and the **random_forest_regression_predict** can then be used to apply that model for prediction. The output of the *fit* tool is a byte array that is a  binary representation of the RF model. This model can then be used as the input to the *predict* tool, along with a list of input raster predictors, which must be in the same order as those used in the *fit* tool (see below). The output of the *predict* tool is a raster. The reason that the RF workflow is split in this way is that often it is the case that you need to experiment with various input predictor sets and parameter values to create an adequate model. There is no need to generate an output classified raster during this experimentation stage, and because prediction can often be the slowest part of the RF modelling process, it is generally only performed after the final model has been identified. The binary representation of the RF-based model can be serialized (i.e., saved to a file) and then later read back into memory to serve as the input for the prediction  step of the workflow (see code example below).  

**Note**: it is very important that the order of feature rasters is the same for both fitting the model and using the model for prediction. It is possible to use a model fitted to one data set to  make preditions for another data set, however, the set of feature reasters specified to the prediction tool must be input in the same sequence used for building the model. For example, one may  train a RF classifer on one set of multi-spectral satellite imagery and then apply that model to classify  a different imagery scene, but the image band sequence must be the same for the Fit/Predict tools otherwise inaccurate predictions will result.  

### Code Example

 

`import os from whitebox_workflows import WbEnvironment 

license_id = 'floating-license-id' wbe = WbEnvironment(license_id) 

try:     wbe.verbose = True     wbe.working_directory = "/path/to/data" # Read the input raster files into memory images = wbe.read_rasters(     'DEV.tif',     'profile_curv.tif',     'tan_curv.tif',     'slope.tif' )  # Read the input training polygons into memory training_data = wbe.read_vector('Ottawa_soils_data.shp')  # Train the model model = wbe.random_forest_regression_fit(     images,      training_data,      field_name = 'Sand',     n_trees = 50,       min_samples_leaf = 1,      min_samples_split = 2,      test_proportion = 0.2 )  # Example of how to serialize the model, i.e., save the model, which is just binary data print('Saving the model to file...') file_path = os.path.join(wbe.working_directory, "rf_model.bin") with open(file_path, "wb") as file:     file.write(bytearray(model))  # Example of how to deserialize the model, i.e. read the model model = [] with open(file_path, mode='rb') as file:     model = list(file.read())  # Use the model to predict rf_image = wbe.random_forest_regression_predict(images, model)  wbe.write_raster(rf_image, 'rf_regression.tif', compress=True)  print('All done!') ` 

except Exception as e:     print("The error raised is: ", e) finally:     wbe.check_in_license(license_id) 

 

### See Also

 

`random_forest_regression_fit`, `random_forest_classification_fit`, `random_forest_classification_predict`, `knn_classification`, `svm_classification`, `parallelepiped_classification`, `evaluate_training_sites` 

### Python API

```python
def random_forest_regression_predict(self, input_rasters: List[Raster], model_bytes: List[int]) -> Raster:
```


---

## Random Sample

**Function name:** `random_sample`


This tool can be used to create a random sample of grid cells. The user specifies the base raster file, which is used to determine the grid dimensions and georeference information for the output raster, and the number of sample random samples (n). The output grid will contain n non-zero grid cells, randomly distributed throughout the raster grid, and a background value of zero. This tool is useful when performing statistical analyses on raster images when you wish to obtain a random sample of data. 

Only valid, non-nodata, cells in the base raster will be sampled. 

### Python API

```python
def random_sample(self, base_raster: Raster = None, num_samples: int = 1000) -> Raster:
```


---

## Raster Area

**Function name:** `raster_area`


This tools estimates the area of each category, polygon, or patch in an input raster. The input raster must be categorical in data scale. Rasters with floating-point cell values are not good candidates for an area analysis. The user must specify whether the output is given in `grid cells` or `map units` (`units`). Map Units are physical units, e.g. if the rasters's scale is in metres, areas will report in square-metres. Notice that square-metres can be converted into hectares by dividing by 10,000 and into square-kilometres by dividing by 1,000,000. If the input raster is in geographic coordinates (i.e. latitude and longitude) a warning will be issued and areas will be estimated based on per-row calculated degree lengths. 

The tool can be run with a raster output (`output`), a text output (`out_text`), or both. If niether outputs are specified, the tool will automatically output a raster named `area.tif`. 

Zero values in the input raster may be excluded from the area analysis if the `zero_back` flag is used. 

To calculate the area of vector polygons, use the `polygon_area` tool instead. 

### See Also

 

`polygon_area`, `raster_histogram` 

### Python API

```python
def raster_area(self, raster: Raster, units: str = "map units", zero_background: bool = False) -> Tuple[Raster, str]:
```


---

## Raster Calculator

**Function name:** `raster_calculator`


The `raster_calculator` tool can be used to perform a complex mathematical operations on one or more input raster images on a cell-to-cell basis. The user inputs an `expression` and a list of input rasters (`input_rasters`),  specified in the same order as the rasters contained within the statement. Rasters are treated like variables (that change value with each grid cell) and are specified within the statement as arbitrarily named variables contained within either double or single quotation marks (e.g. "DEM" > 500.0). The order of raster variables must match the order of rasters within the `input_rasters` list.**Note, all input rasters must share the same number of rows and columns and spatial extent. Use the `resample` tool if this is not the case to convert the one raster's grid resolution to the others. 

### Example

 

`(band3, band4) = wbe.read_rasters('band3.tif', 'band4.tif') result = wbe.raster_calculator("('nir' - 'red') / ('nir' + 'red')", [band4, band3]) wbe.write_raster(result, 'result.tif', True) ` The mathematical expression supports all of the standard algebraic unary and binary operators (+ - * / ^ %),  as well as comparisons (< <= == != >= >) and logical operators (&& ||) with short-circuit support. The order of operations, from highest to lowest is as follows. 

Listed in order of precedence:  OrderSymbolDescription (Highest Precedence)^Exponentiation %Modulo /Division *Multiplication -Subtraction +Addition == != = >Comparisons (all have equal precedence) && andLogical AND with short-circuit (Lowest Precedence)&#124;&#124; orLogical OR with short-circuit    

Several common mathematical functions are also available for use in the input statement. For example: 

` * log(base=10, val) -- Logarithm with optional 'base' as first argument.  If not provided, 'base' defaults to '10'.  Example: log(100) + log(e(), 100) 
 
- e()  -- Euler's number (2.718281828459045) 
-  

pi() -- π (3.141592653589793)  
-  

int(val)  
- ceil(val) 
- floor(val) 
-  

round(modulus=1, val) -- Round with optional 'modulus' as first argument.      Example: round(1.23456) == 1 && round(0.001, 1.23456) == 1.235  
-  

abs(val)  
-  

sign(val)  
-  

min(val, ...) -- Example: min(1, -2, 3, -4) == -4  
-  

max(val, ...) -- Example: max(1, -2, 3, -4) == 3  
-  

sin(radians)    * asin(val)  
- cos(radians)    * acos(val) 
- tan(radians)    * atan(val) 
- sinh(val)       * asinh(val) 
- cosh(val)       * acosh(val) 
- tanh(val)       * atanh(val) ` Notice that the constants pi and e must be specified as functions, `pi()` and `e()`. A number of global variables  are also available to build conditional statements. These include the following: 
 

**Special Variable Names For Use In Conditional Statements:**  NameDescription `nodata`An input raster's NoData value. `null`Same as `nodata`. `minvalue`An input raster's minimum value. `maxvalue`An input raster's maximum value. `rows`The input raster's number of rows. `columns`The input raster's number of columns. `row`The grid cell's row number. `column`The grid cell's column number. `rowy`The row's y-coordinate. `columnx`The column's x-coordinate. `north`The input raster's northern coordinate. `south`The input raster's southern coordinate. `east`The input raster's eastern coordinate. `west`The input raster's western coordinate. `cellsizex`The input raster's grid resolution in the x-direction. `cellsizey`The input raster's grid resolution in the y-direction. `cellsize`The input raster's average grid resolution.   

The special variable names are case-sensitive. If there are more than one raster inputs used in the statement, the functional forms of the `nodata`, `null`, `minvalue`, and `maxvalue` variables should be used, e.g.  `nodata("InputRaster")`, otherwise the value is assumed to specify the attribute of the first raster in the  statement. The following are examples of valid statements: 

` "raster" != 300.0 

"raster" >= (minvalue + 35.0) 

("raster1" >= 25.0) && ("raster2" <= 75.0) -- Evaluates to 1 where both conditions are true. 

tan("raster" * pi() / 180.0) > 1.0 

"raster" == nodata ` Any grid cell in the input rasters containing the NoData value will be assigned NoData in the output raster,  unless a NoData grid cell value allows the statement to evaluate to True (i.e. the mathematical expression  includes the `nodata` value). 

### See Also

 

`ConditionalEvaluation` 

### Python API

```python
def raster_calculator(self, expression: str, input_rasters: List[Raster]) -> Raster:
```


---

## Raster Cell Assignment

**Function name:** `raster_cell_assignment`


This tool can be used to create a new raster with the same coordinates and dimensions (i.e. rows and columns) as an existing base image. Grid cells in the new raster will be assigned either the row or column number or the x- or y-coordinate, depending on the selected option (`assign` flag). The user must also specify the name of the base image (`input`). 

### See Also

 

`NewRasterFromBase` 

### Python API

```python
def raster_cell_assignment(self, raster: Raster, what_to_assign: str = "column") -> Raster:
```


---

## Raster Histogram

**Function name:** `raster_histogram`


This tool produces a histogram (i.e. a frequency distribution graph) for the values contained within an input raster file (`input`). The histogram will be embedded within an output (`output_html_file`) HTML file, which should be automatically displayed after the tool has completed. The user may optionally specify the number of bins (`num_bins`) used in the histogram. If unspecified, this is calculated as: 

`num_bins = ((rows * columns)).log2().ceil() + 1` 

### See Also

 

`attribute_histogram` 

### Python API

```python
def raster_histogram(self, raster: Raster, output_html_file: str) -> None:
```


---

## Raster Perimeter

**Function name:** `raster_perimeter`


This tool can be used to measure the length of the perimeter of polygon features in a raster layer. The user must specify the name of the input raster file (`input`) and optionally an output raster (`output`), which is the raster layer containing the input features assigned the perimeter length. The user may also optionally choose to output text data (`out_text`). Raster-based perimeter estimation uses the accurate, anti-aliasing algorithm of Prashker (2009). 

The input file must be of a categorical data type, containing discrete polygon features that have been assigned unique identifiers. Such rasters are often created by region-grouping (`clump`) a classified raster. 

### Reference

 

Prashker, S. (2009) An anti-aliasing algorithm for calculating the perimeter of raster polygons. Geotec, Ottawa and Geomtics Atlantic, Wolfville, NS. 

### See Also

 

`raster_area`, `clump` 

### Python API

```python
def raster_perimeter(self, raster: Raster, units: str = "map units", zero_background: bool = False) -> Tuple[Raster, str]:
```


---

## Raster Summary Stats

**Function name:** `raster_summary_stats`


This tool outputs distribution summary statistics for input raster images (`input`). The distribution statistics include the raster minimum, maximum, range, total, mean, variance, and standard deviation. These summary statistics are output to the system `stdout`. 

The following is an example of the summary report:  

*********************************  * Welcome to RasterSummaryStats *  *********************************  Reading data... 

Number of non-nodata grid cells: 32083559  Number of nodata grid cells: 3916441  Image minimum: 390.266357421875  Image maximum: 426.0322570800781  Image range: 35.765899658203125  Image total: 13030334843.332886  Image average: 406.13745012929786  Image variance: 31.370027239143383  Image standard deviation: 5.600895217654351   

### See Also

 

`raster_histogram`, `zonal_statistics` 

### Python API

```python
def raster_summary_stats(self, input: Raster) -> str:
```


---

## Reciprocal

**Function name:** `reciprocal`


This tool creates a new raster (`output`) in which each grid cell is equal to one divided by the grid cell values in the input raster image (`input`). **NoData** values in the input image will be assigned **NoData** values in the output image. 

### Python API

```python
def reciprocal(self, raster: Raster) -> Raster:
```


---

## Rescale Value Range

**Function name:** `rescale_value_range`


### Python API

```python
def rescale_value_range(self, raster: Raster, out_min_val: float, out_max_val: float, clip_min: float = float('inf'), clip_max: float = float('-inf')) -> Raster:
```


---

## Root Mean Square Error

**Function name:** `root_mean_square_error`


This tool calculates the root-mean-square-error (RMSE) or root-mean-square-difference (RMSD) from two input rasters. If the two input rasters possess the same number of rows and columns, the RMSE is calucated on a cell-by-cell basis, otherwise bilinear resampling is used. In addition to RMSE, the tool also reports other common accuracy statistics including the mean verical error, the 95% confidence limit (RMSE x 1.96), and the 90% linear error (LE90), which is the 90% percentile of the residuals between two raster surfaces. The LE90 is the most robust of the reported accuracy statistics when the residuals are non-Gaussian. The LE90 requires sorting the residual values, which can be a relatively slow operation for larger rasters. 

### See Also

 

`paired_sample_t_test`, `wilcoxon_signed_rank_test` 

### Python API

```python
def root_mean_square_error(self, input: Raster, reference: Raster) -> str:
```


---

## Round

**Function name:** `round`


Experimental

Rounds each raster cell to the nearest integer.

raster math round

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply round transform to each non-nodata cell.*
`wbe.round(input='dem.tif', output='round_dem.tif')`


---

## Shape Complexity Index Raster

**Function name:** `shape_complexity_index_raster`


This tools calculates a type of shape complexity index for raster objects. The index is equal to the average number of intersections of the group of vertical and horizontal transects passing through an object. Simple objects will have a shape complexity index of 1.0 and more complex shapes, including those containing numerous holes or are winding in shape, will have higher index values. Objects in the input raster (`input`) are designated by their unique identifiers. Identifier values should be positive, non-zero whole numbers. 

### See Also

 

`ShapeComplexityIndex`, `boundary_shape_complexity` 

### Python API

```python
def shape_complexity_index_raster(self, raster: Raster) -> Raster:
```


---

## Sieve

**Function name:** `sieve`


The sieve function removes individual objects in a class map that are less than a threshold area, in grid cells. Pixels contained within the removed small polygons will be replaced with the nearest remaining class value. This operation is common when generalizing class maps, e.g. those derived from an image classification. Thus, this tool provides a similar function to the `generalize_classified_raster` and `generalize_with_similarity` functions. 

### See Also:

 

`generalize_classified_raster`, `generalize_with_similarity` 

### Python API

```python
def sieve(self, input_raster: Raster, threshold: int = 1, zero_background: bool = False) -> Raster:
```


---

## Sin

**Function name:** `sin`


Experimental

Computes the sine of each raster cell value.

raster math sin

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply sin transform to each non-nodata cell.*
`wbe.sin(input='dem.tif', output='sin_dem.tif')`


---

## Sinh

**Function name:** `sinh`


Experimental

Computes the hyperbolic sine of each raster cell.

raster math sinh

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply sinh transform to each non-nodata cell.*
`wbe.sinh(input='dem.tif', output='sinh_dem.tif')`


---

## Sqrt

**Function name:** `sqrt`


Experimental

Computes the square-root of each raster cell.

raster math sqrt

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply sqrt transform to each non-nodata cell.*
`wbe.sqrt(input='dem.tif', output='sqrt_dem.tif')`


---

## Square

**Function name:** `square`


Experimental

Squares each raster cell value.

raster math square

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply square transform to each non-nodata cell.*
`wbe.square(input='dem.tif', output='square_dem.tif')`


---

## Tan

**Function name:** `tan`


Experimental

Computes the tangent of each raster cell value.

raster math tan

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply tan transform to each non-nodata cell.*
`wbe.tan(input='dem.tif', output='tan_dem.tif')`


---

## Tanh

**Function name:** `tanh`


Experimental

Computes the hyperbolic tangent of each raster cell.

raster math tanh

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply tanh transform to each non-nodata cell.*
`wbe.tanh(input='dem.tif', output='tanh_dem.tif')`


---

## TIN Interpolation

**Function name:** `tin_interpolation`


Creates a raster grid based on a triangular irregular network (TIN) fitted to vector points and linear interpolation within each triangular-shaped plane. The TIN creation algorithm is based on `Delaunay triangulation`. 

The user must specify the attribute field containing point values (`field`). Alternatively, if the input Shapefile contains z-values, the interpolation may be based on these values (`use_z`). Either an output grid resolution (`cell_size`) must be specified or alternatively an existing base file (`base`) can be used to determine the output raster's (`output`) resolution and spatial extent. Natural neighbour interpolation generally produces a satisfactorily smooth surface within the region of data points but can produce spurious breaks in the surface outside of this region. Thus, it is recommended that the output surface be clipped to the convex hull of the input points (`clip`). 

### See Also

 

`lidar_tin_gridding`, `construct_vector_tin`, `natural_neighbour_interpolation` 

### Python API

```python
def tin_interpolation(self, points: Vector, field_name: str = "FID", use_z: bool = False, cell_size: float = 0.0, base_raster: Raster = None, max_triangle_edge_length: float = float('inf')) -> Raster:
```


---

## To Degrees

**Function name:** `to_degrees`


Experimental

Converts each raster cell from radians to degrees.

raster math to_degrees

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply to_degrees transform to each non-nodata cell.*
`wbe.to_degrees(input='dem.tif', output='to_degrees_dem.tif')`


---

## To Radians

**Function name:** `to_radians`


Experimental

Converts each raster cell from degrees to radians.

raster math to_radians

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply to_radians transform to each non-nodata cell.*
`wbe.to_radians(input='dem.tif', output='to_radians_dem.tif')`


---

## Trend Surface

**Function name:** `trend_surface`


This tool can be used to interpolate a trend surface from a raster image. The technique uses a polynomial, least-squares regression analysis. The user must specify the name of the input raster file. In addition, the user must specify the polynomial order (1 to 10) for the analysis. A first-order polynomial is a planar surface with no curvature. As the polynomial order is increased, greater flexibility is allowed in the fitted surface. Although polynomial orders as high as 10 are accepted, numerical instability in the analysis often creates artifacts in trend surfaces of orders greater than 5. The operation will display a text report on completion, in addition to the output raster image. The report will list each of the coefficient values and the r-square value. Note that the entire raster image must be able to fit into computer memory, limiting the use of this tool to relatively small rasters. The Trend Surface (Vector Points) tool can be used instead if the input data is vector points contained in a shapefile. 

Numerical stability is enhanced by transforming the x, y, z data by their minimum values before performing the regression analysis. These transform parameters are also reported in the output report. 

### Python API

```python
def trend_surface(self, raster: Raster, output_html_file: str, polynomial_order: int = 1) -> Raster:
```


---

## Trend Surface Vector Points

**Function name:** `trend_surface_vector_points`


This tool can be used to interpolate a trend surface from a vector points file. The technique uses a polynomial, least-squares regression analysis. The user must specify the name of the input shapefile, which must be of a 'Points' base VectorGeometryType and select the attribute in the shapefile's associated attribute table for which to base the trend surface analysis. The attribute must be numerical. In addition, the user must specify the polynomial order (1 to 10) for the analysis. A first-order polynomial is a planar surface with no curvature. As the polynomial order is increased, greater flexibility is allowed in the fitted surface. Although polynomial orders as high as 10 are accepted, numerical instability in the analysis often creates artifacts in trend surfaces of orders greater than 5. The operation will display a text report on completion, in addition to the output raster image. The report will list each of the coefficient values and the r-square value. The Trend Surface tool can be used instead if the input data is a raster image. 

Numerical stability is enhanced by transforming the x, y, z data by their minimum values before performing the regression analysis. These transform parameters are also reported in the output report. 

### Python API

```python
def trend_surface_vector_points(self, input: Vector, cell_size: float, output_html_file: str, field_name: str = "FID", polynomial_order: int = 1) -> Raster:
```


---

## Truncate

**Function name:** `truncate`


Experimental

Truncates each raster cell value to its integer part.

raster math truncate

### Parameters

NameDescriptionRequiredDefault
`input`Input raster file path.Required`input.tif`
`output`Optional output raster file path. If omitted, the result is stored in memory.Optional`output.tif`

### Examples

*Apply truncate transform to each non-nodata cell.*
`wbe.truncate(input='dem.tif', output='truncate_dem.tif')`


---

## Turning Bands Simulation

**Function name:** `turning_bands_simulation`


This tool can be used to create a random field using the turning bands algorithm. The user must specify the name of a base raster image (`base`) from which the output raster will derive its geographical information, dimensions (rows and columns), and other information. In addition, the range (`range`), in x-y units, must be specified. The range determines the correlation length of the resulting field. For a good description of how the algorithm works, see Carr (2002). The turning bands method creates a number of 1-D simulations (called bands) and fuses these together to create a 2-D error field. There is no natural stopping condition in this process, so the user must specify the number of bands to create (`iterations`). The default value of 1000 iterations is reasonable. The fewer iterations used, the more prevalent the 1-D simulations will be in the output error image, effectively creating artifacts. Run time increases with the number of iterations. 

Turning bands simulation is a commonly applied technique in Monte Carlo style simulations of uncertainty. As such, it is frequently run many times during a simulation (often 1000s of times). When this is the case, algorithm performance and efficiency are key considerations. One alternative method to efficiently generate spatially autocorrelated random fields is to apply the `fast_almost_gaussian_filter` tool to the output of the `random_field` tool. This can be used to generate a random field with the desired spatial characteristics and frequency distribution. This is the alternative approach used by the `stochastic_depression_analysis` tool. 

### Reference

 

Carr, J. R. (2002). Data visualization in the geosciences. Upper Saddle River, NJ: Prentice Hall. pp. 267. 

### See Also

 

`random_field`, `fast_almost_gaussian_filter`, `stochastic_depression_analysis` 

### Python API

```python
def turning_bands_simulation(self, base_raster: Raster = None, range: float = 1.0, iterations: int = 1000) -> Raster:
```


---

## Two Sample KS Test

**Function name:** `two_sample_ks_test`


This tool will perform a two-sample Kolmogorov-Smirnov (K-S) test to evaluate whether a significant statistical difference exists between the frequency distributions of two rasters. The null hypothesis is that both samples come from a population with the same distribution. Note that this test evaluates the two input rasters for differences in their overall distribution shape, with no assumption of normality. If there is need to compare the per-pixel differences between two input rasters, a paired-samples test such as the `paired_sample_t_test` or the non-parametric `wilcoxon_signed_rank_test` should be used instead. 

The user must specify the name of the two input raster images (`input1` and `input2`) and the output report HTML file (`output`). The test can be performed optionally on the entire image or on a random sub-sample of pixel values of a user-specified size (`num_samples`). In evaluating the significance of the test, it is important to keep in mind that given a sufficiently large sample, extremely small and non-notable differences can be found to be statistically significant. Furthermore statistical significance says nothing about the practical significance of a difference. 

### See Also

 

`KSTestForNormality`, `paired_sample_t_test`, `wilcoxon_signed_rank_test` 

### Python API

```python
def two_sample_ks_test(self, raster1: Raster, raster2: Raster, output_html_file: str, num_samples: int) -> None:
```


---

## Wilcoxon Signed Rank Test

**Function name:** `wilcoxon_signed_rank_test`


This tool will perform a Wilcoxon signed-rank test to evaluate whether a significant statistical difference exists between the two rasters. The Wilcoxon signed-rank test is often used as a non-parametric equivalent to the paired-samples Student's t-test, and is used when the distribution of sample difference values between the paired inputs is non-Gaussian. The null hypothesis of this test is that difference between the sample pairs follow a symmetric distribution around zero. i.e. that the median difference between pairs of observations is zero. 

The user must specify the name of the two input raster images (`input1` and `input2`) and the output report HTML file (`output`). The test can be performed optionally on the entire image or on a random sub-sample of pixel values of a user-specified size (`num_samples`). In evaluating the significance of the test, it is important to keep in mind that given a sufficiently large sample, extremely small and non-notable differences can be found to be statistically significant. Furthermore statistical significance says nothing about the practical significance of a difference. Note that cells with a difference of zero are excluded from the ranking and tied difference values are assigned their average rank values. 

### See Also

 

`paired_sample_test`, `two_sample_ks_test` 

### Python API

```python
def wilcoxon_signed_rank_test(self, raster1: Raster, raster2: Raster, output_html_file: str, num_samples: int) -> None:
```


---

## Z Scores

**Function name:** `z_scores`


This tool will transform the values in an input raster image (`input`) into `z-scores`. Z-scores are also called standard scores, normal scores, or z-values. A z-score is a dimensionless quantity that is calculated by subtracting the mean from an individual raw value and then dividing the difference by the standard deviation. This conversion process is called *standardizing* or *normalizing* and the result is sometimes referred to as a standardized variable. The mean and standard deviation are estimated using all values in the input image except for NoData values. The input image should not have a Boolean or categorical data scale, i.e. it should be on a continuous scale. 

### See Also

 

`cumulative_distribution` 

### Python API

```python
def z_scores(self, raster: Raster) -> Raster:
```


---

## Zonal Statistics

**Function name:** `zonal_statistics`


This tool can be used to extract common descriptive statistics associated with the distribution of some underlying data raster based on feature units defined by a feature definition raster. For example, this tool can be used to measure the maximum or average slope gradient (data image) for each of a group of watersheds (feature definitions). Although the data raster can contain any type of data, the feature definition raster must be categorical, i.e. it must define area entities using integer values. 

The `stat` parameter can take the values, 'mean', 'median', 'minimum', 'maximum', 'range', 'standard deviation', or 'total'. 

If an output image name is specified, the tool will assign the descriptive statistic value to each of the spatial entities defined in the feature definition raster. If text output is selected, an HTML table will be output, which can then be readily copied into a spreadsheet program for further analysis. This is a very powerful and useful tool for creating numerical summary data from spatial data which can then be interrogated using statistical analyses. At least one output type (image or text) must be specified for the tool to operate. 

NoData values in either of the two input images are ignored during the calculation of the descriptive statistic. 

### See Also

 

`raster_summary_stats` 

### Python API

```python
def zonal_statistics(self, data_raster: Raster, feature_definitions_raster: Raster, stat_type: str = "mean") -> Tuple[Raster, str]:
```
