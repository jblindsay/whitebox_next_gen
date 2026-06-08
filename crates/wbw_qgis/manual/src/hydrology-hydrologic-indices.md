# Hydrologic Indices


---

## Depth To Water

**Function name:** `depth_to_water`


### Description

 

This tool calculates the cartographic depth-to-water (DTW) index described by Murphy et al. (2009). The DTW index has been shown to be related to soil moisture, and is useful for identifying low-lying positions that are likely to experience surface saturated conditions. In this regard, it is similar to each of `wetness_index`, `elevation_above_stream` (HAND), and probability-of-depressions (i.e. `stochastic_depression_analysis`). 

 

The index is the cumulative slope gradient along the least-slope path connecting each grid cell in an input DEM (`dem`) to a surface water cell. Tangent slope (i.e. rise / run) is calculated for each grid cell based on the neighbouring elevation values in the input DEM. The algorithm operates much like a cost-accumulation analysis (`cost_distance`), where the cost of moving through a cell is determined by the cell's tangent slope value and the distance travelled. Therefore, lower DTW values are associated with wetter soils and higher values indicate drier conditions, over longer time periods. Areas of surface water have DTW values of zero. The user must input surface water features, including vector stream lines (`streams`) and/or vector waterbody polygons (`lakes`, i.e. lakes, ponds, wetlands, etc.). At least one of these two optional water feature inputs must be specified. The tool internally rasterizes these vector features, setting the DTW value in the output raster to zero. DTW tends to increase with greater distances from surface water features, and increases more slowly in flatter topography and more rapidly in steeper settings. Murphy et al. (2009) state that DTW is a probablistic model that assumes uniform soil properties, climate, and vegetation. 

Note that DTW values are highly dependent upon the accuracy and extent of the input streams/lakes layer(s). 

### References

 

Murphy, PNC, Gilvie, JO, and Arp, PA (2009) Topographic modelling of soil moisture conditiTons: a comparison and verification of two models. *European Journal of Soil Science*, 60, 94–109, DOI: 10.1111/j.1365-2389.2008.01094.x. 

### See Also

 

`wetness_index`, `elevation_above_stream`, `stochastic_depression_analysis` 

### Python API

```python
def depth_to_water(self, dem: Raster, streams: Optional[Vector] = None, lakes: Optional[Vector] = None) -> Raster:
```


---

## Distance To Outlet

**Function name:** `distance_to_outlet`


### Description

 

This tool calculates the distance of stream grid cells to the channel network outlet cell for each grid cell belonging to a raster stream network. The user must input a raster containing streams data (`streams_raster`), where stream grid cells are denoted by all positive non-zero values, and a D8 flow pointer (i.e. flow direction) raster (`d8_pointer`). The pointer image is used to traverse the stream network and must only be created using the D8 algorithm. Stream cells are designated in the streams image as all grid cells with values greater than zero. Thus, all non-stream or background grid cells are commonly assigned either zeros or NoData values. Background cells will be assigned the NoData value in the output image, unless the `zero_background` parameter is True, in which case non-stream cells will be assigned zero values in the output. 

By default, the pointer raster is assumed to use the clockwise indexing method used by Whitebox. If the pointer file contains ESRI flow direction values instead, the `esri_pointer` parameter must be True. 

### See Also

 

`downslope_distance_to_stream`, `length_of_upstream_channels` 

### Parameters

 

d8_pointer (Raster):     The D8 pointer (flow direction) raster. 

streams_raster (Raster):     The raster object containing the streams data. 

esri_pointer (bool):     Determines whether the d8_pointer raster contains pointer data in the Esri format. Default is False. 

zero_background (bool):     Determines whether the background value in the output raster are assigned zero (True) or NoData values (False). Default is False. 

### Returns

 

Raster: returning value 

### Python API

```python
def distance_to_outlet(self, d8_pointer: Raster, streams_raster: Raster, esri_pointer: bool = False, zero_background: bool = False) -> Raster:
```


---

## Downslope Distance To Stream

**Function name:** `downslope_distance_to_stream`


This tool can be used to calculate the distance from each grid cell in a raster to the nearest stream cell, measured along the downslope flowpath. The user must specify the name of an input digital elevation model (`dem`) and streams raster (`streams`). The DEM must have been pre-processed to remove artifact topographic depressions and flat areas (see `breach_depressions_least_cost`). The streams raster should have been created using one of the DEM-based stream mapping methods, i.e. contributing area thresholding. Stream cells are designated in this raster as all non-zero values. The output of this tool, along with the `elevation_above_stream` tool, can be useful for preliminary flood plain mapping when combined with high-accuracy DEM data. 

By default, this tool calculates flow-path using the D8 flow algorithm. However, the user may specify (`dinf`) that the tool should use the D-infinity algorithm instead. 

### See Also

 

`elevation_above_stream`, `distance_to_outlet` 

### Python API

```python
def downslope_distance_to_stream(self, dem: Raster, streams: Raster, use_dinf: bool = False) -> Raster:
```


---

## Downslope Index

**Function name:** `downslope_index`


This tool can be used to calculate the downslope index described by Hjerdt et al. (2004). The downslope index is a measure of the slope gradient between a grid cell and some downslope location (along the flowpath passing through the upslope grid cell) that represents a specified vertical drop (i.e. a potential head drop). The index has been shown to be useful for hydrological, geomorphological, and biogeochemical applications. 

The user must input a digital elevaton model (DEM) raster. This DEM should be have been pre-processed to remove artifact topographic depressions and flat areas. The user must also specify the head potential drop (d), and the output type. The output type can be either '`tangent`', '`degrees`', '`radians`', or '`distance`'. If '`distance`' is selected as the output type, the output grid actually represents the downslope flowpath length required to drop d meters from each grid cell. Linear interpolation is used when the specified drop value is encountered between two adjacent grid cells along a flowpath traverse. 

Notice that this algorithm is affected by edge contamination. That is, for some grid cells, the edge of the grid will be encountered along a flowpath traverse before the specified vertical drop occurs. In these cases, the value of the downslope index is approximated by replacing d with the actual elevation drop observed along the flowpath. To avoid this problem, the entire watershed containing an area of interest should be contained in the DEM. 

Grid cells containing NoData values in any of the input images are assigned the NoData value in the output raster. The output raster is of the float data type and continuous data scale. 

### Reference

 

Hjerdt, K.N., McDonnell, J.J., Seibert, J. Rodhe, A. (2004) *A new topographic index to quantify downslope controls on local drainage*, **Water Resources Research**, 40, W05602, doi:10.1029/2004WR003130. 

### Python API

```python
def downslope_index(self, dem: Raster, vertical_drop: float, output_type: str = "tangent") -> Raster:
```


---

## Edge Contamination

**Function name:** `edge_contamination`


This tool identifs grid cells in a DEM for which the upslope area extends beyond the raster data extent, so-called 'edge-contamined cells'. If a significant number of edge contaminated cells intersect with your area of interest, it is likely that any estimate of upslope area (i.e. flow accumulation) will be under-estimated.  

The user must specify the  name (`dem`) of the input digital elevation model (DEM) and the  output file (`output`). The DEM must have been hydrologically corrected to remove all spurious depressions and  flat areas. DEM pre-processing is usually achieved using either the `breach_depressions_least_cost` (also `breach_depressions_least_cost`)  or `fill_depressions` tool.  

Additionally, the user must specify the type of flow algorithm used for the analysis (`-flow_type`), which must be  one of 'd8', 'mfd', or 'dinf', based on each of the `D8FlowAccumulation`, `FD8FlowAccumulation`, `DInfFlowAccumulation` methods respectively. 

### See Also

 

`D8FlowAccumulation`, `FD8FlowAccumulation`, `DInfFlowAccumulation` 

### Python API

```python
def edge_contamination(self, dem: Raster, flow_type: str = "mfd", z_factor: float = -1.0) -> Raster:
```


---

## Elev Relative To Watershed Min Max

**Function name:** `elev_relative_to_watershed_min_max`


This tool can be used to express the elevation of a grid cell in a digital elevation model (DEM) as a percentage of the relief between the watershed minimum and maximum values. As such, it provides a basic measure of relative topographic position. The user must input a DEM (`dem`) and watersheds (`watersheds`) raster files. 

### See Also

 

`elev_relative_to_min_max`, `elevation_above_stream`, `ElevAbovePit` 

### Python API

```python
def elev_relative_to_watershed_min_max(self, dem: Raster, watersheds: Raster) -> Raster:
```


---

## Elevation Above Stream

**Function name:** `elevation_above_stream`


This tool can be used to calculate the elevation of each grid cell in a raster above the nearest stream cell, measured along the downslope flowpath. This terrain index, a measure of relative topographic position, is essentially equivalent to the 'height above drainage' (HAND), as described by Renno et al. (2008). The user must specify the name of an input digital elevation model (`dem`) and streams raster (`streams`). The DEM must have been pre-processed to remove artifact topographic depressions and flat areas (see `breach_depressions_least_cost`). The streams raster should have been created using one of the DEM-based stream mapping methods, i.e. contributing area thresholding. Stream cells are designated in this raster as all non-zero values. The output of this tool, along with the `downslope_distance_to_stream` tool, can be useful for preliminary flood plain mapping when combined with high-accuracy DEM data. 

The difference between `elevation_above_stream` and `elevation_above_stream_euclidean` is that the former calculates distances along drainage flow-paths while the latter calculates straight-line distances to streams channels. 

### Reference

 

Renno, C. D., Nobre, A. D., Cuartas, L. A., Soares, J. V., Hodnett, M. G., Tomasella, J., & Waterloo, M. J. (2008). HAND, a new terrain descriptor using SRTM-DEM: Mapping terra-firme rainforest environments in Amazonia. Remote Sensing of Environment, 112(9), 3469-3481. 

### See Also

 

`elevation_above_stream_euclidean`, `downslope_distance_to_stream`, `ElevAbovePit`, `breach_depressions_least_cost` 

### Python API

```python
def elevation_above_stream(self, dem: Raster, streams: Raster) -> Raster:
```


---

## Elevation Above Stream Euclidean

**Function name:** `elevation_above_stream_euclidean`


This tool can be used to calculate the elevation of each grid cell in a raster above the nearest stream cell, measured along the straight-line distance. This terrain index, a measure of relative topographic position, is related to the 'height above drainage' (HAND), as described by Renno et al. (2008). HAND is generally estimated with distances measured along drainage flow-paths, which can be calculated using the `elevation_above_stream` tool. The user must specify the name of an input digital elevation model (`dem`) and streams raster (`streams`). Stream cells are designated in this raster as all non-zero values. The output of this tool, along with the `downslope_distance_to_stream` tool, can be useful for preliminary flood plain mapping when combined with high-accuracy DEM data. 

The difference between `elevation_above_stream` and `elevation_above_stream_euclidean` is that the former calculates distances along drainage flow-paths while the latter calculates straight-line distances to streams channels. 

### Reference

 

Renno, C. D., Nobre, A. D., Cuartas, L. A., Soares, J. V., Hodnett, M. G., Tomasella, J., & Waterloo, M. J. (2008). HAND, a new terrain descriptor using SRTM-DEM: Mapping terra-firme rainforest environments in Amazonia. Remote Sensing of Environment, 112(9), 3469-3481. 

### See Also

 

`elevation_above_stream`, `downslope_distance_to_stream`, `ElevAbovePit` 

### Python API

```python
def elevation_above_stream_euclidean(self, dem: Raster, streams: Raster) -> Raster:
```


---

## Find Noflow Cells

**Function name:** `find_noflow_cells`


This tool can be used to find cells with undefined flow, i.e. no valid flow direction, based on the D8 flow direction algorithm (`d8_pointer`). These cells are therefore either at the bottom of a topographic depression or in the interior of a flat area. In a digital elevation model (DEM) that has been pre-processed to remove all depressions and flat areas (`breach_depressions_least_cost`), this condition will only occur along the edges of the grid, otherwise no-flow grid cells can be situation in the interior. The user must specify the name (`dem`) of the DEM. 

### See Also

 

`d8_pointer`, `breach_depressions_least_cost` 

### Python API

```python
def find_noflow_cells(self, dem: Raster) -> Raster:
```


---

## Find Parallel Flow

**Function name:** `find_parallel_flow`


This tool can be used to find cells in a stream network grid that possess parallel flow directions based on an input D8 flow-pointer grid (`d8_pointer`). Because streams rarely flow in parallel for significant distances, these areas are likely errors resulting from the biased assignment of flow direction based on the D8 method. 

### See Also

 

`d8_pointer` 

### Python API

```python
def find_parallel_flow(self, d8_pntr: Raster, streams: Raster) -> Raster:
```


---

## Hydrologic Connectivity

**Function name:** `hydrologic_connectivity`


**Theory** 

This tool calculates two indices related to hydrologic connectivity within catchments, the *downslope unsaturated length* (DUL) and the *upslope disconnected saturated area* (UDSA). Both of these hydrologic indices are based on the topographic wetness index (`wetness_index`), which measures the propensity for a site to be saturated to the surface, and therefore, to contribute to surface runoff. The `wetness index` (WI) is commonly used in hydrologic modelling, and famously in the TOPMODEL, to simulate variable source area (VSA) dynamics within catchments. The VSA is a dynamic region of surface-saturated soils within catchments that contributes fast overland flow to downslope streams during periods of precipitation. As a catchment's soil saturation deficit decreases ('wetting up'), areas with increasingly lower WI values become saturated to the surface. That is, areas of high WI are the first to become saturated and as the moisture deficit decreases, lower WI-valued cells become saturated, increasing the spatial extent of the source area. As a catchment dries out, the opposite effect occurs. The distribution of WI can therefore be used to map the spatial dyanamics of the VSA. However, the assumption in the TOPMODEL is that any rainfall over surface saturated areas will contribute to fast overland flow pathways and to stream discharge within the time step. 

This method therefore implicitly assumes that all surface saturated grid cells are connected by continuously saturated areas along the downslope flow path connecting the cells to the stream. By comparison, Lane et al. (2004) proposed a modified WI, known as the network index (NI), which allowed for the modelling of disconnected, non-contributing saturated areas. The NI is essentially the downslope minimum WI. Grid cells for which WI > NI are likely to be disconnected during certain conditions from downslope streams, while similarly WI-valued cells are contributing. During these periods, any surface runoff from these cells is likely to contribute to downslope re-infilitration rather than directly to stream discharge via overland flow. This has implications for the timing and quality of stream discharge. 

The DUL and UDSA indices extend the notion of the NI by mapping areas within catchments that are likely, at least during certain periods, to be sites of disconnected, non-contributing saturated areas and sites of re-infiltation respectively. These combined indices allow hydrologists to study the hydrologic connectivity and disconnectivity among areas within catchments.  

The DUL (see image below) is defined for a grid cell as **the number of downslope cells with a WI value lower than the current cell**. Areas with non-zero DUL are likely to become fully saturated, and to contribute to overland flow, before they are directly connected to downslope areas and can contribute to stream flow. Under the appropriate catchment saturation deficit conditions, these are sites of disconnected, non-contributing saturated areas. When non-zero DUL cells are initially saturated, their precipitation excess will contribute to downslope re-infiltation, lessening the catchment's overall saturation deficit, rather than contributing to stormflow.  

  

The UDSA (see image below) is defined for a grid cell as **the number of upslope cells with a WI value higher than the current cell**. Areas with non-zero UDSA are likely to have saturation deficits that are at least partly satisfied by local re-infiltation of overland flow from upslope areas. These non-zero UDSA cells are key sites causing the hydrologic disconnectivity of the catchment during certain conditions.  

 

In the original Lane et al. (2004) NI paper, the authors state that the calculation of the index requires a unique, single downslope flow path for each grid cell. Therefore, the authors used the D8 single-direction flow algorithm to calculate NI. While the D8 method works well to model flow in convergent and channelized areas, it is generally recognized as a poor method for estimating WI on hillslopes, where divergent, non-chanellized flow dominates. Furthermore, the use of the D8 algorithm implied that the only way that WI can decrease downslope is for slope gradient to decrease, since specific contributing area only increases downslope with the D8 method. However, theoretically, WI may also decrease downslope due to flow dispersion, which allows for the upslope area (a surrogate for discharge) to be spread over a larger downslope dispersal area. The original NI formulation could not account for this effect. 

Thus, in the implementation of the `hydrologic_connectivity` tool, WI is first calculated using the multiple flow-direction (MFD) algorithm described by Quinn et al. (1995), which is commonly used to estimate WI. While this implies that there are a multitude of potential flow pathways connecting each grid cell to a downstream location, in reality, if the flow path that follows the path of maximum WI issuing from a cell experiences a reduction in WI (to the point where it becomes less than the issuing cell's WI), then we can safely assume that re-infiltration occurs and the issuing cell is at times disconnected from downslope sites. Thus, after WI has been estimated using the `quinn_flow_accumulation` algorithm, flow directions, which are used to calculate upslope and downslope flow paths for calculating the two indices, are approximated by identifying the downslope neighbour of highest WI value for each grid cell. 

**Operation** 

The user must specify the name of the input digital elevation model (DEM; `dem`), and the output DUL and UDSA rasters (`output1` and `output2`). The DEM must have been hydrologically corrected to remove all spurious depressions and flat areas. DEM pre-processing is usually achived using either the `breach_depressions_least_cost` (also `breach_depressions_least_cost`) or `fill_depressions` tool. The remaining two parameters are associated with the calculation of the Quinn et al. (1995) flow accumulation (`quinn_flow_accumulation`), used to estimate WI. A value must be specified for the exponent parameter (`exponent`), a number that controls the degree of dispersion in the flow-accumulation grid. A lower value yields greater apparent flow dispersion across divergent hillslopes. The exponent value (*h*) should probably be less than 10.0 and values between 1 and 2 are most common. The following equations are used to calculate the portion flow (*Fi*) given to each neighbour, *i*:  

*Fi* = *Li*(tan&beta;)*p* / &Sigma;*i*=1*n*[*Li*(tan&beta;)*p*] 

*p* = (*A* / *threshold* + 1)*h*  

Where *Li* is the contour length, and is 0.5&times;grid size for cardinal directions and 0.354&times;grid size for diagonal directions, *n* = 8, and represents each of the eight neighbouring grid cells, and, *A* is the flow accumultation value assigned to the current grid cell, that is being apportioned downslope. The non-dispersive, channel initiation *threshold* (`threshold`) is a flow-accumulation value (measured in upslope grid cells, which is directly proportional to area) above which flow dispersion is no longer permited. Grid cells with flow-accumulation values above this threshold will have their flow routed in a manner that is similar to the D8 single-flow-direction algorithm, directing all flow towards the steepest downslope neighbour. This is usually done under the assumption that flow dispersion, whilst appropriate on hillslope areas, is not realistic once flow becomes channelized. Importantly, the `threshold` parameter sets the spatial extent of the stream network, with lower values resulting in more extensive networks. 

### References

 

Beven K.J., Kirkby M.J., 1979. A physically-based, variable contributing area model of basin hydrology. *Hydrological Sciences Bulletin* 24: 43–69. 

Lane, S.N., Brookes, C.J., Kirkby, M.J. and Holden, J., 2004. A network‐index‐based version of TOPMODEL for use with high‐resolution digital topographic data. *Hydrological processes*, 18(1), pp.191-201. 

Quinn, P. F., K. J. Beven, Lamb, R. 1995. The in (a/tanβ) index: How to calculate it and how to use it within the topmodel framework. *Hydrological processes* 9(2): 161-182. 

### See Also

 

`wetness_index`, `quinn_flow_accumulation` 

### Python API

```python
def  hydrologic_connectivity(self, dem: Raster, exponent: float = 1.1, convergence_threshold: float = 0.0, z_factor: float = 1.0 ) -> Tuple[Raster, Raster]:
```


---

## Relative Stream Power Index

**Function name:** `relative_stream_power_index`


This tool can be used to calculate the relative stream power (*RSP*) index. This index is directly related to the stream power if the assumption can be made that discharge is directly proportional to upslope contributing area (*As*; `sca`). The index is calculated as:  

*RSP* = *As**p* &times; tan(&beta;)  

where *As* is the specific catchment area (i.e. the upslope contributing area per unit contour length) estimated using one of the available flow accumulation algorithms; &beta; is the local slope gradient in degrees (`slope`); and, *p* (`exponent`) is a user-defined exponent term that controls the location-specific relation between contributing area and discharge. Notice that *As* must not be log-transformed prior to being used; *As* is commonly log-transformed to enhance visualization of the data. The slope raster can be created from the base digital elevation model (DEM) using the `slope` tool. The input images must have the same grid dimensions. 

### Reference

 

Moore, I. D., Grayson, R. B., and Ladson, A. R. (1991). Digital terrain modelling: a review of hydrological, geomorphological, and biological applications. *Hydrological processes*, 5(1), 3-30. 

### See Also

 

`sediment_transport_index`, `slope`, `D8FlowAccumulation` `DInfFlowAccumulation`, `FD8FlowAccumulation` 

### Python API

```python
def relative_stream_power_index(self, specific_catchment_area: Raster, slope: Raster, exponent: float = 1.0) -> Raster:
```


---

## Sediment Transport Index

**Function name:** `sediment_transport_index`


This tool calculates the sediment transport index, or sometimes, length-slope (*LS*) factor, based on input specific contributing area (*As*, i.e. the upslope contributing area per unit contour length; `sca`) and slope gradient (&beta;, measured in degrees; `slope`) rasters. Moore et al. (1991) state that the physical potential for sheet and rill erosion in upland catchments can be evaluated by the product *R K LS*, a component of the Universal Soil Loss Equation (USLE), where *R* is a rainfall and runoff erosivity factor, *K* is a soil erodibility factor, and *LS* is the length-slope factor that accounts for the effects of topography on erosion. To predict erosion at a point in the landscape the LS factor can be written as:  

*LS* = (*n* + 1)(*As* / 22.13)*n*(sin(&beta;) / 0.0896)*m*  

where *n* = 0.4 (`sca_exponent`) and *m* = 1.3 (`slope_exponent`) in its original formulation. 

This index is derived from unit stream-power theory and is sometimes used in place of the length-slope factor in the revised universal soil loss equation (RUSLE) for slope lengths less than 100 m and slope less than 14 degrees. Like many hydrological land-surface parameters `sediment_transport_index` assumes that contributing area is directly related to discharge. Notice that *As* must not be log-transformed prior to being used; *As* is commonly log-transformed to enhance visualization of the data. Also, *As* can be derived using any of the available flow accumulation tools, alghough better results usually result from application of multiple-flow direction algorithms such as `DInfFlowAccumulation` and `FD8FlowAccumulation`. The slope raster can be created from the base digital elevation model (DEM) using the `slope` tool. The input images must have the same grid dimensions. 

### Reference

 

Moore, I. D., Grayson, R. B., and Ladson, A. R. (1991). Digital terrain modelling: a review of hydrological, geomorphological, and biological applications. *Hydrological processes*, 5(1), 3-30. 

### See Also

 

`StreamPowerIndex`, `DInfFlowAccumulation`, `FD8FlowAccumulation` 

### Python API

```python
def sediment_transport_index(self, specific_catchment_area: Raster, slope: Raster, sca_exponent: float = 0.4, slope_exponent: float = 1.3) -> Raster:
```


---

## Wetness Index

**Function name:** `wetness_index`


This tool can be used to calculate the topographic wetness index, commonly used in the TOPMODEL rainfall-runoff framework. The index describes the propensity for a site to be saturated to the surface given its contributing area and local slope characteristics. It is calculated as:  

WI = Ln(As / tan(Slope))  

Where `As` is the specific catchment area (i.e. the upslope contributing area per unit contour length) estimated using one of the available flow accumulation algorithms in the Hydrological Analysis toolbox. Notice that `As` must not be log-transformed prior to being used; log-transformation of `As` is a common practice when visualizing the data. The slope image should be measured in degrees and can be created from the base digital elevation model (DEM) using the `slope` tool. Grid cells with a slope of zero will be assigned **NoData** in the output image to compensate for the fact that division by zero is infinity. These very flat sites likely coincide with the wettest parts of the landscape. The input images must have the same grid dimensions. 

Grid cells possessing the NoData value in either of the input images are assigned NoData value in the output image. The output raster is of the float data type and continuous data scale. 

See Also `slope`, `D8FlowAccumulation`, `DInfFlowAccumulation`, `FD8FlowAccumulation`, `breach_depressions_least_cost` 

### Python API

```python
def wetness_index(self, specific_catchment_area: Raster, slope: Raster) -> Raster:
```
