# Stream Ordering and Metrics


---

## Hack Stream Order

**Function name:** `hack_stream_order`


This tool can be used to assign the Hack stream order to each link in a stream network. According to this common stream numbering system, the main stream is assigned an order of one. All tributaries to the main stream (i.e. the trunk) are assigned an order of two; tributaries to second-order links are assigned an order of three, and so on. The trunk or main stream of the stream network can be defined either based on the furthest upstream distance, at each bifurcation (i.e. network junction). 

Stream order is often used in hydro-geomorphic and ecological studies to quantify the relative size and importance of a stream segment to the overall river system. Unlike some other stream ordering systems, e.g. Horton-Strahler stream order (`strahler_stream_order`) and Shreve's stream magnitude (`shreve_stream_magnitude`), Hack's stream ordering method increases from the catchment outlet towards the channel heads. This has the main advantage that the catchment outlet is likely to be accurately located while the channel network extent may be less accurately mapped. 

The user must input a streams raster image (`streams_raster`) and D8 pointer image (`d8_pntr`). Stream cells are designated in the streams image as all positive, nonzero values. Thus all non-stream or background grid cells are commonly assigned either zeros or NoData values. The pointer image is used to traverse the stream network and should only be created using the D8 algorithm (`d8_pointer`). Background cells will be assigned the NoData value in the output image, unless the `zero_background=True`, in which case non-stream cells will be assigned zero values in the output. 

By default, the pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools. If the pointer file contains ESRI flow direction values instead, the user should specify `esri_pntr=True`. 

### Reference

 

Hack, J. T. (1957). Studies of longitudinal stream profiles in Virginia and Maryland (Vol. 294). US Government Printing Office. 

### See Also

 

`horton_stream_order`, `strahler_stream_order`, `shreve_stream_magnitude`, `topological_stream_order` 

### Python API

```python
def hack_stream_order(self, d8_pntr: Raster, streams_raster: Raster, esri_pntr: bool = False, zero_background: bool = False) -> Raster:
```


---

## Horton Ratios

**Function name:** `horton_ratios`


This function can be used to calculate Horton's so-called laws of drainage network composition for a input stream network. The user must specify an input DEM (which has been suitably hydrologically pre-processed to remove any topographic depressions) and a raster stream network. The function will output a 4-element  tuple containing the bifurcation ratio (Rb), the length ratio (Rl), the area ratio (Ra), and the slope ratio (Rs). These indices are related to drainage network geometry and are used in some geomorphological analysis. The calculation of the ratios is based on the method described by Knighton (1998) Fluvial Forms and Processes:  A New Perspective. 

### Code Example

 

`from whitebox_workflows import WbEnvironment 

### Set up the WbW environment

 

wbe = WbEnvironment() wbe.verbose = True wbe.working_directory = '/path/to/data' 

### Read the inputs

 

dem = wbe.read_raster('DEM.tif') streams = wbe.read_raster('streams.tif') 

### Calculate the Horton ratios

 

(bifurcation_ratio, length_ratio, area_ratio, slope_ratio) = wbe.horton_ratios(dem, streams) 

### Outputs

 

print(f"Bifurcation ratio (Rb): {bifurcation_ratio:.3f}") print(f"Length ratio (Rl): {length_ratio:.3f}") print(f"Area ratio (Ra): {area_ratio:.3f}") print(f"Slope ratio (Rs): {slope_ratio:.3f}") ` 

### See Also

 

`horton_stream_order` 

### Python API

```python
def horton_ratios(self, dem: Raster, streams_raster: Raster) -> Tuple[float, float, float, float]:
```


---

## Horton Stream Order

**Function name:** `horton_stream_order`


This tool can be used to assign the Horton stream order to each link in a stream network. Stream ordering is often used in hydro-geomorphic and ecological studies to quantify the relative size and importance of a stream segment to the overall river system. There are several competing stream ordering schemes. Based on to this common stream numbering system, headwater stream links are assigned an order of one. Stream order only increases downstream when two links of equal order join, otherwise the downstream link is assigned the larger of the two link orders. 

Strahler order and Horton order are similar approaches to assigning stream network hierarchy. Horton stream order essentially starts with the Strahler order scheme, but subsequently replaces each of the assigned stream order value along the main trunk of the network with the order value of the outlet. The main channel is not treated differently compared with other tributaries in the Strahler ordering scheme. 

The user must specify input a streams raster image (`streams_raster`) and D8 pointer image (`d8_pntr`). Stream cells are designated in the streams image as all positive, nonzero values. Thus all non-stream or background grid cells are commonly assigned either zeros or NoData values. The pointer image is used to traverse the stream network and should only be created using the D8 algorithm (`d8_pointer`). Background cells will be assigned the NoData value in the output image, unless the user specifies `zero_background=True`, in which case non-stream cells will be assigned zero values in the output. 

By default, the pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools. If the pointer file contains ESRI flow direction values instead, the user must set `esri_pntr=True`. 

### Reference Horton, R. E. (1945). Erosional development of streams and their

 

drainage basins; hydrophysical approach to quantitative morphology. Geological society of America bulletin, 56(3), 275-370. 

### See Also

 

`hack_stream_order`, `shreve_stream_magnitude`, `strahler_stream_order`, `topological_stream_order` 

### Python API

```python
def horton_stream_order(self, d8_pntr: Raster, streams_raster: Raster, esri_pntr: bool = False, zero_background: bool = False) -> Raster:
```


---

## Shreve Stream Magnitude

**Function name:** `shreve_stream_magnitude`


This tool can be used to assign the Shreve stream magnitude to each link in a stream network. Stream ordering is often used in hydro-geomorphic and ecological studies to quantify the relative size and importance of a stream segment to the overall river system. There are several competing stream ordering schemes. Shreve stream magnitude is equal to the number of headwater links upstream of each link. Headwater stream links are assigned a magnitude of one. 

The user must input a streams raster image (`streams_raster`) and D8 pointer (flow direction) image (`d8_pntr`). Stream cells are designated in the streams raster as all positive, nonzero values. Thus all non-stream or background grid cells are commonly assigned either zeros or NoData values. The pointer image is used to traverse the stream network and should only be created using the D8 algorithm. Background cells will be assigned the NoData value in the output image, unless the user specifies `zero_background=True`, in which case non-stream cells will be assigned zero values in the output. 

By default, the pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools. If the pointer file contains ESRI flow direction values instead, the user should specify `esri_pntr=True`. 

### Reference Shreve, R. L. (1966). Statistical law of stream numbers. The Journal

 

of Geology, 74(1), 17-37. 

### See Also

 

`horton_stream_order`, `hack_stream_order`, `strahler_stream_order`, `topological_stream_order` 

### Python API

```python
def shreve_stream_magnitude(self, d8_pntr: Raster, streams_raster: Raster, esri_pntr: bool = False, zero_background: bool = False) -> Raster:
```


---

## Strahler Order Basins

**Function name:** `strahler_order_basins`


This tool will identify the catchment areas of each Horton-Strahler stream order link in a user-specified stream network (`streams`), i.e. the network's *Strahler basins*. The tool effectively performs a Horton-Strahler stream ordering operation (`horton_stream_order`) followed by by a `watershed` operation. The user must specify the name of a flow pointer (flow direction) raster (`d8_pntr`), a streams raster (`streams`), and the output raster (`output`). The flow pointer and streams rasters should be generated using the `d8_pointer` algorithm. This will require a depressionless DEM, processed using either the `breach_depressions_least_cost` or `fill_depressions` tool. 

By default, the pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools. If the pointer file contains ESRI flow direction values instead, the `esri_pntr` parameter must be specified. 

NoData values in the input flow pointer raster are assigned NoData values in the output image. 

### See Also

 

`horton_stream_order`, `watershed`, `d8_pointer`, `breach_depressions_least_cost`, `fill_depressions` 

### Python API

```python
def strahler_order_basins(self, d8_pointer: Raster, streams: Raster, esri_pntr: bool = False) -> Raster:
```


---

## Strahler Stream Order

**Function name:** `strahler_stream_order`


This tool can be used to assign the Strahler stream order to each link in a stream network. Stream ordering is often used in hydro-geomorphic and ecological studies to quantify the relative size and importance of a stream segment to the overall river system. There are several competing stream ordering schemes. Based on to this common stream numbering system, headwater stream links are assigned an order of one. Stream order only increases downstream when two links of equal order join, otherwise the downstream link is assigned the larger of the two link orders. 

Strahler order and Horton order are similar approaches to assigning stream network hierarchy. Horton stream order essentially starts with the Strahler order scheme, but subsequently replaces each of the assigned stream order value along the main trunk of the network with the order value of the outlet. The main channel is not treated differently compared with other tributaries in the Strahler ordering scheme. 

The user must input a streams raster image (`streams_raster`) and D8 pointer (flow direction) image (`d8_pntr`). Stream cells are designated in the streams image as all positive, nonzero values. Thus all non-stream or background grid cells are commonly assigned either zeros or NoData values. The pointer image is used to traverse the stream network and should only be created using the D8 algorithm (`d8_pointer`). Background cells will be assigned the NoData value in the output image, unless the user specifies `zero_background=True`, in which case non-stream cells will be assigned zero values in the output. 

By default, the pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools. If the pointer file contains ESRI flow direction values instead, the user should specify `esri_pntr=True`. 

### Reference Strahler, A. N. (1957). Quantitative analysis of watershed

 

geomorphology. Eos, Transactions American Geophysical Union, 38(6), 913-920. 

### See Also

 

`horton_stream_order`, `hack_stream_order`, `shreve_stream_magnitude`, `topological_stream_order` 

### Python API

```python
def strahler_stream_order(self, d8_pntr: Raster, streams_raster: Raster, esri_pntr: bool = False, zero_background: bool = False) -> Raster:
```


---

## Stream Link Class

**Function name:** `stream_link_class`


This tool identifies all interior and exterior links, and source, link, and sink nodes in an input stream network (`streams_raster`). The input streams raster is used to designate which grid cells contain a stream and the pointer image is used to traverse the stream network. Stream cells are designated in the streams image as all values greater than zero. Thus, all non-stream or background grid cells are commonly assigned either zeros or NoData values. Background cells will be assigned the NoData value in the output image, unless `zero_background=True`, in which case non-stream cells will be assigned zero values in the output. 

Each feature is assigned the following identifier in the output image: 

Value | Stream Type ----- | -----------  1    | Exterior Link  2    | Interior Link  3    | Source Node (head water)  4    | Link Node  5    | Sink Node 

The user must input an input stream raster (`streams_raster`) and a pointer (flow direction) raster (`d8_pntr`). The flow pointer and streams rasters should be generated using the `d8_pointer` algorithm. This will require a depressionless DEM, processed using either the `breach_depressions_least_cost` or `fill_depressions` tools. 

By default, the pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools. If the pointer file contains ESRI flow direction values instead, set `esri_pntr=True`. 

### See Also

 

`stream_link_identifier` 

### Python API

```python
def stream_link_class(self, d8_pntr: Raster, streams_raster: Raster, esri_pntr: bool = False, zero_background: bool = False) -> Raster:
```


---

## Stream Link Identifier

**Function name:** `stream_link_identifier`


This tool can be used to assign each link in a stream network a unique numeric identifier. This grid is used by a number of other stream network analysis tools. 

The input streams raster (`streams_raster`) is used to designate which grid cells contain a stream and the pointer image is used to traverse the stream network. Stream cells are designated in the streams image as all values greater than zero. Thus, all non-stream or background grid cells are commonly assigned either zeros or NoData values. Background cells will be assigned the NoData value in the output image, unless the user specifies `zero_background=True`, in which case non-stream cells will be assigned zero values in the output. 

The user must specify the name of a flow pointer (flow direction) raster (`d8_pntr`) and a streams raster (`streams_raster`). The flow pointer and streams rasters should be generated using the `d8_pointer` algorithm. This will require a depressionless DEM, processed using either the `breach_depressions_least_cost` or `fill_depressions` tool. 

By default, the pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools. If the pointer file contains ESRI flow direction values instead, set `esri_pntr=True`. 

### See Also

 

`d8_pointer`, `tributary_identifier`, `breach_depressions_least_cost`, `fill_depressions` 

### Python API

```python
def stream_link_identifier(self, d8_pntr: Raster, streams_raster: Raster, esri_pntr: bool = False, zero_background: bool = False) -> Raster:
```


---

## Stream Link Length

**Function name:** `stream_link_length`


This tool can be used to measure the length of each link in a stream network. The user must input a stream link ID raster (`streams_id_raster`), created using the `stream_link_identifier` tool, and D8 pointer raster (`d8_pointer`). The flow pointer raster is used to traverse the stream network and should only be created using the `d8_pointer` algorithm. Stream cells are designated in the stream link ID raster as all non-zero, positive values. Background cells will be assigned the NoData value in the output image, unless `zero_background=True`, in which case non-stream cells will be assigned zero values in the output. 

### See Also

 

`stream_link_identifier`, `d8_pointer`, `stream_link_slope` 

### Python API

```python
def stream_link_length(self, d8_pointer: Raster, streams_id_raster: Raster, esri_pointer: bool = False, zero_background: bool = False) -> Raster:
```


---

## Stream Link Slope

**Function name:** `stream_link_slope`


This tool can be used to measure the average slope gradient, in degrees, of each link in a raster stream network. To estimate the slope of individual grid cells in a raster stream network, use the `stream_slope_continuous` tool instead. The user must input a stream link identifier raster image (`streams_id_raster`), a D8 pointer image (`d8_pointer`), and a digital elevation model (`dem`). The pointer image is used to traverse the stream network and must only be created using the D8 algorithm (`d8_pointer`). Stream cells are designated in the streams image as all values greater than zero. Thus, all non-stream or background grid cells are commonly assigned either zeros or NoData values. Background cells will be assigned the NoData value in the output image, unless `zero_background=True`, in which case non-stream cells will be assigned zero values in the output. 

By default, the pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools. If the pointer file contains ESRI flow direction values instead, set `esri_pointer=True`. 

### See Also

 

`stream_slope_continuous`, `d8_pointer` 

### Python API

```python
def stream_link_slope(self, d8_pointer: Raster, streams_id_raster: Raster, dem: Raster, esri_pointer: bool = False, zero_background: bool = False) -> Raster:
```


---

## Stream Slope Continuous

**Function name:** `stream_slope_continuous`


This tool can be used to measure the slope gradient, in degrees, each grid cell in a raster stream network. To estimate the average slope for each link in a stream network, use the `stream_link_slope` tool instead. The user must input a stream raster image (`streams_raster`), a D8 pointer image (`d8_pointer`), and a digital elevation model (`dem`). The pointer image is used to traverse the stream network and must only be created using the D8 algorithm (`d8_pointer`). Stream cells are designated in the streams image as all values greater than zero. Thus, all non-stream or background grid cells are commonly assigned either zeros or NoData values. Background cells will be assigned the NoData value in the output image, unless `zero_background=True`, in which case non-stream cells will be assigned zero values in the output. 

By default, the pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools. If the pointer file contains ESRI flow direction values instead, set `esri_pointer=True`. 

### See Also

 

`stream_link_slope`, `d8_pointer` 

### Python API

```python
def stream_slope_continuous(self, d8_pointer: Raster, streams_raster: Raster, dem: Raster, esri_pointer: bool = False, zero_background: bool = False) -> Raster:
```


---

## Topological Stream Order

**Function name:** `topological_stream_order`


This tool can be used to assign the topological stream order to each link in a stream network. According to this stream numbering system, the link directly draining to the outlet is assigned an order of one. Each of the two tributaries draining to the order-one link are assigned an order of two, and so on until the most distant link from the catchment outlet has been assigned an order. The topological order can therefore be thought of as a measure of the topological distance of each link in the network to the catchment outlet and is likely to be related to travel time. 

The user must input a streams raster image (`streams_raster`) and D8 pointer image (`d8_pntr`). Stream cells are designated in the streams image as all positive, nonzero values. Thus all non-stream or background grid cells are commonly assigned either zeros or NoData values. The pointer image is used to traverse the stream network and should only be created using the D8 algorithm. Background cells will be assigned the NoData value in the output image, unless the `zero_background=True`, in which case non-stream cells will be assigned zero values in the output. 

By default, the pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools. If the pointer file contains ESRI flow direction values instead, set `esri_pntr=True`. 

### See Also

 

`hack_stream_order`, `horton_stream_order`, `strahler_stream_order`, `shreve_stream_magnitude` 

### Python API

```python
def topological_stream_order(self, d8_pntr: Raster, streams_raster: Raster, esri_pntr: bool = False, zero_background: bool = False) -> Raster:
```


---

## Tributary Identifier

**Function name:** `tributary_identifier`


This tool can be used to assigns a unique identifier to each tributary in a stream network. A tributary is a section of a stream network extending from a channel head downstream to a confluence with a larger stream. Relative stream size is estimated using stream length as a surrogate. Tributaries therefore extend from channel heads downstream until a confluence is encountered in which the intersecting stream is longer, or an outlet cell is detected. 

The input streams raster (`streams_raster`) is used to designate which grid cells contain a stream and the pointer image is used to traverse the stream network. Stream cells are designated in the streams image as all values greater than zero. Thus, all non-stream or background grid cells are commonly assigned either zeros or NoData values. Background cells will be assigned the NoData value in the output image, unless `zero_background=True`, in which case non-stream cells will be assigned zero values in the output. 

The user must specify the name of a flow pointer (flow direction) raster (`d8_pntr`) and a streams raster (`streams_raster`). The flow pointer and streams rasters should be generated using the `d8_pointer` algorithm. This will require a depressionless DEM, processed using either the `breach_depressions_least_cost` or `fill_depressions` tool. flow direction) raster, and the output raster. 

By default, the pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools. If the pointer file contains ESRI flow direction values instead, set `esri_pntr=True`. 

### See Also

 

`d8_pointer`, `stream_link_identifier`, `breach_depressions_least_cost`, `fill_depressions` 

### Python API

```python
def tributary_identifier(self, d8_pntr: Raster, streams_raster: Raster, esri_pntr: bool = False, zero_background: bool = False) -> Raster:
```


---

## Vector Stream Network Analysis

**Function name:** `vector_stream_network_analysis`


This tool performs common stream network analysis operations on an input vector stream file (`streams`). The network indices produced by this analysis are contained within the output vector's (`output`) attribute table. The following table shows each of the network indices that are calculated.  Index NameDescription OUTLETUnique outlet identifying value, used as basin identifier TRIB_IDUnique tributary identifying value DIST2MOUTHDistance to outlet (i.e., mouth node) DS_NODESNumber of downstream nodes TUCLTotal upstream channel length; the channel equivalent to catchment area MAXUPSDISTMaximum upstream distance HORTONHorton stream order STRAHLERStrahler stream order SHREVEShreve stream magnitude HACKHack stream order MAINSTREAMBoolean value indicating whether link is the main stream trunk of its basin MIN_ELEVMinimum link elevation (from DEM) MAX_ELEVMaximum link elevation (from DEM) IS_OUTLETBoolean value indicating whether link is an outlet link   

In addition to the input and output files, the user must also specify the name of an input DEM file (`dem`), the maximum ridge-cutting height, in DEM z units (`cutting_height`), and the snap distance used for identifying any topological errors in the stream file (`snap`).  The main function of the input DEM is to distinguish between outlet and headwater links in the network, which can be differentiated by their elevations during the priority-flood operation used in the algorithm (see Lindsay et al. 2019). The maximum ridge-cutting height parameter is useful for preventing erroneous stream capture in the headwaters when channel heads are very near (within the sanp distance), which is usually very rare. The snap distance parameter is used to deal with certain common topological errors. However, it is advisable that the input streams file be pre-processed prior to analysis.  

Note: The input streams file for this tool should be pre-processed using the `repair_stream_vector_topology` tool. **This is an important step**.  

OUTLET:  

HORTON:  

SHREVE:  

TRIB_ID:  

Many of the network indices output by this tool for vector streams have raster equivalents in WhiteboxTools. For example, see the `strahler_stream_order`, `shreve_stream_magnitude` tools. 

Tool outputs are: stream lines vector, confluences points vector, outlet points vector, and channel head points vector. 

### Reference

 

Lindsay, JB, Yang, W, Hornby, DD. 2019. Drainage network analysis and structuring of topologically noisy vector stream data. ISPRS International Journal of Geo-Information. 8(9), 422; DOI: 10.3390/ijgi8090422 

### See Also

 

`repair_stream_vector_topology`, `strahler_stream_order`, `shreve_stream_magnitude` 

### Python API

```python
def vector_stream_network_analysis(self, streams: Vector, dem: Raster, max_ridge_cutting_height: float = 10.0, snap_distance: f64 = 0.001) -> Tuple[Vector, Vector, Vector, Vector]:
```
