# General Tools


---

## Assess Route

**Function name:** `assess_route`


PROExperimental

Segments route lines and evaluates per-segment terrain metrics from a DEM.

geomorphometry route vector legacy-port


---

## Breakline Mapping

**Function name:** `breakline_mapping`


PROExperimental

Maps breaklines by thresholding log-transformed curvedness and vectorizing thinned linear features.

geomorphometry breaklines curvature vectorization legacy-port

### Parameters

NameDescriptionRequiredDefault
`input`Input DEM raster path or typed raster object.Required`dem.tif`
`threshold`Minimum log-curvedness threshold used for breakline extraction (default 0.8).Optional`0.8`
`min_length`Minimum output line length in grid cells (default 3).Optional`3`
`output`Optional output vector path (default temporary .shp).Optional`breaklines.shp`

### Examples

*Extract breakline vectors from a DEM.*
`wbe.breakline_mapping(input='dem.tif', min_length=6, output='breaklines.shp', threshold=2.0)`


---

## Convergence Index

**Function name:** `convergence_index`


This tool calculates the convergence index (C), described by Koethe and Lehmeier (1996) and Kiss (2004), for each grid cell in an input digital elevation model (DEM). The convergence index measures the average amount by which the aspect value of each of the eight neighbours in a 3x3 kernel deviates from an aspect aligned with the direction towards  the center cell. As such the index measures the degree to which the surrounding topography converges on the center cell. 

C = 1 / 8 &Sigma;|&Phi; - Az0| - 90  

Where &Phi; is the aspect of a neighbour of the center cell and Az0 is the azimuth from the neighbour directed towards the center cell. Note, -90 < C < 90, where highly convergent areas have  values near -90 and highly divergent areas have values near 90. Therefore, in actuality, C is more properly  an index of divergence rather than a convergence index, despite its name. 

 

The user must specify the name of the input DEM (`dem`) and the  output raster (`output`). The Z conversion factor (`zfactor`) is only important when the vertical and  horizontal units are not the same in the DEM, and the DEM is in a projected coordinate system. When this is the case, the algorithm will multiply each elevation  in the DEM by the Z Conversion Factor to perform the unit conversion.  

For DEMs in projected coordinate systems, the tool uses the 3rd-order bivariate Taylor polynomial method described by Florinsky (2016). Based on a polynomial fit of the elevations within the 5x5 neighbourhood surrounding each cell, this method is considered more robust against outlier elevations (noise) than other methods. For DEMs in geographic coordinate systems (i.e. angular units), the tool uses the 3x3 polynomial fitting method for equal angle grids also described by Florinsky (2016). 

### Reference

 

Florinsky, I. (2016). Digital terrain analysis in soil science and geology. Academic Press. 

Kiss, R. (2004). Determination of drainage network in digital elevation models, utilities and  limitations. Journal of Hungarian geomathematics, 2, 17-29. 

Koethe, R. and Lehmeier, F. (1996): SARA - System zur Automatischen Relief-Analyse. User Manual,  2. Edition [Dept. of Geography, University of Goettingen, unpublished] 

### See Also

 

`aspect`, `plan_curvature`, `profile_curvature` 

### Python API

```python
def convergence_index(self, dem: Raster, z_factor: float = 1.0) -> Raster:
```


---

## DEM Void Filling

**Function name:** `dem_void_filling`


### Description

 

This tool implements a modified version of the Delta Surface Fill method of Grohman et al. (2006). It can fill voids (i.e., data holes) contained within a digital elevation model (`dem`) by fusing the data with a second DEM (`fill`) that defines the topographic surface within the void areas. The two surfaces are fused seamlessly so that the transition from the source and fill surfaces is undetectable. The fill surface need not have the same resolution as the source DEM. 

The algorithm works by computing a DEM-of-difference (DoD) for each valid grid cell in the source DEM that also has a valid elevation in the corresponding location within the fill DEM. This difference surface is then used to define offsets within the near void-edge locations. The fill surface elevations are then combined with interpolated offsets, with the interpolation based on near-edge offsets, and used to define a new surface within the void areas of the source DEM in such a way that the data transitions seamlessly from the source data source to the fill data. The image below provides an example of this method. 

 

The user must specify the `mean_plane_dist` parameter, which defines the distance (measured in grid cells)  within a void area from the void's edge. Grid cells within larger voids that are beyond this distance  from their edges have their vertical offsets, needed during the fusion of the DEMs, set to the mean offset for all grid cells that have both valid source and fill elevations. Void cells that are nearer their void  edges have vertical offsets that are interpolated based on nearby offset values (i.e., the DEM of difference). The interpolation uses inverse-distance weighted (IDW) scheme, with a user-specified weight parameter (`weight_value`). 

The `edge_treatment` parameter describes how the data fusion operates at the edges of voids, i.e., the first line of grid cells for which there are both source and fill elevation values. This parameter has values of "use DEM", "use Fill", and "average". Grohman et al. (2006) state that sometimes, due to a weakened signal within these marginal locations between the area of valid data and voids, the estimated elevation values are inaccurate. When this is the case, it is best to use fill elevations in the transitional areas. If this isn't the case, the  "use DEM" is the better option. A compromise between the two options is to average the two elevation sources. 

### References

 

Grohman, G., Kroenung, G. and Strebeck, J., 2006. Filling SRTM voids: The delta surface fill method.  Photogrammetric Engineering and Remote Sensing, 72(3), pp.213-216. 

### Python API

```python
def dem_void_filling(self, dem: Raster, fill: Raster, mean_plane_dist: int = 20, edge_treatment: str = "dem", weight_value: float = 2.0) -> Raster:
```


---

## Deviation From Mean Elevation

**Function name:** `deviation_from_mean_elevation`


This tool can be used to calculate the difference between the elevation of each grid cell and the mean elevation of the centering local neighbourhood, normalized by standard deviation. Therefore, this index of topographic residual is essentially equivalent to a local z-score. This attribute measures the *relative topographic position* as a fraction of local relief, and so is normalized to the local surface roughness. `DevFromMeanElev` utilizes an integral image approach (Crow, 1984) to ensure highly efficient filtering that is invariant with filter size. 

The user must input a digital elevation model (DEM) (`dem`) and the size of the neighbourhood in the *x* and *y* directions (`filterx` and `filtery`), measured in grid size. 

While `DeviationFromMeanElev` calculates the deviation from mean elevation (DEV) at a single, user-defined scale, the `max_elevation_deviation` tool can be used to output the per-pixel maximum DEV value across a range of input scales. 

### See Also

 

`DiffFromMeanElev`, `max_elevation_deviation` 

### Python API

```python
def deviation_from_mean_elevation(self, dem: Raster, filter_size_x: int = 11, filter_size_y: int = 11) -> Raster:
```


---

## Difference From Mean Elevation

**Function name:** `difference_from_mean_elevation`


This tool can be used to calculate the difference between the elevation of each grid cell and the mean elevation of the centering local neighbourhood. This is similar to what a high-pass filter calculates for imagery data, but is intended to work with DEM data instead. This attribute measures the *relative topographic position*. `DiffFromMeanElev` utilizes an integral image approach (Crow, 1984) to ensure highly efficient filtering that is invariant with filter size. 

The user must specify a digital elevation model (DEM) (`dem`) , and the size of the neighbourhood in the *x* and *y* directions (`filterx` and `filtery`), measured in grid size. 

While `DevFromMeanElev` calculates the DIFF at a single, user-defined scale, the `max_difference_from_mean` tool can be used to output the per-pixel maximum DIFF value across a range of input scales. 

### See Also

 

`DevFromMeanElev`, `max_difference_from_mean` 

### Python API

```python
def difference_from_mean_elevation(self, dem: Raster, filter_size_x: int = 11, filter_size_y: int = 11) -> Raster:
```


---

## Directional Relief

**Function name:** `directional_relief`


This tool calculates the relief for each grid cell in a digital elevation model (DEM) in a specified direction. Directional relief is an index of the degree to which a DEM grid cell is higher or lower than its surroundings. It is calculated by subtracting the elevation of a DEM grid cell from the average elevation of those cells which lie between it and the edge of the DEM in a specified compass direction. Thus, positive values indicate that a grid cell is lower than the average elevation of the grid cells in a specific direction (i.e. relatively sheltered), whereas a negative directional relief indicates that the grid cell is higher (i.e. relatively exposed). The algorithm is based on a modification of the procedure described by Lapen and Martz (1993). The modifications include: (1) the ability to specify any direction between 0-degrees and 360-degrees (`azimuth`), and (2) the ability to use a distance-limited search (`max_dist`), such that the ray-tracing procedure terminates before the DEM edge is reached for longer search paths. The algorithm works by tracing a ray from each grid cell in the direction of interest and evaluating the average elevation along the ray. Linear interpolation is used to estimate the elevation of the surface where a ray does not intersect the DEM grid precisely at one of its nodes. The user must input a DEM raster file (`dem`) and a hypothetical wind direction. Furthermore, the user is able to constrain the maximum search distance for the ray tracing. If no maximum search distance is specified, each ray will be traced to the edge of the DEM. The units of the output image are the same as the input DEM. 

Ray-tracing is a highly computationally intensive task and therefore this tool may take considerable time to operate for larger sized DEMs. This tool is parallelized to aid with computational efficiency. NoData valued grid cells in the input image will be assigned NoData values in the output image. The output raster is of the float data type and continuous data scale. Directional relief is best displayed using the blue-white-red bipolar palette to distinguish between the positive and negative values that are present in the output. 

### Reference

 

Lapen, D. R., & Martz, L. W. (1993). The measurement of two simple topographic indices of wind sheltering-exposure from raster digital elevation models. Computers & Geosciences, 19(6), 769-779. 

### See Also

 

`fetch_analysis`, `horizon_angle`, `relative_aspect` 

### Python API

```python
def directional_relief(self, dem: Raster, azimuth: float = 0.0, max_dist: float = float('inf')) -> Raster:
```


---

## Elev Above Pit

**Function name:** `elev_above_pit`


Experimental

Calculate elevation above the nearest depression (pit). Useful for drainage analysis and identifying topographic prominence.

geomorphometry terrain relative-elevation legacy-port


---

## Elev Above Pit Dist

**Function name:** `elev_above_pit_dist`


Experimental

Compatibility alias for elev_above_pit.

geomorphometry terrain legacy-port


---

## Elevation Percentile

**Function name:** `elevation_percentile`


Elevation percentile (EP) is a measure of local topographic position (LTP). It expresses the vertical position for a digital elevation model (DEM) grid cell (z0) as the percentile of the elevation distribution within the filter window, such that:  

EP = counti&isin;C(zi > z0) x (100 / nC)  

where z0 is the elevation of the window's center grid cell, zi is the elevation of cell *i* contained within the neighboring set C, and nC is the number of grid cells contained within the window. 

EP is unsigned and expressed as a percentage, bound between 0% and 100%. Quantile-based estimates (e.g., the median and interquartile range) are often used in nonparametric statistics to provide data variability estimates without assuming the distribution is normal. Thus, EP is largely unaffected by irregularly shaped elevation frequency distributions or by outliers in the DEM, resulting in a highly robust metric of LTP. In fact, elevation distributions within small to medium sized neighborhoods often exhibit skewed, multimodal, and non-Gaussian distributions, where the occurrence of elevation errors can often result in distribution outliers. Thus, based on these statistical characteristics, EP is considered one of the most robust representation of LTP. 

The algorithm implemented by this tool uses the relatively efficient running-histogram filtering algorithm of Huang et al. (1979). Because most DEMs contain floating point data, elevation values must be rounded to be binned. The `sig_digits` parameter is used to determine the level of precision preserved during this binning process. The algorithm is parallelized to further aid with computational efficiency. 

Neighbourhood size, or filter size, is specified in the x and y dimensions using the `filterx` and `filtery` flags. These dimensions should be odd, positive integer values (e.g. 3, 5, 7, 9, etc.). 

### References

 

Newman, D. R., Lindsay, J. B., and Cockburn, J. M. H. (2018). Evaluating metrics of local topographic position for multiscale geomorphometric analysis. Geomorphology, 312, 40-50. 

Huang, T., Yang, G.J.T.G.Y. and Tang, G., 1979. A fast two-dimensional median filtering algorithm. IEEE Transactions on Acoustics, Speech, and Signal Processing, 27(1), pp.13-18. 

### See Also

 

`DevFromMeanElev`, `DiffFromMeanElev` 

### Python API

```python
def elevation_percentile(self, dem: Raster, filter_size_x: int = 11, filter_size_y: int = 11, sig_digits: int = 2) -> Raster:
```


---

## Embankment Mapping

**Function name:** `embankment_mapping`


This tool can be used to map and/or remove road embankments from an input fine-resolution digital elevation  model (`dem`). Fine-resolution LiDAR DEMs can represent surface features such as road and railway  embankments with high fidelity. However, transportation embankments are problematic for several  environmental modelling applications, including soil an vegetation distribution mapping, where the pre-embankment topography is the contolling factor. The algorithm utilizes repositioned (`search_dist`) transportation  network cells, derived from rasterizing a transportation vector (`road_vec`), as seed points in a  region-growing operation. The embankment region grows based on derived morphometric parameters, including  road surface width (`min_road_width`), embankment width (`typical_width` and `max_width`), embankment  height (`max_height`), and absolute slope (`spillout_slope`). The tool can be run in two modes. By default the tool will simply map embankment cells, with a Boolean output raster. If, however, the `remove_embankments` flag is specified, the tool will instead output a DEM for which the mapped embankment grid cells have been excluded and new surfaces have been interpolated based on the surrounding elevation values (see below). 

Hillshade from original DEM:  

Hillshade from embankment-removed DEM:  

### References

 

Van Nieuwenhuizen, N, Lindsay, JB, DeVries, B. 2021. `Automated mapping of transportation embankments in  fine-resolution LiDAR DEMs`. Remote Sensing. 13(7), 1308; https://doi.org/10.3390/rs13071308 

### See Also:

 

`remove_off_terrain_objects`, `smooth_vegetation_residual` 

### Python API

```python
def embankment_mapping(self, dem: Raster, roads_vector: Vector, search_dist: float = 2.5, min_road_width: float = 6.0, typical_embankment_width: float = 30.0, typical_embankment_max_height: float = 2.0, embankment_max_width: float = 60.0, max_upwards_increment: float = 0.05, spillout_slope: float = 4.0, remove_embankments: bool = False) -> Tuple[Raster, Union[Raster, None]]:
```


---

## Exposure Towards Wind Flux

**Function name:** `exposure_towards_wind_flux`


This tool creates a new raster in which each grid cell is assigned the exposure of the land-surface to  a hypothetical wind flux. It can be conceptualized as the angle between a plane orthogonal to the wind  and a plane that represents the local topography at a grid cell (Bohner and Antonic, 2007). The user must input a digital elevation model (`dem`), as well as the dominant wind azimuth (`azimuth`) and a maximum search distance (`max_dist`) used to calclate the horizon angle. Notice that the specified azimuth represents a regional average wind direction.  

Exposure towards the sloped wind flux essentially combines the relative terrain aspect and the maximum upwind  slope (i.e. horizon angle). This terrain attribute accounts for land-surface orientation, relative to the wind,  and shadowing effects of distant topographic features but does not account for deflection of the wind by  topography. This tool should not be used on very extensive areas over which Earth's curvature must be taken into  account. DEMs in projected coordinate systems are preferred. 

**Algorithm Description:** 

Exposure is measured based on the equation presented in Antonic and Legovic (1999):  

cos(*E*) = cos(*S*) sin(*H*) + sin(*S*) cos(*H*) cos(*Az* - *A*)  

Where, *E* is angle between a plane defining the local terrain and a plane orthogonal to the wind flux, *S*  is the terrain slope, *A* is the terrain aspect, *Az* is the azimuth of the wind flux, and *H* is the horizon  angle of the wind flux, which is zero when only the horizontal component of the wind flux is accounted for. 

Exposure images are best displayed using a greyscale or bipolar palette to distinguish between the positive  and negative values that are present in the output. 

### References

 

Antonić, O., & Legović, T. 1999. Estimating the direction of an unknown air pollution source using a digital  elevation model and a sample of deposition. *Ecological modelling*, 124(1), 85-95. 

Böhner, J., & Antonić, O. 2009. Land-surface parameters specific to topo-climatology. Developments in Soil  Science, 33, 195-226. 

### See Also

 

`relative_aspect` 

### Python API

```python
def exposure_towards_wind_flux(self, dem: Raster, azimuth: float = 0.0, max_dist: float = float('inf'), z_factor: float = 1.0) -> Raster:
```


---

## Feature Preserving Smoothing

**Function name:** `feature_preserving_smoothing`


### Description

 

This tool implements a highly modified form of the DEM de-noising algorithm described by Sun et al. (2007). It is very effective at removing surface roughness from digital elevation models (DEMs), without significantly altering breaks-in-slope. As such, this tool should be used for smoothing DEMs rather than either smoothing with low-pass filters (e.g. mean, median, Gaussian filters) or grid size coarsening by resampling. The algorithm works by 1) calculating the surface normal 3D vector of each grid cell in the DEM, 2) smoothing the normal vector field using a filtering scheme that applies more weight to neighbours with lower angular difference in surface normal vectors, and 3) uses the smoothed normal vector field to update the elevations in the input DEM. 

Sun et al.'s (2007) original method was intended to work on input point clouds and fitted triangular irregular networks (TINs). The algorithm has been modified to work with input raster DEMs instead. In so doing, this algorithm calculates surface normal vectors from the planes fitted to 3 x 3 neighbourhoods surrounding each grid cell, rather than the triangular facet. The normal vector field smoothing and elevation updating procedures are also based on raster filtering operations. These modifications make this tool more efficient than Sun's original method, but will also result in a slightly different output than what would be achieved with Sun's method. 

The user must specify the values of three key parameters, including the filter size (`filter`), the normal difference threshold (`norm_diff`), and the number of iterations (`num_iter`). Lindsay et al. (2019) found that **the degree of smoothing was less impacted by the filter size than it was either the normal difference threshold and the number of iterations**. A filter size of 11, the default value, tends to work well in many cases. To increase the level of smoothing applied to the DEM, consider increasing the normal difference threshold, i.e. the angular difference in normal vectors between the center cell of a filter window and a neighbouring cell. This parameter determines which neighbouring values are included in a filtering operation and higher values will result in a greater number of neighbouring cells included, and therefore smoother surfaces. Similarly, increasing the number of iterations from the default value of 3 to upwards of 5-10 will result in significantly greater smoothing. 

Before smoothing treatment:  

After smoothing treatment with FPS:  

For a video tutorial on how to use the `feature_preserving_smoothing` tool, please see `this YouTube video`. 

### Reference

 

Lindsay JB, Francioni A, Cockburn JMH. 2019. LiDAR DEM smoothing and the preservation of drainage features. *Remote Sensing*, 11(16), 1926; DOI: 10.3390/rs11161926. 

Sun, X., Rosin, P., Martin, R., & Langbein, F. (2007). Fast and effective feature-preserving mesh denoising. *IEEE Transactions on Visualization & Computer Graphics*, (5), 925-938. 

### Parameters

 

dem (Raster):     The input digital elevation model (DEM) 

filter_size (int):     The filter size used for smoothing. Default is 11. 

normal_diff_threshold (float):     The maximum allowable difference in the angle of the normals between two grid cells on the same facet. Default is 8.0. 

iterations (int):     The number of iterations used during smoothing. Default is 3. 

max_elevation_diff (float):     The maximum allowable vertical distance that a cell's elevation is allowed to be changed by 

z_factor (float):     Used to convert elevation units so that they match the horizontal units. Unless the two units differ,      this should be set to 1.0. Default is 1.0. 

### Returns

 

Raster: return value 

### Python API

```python
def feature_preserving_smoothing(self, dem: Raster, filter_size: int = 11, normal_diff_threshold: float = 8.0, iterations: int = 3, max_elevation_diff: float = float('inf'), z_factor: float = 1.0) -> Raster:
```


---

## Feature Preserving Smoothing Multiscale

**Function name:** `feature_preserving_smoothing_multiscale`


*No help documentation available for this tool.*


---

## Fetch Analysis

**Function name:** `fetch_analysis`


This tool creates a new raster in which each grid cell is assigned the distance, in meters, to the nearest topographic obstacle in a specified direction. It is a modification of the algorithm described by Lapen and Martz (1993). Unlike the original algorithm, Fetch Analysis is capable of analyzing fetch in any direction from 0-360 degrees. The user must input a digital elevation model (DEM) raster file, a hypothetical wind direction,  and a value for the height increment parameter. The algorithm searches each grid cell in a path following the specified wind direction until the following condition is met:  

*Z*test >= *Z*core + *DI*  

where *Z*core is the elevation of the grid cell at which fetch is being determined, *Z*test is the elevation of the grid cell being tested as a topographic obstacle, *D* is the distance between the two grid cells in meters, and *I* is the height increment in m/m. Lapen and Martz (1993) suggest values for *I* in the range of 0.025 m/m to 0.1 m/m based on their study of snow re-distribution in low-relief agricultural landscapes of the Canadian Prairies. If the directional search does not identify an obstacle grid cell before the edge of the DEM is reached, the distance between the DEM edge and Zcore is entered. Edge distances are assigned negative values to differentiate between these artificially truncated fetch values and those for which a valid topographic obstacle was identified. Notice that linear interpolation is used to estimate the elevation of the surface where a ray (i.e. the search path) does not intersect the DEM grid precisely at one of its nodes. 

Ray-tracing is a highly computationally intensive task and therefore this tool may take considerable time to operate for larger sized DEMs. This tool is parallelized to aid with computational efficiency. NoData valued grid cells in the input image will be assigned NoData values in the output image. Fetch Analysis images are best displayed using the blue-white-red bipolar palette to distinguish between the positive and negative values that are present in the output. 

### Reference

 

Lapen, D. R., & Martz, L. W. (1993). The measurement of two simple topographic indices of wind sheltering-exposure from raster digital elevation models. Computers & Geosciences, 19(6), 769-779. 

### See Also

 

`directional_relief`, `horizon_angle`, `relative_aspect` 

### Python API

```python
def fetch_analysis(self, dem: Raster, azimuth: float = 0.0, height_increment: float = 0.05) -> Raster:
```


---

## Fill Missing Data

**Function name:** `fill_missing_data`


This tool can be used to fill in small gaps in a raster or digital elevation model (DEM). The gaps, or holes, must have recognized NoData values. If gaps do not currently have this characteristic, use the `set_nodata_value` tool and ensure that the data are stored using a raster format that supports NoData values. All valid, non-NoData values in the input raster will be assigned the same value in the output image. 

The algorithm uses an inverse-distance weighted (IDW) scheme based on the valid values on the edge of NoData gaps to estimate gap values. The user must specify the filter size (`filter`), which determines the size of gap that is filled, and the IDW weight (`weight`). 

The filter size, specified in grid cells, is used to determine how far the algorithm will search for valid, non-NoData values. Therefore, setting a larger filter size allows for the filling of larger gaps in the input raster. 

The `no_edges` flag can be used to exclude NoData values that are connected to the edges of the raster. It is usually the case that irregularly shaped DEMs have large regions of NoData values along the containing raster edges. This flag can be used to exclude these regions from the gap-filling operation, leaving only interior gaps for filling. 

### See Also

 

`set_nodata_value` 

### Python API

```python
def fill_missing_data(self, dem: Raster, filter_size: int = 11, weight: float = 2.0, exclude_edge_nodata: bool = False) -> Raster:
```


---

## Find Ridges

**Function name:** `find_ridges`


This tool can be used to identify ridge cells in a digital elevation model (DEM). Ridge cells are those that have lower neighbours either to the north and south or the east and west. Line thinning can optionally be used to create single-cell wide ridge networks by specifying the `line_thin` parameter. 

### Python API

```python
def find_ridges(self, dem: Raster, line_thin: bool = True) -> Raster:
```


---

## Hillshade

**Function name:** `hillshade`


This tool performs a hillshade operation (also called shaded relief) on an input digital elevation model (DEM). The user must input a DEM. Other parameters that must be specified include the illumination source azimuth (`azimuth`), or sun direction (0-360 degrees), the illumination source altitude (`altitude`; i.e. the elevation of the sun above the horizon, measured as an angle from 0 to 90 degrees) and the Z conversion factor (`zfactor`). The *Z conversion factor* is only important when the vertical and horizontal units are not the same in the DEM, and the DEM is in a projected coordinate system. When this is the case, the algorithm will multiply each elevation in the DEM by the Z conversion factor. If the DEM is in the geographic coordinate system (latitude and longitude), the following equation is used:  

zfactor = 1.0 / (111320.0 x cos(mid_lat))  

where `mid_lat` is the latitude of the centre of the raster, in radians. 

The hillshade value (*HS*) of a DEM grid cell is calculate as:  

*HS* = tan(*s*) / [1 - tan(*s*)2]0.5 x [sin(*Alt*) / tan(*s*) - cos(*Alt*) x sin(*Az* - *a*)]  

where *s* and *a* are the local slope gradient and aspect (orientation) respectively and *Alt* and *Az* are the illumination source altitude and azimuth respectively. Slope and aspect are calculated using Horn's (1981) 3rd-order finate difference method. 

### Reference

 

Gallant, J. C., and J. P. Wilson, 2000, Primary topographic attributes, in Terrain Analysis: Principles and Applications, edited by J. P. Wilson and J. C. Gallant pp. 51-86, John Wiley, Hoboken, N.J. 

### See Also

 

`hypsometrically_tinted_hillshade`, `multidirectional_hillshade`, `aspect`, `slope` 

### Python API

```python
def hillshade(self, dem: Raster, azimuth: float = 315.0, altitude: float = 30.0, z_factor: float = 1.0) -> Raster:
```


---

## Hypsometrically Tinted Hillshade

**Function name:** `hypsometrically_tinted_hillshade`


This tool creates a hypsometrically tinted shaded relief (Swiss hillshading) image from an input digital elevation model (DEM). The tool combines a colourized version of the DEM with varying illumination provided by a hillshade image, to produce a composite relief model that can be used to visual topography for more effective interpretation of landscapes. The output of the tool is a 24-bit red-green-blue (RGB) colour image. 

The user must input a DEM. Other parameters that must be specified include the illumination source azimuth (`azimuth`), or sun direction (0-360 degrees), the illumination source altitude (`altitude`; i.e. the elevation of the sun above the horizon, measured as an angle from 0 to 90 degrees), the hillshade weight (`hs_weight`; 0-1), image brightness (`brightness`; 0-1), and atmospheric effects (`atmospheric`; 0-1). The hillshade weight can be used to increase or subdue the relative prevalence of the hillshading effect in the output image. The image brightness parameter is used to create an overall brighter or darker version of the terrain rendering; note however, that very high values may over-saturate the well-illuminated portions of the terrain. The atmospheric effects parameter can be used to introduce a haze or atmosphere effect to the output image. It is intended to reproduce the effect of viewing mountain valley bottoms through a thicker and more dense atmosphere. Values greater than zero will introduce a slightly blue tint, particularly at lower altitudes, blur the hillshade edges slightly, and create a random haze-like speckle in lower areas. The user must also specify the Z conversion factor (`zfactor`). The *Z conversion factor* is only important when the vertical and horizontal units are not the same in the DEM. When this is the case, the algorithm will multiply each elevation in the DEM by the Z conversion factor. If the DEM is in the geographic coordinate system (latitude and longitude), the following equation is used:  

zfactor = 1.0 / (111320.0 x cos(mid_lat))  

where `mid_lat` is the latitude of the centre of the raster, in radians. 

 

### See Also

 

`hillshade`, `multidirectional_hillshade`, `aspect`, `slope` 

### Python API

```python
def hypsometrically_tinted_hillshade(self, dem: Raster, solar_altitude: float = 45.0, hillshade_weight: float = 0.5, brightness: float = 0.5, atmospheric_effects: float = 0.0, palette: str = "atlas", reverse_palette: bool = False, full_360_mode: bool = False, z_factor: float = 1.0) -> Raster:
```


---

## Local Hypsometric Analysis

**Function name:** `local_hypsometric_analysis`


PROExperimental

Computes the minimum local hypsometric integral across a nonlinearly sampled range of neighbourhood scales.

geomorphometry multiscale hypsometry legacy-port

### Parameters

NameDescriptionRequiredDefault
`input`Input DEM raster path or typed raster object.Required`dem.tif`
`min_scale`Minimum half-window radius in cells (default 4).Optional`4`
`step_size`Base step size in cells (default 1). Alias: step.Optional`1`
`num_steps`Number of scales to evaluate (default 10).Optional`10`
`step_nonlinearity`Scale-step nonlinearity in [1,4] (default 1.0).Optional`1.0`
`output`Optional output path for local HI minimum raster.Optional—
`output_scale`Optional output path for scale-of-minimum-HI raster.Optional—

### Examples

*Compute minimum local hypsometric integral and associated scale.*
`wbe.local_hypsometric_analysis(input='dem.tif', min_scale=4, num_steps=10, output='local_hypsometric_analysis.tif', output_scale='local_hypsometric_analysis_scale.tif', step_nonlinearity=1.0, step_size=1)`


---

## Low Points On Headwater Divides

**Function name:** `low_points_on_headwater_divides`


PROExperimental

Locates low pass points along divides between neighboring headwater subbasins.

geomorphometry streams subbasins passes legacy-port

### Parameters

NameDescriptionRequiredDefault
`dem`Input depressionless DEM raster path or typed raster object.Required`dem.tif`
`streams`Input stream raster path (positive values indicate channel cells).Required`streams.tif`
`output`Optional output vector path (default temporary .shp).Optional`low_points_on_headwater_divides.shp`

### Examples

*Find low pass points between neighboring headwater basins.*
`wbe.low_points_on_headwater_divides(dem='dem.tif', output='low_points_on_headwater_divides.shp', streams='streams.tif')`


---

## Max Downslope Elev Change

**Function name:** `max_downslope_elev_change`


This tool calculates the maximum elevation drop between each grid cell and its neighbouring cells within a digital elevation model (DEM). The user must input a DEM (`dem`). 

### See Also

 

`max_upslope_elev_change`, `min_downslope_elev_change`, `num_downslope_neighbours` 

### Python API

```python
def max_downslope_elev_change(self, raster: Raster) -> Raster:
```


---

## Max Upslope Elev Change

**Function name:** `max_upslope_elev_change`


a digital elevation model (DEM). The user must input DEM (`dem`). 

### See Also

 

`max_downslope_elev_change` 

### Python API

```python
def max_upslope_elev_change(self, raster: Raster) -> Raster:
```


---

## Min Downslope Elev Change

**Function name:** `min_downslope_elev_change`


This tool calculates the minimum elevation drop between each grid cell and its neighbouring cells within a digital elevation model (DEM). The user must input a DEM (`dem`). 

### See Also

 

`max_downslope_elev_change`, `num_downslope_neighbours` 

### Python API

```python
def min_downslope_elev_change(self, raster: Raster) -> Raster:
```


---

## Multidirectional Hillshade

**Function name:** `multidirectional_hillshade`


This tool performs a hillshade operation (also called shaded relief) on an input digital elevation model (DEM) with multiple sources of illumination. The user must input a DEM (`dem`). Other parameters that must be specified include the altitude of the illumination sources (`altitude`; i.e. the elevation of the sun above the horizon, measured as an angle from 0 to 90 degrees) and the Z conversion factor (`zfactor`). The *Z conversion factor* is only important when the vertical and horizontal units are not the same in the DEM, and the DEM is in a projected coordinate system. When this is the case, the algorithm will multiply each elevation in the DEM by the Z conversion factor.  

The hillshade value (*HS*) of a DEM grid cell is calculate as:  

*HS* = tan(*s*) / [1 - tan(*s*)2]0.5 x [sin(*Alt*) / tan(*s*) - cos(*Alt*) x sin(*Az* - *a*)]  

where *s* and *a* are the local slope gradient and aspect (orientation) respectively and *Alt* and *Az* are the illumination source altitude and azimuth respectively. Slope and aspect are calculated using Horn's (1981) 3rd-order finate difference method. 

Lastly, the user must specify whether or not to use full 360-degrees of illumination sources (`full_mode`). When this flag is not specified, the tool will perform a weighted summation of the hillshade images from four illumination azimuth positions at 225, 270, 315, and 360 (0) degrees, given weights of 0.1, 0.4, 0.4, and 0.1 respectively. When run in the full 360-degree mode, eight illumination source azimuths are used to calculate the output at 0, 45, 90, 135, 180, 225, 270, and 315 degrees, with weights of 0.15, 0.125, 0.1, 0.05, 0.1, 0.125, 0.15, and 0.2 respectively. 

Classic hillshade (Azimuth=315, Altitude=45.0)  

Multi-directional hillshade (Altitude=45.0, Four-direction mode)  

Multi-directional hillshade (Altitude=45.0, 360-degree mode)  

### See Also

 

`hillshade`, `hypsometrically_tinted_hillshade`, `aspect`, `slope` 

### Python API

```python
def multidirectional_hillshade(self, dem: Raster, altitude: float = 30.0, z_factor: float = 1.0, full_360_mode: bool = False) -> Raster:
```


---

## Num Downslope Neighbours

**Function name:** `num_downslope_neighbours`


This tool calculates the number of downslope neighbours of each grid cell in a raster digital elevation model (DEM). The user must input a DEM (`dem`). The tool examines the eight neighbouring cells for each grid cell in a the DEM and counts the number of neighbours with an elevation less than the centre cell of the 3 x 3 window. The output image can therefore have values raning from 0 to 8. A raster grid cell with eight downslope neighbours is a peak and a cell with zero downslope neighbours is a pit. This tool can be used with the `NumUpslopeNeighbours` tool to assess the degree of local flow divergence/convergence. 

### See Also

 

`NumUpslopeNeighbours` 

### Python API

```python
def num_downslope_neighbours(self, dem: Raster) -> Raster:
```


---

## Num Upslope Neighbours

**Function name:** `num_upslope_neighbours`


Experimental

Counts the number of 8-neighbour cells higher than each DEM cell.

geomorphometry terrain flow legacy-port


---

## Profile

**Function name:** `profile`


This tool can be used to plot the data profile, along a set of one or more vector lines (`lines`), in an input (`surface`) digital elevation model (DEM), or other surface model. The data profile plots surface height (y-axis) against distance along profile (x-axis). The tool outputs an interactive SVG line graph embedded in an HTML document (`output`). If the vector lines file contains multiple line features, the output plot will contain each of the input profiles. 

If you want to extract the `longitudinal profile` of a river, use the `long_profile` tool instead. 

### See Also

 

`long_profile`, `hypsometric_analysis` 

### Python API

```python
def profile(self, lines_vector: Vector, surface: Raster, output_html_file: str) -> None:
```


---

## Slope Vs Aspect Plot

**Function name:** `slope_vs_aspect_plot`


PROExperimental

Creates an HTML radial slope-vs-aspect analysis plot for an input DEM.

geomorphometry terrain plot html legacy-port


---

## Slope Vs Elev Plot

**Function name:** `slope_vs_elev_plot`


This tool can be used to create a slope versus average elevation plot for one or more digital elevation models (DEMs). Similar to a hypsometric analysis (`hypsometric_analysis`), the slope-elevation relation can reveal the basic topographic character of a site. The output of this analysis is an HTML document (`output`) that contains the slope-elevation chart. The tool can plot multiple slope-elevation analyses on the same chart by specifying multiple input DEM files (`inputs`). Each input DEM can have an optional watershed in which the slope-elevation analysis is confined by specifying the optional `watershed` flag. If multiple input DEMs are used, and a watershed is used to confine the analysis to a sub-area, there must be the same number of input raster watershed files as input DEM files. The order of the DEM and watershed files must the be same (i.e. the first DEM file must correspond to the first watershed file, the second DEM file to the second watershed file, etc.). Each watershed file may contain one or more watersheds, designated by unique identifiers. 

 

### See Also

 

`hypsometric_analysis`, `slope_vs_aspect_plot` 

### Python API

```python
def slope_vs_elev_plot(self, dem_rasters: List[Raster], output_html_file: str, watershed_rasters: List[Raster]) -> None:
```


---

## Surface Area Ratio

**Function name:** `surface_area_ratio`


This tool calculates the ratio between the surface area and planar area of grid cells within digital elevation models (DEMs). The tool uses the method of Jenness (2004) to estimate the surface area of a DEM grid cell based on the elevations contained within the 3 x 3 neighbourhood surrounding each cell. The surface area ratio has a lower bound of 1.0 for perfectly flat grid cells and is greater than 1.0 for other conditions. In particular, surface area ratio is a measure of neighbourhood surface shape complexity (texture) and elevation variability (local slope). 

### Reference

 

Jenness, J. S. (2004). Calculating landscape surface area from digital elevation models. Wildlife Society Bulletin, 32(3), 829-839. 

### See Also

 

`ruggedness_index`, `multiscale_roughness`, `circular_variance_of_aspect`, `edge_density` 

### Python API

```python
def surface_area_ratio(self, dem: Raster) -> Raster:
```


---

## Topographic Hachures

**Function name:** `topographic_hachures`


PROExperimental

Creates topographic hachure polylines from a DEM using contour-seeded downslope and upslope flowlines. Legacy authorship attribution is intentionally preserved for this tool.

geomorphometry hachures contours vector legacy-port


---

## Map Off Terrain Objects

**Function name:** `map_off_terrain_objects`


This tool can be used to map off-terrain objects in a digital surface model (DSM) based on cell-to-cell differences in elevations and local slopes. The algorithm works by using a region-growing operation to connect neighbouring grid cells outwards from seed cells. Two neighbouring cells are considered connected if the slope between the two cells is less than the user-specified maximum slope value (`max_slope`). Mapped segments that are less than the minimum feature size (`min_size`), in grid cells, are assigned a common background value. Note that this method of mapping off-terrain objects, and thereby separating ground cells from non-ground objects in DSMs, works best with fine-resolution DSMs that have been interpolated using a non-smoothing method, such as triangulation (TINing) or nearest-neighbour interpolation. 

### See Also

 

`remove_off_terrain_objects` 

### Python API

```python
def map_off_terrain_objects(self, dem: Raster, max_slope: float = float('inf'), min_feature_size: int = 0) -> Raster:
```


---

## Remove Off Terrain Objects

**Function name:** `remove_off_terrain_objects`


This tool can be used to create a bare-earth DEM from a fine-resolution digital surface model. The tool is typically applied to LiDAR DEMs which frequently contain numerous off-terrain objects (OTOs) such as buildings, trees and other vegetation, cars, fences and other anthropogenic objects. The algorithm works by finding and removing steep-sided peaks within the DEM. All peaks within a sub-grid, with a dimension of the user-specified maximum OTO size (`filter`), in pixels, are identified and removed. Each of the edge cells of the peaks are then examined to see if they have a slope that is less than the user-specified minimum OTO edge slope (`slope`) and a back-filling procedure is used. This ensures that OTOs are distinguished from natural topographic features such as hills. The DEM is preprocessed using a white top-hat transform, such that elevations are normalized for the underlying ground surface. 

Note that this tool is appropriate to apply to rasterized LiDAR DEMs. Use the `lidar_ground_point_filter` tool to remove or classify OTOs within a LiDAR point-cloud. 

### Reference

 

J.B. Lindsay (2018) A new method for the removal of off-terrain objects from LiDAR-derived raster surface models. Available online, DOI: `10.13140/RG.2.2.21226.62401` 

### See Also

 

`map_off_terrain_objects`, `tophat_transform`, `lidar_ground_point_filter` 

### Python API

```python
def remove_off_terrain_objects(self, dem: Raster, filter_size: int = 11, slope_threshold: float = 15.0) -> Raster:
```


---

## Ridge And Valley Vectors

**Function name:** `ridge_and_valley_vectors`


This function can be used to extract ridge and channel vectors from an input digital elevation model (DEM). The function works by first calculating elevation percentile (EP) from an input DEM using a neighbourhood size set by the user-specified filter_size parameter. Increasing the value of filter_size can result in more continuous mapped ridge and valley bottom networks. A thresholding operation is then applied to identify cells that have an EP less than the  user-specified ep_threshold (valley bottom regions) and a second thresholding operation maps regions where EP is  greater than 100 - ep_threshold (ridges). Each of these ridge and valley region maps are also multiplied by a slope  mask created by identify all cells with a slope greater than the user-specified slope_threshold value, which is set  to zero by default. This second thresholding can be helpful if the input DEM contains extensive flat areas, which  can be confused for valleys otherwise. The filter_size and ep_threshold parameters are somewhat dependent on one  another. Increasing the filter_size parameter generally requires also increasing the value of the ep_threshold. The  ep_threshold can take values between 5.0 and 50.0, where larger values will generally result in more extensive and  continuous mapped ridge and valley bottom networks. For many DEMs, a value on the higher end of the scale tends to  work best. 

After applying the thresholding operations, the function then applies specialized shape generalization, line thinning,  and vectorization alorithms to produce the final ridge and valley vectors. The user must also specify the value of the min_length parameter, which determines the minimum size, in grid cells, of a mapped line feature. The function outputs a tuple of two vector, the first being the ridge network and the second vector being the valley-bottom network. 

 

### Code Example

 

`from whitebox_workflows import WbEnvironment 

### Set up the WbW environment

 

license_id = 'my-license-id' # Note, this tool requires a license for WbW-Pro wbe = WbEnvironment(license_id) try:     wbe.verbose = True     wbe.working_directory = '/path/to/data' # Read the input DEM dem = wbe.read_raster('DEM.tif')  # Run the operation ridges, valleys = wbe.ridge_and_valley_vectors(dem, filter_size=21, ep_threshold=45.0, slope_threshold=1.0, min_length=25) wbe.write_vector(ridges, 'ridges_lines.shp') wbe.write_vector(valley, 'valley_lines.shp')  print('Done!') ` 

except Exception as e:   print("Error: ", e) finally:     wbe.check_in_license(license_id)  

### See Also:

 

`extract_valleys` 

### Python API

```python
def ridge_and_valley_vectors(self, dem: Raster, filter_size: int = 11, ep_threshold: float = 30.0, slope_threshold: float = 0.0, min_length: int = 20) -> Tuple[Raster, Raster]:
```


---

## Smooth Vegetation Residual

**Function name:** `smooth_vegetation_residual`


PROExperimental

Reduces canopy residual roughness by masking high local DEV responses at small scales and re-interpolating masked elevations.

geomorphometry lidar smoothing dem legacy-port

### Parameters

NameDescriptionRequiredDefault
`input`Input DEM raster path or typed raster object.Required`dem.tif`
`max_scale`Maximum DEV half-window radius in cells (default 30).Optional`30`
`dev_threshold`Minimum DEV magnitude used to flag roughness cells (default 1.0).Optional`1.0`
`scale_threshold`Maximum scale considered roughness (default 5).Optional`5`
`output`Optional output path. If omitted, result stays in memory.Optional—

### Examples

*Suppress vegetation-residual roughness in a LiDAR DEM.*
`wbe.smooth_vegetation_residual(dev_threshold=1.0, input='dem.tif', max_scale=30, output='smooth_vegetation_residual.tif', scale_threshold=5)`
