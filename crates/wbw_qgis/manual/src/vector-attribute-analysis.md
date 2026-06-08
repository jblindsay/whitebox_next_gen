# Attribute Analysis


---

## Add Field

**Function name:** `add_field`


Experimental

Adds a new attribute field with an optional default value.

vector schema attributes

### Parameters

NameDescriptionRequiredDefault
`input`Input vector layer.Required`input.shp`
`field`New field name.Required`NEW_FIELD`
`field_type`Field type: integer, float, text, boolean.Required`float`
`default`Optional default value.Optional—
`output`Output vector path.Required—

### Examples

*Adds a typed field to the layer schema.*
`wbe.add_field(default=0.0, field='NEW_FIELD', field_type='float', input='input.shp', output='add_field.shp')`


---

## Add Geometry Attributes

**Function name:** `add_geometry_attributes`


Experimental

Adds area, length, perimeter, and centroid attributes to vector features.

vector attributes measurements

### Parameters

NameDescriptionRequiredDefault
`input`Input vector layer.Required`input.shp`
`area`Include AREA field (default true).Optional`True`
`length`Include LENGTH field (default true).Optional`True`
`perimeter`Include PERIMETER field (default true).Optional`True`
`centroid`Include centroid X/Y fields (default true).Optional`True`
`output`Output vector path.Required—

### Examples

*Adds geometry-derived attributes to each feature.*
`wbe.add_geometry_attributes(area=True, centroid=True, input='input.shp', length=True, output='geometry_attributes.shp', perimeter=True)`


---

## Attribute Correlation

**Function name:** `attribute_correlation`


This tool can be used to estimate the Pearson product-moment correlation coefficient (*r*) for each pair among a group of attributes associated with the database file of a shapefile. The *r*-value is a measure of the linear association in the variation of the attributes. The coefficient ranges from -1, indicated a perfect negative linear association, to 1, indicated a perfect positive linear association. An *r*-value of 0 indicates no correlation between the test variables. 

Notice that this index is a measure of the linear association; two variables may be strongly related by a non-linear association (e.g. a power function curve) which will lead to an apparent weak association based on the Pearson coefficient. In fact, non-linear associations are very common among spatial variables, e.g. terrain indices such as slope and contributing area. In such cases, it is advisable that the input images are transformed prior to the estimation of the Pearson coefficient, or that an alternative, non-parametric statistic be used, e.g. the Spearman rank correlation coefficient. 

The user must specify the name of the input vector Shapefile (`input`). Correlations will be calculated for each pair of numerical attributes contained within the input file's attribute table and presented in a correlation matrix HMTL output (`output`). 

### See Also

 

`image_correlation`, `attribute_scattergram`, `attribute_histogram` 

### Python API

```python
def attribute_correlation(self, input: Vector, output_html_file: str) -> None:
```


---

## Attribute Histogram

**Function name:** `attribute_histogram`


This tool can be used to create a histogram, which is a graph displaying the frequency distribution of data, for the values contained in a field of an input vector's attribute table. The user must specify the name of an input vector (`input`) and the name of one of the fields (`field`) contained in the associated attribute table. The tool output (`output`) is an HTML formatted histogram analysis report. If the specified field is non-numerical, the tool will produce a bar-chart of class frequency, similar to the tabular output of the `list_unique_values` tool. 

### See Also

 

`list_unique_values`, `raster_histogram` 

### Python API

```python
def attribute_histogram(self, input: Vector, field_name: str, output_html_file: str) -> None:
```


---

## Attribute Scattergram

**Function name:** `attribute_scattergram`


This tool can be used to create a `scattergram` for two numerical fields (`fieldx` and `fieldy`) contained within an input vector's attribute table (`input`). The user must specify the name of an input shapefile and the name of two of the fields contained it the associated attribute table. The tool output (`output`) is an HTML formatted report containing a graphical scattergram plot. 

### See Also

 

`attribute_histogram`, `attribute_correlation` 

### Python API

```python
def attribute_scattergram(self, input: Vector, field_name_x: str, field_name_y: str, output_html_file: str, add_trendline: bool = False) -> None:
```


---

## Delete Field

**Function name:** `delete_field`


Experimental

Deletes one or more attribute fields from a vector layer.

vector schema attributes

### Parameters

NameDescriptionRequiredDefault
`input`Input vector layer.Required`input.shp`
`fields`Comma-delimited field names to delete.Required`FIELD_A,FIELD_B`
`output`Output vector path.Required—

### Examples

*Removes selected fields from a layer schema.*
`wbe.delete_field(fields='FIELD_A,FIELD_B', input='input.shp', output='fields_deleted.shp')`


---

## Extract By Attribute

**Function name:** `extract_by_attribute`


This tool extracts features from an input vector into an output file based on attribute properties. The user must specify the name of the input (`--input`) and output (`--output`) files, along with the filter statement (`--statement`). The conditional statement is a single-line logical condition containing one or more attribute variables contained in the file's attribute table that evaluates to TRUE/FALSE. In addition to the common comparison and logical operators, i.e. < > <= >= == (EQUAL TO) != (NOT EQUAL TO) || (OR) && (AND), conditional statements may contain a any valid mathematical operation and the `null` value.   IdentifierArgument AmountArgument TypesDescription `min`>= 1NumericReturns the minimum of the arguments `max`>= 1NumericReturns the maximum of the arguments `len`1String/TupleReturns the character length of a string, or the amount of elements in a tuple (not recursively) `floor`1NumericReturns the largest integer less than or equal to a number `round`1NumericReturns the nearest integer to a number. Rounds half-way cases away from 0.0 `ceil`1NumericReturns the smallest integer greater than or equal to a number `if`3Boolean, Any, AnyIf the first argument is true, returns the second argument, otherwise, returns the third `contains`2Tuple, any non-tupleReturns true if second argument exists in first tuple argument. `contains_any`2Tuple, Tuple of any non-tupleReturns true if one of the values in the second tuple argument exists in first tuple argument. `typeof`1Anyreturns "string", "float", "int", "boolean", "tuple", or "empty" depending on the type of the argument `math::is_nan`1NumericReturns true if the argument is the floating-point value NaN, false if it is another floating-point value, and throws an error if it is not a number `math::is_finite`1NumericReturns true if the argument is a finite floating-point number, false otherwise `math::is_infinite`1NumericReturns true if the argument is an infinite floating-point number, false otherwise `math::is_normal`1NumericReturns true if the argument is a floating-point number that is neither zero, infinite, [subnormal](https://en.wikipedia.org/wiki/Subnormal_number), or NaN, false otherwise `math::ln`1NumericReturns the natural logarithm of the number `math::log`2Numeric, NumericReturns the logarithm of the number with respect to an arbitrary base `math::log2`1NumericReturns the base 2 logarithm of the number `math::log10`1NumericReturns the base 10 logarithm of the number `math::exp`1NumericReturns `e^(number)`, (the exponential function) `math::exp2`1NumericReturns `2^(number)` `math::pow`2Numeric, NumericRaises a number to the power of the other number `math::cos`1NumericComputes the cosine of a number (in radians) `math::acos`1NumericComputes the arccosine of a number. The return value is in radians in the range [0, pi] or NaN if the number is outside the range [-1, 1] `math::cosh`1NumericHyperbolic cosine function `math::acosh`1NumericInverse hyperbolic cosine function `math::sin`1NumericComputes the sine of a number (in radians) `math::asin`1NumericComputes the arcsine of a number. The return value is in radians in the range [-pi/2, pi/2] or NaN if the number is outside the range [-1, 1] `math::sinh`1NumericHyperbolic sine function `math::asinh`1NumericInverse hyperbolic sine function `math::tan`1NumericComputes the tangent of a number (in radians) `math::atan`1NumericComputes the arctangent of a number. The return value is in radians in the range [-pi/2, pi/2] `math::atan2`2Numeric, NumericComputes the four quadrant arctangent in radians `math::tanh`1NumericHyperbolic tangent function `math::atanh`1NumericInverse hyperbolic tangent function. `math::sqrt`1NumericReturns the square root of a number. Returns NaN for a negative number `math::cbrt`1NumericReturns the cube root of a number `math::hypot`2NumericCalculates the length of the hypotenuse of a right-angle triangle given legs of length given by the two arguments `math::abs`1NumericReturns the absolute value of a number, returning an integer if the argument was an integer, and a float otherwise `str::regex_matches`2String, StringReturns true if the first argument matches the regex in the second argument (Requires `regex_support` feature flag) `str::regex_replace`3String, String, StringReturns the first argument with all matches of the regex in the second argument replaced by the third argument (Requires `regex_support` feature flag) `str::to_lowercase`1StringReturns the lower-case version of the string `str::to_uppercase`1StringReturns the upper-case version of the string `str::trim`1StringStrips whitespace from the start and the end of the string `str::from`>= 0AnyReturns passed value as string `bitand`2IntComputes the bitwise and of the given integers `bitor`2IntComputes the bitwise or of the given integers `bitxor`2IntComputes the bitwise xor of the given integers `bitnot`1IntComputes the bitwise not of the given integer `shl`2IntComputes the given integer bitwise shifted left by the other given integer `shr`2IntComputes the given integer bitwise shifted right by the other given integer `random`0EmptyReturn a random float between 0 and 1. Requires the `rand` feature flag. `pi`0EmptyReturn the value of the PI constant./   

The following are examples of valid conditional statements: 

`HEIGHT >= 300.0 

CROP == "corn" 

(ELEV >= 525.0) && (HGT_AB_GR <= 5.0) 

math::ln(CARBON) > 1.0 

VALUE == null ` 

### Python API

```python
def extract_by_attribute(self, input: Vector, statement: str) -> Vector:
```


---

## Field Calculator

**Function name:** `field_calculator`


Experimental

Calculates or updates a field value from SQL-style or expression-style formulas using feature attributes and geometry variables.

vector attributes expression

### Parameters

NameDescriptionRequiredDefault
`input`Input vector layer.Required`input.shp`
`field`Output field name.Required`score`
`field_type`Output field type: float, integer, text.Optional`float`
`expression`Expression or SQL-style UPDATE assignment evaluated per feature.Required`VALUE * 2.0 + $area`
`overwrite`Overwrite existing field if present (default true).Optional`True`
`preview_rows`Optional number of preview rows to return in payload.Optional`0`
`output`Output vector path. Optional when `preview_rows > 0`.Optional—

### Examples

*Computes a derived numeric field using attributes and geometry.*
`wbe.field_calculator(expression='VALUE * 2.0 + $area', field='score', field_type='float', input='input.shp', output='field_calc.shp', overwrite=True)`

*SQL-style conditional update using CASE and UPDATE wrapper.*
`wbe.field_calculator(input='roads.gpkg', field='SPEED', field_type='integer', expression="UPDATE roads SET SPEED = CASE WHEN TYPE == 'motorway' THEN 100 WHEN TYPE == 'primary' THEN 80 ELSE 60 END", overwrite=True, output='roads_speed.gpkg')`

*Preview-only evaluation for first 10 rows (no output write).*
`wbe.field_calculator(input='roads.gpkg', field='SPEED', field_type='integer', expression="CASE TYPE WHEN 'motorway' THEN 100 ELSE 60 END", overwrite=True, preview_rows=10)`


---

## Filter Vector Features By Area

**Function name:** `filter_vector_features_by_area`


Experimental

Filters polygon features below a minimum area threshold.

vector gis filter polygon legacy-port

### Parameters

NameDescriptionRequiredDefault
`input`Input polygon vector layer.Required`polygons.shp`
`threshold`Minimum polygon area to retain, in layer coordinate units squared.Required`1000.0`
`output`Output vector path.Required—

### Examples

*Removes polygons smaller than the specified area threshold.*
`wbe.filter_vector_features_by_area(input='polygons.shp', output='filtered_polygons.shp', threshold=1000.0)`


---

## List Unique Values

**Function name:** `list_unique_values`


This tool can be used to list each of the unique values contained within a categorical field of an input vector file's attribute table. The tool outputs an HTML formatted report (`output`) containing a table of the unique values and their frequency of occurrence within the data. The user must specify the name of an input shapefile (`input`) and the name of one of the fields (`field`) contained in the associated attribute table. The specified field *should not contained floating-point numerical data*, since the number of categories will likely equal the number of records, which may be quite large. The tool effectively provides tabular output that is similar to the graphical output provided by the `attribute_histogram` tool, which, however, can be applied to continuous data. 

### See Also

 

`attribute_histogram` 

### Python API

```python
def list_unique_values(self, input: Vector, field_name: str) -> Tuple[str, int]:
```


---

## Rename Field

**Function name:** `rename_field`


Experimental

Renames an attribute field in a vector layer.

vector schema attributes

### Parameters

NameDescriptionRequiredDefault
`input`Input vector layer.Required`input.shp`
`field`Existing field name.Required`OLD_NAME`
`new_field`Replacement field name.Required`NEW_NAME`
`output`Output vector path.Required—

### Examples

*Renames one attribute field.*
`wbe.rename_field(field='OLD_NAME', input='input.shp', new_field='NEW_NAME', output='renamed.shp')`
