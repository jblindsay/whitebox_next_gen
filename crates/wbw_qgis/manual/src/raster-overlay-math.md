# Overlay and Math


---

## Add

**Function name:** `add`


Experimental

Adds two rasters on a cell-by-cell basis.

raster math add legacy-port

### Parameters

NameDescriptionRequiredDefault
`input1`First input raster (path string or typed raster object).Required`input1.tif`
`input2`Second input raster (path string or typed raster object).Required`input2.tif`
`output`Optional output raster file path. If omitted, output remains in memory and is returned as a memory:// raster handle.Optional—

### Examples

*Runs add on two DEM rasters and writes the result to dem_sum.tif.*
`wbe.add(input1='dem_a.tif', input2='dem_b.tif', output='dem_sum.tif')`


---

## Average Overlay

**Function name:** `average_overlay`


This tool can be used to find the average value in each cell of a grid from a set of input images (`inputs`). It is therefore similar to the `weighted_sum` tool except that each input image is given equal weighting. This tool operates on a cell-by-cell basis. Therefore, each of the input rasters must share the same number of rows and columns and spatial extent. An error will be issued if this is not the case. At least two input rasters are required to run this tool. Like each of the WhiteboxTools overlay tools, this tool has been optimized for parallel processing. 

### See Also

 

`weighted_sum` 

### Python API

```python
def average_overlay(self, input_rasters: List[Raster]) -> Raster:
```


---

## Bool And

**Function name:** `bool_and`


This tool is a Boolean **AND** operator, i.e. it works on *True* or *False* (1 and 0) values. Grid cells for which the first and second input rasters (`input1`; `input2`) have *True* values are assigned 1 in the output raster, otherwise grid cells are assigned a value of 0. All non-zero values in the input rasters are considered to be *True*, while all zero-valued grid cells are considered to be *False*. Grid cells containing **NoData** values in either of the input rasters will be assigned a **NoData** value in the output raster. 

### See Also

 

`bool_not`, `bool_or`, `bool_xor` 

### Python API

```python
def bool_and(self, input1: Raster, input2: Raster) -> Raster:
```


---

## Bool Not

**Function name:** `bool_not`


This tool is a Boolean **NOT** operator, i.e. it works on *True* or *False* (1 and 0) values. Grid cells for which the first input raster (`input1`) has a *True* value and the second raster (`input2`) has a *False* value are assigned 0 in the output raster, otherwise grid cells are assigned a value of 0. All non-zero values in the input rasters are considered to be *True*, while all zero-valued grid cells are considered to be *False*. Grid cells containing **NoData** values in either of the input rasters will be assigned a **NoData** value in the output raster. Notice that the **Not** operator is asymmetrical, and the order of inputs matters. 

### See Also

 

`bool_and`, `bool_or`, `bool_xor` 

### Python API

```python
def bool_not(self, input1: Raster, input2: Raster) -> Raster:
```


---

## Bool Or

**Function name:** `bool_or`


This tool is a Boolean **OR** operator, i.e. it works on *True* or *False* (1 and 0) values. Grid cells for which the either the first or second input rasters (`input1`; `input2`) have a *True* value are assigned 1 in the output raster, otherwise grid cells are assigned a value of 0. All non-zero values in the input rasters are considered to be *True*, while all zero-valued grid cells are considered to be *False*. Grid cells containing **NoData** values in either of the input rasters will be assigned a **NoData** value in the output raster. 

### See Also

 

`bool_and`, `bool_not`, `bool_xor` 

### Python API

```python
def bool_or(self, input1: Raster, input2: Raster) -> Raster:
```


---

## Bool Xor

**Function name:** `bool_xor`


This tool is a Boolean **XOR** operator, i.e. it works on *True* or *False* (1 and 0) values. Grid cells for which either the first or second input rasters (`input1`; `input2`) have a *True* value but not both are assigned 1 in the output raster, otherwise grid cells are assigned a value of 0. All non-zero values in the input rasters are considered to be *True*, while all zero-valued grid cells are considered to be *False*. Grid cells containing **NoData** values in either of the input rasters will be assigned a **NoData** value in the output raster. Notice that the **Not** operator is asymmetrical, and the order of inputs matters. 

### See Also

 

`bool_and`, `bool_not`, `bool_or` 

### Python API

```python
def bool_xor(self, input1: Raster, input2: Raster) -> Raster:
```


---

## Count If

**Function name:** `count_if`


This tool counts the number of occurrences of a specified value (`value`) in a stack of input rasters (`inputs`). Each grid cell in the output raster (`output`) will contain the number of occurrences of the specified value in the stack of corresponding cells in the input image. At least two input rasters are required to run this tool. Each of the input rasters must share the same number of rows and columns and spatial extent. An error will be issued if this is not the case. 

### See Also

 

`pick_from_list` 

### Python API

```python
def count_if(self, input_rasters: List[Raster], comparison_value: float) -> Raster:
```


---

## Divide

**Function name:** `divide`


Experimental

Divides the first raster by the second on a cell-by-cell basis.

raster math divide legacy-port

### Parameters

NameDescriptionRequiredDefault
`input1`First input raster (path string or typed raster object).Required`input1.tif`
`input2`Second input raster (path string or typed raster object).Required`input2.tif`
`output`Optional output raster file path. If omitted, output remains in memory and is returned as a memory:// raster handle.Optional—

### Examples

*Runs divide on two DEM rasters and writes the result to dem_ratio.tif.*
`wbe.divide(input1='dem_a.tif', input2='dem_b.tif', output='dem_ratio.tif')`


---

## Highest Position

**Function name:** `highest_position`


This tool identifies the stack position (index) of the maximum value within a raster stack on a cell-by-cell basis. For example, if five raster images (`inputs`) are input to the tool, the output raster (`output`) would show which of the five input rasters contained the highest value for each grid cell. The index value in the output raster is the zero-order number of the raster stack, i.e. if the highest value in the stack is contained in the first image, the output value would be 0; if the highest stack value were the second image, the output value would be 1, and so on. If any of the cell values within the stack is NoData, the output raster will contain the NoData value for the corresponding grid cell. The index value is related to the order of the input images. 

### Warning

 

Each of the input rasters must have the same spatial extent and number of rows and columns. 

### See Also

 

`lowest_position`, `pick_from_list` 

### Python API

```python
def highest_position(self, input_rasters: List[Raster]) -> Raster:
```


---

## Lowest Position

**Function name:** `lowest_position`


This tool identifies the stack position (index) of the minimum value within a raster stack on a cell-by-cell basis. For example, if five raster images (`inputs`) are input to the tool, the output raster (`output`) would show which of the five input rasters contained the lowest value for each grid cell. The index value in the output raster is the zero-order number of the raster stack, i.e. if the lowest value in the stack is contained in the first image, the output value would be 0; if the lowest stack value were the second image, the output value would be 1, and so on. If any of the cell values within the stack is NoData, the output raster will contain the NoData value for the corresponding grid cell. The index value is related to the order of the input images. 

### Warning

 

Each of the input rasters must have the same spatial extent and number of rows and columns. 

### See Also

 

`highest_position`, `pick_from_list` 

### Python API

```python
def lowest_position(self, input_rasters: List[Raster]) -> Raster:
```


---

## Max Absolute Overlay

**Function name:** `max_absolute_overlay`


This tool can be used to find the maximum absolute (non-negative) value in each cell of a grid from a set of input images (`inputs`). NoData values in any of the input images will result in a NoData pixel in the output image. 

### Warning

 

Each of the input rasters must have the same spatial extent and number of rows and columns. 

### See Also

 

`max_overlay`, `min_absolute_overlay`, `min_overlay` 

### Python API

```python
def max_absolute_overlay(self, input_rasters: List[Raster]) -> Raster:
```


---

## Max Overlay

**Function name:** `max_overlay`


This tool can be used to find the maximum value in each cell of a grid from a set of input images (`inputs`). NoData values in any of the input images will result in a NoData pixel in the output image (`output`). It is similar to the `Max` mathematical tool, except that it will accept more than two input images. 

### Warning

 

Each of the input rasters must have the same spatial extent and number of rows and columns. 

### See Also

 

`min_overlay`, `max_absolute_overlay` 

### Python API

```python
def max_overlay(self, input_rasters: List[Raster]) -> Raster:
```


---

## Min Absolute Overlay

**Function name:** `min_absolute_overlay`


This tool can be used to find the minimum absolute (non-negative) value in each cell of a grid from a set of input images (`inputs`). NoData values in any of the input images will result in a NoData pixel in the output image. 

### Warning

 

Each of the input rasters must have the same spatial extent and number of rows and columns. 

### See Also

 

`min_overlay`, `max_absolute_overlay`, `max_overlay` 

### Python API

```python
def min_absolute_overlay(self, input_rasters: List[Raster]) -> Raster:
```


---

## Min Overlay

**Function name:** `min_overlay`


This tool can be used to find the minimum value in each cell of a grid from a set of input images (`inputs`). NoData values in any of the input images will result in a NoData pixel in the output image (`output`). It is similar to the `Min` mathematical tool, except that it will accept more than two input images. 

### Warning

 

Each of the input rasters must have the same spatial extent and number of rows and columns. 

### See Also

 

`max_overlay`, `max_absolute_overlay`, `min_absolute_overlay`, `Min` 

### Python API

```python
def min_overlay(self, input_rasters: List[Raster]) -> Raster:
```


---

## Modulo

**Function name:** `modulo`


Experimental

Computes the remainder of dividing the first raster by the second on a cell-by-cell basis.

raster math modulo legacy-port

### Parameters

NameDescriptionRequiredDefault
`input1`First input raster (path string or typed raster object).Required`input1.tif`
`input2`Second input raster (path string or typed raster object).Required`input2.tif`
`output`Optional output raster file path. If omitted, output remains in memory and is returned as a memory:// raster handle.Optional—

### Examples

*Runs modulo on two DEM rasters and writes the result to dem_modulo.tif.*
`wbe.modulo(input1='dem_a.tif', input2='dem_b.tif', output='dem_modulo.tif')`


---

## Multiply

**Function name:** `multiply`


Experimental

Multiplies two rasters on a cell-by-cell basis.

raster math multiply legacy-port

### Parameters

NameDescriptionRequiredDefault
`input1`First input raster (path string or typed raster object).Required`input1.tif`
`input2`Second input raster (path string or typed raster object).Required`input2.tif`
`output`Optional output raster file path. If omitted, output remains in memory and is returned as a memory:// raster handle.Optional—

### Examples

*Runs multiply on two DEM rasters and writes the result to dem_product.tif.*
`wbe.multiply(input1='dem_a.tif', input2='dem_b.tif', output='dem_product.tif')`


---

## Multiply Overlay

**Function name:** `multiply_overlay`


This tool multiplies a stack of raster images (`inputs`) on a pixel-by-pixel basis. This tool is particularly well suited when you need to create a masking layer from the combination of several Boolean rasters, i.e. for constraint mapping applications. NoData values in any of the input images will result in a NoData pixel in the output image (`output`). 

### Warning

 

Each of the input rasters must have the same spatial extent and number of rows and columns. 

### See Also

 

`sum_overlay`, `weighted_sum` 

### Python API

```python
def multiply_overlay(self, input_rasters: List[Raster]) -> Raster:
```


---

## Percent Equal To

**Function name:** `percent_equal_to`


This tool calculates the percentage of a raster stack (`inputs`) that have cell values equal to an input *comparison* raster. The user must specify the name of the value raster (`comparison`), the names of the raster files contained in the stack, and an output raster file name (`output`). The tool, working on a cell-by-cell basis, will count the number of rasters within the stack that have the same grid cell value as the corresponding grid cell in the *comparison* raster. This count is then expressed as a percentage of the number of rasters contained within the stack and output. If any of the rasters within the stack contain the NoData value, the corresponding grid cell in the output raster will be assigned NoData. 

### Warning

 

Each of the input rasters must have the same spatial extent and number of rows and columns. 

### See Also

 

`percent_greater_than`, `percent_less_than` 

### Python API

```python
def percent_equal_to(self, input_rasters: List[Raster], comparison: Raster) -> Raster:
```


---

## Percent Greater Than

**Function name:** `percent_greater_than`


This tool calculates the percentage of a raster stack (`inputs`) that have cell values greater than an input *comparison* raster. The user must specify the name of the value raster (`comparison`), the names of the raster files contained in the stack, and an output raster file name (`output`). The tool, working on a cell-by-cell basis, will count the number of rasters within the stack with larger grid cell values greater than the corresponding grid cell in the *comparison* raster. This count is then expressed as a percentage of the number of rasters contained within the stack and output. If any of the rasters within the stack contain the NoData value, the corresponding grid cell in the output raster will be assigned NoData. 

### Warning

 

Each of the input rasters must have the same spatial extent and number of rows and columns. 

### See Also

 

`percent_less_than`, `percent_equal_to` 

### Python API

```python
def percent_greater_than(self, input_rasters: List[Raster], comparison: Raster) -> Raster:
```


---

## Percent Less Than

**Function name:** `percent_less_than`


This tool calculates the percentage of a raster stack (`inputs`) that have cell values less than an input *comparison* raster. The user must specify the name of the value raster (`comparison`), the names of the raster files contained in the stack, and an output raster file name (`output`). The tool, working on a cell-by-cell basis, will count the number of rasters within the stack with larger grid cell values less than the corresponding grid cell in the *comparison* raster. This count is then expressed as a percentage of the number of rasters contained within the stack and output. If any of the rasters within the stack contain the NoData value, the corresponding grid cell in the output raster will be assigned NoData. 

### Warning

 

Each of the input rasters must have the same spatial extent and number of rows and columns. 

### See Also

 

`percent_greater_than`, `percent_equal_to` 

### Python API

```python
def percent_less_than(self, input_rasters: List[Raster], comparison: Raster) -> Raster:
```


---

## Pick From List

**Function name:** `pick_from_list`


This tool outputs the cell value from a raster stack specified (`inputs`) by a position raster (`pos_input`). The user must specify the name of the position raster, the names of the raster files contained in the stack (i.e. group of rasters), and an output raster file name (`output`). The tool, working on a cell-by-cell basis, will assign the value to the output grid cell contained in the corresponding cell in the stack image in the position specified by the cell value in the position raster. Importantly, the positions raster should be in zero-based order. That is, the first image in the stack should be assigned the value zero, the second raster is assigned 1, and so on. 

At least two input rasters are required to run this tool. Each of the input rasters must share the same number of rows and columns and spatial extent. An error will be issued if this is not the case. 

### See Also

 

`count_if` 

### Python API

```python
def pick_from_list(self, input_rasters: List[Raster], pos_input: Raster) -> Raster:
```


---

## Power

**Function name:** `power`


Experimental

Raises the first raster to the power of the second on a cell-by-cell basis.

raster math power legacy-port

### Parameters

NameDescriptionRequiredDefault
`input1`First input raster (path string or typed raster object).Required`input1.tif`
`input2`Second input raster (path string or typed raster object).Required`input2.tif`
`output`Optional output raster file path. If omitted, output remains in memory and is returned as a memory:// raster handle.Optional—

### Examples

*Runs power on two DEM rasters and writes the result to dem_power.tif.*
`wbe.power(input1='dem_a.tif', input2='dem_b.tif', output='dem_power.tif')`


---

## Standard Deviation Overlay

**Function name:** `standard_deviation_overlay`


This tool can be used to find the standard deviation of the values in each raster cell from a set of input rasters (`inputs`). NoData values in any of the input images will result in a NoData pixel in the output image (`output`).  

### Warning

 

Each of the input rasters must have the same spatial extent and number of rows and columns. 

### See Also

 

`min_overlay`, `max_overlay` 

### Python API

```python
def standard_deviation_overlay(self, input_rasters: List[Raster]) -> Raster:
```


---

## Subtract

**Function name:** `subtract`


Experimental

Subtracts the second raster from the first on a cell-by-cell basis.

raster math subtract legacy-port

### Parameters

NameDescriptionRequiredDefault
`input1`First input raster (path string or typed raster object).Required`input1.tif`
`input2`Second input raster (path string or typed raster object).Required`input2.tif`
`output`Optional output raster file path. If omitted, output remains in memory and is returned as a memory:// raster handle.Optional—

### Examples

*Runs subtract on two DEM rasters and writes the result to dem_difference.tif.*
`wbe.subtract(input1='dem_a.tif', input2='dem_b.tif', output='dem_difference.tif')`


---

## Sum Overlay

**Function name:** `sum_overlay`


This tool calculates the sum for each grid cell from a group of raster images (`inputs`). NoData values in any of the input images will result in a NoData pixel in the output image (`output`). 

### Warning

 

Each of the input rasters must have the same spatial extent and number of rows and columns. 

### See Also

 

`weighted_sum`, `multiply_overlay` 

### Python API

```python
def sum_overlay(self, input_rasters: List[Raster]) -> Raster:
```


---

## Update Nodata Cells

**Function name:** `update_nodata_cells`


This tool will assign the *NoData* valued cells in an input raster (`input1`) the values contained in the corresponding grid cells in a second input raster (`input2`). This operation is sometimes necessary because most other overlay operations exclude areas of *NoData* values from the analysis. This tool can be used when there is need to update the values of a raster within these missing data areas. 

### See Also

 

`IsNodata` 

### Python API

```python
def update_nodata_cells(self, input1: Raster, input2: Raster) -> Raster:
```


---

## Weighted Overlay

**Function name:** `weighted_overlay`


This tool performs a weighted overlay on multiple input images. It can be used to combine multiple factors with varying levels of weight or relative importance. The WeightedOverlay tool is similar to the WeightedSum tool but is more powerful because it automatically converts the input factors to a common user-defined scale and allows the user to specify benefit factors and cost factors. A benefit factor is a factor for which higher values are more suitable. A cost factor is a factor for which higher values are less suitable. By default, WeightedOverlay assumes that input images are benefit factors, unless a cost value of 'true' is entered in the cost array. Constraints are absolute restriction with values of 0 (unsuitable) and 1 (suitable). This tool is particularly useful for performing multi-criteria evaluations (MCE). 

Notice that the algorithm will convert the user-defined factor weights internally such that the sum of the weights is always equal to one. As such, the user can specify the relative weights as decimals, percentages, or relative weightings (e.g. slope is 2 times more important than elevation, in which case the weights may not sum to 1 or 100). 

NoData valued grid cells in any of the input images will be assigned NoData values in the output image. The output raster is of the float data type and continuous data scale. 

### Warning

 

Each of the input rasters must have the same spatial extent and number of rows and columns. 

### Python API

```python
def weighted_overlay(self, factors: List[Raster], weights: List[float], cost: List[Raster] = None, constraints: List[Raster] = None, scale_max: float = 1.0) -> Raster:
```


---

## Weighted Sum

**Function name:** `weighted_sum`


This tool performs a weighted-sum overlay on multiple input raster images. If you have a stack of rasters that you would like to sum, each with an equal weighting (1.0), then use the `sum_overlay` tool instead. 

### Warning

 

Each of the input rasters must have the same spatial extent and number of rows and columns. 

### See Also

 

`sum_overlay` 

### Python API

```python
def weighted_sum(self, input_rasters: List[Raster], weights: List[float]) -> Raster:
```
