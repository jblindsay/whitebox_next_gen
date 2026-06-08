# Shape Metrics


---

## Compactness Ratio

**Function name:** `compactness_ratio`


The compactness ratio is an indicator of polygon shape complexity. The compactness ratio is defined as the polygon area divided by its perimeter. Unlike some other shape parameters (e.g. `ShapeComplexityIndex`), compactness ratio does not standardize to a simple Euclidean shape. Although widely used for landscape analysis, compactness ratio, like its inverse, the `perimeter_area_ratio`, exhibits the undesirable property of polygon size dependence (Mcgarigal et al. 2002). That is, holding shape constant, an increase in polygon size will cause a change in the compactness ratio. 

The output data will be contained in the input vector's attribute table as a new field (COMPACT). 

### See Also

 

`perimeter_area_ratio`, `ShapeComplexityIndex`, `related_circumscribing_circle` 

### Python API

```python
def compactness_ratio(self, input: Vector) -> Vector:
```


---

## Deviation From Regional Direction

**Function name:** `deviation_from_regional_direction`


This tool calculates the degree to which each polygon in an input shapefile (`input`) deviates from the average,  or regional, direction. The input file will have a new attribute inserted in the attribute table, `DEV_DIR`, which will contain the calculated values. The deviation values are in degrees. The orientation of each polygon is determined based on the long-axis of the minimum bounding box fitted to the polygon. The regional direction is based on the  mean direciton of the polygons, weighted by long-axis length (longer polygons contribute more weight) and elongation, i.e., a function of the long and short axis lengths (greater elongation contributes more weight). Polygons with  elongation values lower than the elongation threshold value (`elongation_threshold`), which has values between 0 and 1,  will be excluded from the calculation of the regional direction.  

### See Also

 

`patch_orientation`, `elongation_ratio` 

### Python API

```python
def deviation_from_regional_direction(self, input: Vector, elongation_threshold: float = 0.75) -> Vector:
```


---

## Elongation Ratio

**Function name:** `elongation_ratio`


This tool can be used to calculate the elongation ratio for vector polygons. The elongation ratio values calculated for each vector polygon feature will be placed in the accompanying database file (.dbf) as an elongation field (ELONGATION). 

The elongation ratio (`E`) is:  

E = 1 - S / L  

Where `S` is the short-axis length, and `L` is the long-axis length. Axes lengths are determined by estimating the minimum bounding box. 

The elongation ratio provides similar information as the Linearity Index. The ratio is not an adequate measure of overall polygon narrowness, because a highly sinuous but narrow polygon will have a low linearity (elongation) owing to the compact nature of these polygon. 

### Python API

```python
def elongation_ratio(self, input: Vector) -> Vector:
```


---

## Hole Proportion

**Function name:** `hole_proportion`


This calculates the proportion of the total area of a polygon's holes (i.e. islands) relative to the area of the polygon's hull. It can be a useful measure of shape complexity, or how discontinuous a patch is. The user must specify the name of the input vector file and the output data will be contained within the input vector's database file as a new field (HOLE_PROP). 

### See Also

 

`ShapeComplexityIndex`, `elongation_ratio`, `perimeter_area_ratio` 

### Python API

```python
def hole_proportion(self, input: Vector) -> Vector:
```


---

## Linearity Index

**Function name:** `linearity_index`


This tool calculates the linearity index of polygon features based on a regression analysis. The index is simply the coefficient of determination (r-squared) calculated from a regression analysis of the x and y coordinates of the exterior hull nodes of a vector polygon. Linearity index is a measure of how well a polygon can be described by a straight line. It is a related index to the `elongation_ratio`, but is more efficient to calculate as it does not require finding the minimum bounding box. The Pearson correlation coefficient between linearity index and the elongation ratio for a large data set of lake polygons in northern Canada was found to be 0.656, suggesting a moderate level of association between the two measures of polygon linearity. Note that this index is not useful for identifying narrow yet sinuous polygons, such as meandering rivers. 

The only required input is the name of the file. The linearity values calculated for each vector polygon feature will be placed in the accompanying attribute table as a new field (LINEARITY). 

### See Also

 

`elongation_ratio`, `patch_orientation` 

### Python API

```python
def linearity_index(self, input: Vector) -> Vector:
```


---

## Narrowness Index Vector

**Function name:** `narrowness_index_vector`


*No help documentation available for this tool.*


---

## Patch Orientation

**Function name:** `patch_orientation`


This tool calculates the orientation of polygon features based on the slope of a reduced major axis (RMA) regression line. The regression analysis use the vertices of the exterior hull nodes of a vector polygon. The only required input is the name of the vector polygon file. The orientation values, measured in degrees from north, will be placed in the accompanying attribute table as a new field (ORIENT). The value of the orientation measure for any polygon will depend on how elongated the feature is. 

Note that the output values are polygon orientations and not true directions. While directions may take values ranging from 0-360, orientation is expressed as an angle between 0 and 180 degrees clockwise from north. Lastly, the orientation measure may become unstable when polygons are oriented nearly vertical or horizontal. 

### See Also

 

`linearity_index`, `elongation_ratio` 

### Python API

```python
def patch_orientation(self, input: Vector) -> Vector:
```


---

## Perimeter Area Ratio

**Function name:** `perimeter_area_ratio`


The perimeter-area ratio is an indicator of polygon shape complexity. Unlike some other shape parameters (e.g. shape complexity index), perimeter-area ratio does not standardize to a simple Euclidean shape. Although widely used for landscape analysis, perimeter-area ratio exhibits the undesirable property of polygon size dependence (Mcgarigal et al. 2002). That is, holding shape constant, an increase in polygon size will cause a decrease in the perimeter-area ratio. The perimeter-area ratio is the inverse of the compactness ratio. 

The output data will be displayed as a new field (P_A_RATIO) in the input vector's database file. 

### Python API

```python
def perimeter_area_ratio(self, input: Vector) -> Vector:
```


---

## Polygon Area

**Function name:** `polygon_area`


This tool calculates the area of vector polygons, adding the result to the vector's attribute table (AREA field). The area calculation will account for any holes contained within polygons. The vector should be in a projected coordinate system. 

To calculate the area of raster polygons, use the `raster_area` tool instead. 

### See Also

 

`raster_area` 

### Python API

```python
def polygon_area(self, input: Vector) -> Vector:
```


---

## Polygon Long Axis

**Function name:** `polygon_long_axis`


This tool can be used to map the long axis of polygon features. The long axis is the longer of the two primary axes of the minimum bounding box (MBB), i.e. the smallest box to completely enclose a feature. The long axis is drawn for each polygon in the input vector file such that it passes through the centre point of the MBB. The output file is therefore a vector of simple two-point polylines forming a vector field. 

### Python API

```python
def polygon_long_axis(self, input: Vector) -> Vector:
```


---

## Polygon Perimeter

**Function name:** `polygon_perimeter`


This tool calculates the perimeter of vector polygons, adding the result to the vector's attribute table (PERIMETER field). The area calculation will account for any holes contained within polygons. The vector should be in a a projected coordinate system. 

### Python API

```python
def polygon_perimeter(self, input: Vector) -> Vector:
```


---

## Polygon Short Axis

**Function name:** `polygon_short_axis`


This tool can be used to map the short axis of polygon features. The short axis is the shorter of the two primary axes of the minimum bounding box (MBB), i.e. the smallest box to completely enclose a feature. The short axis is drawn for each polygon in the input vector file such that it passes through the centre point of the MBB. The output file is therefore a vector of simple two-point polylines forming a vector field. 

### Python API

```python
def polygon_short_axis(self, input: Vector) -> Vector:
```


---

## Related Circumscribing Circle

**Function name:** `related_circumscribing_circle`


This tool can be used to calculate the related circumscribing circle (Mcgarigal et al. 2002) for vector polygon features. The related circumscribing circle values calculated for each vector polygon feature will be placed in the accompanying attribute table as a new field (RC_CIRCLE). 

Related circumscribing circle (RCC) is defined as:    

RCC = 1 - A / Ac  

Where `A` is the polygon's area and `Ac` the area of the smallest circumscribing circle. 

Theoretically, `related_circumscribing_circle` ranges from 0 to 1, where a value of 0 indicates a circular polygon and a value of 1 indicates a highly elongated shape. The circumscribing circle provides a measure of polygon elongation. Unlike the `elongation_ratio`, however, it does not provide a measure of polygon direction in addition to overall elongation. Like the `elongation_ratio` and `linearity_index`, `related_circumscribing_circle` is not an adequate measure of overall polygon narrowness, because a highly sinuous but narrow patch will have a low related circumscribing circle index owing to the compact nature of these polygon. 

Note: Holes are excluded from the area calculation of polygons. 

### Python API

```python
def related_circumscribing_circle(self, input: Vector) -> Vector:
```


---

## Shape Complexity Index Vector

**Function name:** `shape_complexity_index_vector`


This tool provides a measure of overall polygon shape complexity, or irregularity, for vector polygons. Several shape indices have been created to compare a polygon's shape to simple Euclidean shapes (e.g. circles, squares, etc.). One of the problems with this approach is that it inherently convolves the characteristics of polygon complexity and elongation. The Shape Complexity Index (SCI) was developed as a parameter for assessing the complexity of a polygon that is independent of its elongation. 

SCI relates a polygon's shape to that of an encompassing convex hull. It is defined as:    

SCI = 1 - A / Ah  

Where `A` is the polygon's area and `Ah` is the area of the convex hull containing the polygon. Convex polygons, i.e. those that do not contain concavities or holes, have a value of 0. As the shape of the polygon becomes more complex, the SCI approaches 1. Note that polygon shape complexity also increases with the greater number of holes (i.e. islands), since holes have the effect of reducing the lake area. 

The SCI values calculated for each vector polygon feature will be placed in the accompanying database file (.dbf) as a complexity field (COMPLEXITY). 

### See Also

 

`shape_complexity_index_raster` 

### Python API

```python
def shape_complexity_index_vector(self, input: Vector) -> Vector:
```
