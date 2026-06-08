# Landform Indices


---

## Elev Relative To Min Max

**Function name:** `elev_relative_to_min_max`


This tool can be used to express the elevation of a grid cell in a digital elevation model (DEM) as a percentage of the relief between the DEM minimum and maximum values. As such, it provides a basic measure of relative topographic position. 

### See Also

 

`elev_relative_to_watershed_min_max`, `elevation_above_stream`, `ElevAbovePit` 

### Python API

```python
def elev_relative_to_min_max(self, dem: Raster) -> Raster:
```


---

## Geomorphons

**Function name:** `geomorphons`


This tool can be used to perform a geomorphons landform classification based on an input digital elevation model (`dem`). The geomorphons concept is based on line-of-sight analysis for the eight topographic profiles in the cardinal directions surrounding each grid cell in the input DEM. The relative sizes of the zenith angle of a profile's maximum elevation angle (i.e. horizon angle) and the nadir angle of a profile's minimum elevation angle are then used to generate a ternary (base-3) digit: 0 when the nadir angle is less than the zenith angle, 1 when the two angles differ by less than a user-defined flatness threshold (`threshold`), and 2 when the nadir angle is greater than the zenith angle. A ternary number is then derived from the digits assigned to each of the eight profiles, with digits sequenced counter-clockwise from east. This ternary number forms the  geomorphons code assigned to the grid cell. There are 38 = 6561 possible codes, although many of these codes are equivalent geomorphons through rotations and reflections. Some of the remaining geomorphons also rarely if ever occur in natural topography. Jasiewicz et al. (2013) identified 10 common landform types by reclassifying related geomorphons codes. The user may choose to output these common forms (`forms`) rather than the the raw ternary code. These landforms include:  ValueLandform Type 1Flat 2Peak (summit) 3Ridge 4Shoulder 5Spur (convex) 6Slope 7Hollow (concave) 8Footslope 9Valley 10Pit (depression)   

One of the main advantages of the geomrophons method is that, being based on minimum/maximum elevation angles, the scale used to estimate the landform type at a site adapts to the surrounding terrain. In principle, choosing a large value of search distance (`search`) should result in identification of a landform element regardless of its scale. 

An experimental feature has been added to correct for global inclination. Global inclination biases the flatness threshold angle becasue it is measured relative to the z-axis, especially in locally flat areas. Including the `residuals` flag "flattens" the input by converting elevation to residuals of a 2-d linear model. 

 

### Reference

 

Jasiewicz, J., and Stepinski, T. F. (2013). Geomorphons — a pattern recognition approach to classification and mapping of landforms. Geomorphology, 182, 147-156. 

### See Also

 

`PennockLandformClass` 

### Python API

```python
def geomorphons(self, dem: Raster, search_distance: int = 1, flatness_threshold: float = 1.0, flatness_distance: int = 0, skip_distance: int = 0, output_forms: bool = True, analyze_residuals: bool = False) -> Raster:
```


---

## Hypsometric Analysis

**Function name:** `hypsometric_analysis`


This tool can be used to derive the hypsometric curve, or area-altitude curve, of one or more input digital elevation models (DEMs) ('inputs'). A hypsometric curve is a histogram or cumulative distribution function of elevations in a geographical area. 

 

### See Also

 

`SlopeVsElevationPlot` 

### Python API

```python
def hypsometric_analysis(self, dem_rasters: List[Raster], output_html_file: str, watershed_rasters: List[Raster] = None) -> None:
```


---

## Multiscale Topographic Position Class

**Function name:** `multiscale_topographic_position_class`


### Description

This tool classifies each DEM grid cell into one of nine multiscale topographic position classes by combining local- and broad-scale maximum standardized elevation deviation (DEVmax) responses. The tool computes DEVmax internally for two user-defined scale ranges and then applies ternary thresholds to both responses. The broad-scale response separates *lowland*, *intermediate*, and *upland* settings, while the local-scale response separates *hollow*, *mid-position*, and *knoll* settings.

The combined output classes are: 0 Lowland hollow, 1 Lowland mid-position, 2 Lowland knoll, 3 Intermediate hollow, 4 Intermediate mid-position, 5 Intermediate knoll, 6 Upland hollow, 7 Upland mid-position, and 8 Upland knoll. The output raster is categorical and is intended to be displayed using a fixed nine-class palette.

The local and broad scale ranges are each defined by minimum scale, maximum scale, and step size parameters. Thresholds (`local_threshold` and `broad_threshold`) control the ternary classification of the corresponding DEVmax mosaics. The optional `min_patch_size` parameter can be used to suppress very small mapped patches after classification. The optional `output_confidence` raster stores a class confidence value in the range [0, 1] based on the margin from the classification thresholds.

### Reference

Lindsay, J. B., Cockburn, J. M. H., and Russell, H. A. J. (2015). An integral image approach to performing multi-scale topographic position analysis. Geomorphology, 245, 51-61.

### See Also

`max_elevation_deviation`, `multiscale_topographic_position_image`

### Python API

```python
def multiscale_topographic_position_class(self, input: Raster, local_min_scale: int = 5, local_max_scale: int = 80, local_step_size: int = 1, broad_min_scale: int = 500, broad_max_scale: int = 2000, broad_step_size: int = 20, local_threshold: float = 0.5, broad_threshold: float = 0.5, min_patch_size: int = 0, output_path: Optional[str] = None, output_confidence_path: Optional[str] = None, callback: Any = None) -> Raster:
```


---

## Pennock Landform Classification

**Function name:** `pennock_landform_classification`


Tool can be used to perform a simple landform classification based on measures of slope gradient and curvature derived from a user-specified digital elevation model (DEM). The classification scheme is based on the method proposed by Pennock, Zebarth, and DeJong (1987). The scheme divides a landscape into seven element types, including: convergent footslopes (CFS), divergent footslopes (DFS), convergent shoulders (CSH), divergent shoulders (DSH), convergent backslopes (CBS), divergent backslopes (DBS), and level terrain (L). The output raster image will record each of these base element types as: 

Element Type  |  Code  ------------- | -------  CFS           |  1  DFS           |  2  CSH           |  3  DSH           |  4  CBS           |  5  DBS           |  6  L             |  7 

The definition of each of the elements, based on the original Pennock et al. (1987) paper, is as follows:  PROFILEGRADIENTPLANElement Concave ( -0.10)High >3.0Concave 0.0CFS Concave ( -0.10)High >3.0Convex >0.0DFS Convex (>0.10)High >3.0Concave 0.0CSH Convex (>0.10)High >3.0Convex >0.0DSH Linear (-0.10...0.10)High >3.0Concave 0.0CBS Linear (-0.10...0.10)High >3.0Convex >0.0DBS    

Where PROFILE is profile curvature, GRADIENT is the slope gradient, and PLAN is the plan curvature. Note that these values are likely landscape and data specific and can be adjusted by the user. Landscape classification schemes that are based on terrain attributes are highly sensitive to short-range topographic variability (i.e. roughness) and can benefit from pre-processing the DEM with a smoothing filter to reduce the effect of surface roughness and emphasize the longer-range topographic signal. The `feature_preserving_smoothing` tool offers excellent performance in smoothing DEMs without removing the sharpness of breaks-in-slope. 

### Reference

 

Pennock, D.J., Zebarth, B.J., and DeJong, E. (1987) Landform classification and soil distribution in hummocky terrain, Saskatchewan, Canada. Geoderma, 40: 297-315. 

### See Also

 

`feature_preserving_smoothing` 

### Python API

```python
def pennock_landform_classification(self, dem: Raster, slope_threshold: float = 3.0, prof_curv_threshold: float = 0.1, plan_curv_threshold: float = 0.0, z_factor: float = 1.0) -> Tuple[Raster, str]:
```


---

## Percent Elev Range

**Function name:** `percent_elev_range`


Percent elevation range (PER) is a measure of local topographic position (LTP). It expresses the vertical position for a digital elevation model (DEM) grid cell (z0) as the percentage of the elevation range within the neighbourhood filter window, such that:  

PER = z0 / (zmax - zmin) x 100  

where z0 is the elevation of the window's center grid cell, zmax is the maximum neighbouring elevation, and zmin is the minimum neighbouring elevation. 

Neighbourhood size, or filter size, is specified in the x and y dimensions using the `filterx` and `filtery`flags. These dimensions should be odd, positive integer values (e.g. 3, 5, 7, 9, etc.). 

Compared with `ElevPercentile` and `DevFromMeanElev`, PER is a less robust measure of LTP that is susceptible to outliers in neighbouring elevations (e.g. the presence of off-terrain objects in the DEM). 

### References

 

Newman, D. R., Lindsay, J. B., and Cockburn, J. M. H. (2018). Evaluating metrics of local topographic position for multiscale geomorphometric analysis. Geomorphology, 312, 40-50. 

### See Also

 

`ElevPercentile`, `DevFromMeanElev`, `DiffFromMeanElev`, `relative_topographic_position` 

### Python API

```python
def percent_elev_range(self, dem: Raster, filter_size_x: int = 11, filter_size_y: int = 11) -> Raster:
```


---

## Relative Topographic Position

**Function name:** `relative_topographic_position`


Relative topographic position (RTP) is an index of local topographic position (i.e. how elevated or low-lying a site is relative to its surroundings) and is a modification of percent elevation range (PER; `percent_elev_range`) and accounts for the elevation distribution. Rather than positioning the central cell's elevation solely between the filter extrema, RTP is a piece-wise function that positions the central elevation relative to the minimum (zmin), mean (&mu;), and maximum values (zmax), within a local neighbourhood of a user-specified size (`filterx`, `filtery`), such that:  

RTP = (z0 − &mu;) / (&mu; − zmin), if z0 < &mu; 

OR 

RTP = (z0 − &mu;) / (zmax - &mu;), if z0 >= &mu;   

The resulting index is bound by the interval [−1, 1], where the sign indicates if the cell is above or below than the filter mean. Although RTP uses the mean to define two linear functions, the reliance on the filter extrema is expected to result in sensitivity to outliers. Furthermore, the use of the mean implies assumptions of unimodal and symmetrical elevation distribution. 

In many cases, Elevation Percentile (`ElevPercentile`) and deviation from mean elevation (`DevFromMeanElev`) provide more suitable and robust measures of relative topographic position. 

### Reference

 

Newman, D. R., Lindsay, J. B., and Cockburn, J. M. H. (2018). Evaluating metrics of local topographic position for multiscale geomorphometric analysis. Geomorphology, 312, 40-50. 

### See Also

 

`DevFromMeanElev`, `DiffFromMeanElev`, `ElevPercentile`, `percent_elev_range` 

### Python API

```python
def relative_topographic_position(self, dem: Raster, filter_size_x: int = 11, filter_size_y: int = 11) -> Raster:
```
