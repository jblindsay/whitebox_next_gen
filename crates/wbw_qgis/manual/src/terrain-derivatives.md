# Terrain Derivatives


---

## Accumulation Curvature

**Function name:** `accumulation_curvature`


### Description

 

This tool calculates the accumulation curvature from a digital elevation model (DEM). Accumulation curvature is the product of profile (vertical) and tangential (horizontal) curvatures at a location (Shary, 1995). This variable has positive values, zero or greater. Florinsky (2017) states that accumulation curvature is a measure of the extent of local accumulation of flows at a given point in the topographic surface. Accumulation curvature is measured in units of m-2. 

 

The user must specify the name of the input DEM (`dem`) and the output raster (`output`). The The Z conversion factor (`zfactor`) is only important when the vertical and horizontal units are not the same in the DEM. When this is the case, the algorithm will multiply each elevation in the DEM by the Z Conversion Factor. Curvature values are often very small and as such the user may opt to log-transform the output raster (`log`). Transforming the values applies the equation by Shary et al. (2002): 

*Θ*' = sign(*Θ*) ln(1 + 10*n*|*Θ*|) 

where *Θ* is the parameter value and *n* is dependent on the grid cell size. 

For DEMs in projected coordinate systems, the tool uses the 3rd-order bivariate Taylor polynomial method described by Florinsky (2016). Based on a polynomial fit of the elevations within the 5x5 neighbourhood surrounding each cell, this method is considered more robust against outlier elevations (noise) than other methods. For DEMs in geographic coordinate systems (i.e. angular units), the tool uses the 3x3 polynomial fitting method for equal angle grids also described by Florinsky (2016). 

### References

 

Florinsky, I. (2016). Digital terrain analysis in soil science and geology. Academic Press. 

Florinsky, I. V. (2017). An illustrated introduction to general geomorphometry. Progress in Physical Geography, 41(6), 723-752. 

Shary PA (1995) Land surface in gravity points classification by a complete system of curvatures. Mathematical Geology 27: 373–390. 

Shary P. A., Sharaya L. S. and Mitusov A. V. (2002) Fundamental quantitative methods of land surface analysis. Geoderma 107: 1–32. 

### See Also

 

`tangential_curvature`, `profile_curvature`, `minimal_curvature`, `maximal_curvature`, `mean_curvature`, `gaussian_curvature` 

### Python API

```python
def accumulation_curvature(self, dem: Raster, log_transform: bool = False, z_factor: float = 1.0) -> Raster:
```


---

## Aspect

**Function name:** `aspect`


This tool calculates slope aspect (i.e. slope orientation in degrees clockwise from north) for each grid cell in an input digital elevation model (DEM). The user must specify the name of the input DEM (`dem`) and the  output raster (`output`). The Z conversion factor (`zfactor`) is only important when the vertical and  horizontal units are not the same in the DEM, and the DEM is in a projected coordinate system. When this is the case, the algorithm will multiply each elevation  in the DEM by the Z Conversion Factor to perform the unit conversion.  

For DEMs in projected coordinate systems, the tool uses the 3rd-order bivariate Taylor polynomial method described by Florinsky (2016). Based on a polynomial fit of the elevations within the 5x5 neighbourhood surrounding each cell, this method is considered more robust against outlier elevations (noise) than other methods. For DEMs in geographic coordinate systems (i.e. angular units), the tool uses the 3x3 polynomial fitting method for equal angle grids also described by Florinsky (2016). 

### Reference

 

Florinsky, I. (2016). Digital terrain analysis in soil science and geology. Academic Press. 

Gallant, J. C., and J. P. Wilson, 2000, Primary topographic attributes, in Terrain Analysis: Principles and Applications, edited by J. P. Wilson and J. C. Gallant pp. 51-86, John Wiley, Hoboken, N.J. 

### See Also

 

`slope`, `plan_curvature`, `profile_curvature` 

### Python API

```python
def aspect(self, dem: Raster, z_factor: float = 1.0) -> Raster:
```


---

## Casorati Curvature

**Function name:** `casorati_curvature`


Experimental

Calculates Casorati curvature from a DEM.

geomorphometry terrain curvature casorati_curvature legacy-port

### Parameters

NameDescriptionRequiredDefault
`input`Input DEM raster path or typed raster object.Required`dem.tif`
`z_factor`Optional z conversion factor (default 1.0). Alias: zfactor.Optional`1.0`
`log_transform`Optional log-transform of output values (default false). Alias: log.Optional`False`
`output`Optional output path. If omitted, result is stored in memory.Optional—

### Examples

*Calculates casorati_curvature from a DEM.*
`wbe.casorati_curvature(input='dem.tif', log_transform=False, output='casorati_curvature.tif', z_factor=1.0)`


---

## Curvedness

**Function name:** `curvedness`


### Description

 

This tool calculates the curvedness (Koenderink and van Doorn, 1992) from a digital elevation model (DEM). Curvedness is the root mean square of maximal and minimal curvatures, and measures the magnitude of surface bending, regardless of shape (Florinsky, 2017). Curvedness is characteristically low-values for flat areas and higher for areas of sharp bending (Florinsky, 2017). The index is also inversely proportional with the size of the object (Koenderink and van Doorn, 1992). Curvedness has values equal to or greater than zero and is measured in units of m-1. 

 

The user must specify the name of the input DEM (`dem`) and the output raster (`output`). The The Z conversion factor (`zfactor`) is only important when the vertical and horizontal units are not the same in the DEM. When this is the case, the algorithm will multiply each elevation in the DEM by the Z Conversion Factor. Raw curvedness values are often challenging to visualize given their range and magnitude, and as such the user may opt to log-transform the output raster (`log`). Transforming the values applies the equation by Shary et al. (2002): 

*Θ*' = sign(*Θ*) ln(1 + 10*n*|*Θ*|) 

where *Θ* is the parameter value and *n* is dependent on the grid cell size. 

For DEMs in projected coordinate systems, the tool uses the 3rd-order bivariate Taylor polynomial method described by Florinsky (2016). Based on a polynomial fit of the elevations within the 5x5 neighbourhood surrounding each cell, this method is considered more robust against outlier elevations (noise) than other methods. For DEMs in geographic coordinate systems (i.e. angular units), the tool uses the 3x3 polynomial fitting method for equal angle grids also described by Florinsky (2016). 

### References

 

Florinsky, I. (2016). Digital terrain analysis in soil science and geology. Academic Press. 

Florinsky, I. V. (2017). An illustrated introduction to general geomorphometry. Progress in Physical Geography, 41(6), 723-752. 

Koenderink, J. J., and Van Doorn, A. J. (1992). Surface shape and curvature scales. Image and vision computing, 10(8), 557-564. 

Shary P. A., Sharaya L. S. and Mitusov A. V. (2002) Fundamental quantitative methods of land surface analysis. Geoderma 107: 1–32. 

### See Also

 

`shape_index`, `minimal_curvature`, `maximal_curvature`, `tangential_curvature`, `profile_curvature`, `mean_curvature`, `gaussian_curvature` 

### Python API

```python
def curvedness(self, dem: Raster, log_transform: bool = False, z_factor: float = 1.0) -> Raster:
```


---

## Difference Curvature

**Function name:** `difference_curvature`


### Description

 

This tool calculates the difference curvature from a digital elevation model (DEM). Difference curvature is half of the difference between profile and tangential curvatures, sometimes called the vertical and horizontal curvatures (Shary, 1995). This variable has an unbounded range that can take either positive or negative values. Florinsky (2017) states that difference curvature measures the extent to which the relative deceleration of flows (measured by kv) is higher than flow convergence at a given point of the topographic surface. Difference curvature is measured in units of m-1. 

 

The user must specify the name of the input DEM (`dem`) and the output raster (`output`). The The Z conversion factor (`zfactor`) is only important when the vertical and horizontal units are not the same in the DEM. When this is the case, the algorithm will multiply each elevation in the DEM by the Z Conversion Factor. Curvature values are often very small and as such the user may opt to log-transform the output raster (`log`). Transforming the values applies the equation by Shary et al. (2002): 

*Θ*' = sign(*Θ*) ln(1 + 10*n*|*Θ*|) 

where *Θ* is the parameter value and *n* is dependent on the grid cell size. 

For DEMs in projected coordinate systems, the tool uses the 3rd-order bivariate Taylor polynomial method described by Florinsky (2016). Based on a polynomial fit of the elevations within the 5x5 neighbourhood surrounding each cell, this method is considered more robust against outlier elevations (noise) than other methods. For DEMs in geographic coordinate systems (i.e. angular units), the tool uses the 3x3 polynomial fitting method for equal angle grids also described by Florinsky (2016). 

### References

 

Florinsky, I. (2016). Digital terrain analysis in soil science and geology. Academic Press. 

Florinsky, I. V. (2017). An illustrated introduction to general geomorphometry. Progress in Physical Geography, 41(6), 723-752. 

Shary PA (1995) Land surface in gravity points classification by a complete system of curvatures. Mathematical Geology 27: 373–390. 

Shary P. A., Sharaya L. S. and Mitusov A. V. (2002) Fundamental quantitative methods of land surface analysis. Geoderma 107: 1–32. 

### See Also

 

`profile_curvature`, `tangential_curvature`, `rotor`, `minimal_curvature`, `maximal_curvature`, `mean_curvature`, `gaussian_curvature` 

### Python API

```python
def difference_curvature(self, dem: Raster, log_transform: bool = False, z_factor: float = 1.0) -> Raster:
```


---

## Gaussian Curvature

**Function name:** `gaussian_curvature`


This tool calculates the Gaussian curvature from a digital elevation model (DEM). Gaussian curvature  is the product of maximal and minimal curvatures, and retains values in each point of the topographic  surface after its bending without breaking, stretching, and compressing (Florinsky, 2017). Gaussian  curvature is measured in units of m-2. 

 

The user must input a DEM (`dem`).The Z conversion factor (`zfactor`) is only important when the vertical and horizontal units are not the same in the DEM. When this is the case, the algorithm will multiply each elevation in the DEM by the Z Conversion Factor. Curvature values are often very small and as such the user may opt to log-transform the output raster (`log`). Transforming the values applies the equation by Shary et al. (2002): 

*Θ*' = sign(*Θ*) ln(1 + 10*n*|*Θ*|)  

where *Θ* is the parameter value and *n* is dependent on the grid cell size. 

For DEMs in projected coordinate systems, the tool uses the 3rd-order bivariate  Taylor polynomial method described by Florinsky (2016). Based on a polynomial fit  of the elevations within the 5x5 neighbourhood surrounding each cell, this method is considered more  robust against outlier elevations (noise) than other methods. For DEMs in geographic coordinate systems (i.e. angular units), the tool uses the 3x3 polynomial fitting method for equal angle grids also  described by Florinsky (2016).  

### References

 

Florinsky, I. (2016). Digital terrain analysis in soil science and geology. Academic Press. 

Florinsky, I. V. (2017). An illustrated introduction to general geomorphometry. Progress in Physical  Geography, 41(6), 723-752. 

Shary P. A., Sharaya L. S. and Mitusov A. V. (2002) Fundamental quantitative methods of land surface analysis.  Geoderma 107: 1–32. 

### See Also

 

`tangential_curvature`, `profile_curvature`, `plan_curvature`, `mean_curvature`, `minimal_curvature`, `maximal_curvature` 

### Python API

```python
def gaussian_curvature(self, dem: Raster, log_transform: bool = False, z_factor: float = 1.0) -> Raster:
```


---

## Generating Function

**Function name:** `generating_function`


### Description

 

This tool calculates the generating function (Shary and Stepanov, 1991) from a digital elevation model (DEM). Florinsky (2016) describes generating function as a measure for the deflection of tangential curvature from loci of extreme curvature of the topographic surface. Florinsky (2016) demonstrated the application of this variable for identifying landscape structural lines, i.e. ridges and thalwegs, for which the generating function takes values near zero. Ridges coincide with divergent areas where generating function is approximately zero, while thalwegs are associated with convergent areas with generating function values near zero. This variable has positive values, zero or greater and is measured in units of m-2. 

 

The user must specify the name of the input DEM (`dem`) and the output raster (`output`). The The Z conversion factor (`zfactor`) is only important when the vertical and horizontal units are not the same in the DEM. When this is the case, the algorithm will multiply each elevation in the DEM by the Z Conversion Factor. Raw generating function values are often challenging to visualize given their range and magnitude, and as such the user may opt to log-transform the output raster (`log`). Transforming the values applies the equation by Shary et al. (2002): 

*Θ*' = sign(*Θ*) ln(1 + 10*n*|*Θ*|) 

where *Θ* is the parameter value and *n* is dependent on the grid cell size. 

This tool uses the 3rd-order bivariate Taylor polynomial method described by Florinsky (2016). Based on a polynomial fit of the elevations within the 5x5 neighbourhood surrounding each cell, this method is considered more robust against outlier elevations (noise) than other methods. For DEMs in geographic coordinate systems, however, this tool cannot use the same  3x3 polynomial fitting method for equal angle grids, also described by Florinsky (2016), that is used by the other curvature tools in this software. That is because generating function uses 3rd order partial derivatives, which cannot be calculated using the 9 elevations in a 3x3; more elevation values are required (i.e. a 5x5 window). Thus, this tool uses the same 5x5 method used for DEMs in projected coordinate systems, and calculates the average linear distance between neighbouring cells in the vertical and horizontal directions using the Vincenty distance function. Note that this may cause a notable slow-down in algorithm performance and has a lower accuracy than would be achieved using an equal angle method, because it assumes a square pixel (in linear units). 

### References

 

Florinsky, I. (2016). Digital terrain analysis in soil science and geology. Academic Press. 

Florinsky, I. V. (2017). An illustrated introduction to general geomorphometry. Progress in Physical Geography, 41(6), 723-752. 

Koenderink, J. J., and Van Doorn, A. J. (1992). Surface shape and curvature scales. Image and vision computing, 10(8), 557-564. 

Shary P. A., Sharaya L. S. and Mitusov A. V. (2002) Fundamental quantitative methods of land surface analysis. Geoderma 107: 1–32. 

Shary P. A. and Stepanov I. N. (1991) Application of the method of second derivatives in geology. Transactions (Doklady) of the USSR Academy of Sciences, Earth Science Sections 320: 87–92. 

### See Also

 

`shape_index`, `minimal_curvature`, `maximal_curvature`, `tangential_curvature`, `profile_curvature`, `mean_curvature`, `gaussian_curvature` 

### Python API

```python
def generating_function(self, dem: Raster, log_transform: bool = False, z_factor: float = 1.0) -> Raster:
```


---

## Horizontal Excess Curvature

**Function name:** `horizontal_excess_curvature`


### Description

 

This tool calculates the horizontal excess curvature from a digital elevation model (DEM). Horizontal excess curvature is the difference of tangential (horizontal) and minimal curvatures at a location (Shary, 1995). This variable has positive values, zero or greater. Florinsky (2017) states that horizontal excess curvature measures the extent to which the bending of a normal section tangential to a contour line is larger than the minimal bending at a given point of the surface. Horizontal excess curvature is measured in units of m-1. 

 

The user must specify the name of the input DEM (`dem`) and the output raster (`output`). The The Z conversion factor (`zfactor`) is only important when the vertical and horizontal units are not the same in the DEM. When this is the case, the algorithm will multiply each elevation in the DEM by the Z Conversion Factor. Curvature values are often very small and as such the user may opt to log-transform the output raster (`log`). Transforming the values applies the equation by Shary et al. (2002): 

*Θ*' = sign(*Θ*) ln(1 + 10*n*|*Θ*|) 

where *Θ* is the parameter value and *n* is dependent on the grid cell size. 

For DEMs in projected coordinate systems, the tool uses the 3rd-order bivariate Taylor polynomial method described by Florinsky (2016). Based on a polynomial fit of the elevations within the 5x5 neighbourhood surrounding each cell, this method is considered more robust against outlier elevations (noise) than other methods. For DEMs in geographic coordinate systems (i.e. angular units), the tool uses the 3x3 polynomial fitting method for equal angle grids also described by Florinsky (2016). 

### References

 

Florinsky, I. (2016). Digital terrain analysis in soil science and geology. Academic Press. 

Florinsky, I. V. (2017). An illustrated introduction to general geomorphometry. Progress in Physical Geography, 41(6), 723-752. 

Shary PA (1995) Land surface in gravity points classification by a complete system of curvatures. Mathematical Geology 27: 373–390. 

Shary P. A., Sharaya L. S. and Mitusov A. V. (2002) Fundamental quantitative methods of land surface analysis. Geoderma 107: 1–32. 

### See Also

 

`tangential_curvature`, `profile_curvature`, `minimal_curvature`, `maximal_curvature`, `mean_curvature`, `gaussian_curvature` 

### Python API

```python
def horizontal_excess_curvature(self, dem: Raster, log_transform: bool = False, z_factor: float = 1.0) -> Raster:
```


---

## Maximal Curvature

**Function name:** `maximal_curvature`


This tool calculates the maximal curvature from a digital elevation model (DEM). Maximal curvature  is the curvature of a principal section with the highest value of curvature at a given point of the  topographic surface (Florinsky, 2017). The values of this curvature are unbounded, and positive  values correspond to ridge positions while negative values are indicative of closed depressions  (Florinsky, 2016). Maximal curvature is measured in units of m-1. 

 

The user must input a DEM (`dem`). The Z conversion factor (`zfactor`) is only important when the vertical and horizontal units are not the same in the DEM. When this is the case, the algorithm will multiply each elevation in the DEM by the Z Conversion Factor. Curvature values are often very small and as such the user may opt to log-transform the output raster (`log`). Transforming the values applies the equation by Shary et al. (2002): 

*Θ*' = sign(*Θ*) ln(1 + 10*n*|*Θ*|)  

where *Θ* is the parameter value and *n* is dependent on the grid cell size. 

For DEMs in projected coordinate systems, the tool uses the 3rd-order bivariate  Taylor polynomial method described by Florinsky (2016). Based on a polynomial fit  of the elevations within the 5x5 neighbourhood surrounding each cell, this method is considered more  robust against outlier elevations (noise) than other methods. For DEMs in geographic coordinate systems (i.e. angular units), the tool uses the 3x3 polynomial fitting method for equal angle grids also  described by Florinsky (2016).  

### References

 

Florinsky, I. (2016). Digital terrain analysis in soil science and geology. Academic Press. 

Florinsky, I. V. (2017). An illustrated introduction to general geomorphometry. Progress in Physical  Geography, 41(6), 723-752. 

Shary P. A., Sharaya L. S. and Mitusov A. V. (2002) Fundamental quantitative methods of land surface analysis.  Geoderma 107: 1–32. 

`minimal_curvature`, `tangential_curvature`, `profile_curvature`, `plan_curvature`, `mean_curvature`, `gaussian_curvature` 

### Python API

```python
def maximal_curvature(self, dem: Raster, log_transform: bool = False, z_factor: float = 1.0) -> Raster:
```


---

## Mean Curvature

**Function name:** `mean_curvature`


This tool calculates the mean curvature, or the rate of change in slope along a flow line, from a digital elevation model (DEM). Curvature is the second derivative of the topographic surface defined by a DEM. Profile curvature characterizes the degree of downslope acceleration or deceleration within the landscape (Gallant and Wilson, 2000). The user must input a DEM (`dem`). WhiteboxTools reports curvature in radians multiplied by 100 for easier interpretation because curvature values are typically very small. The *Z conversion factor* (`zfactor`) is only important when the vertical and horizontal units are not the same in the DEM. When this is the case, the algorithm will multiply each elevation in the DEM by the Z Conversion Factor. If the DEM is in the geographic coordinate system (latitude and longitude), the following equation is used:  

zfactor = 1.0 / (111320.0 x cos(mid_lat))  

where `mid_lat` is the latitude of the centre of the raster, in radians. 

The algorithm uses the same formula for the calculation of plan curvature as Gallant and Wilson (2000). Profile curvature is negative for slope increasing downhill (convex flow profile, typical of upper slopes) and positive for slope decreasing downhill (concave, typical of lower slopes). 

### Reference

 

Gallant, J. C., and J. P. Wilson, 2000, Primary topographic attributes, in Terrain Analysis: Principles and Applications, edited by J. P. Wilson and J. C. Gallant pp. 51-86, John Wiley, Hoboken, N.J. 

### See Also

 

`profile_curvature`, `tangential_curvature`, `total_curvature`, `slope`, `aspect` 

### Python API

```python
def mean_curvature(self, dem: Raster, log_transform: bool = False, z_factor: float = 1.0) -> Raster:
```


---

## Minimal Curvature

**Function name:** `minimal_curvature`


This tool calculates the minimal curvature from a digital elevation model (DEM). Minimal curvature  is the curvature of a principal section with the lowest value of curvature at a given point of the  topographic surface (Florinsky, 2017). The values of this curvature are unbounded, and positive  values correspond to hills while negative values are indicative of valley positions (Florinsky, 2016).  Minimal curvature is measured in units of m-1. 

 

The user must input a DEM (`dem`). The Z conversion factor (`zfactor`) is only important when the vertical and horizontal units are not the same in the DEM. When this is the case, the algorithm will multiply each elevation in the DEM by the Z Conversion Factor. Curvature values are often very small and as such the user may opt to log-transform the output raster (`log`). Transforming the values applies the equation by Shary et al. (2002): 

*Θ*' = sign(*Θ*) ln(1 + 10*n*|*Θ*|)  

where *Θ* is the parameter value and *n* is dependent on the grid cell size. 

For DEMs in projected coordinate systems, the tool uses the 3rd-order bivariate  Taylor polynomial method described by Florinsky (2016). Based on a polynomial fit  of the elevations within the 5x5 neighbourhood surrounding each cell, this method is considered more  robust against outlier elevations (noise) than other methods. For DEMs in geographic coordinate systems (i.e. angular units), the tool uses the 3x3 polynomial fitting method for equal angle grids also  described by Florinsky (2016).  

### References

 

Florinsky, I. (2016). Digital terrain analysis in soil science and geology. Academic Press. 

Florinsky, I. V. (2017). An illustrated introduction to general geomorphometry. Progress in Physical  Geography, 41(6), 723-752. 

Shary P. A., Sharaya L. S. and Mitusov A. V. (2002) Fundamental quantitative methods of land surface analysis.  Geoderma 107: 1–32. 

`maximal_curvature`, `tangential_curvature`, `profile_curvature`, `plan_curvature`, `mean_curvature`, `gaussian_curvature` 

### Python API

```python
def minimal_curvature(self, dem: Raster, log_transform: bool = False, z_factor: float = 1.0) -> Raster:
```


---

## Plan Curvature

**Function name:** `plan_curvature`


This tool calculates the plan curvature (i.e. contour curvature), or the rate of change in aspect along a contour line, from a digital elevation model (DEM). Curvature is the second derivative of the topographic surface defined by a DEM. Plan curvature characterizes the degree of flow convergence or divergence within the landscape (Gallant and Wilson, 2000). The user must input a DEM (`dem`). WhiteboxTools reports curvature in degrees multiplied by 100 for easier interpretation. The *Z conversion factor* (`zfactor`) is only important when the vertical and horizontal units are not the same in the DEM. When this is the case, the algorithm will multiply each elevation in the DEM by the Z Conversion Factor. If the DEM is in the geographic coordinate system (latitude and longitude), the following equation is used:  

zfactor = 1.0 / (111320.0 x cos(mid_lat))  

where `mid_lat` is the latitude of the centre of the raster, in radians. 

The algorithm uses the same formula for the calculation of plan curvature as Gallant and Wilson (2000). Plan curvature is negative for diverging flow along ridges and positive for convergent areas, e.g. along valley bottoms. 

### Reference

 

Gallant, J. C., and J. P. Wilson, 2000, Primary topographic attributes, in Terrain Analysis: Principles and Applications, edited by J. P. Wilson and J. C. Gallant pp. 51-86, John Wiley, Hoboken, N.J. 

### See Also

 

`profile_curvature`, `tangential_curvature`, `total_curvature`, `slope`, `aspect` 

### Python API

```python
def plan_curvature(self, dem: Raster, log_transform: bool = False, z_factor: float = 1.0) -> Raster:
```


---

## Principal Curvature Direction

**Function name:** `principal_curvature_direction`


Experimental

Calculates the principal curvature direction angle (degrees).

geomorphometry terrain curvature principal_curvature_direction legacy-port

### Parameters

NameDescriptionRequiredDefault
`input`Input DEM raster path or typed raster object.Required`dem.tif`
`z_factor`Optional z conversion factor (default 1.0). Alias: zfactor.Optional`1.0`
`log_transform`Optional log-transform of output values (default false). Alias: log.Optional`False`
`output`Optional output path. If omitted, result is stored in memory.Optional—

### Examples

*Calculates principal_curvature_direction from a DEM.*
`wbe.principal_curvature_direction(input='dem.tif', log_transform=False, output='principal_curvature_direction.tif', z_factor=1.0)`


---

## Profile Curvature

**Function name:** `profile_curvature`


This tool calculates the profile curvature, or the rate of change in slope along a flow line, from a digital elevation model (DEM). Curvature is the second derivative of the topographic surface defined by a DEM. Profile curvature characterizes the degree of downslope acceleration or deceleration within the landscape (Gallant and Wilson, 2000). The user must input DEM a (`dem`). WhiteboxTools reports curvature in degrees multiplied by 100 for easier interpretation because curvature values are typically very small. The *Z conversion factor* (`zfactor`) is only important when the vertical and horizontal units are not the same in the DEM. When this is the case, the algorithm will multiply each elevation in the DEM by the Z Conversion Factor. If the DEM is in the geographic coordinate system (latitude and longitude), the following equation is used:  

zfactor = 1.0 / (111320.0 x cos(mid_lat))  

where `mid_lat` is the latitude of the centre of the raster, in radians. 

The algorithm uses the same formula for the calculation of plan curvature as Gallant and Wilson (2000). Profile curvature is negative for slope increasing downhill (convex flow profile, typical of upper slopes) and positive for slope decreasing downhill (concave, typical of lower slopes). 

### Reference

 

Gallant, J. C., and J. P. Wilson, 2000, Primary topographic attributes, in Terrain Analysis: Principles and Applications, edited by J. P. Wilson and J. C. Gallant pp. 51-86, John Wiley, Hoboken, N.J. 

### See Also

 

`profile_curvature`, `tangential_curvature`, `total_curvature`, `slope`, `aspect` 

### Python API

```python
def profile_curvature(self, dem: Raster, log_transform: bool = False, z_factor: float = 1.0) -> Raster:
```


---

## Relative Aspect

**Function name:** `relative_aspect`


This tool creates a new raster in which each grid cell is assigned the terrain aspect relative to a user-specified direction (`azimuth`). Relative terrain aspect is the angular distance (measured in degrees) between the land-surface aspect and the assumed regional wind azimuth (Bohner and Antonic, 2007). It is bound between 0-degrees (windward direction) and 180-degrees (leeward direction). Relative terrain aspect is the simplest of the measures of topographic exposure to wind, taking into account terrain orientation only and neglecting the influences of topographic shadowing by distant landforms and the deflection of wind by topography. 

The user must input a digital elevation model (DEM) (`dem`) and an azimuth (i.e. a wind direction). The Z Conversion Factor (`zfactor`) is only important when the vertical and horizontal units are not the same in the DEM, and the DEM is in a projected coordinate system. When this is the case, the algorithm will multiply each elevation in the DEM by the Z Conversion Factor. 

### Reference

 

Böhner, J., and Antonić, O. (2009). Land-surface parameters specific to topo-climatology. Developments in Soil Science, 33, 195-226. 

### See Also

 

`aspect` 

### Python API

```python
def relative_aspect(self, dem: Raster, azimuth: float = 0.0, z_factor: float = 1.0) -> Raster:
```


---

## Ring Curvature

**Function name:** `ring_curvature`


### Description

 

This tool calculates the ring curvature, which is the product of horizontal excess and vertical excess curvatures (Shary, 1995), from a digital elevation model (DEM). Like `rotor`, ring curvature is used to measure flow line twisting. Ring curvature has values equal to or greater than zero and is measured in units of m-2. 

 

The user must specify the name of the input DEM (`dem`) and the output raster (`output`). The The Z conversion factor (`zfactor`) is only important when the vertical and horizontal units are not the same in the DEM. When this is the case, the algorithm will multiply each elevation in the DEM by the Z Conversion Factor. Curvature values are often very small and as such the user may opt to log-transform the output raster (`log`). Transforming the values applies the equation by Shary et al. (2002): 

*Θ*' = sign(*Θ*) ln(1 + 10*n*|*Θ*|) 

where *Θ* is the parameter value and *n* is dependent on the grid cell size. 

For DEMs in projected coordinate systems, the tool uses the 3rd-order bivariate Taylor polynomial method described by Florinsky (2016). Based on a polynomial fit of the elevations within the 5x5 neighbourhood surrounding each cell, this method is considered more robust against outlier elevations (noise) than other methods. For DEMs in geographic coordinate systems (i.e. angular units), the tool uses the 3x3 polynomial fitting method for equal angle grids also described by Florinsky (2016). 

### References

 

Florinsky, I. (2016). Digital terrain analysis in soil science and geology. Academic Press. 

Florinsky, I. V. (2017). An illustrated introduction to general geomorphometry. Progress in Physical Geography, 41(6), 723-752. 

Shary PA (1995) Land surface in gravity points classification by a complete system of curvatures. Mathematical Geology 27: 373–390. 

Shary P. A., Sharaya L. S. and Mitusov A. V. (2002) Fundamental quantitative methods of land surface analysis. Geoderma 107: 1–32. 

### See Also

 

`rotor`, `minimal_curvature`, `maximal_curvature`, `mean_curvature`, `gaussian_curvature`, `profile_curvature`, `tangential_curvature` 

### Python API

```python
def ring_curvature(self, dem: Raster, log_transform: bool = False, z_factor: float = 1.0) -> Raster:
```


---

## Rotor

**Function name:** `rotor`


### Description

 

This tool calculates the spatial pattern of rotor, which describes the degree to which a flow line twists (Shary, 1991), from a digital elevation model (DEM). Rotor has an unbounded range, with positive values indicating that a flow line turns clockwise and negative values indicating flow lines that turn counter clockwise (Florinsky, 2017). Rotor is measured in units of m-1. 

 

The user must specify the name of the input DEM (`dem`) and the output raster (`output`). The The Z conversion factor (`zfactor`) is only important when the vertical and horizontal units are not the same in the DEM. When this is the case, the algorithm will multiply each elevation in the DEM by the Z Conversion Factor. Curvature values are often very small and as such the user may opt to log-transform the output raster (`log`). Transforming the values applies the equation by Shary et al. (2002): 

*Θ*' = sign(*Θ*) ln(1 + 10*n*|*Θ*|) 

where *Θ* is the parameter value and *n* is dependent on the grid cell size. 

For DEMs in projected coordinate systems, the tool uses the 3rd-order bivariate Taylor polynomial method described by Florinsky (2016). Based on a polynomial fit of the elevations within the 5x5 neighbourhood surrounding each cell, this method is considered more robust against outlier elevations (noise) than other methods. For DEMs in geographic coordinate systems (i.e. angular units), the tool uses the 3x3 polynomial fitting method for equal angle grids also described by Florinsky (2016). 

### References

 

Florinsky, I. (2016). Digital terrain analysis in soil science and geology. Academic Press. 

Florinsky, I. V. (2017). An illustrated introduction to general geomorphometry. Progress in Physical Geography, 41(6), 723-752. 

Shary PA (1991) The second derivative topographic method. In: Stepanov IN (ed) The Geometry of the Earth Surface Structures. Pushchino, USSR: Pushchino Research Centre Press, 30–60 (in Russian). 

Shary P. A., Sharaya L. S. and Mitusov A. V. (2002) Fundamental quantitative methods of land surface analysis. Geoderma 107: 1–32. 

### See Also

 

`ring_curvature`, `profile_curvature`, `tangential_curvature`, `plan_curvature`, `mean_curvature`, `gaussian_curvature`, `minimal_curvature`, `maximal_curvature` 

### Python API

```python
def rotor(self, dem: Raster, log_transform: bool = False, z_factor: float = 1.0) -> Raster:
```


---

## Shape Index

**Function name:** `shape_index`


### Description

 

This tool calculates the shape index (Koenderink and van Doorn, 1992) from a digital elevation model (DEM). This variable ranges from -1 to 1, with positive values indicative of convex landforms, negative values corresponding to concave landforms (Florinsky, 2017). Absolute values from 0.5 to 1.0 are associated with elliptic surfaces (hills and closed depressions), while absolute values from 0.0 to 0.5 are typical of hyperbolic surface form (saddles). Shape index is a dimensionless variable and has utility in landform classification applications. 

 

Koenderink and vsn Doorn (1992) make the following observations about the shape index: 
 
-  

Two shapes for which the shape index differs merely by sign represent complementary pairs that will fit together as ‘stamp’ and ‘mould’ when suitably scaled;  
-  

The shape for which the shape index vanishes - and consequently has indeterminate sign - represents the objects which are congruent to their own moulds;  
-  

Convexities and concavities find their places on opposite sides of the shape scale. These basic shapes are separated by those shapes which are neither convex nor concave, that are the saddle-like objects. The transitional shapes that divide the convexities/concavities from the saddle-shapes are the cylindrical ridge and the cylindrical rut.  
 

The user must specify the name of the input DEM (`dem`) and the output raster (`output`). The The Z conversion factor (`zfactor`) is only important when the vertical and horizontal units are not the same in the DEM. When this is the case, the algorithm will multiply each elevation in the DEM by the Z Conversion Factor. 

For DEMs in projected coordinate systems, the tool uses the 3rd-order bivariate Taylor polynomial method described by Florinsky (2016). Based on a polynomial fit of the elevations within the 5x5 neighbourhood surrounding each cell, this method is considered more robust against outlier elevations (noise) than other methods. For DEMs in geographic coordinate systems (i.e. angular units), the tool uses the 3x3 polynomial fitting method for equal angle grids also described by Florinsky (2016). 

### References

 

Florinsky, I. (2016). Digital terrain analysis in soil science and geology. Academic Press. 

Florinsky, I. V. (2017). An illustrated introduction to general geomorphometry. Progress in Physical Geography, 41(6), 723-752. 

Koenderink, J. J., and Van Doorn, A. J. (1992). Surface shape and curvature scales. Image and vision computing, 10(8), 557-564. 

`curvedness`, `minimal_curvature`, `maximal_curvature`, `tangential_curvature`, `profile_curvature`, `mean_curvature`, `gaussian_curvature` 

### Python API

```python
def shape_index(self, dem: Raster, z_factor: float = 1.0) -> Raster:
```


---

## Slope

**Function name:** `slope`


This tool calculates slope gradient (i.e. slope steepness in degrees, radians, or percent) for each grid cell in an input digital elevation model (DEM). The user must specify the name of the input DEM (`dem`) and the  output raster (`output`). The Z conversion factor (`zfactor`) is only important when the vertical and  horizontal units are not the same in the DEM, and the DEM is in a projected coordinate system. When this is the case, the algorithm will multiply each elevation  in the DEM by the Z Conversion Factor to perform the unit conversion.  

For DEMs in projected coordinate systems, the tool uses the 3rd-order bivariate Taylor polynomial method described by Florinsky (2016). Based on a polynomial fit of the elevations within the 5x5 neighbourhood surrounding each cell, this method is considered more robust against outlier elevations (noise) than other methods. For DEMs in geographic coordinate systems (i.e. angular units), the tool uses the 3x3 polynomial fitting method for equal angle grids also described by Florinsky (2016). 

### Reference

 

Florinsky, I. (2016). Digital terrain analysis in soil science and geology. Academic Press. 

Gallant, J. C., and J. P. Wilson, 2000, Primary topographic attributes, in Terrain Analysis: Principles and Applications, edited by J. P. Wilson and J. C. Gallant pp. 51-86, John Wiley, Hoboken, N.J. 

### See Also

 

`aspect`, `plan_curvature`, `profile_curvature` 

### Python API

```python
def slope(self, dem: Raster, units: str = "degrees", z_factor: float = 1.0) -> Raster:
```


---

## Tangential Curvature

**Function name:** `tangential_curvature`


This tool calculates the tangential curvature, which is the curvature of an inclined plan perpendicular to both the direction of flow and the surface (Gallant and Wilson, 2000). Curvature is a second derivative of the topographic surface defined by a digital elevation model (DEM). The user must input a DEM (`dem`). The output reports curvature in degrees multiplied by 100 for easier interpretation, as curvature values are often very small. The Z Conversion Factor (`zfactor`) is only important when the vertical and horizontal units are not the same in the DEM. When this is the case, the algorithm will multiply each elevation in the DEM by the Z Conversion Factor. If the DEM is in the geographic coordinate system (latitude and longitude), with XY units measured in degrees, an appropriate Z Conversion Factor is calculated internally based on site latitude. 

### Reference

 

Gallant, J. C., and J. P. Wilson, 2000, Primary topographic attributes, in Terrain Analysis: Principles and Applications, edited by J. P. Wilson and J. C. Gallant pp. 51-86, John Wiley, Hoboken, N.J. 

`plan_curvature`, `profile_curvature`, `total_curvature`, `slope`, `aspect` 

### Python API

```python
def tangential_curvature(self, dem: Raster, log_transform: bool = False, z_factor: float = 1.0) -> Raster:
```


---

## Total Curvature

**Function name:** `total_curvature`


This tool calculates the total curvature, which measures the curvature of the topographic surface rather than the curvature of a line across the surface in some direction (Gallant and Wilson, 2000). Total curvature can be positive or negative, with zero curvature indicating that the surface is either flat or the convexity in one direction is balanced by the concavity in another direction, as would occur at a saddle point. Curvature is a second derivative of the topographic surface defined by a digital elevation model (DEM). The user must input a DEM (`dem`).The output reports curvature in degrees multiplied by 100 for easier interpretation, as curvature values are often very small. The Z Conversion Factor (`zfactor`) is only important when the vertical and horizontal units are not the same in the DEM. When this is the case, the algorithm will multiply each elevation in the DEM by the Z Conversion Factor. If the DEM is in the geographic coordinate system (latitude and longitude), with XY units measured in degrees, an appropriate Z Conversion Factor is calculated internally based on site latitude. 

### Reference

 

Gallant, J. C., and J. P. Wilson, 2000, Primary topographic attributes, in Terrain Analysis: Principles and Applications, edited by J. P. Wilson and J. C. Gallant pp. 51-86, John Wiley, Hoboken, N.J. 

`plan_curvature`, `profile_curvature`, `tangential_curvature`, `slope`, `aspect` 

### Python API

```python
def total_curvature(self, dem: Raster, log_transform: bool = False, z_factor: float = 1.0) -> Raster:
```


---

## Unsphericity

**Function name:** `unsphericity`


### Description

 

This tool calculates the spatial pattern of unsphericity curvature, which describes the degree to which the shape of the topographic surface is nonspherical at a given point (Shary, 1995), from a digital elevation model (DEM). It is calculated as half the difference between the `maximal_curvature` and the `minimal_curvature`. Unsphericity has values equal to or greater than zero and is measured in units of m-1. Larger values indicate locations that are less spherical in form. 

 

The user must specify the name of the input DEM (`dem`) and the output raster (`output`). The The Z conversion factor (`zfactor`) is only important when the vertical and horizontal units are not the same in the DEM. When this is the case, the algorithm will multiply each elevation in the DEM by the Z Conversion Factor. Curvature values are often very small and as such the user may opt to log-transform the output raster (`log`). Transforming the values applies the equation by Shary et al. (2002): 

*Θ*' = sign(*Θ*) ln(1 + 10*n*|*Θ*|) 

where *Θ* is the parameter value and *n* is dependent on the grid cell size. 

For DEMs in projected coordinate systems, the tool uses the 3rd-order bivariate Taylor polynomial method described by Florinsky (2016). Based on a polynomial fit of the elevations within the 5x5 neighbourhood surrounding each cell, this method is considered more robust against outlier elevations (noise) than other methods. For DEMs in geographic coordinate systems (i.e. angular units), the tool uses the 3x3 polynomial fitting method for equal angle grids also described by Florinsky (2016). 

### References

 

Florinsky, I. (2016). Digital terrain analysis in soil science and geology. Academic Press. 

Florinsky, I. V. (2017). An illustrated introduction to general geomorphometry. Progress in Physical Geography, 41(6), 723-752. 

Shary PA (1995) Land surface in gravity points classification by a complete system of curvatures. Mathematical Geology 27: 373–390. 

Shary P. A., Sharaya L. S. and Mitusov A. V. (2002) Fundamental quantitative methods of land surface analysis. Geoderma 107: 1–32. 

### See Also

 

`minimal_curvature`, `maximal_curvature`, `mean_curvature`, `gaussian_curvature`, `profile_curvature`, `tangential_curvature` 

### Python API

```python
def unsphericity(self, dem: Raster, log_transform: bool = False, z_factor: float = 1.0) -> Raster:
```


---

## Vertical Excess Curvature

**Function name:** `vertical_excess_curvature`


### Description

 

This tool calculates the vertical excess curvature from a digital elevation model (DEM). Vertical excess curvature is the difference of profile (vertical) and minimal curvatures at a location (Shary, 1995). This variable has positive values, zero or greater. Florinsky (2017) states that vertical excess curvature measures the extent to which the bending of a normal section having a common tangent line with a slope line is larger than the minimal bending at a given point of the surface. Vertical excess curvature is measured in units of m-1. 

 

The user must specify the name of the input DEM (`dem`) and the output raster (`output`). The Z conversion factor (`zfactor`) is only important when the vertical and horizontal units are not the same in the DEM. When this is the case, the algorithm will multiply each elevation in the DEM by the Z Conversion Factor. Curvature values are often very small and as such the user may opt to log-transform the output raster (`log`). Transforming the values applies the equation by Shary et al. (2002): 

*Θ*' = sign(*Θ*) ln(1 + 10*n*|*Θ*|) 

where *Θ* is the parameter value and *n* is dependent on the grid cell size. 

For DEMs in projected coordinate systems, the tool uses the 3rd-order bivariate Taylor polynomial method described by Florinsky (2016). Based on a polynomial fit of the elevations within the 5x5 neighbourhood surrounding each cell, this method is considered more robust against outlier elevations (noise) than other methods. For DEMs in geographic coordinate systems (i.e. angular units), the tool uses the 3x3 polynomial fitting method for equal angle grids also described by Florinsky (2016). 

### References

 

Florinsky, I. (2016). Digital terrain analysis in soil science and geology. Academic Press. 

Florinsky, I. V. (2017). An illustrated introduction to general geomorphometry. Progress in Physical Geography, 41(6), 723-752. 

Shary PA (1995) Land surface in gravity points classification by a complete system of curvatures. Mathematical Geology 27: 373–390. 

Shary P. A., Sharaya L. S. and Mitusov A. V. (2002) Fundamental quantitative methods of land surface analysis. Geoderma 107: 1–32. 

### See Also

 

`tangential_curvature`, `profile_curvature`, `minimal_curvature`, `maximal_curvature`, `mean_curvature`, `gaussian_curvature` 

### Python API

```python
def vertical_excess_curvature(self, dem: Raster, log_transform: bool = False, z_factor: float = 1.0) -> Raster:
```
