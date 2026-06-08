# Longitudinal Profile Analysis


---

## Farthest Channel Head

**Function name:** `farthest_channel_head`


### Description

 

This tool calculates the upstream distance to the farthest stream head for each grid cell belonging to a raster stream network. The user must input a raster containing streams data (`streams`), where stream grid cells are denoted by all positive non-zero values, and a D8 flow pointer (i.e. flow direction) raster (`d8_pointer`). The pointer image is used to traverse the stream network and must only be created using the D8 algorithm. Stream cells are designated in the streams image as all values greater than zero. Thus, all non-stream or background grid cells are commonly assigned either zeros or NoData values. Background cells will be assigned the NoData value in the output image, unless `zero_background=True`, in which case non-stream cells will be assigned zero values in the output. 

By default, the pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools. If the pointer file contains ESRI flow direction values instead, the user should specify `esri_pntr=True`. 

### See Also

 

`length_of_upstream_channels`, `find_main_stem` 

### Parameters

 

d8_pointer (Raster):     The D8 pointer (flow direction) raster. 

streams_raster (Raster):     The raster object containing the streams data. 

esri_pointer (bool):     Determines whether the d8_pointer raster contains pointer data in the Esri format. Default is False. 

zero_background (bool):     Determines whether the background value in the output raster are assigned zero (True) or NoData values (False). Default is False. 

### Returns

 

Raster: returning value 

### Python API

```python
def farthest_channel_head(self, d8_pointer: Raster, streams_raster: Raster, esri_pointer: bool = False, zero_background: bool = False) -> Raster:
```


---

## Find Main Stem

**Function name:** `find_main_stem`


This tool can be used to identify the main channel in a stream network. The user must input a D8 pointer (flow direction) raster (`d8_pointer`), and a streams raster (`streams_raster`). The pointer raster is used to traverse the stream network and should only be created using the `d8_pointer` tool. By default, the pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools:  ... 641281 3202 1684   

If the pointer file contains ESRI flow direction values instead, you must set `esri_pointer=True` parameter must be specified. 

The streams raster should have been created using one of the DEM-based stream mapping methods, i.e. contributing area thresholding. Stream grid cells are designated in the streams image as all positive, non-zero values. All non-stream cells will be assigned the NoData value in the output image, unless the user sets `zero_background=True`. 

The algorithm operates by traversing each stream and identifying the longest stream-path draining to each outlet. When a confluence is encountered, the traverse follows the branch with the larger distance-to-head. 

### See Also

 

`d8_pointer` 

### Python API

```python
def find_main_stem(self, d8_pointer: Raster, streams_raster: Raster, esri_pointer: bool = False, zero_background: bool = False) -> Raster:
```


---

## Length Of Upstream Channels

**Function name:** `length_of_upstream_channels`


This tool calculates, for each stream grid cell in an input streams raster (`streams_raster`) the total length of channels upstream. The user must specify the name of a raster containing streams data (`streams_raster`), where stream grid cells are denoted by all positive non-zero values, and a D8 flow pointer (i.e. flow direction) raster (`d8_pointer`). The pointer image is used to traverse the stream network and must only be created using the D8 algorithm. Stream cells are designated in the streams image as all values greater than zero. Thus, all non-stream or background grid cells are commonly assigned either zeros or NoData values. Background cells will be assigned the NoData value in the output image, unless the user specifies `zero_background=True`, in which case non-stream cells will be assigned zero values in the output. 

By default, the pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools. If the pointer file contains ESRI flow direction values instead, set `esri_pntr=True`. 

### See Also

 

`farthest_channel_head`, `find_main_stem` 

### Python API

```python
def length_of_upstream_channels(self, d8_pointer: Raster, streams_raster: Raster, esri_pointer: bool = False, zero_background: bool = False) -> Raster:
```


---

## Long Profile

**Function name:** `long_profile`


This tool can be used to create a  `longitudinal profile` plot. A longitudinal stream profile is a plot of elevation against downstream distance. Most long profiles use distance from channel head as the distance measure. This tool, however, uses the distance to the stream network outlet cell, or mouth, as the distance measure. The reason for this difference is that while for any one location within a stream network there is only ever one downstream outlet, there are usually many upstream channel heads. Thus plotted using the traditional downstream-distance method, the same point within a network will plot in many different long profile locations, whereas it will always plot on one unique location in the distance-to-mouth method. One consequence of this difference is that the long profile will be oriented from right-to-left rather than left-to-right, as would traditionally be the case. 

The tool outputs an interactive SVG line graph embedded in an HTML document (`output_html_file`). The user must input a D8 pointer (flow direction) raster (`d8_pointer`), a streams raster image (`streams_raster`), and a digital elevation model (`dem`). Stream cells are designated in the streams image as all positive, nonzero values. Thus all non-stream or background grid cells are commonly assigned either zeros or NoData values. The pointer image is used to traverse the stream network and should only be created using the D8 algorithm (`d8_pointer`). The streams image should be derived using a flow accumulation based stream network extraction algorithm, also based on the D8 flow algorithm. 

By default, the pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools. If the pointer file contains ESRI flow direction values instead, set `esri_pointer=True`. 

### See Also

 

`long_profile_from_points`, `profile`, `d8_pointer` 

### Python API

```python
def long_profile(self, d8_pointer: Raster, streams_raster: Raster, dem: Raster, output_html_file: str, esri_pointer: bool = False) -> None:
```


---

## Long Profile From Points

**Function name:** `long_profile_from_points`


This tool can be used to create a  `longitudinal profile` plot for a set of vector points (`points`). A longitudinal stream profile is a plot of elevation against downstream distance. Most long profiles use distance from channel head as the distance measure. This tool, however, uses the distance to the outlet cell, or mouth, as the distance measure. 

The tool outputs an interactive SVG line graph embedded in an HTML document (`output_html_file`). The user input a D8 pointer (`d8_pointer`) image (flow direction), a vector points file (`points`), and a digital elevation model (`dem`). The pointer image is used to traverse the flow path issuing from each initiation point in the vector file; this pointer file should only be created using the D8 algorithm (`d8_pointer`). 

By default, the pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools. If the pointer file contains ESRI flow direction values instead, the `esri_pointer` parameter must be specified. 

### See Also

 

`long_profile`, `profile`, `d8_pointer` 

### Python API

```python
def long_profile_from_points(self, d8_pointer: Raster, points: Vector, dem: Raster, output_html_file: str, esri_pointer: bool = False) -> None:
```
