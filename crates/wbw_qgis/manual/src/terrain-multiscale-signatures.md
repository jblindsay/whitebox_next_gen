# Multiscale Signatures


---

## Max Anisotropy Dev

**Function name:** `max_anisotropy_dev`


Calculates the maximum anisotropy (directionality) in elevation deviation over a range of spatial scales. 

### Python API

```python
def max_anisotropy_dev(self, dem: Raster, min_scale: int = 1, max_scale: int = 100, step_size: int = 1) -> Tuple[Raster, Raster]:
```


---

## Max Anisotropy Dev Signature

**Function name:** `max_anisotropy_dev_signature`


/.// 

### Python API

```python
def max_anisotropy_dev_signature(self, dem: Raster, points: Vector, output_html_file: str, min_scale: int = 1, max_scale: int = 100, step_size: int = 1) -> None:
```


---

## Max Difference From Mean

**Function name:** `max_difference_from_mean`


Calculates the maximum difference from mean elevation over a range of spatial scales. 

### Python API

```python
def max_difference_from_mean(self, dem: Raster, min_scale: int = 1, max_scale: int = 100, step_size: int = 1) -> Tuple[Raster, Raster]:
```


---

## Max Elevation Deviation

**Function name:** `max_elevation_deviation`


This tool can be used to calculate the maximum deviation from mean elevation, *DEVmax* (Lindsay et al. 2015) for each grid cell in a digital elevation model (DEM) across a range specified spatial scales. *DEV* is an elevation residual index and is essentially equivalent to a local elevation z-score. This attribute measures the *relative topographic position* as a fraction of local relief, and so is normalized to the local surface roughness. The multi-scaled calculation of *DEVmax* utilizes an integral image approach (Crow, 1984) to ensure highly efficient filtering that is invariant with filter size, which is the algorithm characteristic that allows for this densely sampled multi-scale analysis. In this way, `max_elevation_deviation` allows users to estimate the locally optimal scale with which to estimate *DEV* on a pixel-by-pixel basis. This multi-scaled version of local topographic position can reveal significant terrain characteristics and can aid with soil, vegetation, landform, and other mapping applications that depend on geomorphometric characterization. 

The user must input a digital elevation model (DEM) (`dem`). The range of scales that are evaluated in calculating *DEVmax* are determined by the user-specified `min_scale`, `max_scale`, and `step` parameters. All filter radii between the minimum and maximum scales, increasing by `step`, will be evaluated. The scale parameters are in units of grid cells and specify kernel size "radii" (*r*), such that:  

*d* = 2*r* + 1  

That is, a radii of 1, 2, 3... yields a square filters of dimension (*d*) 3 x 3, 5 x 5, 7 x 7... 

*DEV* is estimated at each tested filter size and every grid cell is assigned the maximum *DEV* value across the evaluated scales. 

Two output rasters will be generated, including the magnitude (*DEVmax*) and a second raster the assigns each pixel the scale at which *DEVmax* is encountered (*DEVscale*). The *DEVscale* raster can be very useful for revealing multi-scale landscape structure. 

### Reference

 

Lindsay J, Cockburn J, Russell H. 2015. An integral image approach to performing multi-scale topographic position analysis. Geomorphology, 245: 51-61. 

### See Also

 

`DevFromMeanElev`, `max_difference_from_mean`, `multiscale_elevation_percentile` 

### Python API

```python
def max_elevation_deviation(self, dem: Raster, min_scale: int = 1, max_scale: int = 100, step_size: int = 1) -> Tuple[Raster, Raster]:
```


---

## Max Elev Dev Signature

**Function name:** `max_elev_dev_signature`


Experimental

Calculates multiscale elevation-deviation signatures for input point sites and writes an HTML report.

geomorphometry terrain signature multiscale legacy-port

### Parameters

NameDescriptionRequiredDefault
`input`Input DEM raster path.Required`dem.tif`
`points`Input vector point or multipoint file path.Required`sites.geojson`
`min_scale`Minimum half-window radius in cells (default 1).Optional`1`
`max_scale`Maximum half-window radius in cells (default 100).Optional`100`
`step_size`Scale increment in cells (default 10). Alias: step.Optional`10`
`output`Optional output path for the HTML signature report.Optional—

### Examples

*Generate DEV signatures for a set of sample locations.*
`wbe.max_elev_dev_signature(input='dem.tif', max_scale=150, min_scale=1, output='max_elev_dev_signature.html', points='sites.geojson', step_size=5)`


---

## Multiscale Curvatures

**Function name:** `multiscale_curvatures`


### Description

 

This tool calculates several multiscale curvatures and curvature-based indices from an input DEM (`--dem`). There  are 18 curvature types (`--curv_type`) available, including: accumulation curvature, curvedness, difference curvature, Gaussian curvature, generating function, horizontal excess curvature, maximal curvature, mean curvature, minimal curvature, plan curvature, profile curvature, ring curvature, rotor, shape index, tangential curvature, total  curvature, unsphericity, and vertical excess curvature. Each of these curvatures can be measured in non-multiscale fashion using the corresponding tools available in either the WhiteboxTools open-core or the Whitebox extension. 

Like many of the multi-scale land-surface parameter tools available in Whitebox, this tool can be run in two different modes: it can either be used to measure curvature at a single specific scale or to generate a curvature *scale mosaic*. To understand the difference between these two modes, we must first understand how curvatures are measured and how the non-multiscale  curvature tools (e.g. `ProfileCurvature`) work. Curvatures are generally measured by fitting a mathematically defined surface to the elevation values within the local neighbourhood surrounding each grid cell in a DEM. The Whitebox curvature tools use the algorithms described Florinsky (2016), which use the 25 elevations within a 5 x 5 local neighbouhood for projected DEMs, and the nine elevations within a 3 x 3 neighbourhood for DEMs in geographic coordinate systems. This is what determines the scale at which these land-surface parameters are calculated. Because they are calculated using small local neighbourhoods (kernels), then these algorithms are heavily impacted by micro-topographic roughness and DEM noise. For example, in a fine-resolution DEM containing a great deal of micro-topographic roughness, the measured curvature value will be dominated by topographic variation at the scale of the roughness rather than the hillslopes on which that roughness is superimposed. This mis-matched scaling can be  a problem in many applications, e.g. in landform classification and slope failure modelling applications.  

Using the `MultiscaleCurvatures` tool, the user can specify a certain desired scale, larger than that defined by the grid resolution and kernel size, over which a curvature should be characterized. The tool will then use a fast  `Gaussian scale-space` method to remove the topographic variation in the DEM at scales less than the desired scale, and will then characterize the curvature using the usual method based on this scaled DEM. To measure curvature at a single non-local scale, the user must specify a minimum search neighbourhood  radius in grid cells (`--min_scale`) greater than 0.0. Note that a minimum search neighbourhood of 0.0 will replicate the non-multiscale equivalent curvature tool and any `--min_scale` value > 0.0 will apply the Gassian scale space method to eliminate  topographic variation less than the scale of the minimum search neighbourhood. The base step size (`--step`), number of steps (`--num_steps`), and step  nonlinearity (`--step_nonlinearity`) parameters should all be left to their default values of 1 in this case. The output curvature raster will be written to the output magnitude file (`--out_mag`). The following animation shows several multiscale curvature rasters (tangential curvature) measured from a DEM across a range of spatial scales. 

 

Alternatively, one can use this tool to create a curvature scale mosaic. In this case, the user specifies a range of spatial scales (i.e., a scale space) over which to measure curvature. The curvature scale-space is densely sampled and each grid cell is assigned the maximum absolute curvature value (for the specified curvature type) across the scale space. In this *scale-mosaic mode*, the user must also specify the output scale file name (`--out_scale`), which is an output raster that, for each grid cell, specifies the scale at which the maximum absolute curvature was identified. The following is an example of a scale mosaic of unsphericity for an area in Pole Canyon, Utah (`min_scale`=1.0,  `step`=1, `num_steps`=50, `step_nonlinearity`=1.0). 

 

Scale mosaics are useful when modelling spatial distributions of land-surface parameters, like curvatures, in complex and  heterogeneous landscapes that contain an abundance of topographic variation (micro-topography, landforms, etc.) at  widely varying spatial scales, often associated with different geomorphic processes. Notice how in the image above,  relatively strong curvature values are being characterized for both the landforms associated with the smaller-scale  mass-movement processes as well as the broader-scale fluvial erosion (i.e. valley incision and hillslopes). It would be  difficult, or impossible, to achieve this effect using a single, uniform scale. Each location in a land-surface  parameter scale mosaic represents the parameter measured at a *characteristic scale*, given the unique topography of  the site and surroundings. 

The properties of the sampled scale space are determined using the `--min_scale`, `--step`, `--num_steps` (greater than 1), and `--step_nonlinearity` parameters. Experience with multiscale curvature scales spaces has shown that they are more highly variable at shorter spatial scales and change more gradually at broader scales. Therefore, a nonlinear scale sampling interval is used by this  tool to ensure that the scale sampling density is higher for short scale ranges and coarser at longer tested scales,  such that:  

*ri* = *rL* + [step &times; (i - *rL*)]*p*  

Where *ri* is the filter radius for step *i* and *p* is the nonlinear scaling factor (`--step_nonlinearity`) and a step size (`--step`) of *step*. 

In scale-mosaic mode, the user must also decide whether or not to standardize the curvature values (`--standardize`). When this parameter is used, the algorithm will convert each curvature raster associated with each sampled region of scale-space to z-scores (i.e. differenced from the raster-wide mean and divided by the raster-wide standard  deviation). It it usually the case that curvature values measured at broader spatial scales will on the whole  become less strongly valued. Because the scale mosaic algorithm used in this tool assigns each grid cell the  maximum absolute curvature observed within sampled scale-space, this implies that the curvature values associated with more local-scale ranges are more likely to be selected for the final scale-mosaic raster. By standardizing each scaled curvature raster, there is greater opportunity for the final scale-mosaic to represent broader scale topographic variation. Whether or not this is appropriate will depend on the application. However, **it is important to stress that the sampled scale-space need not span the full range of possible scales, from the finest scale determined by the grid resolution up to the broadest scale possible, determined by the spatial extent of  the input DEM**. Often, a better approach is to use this tool to create multiple scale mosaics spanning this range, thereby capturing variation within broadly defined scale ranges. For example, one could create a local-scale, meso-scale, and broad-scale curvature scale mosaics, each of which would capture topographic variation and landforms that are present in the landscape and reflective of processing operating at vastly different spatial scales. When this approach is used, it may not be necessary to standardize each scaled curvature raster, since the gradual decline in curvature values as scales increase is less pronounced within each of these broad scale ranges than across the entirety of possible scale-space. Again, however, this will depend on the application and on the characteristics of the landscape at study. 

Raw curvedness values are often challenging to visualize given their range and magnitude,  and as such the user may opt to log-transform the output raster (`--log`). Transforming the values  applies the equation by Shary et al. (2002): 

*Θ*' = sign(*Θ*) ln(1 + 10*n*|*Θ*|)  

where *Θ* is the parameter value and *n* is dependent on the grid cell size. 

### References

 

Florinsky, I. (2016). Digital terrain analysis in soil science and geology. Academic Press. 

### See Also

 

`gaussian_scale_space`, `accumulation_curvature`, `curvedness`, `difference_curvature`, `gaussian_curvature`,  `generating_function`, `horizontal_excess_curvature`, `maximal_curvature`, `mean_curvature`, `minimal_curvature`, `plan_curvature`, `profile_curvature`, `ring_curvature`, `rotor`, `shape_index`, `tangential_curvature`,  `total_curvature`, `unsphericity`, `vertical_excess_curvature` 

### Python API

```python
def multiscale_curvatures(self, dem: Raster, curv_type: str = 'profile', min_scale: int = 4, step_size: int = 1, num_steps: int = 10, step_nonlinearity: float = 1.0, log_transform: bool = True, standardize: bool = False) -> Tuple[Raster, Raster]:
```


---

## Multiscale Elevated Index

**Function name:** `multiscale_elevated_index`


Experimental

Calculates multiscale elevated-index (MsEI) and key-scale rasters using Gaussian scale-space residuals.

geomorphometry multiscale gss elevated-index


---

## Multiscale Elevation Percentile

**Function name:** `multiscale_elevation_percentile`


This tool calculates the most elevation percentile (EP) across a range of spatial scales. EP is a measure of local topographic position (LTP) and expresses the vertical position for a digital elevation model (DEM) grid cell (z0) as the percentile of the elevation distribution within the filter window, such that:  

EP = counti&isin;C(zi > z0) x (100 / nC)  

where z0 is the elevation of the window's center grid cell, zi is the elevation of cell *i* contained within the neighboring set C, and nC is the number of grid cells contained within the window. 

EP is unsigned and expressed as a percentage, bound between 0% and 100%. This tool outputs two rasters, the multiscale EP magnitude (`out_mag`) and the scale at which the most extreme EP value occurs (`out_scale`). **The magnitude raster is the most extreme EP value (i.e. the furthest from 50%) for each grid cell encountered within the tested scales of EP.** 

Quantile-based estimates (e.g., the median and interquartile range) are often used in nonparametric statistics to provide data variability estimates without assuming the distribution is normal. Thus, EP is largely unaffected by irregularly shaped elevation frequency distributions or by outliers in the DEM, resulting in a highly robust metric of LTP. In fact, elevation distributions within small to medium sized neighborhoods often exhibit skewed, multimodal, and non-Gaussian distributions, where the occurrence of elevation errors can often result in distribution outliers. Thus, based on these statistical characteristics, EP is considered one of the most robust representation of LTP. 

The algorithm implemented by this tool uses the relatively efficient running-histogram filtering algorithm of Huang et al. (1979). Because most DEMs contain floating point data, elevation values must be rounded to be binned. The `sig_digits` parameter is used to determine the level of precision preserved during this binning process. The algorithm is parallelized to further aid with computational efficiency. 

Experience with multiscale EP has shown that it is highly variable at shorter scales and changes more gradually at broader scales. Therefore, a nonlinear scale sampling interval is used by this tool to ensure that the scale sampling density is higher for short scale ranges and coarser at longer tested scales, such that:  

*ri* = *rL* + [step &times; (i - *rL*)]*p*  

Where *ri* is the filter radius for step *i* and *p* is the nonlinear scaling factor (`step_nonlinearity`) and a step size (`step`) of *step*. 

### References

 

Newman, D. R., Lindsay, J. B., and Cockburn, J. M. H. (2018). Evaluating metrics of local topographic position for multiscale geomorphometric analysis. Geomorphology, 312, 40-50. 

Huang, T., Yang, G.J.T.G.Y. and Tang, G., 1979. A fast two-dimensional median filtering algorithm. IEEE Transactions on Acoustics, Speech, and Signal Processing, 27(1), pp.13-18. 

### See Also

 

`elevation_percentile`, `max_elevation_deviation`, `max_difference_from_mean` 

### Python API

```python
def multiscale_elevation_percentile(self, dem: Raster, num_significant_digits: int = 3, min_scale: int = 4, step_size: int = 1, num_steps: int = 10, step_nonlinearity: float = 1.0) -> Tuple[Raster, Raster]:
```


---

## Multiscale Low Lying Index

**Function name:** `multiscale_low_lying_index`


Experimental

Calculates multiscale low-lying-index (MsLLI) and key-scale rasters using Gaussian scale-space residuals.

geomorphometry multiscale gss low-lying-index


---

## Multiscale Roughness

**Function name:** `multiscale_roughness`


/ 

### Python API

```python
def multiscale_roughness(self, dem: Raster, min_scale: int = 1, max_scale: int = 100, step_size: int = 1) -> Tuple[Raster, Raster]:
```


---

## Multiscale Roughness Signature

**Function name:** `multiscale_roughness_signature`


/ 

### Python API

```python
def multiscale_roughness_signature(self, dem: Raster, points: Vector, output_html_file: str, min_scale: int = 1, max_scale: int = 100, step_size: int = 1) -> None:
```


---

## Multiscale Std Dev Normals

**Function name:** `multiscale_std_dev_normals`


This tool can be used to map the spatial pattern of maximum spherical standard deviation (σ*s max*; `out_mag`), as well as the scale at which maximum spherical standard deviation occurs (*rmax*; `out_scale`), for each grid cell in an input DEM (`dem`). This serves as a multi-scale measure of surface roughness, or topographic complexity. The spherical standard deviation (σs) is a measure of the angular spread among *n* unit vectors and is defined as:  

σs = &radic;[-2ln(*R* / *N*)] &times; 180 / &pi;  

Where *R* is the resultant vector length and is derived from the sum of the *x*, *y*, and *z* components of each of the *n* normals contained within a filter kernel, which designates a tested spatial scale. Each unit vector is a 3-dimensional measure of the surface orientation and slope at each grid cell center. The maximum spherical standard deviation is:  

σ*s max*=*max*{σs(*r*):*r*=*rL*...*rU*},  

Experience with roughness scale signatures has shown that σ*s max* is highly variable at shorter scales and changes more gradually at broader scales. Therefore, a nonlinear scale sampling interval is used by this tool to ensure that the scale sampling density is higher for short scale ranges and coarser at longer tested scales, such that:  

*ri* = *rL* + [step &times; (i - *rL*)]*p*  

Where *ri* is the filter radius for step *i* and *p* is the nonlinear scaling factor (`step_nonlinearity`) and a step size (`step`) of *step*. 

Use the `spherical_std_dev_of_normals` tool if you need to calculate σs for a single scale. 

### Reference

 

JB Lindsay, DR Newman, and A  Francioni. 2019 Scale-Optimized Surface Roughness for Topographic Analysis. *Geosciences*, 9(322) doi: 10.3390/geosciences9070322. 

### See Also

 

`spherical_std_dev_of_normals`, `multiscale_std_dev_normals_signature`, `multiscale_roughness` 

### Python API

```python
def multiscale_std_dev_normals(self, dem: Raster, min_scale: int = 4, step_size: int = 1, num_steps: int = 10, step_nonlinearity: float = 1.0, html_signature_file: str = "") -> Tuple[Raster, Raster]:
```


---

## Multiscale Std Dev Normals Signature

**Function name:** `multiscale_std_dev_normals_signature`


/ 

### Python API

```python
def multiscale_std_dev_normals_signature(self, dem: Raster, points: Vector, output_html_file: str, min_scale: int = 4, step_size: int = 1, num_steps: int = 10, step_nonlinearity: float = 1.0) -> None:
```


---

## Multiscale Topographic Position Image

**Function name:** `multiscale_topographic_position_image`


This tool creates a multiscale topographic position (MTP) image (`see here for an example`) from three DEVmax rasters of differing spatial scale ranges. Specifically, `multiscale_topographic_position_image` takes three DEVmax *magnitude* rasters, created using the `max_elevation_deviation` tool, as inputs. The three inputs should correspond to the elevation deviations in the local (`local`), meso (`meso`), and broad (`broad`) scale ranges and will be forced into the blue, green, and red colour components of the colour composite output (`output`) raster. The image lightness value (`lightness`) controls the overall brightness of the output image, as depending on the topography and scale ranges, these images can appear relatively dark. Higher values result in brighter, more colourful output images. 

The user may optionally specify an input hillshade raster. When specified, the hillshade will be used to provide a shaded-relief overlaid on top of the coloured multi-scale information, providing a very effective visualization. Any hillshade image may be used for this purpose, but we have found that multi-directional hillshade (`multidirectional_hillshade`), and  specifically those derived using the 360-degree option, can be most effective for this application. However,  experimentation is likely needed to find the optimal for each unique data set. 

The output images can take some training to interpret correctly and a detailed explanation can be found in Lindsay et al. (2015). Sites within the landscape that occupy prominent topographic positions, either low-lying or elevated, will be apparent by their bright colouring in the MTP image. Those that are coloured more strongly in the blue are promient at the local scale range; locations that are more strongly green coloured are promient at the meso scale; and bright reds in the MTP image are associated with broad-scale landscape prominence. Of course, combination colours are also possible when topography is elevated or low-lying across multiple scale ranges. For example, a yellow area would indicated a site of prominent topographic position across the meso and broadest scale ranges. 

### Reference

 

Lindsay J, Cockburn J, Russell H. 2015. An integral image approach to performing multi-scale topographic position analysis. Geomorphology, 245: 51-61. 

### See Also

 

`max_elevation_deviation` 

### Python API

```python
def multiscale_topographic_position_image(self, local: Raster, meso: Raster, broad: Raster, lightness: float = 1.2) -> Raster:
```


---

## Topographic Position Animation

**Function name:** `topographic_position_animation`


PROExperimental

Creates an interactive HTML viewer and animated GIF of DEV or DEVmax across nonlinearly sampled scales.

geomorphometry terrain topographic-position animation integral-image legacy-port

### Examples

*Animate terrain topographic position through a sequence of DEV scales.*
`wbe.topographic_position_animation(input='dem.tif', num_steps=8, output='topographic_position_animation.html', use_dev_max=True)`
