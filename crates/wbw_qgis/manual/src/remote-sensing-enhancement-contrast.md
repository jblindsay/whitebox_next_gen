# Image Enhancement and Contrast


---

## Balance Contrast Enhancement

**Function name:** `balance_contrast_enhancement`


This tool can be used to reduce colour bias in a colour composite image based on the technique described by Liu (1991). Colour bias is a common phenomena with colour images derived from multispectral imagery, whereby a higher average brightness value in one band results in over-representation of that band in the colour composite. The tool essentially applies a parabolic stretch to each of the three bands in a user specified RGB colour composite, forcing the histograms of each band to have the same minimum, maximum, and average values while maintaining their overall histogram shape. For greater detail on the operation of the tool, please see Liu (1991). Aside from the names of the input and output colour composite images, the user must also set the value of E, the desired output band mean, where 20 < E < 235. 

### Reference

 

Liu, J.G. (1991) Balance contrast enhancement technique and its application in image colour composition. *International Journal of Remote Sensing*, 12:10. 

### See Also

 

`direct_decorrelation_stretch`, `histogram_matching`, `histogram_matching_two_images`, `histogram_equalization`, `gaussian_contrast_stretch` 

### Python API

```python
def balance_contrast_enhancement(self, image: Raster, band_mean: float = 100.0) -> Raster:
```


---

## Create Colour Composite

**Function name:** `create_colour_composite`


This tool can be used to create a colour-composite image from three bands of multi-spectral imagery. The user must input images to enter into the red, green, and blue channels of the resulting composite image. The output image uses the 32-bit aRGB colour model, and therefore, in addition to red, green and blue bands, the user may optionally specify a fourth image that will be used to determine pixel opacity (the 'a' channel). If no opacity image is specified, each pixel will be opaque. This can be useful for cropping an image to an irregular-shaped boundary. The opacity channel can also be used to create transparent gradients in the composite image. 

A balance contrast enhancement (BCE) can optionally be performed on the bands prior to creation of the colour composite. While this operation will add to the runtime of `create_colour_composite`, if the individual input bands have not already had contrast enhancements, then it is advisable that the BCE option be used to improve the quality of the resulting colour composite image. 

NoData values in any of the input images are assigned NoData values in the output image and are not taken into account when performing the BCE operation. Please note, not all images have NoData values identified. When this is the case, and when the background value is 0 (often the case with multispectral imagery), then the `create_colour_composite` tool can be told to ignore zero values using the `zeros` flag. 

### See Also

 

`balance_contrast_enhancement`, `split_colour_composite` 

### Python API

```python
def create_colour_composite(self, red: Raster, green: Raster, blue: Raster, opacity: Raster = None, enhance: bool = True, treat_zeros_as_nodata: bool = False) -> Raster:
```


---

## Direct Decorrelation Stretch

**Function name:** `direct_decorrelation_stretch`


The Direct Decorrelation Stretch (DDS) is a simple type of saturation stretch. The stretch is applied to a colour composite image and is used to improve the saturation, or colourfulness, of the image. The DDS operates by reducing the achromatic (grey) component of a pixel's colour by a scale factor (*k*), such that the red (r), green (g), and blue (b) components of the output colour are defined as: 

r*k* = r - *k* min(r, g, b) 

g*k* = g - *k* min(r, g, b) 

b*k* = b - *k* min(r, g, b) 

The achromatic factor (*k*) can range between 0 (no effect) and 1 (full saturation stretch), although typical values range from 0.3 to 0.7. A linear stretch is used afterwards to adjust overall image brightness. Liu and Moore (1996) recommend applying a colour balance stretch, such as `balance_contrast_enhancement` before using the DDS. 

### Reference

 

Liu, J.G., and Moore, J. (1996) Direct decorrelation stretch technique for RGB colour composition. International Journal of Remote Sensing, 17:5, 1005-1018. 

### See Also

 

`create_colour_composite`, `balance_contrast_enhancement` 

### Python API

```python
def direct_decorrelation_stretch(self, image: Raster, achromatic_factor: float = 0.5, clip_percent: float = 1.0) -> Raster:
```


---

## False Colour Composite

**Function name:** `false_colour_composite`


*No help documentation available for this tool.*


---

## Gamma Correction

**Function name:** `gamma_correction`


This tool performs a gamma colour correction transform on an input image (`input`), such that each input pixel value (zin) is mapped to the corresponding output value (zout) as:  

zout = zin`gamma`  

The user must specify the value of the `gamma` parameter. The input image may be of either a greyscale or RGB colour composite data type. 

### Python API

```python
def gamma_correction(self, raster: Raster, gamma_value: float = 0.5) -> Raster:
```


---

## Gaussian Contrast Stretch

**Function name:** `gaussian_contrast_stretch`


This tool performs a Gaussian stretch on a raster image. The observed histogram of the input image is fitted to a Gaussian histogram, i.e. normal distribution. A histogram matching technique is used to map the values from the input image onto the output Gaussian distribution. The user must input the number of tones (`num_tones`) used. 

This tool is related to the more general `histogram_matching` tool, which can be used to fit any frequency distribution to an input image, and other contrast enhancement tools such as `histogram_equalization`, `min_max_contrast_stretch`, `percentage_contrast_stretch`, `sigmoidal_contrast_stretch`, and `standard_deviation_contrast_stretch`. 

### See Also

 

`piecewise_contrast_stretch`, `histogram_equalization`, `min_max_contrast_stretch`, `percentage_contrast_stretch`, `sigmoidal_contrast_stretch`, `standard_deviation_contrast_stretch`, `histogram_matching` 

### Python API

```python
def gaussian_contrast_stretch(self, raster: Raster, num_tones: int = 256) -> Raster:
```


---

## Histogram Equalization

**Function name:** `histogram_equalization`


This tool alters the cumulative distribution function (CDF) of a raster image to match, as closely as possible, the CDF of a uniform distribution. Histogram equalization works by first calculating the histogram of the input image. This input histogram is then converted into a CDF. Each grid cell value in the input image is then mapped to the corresponding value in the uniform distribution's CDF that has an equivalent (or as close as possible) cumulative probability value. Histogram equalization provides a very effective means of performing image contrast adjustment in an efficient manner with little need for human input. 

The user must specify the name of the input image to perform histogram equalization on. The user must also specify the number of tones, corresponding to the number of histogram bins used in the analysis. 

`histogram_equalization` is related to the `histogram_matching_two_images` tool (used when an image's CDF is to be matched to a reference CDF derived from a reference image). Similarly, `histogram_matching`, and `gaussian_contrast_stretch` are similarly related tools frequently used for image contrast adjustment, where the reference CDFs are uniform and Gaussian (normal) respectively. 

**Notes**: 
 
- The algorithm can introduces gaps in the histograms (steps in the CDF). This is to be expected because the histogram is being distorted. This is more prevalent for integer-level images. 
- Histogram equalization is not appropriate for images containing categorical (class) data. 
 

### See Also

 

`piecewise_contrast_stretch`, `histogram_matching`, `histogram_matching_two_images`, `gaussian_contrast_stretch` 

### Python API

```python
def histogram_equalization(self, raster: Raster, num_tones: int = 256) -> Raster:
```


---

## Histogram Matching

**Function name:** `histogram_matching`


This tool alters the cumulative distribution function (CDF) of a raster image to match, as closely as possible, the CDF of a reference histogram. Histogram matching works by first calculating the histogram of the input image. This input histogram and reference histograms are each then converted into CDFs. Each grid cell value in the input image is then mapped to the corresponding value in the reference CDF that has an equivalent (or as close as possible) cumulative probability value. Histogram matching provides the most flexible means of performing image contrast adjustment. 

The reference histogram must be specified to the tool in the form of a text file (.txt), provided using the `histo_file` flag. This file must contain two columns (delimited by a tab, space, comma, colon, or semicolon) where the first column contains the x value (i.e. the values that will be assigned to the grid cells in the output image) and the second column contains the frequency or probability. Note that 1) the file must not contain a header row, 2) each x value/frequency pair must be on a separate row. It is possible to create this type of histogram using the wide range of distribution tools available in most spreadsheet programs (e.g. Excel or LibreOffice's Calc program). You must save the file as a text-only (ASCII) file. 

`histogram_matching` is related to the `histogram_matching_two_images` tool, which can be used when a reference CDF can be derived from a reference image. `histogram_equalization` and `gaussian_contrast_stretch` are similarly related tools frequently used for image contrast adjustment, where the reference CDFs are uniform and Gaussian (normal) respectively. 

**Notes:** - The algorithm can introduces gaps in the histograms (steps in the CDF). This is to be expected because the histogram is being distorted. This is more prevalent for integer-level images. - Histogram matching is not appropriate for images containing categorical (class) data. - This tool is not intended for images containing RGB data. If this is the case, the colour channels should be split using the `split_colour_composite` tool. 

### See Also

 

`histogram_matching_two_images`, `histogram_equalization`, `gaussian_contrast_stretch`, `split_colour_composite` 

### Python API

```python
def histogram_matching(self, image: Raster, histogram: List[List[float]], histo_is_cumulative: bool = False) -> Raster:
```


---

## Histogram Matching Two Images

**Function name:** `histogram_matching_two_images`


This tool alters the cumulative distribution function (CDF) of a raster image to match, as closely as possible, the CDF of a reference image. Histogram matching works by first calculating the histograms of the input image (i.e. the image to be adjusted) and the reference image. These histograms are then converted into CDFs. Each grid cell value in the input image is then mapped to the corresponding value in the reference CDF that has the an equivalent (or as close as possible) cumulative probability value. A common application of this is to match the images from two sensors with slightly different responses, or images from the same sensor, but the sensor's response is known to change over time.The size of the two images (rows and columns) do not need to be the same, nor do they need to be geographically overlapping. 

`histogram_matching_two_images` is related to the `histogram_matching` tool, which can be used when a reference CDF is used directly rather than deriving it from a reference image. `histogram_equalization` and `gaussian_contrast_stretch` are similarly related tools, where the reference CDFs are uniform and Gaussian (normal) respectively. 

The algorithm may introduces gaps in the histograms (steps in the CDF). This is to be expected because the histograms are being distorted. This is more prevalent for integer-level images. Histogram matching is not appropriate for images containing categorical (class) data. It is also not intended for images containing RGB data, in which case, the colour channels should be split using the `split_colour_composite` tool. 

### See Also

 

`histogram_matching`, `histogram_equalization`, `gaussian_contrast_stretch`, `split_colour_composite` 

### Python API

```python
def histogram_matching_two_images(self, image1: Raster, image2: Raster) -> Raster:
```


---

## IHS To RGB

**Function name:** `ihs_to_rgb`


This tool transforms three intensity, hue, and saturation (IHS; sometimes HSI or HIS) raster images into three equivalent multispectral images corresponding with the red, green, and blue channels of an RGB composite. Intensity refers to the brightness of a color, hue is related to the dominant wavelength of light and is perceived as color, and saturation is the purity of the color (Koutsias et al., 2000). There are numerous algorithms for performing a red-green-blue (RGB) to IHS transformation. This tool uses the transformation described by Haydn (1982). Note that, based on this transformation, the input IHS values must follow the ranges:  

0 < I < 1 

0 < H < 2PI 

0 < S < 1  

The output red, green, and blue images will have values ranging from 0 to 255. The user must specify the names of the intensity, hue, and saturation images (`intensity`, `hue`, `saturation`). These images will generally be created using the `rgb_to_ihs` tool. The user must also specify the names of the output red, green, and blue images (`red`, `green`, `blue`). Image enhancements, such as contrast stretching, are often performed on the individual IHS components, which are then inverse transformed back in RGB components using this tool. The output RGB components can then be used to create an improved color composite image. 

### References

 

Haydn, R., Dalke, G.W. and Henkel, J. (1982) Application of the IHS color transform to the processing of multisensor data and image enhancement. Proc. of the Inter- national Symposium on Remote Sensing of Arid and Semiarid Lands, Cairo, 599-616. 

Koutsias, N., Karteris, M., and Chuvico, E. (2000). The use of intensity-hue-saturation transformation of Landsat-5 Thematic Mapper data for burned land mapping. Photogrammetric Engineering and Remote Sensing, 66(7), 829-840. 

### See Also

 

`rgb_to_ihs`, `balance_contrast_enhancement`, `direct_decorrelation_stretch` 

### Python API

```python
def ihs_to_rgb(self, intensity: Raster, hue: Raster, saturation: Raster) -> Tuple[Raster, Raster, Raster]:
```


---

## Min Max Contrast Stretch

**Function name:** `min_max_contrast_stretch`


This tool performs a Gaussian stretch on a raster image. The observed histogram of the input image is fitted to a Gaussian histogram, i.e. normal distribution. A histogram matching technique is used to map the values from the input image onto the output Gaussian distribution. The user must the number of tones (`num_tones`) used. 

This tool is related to the more general `histogram_matching` tool, which can be used to fit any frequency distribution to an input image, and other contrast enhancement tools such as `histogram_equalization`, `min_max_contrast_stretch`, `percentage_contrast_stretch`, `sigmoidal_contrast_stretch`, and `standard_deviation_contrast_stretch`. 

### See Also

 

`piecewise_contrast_stretch`, `histogram_equalization`, `min_max_contrast_stretch`, `percentage_contrast_stretch`, `sigmoidal_contrast_stretch`, `standard_deviation_contrast_stretch`, `histogram_matching` 

### Python API

```python
def min_max_contrast_stretch(self, raster: Raster, min_val: float, max_val: float, num_tones: int = 256) -> Raster:
```


---

## Mosaic

**Function name:** `mosaic`


This tool will create an image mosaic from one or more input image files using one of three resampling methods including, nearest neighbour, bilinear interpolation, and cubic convolution. The order of the input source image files is important. Grid cells in the output image will be assigned the corresponding value determined from the last image found in the list to possess an overlapping coordinate. 

Note that when the `inputs` parameter is left unspecified, the tool will use all of the *.tif*, *.tiff*, *.rdc*, *.flt*, *.sdat*, and *.dep* files located in the working directory. This can be a useful way of mosaicing large number of tiles, particularly when the text string that would be required to specify all of the input tiles is longer than the allowable limit. 

This is the preferred mosaicing tool to use when appending multiple images with little to no overlapping areas, e.g. tiled data. When images have significant overlap areas, users are advised to use the `mosaic_with_feathering` tool instead. 

Resample is very similar in operation to the Mosaic tool. The Resample tool should be used when there is an existing image into which you would like to dump information from one or more source images. If the source images are more extensive than the destination image, i.e. there are areas that extend beyond the destination image boundaries, these areas will not be represented in the updated image. Grid cells in the destination image that are not overlapping with any of the input source images will not be updated, i.e. they will possess the same value as before the resampling operation. The Mosaic tool is used when there is no existing destination image. In this case, a new image is created that represents the bounding rectangle of each of the two or more input images. Grid cells in the output image that do not overlap with any of the input images will be assigned the NoData value. 

### See Also

 

`mosaic_with_feathering` 

### Python API

```python
def mosaic(self, images: List[Raster], resampling_method: str = "cc") -> Raster:
```


---

## Mosaic With Feathering

**Function name:** `mosaic_with_feathering`


This tool will create a mosaic from two input images. It is similar in operation to the `mosaic` tool, however, this tool is the preferred method of mosaicing images when there is significant overlap between the images. For areas of overlap, the feathering method will calculate the output value as a weighted combination of the two input values, where the weights are derived from the squared distance of the pixel to the edge of the data in each of the input raster files. Therefore, less weight is assigned to an image's pixel value where the pixel is very near the edge of the image. Note that the distance is actually calculated to the edge of the grid and not necessarily the edge of the data, which can differ if the image has been rotated during registration.  The result of this feathering method is that the output mosaic image should have very little evidence of the original image edges within the overlapping area. 

Unlike the Mosaic tool, which can take multiple input images, this tool only accepts two input images. Mosaic is therefore useful when there are many, adjacent or only slightly overlapping images, e.g. for tiled data sets. 

Users may want to use the `histogram_matching` tool prior to mosaicing if the two input images differ significantly in their radiometric properties. i.e. if image contrast differences exist. 

### See Also

 

`mosaic`, `histogram_matching` 

### Python API

```python
def mosaic_with_feathering(self, image1: Raster, image2: Raster, resampling_method: str = "cc", distance_weight: float = 4.0) -> Raster:
```


---

## Normalized Difference Index

**Function name:** `normalized_difference_index`


This tool can be used to calculate a normalized difference index (NDI) from two bands of multispectral image data. A NDI of two band images (`image1` and `image2`) takes the general form:  

NDI = (image1 - image2) / (image1 + image2 + *c*)  

Where *c* is a correction factor sometimes used to avoid division by zero. It is, however, often set to 0.0. In fact, the `normalized_difference_index` tool will set all pixels where `image1 + image2 = 0` to 0.0 in the output image. While this is not strictly mathematically correct (0 / 0 = infinity), it is often the intended output in these cases. 

NDIs generally takes the value range -1.0 to 1.0, although in practice the range of values for a particular image scene may be more restricted than this. 

NDIs have two important properties that make them particularly useful for remote sensing applications. First, they emphasize certain aspects of the shape of the spectral signatures of different land covers. Secondly, they can be used to de-emphasize the effects of variable illumination within a scene. NDIs are therefore frequently used in the field of remote sensing to create vegetation indices and other indices for emphasizing various land-covers and as inputs to analytical operations like image classification. For example, the normalized difference vegetation index (NDVI), one of the most common image-derived products in remote sensing, is calculated as:  

NDVI = (NIR - RED) / (NIR + RED)  

The optimal soil adjusted vegetation index (OSAVI) is:  

OSAVI = (NIR - RED) / (NIR + RED + 0.16)  

The normalized difference water index (NDWI), or normalized difference moisture index (NDMI), is:  

NDWI = (NIR - SWIR) / (NIR + SWIR)  

The normalized burn ratio 1 (NBR1) and normalized burn ration 2 (NBR2) are:  

NBR1 = (NIR - SWIR2) / (NIR + SWIR2) 

NBR2 = (SWIR1 - SWIR2) / (SWIR1 + SWIR2)  

In addition to NDIs, *Simple Ratios* of image bands, are also commonly used as inputs to other remote sensing applications like image classification. Simple ratios can be calculated using the `Divide` tool. Division by zero, in this case, will result in an output NoData value. 

### See Also

 

`Divide` 

### Python API

```python
def normalized_difference_index(self, nir_image: Raster, red_image: Raster, clip_percent: float = 0.0, correction_value: float = 0.0) -> Raster:
```


---

## Panchromatic Sharpening

**Function name:** `panchromatic_sharpening`


Panchromatic sharpening, or simply pan-sharpening, refers to a range of techniques that can be used to merge finer spatial resolution panchromatic images with coarser spatial resolution multi-spectral images. The multi-spectral data provides colour information while the panchromatic image provides improved spatial information. This procedure is sometimes called image fusion. Jensen (2015) describes panchromatic sharpening in detail. 

Whitebox provides two common methods for panchromatic sharpening including the Brovey transformation and the Intensity-Hue-Saturation (IHS) methods. Both of these techniques provide the best results when the range of wavelengths detected by the panchromatic image overlap significantly with the wavelength range covered by the three multi-spectral bands that are used. When this is not the case, the resulting colour composite will likely have colour properties that are dissimilar to the colour composite generated by the original multispectral images. For Landsat ETM+ data, the panchromatic band is sensitive to EMR in the range of 0.52-0.90 micrometres. This corresponds closely to the green (band 2), red (band 3), and near-infrared (band 4). 

### Reference

 

Jensen, J. R. (2015). Introductory Digital Image Processing: A Remote Sensing Perspective. 

### See Also

 

`create_colour_composite` 

### Python API

```python
def panchromatic_sharpening(self, pan: Raster, colour_composite: Raster, red: Raster, green: Raster, blue: Raster, fusion_method: str = "brovey") -> Raster:
```


---

## Percentage Contrast Stretch

**Function name:** `percentage_contrast_stretch`


This tool performs a percentage contrast stretch on a raster image. This operation maps each grid cell value in the input raster image (zin) onto a new scale that ranges from a lower-tail clip value (`min_val`) to the upper-tail clip value (`max_val`), with the user-specified number of tonal values (`num_tones`), such that:  

zout = ((zin – min_val)/(max_val – min_val)) x num_tones  

where zout is the output value. The values of `min_val` and `max_val` are determined from the frequency distribution and the user-specified tail clip value (`clip`). For example, if a value of 1% is specified, the tool will determine the values in the input image for which 1% of the grid cells have a lower value `min_val` and 1% of the grid cells have a higher value `max_val`. The user must also specify which tails (upper, lower, or both) to clip (`tail`). 

This is a type of linear contrast stretch with saturation at the tails of the frequency distribution. This is the same kind of stretch that is used to display raster type data on the fly in many GIS software packages, such that the lower and upper tail values are set using the minimum and maximum display values and the number of tonal values is determined by the number of palette entries. 

### See Also

 

`piecewise_contrast_stretch`, `gaussian_contrast_stretch`, `histogram_equalization`, `min_max_contrast_stretch`, `sigmoidal_contrast_stretch`, `standard_deviation_contrast_stretch` 

### Python API

```python
def percentage_contrast_stretch(self, raster: Raster, clip: float = 1.0, tail: str = "both", num_tones: int = 256) -> Raster:
```


---

## Piecewise Contrast Stretch

**Function name:** `piecewise_contrast_stretch`


### Description

 

This tool can be used to perform a piecewise contrast stretch on an input image (`input`). The input image can be either a single-band image or a colour composite, in which case the contrast stretch will be performed on the intensity values of the hue-saturation-intensity (HSI) transform of the colour image. The user must also specify the name of the output image (`output`) and the break-points that define the piecewise function used to transfer brightness values from the input to the output image. The break-point values are specified as a string parameter (`function`), with each break-point taking the form of (input value, output proportion); (input value, output proportion); (input value, output proportion), etc. Piecewise functions can have as many break-points as desired, and each break-point should be separated by a semicolon (;). The input values are specifies as brightness values in the same units as the input image (unless it is an input colour composite, in which case the intensity values range from 0 to 1). The output function must be specified as a proportion (from 0 to 1) of the output value range, which is specified by the number of output greytones (`greytones`). The `greytones` parameter is ignored if the input image is a colour composite.  Note that there is no need to specify the initial break-point to the piecewise function, as (input min value; 0.0) will be inserted automatically. Similarly, an upper bound of the piecewise function of (input max value; 1.0) will also be inserted automatically. 

Generally you want to set breakpoints by examining the image histogram. Typically it is desirable to map large unpopulated ranges of input brightness values in the input image onto relatively narrow ranges of the output brightness values (i.e. a shallow sloped segment of the piecewise function), and areas of the histogram that are well populated with pixels in the input image with a larger range of brightness values in the output image (i.e. a steeper slope segment). This will have the effect of reducing the number of tones used to display the under-populated tails of the distribution and spreading out the well-populated regions of the histogram, thereby improving the overall contrast and the visual interpretability of the output image. The flexibility of the piecewise contrast stretch can often provide a very suitable means of significantly improving image quality. 

### See Also

 

`raster_histogram`, `gaussian_contrast_stretch`, `min_max_contrast_stretch`, `standard_deviation_contrast_stretch` 

### Python API

```python
def piecewise_contrast_stretch(self, raster: Raster, transformation_statement: str, num_greytones: float = 1024.0) -> Raster:
```


---

## Resample

**Function name:** `resample`


This tool can be used to modify the grid resolution of one or more rasters. The user specifies the names of one or more input rasters (`inputs`).  The resolution of the output raster is determined either using a specified `cell_size` parameter, in which case the output extent is determined by the combined extent of the inputs, or by an optional base raster (`base`), in which case the output raster spatial extent matches that of the base file. This operation is similar to the `mosaic` tool, except that `resample` modifies the output resolution. The `resample` tool may also be used with a single input raster (when the user wants to modify its spatial resolution, whereas, `mosaic` always includes multiple inputs. 

If the input source images are more extensive than the base image (if optionally specified), these areas will not be represented in the output image. Grid cells in the output image that are not overlapping with any of the input source images will not be assigned the NoData value, which will be the same as the first input image. Grid cells in the output image that overlap with multiple input raster cells will be assigned the last input value in the stack. Thus, the order of input images is important. 

### See Also

 

`mosaic` 

### Python API

```python
def resample(self, input_rasters: List[Raster], cell_size: float = 0.0, base_raster: Raster = None, method: str = "cc") -> Raster:
```


---

## RGB To IHS

**Function name:** `rgb_to_ihs`


This tool transforms three raster images of multispectral data (red, green, and blue channels) into their equivalent intensity, hue, and saturation (IHS; sometimes HSI or HIS) images. Intensity refers to the brightness of a color, hue is related to the dominant wavelength of light and is perceived as color, and saturation is the purity of the color (Koutsias et al., 2000). There are numerous algorithms for performing a red-green-blue (RGB) to IHS transformation. This tool uses the transformation described by Haydn (1982). Note that, based on this transformation, the output IHS values follow the ranges:  

0 < I < 1 

0 < H < 2PI 

0 < S < 1  

The user must specify the names of the red, green, and blue images (`red`, `green`, `blue`). Importantly, these images need not necessarily correspond with the specific regions of the electromagnetic spectrum that are red, green, and blue. Rather, the input images are three multispectral images that could be used to create a RGB color composite. The user must also specify the names of the output intensity, hue, and saturation images (`intensity`, `hue`, `saturation`). Image enhancements, such as contrast stretching, are often performed on the IHS components, which are then inverse transformed back in RGB components to then create an improved color composite image. 

### References

 

Haydn, R., Dalke, G.W. and Henkel, J. (1982) Application of the IHS color transform to the processing of multisensor data and image enhancement. Proc. of the Inter- national Symposium on Remote Sensing of Arid and Semiarid Lands, Cairo, 599-616. 

Koutsias, N., Karteris, M., and Chuvico, E. (2000). The use of intensity-hue-saturation transformation of Landsat-5 Thematic Mapper data for burned land mapping. Photogrammetric Engineering and Remote Sensing, 66(7), 829-840. 

### See Also

 

`ihs_to_rgb`, `balance_contrast_enhancement`, `direct_decorrelation_stretch` 

### Python API

```python
def rgb_to_ihs(self, red: Optional[Raster] = None, green: Optional[Raster] = None, blue: Optional[Raster] = None, composite: Optional[Raster] = None) -> Tuple[Raster, Raster, Raster]:
```


---

## Sigmoidal Contrast Stretch

**Function name:** `sigmoidal_contrast_stretch`


This tool performs a sigmoidal stretch on a raster image. This is a transformation where the input image value for a grid cell (zin) is transformed to an output value zout such that:  

zout = (1.0 / (1.0 + exp(*gain*(*cutoff* - z))) - *a* ) / *b* x *num_tones*  

where,  

z = (zin - *MIN*) / *RANGE*, 

*a* = 1.0 / (1.0 + exp(*gain* x *cutoff*)), 

*b* = 1.0 / (1.0 + exp(*gain* x (*cutoff* - 1.0))) - 1.0 / (1.0 + exp(*gain* x *cutoff*)),  

*MIN* and *RANGE* are the minimum value and data range in the input image respectively and *gain* and *cutoff* are user specified parameters (`gain`, `cutoff`). 

Like all of *WhiteboxTools*'s contrast enhancement tools, this operation will work on either greyscale or RGB input images. 

### See Also

 

`piecewise_contrast_stretch`, `gaussian_contrast_stretch`, `histogram_equalization`, `min_max_contrast_stretch`,  `percentage_contrast_stretch`, `standard_deviation_contrast_stretch` 

### Python API

```python
def sigmoidal_contrast_stretch(self, raster: Raster, cutoff: float = 0.0, gain: float = 1.0, num_tones: int = 256) -> Raster:
```


---

## Split Colour Composite

**Function name:** `split_colour_composite`


This tool can be used to split a red-green-blue (RGB) colour-composite image into three separate bands of multi-spectral imagery. The user must specify the input image (`input`) and output red, green, blue images. 

### See Also

 

`create_colour_composite` 

### Python API

```python
def split_colour_composite(self, composite_image: Raster) -> Tuple[Raster, Raster, Raster]:
```


---

## Standard Deviation Contrast Stretch

**Function name:** `standard_deviation_contrast_stretch`


This tool performs a standard deviation contrast stretch on a raster image. This operation maps each grid cell value in the input raster image (zin) onto a new scale that ranges from a lower-tail clip value (`min_val`) to the upper-tail clip value (`max_val`), with the user-specified number of tonal values (`num_tones`), such that:  

zout = ((zin – min_val)/(max_val – min_val)) x num_tones  

where zout is the output value. The values of `min_val` and `max_val` are determined based on the image mean and standard deviation. Specifically, the user must specify the number of standard deviations (`clip` or `stdev`) to be used in determining the min and max clip values. The tool will then calculate the input image mean and standard deviation and estimate the clip values from these statistics. 

This is the same kind of stretch that is used to display raster type data on the fly in many GIS software packages. 

### See Also

 

`piecewise_contrast_stretch`, `gaussian_contrast_stretch`, `histogram_equalization`, `min_max_contrast_stretch`,  `percentage_contrast_stretch`, `sigmoidal_contrast_stretch` 

### Python API

```python
def standard_deviation_contrast_stretch(self, raster: Raster, clip: float = 2.0, num_tones: int = 256) -> Raster:
```


---

## True Colour Composite

**Function name:** `true_colour_composite`


*No help documentation available for this tool.*
