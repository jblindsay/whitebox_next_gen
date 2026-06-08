# Depressions and Storage


---

## Breach Depressions Least Cost

**Function name:** `breach_depressions_least_cost`


This tool can be used to perform a type of optimal depression breaching to prepare a digital elevation model (DEM) for hydrological analysis. Depression breaching is a common alternative to depression filling (`fill_depressions`) and often offers a lower-impact solution to the removal of topographic depressions. This tool implements a method that is loosely based on the algorithm described by Lindsay and Dhun (2015), furthering the earlier algorithm with efficiency optimizations and other significant enhancements. The approach uses a least-cost path analysis to identify the breach channel that connects pit cells (i.e. grid cells for which there is no lower neighbour) to some distant lower cell. Prior to breaching and in order to minimize the depth of breach channels, all pit cells are rised to the elevation of the lowest neighbour minus a small heigh value. Here, the cost of a breach path is determined by the amount of elevation lowering needed to cut the breach channel through the surrounding topography. 

The user must specify the name of the input DEM file (`dem`), the output breached DEM file (`output`), the maximum search window radius (`dist`), the optional maximum breach cost (`max_cost`), and an optional flat height increment value (`flat_increment`). Notice that **if the `flat_increment` parameter is not specified, the small number used to ensure flow across flats will be calculated automatically, which should be preferred in most applications** of the tool. The tool operates by performing a least-cost path analysis for each pit cell, radiating outward until the operation identifies a potential breach destination cell or reaches the maximum breach length parameter. If a value is specified for the optional `max_cost` parameter, then least-cost breach paths that would require digging a channel that is more costly than this value will be left unbreached. The flat increment value is used to ensure that there is a monotonically descending path along breach channels to satisfy the necessary condition of a downslope gradient for flowpath modelling. It is best for this value to be a small value. If left unspecified, the tool with determine an appropriate value based on the range of elevation values in the input DEM, **which should be the case in most applications**. Notice that the need to specify these very small elevation increment values is one of the reasons why the output DEM will always be of a 64-bit floating-point data type, which will often double the storage requirements of a DEM (DEMs are often store with 32-bit precision). Lastly, the user may optionally choose to apply depression filling (`fill`) on any depressions that remain unresolved by the earlier depression breaching operation. This filling step uses an efficient filling method based on flooding depressions from their pit cells until outlets are identified and then raising the elevations of flooded cells back and away from the outlets. 

The tool can be run in two modes, based on whether the `min_dist` is specified. If the `min_dist` flag is specified, the accumulated cost (accum2) of breaching from *cell1* to *cell2* along a channel issuing from *pit* is calculated using the traditional cost-distance function:  

cost1 = z1 - (zpit + *l* &times; *s*) 

cost2 = z2 - [zpit + (*l* + 1)*s*] 

accum2 = accum1 + *g*(cost1 + cost2) / 2.0  

where cost1 and cost2 are the costs associated with moving through *cell1* and *cell2* respectively, z1 and z2 are the elevations of the two cells, zpit is the elevation of the pit cell, *l* is the length of the breach channel to *cell1*, *g* is the grid cell distance between cells (accounting for diagonal distances), and *s* is the small number used to ensure flow across flats. If the `min_dist` flag is not present, the accumulated cost is calculated as:  

accum2 = accum1 + cost2  

That is, without the `min_dist` flag, the tool works to minimize elevation changes to the DEM caused by breaching, without considering the distance of breach channels. Notice that the value `max_cost`, if specified, should account for this difference in the way cost/cost-distances are calculated. The first cell in the least-cost accumulation operation that is identified for which cost2 <= 0.0 is the target cell to which the breach channel will connect the pit along the least-cost path. 

In comparison with the `breach_depressions_least_cost` tool, this breaching method often provides a more satisfactory, lower impact, breaching solution and is often more efficient. It is therefore advisable that users try the `breach_depressions_least_cost` tool to remove depressions from their DEMs first. This tool is particularly well suited to breaching through road embankments. There are instances when a breaching solution is inappropriate, e.g. when a very deep depression such as an open-pit mine occurs in the DEM and long, deep breach paths are created. Often restricting breaching with the `max_cost` parameter, combined with subsequent depression filling (`fill`) can provide an adequate solution in these cases. Nonetheless, there are applications for which full depression filling using the  `fill_depressions` tool may be preferred. 

### Reference

 

Lindsay J, Dhun K. 2015. Modelling surface drainage patterns in altered landscapes using LiDAR. *International Journal of Geographical Information Science*, 29: 1-15. DOI: 10.1080/13658816.2014.975715 

### See Also

 

`breach_depressions_least_cost`, `fill_depressions`, `cost_pathway` 

### Python API

```python
def breach_depressions_least_cost(self, dem: Raster, max_cost: float = float('inf'), max_dist: int = 100, flat_increment: float = float('nan'), fill_deps: bool = False, minimize_dist: bool = False) -> Raster:
```


---

## Breach Single Cell Pits

**Function name:** `breach_single_cell_pits`


This tool calculates the average slope gradient (i.e. slope steepness in degrees) of the flowpaths that pass through each grid cell in an input digital elevation model (DEM). The user must specify the name of a DEM raster (`dem`). It is important that this DEM is pre-processed to remove all topographic depressions and flat areas using a tool such as `breach_depressions_least_cost`. Several intermediate rasters are created and stored in memory during the operation of this tool, which may limit the size of DEM that can be processed, depending on available system resources. 

### See Also

 

`average_upslope_flowpath_length`, `breach_depressions_least_cost` 

### Python API

```python
def breach_single_cell_pits(self, dem: Raster) -> Raster:
```


---

## Burn Streams

**Function name:** `burn_streams`


Stable

Burns a stream network into a DEM by decreasing stream-cell elevations.

stream_network dem_preprocessing


---

## Burn Streams At Roads

**Function name:** `burn_streams_at_roads`


This tool decrements (lowers) the elevations of pixels within an input digital elevation model (DEM) (`dem`) along an input vector stream network (`streams`) at the sites of road (`roads`) intersections. In addition to the input data layers, the user must specify the output raster DEM (`output`), and the maximum road embankment width (`width`), in map units. The road width parameter is used to determine the length of channel along stream lines, at the junctions between streams and roads, that the burning (i.e. decrementing) operation occurs. The algorithm works by identifying stream-road intersection cells, then traversing along the rasterized stream path in the upstream and downstream directions by half the maximum road embankment width. The minimum elevation in each stream traversal is identified and then elevations that are higher than this value are lowered to the minimum elevation during a second stream traversal. 

 

### Reference

 

Lindsay JB. 2016. `The practice of DEM stream burning revisited`.  Earth Surface Processes and Landforms, 41(5): 658–668. DOI: 10.1002/esp.3888 

### See Also

 

`raster_streams_to_vector`, `rasterize_streams` 

### Python API

```python
def burn_streams_at_roads(self, dem: Raster, streams: Vector, roads: Vector, road_width: float) -> Raster:
```


---

## Depth In Sink

**Function name:** `depth_in_sink`


This tool measures the depth that each grid cell in an input (`dem`) raster digital elevation model (DEM) lies within a sink feature, i.e. a closed topographic depression. A sink, or depression, is a bowl-like landscape feature, which is characterized by interior drainage and groundwater recharge. The `depth_in_sink` tool operates by differencing a filled DEM, using the same depression filling method as `fill_depressions`, and the original surface model. 

In addition to the names of the input DEM (`dem`) and the output raster (`output`), the user must specify whether the background value (i.e. the value assigned to grid cells that are not contained within sinks) should be set to 0.0 (`zero_background`) Without this optional parameter specified, the tool will use the NoData value as the background value. 

### Reference

 

Antonić, O., Hatic, D., & Pernar, R. (2001). DEM-based depth in sink as an environmental estimator. Ecological Modelling, 138(1-3), 247-254. 

### See Also

 

`fill_depressions` 

### Python API

```python
def depth_in_sink(self, dem: Raster, zero_background: bool = False) -> Raster:
```


---

## Fill Burn

**Function name:** `fill_burn`


Burns streams into a DEM using the FillBurn (Saunders, 1999) method which produces a hydro-enforced DEM. This tool uses the algorithm described in: 

Lindsay JB. 2016. The practice of DEM stream burning revisited. Earth Surface Processes and Landforms, 41(5): 658-668. DOI: 10.1002/esp.3888 

And: 

Saunders, W. 1999. Preparation of DEMs for use in environmental modeling analysis, in: ESRI User Conference. pp. 24-30. 

### Python API

```python
def fill_burn(self, dem: Raster, streams: Vector) -> Raster:
```


---

## Fill Depressions

**Function name:** `fill_depressions`


This tool can be used to fill all of the depressions in a digital elevation model (DEM) and to remove the flat areas. This is a common pre-processing step required by many flow-path analysis tools to ensure continuous flow from each grid cell to an outlet located along the grid edge. The `fill_depressions` algorithm operates by first identifying single-cell pits, that is, interior grid cells with no lower neighbouring cells. Each pit cell is then visited from highest to lowest and a priority region-growing operation is initiated. The area of monotonically increasing elevation, starting from the pit cell and growing based on flood order, is identified. Once a cell, that has not been previously visited and possessing a lower elevation than its discovering neighbour cell, is identified the discovering neighbour is labelled as an outlet (spill point) and the outlet elevation is noted. The algorithm then back-fills the labelled region, raising the elevation in the output DEM (`output`) to that of the outlet. Once this process is completed for each pit cell (noting that nested pit cells are often solved by prior pits) the flat regions of filled pits are optionally treated (`fix_flats`) with an applied small slope gradient away from outlets (note, more than one outlet cell may exist for each depression). The user may optionally specify the size of the elevation increment used to solve flats (`flat_increment`), although **it is best to not specify this optional value and to let the algorithm determine the most suitable value itself**. The flat-fixing method applies a small gradient away from outlets using another priority region-growing operation (i.e. based on a priority queue operation), where priorities are set by the elevations in the input DEM (`input`). This in effect ensures a gradient away from outlet cells but also following the natural pre-conditioned topography internal to depression areas. For example, if a large filled area occurs upstream of a damming road-embankment, the filled DEM will possess flow directions that are similar to the un-flooded valley, with flow following the valley bottom. In fact, the above case is better handled using the `breach_depressions_least_cost` tool, which would simply cut through the road embankment at the likely site of a culvert. However, the flat-fixing method of `fill_depressions` does mean that this common occurrence in LiDAR DEMs is less problematic. 

The `breach_depressions_least_cost`, while slightly less efficient than either other hydrological preprocessing methods, often provides a lower impact solution to topographic depressions and should be preferred in most applications. In comparison with the `breach_depressions_least_cost` tool, the depression filling method often provides a less satisfactory, higher impact solution. **It is advisable that users try the `breach_depressions_least_cost` tool to remove depressions from their DEMs before using `fill_depressions`**. Nonetheless, there are applications for which full depression filling using the `fill_depressions` tool may be preferred. 

Note that this tool will not fill in NoData regions within the DEM. It is advisable to remove such regions using the `fill_missing_data` tool prior to application. 

### See Also

 

`breach_depressions_least_cost`, `breach_depressions_least_cost`, `sink`, `depth_in_sink`, `fill_missing_data` 

### Python API

```python
def fill_depressions(self, dem: Raster, fix_flats: bool = True, flat_increment: float = float('nan'), max_depth: float = float('inf')) -> Raster:
```


---

## Fill Depressions Planchon And Darboux

**Function name:** `fill_depressions_planchon_and_darboux`


This tool can be used to fill all of the depressions in a digital elevation model (DEM) and to remove the flat areas using the Planchon and Darboux (2002) method. This is a common pre-processing step required by many flow-path analysis tools to ensure continuous flow from each grid cell to an outlet located along the grid edge. **This tool is currently not the most efficient depression-removal algorithm available in WhiteboxTools**; `fill_depressions` and `breach_depressions_least_cost` are both more efficient and often produce better, lower-impact results. 

The user may optionally specify the size of the elevation increment used to solve flats (`flat_increment`), although **it is best not to specify this optional value and to let the algorithm determine the most suitable value itself**. 

### Reference

 

Planchon, O. and Darboux, F., 2002. A fast, simple and versatile algorithm to fill the depressions of digital elevation models. Catena, 46(2-3), pp.159-176. 

### See Also

 

`fill_depressions`, `breach_depressions_least_cost` 

### Python API

```python
def fill_depressions_planchon_and_darboux(self, dem: Raster, fix_flats: bool = True, flat_increment: float = float('nan')) -> Raster:
```


---

## Fill Depressions Wang And Liu

**Function name:** `fill_depressions_wang_and_liu`


This tool can be used to fill all of the depressions in a digital elevation model (DEM) and to remove the flat areas. This is a common pre-processing step required by many flow-path analysis tools to ensure continuous flow from each grid cell to an outlet located along the grid edge. The `fill_depressions_wang_and_liu` algorithm is based on the computationally efficient approach of examining each cell based on its spill elevation, starting from the edge cells, and visiting cells from lowest order using a priority queue. As such, it is based on the algorithm first proposed by Wang and Liu (2006). However, it is currently not the most efficient depression-removal algorithm available in WhiteboxTools; `fill_depressions` and `breach_depressions_least_cost` are both more efficient and often produce better, lower-impact results. 

If the input DEM has gaps, or missing-data holes, that contain NoData values, it is better to use the `fill_missing_data` tool to repair these gaps. This tool will interpolate values across the gaps and produce a more natural-looking surface than the flat areas that are produced by depression filling. Importantly, the `fill_depressions` tool algorithm implementation assumes that there are no 'donut hole' NoData gaps within the area of valid data. Any NoData areas along the edge of the grid will simply be ignored and will remain NoData areas in the output image. 

The user may optionally specify the size of the elevation increment used to solve flats (`flat_increment`), although **it is best not to specify this optional value and to let the algorithm determine the most suitable value itself**. 

### Reference

 

Wang, L. and Liu, H. 2006. An efficient method for identifying and filling surface depressions in digital elevation models for hydrologic analysis and modelling. International Journal of Geographical Information Science, 20(2): 193-213. 

### See Also

 

`fill_depressions`, `breach_depressions_least_cost`, `breach_depressions_least_cost`, `fill_missing_data` 

### Python API

```python
def fill_depressions_wang_and_liu(self, dem: Raster, fix_flats: bool = True, flat_increment: float = float('nan')) -> Raster:
```


---

## Fill Pits

**Function name:** `fill_pits`


This tool can be used to remove pits from a digital elevation model (DEM). Pits are single grid cells with no downslope neighbours. They are important because they impede overland flow-paths. This tool will remove any pits in the input DEM that can be resolved by raising the elevation of the pit such that flow will continue past the pit cell to one of the downslope neighbours. Notice that this tool can be a useful pre-processing technique before running one of the more robust depression breaching (`breach_depressions_least_cost`) or filling (`fill_depressions`) techniques, which are designed to remove larger depression features. 

### See Also

 

`breach_depressions_least_cost`, `fill_depressions`, `breach_single_cell_pits` 

### Python API

```python
def fill_pits(self, dem: Raster) -> Raster:
```


---

## Flatten Lakes

**Function name:** `flatten_lakes`


This tool can be used to set the elevations contained in a set of input vector lake polygons (`lakes`) to a consistent value within an input (`dem`) digital elevation model (DEM). Lake flattening is a common pre-processing step for DEMs intended for use in hydrological applications. This algorithm determines lake elevation automatically based on the minimum perimeter elevation for each lake polygon. The minimum perimeter elevation is assumed to be the lake outlet elevation and is assigned to the entire interior region of lake polygons, excluding island geometries. Note, this tool will not provide satisfactory results if the input vector polygons contain wide river features rather than true lakes. When this is the case, the tool will lower the entire river to the elevation of its mouth, leading to the creation of an artificial gorge. 

### See Also

 

`fill_depressions` 

### Python API

```python
def flatten_lakes(self, dem: Raster, lakes: Vector) -> Raster:
```


---

## Impoundment Size Index

**Function name:** `impoundment_size_index`


This tool can be used to calculate the impoundment size index (ISI) from a digital elevation model (DEM). The ISI is a land-surface parameter related to the size of the impoundment that would result from inserting a dam of a user-specified maximum length (`damlength`) into each DEM grid cell. The tool requires the user to specify the name of one or more of the possible outputs, which include the mean flooded depth (`out_mean`), the maximum flooded depth (`out_max`), the flooded volume (`out_volume`), the flooded area (`out_area`), and the dam height (`out_dam_height`). 

Please note that this tool performs an extremely complex and computationally intensive flow-accumulation operation. As such, it may take a substantial amount of processing time and may encounter issues (including memory issues) when applied to very large DEMs. It is not necessary to pre-process the input DEM (`dem`) to remove topographic depressions and flat areas. The internal flow-accumulation operation will not be confounded by the presence of these features. 

### Reference

 

Lindsay, JB (2015) Modelling the spatial pattern of potential impoundment size from DEMs. Online resource: `Whitebox Blog` 

### See Also

 

`insert_dams`, `stochastic_depression_analysis` 

### Python API

```python
def impoundment_size_index(self, dem: Raster, max_dam_length: float, output_mean: bool = False, output_max: bool = False, output_volume: bool = False, output_area: bool = False, output_height: bool = False) -> Tuple[Union[Raster, None], Union[Raster, None], Union[Raster, None], Union[Raster, None], Union[Raster, None]]:
```


---

## Insert Dams

**Function name:** `insert_dams`


This tool can be used to insert dams at one or more user-specified points (`dam_pts`), and of a maximum length (`damlength`), within an input digital elevation model (DEM) (`dem`). This tool can be thought of as providing the impoundment feature that is calculated internally during a run of the the impoundment size index (ISI) tool for a set of points of interest. from a  (DEM). 

### Reference

 

Lindsay, JB (2015) Modelling the spatial pattern of potential impoundment size from DEMs. Online resource: `Whitebox Blog` 

### See Also

 

`impoundment_size_index`, `stochastic_depression_analysis` 

### Python API

```python
def insert_dams(self, dem: Raster, dam_points: Vector, dam_length: float) -> Raster:
```


---

## Raise Walls

**Function name:** `raise_walls`


This tool is used to increment the elevations in a digital elevation model (DEM) along the boundaries of a vector lines or polygon layer. The user must specify the name of the raster DEM (`dem`), the vector file (`input`), the output file name (`output`), the increment height (`height`), and an optional breach lines vector layer (`breach`). The breach lines layer can be used to breach a whole in the raised walls at intersections with the wall layer. 

### Python API

```python
def raise_walls(self, dem: Raster, walls: Vector, breach_lines: Vector, wall_height: float = 100.0) -> Raster:
```


---

## Sink

**Function name:** `sink`


This tool measures the depth that each grid cell in an input (`dem`) raster digital elevation model (DEM) lies within a sink feature, i.e. a closed topographic depression. A sink, or depression, is a bowl-like landscape feature, which is characterized by interior drainage and groundwater recharge. The `depth_in_sink` tool operates by differencing a filled DEM, using the same depression filling method as `fill_depressions`, and the original surface model. 

In addition to the names of the input DEM (`dem`) and the output raster (`output`), the user must specify whether the background value (i.e. the value assigned to grid cells that are not contained within sinks) should be set to 0.0 (`zero_background`) Without this optional parameter specified, the tool will use the NoData value as the background value. 

### Reference

 

Antonić, O., Hatic, D., & Pernar, R. (2001). DEM-based depth in sink as an environmental estimator. Ecological Modelling, 138(1-3), 247-254. 

### See Also

 

`fill_depressions` 

### Python API

```python
def sink(self, dem: Raster, zero_background: bool = False) -> Raster:
```


---

## Stochastic Depression Analysis

**Function name:** `stochastic_depression_analysis`


This tool performs a stochastic analysis of depressions within a DEM, calculating the probability of each cell belonging to a depression. This land-surface parameter (pdep) has been widely applied in wetland and bottom-land mapping applications. 

This tool differs from the original Whitebox GAT tool in a few significant ways: 
 
-  

The Whitebox GAT tool took an error histogram as an input. In practice people found    it difficult to create this input. Usually they just generated a normal distribution    in a spreadsheet using information about the DEM root-mean-square-error (RMSE). As    such, this tool takes a RMSE input and generates the histogram internally. This is    more convienent for most applications but loses the flexibility of specifying the    error distribution more completely.  
-  

The Whitebox GAT tool generated the error fields using the turning bands method.    This tool generates a random Gaussian error field with no spatial autocorrelation    and then applies local spatial averaging using a Gaussian filter (the size of    which depends of the error autocorrelation length input) to increase the level of    autocorrelation. We use the Fast Almost Gaussian Filter of Peter Kovesi (2010),    which uses five repeat passes of a mean filter, based on an integral image. This    filter method is highly efficient. This results in a significant performance    increase compared with the original tool.  
-  

Parts of the tool's workflow utilize parallel processing. However, the depression    filling operation, which is the most time-consuming part of the workflow, is    not parallelized.  
 

In addition to the input DEM (`dem`) and output pdep file name (`output`), the user must specify the nature of the error model, including the root-mean-square error (`rmse`) and the error field correlation length (`range`, in map units). These parameters determine the statistical frequency distribution and spatial characteristics of the modeled error fields added to the DEM in each iteration of the simulation. The user must also specify the number of iterations (`iterations`). A larger number of iterations will produce a smoother pdep raster. 

This tool creates several temporary rasters in memory and, as a result, is very memory hungry. This will necessarily limit the size of DEMs that can be processed on more memory-constrained systems. As a rough guide for usage, **the computer system will need 6-10 times more memory than the file size of the DEM**. If your computer possesses insufficient memory, you may consider splitting the input DEM apart into smaller tiles. 

 

For a video demonstrating the application of the `stochastic_depression_analysis` tool, see  `this YouTube video`. 

### Reference

 

Lindsay, J. B., & Creed, I. F. (2005). Sensitivity of digital landscapes to artifact depressions in remotely-sensed DEMs. Photogrammetric Engineering & Remote Sensing, 71(9), 1029-1036. 

### See Also

 

`impoundment_size_index`, `fast_almost_gaussian_filter` 

### Python API

```python
def stochastic_depression_analysis(self, dem: Raster, rmse: float, range: float, iterations: int = 100) -> Raster:
```


---

## Topological Breach Burn

**Function name:** `topological_breach_burn`


PROExperimental

Burns streams into a DEM, conditions the surface, and returns stream, DEM, pointer, and accumulation rasters.

hydrology stream_burning d8

### Examples

*Generate topologically conditioned stream-burning outputs*


---

## Upslope Depression Storage

**Function name:** `upslope_depression_storage`


This tool estimates the average upslope depression storage depth using the FD8 flow algorithm. The input DEM (`dem`) need not be hydrologically corrected; the tool will internally map depression storage and resolve flowpaths using depression filling. This input elevation model should be of a fine resolution (< 2 m), and is ideally derived using LiDAR. The tool calculates the total upslope depth of depression storage, which is divided by the number of upslope cells in the final step of the process, yielding the average upslope depression depth. Roughened surfaces tend to have higher values compared with smoothed surfaces. Values, particularly on hillslopes, may be very small (< 0.01 m). 

### See Also

 

`FD8FlowAccumulation`, `fill_depressions`, `depth_in_sink` 

### Python API

```python
def upslope_depression_storage(self, dem: Raster) -> Raster:
```
