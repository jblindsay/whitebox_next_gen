# Working with Vectors

This chapter covers schema inspection, feature iteration, attribute reads/writes,
and persistence workflows.

## Read and Inspect

```r
library(whiteboxworkflows)

v <- wbw_read_vector('roads.gpkg')
schema <- v$schema()
print(schema)
print(v$metadata())
```

## Iterate Through Features

```r
library(whiteboxworkflows)

v <- wbw_read_vector('roads.gpkg')
count <- v$num_records()

for (i in seq_len(count)) {
  attrs <- v$attributes(i)
  # attrs is a named list
  print(attrs)
}
```

## Read and Update Attribute Table

```r
library(whiteboxworkflows)

v <- wbw_read_vector('roads.gpkg')

name1 <- v$attribute(1, 'name')
print(name1)

v$update_attribute(1, 'name', 'Main Street')
v$update_attributes(2, list(speed = 50, class = 'collector'))
v$add_field('reviewed', field_type = 'integer', default_value = 0)
```

## Persist Vector Outputs

```r
library(whiteboxworkflows)

s <- wbw_session()
roads <- wbw_read_vector('roads.gpkg')

wbw_run_tool(
  'buffer_vector',
  args = list(input = roads$file_path(), output = 'roads_buffer.gpkg', distance = 15.0),
  session = s
)

buffered <- wbw_read_vector('roads_buffer.gpkg')
print(buffered$metadata())
```

## Practical Notes

- Call `schema()` first to confirm field names and expected types.
- Use `update_attributes()` for grouped feature edits.
- Re-read output files to validate schema and values after writes.
