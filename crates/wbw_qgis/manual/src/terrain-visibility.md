# Visibility Analysis


---

## Average Horizon Distance

**Function name:** `average_horizon_distance`


PROExperimental

Calculates average distance to horizon across azimuth directions.

geomorphometry terrain visibility


---

## Horizon Angle

**Function name:** `horizon_angle`


This tool calculates the horizon angle (*Sx*), i.e. the maximum slope along a specified azimuth (0-360 degrees) for each grid cell in an input digital elevation model (DEM). Horizon angle is sometime referred to as the maximum upwind slope in wind exposure/sheltering studies. Positive values can be considered sheltered with respect to the azimuth and negative values are exposed. Thus, *Sx* is a measure of exposure to a wind from a specific direction. The algorithm works by tracing a ray from each grid cell in the direction of interest and evaluating the slope for each location in which the DEM grid is intersected by the ray. Linear interpolation is used to estimate the elevation of the surface where a ray does not intersect the DEM grid precisely at one of its nodes. 

The user is able to constrain the maximum search distance (`max_dist`) for the ray tracing by entering a valid maximum search distance value (in the same units as the X-Y coordinates of the input raster DEM). If the maximum search distance is left blank, each ray will be traced to the edge of the DEM, which will add to the computational time. 

Maximum upwind slope should not be calculated for very extensive areas over which the Earth's curvature must be taken into account. Also, this index does not take into account the deflection of wind by topography. However, averaging the horizon angle over a window of directions can yield a more robust measure of exposure, compensating for the deflection of wind from its regional average by the topography. For example, if you are interested in measuring the exposure of a landscape to a northerly wind, you could perform the following calculation:  

Sx(N) = [Sx(345)+Sx(350)+Sx(355)+Sx(0)+Sx(5)+Sx(10)+Sx(15)] / 7.0  

Ray-tracing is a highly computationally intensive task and therefore this tool may take considerable time to operate for larger sized DEMs. Maximum upwind slope is best displayed using a Grey scale palette that is inverted. 

Horizon angle is best visualized using a white-to-black palette and rescaled from approximately -10 to 70 (see below for an example of horizon angle calculated at a 150-degree azimuth). 

 

### See Also

 

`time_in_daylight` 

### Python API

```python
def horizon_angle(self, dem: Raster, azimuth: float = 0.0, max_dist: float = float('inf')) -> Raster:
```


---

## Horizon Area

**Function name:** `horizon_area`


PROExperimental

Calculates area of the horizon polygon (hectares).

geomorphometry terrain visibility


---

## Openness

**Function name:** `openness`


### Description

 

This tool calculates the Yokoyama et al. (2002) topographic openness index from an input DEM (`input`). Openness has two viewer perspectives, which correspond with positive and negative openness outputs (`pos_output` and `neg_output`). Positive values, expressing openness above the surface, are high for convex forms, whereas negative values describe this attribute below the surface and are high for concave forms. Openness is an angular value that is an average of the horizon angle in the eight cardinal directions to a maximum search distance (`dist`), measured in grid cells. Openness rasters are best visualized using a greyscale palette. 

Positive Openness:  

Negative Openness:  

### References

 

Yokoyama, R., Shirasawa, M., & Pike, R. J. (2002). Visualizing topography by openness: a new application of image processing to digital elevation models. Photogrammetric engineering and remote sensing, 68(3), 257-266. 

### See Also

 

`viewshed`, `horizon_angle`, `time_in_daylight`, `hillshade` 

### Python API

```python
def openness(self, dem: Raster, dist: int = 20) -> Tuple[Raster, Raster]:
```


---

## Shadow Animation

**Function name:** `shadow_animation`


PROExperimental

Creates an interactive HTML viewer and animated GIF showing terrain shadows throughout a day.

geomorphometry terrain visibility solar animation legacy-port

### Examples

*Generate a day-long shadow animation from a DEM or DSM.*
`wbe.shadow_animation(date='21/06/2021', dem='dsm.tif', location='43.5448/-80.2482/-4', output='shadow_animation.html')`


---

## Shadow Image

**Function name:** `shadow_image`


PROExperimental

Generates a terrain shadow intensity raster for a specified date, time, and location.

geomorphometry terrain visibility solar legacy-port


---

## Sky View Factor

**Function name:** `sky_view_factor`


This tool calculates the sky-view factor (SVF) from an input digital elevation model (DEM) or digital surface model (DSM). The SVF is the proportion of the celestial hemisphere above a point on the earth's surface that is not obstructed by the surrounding land surface. It is often used to model the diffuse light that is received at the surface and has also been applied as a relief-shading technique  (Böhner et al., 2009; Zakšek et al., 2011). 

 

The user must specify an input DEM (`dem`), the azimuth fraction (`az_fraction`), the maximum search distance (`max_dist`), and the height offset of the observer (`observer_hgt_offset`). The input DEM  should usually be a digital surface model (DSM) that contains significant off-terrain objects. Such a  model, for example, could be created using the first-return points of a LiDAR data set, or using the  `lidar_digital_surface_model` tool. The azimuth  fraction should be an even divisor of 360-degrees and must be between 1-45 degrees.  

The tool operates by calculating horizon angle (see `horizon_angle`)  rasters from the DSM based on the user-specified azimuth fraction (`az_fraction`). For example, if an azimuth  fraction of 15-degrees is specified, horizon angle rasters would be calculated for the solar azimuths 0,  15, 30, 45... A horizon angle raster evaluates the vertical angle between each grid cell in a DSM and a  distant obstacle (e.g. a mountain ridge, building, tree, etc.) that obscures the view in a specified  direction. In calculating horizon angle, the user must specify the maximum search distance (`max_dist`),  in map units, beyond which the query for higher, more distant objects will cease. This parameter strongly impacts the performance of the function, with larger values resulting in significantly longer processing-times.   

This tool uses the method described by Zakšek et al. (2011) to calculate SVF, which differs slightly from the method described by Böhner et al. (2009), as implemented in the Saga software. Most notably the Whitebox implementation does not involve local surface slope gradient and is closer in definition to the Saga 'Visible Sky' index.  

There are other significant differences between the Whitebox and Saga implementations of SVF. For a given maximum search distance, the Whitebox SVF will be substantially faster to calculate. Furthermore, the  Whitebox implementation has the ability to specify a height offset of the observer from the ground surface, using the `observer_hgt_offset` parameter. For example, the following image shows the spatial  pattern derived from a LiDAR DSM using `observer_hgt_offset = 0.0`: 

 

Notice that there are several places, plarticularly on the flatter rooftops, where the local noise in the LiDAR DEM, associated with the individual scan lines, has resulted in a somewhat noisy pattern  in the output. By adding a small height offset of the scale of this noise variation (0.15 m), we see  that most of this noisy pattern is removed in the output below: 

 

This feature makes the function more robust against DEM noise. As another example of the usefulness of this additional parameter, in the image below, the `observer_hgt_offset` parameter has been used to  measure the pattern of the index at a typical human height (1.7 m): 

 

Notice how overall visiblility increases at this height. 

### References

 

Böhner, J. and Antonić, O., 2009. Land-surface parameters specific to topo-climatology. Developments in soil science, 33, pp.195-226. 

Zakšek, K., Oštir, K. and Kokalj, Ž., 2011. Sky-view factor as a relief visualization technique. Remote sensing, 3(2), pp.398-415. 

### See Also

 

`average_horizon_distance`, `horizon_area`, `openness`, `lidar_digital_surface_model`, `horizon_angle` 

### Python API

```python
def sky_view_factor(self, dem: Raster, az_fraction: float = 5.0, max_dist: float = float('inf'), observer_hgt_offset: float = 0.0) -> Raster:
```


---

## Skyline Analysis

**Function name:** `skyline_analysis`


PROExperimental

Performs skyline analysis for one or more observation points and writes a vector horizon trace plus HTML report.

geomorphometry terrain visibility skyline


---

## Time In Daylight

**Function name:** `time_in_daylight`


This tool calculates the proportion of time a location is within daylight. That is, it calculates the proportion of time, during a user-defined time frame, that a grid cell in an input digital elevation model (`dem`) is outside of an area of shadow cast by a local object. The input DEM should truly be a digital surface model (DSM) that contains significant off-terrain objects. Such a model, for example, could be created using the first-return points of a LiDAR data set, or using the `lidar_digital_surface_model` tool. 

The tool operates by calculating a solar almanac, which estimates the sun's position for the location, in latitude and longitude coordinate (`lat`, `long`), of the input DSM. The algorithm then calculates horizon angle (see `horizon_angle`) rasters from the DSM based on the user-specified azimuth fraction (`az_fraction`). For example, if an azimuth fraction of 15-degrees is specified, horizon angle rasters could be calculated for the solar azimuths 0, 15, 30, 45... In reality, horizon angle rasters are only calculated for azimuths for which the sun is above the horizon for some time during the tested time period. A horizon angle raster evaluates the vertical angle between each grid cell in a DSM and a distant obstacle (e.g. a mountain ridge, building, tree, etc.) that blocks the view along a specified direction. In calculating horizon angle, the user must specify the maximum search distance (`max_dist`) beyond which the query for higher, more distant objects will cease. This parameter strongly impacts the performance of the tool, with larger values resulting in significantly longer run-times. Users are advised to set the `max_dist` based on the maximum shadow length expected in an area. For example, in a relatively flat urban landscape, the tallest building will likely determine the longest shadow lengths. All grid cells for which the calculated solar positions throughout the time frame are higher than the cell's horizon angle are deemed to be illuminated during the time the sun is in the corresponding azimuth fraction. 

By default, the tool calculates time-in-daylight for a time-frame spanning an entire year. That is, the solar almanac is calculated for each hour, at 10-second intervals, and for each day of the year. Users may alternatively restrict the time of year over which time-in-daylight is calculated by specifying a starting day (1-365; `start_day`) and ending day (1-365; `end_day`). Similarly, by specifying start time (`start_time`) and end time (`end_time`) parameters, the user is able to measure time-in-daylight for specific ranges of the day (e.g. for the morning or afternoon hours). These time parameters must be specified in 24-hour time (HH:MM:SS), e.g. 15:30:00. `sunrise` and `sunset` are also acceptable inputs for the start time and end time respectively. The timing of sunrise and sunset on each day in the tested time-frame will be determined using the solar almanac. 

 

### See Also

 

`lidar_digital_surface_model`, `horizon_angle` 

### Python API

```python
def time_in_daylight(self, dem: Raster, az_fraction: float = 5.0, max_dist: float = float('inf'), latitude: float = 0.0, longitude: float = 0.0, utc_offset_str: str = "UTC+00:00", start_day: int = 1, end_day: int = 365, start_time: str = "sunrise", end_time: str = "sunset") -> Raster:
```


---

## Viewshed

**Function name:** `viewshed`


This tool can be used to calculate the viewshed (i.e. the visible area) from a location (i.e. viewing station) or group of locations based on the topography defined by an input digital elevation model (DEM). The user must input a DEM (`dem`), a viewing station input vector file (`stations`) and the viewing height (`height`). Viewing station locations are specified as points within an input shapefile. The output image indicates the number of stations visible from each grid cell. The viewing height is in the same units as the elevations of the DEM and represent a height above the ground elevation from which the viewshed is calculated. 

`viewshed` should be used when there are a relatively small number of target sites for which visibility needs to be assessed. If you need to assess general landscape visibility as a land-surface parameter, the `visibility_index` tool should be used instead. 

Viewshed analysis is a very computationally intensive task. Depending on the size of the input DEM grid and the number of viewing stations, this operation may take considerable time to complete. Also, this implementation of the viewshed algorithm does not account for the curvature of the Earth. This should be accounted for if viewsheds are being calculated over very extensive areas. 

### See Also

 

`visibility_index` 

### Python API

```python
def viewshed(self, dem: Raster, station_points: Vector, station_height: float = 2.0) -> Raster:
```


---

## Visibility Index

**Function name:** `visibility_index`


This tool can be used to calculate a measure of landscape visibility based on the topography of an input digital elevation model (DEM). The user must input DEM a (`dem`),  the viewing height (`height`), and a resolution factor (`res_factor`). Viewsheds are calculated for a subset of grid cells in the DEM based on the resolution factor. The visibility index value (0.0-1.0) indicates the proportion of tested stations (determined by the resolution factor) that each cell is visible from. The viewing height is in the same units as the elevations of the DEM and represent a height above the ground elevation. Each tested grid cell's viewshed will be calculated in parallel. However, visibility index is one of the most computationally intensive geomorphometric indices to calculate. Depending on the size of the input DEM grid and the resolution factor, this operation may take considerable time to complete. If the task is too long-running, it is advisable to raise the resolution factor. A resolution factor of 2 will skip every second row and every second column (effectively evaluating the viewsheds of a quarter of the DEM's grid cells). Increasing this value decreases the number of calculated viewshed but will result in a lower accuracy estimate of overall visibility. In addition to the high computational costs of this index, the tool also requires substantial memory resources to operate. Each of these limitations should be considered before running this tool on a particular data set. This tool is best to apply on computer systems with high core-counts and plenty of memory. 

 

### See Also

 

`viewshed` 

### Python API

```python
def visibility_index(self, dem: Raster, station_height: float = 2.0, resolution_factor: int = 8) -> Raster:
```
