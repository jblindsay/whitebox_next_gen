# General Tools


---

## Assign Projection LiDAR

**Function name:** `assign_projection_lidar`


*No help documentation available for this tool.*


---

## Assign Projection Raster

**Function name:** `assign_projection_raster`


*No help documentation available for this tool.*


---

## Assign Projection Vector

**Function name:** `assign_projection_vector`


*No help documentation available for this tool.*


---

## Georeference Raster From Control Points

**Function name:** `georeference_raster_from_control_points`


*No help documentation available for this tool.*


---

## Reproject LiDAR

**Function name:** `reproject_lidar`


*No help documentation available for this tool.*


---

## Reproject Raster

**Function name:** `reproject_raster`


*No help documentation available for this tool.*


---

## Reproject Vector

**Function name:** `reproject_vector`


Experimental

Reprojects an input vector layer to a destination EPSG code.

vector projection crs

### Parameters

NameDescriptionRequiredDefault
`input`Input vector layer.Required`input.shp`
`epsg`Destination EPSG code.Required`4326`
`output`Output vector path.Required—

### Examples

*Reprojects a vector layer to EPSG:4326.*
`wbe.reproject_vector(epsg=4326, input='input.shp', output='reprojected.shp')`


---

## Orthorectification

**Function name:** `orthorectification`


*No help documentation available for this tool.*
