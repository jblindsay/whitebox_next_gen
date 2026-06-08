# Flow Routing


---

## Average Flowpath Slope

**Function name:** `average_flowpath_slope`


This tool calculates the average slope gradient (i.e. slope steepness in degrees) of the flowpaths that pass through each grid cell in an input digital elevation model (DEM). The user must specify the name of a DEM raster (`dem`). It is important that this DEM is pre-processed to remove all topographic depressions and flat areas using a tool such as `breach_depressions_least_cost`. Several intermediate rasters are created and stored in memory during the operation of this tool, which may limit the size of DEM that can be processed, depending on available system resources. 

### See Also

 

`average_upslope_flowpath_length`, `breach_depressions_least_cost` 

### Python API

```python
def average_flowpath_slope(self, dem: Raster) -> Raster:
```


---

## Average Upslope Flowpath Length

**Function name:** `average_upslope_flowpath_length`


This tool calculates the average slope gradient (i.e. slope steepness in degrees) of the flowpaths that pass through each grid cell in an input digital elevation model (DEM). The user must specify the name of a DEM raster (`dem`). It is important that this DEM is pre-processed to remove all topographic depressions and flat areas using a tool such as `breach_depressions_least_cost`. Several intermediate rasters are created and stored in memory during the operation of this tool, which may limit the size of DEM that can be processed, depending on available system resources. 

### See Also

 

`average_upslope_flowpath_length`, `breach_depressions_least_cost` 

### Python API

```python
def average_upslope_flowpath_length(self, dem: Raster) -> Raster:
```


---

## D8 Flow Accum

**Function name:** `d8_flow_accum`


This tool is used to generate a flow accumulation grid (i.e. catchment area) using the D8 (O'Callaghan and Mark, 1984) algorithm. This algorithm is an example of single-flow-direction (SFD) method because the flow entering each grid cell is routed to only one downslope neighbour, i.e. flow divergence is not permitted. The user must specify the name of the input digital elevation model (DEM) or flow pointer raster (`input`) derived using the D8 or Rho8 method (`d8_pointer`, `rho8_pointer`). If an input DEM is used, it must have been hydrologically corrected to remove all spurious depressions and flat areas. DEM pre-processing is usually achieved using the `breach_depressions_least_cost` or `fill_depressions` tools. If a D8 pointer raster is input, the user must also specify the optional `pntr` flag. If the D8 pointer follows the Esri pointer scheme, rather than the default WhiteboxTools scheme, the user must also specify the optional `esri_pntr` flag. 

In addition to the input DEM/pointer, the user must specify the output type. The output flow-accumulation can be 1) `cells` (i.e. the number of inflowing grid cells), `catchment area` (i.e. the upslope area), or `specific contributing area` (i.e. the catchment area divided by the flow width. The default value is `cells`. The user must also specify whether the output flow-accumulation grid should be log-tranformed (`log`), i.e. the output, if this option is selected, will be the natural-logarithm of the accumulated flow value. This is a transformation that is often performed to better visualize the contributing area distribution. Because contributing areas tend to be very high along valley bottoms and relatively low on hillslopes, when a flow-accumulation image is displayed, the distribution of values on hillslopes tends to be 'washed out' because the palette is stretched out to represent the highest values. Log-transformation provides a means of compensating for this phenomenon. Importantly, however, log-transformed flow-accumulation grids must not be used to estimate other secondary terrain indices, such as the wetness index, or relative stream power index. 

Grid cells possessing the **NoData** value in the input DEM/pointer raster are assigned the **NoData** value in the output flow-accumulation image. 

### Reference

 

O'Callaghan, J. F., & Mark, D. M. 1984. The extraction of drainage networks from digital elevation data. *Computer Vision, Graphics, and Image Processing*, 28(3), 323-344. 

### See Also:

 

`FD8FlowAccumulation`, `quinn_flow_accumulation`, `qin_flow_accumulation`, `DInfFlowAccumulation`, `MDInfFlowAccumulation`, `rho8_pointer`, `d8_pointer`, `breach_depressions_least_cost`, `fill_depressions` 

### Python API

```python
def d8_flow_accum(self, raster: Raster, out_type: str = "sca", log_transform: bool = False, clip: bool = False, input_is_pointer: bool = False, esri_pntr: bool = False) -> Raster:
```


---

## D8 Mass Flux

**Function name:** `d8_mass_flux`


This tool can be used to perform a mass flux calculation using DEM-based surface flow-routing techniques. For example, it could be used to model the distribution of sediment or phosphorous within a catchment. Flow-routing is based on a D8 flow pointer (i.e. flow direction) derived from an input depresionless DEM (`dem`). The user must also specify the names of loading (`loading`), efficiency (`efficiency`), and absorption (`absorption`) rasters, as well as the output raster. Mass Flux operates very much like a flow-accumulation operation except that rather than accumulating catchment areas the algorithm routes a quantity of mass, the spatial distribution of which is specified within the loading image. The efficiency and absorption rasters represent spatial distributions of losses to the accumulation process, the difference being that the efficiency raster is a proportional loss (e.g. only 50% of material within a particular grid cell will be directed downslope) and the absorption raster is an loss specified as a quantity in the same units as the loading image. The efficiency image can range from 0 to 1, or alternatively, can be expressed as a percentage. The equation for determining the mass sent from one grid cell to a neighbouring grid cell is:  

*Outflowing Mass* = (*Loading* - *Absorption* + *Inflowing Mass*) &times; *Efficiency*  

This tool assumes that each of the three input rasters have the same number of rows and columns and that any **NoData** cells present are the same among each of the inputs. 

### See Also

 

`DInfMassFlux` 

### Python API

```python
def d8_mass_flux(self, dem: Raster, loading: Raster, efficiency: Raster, absorption: Raster) -> Raster:
```


---

## D8 Pointer

**Function name:** `d8_pointer`


This tool is used to generate a flow pointer grid using the simple D8 (O'Callaghan and Mark, 1984) algorithm. The user must specify the name (`dem`) of a digital elevation model (DEM) that has been hydrologically corrected to remove all spurious depressions and flat areas. DEM pre-processing is usually achieved using either the `breach_depressions_least_cost` or `fill_depressions` tool. The local drainage direction raster output (`output`) by this tool serves as a necessary input for several other spatial hydrology and stream network analysis tools in the toolset. Some tools will calculate this flow pointer raster directly from the input DEM. 

By default, D8 flow pointers use the following clockwise, base-2 numeric index convention:  ... 641281 3202 1684   

Notice that grid cells that have no lower neighbours are assigned a flow direction of zero. In a DEM that has been pre-processed to remove all depressions and flat areas, this condition will only occur along the edges of the grid. If the pointer file contains ESRI flow direction values instead, the `esri_pntr` parameter must be specified. 

Grid cells possessing the NoData value in the input DEM are assigned the NoData value in the output image. 

### Memory Usage

 

The peak memory usage of this tool is approximately 10 bytes per grid cell. 

### Reference

 

O'Callaghan, J. F., & Mark, D. M. (1984). The extraction of drainage networks from digital elevation data. Computer vision, graphics, and image processing, 28(3), 323-344. 

### See Also

 

`DInfPointer`, `fd8_pointer`, `breach_depressions_least_cost`, `fill_depressions` 

### Python API

```python
def d8_pointer(self, dem: Raster, esri_pointer: bool = False) -> Raster:
```


---

## D-Infinity Flow Accum

**Function name:** `dinf_flow_accum`


This tool is used to generate a flow accumulation grid (i.e. contributing area) using the D-infinity algorithm (Tarboton, 1997). This algorithm is an examples of a multiple-flow-direction (MFD) method because the flow entering each grid cell is routed to one or two downslope neighbour, i.e. flow divergence is permitted. The user must specify the name of the input digital elevation model or D-infinity pointer raster (`input`). If an input DEM is specified, the DEM should have been hydrologically corrected to remove all spurious depressions and flat areas. DEM pre-processing is usually achieved using the `breach_depressions_least_cost` or `fill_depressions` tool. 

In addition to the input DEM/pointer raster name, the user must specify the output type (`out_type`). The output flow-accumulation can be 1) specific catchment area (SCA), which is the upslope contributing area divided by the contour length (taken as the grid resolution), 2) total catchment area in square-metres, or 3) the number of upslope grid cells. The user must also specify whether the output flow-accumulation grid should be log-tranformed, i.e. the output, if this option is selected, will be the natural-logarithm of the accumulated area. This is a transformation that is often performed to better visualize the contributing area distribution. Because contributing areas tend to be very high along valley bottoms and relatively low on hillslopes, when a flow-accumulation image is displayed, the distribution of values on hillslopes tends to be 'washed out' because the palette is stretched out to represent the highest values. Log-transformation (`log`) provides a means of compensating for this phenomenon. Importantly, however, log-transformed flow-accumulation grids must not be used to estimate other secondary terrain indices, such as the wetness index, or relative stream power index. 

Grid cells possessing the NoData value in the input DEM/pointer raster are assigned the NoData value in the output flow-accumulation image. The output raster is of the float data type and continuous data scale. 

### Reference

 

Tarboton, D. G. (1997). A new method for the determination of flow directions and upslope areas in grid digital elevation models. Water resources research, 33(2), 309-319. 

### See Also

 

`DInfPointer`, D8FlowAccumulation`, <a href="https://www.whiteboxgeo.com/manual/wbw-user-manual/book/tool_help.html#quinn_flow_accumulation">quinn_flow_accumulation</a>, <a href="https://www.whiteboxgeo.com/manual/wbw-user-manual/book/tool_help.html#qin_flow_accumulation">qin_flow_accumulation</a>,`FD8FlowAccumulation`,`MDInfFlowAccumulation`, `rho8_pointer`, `breach_depressions_least_cost`, `fill_depressions` 

### Python API

```python
def dinf_flow_accum(self, dem: Raster, out_type: str = "sca", convergence_threshold: float = float('inf'), log_transform: bool = False, clip: bool = False, input_is_pointer: bool = False) -> Raster:
```


---

## D-Infinity Mass Flux

**Function name:** `dinf_mass_flux`


This tool can be used to perform a mass flux calculation using DEM-based surface flow-routing techniques. For example, it could be used to model the distribution of sediment or phosphorous within a catchment. Flow-routing is based on a D-Infinity flow pointer derived from an input DEM (`dem`). The user must also specify the names of loading (`loading`), efficiency (`efficiency`), and absorption (`absorption`) rasters, as well as the output raster. Mass Flux operates very much like a flow-accumulation operation except that rather than accumulating catchment areas the algorithm routes a quantity of mass, the spatial distribution of which is specified within the loading image. The efficiency and absorption rasters represent spatial distributions of losses to the accumulation process, the difference being that the efficiency raster is a proportional loss (e.g. only 50% of material within a particular grid cell will be directed downslope) and the absorption raster is an loss specified as a quantity in the same units as the loading image. The efficiency image can range from 0 to 1, or alternatively, can be expressed as a percentage. The equation for determining the mass sent from one grid cell to a neighbouring grid cell is:  

*Outflowing Mass* = (*Loading* - *Absorption* + *Inflowing Mass*) &times; *Efficiency*  

This tool assumes that each of the three input rasters have the same number of rows and columns and that any **NoData** cells present are the same among each of the inputs. 

### See Also

 

`d8_mass_flux` 

### Python API

```python
def dinf_mass_flux(self, dem: Raster, loading: Raster, efficiency: Raster, absorption: Raster) -> Raster:
```


---

## D-Infinity Pointer

**Function name:** `dinf_pointer`


This tool is used to generate a flow pointer grid (i.e. flow direction) using the D-infinity (Tarboton, 1997) algorithm. Dinf is a multiple-flow-direction (MFD) method because the flow entering each grid cell is routed one or two downslope neighbours, i.e. flow divergence is permitted. The user must specify the name of a digital elevation model (DEM; `dem`) that has been hydrologically corrected to remove all spurious depressions and flat areas (`breach_depressions_least_cost`, `fill_depressions`). DEM pre-processing is usually achieved using the `breach_depressions_least_cost` or `fill_depressions` tool1. Flow directions are specified in the output flow-pointer grid (`output`) as azimuth degrees measured from north, i.e. any value between 0 and 360 degrees is possible. A pointer value of -1 is used to designate a grid cell with no flow-pointer. This occurs when a grid cell has no downslope neighbour, i.e. a pit cell or topographic depression. Like aspect grids, Dinf flow-pointer grids are best visualized using a circular greyscale palette. 

Grid cells possessing the NoData value in the input DEM are assigned the NoData value in the output image. The output raster is of the float data type and continuous data scale. 

### Reference

 

Tarboton, D. G. (1997). A new method for the determination of flow directions and upslope areas in grid digital elevation models. Water resources research, 33(2), 309-319. 

### See Also

 

`DInfFlowAccumulation`, `breach_depressions_least_cost`, `fill_depressions` 

### Python API

```python
def dinf_pointer(self, dem: Raster) -> Raster:
```


---

## Downslope Flowpath Length

**Function name:** `downslope_flowpath_length`


This tool can be used to calculate the downslope flowpath length from each grid cell in a raster to an outlet cell either at the edge of the grid or at the outlet point of a watershed. The user must specify the name of a flow pointer grid (`d8_pntr`) derived using the D8 flow algorithm (`d8_pointer`). This grid should be derived from a digital elevation model (DEM) that has been pre-processed to remove artifact topographic depressions and flat areas (`breach_depressions_least_cost`, `fill_depressions`). The user may also optionally provide watershed (`watersheds`) and weights (`weights`) images. The optional watershed image can be used to define one or more irregular-shaped watershed boundaries. Flowpath lengths are measured within each watershed in the watershed image (each defined by a unique identifying number) as the flowpath length to the watershed's outlet cell. 

The optional weight image is multiplied by the flow-length through each grid cell. This can be useful when there is a need to convert the units of the output image. For example, the default unit of flowpath lengths is the same as the input image(s). Thus, if the input image has X-Y coordinates measured in metres, the output image will likely contain very large values. A weight image containing a value of 0.001 for each grid cell will effectively convert the output flowpath lengths into kilometres. The weight image can also be used to convert the flowpath distances into travel times by multiplying the flow distance through a grid cell by the average velocity. 

NoData valued grid cells in any of the input images will be assigned NoData values in the output image. The output raster is of the float data type and continuous data scale. 

### See Also

 

`d8_pointer`, `elevation_above_stream`, `breach_depressions_least_cost`, `fill_depressions`, `watershed` 

### Python API

```python
def downslope_flowpath_length(self, d8_pointer: Raster, watersheds: Raster, weights: Raster, esri_pntr: bool = False) -> Raster:
```


---

## FD8 Flow Accum

**Function name:** `fd8_flow_accum`


This tool is used to generate a flow accumulation grid (i.e. contributing area) using the FD8 algorithm (Freeman, 1991), sometimes referred to as FMFD. This algorithm is an examples of a multiple-flow-direction (MFD) method because the flow entering each grid cell is routed to each downslope neighbour, i.e. flow divergence is permitted. The user must specify the name (`dem`) of the input digital elevation model (DEM). The DEM must have been hydrologically corrected to remove all spurious depressions and flat areas. DEM pre-processing is usually achieved using either the `breach_depressions_least_cost` (also `breach_depressions_least_cost`) or `fill_depressions` tool. A value must also be specified for the exponent parameter (`exponent`), a number that controls the degree of dispersion in the resulting flow-accumulation grid. A lower value yields greater apparent flow dispersion across divergent hillslopes. Some experimentation suggests that a value of 1.1 is appropriate (Freeman, 1991), although this is almost certainly landscape-dependent. 

In addition to the input DEM, the user must specify the output type (`out_type`). The output flow-accumulation can be 1) `cells` (i.e. the number of inflowing grid cells), `catchment area` (i.e. the upslope area), or `specific contributing area` (i.e. the catchment area divided by the flow width. The default value is `cells`. The user must also specify whether the output flow-accumulation grid should be log-tranformed (`log`), i.e. the output, if this option is selected, will be the natural-logarithm of the accumulated flow value. This is a transformation that is often performed to better visualize the contributing area distribution. Because contributing areas tend to be very high along valley bottoms and relatively low on hillslopes, when a flow-accumulation image is displayed, the distribution of values on hillslopes tends to be 'washed out' because the palette is stretched out to represent the highest values. Log-transformation provides a means of compensating for this phenomenon. Importantly, however, log-transformed flow-accumulation grids must not be used to estimate other secondary terrain indices, such as the wetness index, or relative stream power index. 

The non-dispersive threshold (`threshold`) is a flow-accumulation value (measured in upslope grid cells, which is directly proportional to area) above which flow dispersion is no longer permitted. Grid cells with flow-accumulation values above this threshold will have their flow routed in a manner that is similar to the D8 single-flow-direction algorithm, directing all flow towards the steepest downslope neighbour. This is usually done under the assumption that flow dispersion, whilst appropriate on hillslope areas, is not realistic once flow becomes channelized. 

### Reference

 

Freeman, T. G. (1991). Calculating catchment area with divergent flow based on a regular grid. Computers and Geosciences, 17(3), 413-422. 

### See Also

 

`D8FlowAccumulation`, `quinn_flow_accumulation`, `qin_flow_accumulation`, `DInfFlowAccumulation`, `MDInfFlowAccumulation`, `rho8_pointer` 

### Python API

```python
def fd8_flow_accum(self, dem: Raster, out_type: str = "sca", exponent: float = 1.1, convergence_threshold: float = float('inf'), log_transform: bool = False, clip: bool = False) -> Raster:
```


---

## FD8 Pointer

**Function name:** `fd8_pointer`


This tool is used to generate a flow pointer grid (i.e. flow direction) using the FD8 (Freeman, 1991) algorithm. FD8 is a multiple-flow-direction (MFD) method because the flow entering each grid cell is routed one or more downslope neighbours, i.e. flow divergence is permitted. The user must specify the name of a digital elevation model (DEM; `dem`) that has been hydrologically corrected to remove all spurious depressions and flat areas. DEM pre-processing is usually achieved using the `breach_depressions_least_cost` or `fill_depressions` tools. 

By default, D8 flow pointers use the following clockwise, base-2 numeric index convention:  ... 641281 3202 1684   

In the case of the FD8 algorithm, some portion of the flow entering a grid cell will be sent to each downslope neighbour. Thus, the FD8 flow-pointer value is the sum of each of the individual pointers for all downslope neighbours. For example, if a grid cell has downslope neighbours to the northeast, east, and south the corresponding FD8 flow-pointer value will be 1 + 2 + 8 = 11. Using the naming convention above, this is the only combination of flow-pointers that will result in the combined value of 11. Using the base-2 naming convention allows for the storage of complex combinations of flow-points using a single numeric value, which is the reason for using this somewhat odd convention. 

### Reference

 

Freeman, T. G. (1991). Calculating catchment area with divergent flow based on a regular grid. Computers and Geosciences, 17(3), 413-422. 

### See Also

 

`FD8FlowAccumulation`, `d8_pointer`, `DInfPointer`, `breach_depressions_least_cost`, `fill_depressions` 

### Python API

```python
def fd8_pointer(self, dem: Raster) -> Raster:
```


---

## Flow Accum Full Workflow

**Function name:** `flow_accum_full_workflow`


Resolves all of the depressions in a DEM, outputting a breached DEM, an aspect-aligned non-divergent flow pointer, and a flow accumulation raster. 

### Python API

```python
def flow_accum_full_workflow(self, dem: Raster, out_type: str = "sca", log_transform: bool = False, clip: bool = False, esri_pntr: bool = False) -> Tuple[Raster, Raster, Raster]:
```


---

## Flow Length Diff

**Function name:** `flow_length_diff`


FlowLengthDiff calculates the local maximum absolute difference in downslope flowpath length, which is useful in mapping drainage divides and ridges. 

### See Also

 

`max_branch_length` 

### Python API

```python
def flow_length_diff(self, d8_pointer: Raster, esri_pointer: bool = False, log_transform: bool = False) -> Raster:
```


---

## Max Upslope Flowpath Length

**Function name:** `max_upslope_flowpath_length`


This tool calculates the maximum length of the flowpaths that run through each grid cell (in map horizontal units) in an input digital elevation model (`dem`). The tool works by first calculating the D8 flow pointer (`d8_pointer`) from the input DEM. The DEM must be depressionless and should have been pre-processed using the `breach_depressions_least_cost` or `fill_depressions` tool. The user must also specify the name of output raster (`output`). 

### See Also

 

`d8_pointer`, `breach_depressions_least_cost`, `fill_depressions`, `average_upslope_flowpath_length`, `downslope_flowpath_length`, `downslope_distance_to_stream` 

### Python API

```python
def max_upslope_flowpath_length(self, dem: Raster) -> Raster:
```


---

## Max Upslope Value

**Function name:** `max_upslope_value`


This tool calculates the maximum length of the flowpaths that run through each grid cell (in map horizontal units) in an input digital elevation model (`dem`). The tool works by first calculating the D8 flow pointer (`d8_pointer`) from the input DEM. The DEM must be depressionless and should have been pre-processed using the `breach_depressions_least_cost` or `fill_depressions` tool. The user must also specify the name of output raster (`output`). 

### See Also

 

`d8_pointer`, `breach_depressions_least_cost`, `fill_depressions`, `average_upslope_flowpath_length`, `downslope_flowpath_length`, `downslope_distance_to_stream` 

### Python API

```python
def max_upslope_value(self, dem: Raster, values_raster: Raster) -> Raster:
```


---

## MD-Infinity Flow Accum

**Function name:** `mdinf_flow_accum`


This tool is used to generate a flow accumulation grid (i.e. contributing area) using the MD-infinity algorithm (Seibert and McGlynn, 2007). This algorithm is an examples of a multiple-flow-direction (MFD) method because the flow entering each grid cell is routed to one or two downslope neighbour, i.e. flow divergence is permitted. The user must specify the name of the input digital elevation model (`dem`). The DEM should have been hydrologically corrected to remove all spurious depressions and flat areas. DEM pre-processing is usually achieved using the `breach_depressions_least_cost` or `fill_depressions` tool. 

In addition to the input flow-pointer grid name, the user must specify the output type (`out_type`). The output flow-accumulation can be 1) specific catchment area (SCA), which is the upslope contributing area divided by the contour length (taken as the grid resolution), 2) total catchment area in square-metres, or 3) the number of upslope grid cells. The user must also specify whether the output flow-accumulation grid should be log-tranformed, i.e. the output, if this option is selected, will be the natural-logarithm of the accumulated area. This is a transformation that is often performed to better visualize the contributing area distribution. Because contributing areas tend to be very high along valley bottoms and relatively low on hillslopes, when a flow-accumulation image is displayed, the distribution of values on hillslopes tends to be 'washed out' because the palette is stretched out to represent the highest values. Log-transformation (`log`) provides a means of compensating for this phenomenon. Importantly, however, log-transformed flow-accumulation grids must not be used to estimate other secondary terrain indices, such as the wetness index, or relative stream power index. 

Grid cells possessing the NoData value in the input DEM raster are assigned the NoData value in the output flow-accumulation image. The output raster is of the float data type and continuous data scale. 

### Reference

 

Seibert, J. and McGlynn, B.L., 2007. A new triangular multiple flow direction algorithm for computing upslope areas from gridded digital elevation models. Water resources research, 43(4). 

### See Also

 

`D8FlowAccumulation`, `FD8FlowAccumulation`, `quinn_flow_accumulation`, `qin_flow_accumulation`, `DInfFlowAccumulation`, `MDInfFlowAccumulation`, `rho8_pointer`, `breach_depressions_least_cost` 

### Python API

```python
def mdinf_flow_accum(self, dem: Raster, out_type: str = "sca", exponent: float = 1.1, convergence_threshold: float = float('inf'), log_transform: bool = False, clip: bool = False) -> Raster:
```


---

## Minimal Dispersion Flow Algorithm

**Function name:** `minimal_dispersion_flow_algorithm`


Experimental

Generates MDFA flow-direction and flow-accumulation rasters from a DEM.

hydrology flow-direction flow-accumulation mdfa

### Examples

*Compute MDFA direction and specific contributing area from DEM*


---

## Num Inflowing Neighbours

**Function name:** `num_inflowing_neighbours`


This tool calculates the number of inflowing neighbours for each grid cell in a raster file. The user must specify the names of an input digital elevation model (DEM) file (`dem`) and the output raster file (`output`). The tool calculates the D8 pointer file internally in order to identify inflowing neighbouring cells. 

Grid cells in the input DEM that contain the NoData value will be assigned the NoData value in the output image. The output image is of the integer data type and continuous data scale. 

### See Also

 

`num_downslope_neighbours`, `NumUpslopeNeighbours` 

### Python API

```python
def num_inflowing_neighbours(self, dem: Raster) -> Raster:
```


---

## Qin Flow Accumulation

**Function name:** `qin_flow_accumulation`


This tool is used to generate a flow accumulation grid (i.e. contributing area) using the Qin et al. (2007)  flow algorithm, not to be confused with the similarly named `quinn_flow_accumulation` tool. This algorithm is an  examples of a multiple-flow-direction (MFD) method because the flow entering each grid cell is routed to more  than one downslope neighbour, i.e. flow *divergence* is permitted. It is based on a modification of the Freeman (1991; `FD8FlowAccumulation`) and Quinn et al. (1995; `quinn_flow_accumulation`) methods. The Qin method relates  the degree of flow dispersion from a grid cell to the local maximum downslope gradient. Specifically, steeper  terrain experiences more convergent flow while flatter slopes experience more flow divergence.  

The following equations are used to calculate the portion flow (*Fi*) given to each neighbour, *i*:  

*Fi* = *Li*(tan&beta;)*f(e)* / &Sigma;*i*=1*n*[*Li*(tan&beta;)*f(e)*] 

*f(e)* = min(*e*, *eU*) / *eU* &times; (*pU* - 1.1) + 1.1  

Where *Li* is the contour length, and is 0.5&times;cell size for cardinal directions and 0.354&times;cell size for diagonal directions, *n* = 8, and represents each of the eight neighbouring grid cells. The exponent *f(e)* controls the proportion of flow allocated to each downslope neighbour of a grid cell, based on the local maximum downslope gradient (*e*), and the user-specified upper boundary of *e* (*eU*; `max_slope`), and the upper  boundary of the exponent (*pU*; `exponent`), *f(e)*. Note that the original Qin (2007) implementation allowed for user-specified lower boundaries on the slope (*eL*) and exponent (*pL*)  parameters as well. In this implementation, these parameters are assumed to be 0.0 and 1.1 respectively, and are not user adjustable. Also note, the `exponent` parameter should be less than 50.0, as higher values may cause numerical instability. 

The user must specify the  name (`dem`) of the input digital elevation model (DEM) and the output file (`output`).  The DEM must have been hydrologically corrected to remove all spurious depressions and flat areas. DEM  pre-processing is usually achieved using either the `breach_depressions_least_cost` (also `breach_depressions_least_cost`) or  `fill_depressions` tool.  

The user-specified non-dispersive, channel initiation *threshold* (`threshold`) is a flow-accumulation  value (measured in upslope grid cells, which is directly proportional to area) above which flow dispersion is  no longer permitted. Grid cells with flow-accumulation values above this area threshold will have their flow routed in a manner that is similar to the D8 single-flow-direction algorithm, directing all flow towards the steepest downslope neighbour. This is usually done under the assumption that flow dispersion, whilst appropriate on hillslope areas, is not realistic once flow becomes channelized. Importantly, the `threshold` parameter sets  the spatial extent of the stream network, with lower values resulting in more extensive networks.  

In addition to the input DEM, output file (`output`), and exponent, the user must also specify the output type (`out_type`). The output flow-accumulation can be: 1) `cells` (i.e. the number of inflowing grid cells), `catchment area` (i.e. the upslope area), or `specific contributing area` (i.e. the catchment area divided by the flow width). The default value is `specific contributing area`. The user must also specify whether the output flow-accumulation grid should be log-tranformed (`log`), i.e. the output, if this option is selected, will be the natural-logarithm of the accumulated flow value. This is a transformation that is often performed to better visualize the contributing area distribution. Because contributing areas tend to be very high along valley bottoms and relatively low on hillslopes, when a flow-accumulation image is displayed, the distribution of values on hillslopes tends to be 'washed out' because the palette is stretched out to represent the highest values. Log-transformation provides a means of compensating for this phenomenon. Importantly, however, log-transformed flow-accumulation grids must not be used to estimate other secondary terrain indices, such as the wetness index (`wetness_index`), or relative stream power index (`StreamPowerIndex`). 

### Reference

 

Freeman, T. G. (1991). Calculating catchment area with divergent flow based on a regular grid. Computers and Geosciences, 17(3), 413-422. 

Qin, C., Zhu, A. X., Pei, T., Li, B., Zhou, C., & Yang, L. 2007. An adaptive approach to selecting a  flow‐partition exponent for a multiple‐flow‐direction algorithm. *International Journal of Geographical  Information Science*, 21(4), 443-458. 

Quinn, P. F., K. J. Beven, Lamb, R. 1995. The in (a/tanβ) index: How to calculate it and how to use it within  the topmodel framework. *Hydrological Processes* 9(2): 161-182. 

### See Also

 

`D8FlowAccumulation`, `quinn_flow_accumulation`, `FD8FlowAccumulation`, `DInfFlowAccumulation`, `MDInfFlowAccumulation`, `rho8_pointer`, `wetness_index` 

### Python API

```python
def qin_flow_accumulation(self, dem: Raster, out_type: str = "sca", exponent: float = 10.0, max_slope: float = 45.0, convergence_threshold: float = float('inf'), log_transform: bool = False, clip: bool = False) -> Raster:
```


---

## Quinn Flow Accumulation

**Function name:** `quinn_flow_accumulation`


This tool is used to generate a flow accumulation grid (i.e. contributing area) using the Quinn et al. (1995)  flow algorithm, sometimes called QMFD or QMFD2, and not to be confused with the similarly named `qin_flow_accumulation` tool. This algorithm is an examples of a multiple-flow-direction (MFD) method because the flow entering each grid cell is routed to more than one downslope neighbour, i.e. flow *divergence* is permitted. The user must specify the name (`dem`) of the input digital elevation model (DEM). The DEM must have been hydrologically corrected to remove all spurious depressions and flat areas. DEM pre-processing is usually achieved using either the `breach_depressions_least_cost` (also `breach_depressions_least_cost`) or `fill_depressions` tool. A value must also be specified for the exponent parameter (`exponent`), a number that controls the degree of dispersion in the resulting flow-accumulation grid. A lower value yields greater apparent flow dispersion across divergent hillslopes. The exponent value (*h*) should probably be less than 50.0, as higher values may cause numerical instability, and values between 1 and 2 are most common.  The following equations are used to calculate the portion flow (*Fi*) given to each neighbour, *i*:  

*Fi* = *Li*(tan&beta;)*p* / &Sigma;*i*=1*n*[*Li*(tan&beta;)*p*] 

*p* = (*A* / *threshold* + 1)*h*  

Where *Li* is the contour length, and is 0.5&times;cell size for cardinal directions and 0.354&times;cell size for diagonal directions, *n* = 8, and represents each of the eight neighbouring grid cells, and, *A* is the flow accumulation value assigned to the current grid cell, that is being  apportioned downslope. The non-dispersive, channel initiation *threshold* (`threshold`) is a flow-accumulation  value (measured in upslope grid cells, which is directly proportional to area) above which flow dispersion is  no longer permitted. Grid cells with flow-accumulation values above this threshold will have their flow routed  in a manner that is similar to the D8 single-flow-direction algorithm, directing all flow towards the steepest  downslope neighbour. This is usually done under the assumption that flow dispersion, whilst appropriate on  hillslope areas, is not realistic once flow becomes channelized. Importantly, the `threshold` parameter sets  the spatial extent of the stream network, with lower values resulting in more extensive networks.  

In addition to the input DEM, output file (`output`), and exponent, the user must also specify the output type (`out_type`). The output flow-accumulation can be: 1) `cells` (i.e. the number of inflowing grid cells), `catchment area` (i.e. the upslope area), or `specific contributing area` (i.e. the catchment area divided by the flow width). The default value is `specific contributing area`. The user must also specify whether the output flow-accumulation grid should be log-transformed (`log`), i.e. the output, if this option is selected, will be the natural-logarithm of the accumulated flow value. This is a transformation that is often performed to better visualize the contributing area distribution. Because contributing areas tend to be very high along valley bottoms and relatively low on hillslopes, when a flow-accumulation image is displayed, the distribution of values on hillslopes tends to be 'washed out' because the palette is stretched out to represent the highest values. Log-transformation provides a means of compensating for this phenomenon. Importantly, however, log-transformed flow-accumulation grids must not be used to estimate other secondary terrain indices, such as the wetness index (`wetness_index`), or relative stream power index (`StreamPowerIndex`). The Quinn et al. (1995) algorithm is commonly used to calculate wetness index. 

### Reference

 

Quinn, P. F., K. J. Beven, Lamb, R. 1995. The in (a/tanβ) index: How to calculate it and how to use it within  the topmodel framework. *Hydrological Processes* 9(2): 161-182. 

### See Also

 

`D8FlowAccumulation`, `qin_flow_accumulation`, `FD8FlowAccumulation`, `DInfFlowAccumulation`, `MDInfFlowAccumulation`, `rho8_pointer`, `wetness_index` 

### Python API

```python
def quinn_flow_accumulation(self, dem: Raster, out_type: str = "sca", exponent: float = 1.1, convergence_threshold: float = float('inf'), log_transform: bool = False, clip: bool = False) -> Raster:
```


---

## Rho8 Flow Accum

**Function name:** `rho8_flow_accum`


This tool is used to generate a flow accumulation grid (i.e. contributing area) using the Fairfield and Leymarie (1991)  flow algorithm, often called Rho8. Like the D8 flow method, this algorithm is an examples of a single-flow-direction (SFD) method because the flow entering each grid cell is routed to only one downslope neighbour, i.e. flow *divergence* is not permitted. The user must specify the name of the input file (`input`), which may be either a digital elevation model (DEM) or a Rho8 pointer file (see `rho8_pointer`). If a DEM is input, it must have been hydrologically corrected to remove all spurious depressions and flat areas. DEM pre-processing is usually achieved using either the `breach_depressions_least_cost` (also `breach_depressions_least_cost`) or `fill_depressions` tool.  

In addition to the input and output (`output`)files, the user must also specify the output type (`out_type`). The output flow-accumulation can be: 1) `cells` (i.e. the number of inflowing grid cells), `catchment area` (i.e. the upslope area), or `specific contributing area` (i.e. the catchment area divided by the flow width). The default value is `specific contributing area`. The user must also specify whether the output flow-accumulation grid should be log-tranformed (`log`), i.e. the output, if this option is selected, will be the natural-logarithm of the accumulated flow value. This is a transformation that is often performed to better visualize the contributing area distribution. Because contributing areas tend to be very high along valley bottoms and relatively low on hillslopes, when a flow-accumulation image is displayed, the distribution of values on hillslopes tends to be 'washed out' because the palette is stretched out to represent the highest values. Log-transformation provides a means of compensating for this phenomenon. Importantly, however, log-transformed flow-accumulation grids must not be used to estimate other secondary terrain indices, such as the wetness index (`wetness_index`), or relative stream power index (`StreamPowerIndex`). 

If a Rho8 pointer is used as the input raster, the user must specify this (`pntr`). Similarly,  if a pointer input is used and the pointer follows the Esri pointer convention, rather than the  default WhiteboxTools convension for pointer files, then this must also be specified (`esri_pntr`). 

### Reference

 

Fairfield, J., and Leymarie, P. 1991. Drainage networks from grid digital elevation models. *Water Resources Research*, 27(5), 709-717. 

### See Also

 

`rho8_pointer`, `D8FlowAccumulation`, `qin_flow_accumulation`, `FD8FlowAccumulation`, `DInfFlowAccumulation`, `MDInfFlowAccumulation`, `wetness_index` 

### Python API

```python
def rho8_flow_accum(self, raster: Raster, out_type: str = "sca", log_transform: bool = False, clip: bool = False, input_is_pointer: bool = False, esri_pntr: bool = False) -> Raster:
```


---

## Rho8 Pointer

**Function name:** `rho8_pointer`


This tool is used to generate a flow pointer grid (i.e. flow direction) using the stochastic Rho8 (J. Fairfield and P. Leymarie, 1991) algorithm. Like the D8 flow algorithm (`d8_pointer`), Rho8 is a single-flow-direction (SFD) method because the flow entering each grid cell is routed to only one downslope neighbour, i.e. flow divergence is not permitted. The user must specify the name of a digital elevation model (DEM) file (`dem`) that has been hydrologically corrected to remove all spurious depressions and flat areas (`breach_depressions_least_cost`, `fill_depressions`). The output of this tool (`output`) is often used as the input to the `Rho8FlowAccumulation` tool. 

By default, the Rho8 flow pointers use the following clockwise, base-2 numeric index convention:  ... 641281 3202 1684   

Notice that grid cells that have no lower neighbours are assigned a flow direction of zero. In a DEM that has been pre-processed to remove all depressions and flat areas, this condition will only occur along the edges of the grid. If the pointer file contains ESRI flow direction values instead, the `esri_pntr` parameter must be specified. 

Grid cells possessing the NoData value in the input DEM are assigned the NoData value in the output image. 

### Memory Usage

 

The peak memory usage of this tool is approximately 10 bytes per grid cell. 

### References

 

Fairfield, J., and Leymarie, P. 1991. Drainage networks from grid digital elevation models. *Water Resources Research*, 27(5), 709-717. 

### See Also

 

`Rho8FlowAccumulation`, `d8_pointer`, `fd8_pointer`, `DInfPointer`, `breach_depressions_least_cost`, `fill_depressions` 

### Python API

```python
def rho8_pointer(self, dem: Raster, esri_pntr: bool = False) -> Raster:
```


---

## Trace Downslope Flowpaths

**Function name:** `trace_downslope_flowpaths`


This tool can be used to mark the flowpath initiated from user-specified locations downslope and terminating at either the grid's edge or a grid cell with undefined flow direction. The user must input the name of a D8 flow pointer grid (`d8_pntr`) and an input vector file indicating the location of one or more initiation points, i.e. 'seed points' (`seed_pts`). The seed point file must be a vector of the POINT VectorGeometryType. Note that the flow pointer should be generated from a DEM that has been processed to remove all topographic depression (see `breach_depressions_least_cost` and `fill_depressions`) and created using the D8 flow algorithm (`d8_pointer`). 

### See Also

 

`d8_pointer`, `breach_depressions_least_cost`, `fill_depressions`, `downslope_flowpath_length`, `downslope_distance_to_stream` 

### Python API

```python
def trace_downslope_flowpaths(self, seed_points: Vector, d8_pointer: Raster, esri_pntr: bool = False, zero_background: bool = False) -> Raster:
```
