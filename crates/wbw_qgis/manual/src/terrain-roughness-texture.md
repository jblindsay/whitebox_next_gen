# Roughness and Texture


---

## Average Normal Vector Angular Deviation

**Function name:** `average_normal_vector_angular_deviation`


This tool characterizes the spatial distribution of the average normal vector angular deviation, a measure of surface roughness. Working in the field of 3D printing, Ko et al. (2016) defined a measure of surface roughness based on quantifying the angular deviations in the direction of the normal vector of a real surface from its ideal (i.e. smoothed) form. This measure of surface complexity is therefore in units of degrees. Specifically, roughness is defined in this study as the neighborhood-averaged difference in the normal vectors of the original DEM and a smoothed DEM surface. Smoothed surfaces are derived by applying a Gaussian blur of the same size as the neighborhood (`filter`). 

The `multiscale_roughness` tool calculates the same measure of surface roughness, except that it is designed to work with multiple spatial scales. 

### Reference

 

Ko, M., Kang, H., ulrim Kim, J., Lee, Y., & Hwang, J. E. (2016, July). How to measure quality of affordable 3D printing: Cultivating quantitative index in the user community. In International Conference on Human-Computer Interaction (pp. 116-121). Springer, Cham. 

Lindsay, J. B., & Newman, D. R. (2018). Hyper-scale analysis of surface roughness. PeerJ Preprints, 6, e27110v1. 

### See Also

 

`multiscale_roughness`, `spherical_std_dev_of_normals`, `circular_variance_of_aspect` 

### Python API

```python
def average_normal_vector_angular_deviation(self, dem: Raster, filter_size: int = 11) -> Raster:
```


---

## Circular Variance Of Aspect

**Function name:** `circular_variance_of_aspect`


This tool can be used to calculate the circular variance (i.e. one minus the mean resultant length) of aspect for a digital elevation model (DEM). This is a measure of how variable slope aspect is within a local neighbourhood of a specified size (`filter`). `circular_variance_of_aspect` is therefore a measure of **surface shape complexity**, or texture. It will take a value of 0.0 for smooth sites and near 1.0 in areas of high surface roughness or complex topography. 

The local neighbourhood size (`filter`) must be any odd integer equal to or greater than three. Grohmann et al. (2010) found that vector dispersion, a related measure of angular variance, increases monotonically with scale. This is the result of the angular dispersion measure integrating (accumulating) all of the surface variance of smaller scales up to the test scale. A more interesting scale relation can therefore be estimated by isolating the amount of surface complexity associated with specific scale ranges. That is, at large spatial scales, the metric should reflect the texture of large-scale landforms rather than the accumulated complexity at all smaller scales, including microtopographic roughness. As such, ***this tool normalizes the surface complexity of scales that are smaller than the filter size by applying Gaussian blur*** (with a standard deviation of one-third the filter size) to the DEM prior to calculating `circular_variance_of_aspect`. In this way, the resulting distribution is able to isolate and highlight the surface shape complexity associated with landscape features of a similar scale to that of the filter size. 

This tool makes extensive use of `integral images` (i.e. summed-area tables) and parallel processing to ensure computational efficiency. It may, however, require substantial memory resources when applied to larger DEMs. 

### References

 

Grohmann, C. H., Smith, M. J., & Riccomini, C. (2010). Multiscale analysis of topographic surface roughness in the Midland Valley, Scotland. *IEEE Transactions on Geoscience and Remote Sensing*, 49(4), 1200-1213. 

### See Also

 

`aspect`, `spherical_std_dev_of_normals`, `multiscale_roughness`, `edge_density`, `surface_area_ratio`, `ruggedness_index` 

### Python API

```python
def circular_variance_of_aspect(self, dem: Raster, filter_size: int = 11) -> Raster:
```


---

## Edge Density

**Function name:** `edge_density`


This tool calculates the density of edges, or breaks-in-slope within an input digital elevation model (DEM). A break-in-slope occurs between two neighbouring grid cells if the angular difference between their normal vectors is greater than a user-specified threshold value (`norm_diff`). `edge_density` calculates the proportion of edge cells within the neighbouring window, of square filter dimension `filter`, surrounding each grid cell. Therefore, `EdgeDensity`is a measure of how complex the topographic surface is within a local neighbourhood. It is therefore a measure of topographic texture. It will take a value near 0.0 for smooth sites and 1.0 in areas of high surface roughness or complex topography. 

The distribution of `edge_density` is highly dependent upon the value of the `norm_diff` used in the calculation. This threshold may require experimentation to find an appropriate value and is likely dependent upon the topography and source data. Nonetheless, experience has shown that `edge_density` provides one of the best measures of surface texture of any of the available roughness tools. 

### See Also

 

`circular_variance_of_aspect`, `multiscale_roughness`, `surface_area_ratio`, `ruggedness_index` 

### Python API

```python
def edge_density(self, dem: Raster, filter_size: int = 11, normal_diff_threshold: float = 5.0, z_factor: float = 1.0) -> Raster:
```


---

## Ruggedness Index

**Function name:** `ruggedness_index`


The terrain ruggedness index (TRI) is a measure of local topographic relief. The TRI calculates the root-mean-square-deviation (RMSD) for each grid cell in a digital elevation model (DEM), calculating the residuals (i.e. elevation differences) between a grid cell and its eight neighbours. Notice that, unlike the output of this tool, the original Riley et al. (1999) TRI did not normalize for the number of cells in the local window (i.e. it is a root-square-deviation only). However, using the mean has the advantage of allowing for the varying number of neighbouring cells along the grid edges and in areas bordering NoData cells. This modification does however imply that the output of this tool cannot be directly compared with the index ranges of level to extremely rugged terrain provided in Riley et al. (1999) 

### Reference

 

Riley, S. J., DeGloria, S. D., and Elliot, R. (1999). Index that quantifies topographic heterogeneity. *Intermountain Journal of Sciences*, 5(1-4), 23-27. 

### See Also

 

`relative_topographic_position`, `DevFromMeanElev` 

### Python API

```python
def ruggedness_index(self, input: Raster) -> Raster:
```


---

## Spherical Std Dev Of Normals

**Function name:** `spherical_std_dev_of_normals`


This tool can be used to calculate the spherical standard deviation of the distribution of surface normals for an input digital elevation model (DEM; `dem`). This is a measure of the angular dispersion of the surface normal vectors within a local neighbourhood of a specified size (`filter`). `spherical_std_dev_of_normals` is therefore a measure of surface shape complexity, texture, and roughness. The ` spherical standard deviation` (*s*) is defined as:  

*s* = &radic;[-2ln(*R* / *N*)] &times; 180 / &pi;  

where *R* is the resultant vector length and *N* is the number of unit normal vectors within the local neighbourhood. *s* is measured in degrees and is zero for simple planes and increases infinitely with increasing surface complexity or roughness. Note that this formulation of the spherical standard deviation assumes an underlying wrapped normal distribution. 

The local neighbourhood size (`filter`) must be any odd integer equal to or greater than three. Grohmann et al. (2010) found that vector dispersion, a related measure of angular dispersion, increases monotonically with scale. This is the result of the angular dispersion measure integrating (accumulating) all of the surface variance of smaller scales up to the test scale. A more interesting scale relation can therefore be estimated by isolating the amount of surface complexity associated with specific scale ranges. That is, at large spatial scales, *s* should reflect the texture of large-scale landforms rather than the accumulated complexity at all smaller scales, including microtopographic roughness. As such, ***this tool normalizes the surface complexity of scales that are smaller than the filter size by applying Gaussian blur*** (with a standard deviation of one-third the filter size) to the DEM prior to calculating *R*. In this way, the resulting distribution is able to isolate and highlight the surface shape complexity associated with landscape features of a similar scale to that of the filter size. 

This tool makes extensive use of `integral images` (i.e. summed-area tables) and parallel processing to ensure computational efficiency. It may, however, require substantial memory resources when applied to larger DEMs. 

### References

 

Grohmann, C. H., Smith, M. J., & Riccomini, C. (2010). Multiscale analysis of topographic surface roughness in the Midland Valley, Scotland. *IEEE Transactions on Geoscience and Remote Sensing*, 49(4), 1200-1213. 

Hodgson, M. E., and Gaile, G. L. (1999). A cartographic modeling approach for surface orientation-related applications. *Photogrammetric Engineering and Remote Sensing*, 65(1), 85-95. 

Lindsay J. B., Newman* D. R., Francioni, A. 2019. Scale-optimized surface roughness for topographic analysis. *Geosciences*,  9(7) 322. DOI: 10.3390/geosciences9070322. 

### See Also

 

`circular_variance_of_aspect`, `multiscale_roughness`, `edge_density`, `surface_area_ratio`, `ruggedness_index` 

### Python API

```python
def spherical_std_dev_of_normals(self, dem: Raster, filter_size: int = 11) -> Raster:
```


---

## Standard Deviation Of Slope

**Function name:** `standard_deviation_of_slope`


Calculates the standard deviation of slope from an input DEM, a metric of roughness described by Grohmann et al., (2011). 

### Python API

```python
def standard_deviation_of_slope(self, dem: Raster, filter_size: int = 11, z_factor: float = 1.0) -> Raster:
```
