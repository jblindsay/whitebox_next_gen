# Distance and Cost


---

## Buffer Raster

**Function name:** `buffer_raster`


This tool can be used to identify an area of interest within a specified distance of features of interest in a raster data set. 

The Euclidean distance (i.e. straight-line distance) is calculated between each grid cell and the nearest 'target cell' in the input image. Distance is calculated using the efficient method of Shih and Wu (2004). Target cells are all non-zero, non-NoData grid cells. Because NoData values in the input image are assigned the NoData value in the output image, the only valid background value in the input image is zero. 

The user must specify the input and output image names, the desired buffer size (`size`), and, optionally, whether the distance units are measured in grid cells (i.e. `gridcells` flag). If the `gridcells` flag is not specified, the linear units of the raster's coordinate reference system will be used. 

### Reference

 

Shih FY and Wu Y-T (2004), Fast Euclidean distance transformation in two scans using a 3 x 3 neighborhood, *Computer Vision and Image Understanding*, 93: 195-205. 

### See Also

 

`euclidean_distance` 

### Python API

```python
def buffer_raster(self, input: Raster, buffer_size: float, grid_cells_units: bool = False) -> Raster:
```


---

## Cost Allocation

**Function name:** `cost_allocation`


This tool can be used to identify the 'catchment area' of each source grid cell in a cost-distance analysis. The user must specify the names of the input *source* and *back-link* raster files. Source cells (i.e. starting points for the cost-distance or least-cost path analysis) are designated as all positive, non-zero valued grid cells in the *source* raster. A *back-link* raster file can be created using the `cost_distance` tool and is conceptually similar to the D8 flow-direction pointer raster grid in that it describes the connectivity between neighbouring cells on the accumulated cost surface. 

NoData values in the input *back-link* image are assigned NoData values in the output image. 

### See Also

 

`cost_distance`, `cost_pathway`, `euclidean_allocation` 

### Python API

```python
def cost_allocation(self, source: Raster, backlink: Raster) -> Raster:
```


---

## Cost Distance

**Function name:** `cost_distance`


This tool can be used to perform cost-distance or least-cost pathway analyses. Specifically, this tool can be used to calculate the accumulated cost of traveling from the 'source grid cell' to each other grid cell in a raster dataset. It is based on the costs associated with traveling through each cell along a pathway represented in a cost (or friction) surface. If there are multiple source grid cells, each cell in the resulting cost-accumulation surface will reflect the accumulated cost to the source cell that is connected by the minimum accumulated cost-path. The user must specify the names of the raster file containing the source cells (`source`), the raster file containing the cost surface information (`cost`), the output cost-accumulation surface raster (`out_accum`), and the output back-link raster (`out_backlink`). Source cells are designated as all positive, non-zero valued grid cells in the source raster. The cost (friction) raster can be created by combining the various cost factors associated with the specific problem (e.g. slope gradient, visibility, etc.) using a raster calculator or the `weighted_overlay` tool. 

While the cost-accumulation surface raster can be helpful for visualizing the three-dimensional characteristics of the 'cost landscape', it is actually the back-link raster that is used as inputs to the other two cost-distance tools, `cost_allocation` and `cost_pathway`, to determine the least-cost linkages among neighbouring grid cells on the cost surface. If the accumulated cost surface is analogous to a digital elevation model (DEM) then the back-link raster is equivalent to the D8 flow-direction pointer. In fact, it is created in a similar way and uses the same convention for designating 'flow directions' between neighbouring grid cells. The algorithm for the cost distance accumulation operation uses a type of priority-flood method similar to what is used for depression filling and flow accumulation operations. 

NoData values in the input cost surface image are ignored during processing and assigned NoData values in the outputs. The output cost accumulation raster is of the float data type and continuous data scale. 

### See Also

 

`cost_allocation`, `cost_pathway`, `weighted_overlay` 

### Python API

```python
def cost_distance(self, source: Raster, cost: Raster) -> Tuple[Raster, Raster]:
```


---

## Cost Pathway

**Function name:** `cost_pathway`


This tool can be used to map the least-cost pathway connecting each destination grid cell in a cost-distance analysis to a source cell. The user must specify the names of the input *destination* and *back-link* raster files. Destination cells (i.e. end points for the least-cost path analysis) are designated as all positive, non-zero valued grid cells in the *destination* raster. A *back-link* raster file can be created using the `cost_distance` tool and is conceptually similar to the D8 flow-direction pointer raster grid in that it describes the connectivity between neighbouring cells on the accumulated cost surface. All background grid cells in the output image are assigned the NoData value. 

NoData values in the input *back-link* image are assigned NoData values in the output image. 

### See Also

 

`cost_distance`, `cost_allocation` 

### Python API

```python
def cost_pathway(self, destination: Raster, backlink: Raster, zero_background: bool = False) -> Raster:
```


---

## Euclidean Allocation

**Function name:** `euclidean_allocation`


This tool assigns grid cells in the output image the value of the nearest target cell in the input image, measured by the Euclidean distance (i.e. straight-line distance). Thus, `euclidean_allocation` essentially creates the Voronoi diagram for a set of target cells. Target cells are all non-zero, non-NoData grid cells in the input image. Distances are calculated using the same efficient algorithm (Shih and Wu, 2003) as the `euclidean_distance` tool. 

### Reference

 

Shih FY and Wu Y-T (2004), Fast Euclidean distance transformation in two scans using a 3 x 3 neighborhood, *Computer Vision and Image Understanding*, 93: 195-205. 

### See Also

 

`euclidean_distance`, `voronoi_diagram`, `cost_allocation` 

### Python API

```python
def euclidean_allocation(self, input: Raster) -> Raster:
```


---

## Euclidean Distance

**Function name:** `euclidean_distance`


This tool will estimate the Euclidean distance (i.e. straight-line distance) between each grid cell and the nearest 'target cell' in the input image. Target cells are all non-zero, non-NoData grid cells. Distance in the output image is measured in the same units as the horizontal units of the input image. 

### Algorithm Description

 

The algorithm is based on the highly efficient distance transform of Shih and Wu (2003). It makes four passes of the image; the first pass initializes the output image; the second and third passes calculate the minimum squared Euclidean distance by examining the 3 x 3 neighbourhood surrounding each cell; the last pass takes the square root of cell values, transforming them into true Euclidean distances, and deals with NoData values that may be present. All NoData value grid cells in the input image will contain NoData values in the output image. As such, NoData is not a suitable background value for non-target cells. Background areas should be designated with zero values. 

### Reference

 

Shih FY and Wu Y-T (2004), Fast Euclidean distance transformation in two scans using a 3 x 3 neighborhood, *Computer Vision and Image Understanding*, 93: 195-205. 

### See Also

 

`euclidean_allocation`, `cost_distance` 

### Python API

```python
def euclidean_distance(self, input: Raster) -> Raster:
```
