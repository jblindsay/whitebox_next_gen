# Math and Statistical Tools

This document covers the Math tools currently ported into the new backend.

## Unary Raster Math

These tools apply an element-wise mathematical operation to every non-nodata cell of a single input raster and write the result to an output raster.

### Unary Tool Index

- `abs`
- `arccos`
- `arcosh`
- `arcsin`
- `arctan`
- `arsinh`
- `artanh`
 - `bool_not`
- `ceil`
- `cos`
- `cosh`
- `decrement`
- `exp`
- `exp2`
- `floor`
- `increment`
- `is_nodata`
- `ln`
- `log10`
- `log2`
- `negate`
- `reciprocal`
- `round`
- `sin`
- `sinh`
- `sqrt`
- `square`
- `tan`
- `tanh`
- `to_degrees`
- `to_radians`
- `truncate`

### `abs`

Calculates the absolute value of each non-nodata raster cell.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.abs(input_raster, output_path="abs_dem.tif")
```

---

### `arccos`

Computes the inverse cosine (in radians) of each non-nodata raster cell. Input values must be in the range \[-1, 1\].

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.arccos(input_raster, output_path="arccos_dem.tif")
```

---

### `arcosh`

Computes the inverse hyperbolic cosine of each non-nodata raster cell. Input values must be ≥ 1.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.arcosh(input_raster, output_path="arcosh_dem.tif")
```

---

### `arcsin`

Computes the inverse sine (in radians) of each non-nodata raster cell. Input values must be in the range \[-1, 1\].

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.arcsin(input_raster, output_path="arcsin_dem.tif")
```

---

### `arctan`

Computes the inverse tangent (in radians) of each non-nodata raster cell.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.arctan(input_raster, output_path="arctan_dem.tif")
```

---

### `arsinh`

Computes the inverse hyperbolic sine of each non-nodata raster cell.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.arsinh(input_raster, output_path="arsinh_dem.tif")
```

---

### `artanh`

Computes the inverse hyperbolic tangent of each non-nodata raster cell. Input values must be in the range (-1, 1).

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.artanh(input_raster, output_path="artanh_dem.tif")
```

---

### `ceil`

Rounds each non-nodata raster cell upward to the nearest integer.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.ceil(input_raster, output_path="ceil_dem.tif")
```

---

### `cos`

Computes the cosine (input in radians) of each non-nodata raster cell.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.cos(input_raster, output_path="cos_dem.tif")
```

---

### `cosh`

Computes the hyperbolic cosine of each non-nodata raster cell.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.cosh(input_raster, output_path="cosh_dem.tif")
```

---

### `decrement`

Subtracts 1 from each non-nodata raster cell value.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.decrement_raster(input_raster, output_path="dem_minus1.tif")
```

Note: The WbEnvironment method is named `decrement_raster` to avoid a name clash with the cell-level `decrement` helper method.

---

### `exp`

Computes *e* raised to the power of each non-nodata raster cell value.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.exp(input_raster, output_path="exp_dem.tif")
```

---

### `exp2`

Computes 2 raised to the power of each non-nodata raster cell value.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.exp2(input_raster, output_path="exp2_dem.tif")
```

---

### `floor`

Rounds each non-nodata raster cell downward to the nearest integer.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.floor(input_raster, output_path="floor_dem.tif")
```

---

### `increment`

Adds 1 to each non-nodata raster cell value.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.increment_raster(input_raster, output_path="dem_plus1.tif")
```

Note: The WbEnvironment method is named `increment_raster` to avoid a name clash with the cell-level `increment` helper method.

---

### `is_nodata`

Outputs 1.0 for every nodata cell and 0.0 for every valid cell. Useful for creating nodata masks.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
mask = env.is_nodata_raster(input_raster, output_path="dem_nodata_mask.tif")
```

Note: The WbEnvironment method is named `is_nodata_raster` to avoid a name clash with the Raster-class `is_nodata` predicate.

---

### `ln`

Computes the natural logarithm of each non-nodata raster cell.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.ln(input_raster, output_path="ln_dem.tif")
```

---

### `log10`

Computes the base-10 logarithm of each non-nodata raster cell.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.log10(input_raster, output_path="log10_dem.tif")
```

---

### `log2`

Computes the base-2 logarithm of each non-nodata raster cell.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.log2(input_raster, output_path="log2_dem.tif")
```

---

### `negate`

Negates each non-nodata raster cell value (multiplies by -1).

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.negate(input_raster, output_path="neg_dem.tif")
```

---

### `reciprocal`

Computes the reciprocal (1/x) of each non-nodata raster cell.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.reciprocal(input_raster, output_path="recip_dem.tif")
```

---

### `round`

Rounds each non-nodata raster cell to the nearest integer.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.round(input_raster, output_path="round_dem.tif")
```

---

### `sin`

Computes the sine (input in radians) of each non-nodata raster cell.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.sin(input_raster, output_path="sin_dem.tif")
```

---

### `sinh`

Computes the hyperbolic sine of each non-nodata raster cell.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.sinh(input_raster, output_path="sinh_dem.tif")
```

---

### `sqrt`

Computes the square root of each non-nodata raster cell.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.sqrt(input_raster, output_path="sqrt_dem.tif")
```

---

### `square`

Squares each non-nodata raster cell value (x²).

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.square(input_raster, output_path="square_dem.tif")
```

---

### `tan`

Computes the tangent (input in radians) of each non-nodata raster cell.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.tan(input_raster, output_path="tan_dem.tif")
```

---

### `tanh`

Computes the hyperbolic tangent of each non-nodata raster cell.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.tanh(input_raster, output_path="tanh_dem.tif")
```

---

### `to_degrees`

Converts each non-nodata raster cell from radians to degrees.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.to_degrees(input_raster, output_path="deg_dem.tif")
```

---

### `to_radians`

Converts each non-nodata raster cell from degrees to radians.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.to_radians(input_raster, output_path="rad_dem.tif")
```

---

### `truncate`

Truncates each non-nodata raster cell to its integer part (rounds toward zero).

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.truncate(input_raster, output_path="trunc_dem.tif")
```

---

### `bool_not`

Computes a logical NOT of each non-nodata raster cell, outputting 1 where the input cell value is 0 and 0 otherwise.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | string | yes | Input raster file path. |
| `output` | string | yes | Output raster file path. |

**WbEnvironment usage**

```python
result = env.bool_not(input_raster, output_path="bool_not_dem.tif")
```

---

## Binary Raster Math

These tools combine two rasters on a cell-by-cell basis.

### Binary Tool Index

- `add`
 - `atan2`
 - `bool_and`
 - `bool_or`
 - `bool_xor`
- `divide`
 - `equal_to`
 - `greater_than`
 - `integer_division`
 - `less_than`
 - `modulo`
- `multiply`
 - `not_equal_to`
 - `power`
- `subtract`

All binary raster math tools share the same core parameter shape: `input1`, `input2`, and optional `output`.

**WbEnvironment usage examples**

```python
sum_raster = env.add(raster_a, raster_b, output_path="sum.tif")
pow_raster = env.power(raster_a, raster_b, output_path="power.tif")
mask = env.greater_than(raster_a, raster_b, output_path="gt_mask.tif")
logic = env.bool_and(raster_a, raster_b, output_path="and_mask.tif")
```

Tool semantics:

- `atan2`: four-quadrant inverse tangent, cell by cell.
- `bool_and`, `bool_or`, `bool_xor`: treat any non-zero value as `true` and return `1.0` or `0.0`.
- `equal_to`, `not_equal_to`, `greater_than`, `less_than`: comparison predicates that return `1.0` or `0.0`.
- `integer_division`: divides and truncates the result toward zero.
- `modulo`: returns the remainder of division.
- `power`: raises `input1` to the power of `input2`.

See [tools_gis.md](tools_gis.md) for the older documentation context around `add`, `subtract`, `multiply`, and `divide`.

---

## Statistical Raster Tools

### Tool Index

- `raster_summary_stats`
- `raster_histogram`
- `list_unique_values_raster`
- `z_scores`
- `rescale_value_range`
- `max`
- `min`
- `quantiles`
- `list_unique_values`
- `root_mean_square_error`
- `random_field`
- `random_sample`
- `cumulative_distribution`
- `crispness_index`
- `ks_normality_test`
- `inplace_add`
- `inplace_subtract`
- `inplace_multiply`
- `inplace_divide`
- `attribute_histogram`
- `attribute_scattergram`
- `attribute_correlation`
- `cross_tabulation`
- `zonal_statistics`
- `turning_bands_simulation`
- `trend_surface`
- `trend_surface_vector_points`
- `raster_calculator`
- `principal_component_analysis`
- `inverse_pca`

### `raster_summary_stats`

Computes summary statistics for valid raster cells and returns a JSON report string.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster object. |

**WbEnvironment usage**

```python
report_json = env.raster_summary_stats(input_raster)
```

---

### `raster_histogram`

Builds a fixed-bin histogram for valid raster cells and returns a JSON report string.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster object. |
| `bins` | int | no | Number of bins (default 256). |

**WbEnvironment usage**

```python
hist_json = env.raster_histogram(input_raster, bins=256)
```

---

### `list_unique_values_raster`

Lists unique raster values (integers) up to a maximum count and returns a JSON report string.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster object. |
| `max_values` | int | no | Maximum values to return (default 10000). |

**WbEnvironment usage**

```python
unique_json = env.list_unique_values_raster(classified_raster, max_values=5000)
```

---

### `z_scores`

Standardizes raster values to z-scores using global mean and standard deviation.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster object. |
| `output_path` | string | no | Optional output raster path. |

**WbEnvironment usage**

```python
z = env.z_scores(input_raster, output_path="z_scores.tif")
```

---

### `rescale_value_range`

Linearly rescales raster values into a target output range.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster object. |
| `out_min` | float | yes | Target minimum output value. |
| `out_max` | float | yes | Target maximum output value. |
| `clip_min` | float | no | Optional input clip minimum. |
| `clip_max` | float | no | Optional input clip maximum. |
| `output_path` | string | no | Optional output raster path. |

**WbEnvironment usage**

```python
rescaled = env.rescale_value_range(input_raster, 0.0, 255.0, output_path="rescaled.tif")
```

---

### `max`

Computes cellwise maximum using raster/raster or raster/constant operands.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input1` | Raster or float | yes | First operand. |
| `input2` | Raster or float | yes | Second operand. |
| `output_path` | string | no | Optional output raster path. |

**WbEnvironment usage**

```python
maxed = env.max(dem_a, dem_b, output_path="max_ab.tif")
maxed_const = env.max(dem_a, 100.0, output_path="max_const.tif")
```

---

### `min`

Computes cellwise minimum using raster/raster or raster/constant operands.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input1` | Raster or float | yes | First operand. |
| `input2` | Raster or float | yes | Second operand. |
| `output_path` | string | no | Optional output raster path. |

**WbEnvironment usage**

```python
mined = env.min(dem_a, dem_b, output_path="min_ab.tif")
mined_const = env.min(dem_a, 0.0, output_path="min_const.tif")
```

---

### `quantiles`

Assigns each valid raster cell to a quantile class from `1..num_quantiles`.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster object. |
| `num_quantiles` | int | no | Number of classes (default 5). |
| `output_path` | string | no | Optional output raster path. |

**WbEnvironment usage**

```python
q = env.quantiles(input_raster, num_quantiles=5, output_path="quantiles.tif")
```

---

### `list_unique_values`

Reports unique value counts for a vector attribute field and returns a JSON report string.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector object. |
| `field_name` | string | yes | Attribute field name to summarize. |

**WbEnvironment usage**

```python
report_json = env.list_unique_values(parcels, "landuse")
```

---

### `root_mean_square_error`

Computes RMSE and related vertical-accuracy metrics between comparison and base rasters.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Comparison raster. |
| `base` | Raster | yes | Base/reference raster. |

**WbEnvironment usage**

```python
report_json = env.root_mean_square_error(dem_test, dem_reference)
```

---

### `random_field`

Creates a raster filled with standard normal random values using a base raster for geometry.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `base` | Raster | yes | Base raster defining output geometry. |
| `output_path` | string | no | Optional output raster path. |

**WbEnvironment usage**

```python
rand_img = env.random_field(base_raster, output_path="random_field.tif")
```

---

### `random_sample`

Creates a raster with randomly located valid sample cells labelled with unique IDs.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `base` | Raster | yes | Base raster used for output geometry and valid-cell mask. |
| `num_samples` | int | no | Number of sample cells to generate (default 1000). |
| `output_path` | string | no | Optional output raster path. |

**WbEnvironment usage**

```python
sample = env.random_sample(base_raster, num_samples=500, output_path="sample.tif")
```

---

### `cumulative_distribution`

Transforms raster values into cumulative distribution probabilities in the range `0..1`.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster object. |
| `output_path` | string | no | Optional output raster path. |

**WbEnvironment usage**

```python
cdf = env.cumulative_distribution(input_raster, output_path="cdf.tif")
```

---

### `crispness_index`

Calculates the crispness index for a membership-probability raster and returns a JSON report string.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Membership-probability raster. |

**WbEnvironment usage**

```python
report_json = env.crispness_index(probability_raster)
```

---

### `ks_normality_test`

Runs a Kolmogorov-Smirnov normality test on raster values and returns a JSON report string.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster object. |
| `num_samples` | int | no | Optional random sample size; omit to use all valid cells. |

**WbEnvironment usage**

```python
report_json = env.ks_normality_test(input_raster, num_samples=1000)
```

---

### `kappa_index`

Computes Cohen's kappa, overall agreement, and per-class agreement metrics for two categorical rasters and returns a JSON report string.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input1` | Raster | yes | Classification raster. |
| `input2` | Raster | yes | Reference raster. |

**WbEnvironment usage**

```python
report_json = env.kappa_index(classified_raster, reference_raster)
```

---

### `paired_sample_t_test`

Runs a paired-sample t-test on two rasters using valid paired cells and returns a JSON report string.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input1` | Raster | yes | First raster in the pair. |
| `input2` | Raster | yes | Second raster in the pair. |
| `num_samples` | int | no | Optional random sample size; omit to use all valid pairs. |

**WbEnvironment usage**

```python
report_json = env.paired_sample_t_test(before_raster, after_raster, num_samples=2000)
```

---

### `phi_coefficient`

Performs a binary-class agreement assessment between two rasters and returns a JSON report string containing contingency counts and the $\phi$ coefficient.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input1` | Raster | yes | First binary raster. Non-zero cells are treated as class presence. |
| `input2` | Raster | yes | Second binary raster. Non-zero cells are treated as class presence. |

**WbEnvironment usage**

```python
report_json = env.phi_coefficient(predicted_binary, reference_binary)
```

---

### `image_correlation`

Computes a Pearson correlation matrix among two or more input rasters and returns a JSON report string.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_rasters` | list[Raster or str] | yes | Input raster handles or file paths (at least two). |

**WbEnvironment usage**

```python
report_json = env.image_correlation([band1, band2, band3])
```

---

### `image_autocorrelation`

Computes global Moran's $I$ for one or more rasters and returns a JSON report string with normality/randomization statistics.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_rasters` | list[Raster or str] | yes | Input raster handles or file paths (at least one). |
| `contiguity` | string | no | Neighborhood rule: `rook`, `king`/`queen`, or `bishop` (default `rook`). |

**WbEnvironment usage**

```python
report_json = env.image_autocorrelation([elevation, slope], contiguity="king")
```

---

### `image_correlation_neighbourhood_analysis`

Computes moving-window local correlation between two rasters and returns two rasters: local correlation values and local p-values.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input1` | Raster | yes | First input raster. |
| `input2` | Raster | yes | Second input raster. |
| `filter_size` | int | no | Moving window size in cells (default `11`, minimum `3`). |
| `correlation_stat` | string | no | Correlation metric: `pearson`, `spearman`, or `kendall` (default `pearson`). |
| `output1_path` | string | no | Optional path for the local-correlation raster. |
| `output2_path` | string | no | Optional path for the local-significance raster. |

**WbEnvironment usage**

```python
local_r, local_p = env.image_correlation_neighbourhood_analysis(
	band1,
	band2,
	filter_size=11,
	correlation_stat="spearman",
)
```

---

### `image_regression`

Performs bivariate linear regression using two rasters and returns a residual raster plus a JSON report string containing model, ANOVA, and coefficient statistics.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `independent_variable` | Raster | yes | Independent variable raster (X). |
| `dependent_variable` | Raster | yes | Dependent variable raster (Y). |
| `standardize_residuals` | bool | no | Standardize residuals by model standard error (default `False`). |
| `output_path` | string | no | Optional output path for residual raster. |

**WbEnvironment usage**

```python
residuals, report_json = env.image_regression(
    independent_raster,
    dependent_raster,
    standardize_residuals=True,
)
```

---

### `dbscan`

Performs unsupervised DBSCAN (Density-Based Spatial Clustering of Applications with Noise) clustering on a stack of input rasters. Each cell's feature vector spans all input bands. Cluster IDs (0-based, I16) are written to the output raster; noise cells and nodata cells receive the nodata value (-32768). A JSON report string is also returned with summary statistics.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_rasters` | List[Raster] | yes | Feature-band rasters forming the multi-dimensional feature space. |
| `scaling_method` | str | no | Feature scaling: `"none"` (default), `"normalize"` (0–1), or `"standardize"` (z-scores). |
| `search_distance` | float | no | Epsilon neighbourhood radius in feature space (default `1.0`). |
| `min_points` | int | no | Minimum number of neighbours within epsilon for a core point (default `5`). |
| `output_path` | str | no | Optional output raster path. |

**WbEnvironment usage**

```python
clusters, report_json = env.dbscan(
    [band1, band2, band3],
    scaling_method="normalize",
    search_distance=0.1,
    min_points=10,
)
```

---

| `num_samples` | int | no | Optional random sample size per raster; omit to use all valid values. |

**WbEnvironment usage**

```python
report_json = env.two_sample_ks_test(raster_a, raster_b, num_samples=3000)
```

---

### `wilcoxon_signed_rank_test`

Runs a Wilcoxon signed-rank test on paired raster differences and returns a JSON report string.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input1` | Raster | yes | First raster in the pair. |
| `input2` | Raster | yes | Second raster in the pair. |
| `num_samples` | int | no | Optional random sample size; omit to use all valid pairs. |

**WbEnvironment usage**

```python
report_json = env.wilcoxon_signed_rank_test(before_raster, after_raster, num_samples=2000)
```

---

### `conditional_evaluation`

Evaluates a per-cell boolean statement and assigns TRUE/FALSE outputs from constants, rasters, or expressions.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster used for `value` and raster geometry variables. |
| `statement` | string | yes | Conditional expression evaluated per cell. |
| `true_value` | Raster, float, or string | no | Value/expression used when condition is true (defaults to NoData). |
| `false_value` | Raster, float, or string | no | Value/expression used when condition is false (defaults to NoData). |
| `output_path` | string | no | Optional output raster path. |

**WbEnvironment usage**

```python
out = env.conditional_evaluation(
	input_raster,
	"(value >= 25.0) && (value <= 75.0)",
	true_value=1.0,
	false_value=0.0,
	output_path="conditional.tif",
)
```

---

### `anova`

Performs one-way ANOVA of raster values grouped by a class raster and returns a JSON report string.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Measurement raster. |
| `features` | Raster | yes | Class/category raster defining groups. |

**WbEnvironment usage**

```python
report_json = env.anova(measurement_raster, class_raster)
```

---

### `inplace_add`

Performs the legacy in-place add operation `input1 += input2` and returns the updated raster handle.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input1` | Raster | yes | Raster to modify. |
| `input2` | Raster or float | yes | Raster or constant addend. |

**WbEnvironment usage**

```python
updated = env.inplace_add(input_raster, 10.0)
```

---

### `inplace_subtract`

Performs the legacy in-place subtract operation `input1 -= input2` and returns the updated raster handle.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input1` | Raster | yes | Raster to modify. |
| `input2` | Raster or float | yes | Raster or constant subtrahend. |

**WbEnvironment usage**

```python
updated = env.inplace_subtract(input_raster, other_raster)
```

---

### `inplace_multiply`

Performs the legacy in-place multiply operation `input1 *= input2` and returns the updated raster handle.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input1` | Raster | yes | Raster to modify. |
| `input2` | Raster or float | yes | Raster or constant multiplier. |

**WbEnvironment usage**

```python
updated = env.inplace_multiply(input_raster, 1.25)
```

---

### `inplace_divide`

Performs the legacy in-place divide operation `input1 /= input2` and returns the updated raster handle.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input1` | Raster | yes | Raster to modify. |
| `input2` | Raster or float | yes | Raster or non-zero constant divisor. |

**WbEnvironment usage**

```python
updated = env.inplace_divide(input_raster, 2.0)
```

---

### `attribute_histogram`

Builds histogram summary counts for a numeric vector attribute field and returns a JSON report string.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector object. |
| `field_name` | string | yes | Numeric attribute field name. |

**WbEnvironment usage**

```python
report_json = env.attribute_histogram(parcels, "area")
```

---

### `attribute_scattergram`

Computes scattergram summary statistics for two numeric vector fields and returns a JSON report string.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector object. |
| `field_name_x` | string | yes | Numeric x-axis field name. |
| `field_name_y` | string | yes | Numeric y-axis field name. |
| `trendline` | bool | no | Include linear trendline metrics (default `False`). |

**WbEnvironment usage**

```python
report_json = env.attribute_scattergram(parcels, "area", "perimeter", trendline=True)
```

---

### `attribute_correlation`

Computes the Pearson correlation matrix among numeric vector attribute fields and returns a JSON report string.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector object. |

**WbEnvironment usage**

```python
report_json = env.attribute_correlation(parcels)
```

---

### `cross_tabulation`

Creates a categorical contingency table between two rasters and returns a JSON report string.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input1` | Raster | yes | First categorical raster. |
| `input2` | Raster | yes | Second categorical raster. |

**WbEnvironment usage**

```python
report_json = env.cross_tabulation(classes_2000, classes_2020)
```

---

### `zonal_statistics`

Computes statistics (mean, median, min, max, range, standard deviation, diversity, total) for zones defined by a features raster and returns results as a raster and JSON report.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `data_raster` | Raster | yes | Raster containing values to summarize. |
| `features_raster` | Raster | yes | Raster containing integer zone IDs. |
| `stat_type` | string | no | Statistic to compute (default `"mean"`). Options: `mean`, `median`, `min`, `max`, `range`, `std_dev`, `diversity`, `total`. |
| `zero_is_background` | bool | no | Exclude zone ID 0 from analysis (default `False`). |
| `output_path` | string | no | Output raster file path. |
| `callback` | function | no | Progress callback function. |

**WbEnvironment usage**

```python
result = env.zonal_statistics(
    dem, 
    zone_raster, 
    stat_type="mean", 
    output_path="dem_zones_mean.tif"
)
```

---

### `turning_bands_simulation`

Simulates a spatially-autocorrelated random field using Carr's turning bands algorithm. Useful for Monte Carlo uncertainty analysis and geostatistical simulation.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `base_raster` | Raster | yes | Base raster defining output extent and resolution. |
| `range` | float | yes | Autocorrelation range parameter (determines smoothness). |
| `iterations` | int | no | Number of random bands to accumulate (default `1000`). |
| `output_path` | string | no | Output raster file path. |
| `callback` | function | no | Progress callback function. |

**WbEnvironment usage**

```python
simulated = env.turning_bands_simulation(
    template_raster,
    range=100.0,
    iterations=500,
    output_path="simulated_field.tif"
)
```

---

### `trend_surface`

Fits a polynomial trend surface to raster data via least-squares regression. Supports polynomial orders 1–10.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_raster` | Raster | yes | Input raster to fit. |
| `polynomial_order` | int | no | Polynomial order 1–10 (default `1` for linear). |
| `output_path` | string | no | Output raster file path. |
| `callback` | function | no | Progress callback function. |

**WbEnvironment usage**

```python
trend = env.trend_surface(
    dem,
    polynomial_order=2,
    output_path="dem_trend_quadratic.tif"
)
```

---

### `trend_surface_vector_points`

Fits a polynomial trend surface to vector point data (with attribute values) and generates an output raster.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `vector_input` | Vector | yes | Input point vector layer. |
| `cell_size` | float | yes | Output raster cell size. |
| `field_name` | string | yes | Name of numeric attribute field to fit. |
| `polynomial_order` | int | no | Polynomial order 1–10 (default `1` for linear). |
| `output_path` | string | no | Output raster file path. |
| `callback` | function | no | Progress callback function. |

**WbEnvironment usage**

```python
trend = env.trend_surface_vector_points(
    sample_points,
    cell_size=10.0,
    field_name="elevation",
    polynomial_order=2,
    output_path="points_trend_surface.tif"
)
```

---

### `raster_calculator`

Evaluates arbitrary mathematical expressions cell-by-cell over one or more input rasters.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `expression` | string | yes | Mathematical expression (e.g., `"'nir' - 'red'"` for NDVI). |
| `input_rasters` | list[Raster] | yes | List of input rasters in expression order. |
| `output_path` | string | no | Output raster file path. |
| `callback` | function | no | Progress callback function. |

**Special Variables**

Expression can reference: `rows`, `columns`, `north`, `south`, `east`, `west`, `cellsizex`, `cellsizey`, `cellsize`, `nodata`, `null`, `minvalue`, `maxvalue`, `pi`, `e`, `row`, `column`, `rowy`, `columnx`, and inputs as value1, value2, etc.

**WbEnvironment usage**

```python
# NDVI example with named rasters
ndvi = env.raster_calculator(
    "'nir' - 'red' / ('nir' + 'red')",
    [nir_band, red_band],
    output_path="ndvi.tif"
)
```

---

### `principal_component_analysis`

Performs PCA (dimensionality reduction) on 3 or more raster bands. Outputs component rasters and eigendecomposition report.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_rasters` | list[Raster] | yes | Input raster bands (3 or more). |
| `num_components` | int | no | Number of PCA components to output (default: all input bands). |
| `standardized` | bool | no | Use correlation matrix (standardized) vs covariance (default `True`). |
| `output_path` | string | no | Output directory for component rasters. |
| `callback` | function | no | Progress callback function. |

**WbEnvironment usage**

```python
# Perform PCA on multispectral bands
components = env.principal_component_analysis(
    [band1, band2, band3, band4],
    num_components=3,
    standardized=True,
    output_path="pca_components/"
)
```

**Output Files**

Component rasters: `{stem}_comp1.tif`, `{stem}_comp2.tif`, etc. JSON report includes eigenvalues, eigenvectors, and factor loadings.

---

### `inverse_pca`

Reconstructs original band images from PCA components, with optional noise reduction by excluding high-order components.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `component_rasters` | list[Raster] | yes | PCA component rasters (in order). |
| `pca_report` | string | yes | JSON report string from `principal_component_analysis()`. |
| `output_path` | string | no | Output directory for reconstructed bands. |
| `callback` | function | no | Progress callback function. |

**WbEnvironment usage**

```python
# Reconstruct with denoising (exclude last 2 components)
reconstructed = env.inverse_pca(
    [comp1, comp2, comp3],
    pca_report=report_json,
    output_path="reconstructed_bands/"
)
```

**Output Files**

Reconstructed rasters: `{stem}_img1.tif`, `{stem}_img2.tif`, etc.

---
