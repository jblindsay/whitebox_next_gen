# Edge and Feature Detection


---

## Canny Edge Detection

**Function name:** `canny_edge_detection`


### Description

 

This tool performs a `Canny edge-detection` filtering operation on an input image (`input`). The Canny edge-detection filter is a multi-stage filter that combines a Gassian filtering (`gaussian_filter`) operation with various thresholding operations to generate a single-cell wide edges output raster (`output`). The `sigma` parameter, measured in grid cells determines the size of the Gaussian filter kernel. The `low` and `high` parameters determine the characteristics of the thresholding steps; both parameters range from 0.0 to 1.0. 

By default, the output raster will be Boolean, with 1's designating edge-cells. It is possible, using the `add_back` parameter to add the edge cells back into the original image, providing an edge-enchanced output, similar in concept to the `unsharp_masking` operation. 

 

### References

 

This implementation was inspired by the algorithm described here: `https://towardsdatascience.com/canny-edge-detection-step-by-step-in-python-computer-vision-b49c3a2d8123` 

### See Also

 

`gaussian_filter`, `sobel_filter`, `unsharp_masking`, `scharr_filter` 

### Python API

```python
def canny_edge_detection(self, input: Raster, sigma: float = 0.5, low_threshold: float = 0.05, high_threshold: float = 0.15, add_back_to_image: bool = False) -> Raster:
```


---

## Corner Detection

**Function name:** `corner_detection`


This tool identifies corner patterns in boolean images using hit-and-miss pattern matching. Foreground pixels in the input image (`input`) are designated by any positive, non-zero values. Zero-valued and NoData-valued grid cells are interpreted by the algorithm as background values. 

### Reference

 

Fisher, R, Brown, N, Cammas, N, Fitzgibbon, A, Horne, S, Koryllos, K, Murdoch, A, Robertson, J, Sharman, T, Strachan, C, 2004. Hypertext Image Processing Resource. online: http://homepages.inf.ed.ac.uk/rbf/HIPR2/hitmiss.htm 

### Python API

```python
def corner_detection(self, raster: Raster) -> Raster:
```


---

## Laplacian Filter

**Function name:** `laplacian_filter`


This tool can be used to perform a Laplacian filter on a raster image. A Laplacian filter can be used to emphasize the edges in an image. As such, this filter type is commonly used in edge-detection applications. The algorithm operates by convolving a kernel of weights with each grid cell and its neighbours in an image. Four 3x3 sized filters and one 5x5 filter are available for selection. The weights of the kernels are as follows: 

3x3(1)  ... 0-10 -14-1 0-10   

3x3(2)  ... 0-10 -15-1 0-10   

3x3(3)  ... -1-1-1 -18-1 -1-1-1   

3x3(4)  ... 1-21 -24-2 1-21   

5x5(1)  ..... 00-100 0-1-2-10 -1-217-2-1 0-1-2-10 00-100   

5x5(2)  ..... 00-100 0-1-2-10 -1-216-2-1 0-1-2-10 00-100   

The user must specify the `variant`, including '3x3(1)', '3x3(2)', '3x3(3)', '3x3(4)', '5x5(1)', and '5x5(2)'. The user may also optionally clip the output image distribution tails by a specified amount (e.g. 1%). 

### See Also

 

`prewitt_filter`, `sobel_filter` 

### Python API

```python
def laplacian_filter(self, raster: Raster, variant: str = "3x3(1)", clip_amount: float = 0.0) -> Raster:
```


---

## Laplacian Of Gaussians Filter

**Function name:** `laplacian_of_gaussians_filter`


The Laplacian-of-Gaussian (LoG) is a spatial filter used for edge enhancement and is closely related to the difference-of-Gaussians filter (`DiffOfGaussianFilter`). The formulation of the LoG filter algorithm is based on the equation provided in the Hypermedia Image Processing Reference (HIPR) 2. The LoG operator calculates the second spatial derivative of an image. In areas where image intensity is constant, the LoG response will be zero. Near areas of change in intensity the LoG will be positive on the darker side, and negative on the lighter side. This means that at a sharp edge, or boundary, between two regions of uniform but different intensities, the LoG response will be: 
 
- zero at a long distance from the edge, 
- positive just to one side of the edge, 
- negative just to the other side of the edge, 
- zero at some point in between, on the edge itself. 
 

The user may optionally choose to reflecting the data along image edges. **NoData** values in the input image are similarly valued in the output. The output raster is of the float data type and continuous data scale. 

### Reference

 

Fisher, R. 2004. *Hypertext Image Processing Resources 2 (HIPR2)*. Available online: http://homepages.inf.ed.ac.uk/rbf/HIPR2/roberts.htm 

### See Also

 

`DiffOfGaussianFilter` 

### Python API

```python
def laplacian_of_gaussians_filter(self, raster: Raster, sigma: float = 0.75) -> Raster:
```


---

## Prewitt Filter

**Function name:** `prewitt_filter`


This tool performs a 3 &times; 3 Prewitt edge-detection filter on a raster image. The Prewitt filter is similar to the `sobel_filter`, in that it identifies areas of high slope in the input image through the calculation of slopes in the x and y directions. The Prewitt edge-detection filter, however, gives less weight to nearer cell values within the moving window, or kernel. For example, a Prewitt filter uses the following schemes to calculate x and y slopes: 

X-direction slope  ... -101 -101 -101   

Y-direction slope  ... 111 000 -1-1-1   

Each grid cell in the output image is assigned the square-root of the squared sum of the x and y slopes. 

The user may optionally clip the output image distribution tails by a specified amount (e.g. 1%). 

### See Also

 

`sobel_filter` 

### Python API

```python
def prewitt_filter(self, raster: Raster, clip_tails: float = 0.0) -> Raster:
```


---

## Roberts Cross Filter

**Function name:** `roberts_cross_filter`


This tool performs Robert's Cross edge-detection filter on a raster image. The `roberts_cross_filter` is similar to the `sobel_filter` and `prewitt_filter`, in that it identifies areas of high slope in the input image through the calculation of slopes in the x and y directions. A Robert's Cross filter uses the following 2 &times; 2 schemes to calculate slope magnitude, |*G*|:  .. P1P2 P3P4    *G*=P1 - P4+P2- P3   

Note, the filter is centered on pixel P1 and P2, P3, and P4 are the neighbouring pixels towards the east, south, and south-east respectively. 

The output image may be overwhelmed by a relatively small number of high-valued pixels, stretching the palette. The user may therefore optionally clip the output image distribution tails by a specified amount (`clip`) for improved visualization. 

### Reference

 

Fisher, R. 2004. *Hypertext Image Processing Resources 2 (HIPR2)*. Available online: http://homepages.inf.ed.ac.uk/rbf/HIPR2/roberts.htm 

### See Also

 

`sobel_filter`, `prewitt_filter` 

### Python API

```python
def roberts_cross_filter(self, raster: Raster, clip_amount: float = 0.0) -> Raster:
```


---

## Sobel Filter

**Function name:** `sobel_filter`


This tool performs a 3 &times; 3 or 5 &times; 5 Sobel edge-detection filter on a raster image. The Sobel filter is similar to the `prewitt_filter`, in that it identifies areas of high slope in the input image through the calculation of slopes in the x and y directions. The Sobel edge-detection filter, however, gives more weight to nearer cell values within the moving window, or kernel. For example, a 3 &times; 3 Sobel filter uses the following schemes to calculate x and y slopes: 

X-direction slope  ... -101 -202 -101   

Y-direction slope  ... 121 000 -1-2-1   

Each grid cell in the output image is assigned the square-root of the squared sum of the x and y slopes. 

The user must specify the `variant`, including '3x3' and '5x5' variants. The user may also optionally clip the output image distribution tails by a specified amount (e.g. 1%). 

### See Also

 

`prewitt_filter` 

### Python API

```python
def sobel_filter(self, raster: Raster, variant: str = "3x3", clip_tails: float = 0.0) -> Raster:
```
