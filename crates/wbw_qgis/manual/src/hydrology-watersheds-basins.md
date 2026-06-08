# Watersheds and Basins


---

## Basins

**Function name:** `basins`


This tool can be used to delineate all of the drainage basins contained within a local drainage direction, or flow pointer raster (`d8_pntr`), and draining to the edge of the data. The flow pointer raster must be derived using the `d8_pointer` tool and should have been extracted from a digital elevation model (DEM) that has been hydrologically pre-processed to remove topographic depressions and flat areas, e.g. using the `breach_depressions_least_cost` tool. By default, the flow pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools:  ... 641281 3202 1684   

If the pointer file contains ESRI flow direction values instead, the `esri_pntr` parameter must be specified. 

The `basins` and `watershed` tools are similar in function but while the `watershed` tool identifies the upslope areas that drain to one or more user-specified outlet points, the `basins` tool automatically sets outlets to all grid cells situated along the edge of the data that do not have a defined flow direction (i.e. they do not have a lower neighbour). Notice that these edge outlets need not be situated along the edges of the flow-pointer raster, but rather along the edges of the region of valid data. That is, the DEM from which the flow-pointer has been extracted may incompletely fill the containing raster, if it is irregular shaped, and NoData regions may occupy the peripherals. Thus, the entire region of valid data in the flow pointer raster will be divided into a set of mutually exclusive basins using this tool. 

### See Also

 

`watershed`, `d8_pointer`, `breach_depressions_least_cost` 

### Python API

```python
def basins(self, d8_pntr: Raster, esri_pntr: bool = False) -> Raster:
```


---

## Flood Order

**Function name:** `flood_order`


This tool takes an input digital elevation model (DEM) and creates an output raster where every grid cell contains the flood order of that cell within the DEM. The flood order is the sequence of grid cells that are encountered during a search, starting from the raster grid edges and the lowest grid cell, moving inward at increasing elevations. This is in fact similar to how the highly efficient Wang and Liu (2006) depression filling algorithm and the Breach Depressions (Fast) operates. The output flood order raster contains the sequential order, from lowest edge cell to the highest pixel in the DEM. 

Like the `fill_depressions` tool, `flood_order` will read the entire DEM into memory. This may make the algorithm ill suited to processing massive DEMs except where the user's computer has substantial memory (RAM) resources. 

### Reference

 

Wang, L., and Liu, H. (2006). An efficient method for identifying and filling surface depressions in digital elevation models for hydrologic analysis and modelling. International Journal of Geographical Information Science, 20(2), 193-213. 

### See Also

 

`fill_depressions` 

### Python API

```python
def flood_order(self, dem: Raster) -> Raster:
```


---

## Hillslopes

**Function name:** `hillslopes`


This tool decrements (lowers) the elevations of pixels within an input digital elevation model (DEM) (`dem`) along an input vector stream network (`streams`) at the sites of road (`roads`) intersections. In addition to the input data layers, the user must specify the output raster DEM (`output`), and the maximum road embankment width (`width`), in map units. The road width parameter is used to determine the length of channel along stream lines, at the junctions between streams and roads, that the burning (i.e. decrementing) operation occurs. The algorithm works by identifying stream-road intersection cells, then traversing along the rasterized stream path in the upstream and downstream directions by half the maximum road embankment width. The minimum elevation in each stream traversal is identified and then elevations that are higher than this value are lowered to the minimum elevation during a second stream traversal. 

 

### Reference

 

Lindsay JB. 2016. `The practice of DEM stream burning revisited`.  Earth Surface Processes and Landforms, 41(5): 658–668. DOI: 10.1002/esp.3888 

### See Also

 

`raster_streams_to_vector`, `rasterize_streams` 

### Python API

```python
def hillslopes(self, d8_pntr: Raster, streams: Raster, esri_pntr: bool = False) -> Raster:
```


---

## Isobasins

**Function name:** `isobasins`


This tool can be used to divide a landscape into a group of nearly equal-sized watersheds, known as *isobasins*. The user must specify the name (`dem`) of a digital elevation model (DEM), the output raster name (`output`), and the isobasin target area (`size`) specified in units of grid cells. The DEM must have been hydrologically corrected to remove all spurious depressions and flat areas. DEM pre-processing is usually achieved using either the `breach_depressions_least_cost` or `fill_depressions` tool. Several temporary rasters are created during the execution and stored in memory of this tool. 

The tool can optionally (`connections`) output a CSV table that contains the upstream/downstream connections among isobasins. That is, this table will identify the downstream basin of each isobasin, or will list N/A in the event that there is no downstream basin, i.e. if it drains to an edge. Additionally, the CSV file will contain information about the number of grid cells in each isobasin and the isobasin outlet's row and column number and flow direction. The output CSV file will have the same name as the output raster, but with a *.csv file extension. 

### See Also

 

`watershed`, `basins`, `breach_depressions_least_cost`, `fill_depressions` 

### Python API

```python
def isobasins(self, dem: Raster, target_size: float, connections: bool = False, csv_file: str = "" ) -> Raster:
```


---

## Jenson Snap Pour Points

**Function name:** `jenson_snap_pour_points`


This tool measures the depth that each grid cell in an input (`dem`) raster digital elevation model (DEM) lies within a sink feature, i.e. a closed topographic depression. A sink, or depression, is a bowl-like landscape feature, which is characterized by interior drainage and groundwater recharge. The `depth_in_sink` tool operates by differencing a filled DEM, using the same depression filling method as `fill_depressions`, and the original surface model. 

In addition to the names of the input DEM (`dem`) and the output raster (`output`), the user must specify whether the background value (i.e. the value assigned to grid cells that are not contained within sinks) should be set to 0.0 (`zero_background`) Without this optional parameter specified, the tool will use the NoData value as the background value. 

### Reference

 

Antonić, O., Hatic, D., & Pernar, R. (2001). DEM-based depth in sink as an environmental estimator. Ecological Modelling, 138(1-3), 247-254. 

### See Also

 

`fill_depressions` 

### Python API

```python
def jenson_snap_pour_points(self, pour_pts: Vector, streams: Raster, snap_dist: float = 0.0) -> Vector:
```


---

## Longest Flowpath

**Function name:** `longest_flowpath`


This tool delineates the longest flowpaths for a group of subbasins or watersheds. Flowpaths are initiated along drainage divides and continue along the D8-defined flow direction until either the subbasin outlet or DEM edge is encountered. Each input subbasin/watershed will have an associated vector flowpath in the output image. `longest_flowpath` is similar to the `r.lfp` plugin tool for GRASS GIS. The length of the longest flowpath draining to an outlet is related to the time of concentration, which is a parameter used in certain hydrological models. 

The user must input the filename of a digital elevation model (DEM), a basins raster, and the output vector. The DEM must be depressionless and should have been pre-processed using the `breach_depressions_least_cost` or `fill_depressions` tool. The *basins raster* must contain features that are delineated by categorical (integer valued) unique identifier values. All non-NoData, non-zero valued grid cells in the basins raster are interpreted as belonging to features. In practice, this tool is usual run using either a single watershed, a group of contiguous non-overlapping watersheds, or a series of nested subbasins. These are often derived using the `watershed` tool, based on a series of input outlets, or the `subbasins` tool, based on an input stream network. If subbasins are input to `longest_flowpath`, each traced flowpath will include only the non-overlapping portions within nested areas. Therefore, this can be a convenient method of delineating the longest flowpath to each bifurcation in a stream network. 

The output vector file will contain fields in the attribute table that identify the associated basin unique identifier (*BASIN*), the elevation of the flowpath source point on the divide (*UP_ELEV*), the elevation of the outlet point (*DN_ELEV*), the length of the flowpath (*LENGTH*), and finally, the average slope (*AVG_SLOPE*) along the flowpath, measured as a percent grade. 

### See Also

 

`max_upslope_flowpath_length`, `breach_depressions_least_cost`, `fill_depressions`, `watershed`, `subbasins` 

### Python API

```python
def longest_flowpath(self, dem: Raster, basins: Raster) -> Vector:
```


---

## Max Branch Length

**Function name:** `max_branch_length`


Maximum branch length (`Bmax`) is the longest branch length between a grid cell's flowpath and the flowpaths initiated at each of its neighbours. It can be conceptualized as the downslope distance that a volume of water that is split into two portions by a drainage divide would travel before reuniting. 

If the two flowpaths of neighbouring grid cells do not intersect, `Bmax` is simply the flowpath length from the starting cell to its terminus at the edge of the grid or a cell with undefined flow direction (i.e. a pit cell either in a topographic depression or at the edge of a major body of water). 

The pattern of `Bmax` derived from a DEM should be familiar to anyone who has interpreted upslope contributing area images. In fact, `Bmax` can be thought of as the complement of upslope contributing area. Whereas contributing area is greatest along valley bottoms and lowest at drainage divides, `Bmax` is greatest at divides and lowest along channels. The two topographic attributes are also distinguished by their units of measurements; `Bmax` is a length rather than an area. The presence of a major drainage divide between neighbouring grid cells is apparent in a `Bmax` image as a linear feature, often two grid cells wide, of relatively high values. This property makes `Bmax` a useful land surface parameter for mapping ridges and divides. 

`Bmax` is useful in the study of landscape structure, particularly with respect to drainage patterns. The index gives the relative significance of a specific location along a divide, with respect to the dispersion of materials across the landscape, in much the same way that stream ordering can be used to assess stream size. 

 

### See Also

 

`flow_length_diff` 

### Reference

 

Lindsay JB, Seibert J. 2013. Measuring the significance of a divide to local drainage patterns. International Journal of Geographical Information Science, 27: 1453-1468. DOI: 10.1080/13658816.2012.705289 

### Python API

```python
def max_branch_length(self, dem: Raster, log_transform: bool = False) -> Raster:
```


---

## Snap Pour Points

**Function name:** `snap_pour_points`


This tool measures the depth that each grid cell in an input (`dem`) raster digital elevation model (DEM) lies within a sink feature, i.e. a closed topographic depression. A sink, or depression, is a bowl-like landscape feature, which is characterized by interior drainage and groundwater recharge. The `depth_in_sink` tool operates by differencing a filled DEM, using the same depression filling method as `fill_depressions`, and the original surface model. 

In addition to the names of the input DEM (`dem`) and the output raster (`output`), the user must specify whether the background value (i.e. the value assigned to grid cells that are not contained within sinks) should be set to 0.0 (`zero_background`) Without this optional parameter specified, the tool will use the NoData value as the background value. 

### Reference

 

Antonić, O., Hatic, D., & Pernar, R. (2001). DEM-based depth in sink as an environmental estimator. Ecological Modelling, 138(1-3), 247-254. 

### See Also

 

`fill_depressions` 

### Python API

```python
def snap_pour_points(self, pour_pts: Vector, flow_accum: Raster, snap_dist: float = 0.0) -> Vector:
```


---

## Subbasins

**Function name:** `subbasins`


This tool will identify the catchment areas to each link in a user-specified stream network, i.e. the network's sub-basins. `subbasins` effectively performs a stream link ID operation (`stream_link_identifier`) followed by a `watershed` operation. The user must specify the name of a flow pointer (flow direction) raster (`d8_pntr`), a streams raster (`streams`), and the output raster (`output`). The flow pointer and streams rasters should be generated using the `d8_pointer` algorithm. This will require a depressionless DEM, processed using either the `breach_depressions_least_cost` or `fill_depressions` tool. 

`hillslopes` are conceptually similar to sub-basins, except that sub-basins do not distinguish between the right-bank and left-bank catchment areas of stream links. The Sub-basins tool simply assigns a unique identifier to each stream link in a stream network. 

By default, the pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools. If the pointer file contains ESRI flow direction values instead, the `esri_pntr` parameter must be specified. 

NoData values in the input flow pointer raster are assigned NoData values in the output image. 

### See Also

 

`stream_link_identifier`, `watershed`, `hillslopes`, `d8_pointer`, `breach_depressions_least_cost`, `fill_depressions` 

### Python API

```python
def subbasins(self, d8_pntr: Raster, streams: Raster, esri_pntr: bool = False) -> Raster:
```


---

## Unnest Basins

**Function name:** `unnest_basins`


In some applications it is necessary to relate a measured variable for a group of hydrometric stations (e.g. characteristics of flow timing and duration or water chemistry) to some characteristics of each outlet's catchment (e.g. mean slope, area of wetlands, etc.). When the group of outlets are nested, i.e. some stations are located downstream of others, then performing a watershed operation will result in inappropriate watershed delineation. In particular, the delineated watersheds of each nested outlet will not include the catchment areas of upstream outlets. This creates a serious problem for this type of application. 

The Unnest Basin tool can be used to perform a watershedding operation based on a group of specified pour points, i.e. outlets or target cells, such that each complete watershed is delineated. The user must specify the name of a flow pointer (flow direction) raster, a pour point raster, and the name of the output rasters. Multiple numbered outputs will be created, one for each nesting level. Pour point, or target, cells are denoted in the input pour-point image as any non-zero, non-NoData value. The flow pointer raster should be generated using the D8 algorithm. 

### Python API

```python
def unnest_basins(self, d8_pointer: Raster, pour_points: Vector, esri_pntr: bool = False) -> List[Raster]:
```


---

## Watershed

**Function name:** `watershed`


This tool will perform a watershedding operation based on a group of input vector pour points (`pour_pts`), i.e. outlets or points-of-interest. Watershedding is a procedure that identifies all of the cells upslope of a cell of interest (pour point) that are connected to the pour point by a flow-path. The user must input a D8-derived flow pointer (flow direction) raster (`d8_pntr`) and a vector pour point file (`pour_pts`). The pour points must be of a Point ShapeType (i.e. Point, PointZ, PointM, MultiPoint, MultiPointZ, MultiPointM). Watersheds will be assigned the input pour point FID value. The flow pointer raster must be generated using the D8 algorithm, `d8_pointer`. 

Pour point vectors can be attained by on-screen digitizing to designate these points-of-interest locations. Because pour points are usually, although not always, situated on a stream network, it is recommended that you use Jenson's method (`jenson_snap_pour_points`) to snap pour points on the stream network. This will ensure that the digitized outlets are coincident with the digital stream contained within the DEM flowpaths. If this is not done prior to inputting a pour-point set to the `watershed` tool, anomalously small watersheds may be output, as pour points that fall off of the main flow path (even by one cell) in the D8 pointer will yield very different catchment areas. 

If a raster pour point is specified instead of vector points, the watershed labels will derive their IDs from the grid cell values of all non-zero, non-NoData valued grid cells in the pour points file. Notice that this file can contain any integer data. For example, if a lakes raster, with each lake possessing a unique ID, is used as the pour points raster, the tool will map the watersheds draining to each of the input lake features. Similarly, a pour points raster may actually be a streams file, such as what is generated by the `stream_link_identifier` tool. 

By default, the pointer raster is assumed to use the clockwise indexing method used by Whitebox Workflows. If the pointer file contains ESRI flow direction values instead, the `esri_pntr` must be True. 

There are several tools that perform similar watershedding operations in Whitebox Workflows. `watershed` is appropriate to use when you have a set of specific locations for which you need to derive the watershed areas. Use the `basins` tool instead when you simply want to find the watersheds draining to each outlet situated along the edge of a DEM. The `isobasins` tool can be used to divide a landscape into roughly equally sized watersheds. The `subbasins` and `strahler_order_basins` are useful when you need to find the areas draining to each link within a stream network. Finally, `hillslopes` can be used to identify the areas draining the each of the left and right banks of a stream network. 

### Reference

 

Jenson, S. K. (1991), Applications of hydrological information automatically extracted from digital elevation models, Hydrological Processes, 5, 31–44, doi:10.1002/hyp.3360050104. 

Lindsay JB, Rothwell JJ, and Davies H. 2008. Mapping outlet points used for watershed delineation onto DEM-derived stream networks, Water Resources Research, 44, W08442, doi:10.1029/2007WR006507. 

### See Also

 

`d8_pointer`, `basins`, `subbasins`, `isobasins`, `strahler_order_basins`, `hillslopes`, `jenson_snap_pour_points`, `breach_depressions_least_cost`, `fill_depressions` 

### Python API

```python
def watershed(self, d8_pointer: Raster, pour_points: Vector, esri_pntr: bool = False) -> Raster:
```


---

## Watershed From Raster Pour Points

**Function name:** `watershed_from_raster_pour_points`


This tool will perform a watershedding operation based on a group of input raster containing point points (`pour_points`).  Watershedding is a procedure that identifies all of the cells upslope of a cell of interest (pour point) that are connected to the pour point by a flow-path. The user must input a D8-derived flow pointer (flow direction) raster (`d8_pointer`) and a pour points raster (`pour_points`). The flow pointer raster must be generated using the D8 algorithm, `d8_pointer`. 

Watershed labels will derive their IDs from the grid cell values of all non-zero, non-NoData valued grid cells in the pour points file. Notice that this file can contain any integer data. For example, if a lakes raster, with each lake possessing a unique ID, is used as the pour points raster, the tool will map the watersheds draining to each of the input lake features. Similarly, a pour points raster may actually be a streams file, such as what is generated by the `stream_link_identifier` tool. 

By default, the pointer raster is assumed to use the clockwise indexing method used by Whitebox Workflows. If the pointer file contains ESRI flow direction values instead, the `esri_pntr` parameter must be specified. 

There are several tools that perform similar watershedding operations in Whitebox Workflows. `watershed` is appropriate to use when you have a set of specific locations for which you need to derive the watershed areas. Use the `basins` tool instead when you simply want to find the watersheds draining to each outlet situated along the edge of a DEM. The `isobasins` tool can be used to divide a landscape into roughly equally sized watersheds. The `subbasins` and `strahler_order_basins` are useful when you need to find the areas draining to each link within a stream network. Finally, `hillslopes` can be used to identify the areas draining the each of the left and right banks of a stream network. 

### Reference

 

Jenson, S. K. (1991), Applications of hydrological information automatically extracted from digital elevation models, Hydrological Processes, 5, 31–44, doi:10.1002/hyp.3360050104. 

Lindsay JB, Rothwell JJ, and Davies H. 2008. Mapping outlet points used for watershed delineation onto DEM-derived stream networks, Water Resources Research, 44, W08442, doi:10.1029/2007WR006507. 

### See Also

 

`d8_pointer`, `basins`, `subbasins`, `isobasins`, `strahler_order_basins`, `hillslopes`, `jenson_snap_pour_points`, `breach_depressions_least_cost`, `fill_depressions` 

### Python API

```python
def watershed_from_raster_pour_points(self, d8_pointer: Raster, pour_points: Raster, esri_pntr: bool = False) -> Raster:
```
