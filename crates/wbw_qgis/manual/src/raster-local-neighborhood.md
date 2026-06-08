# Local and Neighborhood


---

## Image Correlation Neighbourhood Analysis

**Function name:** `image_correlation_neighbourhood_analysis`


This tool can be used to perform nieghbourhood-based (i.e. using roving search windows applied to each grid cell) correlation analysis on two input rasters (`input1` and `input2`). The tool outputs a correlation value raster (`output1`) and a significance (p-value) raster (`output2`). Additionally, the user must specify the size of the search window (`filter`) and the correlation statistic (`stat`). Options for the correlation statistic include ``pearson``, ``kendall``, and ``spearman``. Notice that Pearson's *r* is the most computationally efficient of the three correlation metrics but is unsuitable when the input distributions are non-linearly associated, in which case, either Spearman's Rho or Kendall's tau-b correlations are more suited. Both Spearman and Kendall correlations evaluate monotonic associations without assuming linearity in the relation. Kendall's tau-b is by far the most computationally expensive of the three statistics and may not be suitable to larger sized search windows. 

### See Also

 

`image_correlation`, `image_regression` 

### Python API

```python
def image_correlation_neighbourhood_analysis(self, raster1: Raster, raster2: Raster, filter_size: int = 11, correlation_stat: str = "pearson") -> Tuple[Raster, Raster]:
```


---

## Natural Neighbour Interpolation

**Function name:** `natural_neighbour_interpolation`


This tool can be used to interpolate a set of input vector points (`input`) onto a raster grid using Sibson's (1981) natural neighbour method. Similar to inverse-distance-weight interpolation (`idw_interpolation`), the natural neighbour method performs a weighted averaging of nearby point values to estimate the attribute (`field`) value at grid cell intersections in the output raster (`output`). However, the two methods differ quite significantly in the way that neighbours are identified and in the weighting scheme. First, natural neigbhour identifies neighbours to be used in the interpolation of a point by finding the points connected to the estimated value location in a `Delaunay triangulation`, that is, the so-called *natural neighbours*. This approach has the main advantage of not having to specify an arbitrary search distance or minimum number of nearest neighbours like many other interpolators do. Weights in the natural neighbour scheme are determined using an area-stealing approach, whereby the weight assigned to a neighbour's value is determined by the proportion of its `Voronoi polygon` that would be lost by inserting the interpolation point into the Voronoi diagram. That is, inserting the interpolation point into the Voronoi diagram results in the creation of a new polygon and shrinking the sizes of the Voronoi polygons associated with each of the natural neighbours. The larger the area by which a neighbours polygon is reduced through the insertion, relative to the polygon of the interpolation point, the greater the weight given to the neighbour point's value in the interpolation. Interpolation weights sum to one because the sum of the reduced polygon areas must account for the entire area of the interpolation points polygon. 

The user must specify the attribute field containing point values (`field`). Alternatively, if the input Shapefile contains z-values, the interpolation may be based on these values (`use_z`). Either an output grid resolution (`cell_size`) must be specified or alternatively an existing base file (`base`) can be used to determine the output raster's (`output`) resolution and spatial extent. Natural neighbour interpolation generally produces a satisfactorily smooth surface within the region of data points but can produce spurious breaks in the surface outside of this region. Thus, it is recommended that the output surface be clipped to the convex hull of the input points (`clip`). 

### Reference

 

Sibson, R. (1981). "A brief description of natural neighbor interpolation (Chapter 2)". In V. Barnett (ed.). Interpolating Multivariate Data. Chichester: John Wiley. pp. 21–36. 

### See Also

 

`idw_interpolation`, `NearestNeighbourGridding` 

### Python API

```python
def natural_neighbour_interpolation(self, points: Vector, field_name: str = "FID", use_z: bool = False, cell_size: float = 0.0, base_raster: Raster = None, clip_to_hull: bool = True) -> Raster:
```


---

## Nearest Neighbour Interpolation

**Function name:** `nearest_neighbour_interpolation`


Creates a raster grid based on a set of vector points and assigns grid values using the nearest neighbour. 

### Python API

```python
def nearest_neighbour_interpolation(self, points: Vector, field_name: str = "FID", use_z: bool = False, cell_size: float = 0.0, base_raster: Raster = None, max_dist: float = float('inf')) -> Raster:
```
