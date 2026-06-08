# Reclass and Mask


---

## Conditional Evaluation

**Function name:** `conditional_evaluation`


Experimental

Performs if-then-else conditional evaluation on raster cells.

raster math conditional legacy-port

### Parameters

NameDescriptionRequiredDefault
`input`Input raster path.Required`input.tif`
`statement`Conditional expression evaluated per cell.Required`value > 35.0`
`true`Value or raster/expression used when condition is true.Optional`1.0`
`false`Value or raster/expression used when condition is false.Optional`0.0`
`output`Optional output raster path.Optional—

### Examples

*Assign values based on a per-cell condition.*
`wbe.conditional_evaluation(false='dem.tif', input='dem.tif', output='conditional.tif', statement='value > 2500.0', true=2500.0)`


---

## Reclass

**Function name:** `reclass`


This tool creates a new raster in which the value of each grid cell is determined by an input raster (`input`) and a collection of user-defined classes. The user must specify the *New* value, the *From* value, and the *To Just Less Than* value of each class triplet of the `reclass_value` parameter. Classes must be mutually exclusive. Reclass values must be presented as lists-of-lists, where each row of the list contains either three (`assign_mode=False`) or two  (`assign_mode=True`) values. If assign-mode is True, then the pair of values represent New value and Old value keys. As an example: 

`reclassed = wbe.reclass(raster, [[1.0, 0.0, 100.0], [2.0, 100.0, 200.0]], assign_mode=False) ` 

### Python API

```python
def reclass(self, raster: Raster, reclass_values: List[List[float]], assign_mode: bool = False) -> Raster:
```


---

## Reclass Equal Interval

**Function name:** `reclass_equal_interval`


This tool reclassifies the values in an input raster (`input`) file based on an equal-interval scheme, where the user must specify the reclass interval value (`interval`), the starting value (`start_val`), and optionally, the ending value (`end_val`). Grid cells containing values that fall outside of the range defined by the starting and ending values, will be assigned their original values in the output grid. If the user does not specify an ending value, the tool will assign a very large positive value. 

### See Also

 

`reclass` 

### Python API

```python
def reclass_equal_interval(self, raster: Raster, interval_size: float, start_value: float = float('-inf'), end_value: float = float('inf')) -> Raster:
```
