# Vector and Table I/O


---

## Add Point Coordinates To Table

**Function name:** `add_point_coordinates_to_table`


### Description

 

This tool modifies the attribute table of a vector of POINT VectorGeometryType by adding two fields, XCOORD and YCOORD, containing each point's X and Y coordinates respectively. 

### Parameters

 

input (Vector):     The input Vector object 

### Returns

 

Vector: the returning value 

### Python API

```python
def add_point_coordinates_to_table(self, input: Vector) -> Vector:
```


---

## Clean Vector

**Function name:** `clean_vector`


### Description

 

This tool can be used to remove all features in Shapefiles that are of the `null` VectorGeometryType. It also removes line features with fewer than two vertices and polygon features with fewer than three vertices. 

### Parameters

 

input (Vector):     The input Vector object 

### Returns

 

Vector: the returning value 

### Python API

```python
def clean_vector(self, input: Vector) -> Vector:
```


---

## CSV Points To Vector

**Function name:** `csv_points_to_vector`


This tool can be used to import a series of points contained within a comma-separated values (*.csv) file (`input_file`) into a vector shapefile of a POINT VectorGeometryType. The input file must be an ASCII text file with a .csv extensions. The tool will automatically detect the field data type; for numeric fields, it will also determine the appropriate length and precision. The user must specify the x-coordinate (`x_field_num`) and y-coordiante (`y_field_num`) fields. All fields are imported as attributes in the output (`output`) vector file. The tool assumes that the first line of the file is a header line from which field names are retrieved. 

### See Also

 

`merge_table_with_csv`, `export_table_to_csv` 

### Python API

```python
def csv_points_to_vector(self, input_file: str, x_field_num: int = 0, y_field_num: int = 1, epsg: int = 0) -> Vector:
```


---

## Export Table To CSV

**Function name:** `export_table_to_csv`


This tool can be used to export a vector's attribute table to a comma separated values (CSV) file. CSV files stores tabular data (numbers and text) in plain-text form such that each row corresponds to a record and each column to a field. Fields are typically separated by commas within records. The user must specify the name of the vector (and associated attribute file), the name of the output CSV file, and whether or not to include the field names as a header column in the output CSV file. 

### See Also

 

`merge_table_with_csv` 

### Python API

```python
def export_table_to_csv(self, input: Vector, output_csv_file: str, headers: bool = True) -> None:
```


---

## Join Tables

**Function name:** `join_tables`


This tool can be used to join (i.e. merge) a vector's attribute table with a second table. The user must specify the name of the vector file (and associated attribute file) as well as the *primary key* within the table. The *primary key* (`pkey` flag) is the field within the table that is being appended to that serves as the identifier. Additionally, the user must specify the name of a second vector from which the data appended into the first table will be derived. The *foreign key* (`fkey` flag), the identifying field within the second table that corresponds with the data contained within the primary key in the table, must be specified. Both the primary and foreign keys should either be strings (text) or integer values. *Fields containing decimal values are not good candidates for keys.* Lastly, the names of the field within the second file to include in the merge operation can also be input (`import_field`). If the `import_field` field is not input, all fields in the attribute table of the second file, that are not the foreign key nor FID, will be imported to the first table. 

Merging works for one-to-one and many-to-one database relations. A *one-to-one* relations exists when each record in the attribute table corresponds to one record in the second table and each primary key is unique. Since each record in the attribute table is associated with a geospatial feature in the vector, an example of a one-to-one relation may be where the second file contains AREA and PERIMETER fields for each polygon feature in the vector. This is the most basic type of relation. A many-to-one relation would exist when each record in the first attribute table corresponds to one record in the second file and the primary key is NOT unique. Consider as an example a vector and attribute table associated with a world map of countries. Each country has one or more more polygon features in the shapefile, e.g. Canada has its mainland and many hundred large islands. You may want to append a table containing data about the population and area of each country. In this case, the COUNTRY columns in the attribute table and the second file serve as the primary and foreign keys respectively. While there may be many duplicate primary keys (all of those Canadian polygons) each will correspond to only one foreign key containing the population and area data. This is a *many-to-one* relation. The `join_tables` tool does not support one-to-many nor many-to-many relations. 

### See Also

 

`merge_table_with_csv`, `reinitialize_attribute_table`, `export_table_to_csv` 

### Python API

```python
def join_tables(self, primary_vector: Vector, primary_key_field: str, foreign_vector: Vector, foreign_key_field: str, import_field: str = "") -> None:
```


---

## Merge Table With CSV

**Function name:** `merge_table_with_csv`


This tool can be used to merge a vector's attribute table with data contained within a comma separated values (CSV) text file. CSV files stores tabular data (numbers and text) in plain-text form such that each row is a record and each column a field. Fields are typically separated by commas although the tool will also support seimi-colon, tab, and space delimited files. The user must specify the name of the vector (and associated attribute file) as well as the *primary key* within the table. The *primary key* (`pkey` flag) is the field within the table that is being appended to that serves as the unique identifier. Additionally, the user must specify the name of a CSV text file with either a *.csv or *.txt extension. The file must possess a header row, i.e. the first row must contain information about the names of the various fields. The *foreign key* (`fkey` flag), that is the identifying field within the CSV file that corresponds with the data contained within the *primary key* in the table, must also be specified. Both the primary and foreign keys should either be strings (text) or integer values. *Fields containing decimal values are not good candidates for keys.* Lastly, the user may optionally specify the name of a field within the CSV file to import in the merge operation (`import_field` flag). If this flag is not specified, all of the fields within the CSV, with the exception of the foreign key, will be appended to the attribute table. 

Merging works for one-to-one and many-to-one database relations. A *one-to-one* relations exists when each record in the attribute table corresponds to one record in the second table and each primary key is unique. Since each record in the attribute table is associated with a geospatial feature in the vector, an example of a one-to-one relation may be where the second file contains AREA and PERIMETER fields for each polygon feature in the vector. This is the most basic type of relation. A many-to-one relation would exist when each record in the first attribute table corresponds to one record in the second file and the primary key is NOT unique. Consider as an example a vector and attribute table associated with a world map of countries. Each country has one or more more polygon features in the shapefile, e.g. Canada has its mainland and many hundred large islands. You may want to append a table containing data about the population and area of each country. In this case, the COUNTRY columns in the attribute table and the second file serve as the primary and foreign keys respectively. While there may be many duplicate primary keys (all of those Canadian polygons) each will correspond to only one foreign key containing the population and area data. This is a *many-to-one* relation. The `join_tables` tool does not support one-to-many nor many-to-many relations. 

### See Also

 

`join_tables`, `reinitialize_attribute_table`, `export_table_to_csv` 

### Python API

```python
def merge_table_with_csv(self, primary_vector: Vector, primary_key_field: str, foreign_csv_filename: str, foreign_key_field: str, import_field: str = "") -> None:
```


---

## Merge Vectors

**Function name:** `merge_vectors`


Combines two or more input vectors of the same ShapeType creating a single, new output vector. Importantly, the attribute table of the output vector will contain the ubiquitous file-specific FID, the parent file name, the parent FID, and the list of attribute fields that are shared among each of the input files. For a field to be considered common between tables, it must have the same `name` and `field_type` (i.e. data type and precision). 

Overlapping features will not be identified nor handled in the merging. If you have significant areas of overlap, it is advisable to use one of the vector overlay tools instead. 

The difference between `merge_vectors` and the `Append` tool is that merging takes two or more files and creates one new file containing the features of all inputs, and `Append` places the features of a single vector into another existing (appended) vector. 

This tool only operates on vector files. Use the `mosaic` tool to combine raster data. 

### See Also

 

`Append`, `mosaic` 

### Python API

```python
def merge_vectors(self, input_vectors: List[Vector]) -> Vector:
```


---

## Reinitialize Attribute Table

**Function name:** `reinitialize_attribute_table`


Reinitializes a vector's attribute table deleting all fields but the feature ID (FID). Caution: this tool overwrites the input file's attribute table. 

### Python API

```python
def reinitialize_attribute_table(self, input: Vector) -> None:
```


---

## Vector Summary Statistics

**Function name:** `vector_summary_statistics`


Experimental

Computes grouped summary statistics for a numeric field and writes the result to CSV.

vector statistics table

### Parameters

NameDescriptionRequiredDefault
`input`Input vector layer.Required`input.shp`
`group_field`Grouping field name.Required`CLASS`
`value_field`Numeric value field name.Required`VALUE`
`output`Output CSV path.Required—

### Examples

*Summarizes a value field by category.*
`wbe.vector_summary_statistics(group_field='CLASS', input='input.shp', output='summary.csv', value_field='VALUE')`
