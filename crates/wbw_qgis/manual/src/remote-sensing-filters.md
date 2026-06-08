# Image Filters


---

## Adaptive Filter

**Function name:** `adaptive_filter`


This tool performs a type of adaptive filter on a raster image. An adaptive filter can be used to reduce the level of random noise (shot noise) in an image. The algorithm operates by calculating the average value in a moving window centred on each grid cell. If the absolute difference between the window mean value and the centre grid cell value is beyond a user-defined threshold (`threshold`), the grid cell in the output image is assigned the mean value, otherwise it is equivalent to the original value. Therefore, the algorithm only modifies the image where grid cell values are substantially different than their neighbouring values. 

Neighbourhood size, or filter size, is specified in the x and y dimensions using `filterx` and `filtery`.  These dimensions should be odd, positive integer values (e.g. 3, 5, 7, 9, etc.). 

### See Also

 

`mean_filter` 

### Python API

```python
def adaptive_filter(self, raster: Raster, filter_size_x: int = 11, filter_size_y: int = 11, threshold: float = 2.0) -> Raster:
```


---

## Anisotropic Diffusion Filter

**Function name:** `anisotropic_diffusion_filter`


Experimental

Performs Perona-Malik edge-preserving anisotropic diffusion smoothing.

remote_sensing raster filter anisotropic_diffusion_filter legacy-port

### Parameters

NameDescriptionRequiredDefault
`input`Input raster path or typed raster object.Required`input.tif`
`iterations`Number of diffusion iterations (default 10).Optional`10`
`kappa`Edge sensitivity parameter (default 20.0).Optional`20.0`
`lambda`Time-step in (0, 0.25], default 0.2.Optional`0.2`
`output`Optional output path. If omitted, output remains in memory.Optional—

### Examples

*Applies anisotropic_diffusion_filter to an input raster.*
`wbe.anisotropic_diffusion_filter(input='image.tif', output='anisotropic_diffusion_filter.tif')`


---

## Bilateral Filter

**Function name:** `bilateral_filter`


This tool can be used to perform an edge-preserving smoothing filter, or bilateral filter, on an image. A bilateral filter can be used to emphasize the longer-range variability in an image, effectively acting to smooth the image, while reducing the edge blurring effect common with other types of smoothing filters. As such, this filter is very useful for reducing the noise in an image. Bilateral filtering is a non-linear filtering technique introduced by Tomasi and Manduchi (1998). The algorithm operates by convolving a kernel of weights with each grid cell and its neighbours in an image. The bilateral filter is related to Gaussian smoothing, in that the weights of the convolution kernel are partly determined by the 2-dimensional Gaussian (i.e. normal) curve, which gives stronger weighting to cells nearer the kernel centre. Unlike the `gaussian_filter`, however, the bilateral kernel weightings are also affected by their similarity to the intensity value of the central pixel. Pixels that are very different in intensity from the central pixel are weighted less, also based on a Gaussian weight distribution. Therefore, this non-linear convolution filter is determined by the spatial and intensity domains of a localized pixel neighborhood. 

The heavier weighting given to nearer and similar-valued pixels makes the bilateral filter an attractive alternative for image smoothing and noise reduction compared to the much-used Mean filter. The size of the filter is determined by setting the standard deviation distance parameter (`sigma_dist`); the larger the standard deviation the larger the resulting filter kernel. The standard deviation can be any number in the range 0.5-20 and is specified in the unit of pixels. The standard deviation intensity parameter (`sigma_int`), specified in the same units as the z-values, determines the intensity domain contribution to kernel weightings. 

### References

 

Tomasi, C., & Manduchi, R. (1998, January). Bilateral filtering for gray and color images. In null (p. 839). IEEE. 

### See Also

 

`edge_preserving_mean_filter` 

### Python API

```python
def bilateral_filter(self, raster: Raster, sigma_dist: float = 0.75, sigma_int: float = 1.0) -> Raster:
```


---

## Closing

**Function name:** `closing`


This tool performs a closing operation on an input greyscale image (`input`). A `closing` is a mathematical morphology operation involving an erosion (minimum filter) of a dilation (maximum filter) set. `closing` operations, together with the `opening` operation, is frequently used in the fields of computer vision and digital image processing for image noise removal. The user must specify the size of the moving window in both the x and y directions (`filterx` and `filtery`). 

### See Also

 

`opening`, `tophat_transform` 

### Python API

```python
def closing(self, raster: Raster, filter_size_x: int = 11, filter_size_y: int = 11) -> Raster:
```


---

## Conservative Smoothing Filter

**Function name:** `conservative_smoothing_filter`


This tool performs a conservative smoothing filter on a raster image. A conservative smoothing filter can be used to remove short-range variability in an image, effectively acting to smooth the image. It is particularly useful for eliminating local spikes and reducing the noise in an image. The algorithm operates by calculating the minimum and maximum neighbouring values surrounding a grid cell. If the cell at the centre of the kernel is greater than the calculated maximum value, it is replaced with the maximum value in the output image. Similarly, if the cell value at the kernel centre is less than the neighbouring minimum value, the corresponding grid cell in the output image is replaced with the minimum value. This filter tends to alter an image very little compared with other smoothing filters such as the `mean_filter`, `edge_preserving_mean_filter`, `bilateral_filter`, `median_filter`, `gaussian_filter`, or `olympic_filter`. 

Neighbourhood size, or filter size, is specified in the x and y dimensions using the `filterx` and `filtery` flags. These dimensions should be odd, positive integer values (e.g. 3, 5, 7, 9, etc.). 

### See Also

 

`mean_filter`, `edge_preserving_mean_filter`, `bilateral_filter`, `median_filter`, `gaussian_filter`, `olympic_filter` 

### Python API

```python
def conservative_smoothing_filter(self, raster: Raster, filter_size_x: int = 3, filter_size_y: int = 3) -> Raster:
```


---

## Diff Of Gaussians Filter

**Function name:** `diff_of_gaussians_filter`


This tool can be used to perform a difference-of-Gaussians (DoG) filter on a raster image. In digital image processing, DoG is a feature enhancement algorithm that involves the subtraction of one blurred version of an image from another, less blurred version of the original. The blurred images are obtained by applying filters with Gaussian-weighted kernels of differing standard deviations to the input image (`input`). Blurring an image using a Gaussian-weighted kernel suppresses high-frequency spatial information and emphasizes lower-frequency variation. Subtracting one blurred image from the other preserves spatial information that lies between the range of frequencies that are preserved in the two blurred images. Thus, the difference-of-Gaussians is a band-pass filter that discards all but a specified range of spatial frequencies that are present in the original image. 

The algorithm operates by differencing the results of convolving two kernels of weights with each grid cell and its neighbours in an image. The weights of the convolution kernels are determined by the 2-dimensional Gaussian (i.e. normal) curve, which gives stronger weighting to cells nearer the kernel centre. The size of the two convolution kernels are determined by setting the two standard deviation parameters (`sigma1` and `sigma2`); the larger the standard deviation the larger the resulting filter kernel. The second standard deviation should be a larger value than the first, however if this is not the case, the tool will automatically swap the two parameters. Both standard deviations can range from 0.5-20. 

The difference-of-Gaussians filter can be used to emphasize edges present in an image. Other edge-sharpening filters also operate by enhancing high-frequency detail, but because random noise also has a high spatial frequency, many of these sharpening filters tend to enhance noise, which can be an undesirable artifact. The difference-of-Gaussians filter can remove high-frequency noise while emphasizing edges. This filter can, however, reduce overall image contrast. 

### See Also

 

`gaussian_filter`, `fast_almost_gaussian_filter`, `laplacian_filter`, LaplacianOfGaussianFilter` 

### Python API

```python
def diff_of_gaussians_filter(self, raster: Raster, sigma1: float = 2.0, sigma2: float = 4.0) -> Raster:
```


---

## Diversity Filter

**Function name:** `diversity_filter`


This tool assigns each cell in the output grid the number of different values in a moving window centred on each grid cell in the input raster. The input image should contain integer values but floating point data are allowable and will be handled by multiplying pixel values by 1000 and rounding. Neighbourhood size, or filter size, is specified in the x and y dimensions using the `filterx` and `filtery` flags. These dimensions should be odd, positive integer values, e.g. 3, 5, 7, 9... If the kernel filter size is the same in the x and y dimensions, the silent `filter` flag may be used instead (command-line interface only). 

### See Also

 

`majority_filter` 

### Python API

```python
def diversity_filter(self, raster: Raster, filter_size_x: int = 11, filter_size_y: int = 11) -> Raster:
```


---

## Edge Preserving Mean Filter

**Function name:** `edge_preserving_mean_filter`


This tool performs a type of edge-preserving mean filter operation on an input image (`input`). The filter, a type of low-pass filter, can be used to emphasize the longer-range variability in an image, effectively acting to smooth the image and to reduce noise in the image. The algorithm calculates the average value in a moving window centred on each grid cell, including in the averaging only the set of neighbouring values for which the absolute value difference with the centre value is less than a specified threshold value (`threshold`). It is, therefore, similar to the `bilateral_filter`, except all neighbours within the threshold difference are equally weighted and neighbour distance is not accounted for. Filter kernels are always square, and filter size, is specified using the `filter` parameter. This dimensions should be odd, positive integer values, e.g. 3, 5, 7, 9... 

This tool works with both greyscale and red-green-blue (RGB) input images. RGB images are decomposed into intensity-hue-saturation (IHS) and the filter is applied to the intensity channel. If an RGB image is input, the threshold value must be in the range 0.0-1.0 (more likely less than 0.15), where a value of 1.0 would result in an ordinary mean filter (`mean_filter`). NoData values in the input image are ignored during filtering. 

### See Also

 

`mean_filter`, `bilateral_filter`, `edge_preserving_mean_filter`, `gaussian_filter`, `median_filter`, `rgb_to_ihs` 

### Python API

```python
def edge_preserving_mean_filter(self, raster: Raster, filter_size: int = 11, threshold: float = 15.0) -> Raster:
```


---

## Emboss Filter

**Function name:** `emboss_filter`


This tool can be used to perform one of eight 3x3 emboss filters on a raster image. Like the `sobel_filter` and `prewitt_filter`, the `emboss_filter` is often applied in edge-detection applications. While these other two common edge-detection filters approximate the slope magnitude of the local neighbourhood surrounding each grid cell, the `emboss_filter` can be used to estimate the directional slope. The kernel weights for each of the eight available filters are as follows: 

North (`n`)  ... 0-10 000 010   

Northeast (`ne`)  ... 00-1 000 -100    

East (`e`)  ... 000 10-1 000   

Southeast (`se`)  ... 100 000 00-1   

South (`s`)  ... 010 000 0-10   

Southwest (`sw`)  ... 001 000 -100   

West (`w`)  ... 000 -101 000   

Northwest (`nw`)  ... -100 000 001   

The user must specify the `direction`, options include 'n', 's', 'e', 'w', 'ne', 'se', 'nw', 'sw'. The user may also optionally clip the output image distribution tails by a specified amount (e.g. 1%). 

### See Also

 

`sobel_filter`, `prewitt_filter` 

### Python API

```python
def emboss_filter(self, raster: Raster, direction: str = "n", clip_amount: float = 0.0) -> Raster:
```


---

## Fast Almost Gaussian Filter

**Function name:** `fast_almost_gaussian_filter`


The tool is somewhat modified from Dr. Kovesi's original Matlab code in that it works with both greyscale and RGB images (decomposes to HSI and uses the intensity data) and it handles the case of rasters that contain NoData values. This adds complexity to the original 20 additions and 5 multiplications assertion of the original paper. 

Also note, for small values of sigma (< 1.8), you should probably just use the regular GaussianFilter tool. 

### Reference

 

P. Kovesi 2010 Fast Almost-Gaussian Filtering, Digital Image Computing: Techniques and Applications (DICTA), 2010 International Conference on. 

### Python API

```python
def fast_almost_gaussian_filter(self, raster: Raster, sigma: float = 1.8) -> Raster:
```


---

## Flip Image

**Function name:** `flip_image`


This tool can be used to flip, or reflect, an image (`input`) either vertically, horizontally, or both. The axis of reflection is specified using the `direction` parameter. The input image is not reflected in place; rather, the reflected image is stored in a separate output file. 

### Python API

```python
def flip_image(self, raster: Raster, direction: str = "v") -> Raster:
```


---

## Frangi Filter

**Function name:** `frangi_filter`


Experimental

Performs multiscale Frangi vesselness enhancement.

remote_sensing raster filter frangi_filter legacy-port

### Parameters

NameDescriptionRequiredDefault
`input`Input raster path or typed raster object.Required`input.tif`
`scales`List of Gaussian-like scales in pixels (default [1.0, 2.0, 3.0]).Optional`[1.0, 2.0, 3.0]`
`beta`Frangi beta parameter for blob suppression (default 0.5).Optional`0.5`
`c`Frangi c parameter for structure sensitivity (default 15.0).Optional`15.0`
`output`Optional output path. If omitted, output remains in memory.Optional—

### Examples

*Applies frangi_filter to an input raster.*
`wbe.frangi_filter(input='image.tif', output='frangi_filter.tif')`


---

## Gabor Filter Bank

**Function name:** `gabor_filter_bank`


Experimental

Performs multi-orientation Gabor response filtering.

remote_sensing raster filter gabor_filter_bank legacy-port

### Parameters

NameDescriptionRequiredDefault
`input`Input raster path or typed raster object.Required`input.tif`
`sigma`Gaussian envelope sigma in pixels (default 2.0).Optional`2.0`
`frequency`Sinusoid spatial frequency in cycles/pixel (default 0.2).Optional`0.2`
`orientations`Number of orientations in the filter bank (default 6).Optional`6`
`output`Optional output path. If omitted, output remains in memory.Optional—

### Examples

*Applies gabor_filter_bank to an input raster.*
`wbe.gabor_filter_bank(input='image.tif', output='gabor_filter_bank.tif')`


---

## Gaussian Filter

**Function name:** `gaussian_filter`


This tool can be used to perform a Gaussian filter on a raster image. A Gaussian filter can be used to emphasize the longer-range variability in an image, effectively acting to smooth the image. This can be useful for reducing the noise in an image. The algorithm operates by convolving a kernel of weights with each grid cell and its neighbours in an image. The weights of the convolution kernel are determined by the 2-dimensional Gaussian (i.e. normal) curve, which gives stronger weighting to cells nearer the kernel centre. It is this characteristic that makes the Gaussian filter an attractive alternative for image smoothing and noise reduction than the `mean_filter`. The size of the filter is determined by setting the standard deviation parameter (`sigma`), which is in units of grid cells; the larger the standard deviation the larger the resulting filter kernel. The standard deviation can be any number in the range 0.5-20. 

`gaussian_filter` works with both greyscale and red-green-blue (RGB) colour images. RGB images are decomposed into intensity-hue-saturation (IHS) and the filter is applied to the intensity channel. NoData values in the input image are ignored during processing. 

Like many low-pass filters, Gaussian filtering can significantly blur well-defined edges in the input image. The `edge_preserving_mean_filter` and `bilateral_filter` offer more robust feature preservation during image smoothing. `gaussian_filter` is relatively slow compared to the `fast_almost_gaussian_filter` tool, which offers a fast-running approximatation to a Gaussian filter for larger kernel sizes. 

### See Also

 

`fast_almost_gaussian_filter`, `mean_filter`, `median_filter`, `rgb_to_ihs` 

### Python API

```python
def gaussian_filter(self, raster: Raster, sigma: float = 0.75) -> Raster:
```


---

## GLCM Texture

**Function name:** `glcm_texture`


### Description

Computes general-purpose local texture metrics from a single-band raster using a gray-level co-occurrence matrix (GLCM) within a moving window. Output is written as a multiband raster so that large metric sets remain manageable in Python/R APIs and QGIS.

Use `features` to choose which metrics are emitted. Supported feature names are `contrast`, `dissimilarity`, `homogeneity`, `asm`, `energy`, `entropy`, `mean`, `variance`, and `correlation`. Use `direction_aggregation` to combine directions (`mean`, `min`, `max`, `range`) or keep each direction as separate output bands (`separate`).

Angles are specified in degrees using a comma-separated list from `0,45,90,135`. Increasing `window_size` and `levels` generally improves stability at higher computational cost.

### Python API

```python
def glcm_texture(self, input: Raster, window_size: int = 7, distance: int = 1, angles: str = "0,45,90,135", features: str = "contrast,homogeneity,energy,entropy", direction_aggregation: str = "mean", levels: int = 32, symmetric: bool = True) -> Raster:
```

### Example

`glcm = wbe.glcm_texture(
    input=raster,
    window_size=9,
    distance=1,
    angles="0,45,90,135",
    features="contrast,homogeneity,entropy",
    direction_aggregation="mean",
    levels=32,
    output="glcm_texture.tif",
)`

### See Also

`image_segmentation`, `object_features_texture_glcm_basic`


---

## Guided Filter

**Function name:** `guided_filter`


Experimental

Performs edge-preserving guided filtering using local linear models.

remote_sensing raster filter guided_filter legacy-port

### Parameters

NameDescriptionRequiredDefault
`input`Input raster path or typed raster object.Required`input.tif`
`radius`Guided filter window radius in pixels (default 4).Optional`4`
`epsilon`Regularization parameter for local variance (default 0.01).Optional`0.01`
`output`Optional output path. If omitted, output remains in memory.Optional—

### Examples

*Applies guided_filter to an input raster.*
`wbe.guided_filter(input='image.tif', output='guided_filter.tif')`


---

## High Pass Bilateral Filter

**Function name:** `high_pass_bilateral_filter`


Experimental

Computes a high-pass residual by subtracting bilateral smoothing from the input raster.

raster image filter high-pass legacy-port

### Parameters

NameDescriptionRequiredDefault
`input`Input raster path or typed raster object.Required`input.tif`
`sigma_dist`Standard deviation of the spatial (distance) Gaussian kernel, in pixels (0.5–20.0, default 0.75).Optional`0.75`
`sigma_int`Standard deviation of the intensity Gaussian kernel, in raster-value units (default 1.0).Optional`1.0`
`treat_as_rgb`Set true to force HSI-intensity filtering for packed RGB rasters before high-pass differencing.Optional`False`
`assume_three_band_rgb`When true (default), and no explicit color metadata is present, allow 3-band uint8/uint16 RGB interpretation.Optional`True`
`output`Optional output file path. If omitted, output remains in memory.Optional—

### Examples

*Applies high-pass bilateral filtering to emphasize local texture.*
`wbe.high_pass_bilateral_filter(assume_three_band_rgb=True, input='image.tif', output='image_highpass_bilateral.tif', sigma_dist=1.5, sigma_int=25.0, treat_as_rgb=False)`


---

## High Pass Filter

**Function name:** `high_pass_filter`


This tool performs a high-pass filter on a raster image. High-pass filters can be used to emphasize the short-range variability in an image. The algorithm operates essentially by subtracting the value at the grid cell at the centre of the window from the average value in the surrounding neighbourhood (i.e. window.) 

Neighbourhood size, or filter size, is specified in the x and y dimensions using the `filterx` and `filtery` flags. These dimensions should be odd, positive integer values (e.g. 3, 5, 7, 9, etc.). 

### See Also

 

`high_pass_median_filter`, `mean_filter` 

### Python API

```python
def high_pass_filter(self, raster: Raster, filter_size_x: int = 11, filter_size_y: int = 11) -> Raster:
```


---

## High Pass Median Filter

**Function name:** `high_pass_median_filter`


This tool performs a high-pass median filter on a raster image. High-pass filters can be used to emphasize the short-range variability in an image. The algorithm operates essentially by subtracting the value at the grid cell at the centre of the window from the median value in the surrounding neighbourhood (i.e. window.) 

Neighbourhood size, or filter size, is specified in the x and y dimensions using the `filterx` and `filtery` flags. These dimensions should be odd, positive integer values (e.g. 3, 5, 7, 9, etc.). 

### See Also

 

`high_pass_filter`, `median_filter` 

### Python API

```python
def high_pass_median_filter(self, raster: Raster, filter_size_x: int = 11, filter_size_y: int = 11, sig_digits: int = 2) -> Raster:
```


---

## Integral Image Transform

**Function name:** `integral_image_transform`


This tool transforms an input raster image into an integral image, or summed area table. Integral images are the two-dimensional equivalent to a cumulative distribution function. Each pixel contains the sum of all pixels contained within the enclosing rectangle above and to the left of a pixel. Images with a very large number of grid cells will likely experience numerical overflow errors when converted to an integral image. Integral images are used in a wide variety of computer vision and digital image processing applications, including texture mapping. They allow for the efficient calculation of very large filters and are the basis of several of *WhiteboxTools*'s image filters. 

### Reference

 

Crow, F. C. (1984, January). Summed-area tables for texture mapping. In ACM SIGGRAPH computer graphics (Vol. 18, No. 3, pp. 207-212). ACM. 

### Python API

```python
def integral_image_transform(self, raster: Raster) -> Raster:
```


---

## K Nearest Mean Filter

**Function name:** `k_nearest_mean_filter`


This tool performs a k-nearest mean filter on a raster image. A mean filter can be used to emphasize the longer-range variability in an image, effectively acting to smooth or blur the image. This can be useful for reducing the noise in an image. The algorithm operates by calculating the average of a specified number (*k*) values in a moving window centred on each grid cell. The *k* values used in the average are those cells in the window with the nearest intensity values to that of the centre cell. As such, this is a type of edge-preserving smoothing filter. The `bilateral_filter` and `edge_preserving_mean_filter` are examples of more sophisticated edge-preserving smoothing filters. 

Neighbourhood size, or filter size, is specified in the x and y dimensions using the `filterx` and `filtery` flags. These dimensions should be odd, positive integer values (e.g. 3, 5, 7, 9, etc.). 

NoData values in the input image are ignored during filtering. 

### See Also

 

`mean_filter`, `bilateral_filter`, `edge_preserving_mean_filter` 

### Python API

```python
def k_nearest_mean_filter(self, raster: Raster, filter_size_x: int = 3, filter_size_y: int = 3, k: int = 5) -> Raster:
```


---

## Kuwahara Filter

**Function name:** `kuwahara_filter`


Experimental

Performs edge-preserving Kuwahara filtering using minimum-variance subwindows.

remote_sensing raster filter kuwahara_filter legacy-port

### Parameters

NameDescriptionRequiredDefault
`input`Input raster path or typed raster object.Required`input.tif`
`radius`Kuwahara quadrant radius in pixels (default 2).Optional`2`
`output`Optional output path. If omitted, output remains in memory.Optional—

### Examples

*Applies kuwahara_filter to an input raster.*
`wbe.kuwahara_filter(input='image.tif', output='kuwahara_filter.tif')`


---

## Lee Filter

**Function name:** `lee_filter`


The Lee Sigma filter is a low-pass filter used to smooth the input image (`input`). The user must specify the dimensions of the filter (`filterx` and `filtery`) as well as the *sigma* (`sigma`) and *M* (`m`) parameter. 

### Reference

 

Lee, J. S. (1983). Digital image smoothing and the sigma filter. Computer vision, graphics, and image processing, 24(2), 255-269. 

### See Also

 

`mean_filter`, `gaussian_filter` 

### Python API

```python
def lee_filter(self, raster: Raster, filter_size_x: int = 11, filter_size_y: int = 11, sigma: float = 10.0, m_value: float = 5.0) -> Raster:
```


---

## Line Detection Filter

**Function name:** `line_detection_filter`


This tool can be used to perform one of four 3x3 line-detection filters on a raster image. These filters can be used to find one-cell-thick vertical, horizontal, or angled (135-degrees or 45-degrees) lines in an image. Notice that line-finding is a similar application to edge-detection. Common edge-detection filters include the Sobel and Prewitt filters. The kernel weights for each of the four line-detection filters are as follows: 

'v' (Vertical)  ... -12-1 -12-1 -12-1   

'h' (Horizontal)  ... -1-1-1 222 -1-1-1   

'45' (Northeast-Southwest)  ... -1-12 -12-1 2-1-1   

'135' (Northwest-Southeast)  ... 2-1-1 -12-1 -1-12   

The user must specify the `variant`, including 'v', 'h', '45', and '135', for vertical, horizontal, northeast-southwest, and northwest-southeast directions respectively. The user may also optionally clip the output image distribution tails by a specified amount (e.g. 1%). 

### See Also

 

`prewitt_filter`, `sobel_filter` 

### Python API

```python
def line_detection_filter(self, raster: Raster, variant: str = "v", abs_values: bool = False, clip_tails: float = 0.0) -> Raster:
```


---

## Line Thinning

**Function name:** `line_thinning`


This image processing tool reduces all polygons in a Boolean raster image to their single-cell wide skeletons. This operation is sometimes called line thinning or skeletonization. In fact, the input image need not be truly Boolean (i.e. contain only 1's and 0's). All non-zero, positive values are considered to be foreground pixels while all zero valued cells are considered background pixels. The `remove_spurs` tool is useful for cleaning up an image before performing a line thinning operation. 

Note: Unlike other filter-based operations in *WhiteboxTools*, this algorithm can't easily be parallelized because the output raster must be read and written to during the same loop. 

### See Also

 

`remove_spurs`, `thicken_raster_line` 

### Python API

```python
def line_thinning(self, raster: Raster) -> Raster:
```


---

## LiDAR Ground Point Filter

**Function name:** `lidar_ground_point_filter`


This tool can be used to perform a slope-based classification, or filtering (i.e. removal), of non-ground points within a LiDAR point-cloud. The user must specify the name of the input and output LiDAR files (`input` and `output`). Inter-point slopes are compared between pair of points contained within local neighbourhoods of size `radius`. Neighbourhoods with fewer than the user-specified minimum number of points (`min_neighbours`) are extended until the minimum point number is equaled or exceeded. Points that are above neighbouring points by the minimum (`height_threshold`) and have an inter-point slope greater than the user-specifed threshold (`slope_threshold`) are considered non-ground points and are either optionally (`classify`) excluded from the output point-cloud or assigned the *unclassified* (value 1) class value. 

Slope-based ground-point classification methods suffer from the challenge of uses a constant slope threshold under varying terrain slopes. Some researchers have developed schemes for varying the slope threshold based on underlying terrain slopes. `lidar_ground_point_filter` instead allow the user to optionally (`slope_norm`) normalize the underlying terrain (i.e. flatten the terrain) using a white top-hat transform. A constant slope threshold may then be used without contributing to poorer performance under steep topography. Note, that this option, while useful in rugged terrain, is computationally intensive. If the point-cloud is of a relatively flat terrain, this option may be excluded. 

While this tool is appropriately applied to LiDAR point-clouds, the `remove_off_terrain_objects` tool can be used to remove off-terrain objects from rasterized LiDAR digital elevation models (DEMs). 

### Reference

 

Vosselman, G. (2000). Slope based filtering of laser altimetry data. *International Archives of Photogrammetry and Remote Sensing*, 33(B3/2; PART 3), 935-942. 

### See Also

 

`improved_ground_point_filter`, `remove_off_terrain_objects` 

### Python API

```python
def lidar_ground_point_filter(self, input_lidar: Optional[Lidar], search_radius: float = 2.0, min_neighbours: int = 0, slope_threshold: float = 45.0, height_threshold: float = 1.0, classify: bool = False, slope_norm: bool = True, height_above_ground: bool = False) -> Lidar:
```


---

## Majority Filter

**Function name:** `majority_filter`


This tool performs a range filter on an input image (`input`). A range filter assigns to each cell in the output grid. The range (maximum - minimum) of the values contained within a moving window centred on each grid cell. 

Neighbourhood size, or filter size, is specified in the x and y dimensions using the `filterx` and `filtery` flags. These dimensions should be odd, positive integer values (e.g. 3, 5, 7, 9, etc.). 

### See Also

 

`total_filter` 

### Python API

```python
def majority_filter(self, raster: Raster, filter_size_x: int = 11, filter_size_y: int = 11) -> Raster:
```


---

## Maximum Filter

**Function name:** `maximum_filter`


This tool assigns each cell in the output grid. The maximum value in a moving window centred on each grid cell in the input raster (`input`). A maximum filter is the equivalent of the mathematical morphological `dilation` operator. 

Neighbourhood size, or filter size, is specified in the x and y dimensions using the `filterx` and `filtery` flags. These dimensions should be odd, positive integer values, e.g. 3, 5, 7, 9... If the kernel filter size is the same in the x and y dimensions, the silent `filter` flag may be used instead (command-line interface only). 

This tool takes advantage of the redundancy between overlapping, neighbouring filters to enhance computationally efficiency. Like most of WhiteboxTools' filters, it is also parallelized for further efficiency. 

### See Also

 

`minimum_filter` 

### Python API

```python
def maximum_filter(self, raster: Raster, filter_size_x: int = 11, filter_size_y: int = 11) -> Raster:
```


---

## Mean Filter

**Function name:** `mean_filter`


This tool performs a mean filter operation on a raster image. A mean filter, a type of low-pass filter, can be used to emphasize the longer-range variability in an image, effectively acting to smooth the image. This can be useful for reducing the noise in an image. This tool utilizes an integral image approach (Crow, 1984) to ensure highly efficient filtering that is invariant to filter size. The algorithm operates by calculating the average value in a moving window centred on each grid cell.  Neighbourhood size, or filter size, is specified in the x and y dimensions using the `filterx` and `filtery` flags. These dimensions should be odd, positive integer values, e.g. 3, 5, 7, 9... If the kernel filter size is the same in the x and y dimensions, the silent `filter` flag may be used instead (command-line interface only). 

Although commonly applied in digital image processing, mean filters are generally considered to be quite harsh, with respect to their impact on the image, compared to other smoothing filters such as the edge-preserving smoothing filters including the `bilateral_filter`, `median_filter`, `olympic_filter`, `edge_preserving_mean_filter` and even `gaussian_filter`. 

This tool works with both greyscale and red-green-blue (RGB) images. RGB images are decomposed into intensity-hue-saturation (IHS) and the filter is applied to the intensity channel. NoData values in the input image are ignored during filtering. NoData values are assigned to all sites beyond the raster. 

### Reference

 

Crow, F. C. (1984, January). Summed-area tables for texture mapping. In ACM SIGGRAPH computer graphics (Vol. 18, No. 3, pp. 207-212). ACM. 

### See Also

 

`bilateral_filter`, `edge_preserving_mean_filter`, `gaussian_filter`, `median_filter`, `rgb_to_ihs` 

### Python API

```python
def mean_filter(self, raster: Raster, filter_size_x: int = 11, filter_size_y: int = 11) -> Raster:
```


---

## Median Filter

**Function name:** `median_filter`


This tool performs a median filter on a raster image. Median filters, a type of low-pass filter, can be used to emphasize the longer-range variability in an image, effectively acting to smooth the image. This can be useful for reducing the noise in an image. The algorithm operates by calculating the median value (middle value in a sorted list) in a moving window centred on each grid cell. Specifically, this tool uses the efficient running-median filtering algorithm of Huang et al. (1979). The median value is not influenced by anomolously high or low values in the distribution to the extent that the average is. As such, the median filter is far less sensitive to shot noise in an image than the mean filter. 

Neighbourhood size, or filter size, is specified in the x and y dimensions using the `filterx` and `filtery`flags. These dimensions should be odd, positive integer values (e.g. 3, 5, 7, 9, etc.). 

### Reference

 

Huang, T., Yang, G.J.T.G.Y. and Tang, G., 1979. A fast two-dimensional median filtering algorithm. IEEE Transactions on Acoustics, Speech, and Signal Processing, 27(1), pp.13-18. 

### See Also

 

`bilateral_filter`, `edge_preserving_mean_filter`, `gaussian_filter`, `mean_filter` 

### Python API

```python
def median_filter(self, raster: Raster, filter_size_x: int = 11, filter_size_y: int = 11, sig_digits: int = 2) -> Raster:
```


---

## Minimum Filter

**Function name:** `minimum_filter`


This tool assigns each cell in the output grid the minimum value in a moving window centred on each grid cell in the input raster (`input`). A maximum filter is the equivalent of the mathematical morphological `erosion` operator. 

Neighbourhood size, or filter size, is specified in the x and y dimensions using the `filterx` and `filtery` flags. These dimensions should be odd, positive integer values, e.g. 3, 5, 7, 9... If the kernel filter size is the same in the x and y dimensions, the silent `filter` flag may be used instead (command-line interface only). 

This tool takes advantage of the redundancy between overlapping, neighbouring filters to enhance computationally efficiency. Like most of WhiteboxTools' filters, it is also parallelized for further efficiency. 

### See Also

 

`maximum_filter` 

### Python API

```python
def minimum_filter(self, raster: Raster, filter_size_x: int = 11, filter_size_y: int = 11) -> Raster:
```


---

## Non Local Means Filter

**Function name:** `non_local_means_filter`


Experimental

Performs non-local means denoising using patch similarity weighting.

remote_sensing raster filter non_local_means_filter legacy-port

### Parameters

NameDescriptionRequiredDefault
`input`Input raster path or typed raster object.Required`input.tif`
`search_radius`Search window radius in pixels (default 5).Optional`5`
`patch_radius`Patch radius in pixels (default 1).Optional`1`
`h`Filtering strength parameter (default 10.0).Optional`10.0`
`output`Optional output path. If omitted, output remains in memory.Optional—

### Examples

*Applies non_local_means_filter to an input raster.*
`wbe.non_local_means_filter(input='image.tif', output='non_local_means_filter.tif')`


---

## Opening

**Function name:** `opening`


This tool performs an opening operation on an input greyscale image (`input`). An `opening` is a mathematical morphology operation involving a dilation (maximum filter) on an erosion (minimum filter) set. `opening` operations, together with the `closing` operation, is frequently used in the fields of computer vision and digital image processing for image noise removal. The user must specify the size of the moving window in both the x and y directions (`filterx` and `filtery`). 

### See Also

 

`closing`, `tophat_transform` 

### Python API

```python
def opening(self, raster: Raster, filter_size_x: int = 11, filter_size_y: int = 11) -> Raster:
```


---

## Olympic Filter

**Function name:** `olympic_filter`


This filter is a modification of the `mean_filter`, whereby the highest and lowest values in the kernel are dropped, and the remaining values are averaged to replace the central pixel. The result is a low-pass smoothing filter that is more robust than the `mean_filter`, which is more strongly impacted by the presence of outlier values. It is named after a system of scoring Olympic events. 

Neighbourhood size, or filter size, is specified in the x and y dimensions using the `filterx` and `filtery` flags. These dimensions should be odd, positive integer values (e.g. 3, 5, 7, 9, etc.). 

### See Also

 

`mean_filter` 

### Python API

```python
def olympic_filter(self, raster: Raster, filter_size_x: int = 11, filter_size_y: int = 11) -> Raster:
```


---

## Percentile Filter

**Function name:** `percentile_filter`


This tool calculates the percentile of the center cell in a moving filter window applied to an input image (`input). This indicates the value below which a given percentage of the neighbouring values in within the filter fall. For example, the 35th percentile is the value below which 35% of the neighbouring values in the filter window may be found. As such, the percentile of a pixel value is indicative of the relative location of the site within the statistical distribution of values contained within a filter window. When applied to input digital elevation models, percentile is a measure of local topographic position, or elevation residual. 

Neighbourhood size, or filter size, is specified in the x and y dimensions using the `filterx` and `filtery` flags. These dimensions should be odd, positive integer values, e.g. 3, 5, 7, 9... If the kernel filter size is the same in the x and y dimensions, the silent `filter` flag may be used instead (command-line interface only). 

This tool takes advantage of the redundancy between overlapping, neighbouring filters to enhance computationally efficiency, using a method similar to Huang et al. (1979). This efficient method of calculating percentiles requires rounding of floating-point inputs, and therefore the user must specify the number of significant digits (`sig_digits`) to be used during the processing. Like most of WhiteboxTools' filters, this tool is also parallelized for further efficiency. 

### Reference

 

Huang, T., Yang, G.J.T.G.Y. and Tang, G., 1979. A fast two-dimensional median filtering algorithm. IEEE Transactions on Acoustics, Speech, and Signal Processing, 27(1), pp.13-18. 

### See Also

 

`median_filter` 

### Python API

```python
def percentile_filter(self, raster: Raster, filter_size_x: int = 11, filter_size_y: int = 11, sig_digits: int = 2) -> Raster:
```


---

## Range Filter

**Function name:** `range_filter`


This tool performs a range filter on an input image (`input`). A range filter assigns to each cell in the output grid the range (maximum - minimum) of the values contained within a moving window centred on each grid cell. 

Neighbourhood size, or filter size, is specified in the x and y dimensions using the `filterx` and `filtery` flags. These dimensions should be odd, positive integer values (e.g. 3, 5, 7, 9, etc.). 

### See Also

 

`total_filter` 

### Python API

```python
def range_filter(self, raster: Raster, filter_size_x: int = 11, filter_size_y: int = 11) -> Raster:
```


---

## Remove Spurs

**Function name:** `remove_spurs`


This image processing tool removes small irregularities (i.e. spurs) on the boundaries of objects in a Boolean input raster image (`input`). This operation is sometimes called *pruning*. Remove Spurs is a useful tool for cleaning an image before performing a line thinning operation. In fact, the input image need not be truly Boolean (i.e. contain only 1's and 0's). All non-zero, positive values are considered to be foreground pixels while all zero valued cells are considered background pixels. 

Note: Unlike other filter-based operations in *WhiteboxTools*, this algorithm can't easily be parallelized because the output raster must be read and written to during the same loop. 

### See Also

 

`line_thinning` 

### Python API

```python
def remove_spurs(self, raster: Raster, max_iterations: int = 10) -> Raster:
```


---

## Savitzky Golay 2D Filter

**Function name:** `savitzky_golay_2d_filter`


Experimental

Performs 2D Savitzky-Golay smoothing.

remote_sensing raster filter savitzky_golay_2d_filter legacy-port

### Parameters

NameDescriptionRequiredDefault
`input`Input raster path or typed raster object.Required`input.tif`
`window_size`Odd window size (default 5). Currently supports 5 for polynomial order 2.Optional`5`
`output`Optional output path. If omitted, output remains in memory.Optional—

### Examples

*Applies savitzky_golay_2d_filter to an input raster.*
`wbe.savitzky_golay_2d_filter(input='image.tif', output='savitzky_golay_2d_filter.tif')`


---

## Scharr Filter

**Function name:** `scharr_filter`


This tool performs a Scharr edge-detection filter on a raster image. The Scharr filter is similar to the `sobel_filter` and `prewitt_filter`, in that it identifies areas of high slope in the input image through the calculation of slopes in the x and y directions. A 3 &times; 3 Scharr filter uses the following schemes to calculate x and y slopes: 

X-direction slope  ... 30-3 100-10 30-3   

Y-direction slope  ... 3103 000 -3-10-3   

Each grid cell in the output image is assigned the square-root of the squared sum of the x and y slopes. 

The output image may be overwhelmed by a relatively small number of high-valued pixels, stretching the palette. The user may therefore optionally clip the output image distribution tails by a specified amount (`clip`) for improved visualization. 

### See Also

 

`sobel_filter`, `prewitt_filter` 

### Python API

```python
def scharr_filter(self, raster: Raster, clip_tails: float = 0.0) -> Raster:
```


---

## Standard Deviation Filter

**Function name:** `standard_deviation_filter`


This tool performs a standard deviation filter on an input image (`input`). A standard deviation filter assigns to each cell in the output grid the `standard deviation`, a measure of dispersion, of the values contained within a moving window centred on each grid cell. 

Neighbourhood size, or filter size, is specified in the x and y dimensions using the `filterx` and `filtery` flags. These dimensions should be odd, positive integer values (e.g. 3, 5, 7, 9, etc.). 

### See Also

 

`range_filter`, `total_filter` 

### Python API

```python
def standard_deviation_filter(self, raster: Raster, filter_size_x: int = 11, filter_size_y: int = 11) -> Raster:
```


---

## Thicken Raster Line

**Function name:** `thicken_raster_line`


This image processing tool can be used to thicken single-cell wide lines within a raster file along diagonal sections of the lines. Because of the limitation of the raster data format, single-cell wide raster lines can be traversed along diagonal sections without passing through a line grid cell. This causes problems for various raster analysis functions for which lines are intended to be barriers. This tool will thicken raster lines, such that it is impossible to cross a line without passing through a line grid cell. While this can also be achieved using a maximum filter, unlike the filter approach, this tool will result in the smallest possible thickening to achieve the desired result. 

All non-zero, positive values are considered to be foreground pixels while all zero valued cells or NoData cells are considered background pixels. 

Note: Unlike other filter-based operations in *WhiteboxTools*, this algorithm can't easily be parallelized because the output raster must be read and written to during the same loop. 

### See Also

 

`line_thinning` 

### Python API

```python
def thicken_raster_line(self, raster: Raster) -> Raster:
```


---

## Tophat Transform

**Function name:** `tophat_transform`


This tool performs either a white or black `top-hat transform` on an input image. A top-hat transform is a common digital image processing operation used for various tasks, such as feature extraction, background equalization, and image enhancement. The size of the rectangular *structuring element* used in the filtering can be specified using the `filterx` and `filtery` flags. 

There are two distinct types of top-hat transform including *white* and *black* top-hat transforms. The white top-hat transform is defined as the difference between the input image and its `opening` by some structuring element. An opening operation is the `dilation` (maximum filter) of an `erosion` (minimum filter) image. The black top-hat transform, by comparison, is defined as the difference between the `closing` and the input image. The user specifies which of the two flavours of top-hat transform the tool should perform by specifying either 'white' or 'black' with the `variant` flag. 

### See Also:

 

`closing`, `opening`, `maximum_filter`, `minimum_filter` 

### Python API

```python
def tophat_transform(self, raster: Raster, filter_size_x: int = 11, filter_size_y: int = 11, variant: str = "white") -> Raster:
```


---

## Total Filter

**Function name:** `total_filter`


This tool performs a total filter on an input image. A total filter assigns to each cell in the output grid the total (sum) of all values in a moving window centred on each grid cell. 

Neighbourhood size, or filter size, is specified in the x and y dimensions using the `filterx` and `filtery` flags. These dimensions should be odd, positive integer values (e.g. 3, 5, 7, 9, etc.). 

### See Also

 

`range_filter` 

### Python API

```python
def total_filter(self, raster: Raster, filter_size_x: int = 11, filter_size_y: int = 11) -> Raster:
```


---

## Unsharp Masking

**Function name:** `unsharp_masking`


Unsharp masking is an image edge-sharpening technique commonly applied in digital image processing. Admittedly, the name 'unsharp' seems somewhat counter-intuitive given the purpose of the filter, which is to enchance the definition of edge features within the input image (`input`). This name comes from the use of a blurred, or unsharpened, intermediate image (mask) in the process. The blurred image is combined with the positive (original) image, creating an image that exhibits enhanced feature definition. A caution is needed in that the output image, although clearer, may be a less accurate representation of the image's subject. The output may also contain more speckle than the input image. 

In addition to the input (`input`) and output image files, the user must specify the values of three parameters: the standard deviation distance (`sigma`), which is a measure of the filter size in pixels, the amount (`amount`), a percentage value that controls the magnitude of each overshoot at edges, and lastly, the threshold (`threshold`), which controls the minimal brightness change that will be sharpened. Pixels with values differ after the calculation of the filter by less than the threshold are unmodified in the output image. 

`unsharp_masking` works with both greyscale and red-green-blue (RGB) colour images. RGB images are decomposed into intensity-hue-saturation (IHS) and the filter is applied to the intensity channel. Importantly, the intensity values range from 0-1, which is important when setting the threshold value for colour images. NoData values in the input image are ignored during processing. 

### See Also

 

`gaussian_filter`, `high_pass_filter` 

### Python API

```python
def unsharp_masking(self, raster: Raster, sigma: float = 0.75, amount: float = 100.0, threshold: float = 0.0) -> Raster:
```


---

## User Defined Weights Filter

**Function name:** `user_defined_weights_filter`


NoData values in the input image are ignored during the convolution operation. This can lead to unexpected behavior at the edges of images (since the default behavior is to return NoData when addressing cells beyond the grid edge) and where the grid contains interior areas of NoData values. Normalization of kernel weights can be useful for handling the edge effects associated with interior areas of NoData values. When the normalization option is selected, the sum of the cell value-weight product is divided by the sum of the weights on a cell-by-cell basis. Therefore, if the kernel at a particular grid cell contains neighboring cells of NoData values, normalization effectively re-adjusts the weighting to account for the missing data values. Normalization also ensures that the output image will possess values within the range of the input image and allows the user to specify integer value weights in the kernel. However, note that this implies that the sum of weights should equal one. In some cases, alternative sums (e.g. zero) are more appropriate, and as such normalization should not be applied in these cases. 

### Python API

```python
def user_defined_weights_filter(self, raster: Raster, weights: List[List[float]], kernel_center: str = "center", normalize_weights: bool = False) -> Raster:
```


---

## Wiener Filter

**Function name:** `wiener_filter`


Experimental

Performs adaptive Wiener denoising using local mean and variance.

remote_sensing raster filter wiener_filter legacy-port

### Parameters

NameDescriptionRequiredDefault
`input`Input raster path or typed raster object.Required`input.tif`
`radius`Wiener local window radius in pixels (default 2).Optional`2`
`noise_variance`Optional additive noise variance. If omitted, estimated from local variance map.Optional—
`output`Optional output path. If omitted, output remains in memory.Optional—

### Examples

*Applies wiener_filter to an input raster.*
`wbe.wiener_filter(input='image.tif', output='wiener_filter.tif')`
