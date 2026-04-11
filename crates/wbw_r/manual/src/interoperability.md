# Interoperability

This chapter provides practical roundtrip patterns between WbW-R and R spatial tooling.

## Copy-Boundary Model

- Array exchange via `to_array()` and `wbw_array_to_raster(...)` is an explicit in-memory boundary.
- terra/stars/sf workflows are typically file or object conversion boundaries.
- Always validate metadata after roundtrip.

## terra Roundtrip

```r
library(whiteboxworkflows)
library(terra)

r <- wbw_read_raster('dem.tif')
terra_r <- r$to_terra()

# terra-side operation
terra_r2 <- terra::focal(terra_r, w = 3, fun = mean, na.rm = TRUE)
terra::writeRaster(terra_r2, 'dem_terra_smoothed.tif', overwrite = TRUE)

r_back <- wbw_read_raster('dem_terra_smoothed.tif')
print(r_back$metadata())
```

## stars Roundtrip

```r
library(whiteboxworkflows)
library(stars)

r <- wbw_read_raster('dem.tif')
st <- r$to_stars()

# stars-side operation
st2 <- st * 1.05
st_as_stars <- st_as_stars(st2)
write_stars(st_as_stars, 'dem_stars_scaled.tif', driver = 'GTiff')

r_back <- wbw_read_raster('dem_stars_scaled.tif')
print(r_back$metadata())
```

## sf Roundtrip (Vector)

```r
library(whiteboxworkflows)
library(sf)

v <- wbw_read_vector('roads.gpkg')
sf_obj <- v$to_sf()

# sf-side edit
sf_obj$len_m <- as.numeric(st_length(sf_obj))
sf_obj <- sf_obj[sf_obj$len_m > 20, ]
st_write(sf_obj, 'roads_sf_filtered.gpkg', delete_dsn = TRUE, quiet = TRUE)

v_back <- wbw_read_vector('roads_sf_filtered.gpkg')
print(v_back$schema())
```

## Validation Checklist

1. Re-check CRS and bounds after conversion.
2. Re-check schema and representative attributes for vector flows.
3. Prefer stable interchange formats (`.tif`, `.gpkg`) for routine roundtrips.
