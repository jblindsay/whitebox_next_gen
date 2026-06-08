# Stream Network Extraction


---

## Extract Streams

**Function name:** `extract_streams`


### Description

 

This tool can be used to extract, or map, the likely stream cells from an input flow-accumulation image (`flow_accumulation`). The algorithm applies a threshold to the input flow accumulation image such that streams are considered to be all grid cells with accumulation values greater than the specified threshold (`threshold`). As such, this threshold represents the minimum area (area is used here as a surrogate for discharge) required to *initiate and maintain a channel*. Smaller threshold values result in more extensive stream networks and vice versa. Unfortunately there is very little guidance regarding an appropriate method for determining the channel initiation area threshold in practice. As such, it is frequently determined either by examining map or imagery data, using field work, or by experimentation until a suitable or desirable channel network is identified. Notice that the threshold value will be unique for each landscape and dataset (including source and grid resolution), further complicating its *a priori* determination. There is also evidence that in some landscape the threshold is a combined upslope area-slope function. Generally, a lower threshold is appropriate in humid climates and a higher threshold is appropriate in areas underlain by more resistant bedrock. Climate and bedrock resistance are two factors related to drainage density, i.e. the extent to which a landscape is dissected by drainage channels. 

The background value of the output raster will be the NoData value unless `zero_background` is set to True. 

### See Also

 

`extract_valleys` 

### Parameters

 

flow_accumulation (Raster):     The input flow accumulation Raster object. 

threshold (float):     The minimum accumulation value required to be part of a stream channel. Default is 0.0, but should be set higher. 

zero_background (bool):     Whether the output raster uses 0.0 for non-channel cells (True) or NoData (False). Default is False. 

### Returns:

 

Raster 

### Python API

```python
def extract_streams(self, flow_accumulation: Raster, threshold: float = 0.0, zero_background: bool = False) -> Raster:
```


---

## Extract Valleys

**Function name:** `extract_valleys`


This tool can be used to extract channel networks from an input digital elevation models (`dem`) using one of three techniques that are based on local topography alone. 

The Lindsay (2006) 'lower-quartile' method (`variant='LQ'`) algorithm is a type of 'valley recognition' method. Other channel mapping methods, such as the Johnston and Rosenfeld (1975) algorithm, experience problems because channel profiles are not always 'v'-shaped, nor are they always apparent in small 3 x 3 windows. The lower-quartile method was developed as an alternative and more flexible valley recognition channel mapping technique. The lower-quartile method operates by running a filter over the DEM that calculates the percentile value of the centre cell with respect to the distribution of elevations within the filter window. The roving window is circular, the diameter of which should reflect the topographic variation of the area (e.g. the channel width or average hillslope length). If this variant is selected, the user must specify the `filter_size` parameter, in pixels, and this value should be an odd number (e.g. 3, 5, 7, etc.). The appropriateness of the selected window diameter will depend on the grid resolution relative to the scale of topographic features. Cells that are within the lower quartile of the distribution of elevations of their neighbourhood are flagged. Thus, the algorithm identifies grid cells that are in relatively low topographic positions at a local scale. This approach to channel mapping is only appropriate in fluvial landscapes. In regions containing numerous lakes and wetlands, the algorithm will pick out the edges of features. 

The Johnston and Rosenfeld (1975) algorithm (`variant='JandR'`) is a type of 'valley recognition' method and operates as follows: channel cells are flagged in a 3 x 3 window if the north and south neighbours are higher than the centre grid cell or if the east and west neighbours meet this same criterion. The group of cells that are flagged after one pass of the roving window constituted the drainage network. This method is best applied to DEMs that are relatively smooth and do not exhibit high levels of short-range roughness. As such, it may be desirable to use a smoothing filter before applying this tool. The `feature_preserving_smoothing` is a good option for removing DEM roughness while preserving the topographic information contain in breaks-in-slope (i.e. edges). 

The Peucker and Douglas (1975) algorithm (`variant='PandD'`) is one of the simplest and earliest algorithms for topography-based network extraction. Their 'valley recognition' method operates by passing a 2 x 2 roving window over a DEM and flagging the highest grid cell in each group of four. Once the window has passed over the entire DEM, channel grid cells are left unflagged. This method is also best applied to DEMs that are relatively smooth and do not exhibit high levels of short-range roughness. Pre-processing the DEM with the `feature_preserving_smoothing` tool may also be useful when applying this method. 

Each of these methods of extracting valley networks result in line networks that can be wider than a single grid cell. As such, it is often desirable to thin the resulting network using a line-thinning algorithm. The option to perform line-thinning is provided by the tool as a post-processing step (`line_thin=True`). 

### References

 

Johnston, E. G., & Rosenfeld, A. (1975). Digital detection of pits, peaks, ridges, and ravines. IEEE Transactions on Systems, Man, and Cybernetics, (4), 472-480. 

Lindsay, J. B. (2006). Sensitivity of channel mapping techniques to uncertainty in digital elevation data. International Journal of Geographical Information Science, 20(6), 669-692. 

Peucker, T. K., & Douglas, D. H. (1975). Detection of surface-specific points by local parallel processing of discrete terrain elevation data. Computer Graphics and image processing, 4(4), 375-387. 

### See Also

 

`feature_preserving_smoothing` 

### Python API

```python
def extract_valleys(self, dem: Raster, variant: str = "lq", line_thin: bool = False, filter_size: int = 5) -> Raster:
```


---

## Prune Vector Streams

**Function name:** `prune_vector_streams`


### Description

 

This tool can be used to prune the smallest branches of a vector stream network based on a threshold in link magnitude. The function automatically calculates the Shreve magnitude of each link in the input streams vector. This operation requires an input digital elevation model (DEM). The function is also capable of calculating the link magnitude from stream networks that have some minor topological errors (e.g., line overshoots or undershoots). This requires the input of a `snap_distance` parameter (default = 0.0). 

### See Also

 

`vector_stream_network_analysis`, `repair_stream_vector_topology` 

### Python API

```python
def prune_vector_streams(self, streams: Vector, dem: Raster, threshold: float, snap_distance: float = 0.001) -> Vector:
```


---

## Raster Streams To Vector

**Function name:** `raster_streams_to_vector`


This tool converts a raster stream file into a vector file. The user must specify an input raster streams file (`streams`), and an input D8 flow pointer file (`d8_pointer`). Streams in the input raster streams file are denoted by cells containing any positive, non-zero integer. A field in the output vector's database file, called STRM_VAL, will correspond to this positive integer value. The database file will also have a field for the length of each link in the stream network. The flow pointer file must be calculated from a DEM with all topographic depressions and flat areas removed and must be calculated using the D8 flow pointer algorithm (`d8_pointer`). The output vector will contain PolyLine features. 

### See Also

 

`rasterize_streams`, `raster_to_vector_lines` 

### Python API

```python
def raster_streams_to_vector(self, streams: Raster, d8_pointer: Raster, esri_pointer: bool = False) -> Vector:
```


---

## Rasterize Streams

**Function name:** `rasterize_streams`


This tool can be used rasterize an input vector stream network (`streams`) using on Lindsay (2016) method. The user inputs an existing raster (`base_raster`), from which the output raster's grid resolution is determined. 

### Reference

 

Lindsay JB. 2016. The practice of DEM stream burning revisited. Earth Surface Processes and Landforms, 41(5): 658–668. DOI: 10.1002/esp.3888 

### See Also

 

`raster_streams_to_vector` 

### Python API

```python
def rasterize_streams(self, streams: Vector, base_raster: Raster = None, zero_background: bool = False, use_feature_id: bool = False) -> Raster:
```


---

## Remove Short Streams

**Function name:** `remove_short_streams`


This tool can be used to remove stream links in a stream network that are shorter than a user-specified length (`min_length`). The user must input a streams raster image (`streams_raster`) and D8 pointer (flow direction) image (`d8_pntr`). Stream cells are designated in the streams raster as all positive, nonzero values. Thus all non-stream or background grid cells are commonly assigned either zeros or NoData values. The pointer raster is used to traverse the stream network and should only be created using the D8 algorithm (`d8_pointer`). 

By default, the pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools. If the pointer file contains ESRI flow direction values instead, the user must specify `esri_pntr=True`. 

### See Also

 

`extract_streams`, `d8_pointer` 

### Python API

```python
def remove_short_streams(self, d8_pntr: Raster, streams_raster: Raster, min_length: float = 0.0, esri_pntr: bool = False) -> Raster:
```


---

## Repair Stream Vector Topology

**Function name:** `repair_stream_vector_topology`


This tool can be used to resolve many of the topological errors and inconsistencies associated with manually digitized vector stream networks, i.e. hydrography data. A properly structured stream network should consist of a series of stream segments that connect a channel head to a downstream confluence, or an upstream confluence to a downstream confluence/outlet. This tool will join vector arcs that connect at arbitrary, non-confluence points along stream segments. It also splits an arc where a tributary stream connects at a mid-point, thereby creating a proper confluence where two upstream triburaries converge into a downstream segment. The tool also handles non-connecting tributaries caused by dangling arcs, i.e. overshoots and undershoots. 

 

The user must specify the name of the input vector stream network (`input`) and the output file (`output`). Additionally, a distance threshold for snapping dangling arcs (`snap`) must be specified. This distance is in the input layer's x-y units. The tool works best on projected input data, however, if the input are in geographic coordinates (latitude and longitude), then specifying a small valued snap distance is advisable. Notice that the attributes of the input layer will not be carried over to the output file because there is not a one-for-one feature correspondence between the two files due to the joins and splits of stream segments. Instead the output attribute table will only contain a feature ID (FID) entry.  

Note: this tool should be used to pre-process vector streams that are input to the `vector_stream_network_analysis` tool.  

### See Also

 

`vector_stream_network_analysis`, `fix_dangling_arcs`


---

## River Centerlines

**Function name:** `river_centerlines`


Note this tool is part of a `WhiteboxTools extension product`. Please visit `Whitebox Geospatial Inc.` for information about purchasing a license activation key (`https://www.whiteboxgeo.com/extension-pricing/`).  

This tool can map river centerlines, or medial-lines, from input river rasters (`input`). The input river (or water) raster is often derived from an image classification performed on multispectral satellite imagery. The river raster must be a Boolean (1 for water, 0/NoData for not-water) and can be derived either by reclassifying the classification output, or derived using a 1-class classification procedure. For example, using the `parallelepiped_classification` tool, it is possible to train the classifier using water training polygons, and all other land classes will simply be left unclassified. It may be necessary to perform some pre-processing on the water Boolean raster before input to the centerlines tool. For example, you may need to remove smaller water polygons associated with small lakes and ponds, and you may want to remove small islands from the remaining water features. This tool will often create a bifurcating vector path around islands within rivers, even if those islands are a single-cell in size. The `RemoveRasterPolygonHoles` tool can be used to remove islands in the water raster that are smaller than a user-specified size. The user must also specify the minimum line length (`min_length`), which determines the level of detail in the final rivers map. For example, in the first iamge below, a value of 30 grid cells was used for the `min_length` parameter, while a value of 5 was used in the second image, which possesses far more (arguably too much) detail. 

 

 

Lastly, the user must specify the `radius` parameter value. At times, the tool will be able to connect distant water polygons that are part of the same feature and this parameter determines the size of the search radius used to identify separated line end-nodes that are candidates for connection. It is advisable that this value not be set too high, or else unexpected connections may be made between unrelated water features. However, a value of between 1 and 5 can produce satisfactory results. Experimentation may be needed to find an appropriate value for any one data set however. The image below provides an example of this characteristic of the tool, where the resulting vector stream centerline passes through disconnected raster water polygons in the underlying input image in four locations. 

 

`**Here**` is a video that demonstrates how to apply this tool to map river center-lines taken from a water raster created by classifying a Sentinel-2 multi-spectral satellite imagery data set. 

### See Also

 

`parallelepiped_classification`, `RemoveRasterPolygonHoles` 

### Python API

```python
def river_centerlines(self, raster: Raster, min_length: int = 3, search_radius: int = 9) -> Vector:
```
