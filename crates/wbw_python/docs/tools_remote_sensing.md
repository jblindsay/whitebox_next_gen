# Whitebox Workflows for Python — Remote Sensing Tools

This document covers all **Remote Sensing** tools exposed through the `WbEnvironment` API.
For common conventions, Raster I/O, and math operators see [TOOLS.md](../TOOLS.md).

---

## Remote Sensing

These tools are grouped under the `remote_sensing` tool module in the backend.
Image filters are currently the first subset documented here; additional non-filter
remote sensing tools will be added to this same section as they are ported.

### Tools (Alphabetical)

- [`wbe.adaptive_filter`](#wbeadaptive_filter)
- [`wbe.anisotropic_diffusion_filter`](#wbeanisotropic_diffusion_filter)
- [`wbe.balance_contrast_enhancement`](#wbebalance_contrast_enhancement)
- [`wbe.bilateral_filter`](#wbebilateral_filter)
- [`wbe.canny_edge_detection`](#wbecanny_edge_detection)
- [`wbe.conservative_smoothing_filter`](#wbeconservative_smoothing_filter)
- [`wbe.change_vector_analysis`](#wbechange_vector_analysis)
- [`wbe.closing`](#wbeclosing)
- [`wbe.corner_detection`](#wbecorner_detection)
- [`wbe.correct_vignetting`](#wbecorrect_vignetting)
- [`wbe.create_colour_composite`](#wbecreate_colour_composite)
- [`wbe.diff_of_gaussians_filter`](#wbediff_of_gaussians_filter)
- [`wbe.direct_decorrelation_stretch`](#wbedirect_decorrelation_stretch)
- [`wbe.diversity_filter`](#wbediversity_filter)
- [`wbe.edge_preserving_mean_filter`](#wbeedge_preserving_mean_filter)
- [`wbe.emboss_filter`](#wbeemboss_filter)
- [`wbe.evaluate_training_sites`](#wbeevaluate_training_sites)
- [`wbe.fast_almost_gaussian_filter`](#wbefast_almost_gaussian_filter)
- [`wbe.flip_image`](#wbeflip_image)
- [`wbe.frost_filter`](#wbefrost_filter)
- [`wbe.frangi_filter`](#wbefrangi_filter)
- [`wbe.fuzzy_knn_classification`](#wbefuzzy_knn_classification)
- [`wbe.gabor_filter_bank`](#wbegabor_filter_bank)
- [`wbe.generalize_classified_raster`](#wbegeneralize_classified_raster)
- [`wbe.generalize_with_similarity`](#wbegeneralize_with_similarity)
- [`wbe.gaussian_contrast_stretch`](#wbegaussian_contrast_stretch)
- [`wbe.gaussian_filter`](#wbegaussian_filter)
- [`wbe.gamma_correction`](#wbegamma_correction)
- [`wbe.gamma_map_filter`](#wbegamma_map_filter)
- [`wbe.guided_filter`](#wbeguided_filter)
- [`wbe.high_pass_filter`](#wbehigh_pass_filter)
- [`wbe.high_pass_bilateral_filter`](#wbehigh_pass_bilateral_filter)
- [`wbe.high_pass_median_filter`](#wbehigh_pass_median_filter)
- [`wbe.histogram_equalization`](#wbehistogram_equalization)
- [`wbe.histogram_matching`](#wbehistogram_matching)
- [`wbe.histogram_matching_two_images`](#wbehistogram_matching_two_images)
- [`wbe.ihs_to_rgb`](#wbeihs_to_rgb)
- [`wbe.k_means_clustering`](#wbek_means_clustering)
- [`wbe.k_nearest_mean_filter`](#wbek_nearest_mean_filter)
- [`wbe.knn_classification`](#wbeknn_classification)
- [`wbe.knn_regression`](#wbeknn_regression)
- [`wbe.logistic_regression`](#wbelogistic_regression)
- [`wbe.random_forest_classification`](#wberandom_forest_classification)
- [`wbe.random_forest_classification_fit`](#wberandom_forest_classification_fit)
- [`wbe.random_forest_classification_predict`](#wberandom_forest_classification_predict)
- [`wbe.random_forest_regression`](#wberandom_forest_regression)
- [`wbe.random_forest_regression_fit`](#wberandom_forest_regression_fit)
- [`wbe.random_forest_regression_predict`](#wberandom_forest_regression_predict)
- [`wbe.kuan_filter`](#wbekuan_filter)
- [`wbe.kuwahara_filter`](#wbekuwahara_filter)
- [`wbe.image_slider`](#wbeimage_slider)
- [`wbe.image_segmentation`](#wbeimage_segmentation)
- [`wbe.integral_image_transform`](#wbeintegral_image_transform)
- [`wbe.image_stack_profile`](#wbeimage_stack_profile)
- [`wbe.laplacian_filter`](#wbelaplacian_filter)
- [`wbe.laplacian_of_gaussians_filter`](#wbelaplacian_of_gaussians_filter)
- [`wbe.lee_filter`](#wbelee_filter)
- [`wbe.line_detection_filter`](#wbeline_detection_filter)
- [`wbe.line_thinning`](#wbeline_thinning)
- [`wbe.majority_filter`](#wbemajority_filter)
- [`wbe.maximum_filter`](#wbemaximum_filter)
- [`wbe.mean_filter`](#wbemean_filter)
- [`wbe.median_filter`](#wbemedian_filter)
- [`wbe.min_max_contrast_stretch`](#wbemin_max_contrast_stretch)
- [`wbe.min_dist_classification`](#wbemin_dist_classification)
- [`wbe.minimum_filter`](#wbeminimum_filter)
- [`wbe.modified_k_means_clustering`](#wbemodified_k_means_clustering)
- [`wbe.mosaic`](#wbemosaic)
- [`wbe.mosaic_with_feathering`](#wbemosaic_with_feathering)
- [`wbe.non_local_means_filter`](#wbenon_local_means_filter)
- [`wbe.nnd_classification`](#wbennd_classification)
- [`wbe.normalized_difference_index`](#wbenormalized_difference_index)
- [`wbe.opening`](#wbeopening)
- [`wbe.olympic_filter`](#wbeolympic_filter)
- [`wbe.otsu_thresholding`](#wbeotsu_thresholding)
- [`wbe.panchromatic_sharpening`](#wbepanchromatic_sharpening)
- [`wbe.parallelepiped_classification`](#wbeparallelepiped_classification)
- [`wbe.percentage_contrast_stretch`](#wbepercentage_contrast_stretch)
- [`wbe.piecewise_contrast_stretch`](#wbepiecewise_contrast_stretch)
- [`wbe.percentile_filter`](#wbepercentile_filter)
- [`wbe.prewitt_filter`](#wbeprewitt_filter)
- [`wbe.range_filter`](#wberange_filter)
- [`wbe.rgb_to_ihs`](#wbergb_to_ihs)
- [`wbe.remove_spurs`](#wberemove_spurs)
- [`wbe.roberts_cross_filter`](#wberoberts_cross_filter)
- [`wbe.resample`](#wberesample)
- [`wbe.savitzky_golay_2d_filter`](#wbesavitzky_golay_2d_filter)
- [`wbe.scharr_filter`](#wbescharr_filter)
- [`wbe.sigmoidal_contrast_stretch`](#wbesigmoidal_contrast_stretch)
- [`wbe.sobel_filter`](#wbesobel_filter)
- [`wbe.split_colour_composite`](#wbesplit_colour_composite)
- [`wbe.standard_deviation_contrast_stretch`](#wbestandard_deviation_contrast_stretch)
- [`wbe.standard_deviation_filter`](#wbestandard_deviation_filter)
- [`wbe.svm_classification`](#wbesvm_classification)
- [`wbe.svm_regression`](#wbesvm_regression)
- [`wbe.thicken_raster_line`](#wbethicken_raster_line)
- [`wbe.total_filter`](#wbetotal_filter)
- [`wbe.tophat_transform`](#wbetophat_transform)
- [`wbe.unsharp_masking`](#wbeunsharp_masking)
- [`wbe.user_defined_weights_filter`](#wbeuser_defined_weights_filter)
- [`wbe.wiener_filter`](#wbewiener_filter)
- [`wbe.write_function_memory_insertion`](#wbewrite_function_memory_insertion)

### `wbe.gaussian_filter`

```
wbe.gaussian_filter(
    input,
    sigma=None,
    treat_as_rgb=False,
    assume_three_band_rgb=True,
    output_path=None,
    callback=None,
) -> Raster
```

Applies a Gaussian smoothing kernel to a raster image.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `sigma` | `float \| None` | `0.75` | Gaussian standard deviation in pixels (0.5–20.0). Larger values produce a wider, smoother kernel |
| `treat_as_rgb` | `bool` | `False` | Force packed-RGB processing in HSI intensity space. When false, packed RGB may still be auto-detected from raster metadata |
| `assume_three_band_rgb` | `bool` | `True` | When `True`, 3-band uint8/uint16 rasters are treated as RGB if no explicit colour metadata is present. Set `False` for multispectral datasets |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
# Default smoothing
smoothed = wbe.gaussian_filter(image)

# Wider kernel
smoothed_wide = wbe.gaussian_filter(image, sigma=3.0, output_path='smoothed.tif')

# Disable 3-band RGB heuristic for a multispectral image
ms_smoothed = wbe.gaussian_filter(ms_image, sigma=1.5, assume_three_band_rgb=False)
```

---

### `wbe.bilateral_filter`

```
wbe.bilateral_filter(
    input,
    sigma_dist=None,
    sigma_int=None,
    treat_as_rgb=False,
    assume_three_band_rgb=True,
    output_path=None,
    callback=None,
) -> Raster
```

Applies an edge-preserving bilateral smoothing filter.  Nearby pixels that are
similar in intensity are averaged together; pixels across edges (large intensity
differences) are weighted much less, preserving sharp boundaries.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `sigma_dist` | `float \| None` | `0.75` | Spatial (distance) Gaussian standard deviation in pixels (0.5–20.0). Controls the filter radius |
| `sigma_int` | `float \| None` | `1.0` | Intensity Gaussian standard deviation in raster value units. Larger values reduce edge-preservation and approach a plain Gaussian blur |
| `treat_as_rgb` | `bool` | `False` | Force packed-RGB processing in HSI intensity space |
| `assume_three_band_rgb` | `bool` | `True` | When `True`, 3-band uint8/uint16 rasters are treated as RGB if no explicit colour metadata is present |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
# Default edge-preserving smoothing
smoothed = wbe.bilateral_filter(image)

# Stronger smoothing with larger kernels
smoothed_strong = wbe.bilateral_filter(
    image, sigma_dist=3.0, sigma_int=50.0, output_path='bilateral.tif'
)

# Disable the 3-band RGB heuristic for a multispectral image
ms_smoothed = wbe.bilateral_filter(
    ms_image, sigma_dist=1.5, sigma_int=25.0, assume_three_band_rgb=False
)
```

---

### `wbe.balance_contrast_enhancement`

```
wbe.balance_contrast_enhancement(
    input,
    band_mean=None,
    output_path=None,
    callback=None,
) -> Raster
```

Reduces colour bias in a packed RGB raster using per-channel balance contrast enhancement.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input packed RGB raster |
| `band_mean` | `float \| None` | `100.0` | Desired output mean brightness for each colour channel |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
bce = wbe.balance_contrast_enhancement(rgb_image)
bce120 = wbe.balance_contrast_enhancement(rgb_image, band_mean=120.0)
```

---

### `wbe.change_vector_analysis`

```
wbe.change_vector_analysis(
    date1,
    date2,
    magnitude_output=None,
    direction_output=None,
    callback=None,
) -> dict[str, Raster]
```

Performs change vector analysis on two-date multispectral datasets.
Returns both vector magnitude and direction-code rasters.

`date1` and `date2` can be either string lists (comma/semicolon-delimited)
or arrays of raster inputs, and must have equal lengths.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `date1` | `list[Raster] \| str` | required | Earlier-date raster list |
| `date2` | `list[Raster] \| str` | required | Later-date raster list in matching band order |
| `magnitude_output` | `str \| None` | `None` | Optional output path for CVA magnitude raster |
| `direction_output` | `str \| None` | `None` | Optional output path for CVA direction-code raster |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Returns**

A dict with keys `"magnitude"` and `"direction"` containing output rasters.

**Examples**

```python
cva = wbe.change_vector_analysis(
    date1=[d1_red, d1_green, d1_blue],
    date2=[d2_red, d2_green, d2_blue],
)
mag = cva["magnitude"]
direction = cva["direction"]
```

---

### `wbe.write_function_memory_insertion`

```
wbe.write_function_memory_insertion(
    input1,
    input2,
    input3=None,
    output_path=None,
    callback=None,
) -> Raster
```

Creates a packed RGB change-visualization composite from two or three single-band dates.

When `input3` is omitted, the second-date raster is used for both green and blue channels,
producing the classic two-date red/cyan change visualization.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input1` | `Raster` | required | First-date single-band raster (red channel) |
| `input2` | `Raster` | required | Second-date single-band raster (green channel) |
| `input3` | `Raster \| None` | `None` | Optional third-date single-band raster (blue channel); defaults to `input2` |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
# Two-date mode (R=date1, G/B=date2)
wfmi2 = wbe.write_function_memory_insertion(date1, date2)

# Three-date mode (R=date1, G=date2, B=date3)
wfmi3 = wbe.write_function_memory_insertion(date1, date2, input3=date3)
```

---

### `wbe.closing`

```
wbe.closing(
    input,
    filter_size_x=None,
    filter_size_y=None,
    output_path=None,
    callback=None,
) -> Raster
```

Performs a morphological closing using a rectangular structuring element.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `filter_size_x` | `int \| None` | `11` | Odd neighborhood width |
| `filter_size_y` | `int \| None` | `11` | Odd neighborhood height |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
closed = wbe.closing(image)
closed5 = wbe.closing(image, filter_size_x=5, filter_size_y=5)
```

---

### `wbe.corner_detection`

```
wbe.corner_detection(
    input,
    output_path=None,
    callback=None,
) -> Raster
```

Identifies corner patterns in binary rasters using hit-and-miss templates.
Foreground cells are values greater than zero; zero and nodata are treated as background.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input binary raster |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
corners = wbe.corner_detection(binary_image)
```

---

### `wbe.create_colour_composite`

```
wbe.create_colour_composite(
    red,
    green,
    blue,
    opacity=None,
    enhance=None,
    treat_zeros_as_nodata=None,
    output_path=None,
    callback=None,
) -> Raster
```

Creates a packed RGB colour composite from red, green, blue, and optional opacity rasters.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `red` | `Raster` | required | Red-band raster |
| `green` | `Raster` | required | Green-band raster |
| `blue` | `Raster` | required | Blue-band raster |
| `opacity` | `Raster \| None` | `None` | Optional opacity raster mapped into the alpha channel |
| `enhance` | `bool \| None` | `True` | Apply balance contrast enhancement after composing |
| `treat_zeros_as_nodata` | `bool \| None` | `False` | Treat zero values in RGB inputs as background/nodata |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
rgb = wbe.create_colour_composite(red_band, green_band, blue_band)
rgba = wbe.create_colour_composite(
    red_band, green_band, blue_band, opacity=mask, enhance=False
)
```

---

### `wbe.anisotropic_diffusion_filter`

```
wbe.anisotropic_diffusion_filter(
    input,
    iterations=None,
    kappa=None,
    lambda=None,
    output_path=None,
    callback=None,
) -> Raster
```

Perona-Malik anisotropic diffusion smoothing. This filter reduces noise while
limiting diffusion across strong edges.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `iterations` | `int \| None` | `10` | Number of diffusion iterations |
| `kappa` | `float \| None` | `20.0` | Edge sensitivity (higher values smooth across larger gradients) |
| `lambda` | `float \| None` | `0.2` | Diffusion time-step in `(0, 0.25]` |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
ad = wbe.anisotropic_diffusion_filter(image)
ad_strong = wbe.anisotropic_diffusion_filter(image, iterations=20, kappa=15.0, lambda=0.2)
```

---

### `wbe.gamma_correction`

```
wbe.gamma_correction(
    input,
    gamma=None,
    output_path=None,
    callback=None,
) -> Raster
```

Applies gamma intensity correction where output values are computed as
`z_out = z_in^gamma`.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `gamma` | `float \| None` | `0.5` | Gamma exponent in `[0, 4]` |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
gamma_light = wbe.gamma_correction(image, gamma=0.5)
gamma_dark = wbe.gamma_correction(image, gamma=2.0)
```

---

### `wbe.guided_filter`

```
wbe.guided_filter(
    input,
    radius=None,
    epsilon=None,
    output_path=None,
    callback=None,
) -> Raster
```

Edge-preserving guided filter using local linear models and box-filtered
statistics.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `radius` | `int \| None` | `4` | Local window radius in pixels |
| `epsilon` | `float \| None` | `0.01` | Regularization term for local variance |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
gf = wbe.guided_filter(image)
gf_tight = wbe.guided_filter(image, radius=8, epsilon=0.001)
```

---

### `wbe.wiener_filter`

```
wbe.wiener_filter(
    input,
    radius=None,
    noise_variance=None,
    output_path=None,
    callback=None,
) -> Raster
```

Adaptive Wiener denoising filter based on local mean and variance statistics.
If `noise_variance` is omitted, the filter estimates it from the image's local
variance map.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `radius` | `int \| None` | `2` | Local window radius in pixels |
| `noise_variance` | `float \| None` | estimated | Optional additive noise variance |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
wiener = wbe.wiener_filter(image)
wiener_known_noise = wbe.wiener_filter(image, radius=3, noise_variance=4.0)
```

---

### `wbe.non_local_means_filter`

```
wbe.non_local_means_filter(
    input,
    search_radius=None,
    patch_radius=None,
    h=None,
    output_path=None,
    callback=None,
) -> Raster
```

Non-local means denoiser that averages similar patches within a search window.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `search_radius` | `int \| None` | `5` | Search window radius in pixels |
| `patch_radius` | `int \| None` | `1` | Patch radius in pixels |
| `h` | `float \| None` | `10.0` | Filtering strength parameter |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
nlm = wbe.non_local_means_filter(image)
nlm_strong = wbe.non_local_means_filter(image, search_radius=7, patch_radius=1, h=15.0)
```

---

### `wbe.kuwahara_filter`

```
wbe.kuwahara_filter(
    input,
    radius=None,
    output_path=None,
    callback=None,
) -> Raster
```

Edge-preserving Kuwahara filter that selects the lowest-variance quadrant mean
for each pixel.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `radius` | `int \| None` | `2` | Quadrant radius in pixels |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
kuw = wbe.kuwahara_filter(image)
kuw_wide = wbe.kuwahara_filter(image, radius=3)
```

---

### `wbe.frost_filter`

```
wbe.frost_filter(
    input,
    radius=None,
    damping_factor=None,
    output_path=None,
    callback=None,
) -> Raster
```

Adaptive Frost speckle filter with exponential distance weighting controlled by
local statistics.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `radius` | `int \| None` | `2` | Local window radius in pixels |
| `damping_factor` | `float \| None` | `2.0` | Exponential damping factor |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
frost = wbe.frost_filter(image)
frost_strong = wbe.frost_filter(image, radius=3, damping_factor=3.0)
```

---

### `wbe.gamma_map_filter`

```
wbe.gamma_map_filter(
    input,
    radius=None,
    enl=None,
    output_path=None,
    callback=None,
) -> Raster
```

Gamma-MAP speckle filter for radar imagery using local coefficient-of-variation
regimes and ENL.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `radius` | `int \| None` | `2` | Local window radius in pixels |
| `enl` | `float \| None` | `1.0` | Equivalent number of looks |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
gmap = wbe.gamma_map_filter(image)
gmap_enl4 = wbe.gamma_map_filter(image, radius=3, enl=4.0)
```

---

### `wbe.kuan_filter`

```
wbe.kuan_filter(
    input,
    radius=None,
    enl=None,
    output_path=None,
    callback=None,
) -> Raster
```

Kuan speckle filter for radar imagery using an adaptive linear combination of
the center pixel and local mean.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `radius` | `int \| None` | `2` | Local window radius in pixels |
| `enl` | `float \| None` | `1.0` | Equivalent number of looks |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
kuan = wbe.kuan_filter(image)
kuan_enl4 = wbe.kuan_filter(image, radius=3, enl=4.0)
```

---

### `wbe.gabor_filter_bank`

```
wbe.gabor_filter_bank(
    input,
    sigma=None,
    frequency=None,
    orientations=None,
    output_path=None,
    callback=None,
) -> Raster
```

Multi-orientation Gabor filter bank. The output is the maximum response across
all tested orientations.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `sigma` | `float \| None` | `2.0` | Gaussian envelope sigma in pixels |
| `frequency` | `float \| None` | `0.2` | Sinusoid spatial frequency in cycles/pixel |
| `orientations` | `int \| None` | `6` | Number of orientations in the filter bank |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
gabor = wbe.gabor_filter_bank(image)
gabor_dense = wbe.gabor_filter_bank(image, sigma=3.0, frequency=0.15, orientations=8)
```

---

### `wbe.frangi_filter`

```
wbe.frangi_filter(
    input,
    scales=None,
    beta=None,
    c=None,
    output_path=None,
    callback=None,
) -> Raster
```

Multiscale Frangi vesselness enhancement filter for curvilinear feature
emphasis.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `scales` | `list[float] \| None` | `[1.0, 2.0, 3.0]` | Scale list used in multiscale response |
| `beta` | `float \| None` | `0.5` | Blob suppression parameter |
| `c` | `float \| None` | `15.0` | Structure sensitivity parameter |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
frangi = wbe.frangi_filter(image)
frangi_multi = wbe.frangi_filter(image, scales=[1.0, 2.0, 4.0], beta=0.5, c=20.0)
```

---

### `wbe.savitzky_golay_2d_filter`

```
wbe.savitzky_golay_2d_filter(
    input,
    window_size=None,
    output_path=None,
    callback=None,
) -> Raster
```

2D Savitzky-Golay smoothing filter. Current implementation supports
`window_size=5` (quadratic smoothing kernel).

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `window_size` | `int \| None` | `5` | Odd window size (currently fixed to 5) |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
sg = wbe.savitzky_golay_2d_filter(image)
sg5 = wbe.savitzky_golay_2d_filter(image, window_size=5)
```

---

### `wbe.fast_almost_gaussian_filter`

```
wbe.fast_almost_gaussian_filter(
    input,
    sigma=None,
    output_path=None,
    callback=None,
) -> Raster
```

Fast approximation of Gaussian smoothing using repeated box filtering. This is
typically best for larger kernels (`sigma >= 1.8`).

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `sigma` | `float \| None` | `1.8` | Target Gaussian sigma. Values below 1.8 are clamped to 1.8 |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
smoothed = wbe.fast_almost_gaussian_filter(image)
smoothed_big = wbe.fast_almost_gaussian_filter(image, sigma=5.0)
```

---

### `wbe.edge_preserving_mean_filter`

```
wbe.edge_preserving_mean_filter(
    input,
    filter_size=None,
    threshold=None,
    output_path=None,
    callback=None,
) -> Raster
```

Applies a thresholded local mean where only neighbors within `threshold` of the
center value are included in the average.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `filter_size` | `int \| None` | `11` | Odd square neighborhood size in pixels |
| `threshold` | `float \| None` | `15.0` | Max absolute neighbor difference allowed in local mean |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
epm = wbe.edge_preserving_mean_filter(image)
epm_tight = wbe.edge_preserving_mean_filter(image, filter_size=9, threshold=5.0)
```

---

### `wbe.unsharp_masking`

```
wbe.unsharp_masking(
    input,
    sigma=None,
    amount=None,
    threshold=None,
    output_path=None,
    callback=None,
) -> Raster
```

Sharpens edges by subtracting a Gaussian blur from the source and adding a
scaled residual back to the image.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `sigma` | `float \| None` | `0.75` | Gaussian sigma for blur mask (0.5–20.0) |
| `amount` | `float \| None` | `100.0` | Residual multiplier for sharpening strength |
| `threshold` | `float \| None` | `0.0` | Minimum absolute residual needed to sharpen |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
sharp = wbe.unsharp_masking(image)
sharp_strong = wbe.unsharp_masking(image, sigma=1.5, amount=150.0, threshold=0.01)
```

---

### `wbe.diff_of_gaussians_filter`

```
wbe.diff_of_gaussians_filter(
    input,
    sigma1=None,
    sigma2=None,
    output_path=None,
    callback=None,
) -> Raster
```

Difference-of-Gaussians (DoG) band-pass filter computed as blur(`sigma1`) -
blur(`sigma2`).

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `sigma1` | `float \| None` | `2.0` | Smaller Gaussian sigma (0.25–20.0) |
| `sigma2` | `float \| None` | `4.0` | Larger Gaussian sigma (0.5–20.0). If reversed, values are swapped internally |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
dog = wbe.diff_of_gaussians_filter(image)
dog_custom = wbe.diff_of_gaussians_filter(image, sigma1=1.5, sigma2=3.0)
```

---

### `wbe.adaptive_filter`

```
wbe.adaptive_filter(
    input,
    filter_size_x=None,
    filter_size_y=None,
    threshold=None,
    output_path=None,
    callback=None,
) -> Raster
```

Adaptive smoothing that replaces only center cells whose local z-score exceeds
`threshold` relative to neighborhood mean and variance.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `filter_size_x` | `int \| None` | `11` | Odd neighborhood width |
| `filter_size_y` | `int \| None` | `11` | Odd neighborhood height |
| `threshold` | `float \| None` | `2.0` | Absolute z-score threshold for replacement |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
adaptive = wbe.adaptive_filter(image)
adaptive_tight = wbe.adaptive_filter(image, filter_size_x=9, filter_size_y=9, threshold=1.5)
```

---

### `wbe.lee_filter`

```
wbe.lee_filter(
    input,
    filter_size_x=None,
    filter_size_y=None,
    sigma=None,
    m_value=None,
    output_path=None,
    callback=None,
) -> Raster
```

Lee sigma filter that averages in-range neighbors (`z ± sigma`) and falls back
to immediate-neighbor averaging when in-range support is low.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `filter_size_x` | `int \| None` | `11` | Odd neighborhood width |
| `filter_size_y` | `int \| None` | `11` | Odd neighborhood height |
| `sigma` | `float \| None` | `10.0` | Intensity inclusion half-width around center value |
| `m_value` | `float \| None` | `5.0` | Minimum in-range sample count before fallback |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
lee = wbe.lee_filter(image)
lee_custom = wbe.lee_filter(image, filter_size_x=9, filter_size_y=9, sigma=6.0, m_value=4.0)
```

---

### `wbe.conservative_smoothing_filter`

```
wbe.conservative_smoothing_filter(
    input,
    filter_size_x=None,
    filter_size_y=None,
    output_path=None,
    callback=None,
) -> Raster
```

Conservative smoother that clips spike values to neighborhood extrema while
preserving most non-outlier cells.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `filter_size_x` | `int \| None` | `3` | Odd neighborhood width |
| `filter_size_y` | `int \| None` | `3` | Odd neighborhood height |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
cs = wbe.conservative_smoothing_filter(image)
cs_wide = wbe.conservative_smoothing_filter(image, filter_size_x=5, filter_size_y=5)
```

---

### `wbe.olympic_filter`

```
wbe.olympic_filter(
    input,
    filter_size_x=None,
    filter_size_y=None,
    output_path=None,
    callback=None,
) -> Raster
```

Mean-like smoother that drops the minimum and maximum values in each
neighborhood before averaging.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `filter_size_x` | `int \| None` | `11` | Odd neighborhood width |
| `filter_size_y` | `int \| None` | `11` | Odd neighborhood height |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
olympic = wbe.olympic_filter(image)
olympic_small = wbe.olympic_filter(image, filter_size_x=5, filter_size_y=5)
```

---

### `wbe.k_nearest_mean_filter`

```
wbe.k_nearest_mean_filter(
    input,
    filter_size_x=None,
    filter_size_y=None,
    k=None,
    output_path=None,
    callback=None,
) -> Raster
```

Edge-preserving smoother that averages only the `k` local neighbors most
similar to the center value.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `filter_size_x` | `int \| None` | `3` | Odd neighborhood width |
| `filter_size_y` | `int \| None` | `3` | Odd neighborhood height |
| `k` | `int \| None` | `5` | Number of nearest neighbors to include in mean |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
knn_mean = wbe.k_nearest_mean_filter(image)
knn_mean_custom = wbe.k_nearest_mean_filter(image, filter_size_x=5, filter_size_y=5, k=8)
```

---

### `wbe.high_pass_median_filter`

```
wbe.high_pass_median_filter(
    input,
    filter_size_x=None,
    filter_size_y=None,
    sig_digits=None,
    output_path=None,
    callback=None,
) -> Raster
```

High-pass median filter that outputs center value minus local median using
quantized histogram bins controlled by `sig_digits`.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `filter_size_x` | `int \| None` | `11` | Odd neighborhood width |
| `filter_size_y` | `int \| None` | `11` | Odd neighborhood height |
| `sig_digits` | `int \| None` | `2` | Significant digits used for quantization bins |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
hpm = wbe.high_pass_median_filter(image)
hpm_precise = wbe.high_pass_median_filter(image, filter_size_x=7, filter_size_y=7, sig_digits=3)
```

---

### `wbe.laplacian_of_gaussians_filter`

```
wbe.laplacian_of_gaussians_filter(
    input,
    sigma=None,
    output_path=None,
    callback=None,
) -> Raster
```

Laplacian-of-Gaussians (LoG) edge-enhancement filter using a sigma-derived
kernel footprint.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `sigma` | `float \| None` | `0.75` | Gaussian sigma used by LoG kernel (0.5–20.0) |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
log_img = wbe.laplacian_of_gaussians_filter(image)
log_wide = wbe.laplacian_of_gaussians_filter(image, sigma=2.0)
```

---

### `wbe.diversity_filter`

```
wbe.diversity_filter(
    input,
    filter_size_x=None,
    filter_size_y=None,
    output_path=None,
    callback=None,
) -> Raster
```

Moving-window diversity filter (count of unique values in each neighborhood).

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `filter_size_x` | `int \| None` | `11` | Odd neighborhood width |
| `filter_size_y` | `int \| None` | `11` | Odd neighborhood height |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
div = wbe.diversity_filter(image)
div5 = wbe.diversity_filter(image, filter_size_x=5, filter_size_y=5)
```

---

### `wbe.emboss_filter`

```
wbe.emboss_filter(
    input,
    direction=None,
    clip_amount=None,
    output_path=None,
    callback=None,
) -> Raster
```

Directional emboss convolution filter.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `direction` | `str \| None` | `"n"` | Emboss direction: `n`, `s`, `e`, `w`, `ne`, `nw`, `se`, `sw` |
| `clip_amount` | `float \| None` | `0.0` | Optional symmetric tail clipping percent (0-40) |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
emb = wbe.emboss_filter(image)
emb_ne = wbe.emboss_filter(image, direction='ne', clip_amount=1.0)
```

---

### `wbe.high_pass_filter`

```
wbe.high_pass_filter(
    input,
    filter_size_x=None,
    filter_size_y=None,
    output_path=None,
    callback=None,
) -> Raster
```

High-pass filter using neighborhood mean subtraction.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `filter_size_x` | `int \| None` | `11` | Odd neighborhood width |
| `filter_size_y` | `int \| None` | `11` | Odd neighborhood height |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
hp = wbe.high_pass_filter(image)
hp7 = wbe.high_pass_filter(image, filter_size_x=7, filter_size_y=7)
```

---

### `wbe.high_pass_bilateral_filter`

```
wbe.high_pass_bilateral_filter(
    input,
    sigma_dist=None,
    sigma_int=None,
    treat_as_rgb=None,
    assume_three_band_rgb=None,
    output_path=None,
    callback=None,
) -> Raster
```

Computes a high-pass residual by subtracting bilateral smoothing from the input raster.
This emphasizes local texture while reducing dominance of strong edges.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `sigma_dist` | `float \| None` | `0.75` | Spatial Gaussian standard deviation in pixels (0.5–20.0) |
| `sigma_int` | `float \| None` | `1.0` | Intensity Gaussian standard deviation in raster-value units |
| `treat_as_rgb` | `bool \| None` | `False` | Force packed RGB HSI-intensity processing |
| `assume_three_band_rgb` | `bool \| None` | `True` | Enable 3-band RGB heuristic when metadata is absent |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
hpb = wbe.high_pass_bilateral_filter(image)
hpb_tex = wbe.high_pass_bilateral_filter(image, sigma_dist=2.5, sigma_int=4.0)
```

---

### `wbe.laplacian_filter`

```
wbe.laplacian_filter(
    input,
    variant=None,
    clip_amount=None,
    output_path=None,
    callback=None,
) -> Raster
```

Laplacian edge/sharpen filter.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `variant` | `str \| None` | `"3x3(1)"` | Kernel variant: `3x3(1)`, `3x3(2)`, `3x3(3)`, `3x3(4)`, `5x5(1)`, `5x5(2)` |
| `clip_amount` | `float \| None` | `0.0` | Optional symmetric tail clipping percent (0-40) |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
lap = wbe.laplacian_filter(image)
lap5 = wbe.laplacian_filter(image, variant='5x5(1)', clip_amount=1.0)
```

---

### `wbe.line_detection_filter`

```
wbe.line_detection_filter(
    input,
    variant=None,
    abs_values=None,
    clip_tails=None,
    output_path=None,
    callback=None,
) -> Raster
```

Directional line-detection convolution filter.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `variant` | `str \| None` | `"v"` | Line direction variant: `v`, `h`, `45`, `135` |
| `abs_values` | `bool \| None` | `False` | If `True`, return absolute response |
| `clip_tails` | `float \| None` | `0.0` | Optional symmetric tail clipping percent (0-40) |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
line_v = wbe.line_detection_filter(image)
line_45 = wbe.line_detection_filter(image, variant='45', abs_values=True)
```

---

### `wbe.line_thinning`

```
wbe.line_thinning(
    input,
    output_path=None,
    callback=None,
) -> Raster
```

Performs iterative skeletonization, reducing connected binary features to one-cell-wide lines.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster; positive values are treated as foreground |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
skeleton = wbe.line_thinning(binary_image)
```

---

### `wbe.generalize_classified_raster`

```
wbe.generalize_classified_raster(
    input,
    area_threshold=5,
    method="longest",
    output_path=None,
    callback=None,
) -> Raster
```

Generalizes a classified raster by reassigning small patches to neighboring classes.
Use `method="longest"`, `"largest"`, or `"nearest"` depending on the desired merge behavior.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input classified raster |
| `area_threshold` | `int` | `5` | Minimum feature size (cells); smaller patches are reassigned |
| `method` | `str` | `"longest"` | Merge strategy: `longest`, `largest`, or `nearest` |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
generalized = wbe.generalize_classified_raster(
    classes,
    area_threshold=15,
    method="largest",
)
```

---

### `wbe.majority_filter`

```
wbe.majority_filter(
    input,
    filter_size_x=None,
    filter_size_y=None,
    output_path=None,
    callback=None,
) -> Raster
```

Moving-window majority (mode) filter.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `filter_size_x` | `int \| None` | `11` | Odd neighborhood width |
| `filter_size_y` | `int \| None` | `11` | Odd neighborhood height |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
maj = wbe.majority_filter(image)
maj3 = wbe.majority_filter(image, filter_size_x=3, filter_size_y=3)
```

---

### `wbe.maximum_filter`

```
wbe.maximum_filter(
    input,
    filter_size_x=None,
    filter_size_y=None,
    output_path=None,
    callback=None,
) -> Raster
```

Moving-window maximum filter.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `filter_size_x` | `int \| None` | `11` | Odd neighborhood width |
| `filter_size_y` | `int \| None` | `11` | Odd neighborhood height |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
mx = wbe.maximum_filter(image)
mx7 = wbe.maximum_filter(image, filter_size_x=7, filter_size_y=7)
```

---

### `wbe.mean_filter`

```
wbe.mean_filter(
    input,
    filter_size_x=None,
    filter_size_y=None,
    output_path=None,
    callback=None,
) -> Raster
```

Moving-window mean filter.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `filter_size_x` | `int \| None` | `11` | Odd neighborhood width |
| `filter_size_y` | `int \| None` | `11` | Odd neighborhood height |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
mn = wbe.mean_filter(image)
mn9 = wbe.mean_filter(image, filter_size_x=9, filter_size_y=9)
```

---

### `wbe.median_filter`

```
wbe.median_filter(
    input,
    filter_size_x=None,
    filter_size_y=None,
    sig_digits=None,
    output_path=None,
    callback=None,
) -> Raster
```

Moving-window median filter using quantization bins controlled by `sig_digits`.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `filter_size_x` | `int \| None` | `11` | Odd neighborhood width |
| `filter_size_y` | `int \| None` | `11` | Odd neighborhood height |
| `sig_digits` | `int \| None` | `2` | Significant digits used for quantized rank filtering |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
med = wbe.median_filter(image)
med_precise = wbe.median_filter(image, filter_size_x=7, filter_size_y=7, sig_digits=3)
```

---

### `wbe.minimum_filter`

```
wbe.minimum_filter(
    input,
    filter_size_x=None,
    filter_size_y=None,
    output_path=None,
    callback=None,
) -> Raster
```

Moving-window minimum filter.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `filter_size_x` | `int \| None` | `11` | Odd neighborhood width |
| `filter_size_y` | `int \| None` | `11` | Odd neighborhood height |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
mn = wbe.minimum_filter(image)
mn5 = wbe.minimum_filter(image, filter_size_x=5, filter_size_y=5)
```

---

### `wbe.percentile_filter`

```
wbe.percentile_filter(
    input,
    filter_size_x=None,
    filter_size_y=None,
    sig_digits=None,
    output_path=None,
    callback=None,
) -> Raster
```

Moving-window percentile-rank filter using quantization bins controlled by `sig_digits`.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `filter_size_x` | `int \| None` | `11` | Odd neighborhood width |
| `filter_size_y` | `int \| None` | `11` | Odd neighborhood height |
| `sig_digits` | `int \| None` | `2` | Significant digits used for quantized rank filtering |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
pct = wbe.percentile_filter(image)
pct_precise = wbe.percentile_filter(image, filter_size_x=7, filter_size_y=7, sig_digits=3)
```

---

### `wbe.prewitt_filter`

```
wbe.prewitt_filter(
    input,
    clip_tails=None,
    output_path=None,
    callback=None,
) -> Raster
```

Prewitt edge-detection filter.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `clip_tails` | `float \| None` | `0.0` | Optional symmetric tail clipping percent (0-40) |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
prew = wbe.prewitt_filter(image)
prew_clip = wbe.prewitt_filter(image, clip_tails=1.0)
```

---

### `wbe.range_filter`

```
wbe.range_filter(
    input,
    filter_size_x=None,
    filter_size_y=None,
    output_path=None,
    callback=None,
) -> Raster
```

Moving-window range filter (`max - min`).

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `filter_size_x` | `int \| None` | `11` | Odd neighborhood width |
| `filter_size_y` | `int \| None` | `11` | Odd neighborhood height |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
rng = wbe.range_filter(image)
rng7 = wbe.range_filter(image, filter_size_x=7, filter_size_y=7)
```

---

### `wbe.remove_spurs`

```
wbe.remove_spurs(
    input,
    max_iterations=None,
    output_path=None,
    callback=None,
) -> Raster
```

Removes small spur branches from binary raster features using iterative pruning templates.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster; positive values are treated as foreground |
| `max_iterations` | `int \| None` | `10` | Maximum number of pruning iterations |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
pruned = wbe.remove_spurs(binary_image)
pruned_strict = wbe.remove_spurs(binary_image, max_iterations=20)
```

---

### `wbe.roberts_cross_filter`

```
wbe.roberts_cross_filter(
    input,
    clip_amount=None,
    output_path=None,
    callback=None,
) -> Raster
```

Roberts Cross edge-detection filter.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `clip_amount` | `float \| None` | `0.0` | Optional symmetric tail clipping percent (0-40) |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
rob = wbe.roberts_cross_filter(image)
rob_clip = wbe.roberts_cross_filter(image, clip_amount=1.0)
```

---

### `wbe.scharr_filter`

```
wbe.scharr_filter(
    input,
    clip_tails=None,
    output_path=None,
    callback=None,
) -> Raster
```

Scharr edge-detection filter.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `clip_tails` | `float \| None` | `0.0` | Optional symmetric tail clipping percent (0-40) |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
sch = wbe.scharr_filter(image)
sch_clip = wbe.scharr_filter(image, clip_tails=1.0)
```

---

### `wbe.sobel_filter`

```
wbe.sobel_filter(
    input,
    variant=None,
    clip_tails=None,
    output_path=None,
    callback=None,
) -> Raster
```

Sobel edge-detection filter.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `variant` | `str \| None` | `"3x3"` | Kernel size variant: `3x3` or `5x5` |
| `clip_tails` | `float \| None` | `0.0` | Optional symmetric tail clipping percent (0-40) |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
sob = wbe.sobel_filter(image)
sob5 = wbe.sobel_filter(image, variant='5x5', clip_tails=1.0)
```

---

### `wbe.standard_deviation_filter`

```
wbe.standard_deviation_filter(
    input,
    filter_size_x=None,
    filter_size_y=None,
    output_path=None,
    callback=None,
) -> Raster
```

Moving-window standard deviation filter.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `filter_size_x` | `int \| None` | `11` | Odd neighborhood width |
| `filter_size_y` | `int \| None` | `11` | Odd neighborhood height |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
std = wbe.standard_deviation_filter(image)
std5 = wbe.standard_deviation_filter(image, filter_size_x=5, filter_size_y=5)
```

---

### `wbe.total_filter`

```
wbe.total_filter(
    input,
    filter_size_x=None,
    filter_size_y=None,
    output_path=None,
    callback=None,
) -> Raster
```

Moving-window total (sum) filter.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `filter_size_x` | `int \| None` | `11` | Odd neighborhood width |
| `filter_size_y` | `int \| None` | `11` | Odd neighborhood height |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
tot = wbe.total_filter(image)
tot9 = wbe.total_filter(image, filter_size_x=9, filter_size_y=9)
```

---

### `wbe.user_defined_weights_filter`

```
wbe.user_defined_weights_filter(
    input,
    weights,
    kernel_center=None,
    normalize_weights=None,
    output_path=None,
    callback=None,
) -> Raster
```

Applies a user-defined convolution kernel.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `weights` | `list[list[float]]` | required | 2D convolution kernel with equal row lengths |
| `kernel_center` | `str \| None` | `"center"` | Kernel center policy (`center` by default) |
| `normalize_weights` | `bool \| None` | `False` | If `True`, normalize kernel sum before convolution |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
sharpen = wbe.user_defined_weights_filter(
    image,
    weights=[[0, -1, 0], [-1, 5, -1], [0, -1, 0]],
)
```

---

### `wbe.flip_image`

```
wbe.flip_image(
    input,
    direction=None,
    output_path=None,
    callback=None,
) -> Raster
```

Flips an image vertically, horizontally, or both.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `direction` | `str \| None` | `"vertical"` | Flip direction: `vertical`, `horizontal`, or `both` |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
flip_v = wbe.flip_image(image)
flip_h = wbe.flip_image(image, direction='horizontal')
flip_b = wbe.flip_image(image, direction='both')
```

---

### `wbe.direct_decorrelation_stretch`

```
wbe.direct_decorrelation_stretch(
    input,
    achromatic_factor=None,
    clip_percent=None,
    output_path=None,
    callback=None,
) -> Raster
```

Improves packed RGB image saturation by reducing the achromatic component and linearly stretching the result.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input packed RGB raster |
| `achromatic_factor` | `float \| None` | `0.5` | Grey-component reduction factor from 0 to 1 |
| `clip_percent` | `float \| None` | `1.0` | Percent tail clipping used in the final linear stretch |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
dds = wbe.direct_decorrelation_stretch(rgb_image)
dds_strong = wbe.direct_decorrelation_stretch(rgb_image, achromatic_factor=0.7, clip_percent=2.0)
```

---

### `wbe.image_slider`

```
wbe.image_slider(
    left_raster,
    right_raster,
    output_html_file=None,
    left_label="",
    right_label="",
    image_height=600,
    callback=None,
) -> str
```

Creates an interactive HTML swipe/slider view for two input rasters.
The tool also writes PNG previews beside the HTML output.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `left_raster` | `Raster` | required | Left image input |
| `right_raster` | `Raster` | required | Right image input |
| `output_html_file` | `str \| None` | `None` | HTML output path; defaults to `image_slider.html` in the working directory |
| `left_label` | `str` | `""` | Optional left-side label |
| `right_label` | `str` | `""` | Optional right-side label |
| `image_height` | `int` | `600` | Slider height in pixels (minimum 50) |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
html = wbe.image_slider(
    left_raster=before,
    right_raster=after,
    left_label="Before",
    right_label="After",
    image_height=640,
    output_html_file="change_slider.html",
)
```

---

### `wbe.integral_image_transform`

```
wbe.integral_image_transform(
    input,
    output_path=None,
    callback=None,
) -> Raster
```

Computes the summed-area table (integral image) transform for each raster band.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
ii = wbe.integral_image_transform(image)
```

---

### `wbe.opening`

```
wbe.opening(
    input,
    filter_size_x=None,
    filter_size_y=None,
    output_path=None,
    callback=None,
) -> Raster
```

Performs a morphological opening using a rectangular structuring element.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `filter_size_x` | `int \| None` | `11` | Odd neighborhood width |
| `filter_size_y` | `int \| None` | `11` | Odd neighborhood height |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
opened = wbe.opening(image)
opened7 = wbe.opening(image, filter_size_x=7, filter_size_y=7)
```

---

### `wbe.otsu_thresholding`

```
wbe.otsu_thresholding(
    input,
    output_path=None,
    callback=None,
) -> Raster
```

Applies Otsu's automatic thresholding to produce a binary raster with values of 0 and 1.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
binary = wbe.otsu_thresholding(image)
```

---

### `wbe.normalized_difference_index`

```
wbe.normalized_difference_index(
    input,
    band1=None,
    band2=None,
    output_path=None,
    callback=None,
) -> Raster
```

Computes a normalized difference index: `(band1 - band2) / (band1 + band2)`.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input multiband raster |
| `band1` | `int \| None` | `1` | One-based index of first band |
| `band2` | `int \| None` | `2` | One-based index of second band |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
ndi = wbe.normalized_difference_index(multiband)
ndvi_like = wbe.normalized_difference_index(multiband, band1=5, band2=4)
```

---

### `wbe.histogram_equalization`

```
wbe.histogram_equalization(
    input,
    num_tones=None,
    output_path=None,
    callback=None,
) -> Raster
```

Applies histogram equalization to improve image contrast.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `num_tones` | `int \| None` | `256` | Number of output tones |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
eq = wbe.histogram_equalization(image)
eq1024 = wbe.histogram_equalization(image, num_tones=1024)
```

---

### `wbe.histogram_matching`

```
wbe.histogram_matching(
    input,
    histogram,
    is_cumulative=None,
    output_path=None,
    callback=None,
) -> Raster
```

Matches an image histogram to a user-supplied reference histogram.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `histogram` | `list[list[float]] \| list[dict]` | required | Reference histogram as `[[value, frequency], ...]` or `[{"x": value, "y": frequency}, ...]` |
| `is_cumulative` | `bool \| None` | `False` | Set `True` if histogram frequencies are already cumulative |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
ref = [[0.0, 0.05], [64.0, 0.25], [128.0, 0.75], [255.0, 1.0]]
matched = wbe.histogram_matching(image, histogram=ref, is_cumulative=True)
```

---

### `wbe.histogram_matching_two_images`

```
wbe.histogram_matching_two_images(
    input,
    reference,
    output_path=None,
    callback=None,
) -> Raster
```

Matches an input image histogram to the histogram of a reference image.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `reference` | `Raster` | required | Reference raster whose distribution is used as the target |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
matched = wbe.histogram_matching_two_images(source_image, reference_image)
```

---

### `wbe.gaussian_contrast_stretch`

```
wbe.gaussian_contrast_stretch(
    input,
    num_tones=None,
    output_path=None,
    callback=None,
) -> Raster
```

Performs Gaussian contrast stretching by matching to a Gaussian reference histogram.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `num_tones` | `int \| None` | `256` | Number of output tones |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
gcs = wbe.gaussian_contrast_stretch(image)
gcs1024 = wbe.gaussian_contrast_stretch(image, num_tones=1024)
```

---

### `wbe.min_max_contrast_stretch`

```
wbe.min_max_contrast_stretch(
    input,
    min_val,
    max_val,
    num_tones=None,
    output_path=None,
    callback=None,
) -> Raster
```

Linearly stretches values between user-provided minimum and maximum limits.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `min_val` | `float` | required | Lower bound for scaling |
| `max_val` | `float` | required | Upper bound for scaling |
| `num_tones` | `int \| None` | `256` | Number of output tones |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
stretch = wbe.min_max_contrast_stretch(image, min_val=50.0, max_val=1500.0)
```

---

### `wbe.mosaic`

```
wbe.mosaic(
    inputs,
    method=None,
    output_path=None,
    callback=None,
) -> Raster
```

Mosaics two or more rasters into a new output image spanning the combined extent
of all input rasters.

Cells with overlap are resolved by input order, with later rasters in `inputs`
taking precedence over earlier rasters.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `inputs` | `list[Raster]` | required | Input rasters to mosaic (minimum 2) |
| `method` | `str \| None` | `"nn"` | Resampling method: `"nn"`, `"bilinear"`, or `"cc"` |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
mosaic = wbe.mosaic([tile1, tile2, tile3])
mosaic_smooth = wbe.mosaic([tile1, tile2], method="cc", output_path="mosaic.tif")
```

---

### `wbe.mosaic_with_feathering`

```
wbe.mosaic_with_feathering(
    input1,
    input2,
    method=None,
    weight=None,
    output_path=None,
    callback=None,
) -> Raster
```

Mosaics two rasters and feather-blends overlap zones to reduce seam artifacts.

In overlapping areas, each raster contributes according to distance-to-edge
weights, so cells further from source-image edges receive larger influence.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input1` | `Raster` | required | First input raster |
| `input2` | `Raster` | required | Second input raster |
| `method` | `str \| None` | `"cc"` | Resampling method: `"nn"`, `"bilinear"`, or `"cc"` |
| `weight` | `float \| None` | `4.0` | Distance-weight exponent used in overlap blending |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
feathered = wbe.mosaic_with_feathering(image1, image2)
feathered_soft = wbe.mosaic_with_feathering(
    image1,
    image2,
    method="bilinear",
    weight=2.0,
    output_path="mosaic_feathered.tif",
)
```

---

### `wbe.k_means_clustering`

```
wbe.k_means_clustering(
    inputs,
    classes,
    max_iterations=None,
    class_change=None,
    initialize=None,
    min_class_size=None,
    out_html=None,
    output_path=None,
    callback=None,
) -> Raster
```

Performs k-means clustering on two or more input rasters (typically a
multispectral stack) and returns a categorical class raster.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `inputs` | `list[Raster]` | required | Input rasters (minimum 2) |
| `classes` | `int` | required | Number of target classes |
| `max_iterations` | `int \| None` | `10` | Maximum iteration count (2-250) |
| `class_change` | `float \| None` | `2.0` | Percent changed-cell stop threshold (0-25) |
| `initialize` | `str \| None` | `"diagonal"` | Initial centroid mode: `"diagonal"` or `"random"` |
| `min_class_size` | `int \| None` | `10` | Minimum class size used when updating centroids |
| `out_html` | `str \| None` | `None` | Optional output HTML report path |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
classes = wbe.k_means_clustering(
    inputs=[b1, b2, b3],
    classes=10,
    initialize="random",
    out_html="kmeans_report.html",
)
```

---

### `wbe.modified_k_means_clustering`

```
wbe.modified_k_means_clustering(
    inputs,
    merge_dist,
    start_clusters=None,
    max_iterations=None,
    class_change=None,
    out_html=None,
    output_path=None,
    callback=None,
) -> Raster
```

Runs a modified k-means workflow that begins with an overestimated cluster
count and merges nearby centroids during iteration.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `inputs` | `list[Raster]` | required | Input rasters (minimum 2) |
| `merge_dist` | `float` | required | Euclidean centroid merge threshold |
| `start_clusters` | `int \| None` | `1000` | Initial cluster count before merging |
| `max_iterations` | `int \| None` | `10` | Maximum iteration count (2-250) |
| `class_change` | `float \| None` | `2.0` | Percent changed-cell stop threshold (0-25) |
| `out_html` | `str \| None` | `None` | Optional output HTML report path |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
mod_classes = wbe.modified_k_means_clustering(
    inputs=[b1, b2, b3],
    start_clusters=80,
    merge_dist=25.0,
    out_html="modified_kmeans_report.html",
)
```

---

### `wbe.correct_vignetting`

```
wbe.correct_vignetting(
    input,
    pp,
    focal_length=None,
    image_width=None,
    n=None,
    output_path=None,
    callback=None,
) -> Raster
```

Corrects lens vignetting (darkening toward image edges) relative to a principal
point using a cosine falloff model.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster image |
| `pp` | `Vector \| str \| dict` | required | Point vector layer (path or typed vector object) containing the principal point |
| `focal_length` | `float \| None` | `304.8` | Camera focal length in mm |
| `image_width` | `float \| None` | `228.6` | Distance between left-right image edges in mm |
| `n` | `float \| None` | `4.0` | Vignetting model exponent |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
corrected = wbe.correct_vignetting(
    input=image,
    pp="principal_point.geojson",
    focal_length=304.8,
    image_width=228.6,
    n=4.0,
)
```

---

### `wbe.image_stack_profile`

```
wbe.image_stack_profile(
    inputs,
    points,
    output_html=None,
    callback=None,
) -> dict
```

Extracts point signatures across an ordered stack of rasters and returns the
profile values as structured output. An optional HTML report can also be
generated.

`points` should reference a vector point layer. Each point geometry is mapped
to pixel row/column locations using the first raster in `inputs`.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `inputs` | `list[Raster]` | required | Input raster stack (minimum 2) |
| `points` | `Vector \| str \| dict` | required | Point vector layer (path or typed vector object) with sample locations |
| `output_html` | `str \| None` | `None` | Optional HTML report output path |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
profiles = wbe.image_stack_profile(
    inputs=[image1, image2, image3],
    points="sample_points.geojson",
    output_html="stack_profile.html",
)
```

---

### `wbe.panchromatic_sharpening`

```
wbe.panchromatic_sharpening(
    red=None,
    green=None,
    blue=None,
    composite=None,
    pan,
    method=None,
    output_mode=None,
    output_path=None,
    callback=None,
) -> Raster
```

Fuses multispectral and panchromatic rasters using either Brovey or IHS pan-sharpening.

Provide either separate `red`/`green`/`blue` rasters or a packed RGB `composite` raster.

`output_mode` controls output encoding:

1. `packed` (default): single-band packed RGB raster
2. `bands`: 3-band raster with explicit R, G, B channels

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `red` | `Raster \| None` | `None` | Red-band raster (mutually exclusive with `composite`) |
| `green` | `Raster \| None` | `None` | Green-band raster (mutually exclusive with `composite`) |
| `blue` | `Raster \| None` | `None` | Blue-band raster (mutually exclusive with `composite`) |
| `composite` | `Raster \| None` | `None` | Packed RGB multispectral raster (mutually exclusive with `red`/`green`/`blue`) |
| `pan` | `Raster` | required | Panchromatic raster |
| `method` | `str \| None` | `"brovey"` | Fusion method: `"brovey"` or `"ihs"` |
| `output_mode` | `str \| None` | `"packed"` | Output encoding: `"packed"` or `"bands"` |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
# Default packed RGB output
ps = wbe.panchromatic_sharpening(
    red=red_band, green=green_band, blue=blue_band, pan=pan_band
)

# 3-band RGB output mode
ps_bands = wbe.panchromatic_sharpening(
    composite=rgb_composite,
    pan=pan_band,
    method="ihs",
    output_mode="bands",
)
```

---

### `wbe.resample`

```
wbe.resample(
    inputs,
    cell_size=None,
    base=None,
    method=None,
    output_path=None,
    callback=None,
) -> Raster
```

Resamples one or more source rasters into a destination grid defined by either
an explicit `cell_size` or a `base` raster.

If both `cell_size` and `base` are provided, `base` determines the output extent
and resolution. Cells with overlap are resolved by input order, with later
rasters in `inputs` taking precedence.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `inputs` | `list[Raster]` | required | Input rasters to resample (minimum 1) |
| `cell_size` | `float \| None` | `None` | Output cell size when `base` is not provided |
| `base` | `Raster \| None` | `None` | Base raster defining output extent/grid (takes precedence over `cell_size`) |
| `method` | `str \| None` | `"cc"` | Resampling method: `"nn"`, `"bilinear"`, or `"cc"` |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
# Resample to a new cell size over combined input extent
resampled = wbe.resample([image1, image2], cell_size=10.0)

# Resample into an existing grid definition
resampled_to_base = wbe.resample([image1], base=target_grid, method="bilinear")
```

---

### `wbe.piecewise_contrast_stretch`

```
wbe.piecewise_contrast_stretch(
    input,
    transformation_statement,
    num_greytones=1024,
    output_path=None,
    callback=None,
) -> Raster
```

Applies a piecewise linear contrast transfer function to raster brightness values.
For packed RGB rasters, mapping is applied to HSI intensity and colour is preserved.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `transformation_statement` | `str` | required | Breakpoint statement like `"(50,0.1);(120,0.6);(180,0.85)"` |
| `num_greytones` | `int` | `1024` | Number of output tones for non-RGB output (minimum 32) |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
stretched = wbe.piecewise_contrast_stretch(
    image,
    transformation_statement="(80,0.2);(140,0.7);(200,0.92)",
    num_greytones=512,
)
```

---

### `wbe.percentage_contrast_stretch`

```
wbe.percentage_contrast_stretch(
    input,
    clip=None,
    tail=None,
    num_tones=None,
    output_path=None,
    callback=None,
) -> Raster
```

Performs linear contrast stretching with percentile clipping.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `clip` | `float \| None` | `1.0` | Percentile clip amount (0-50) |
| `tail` | `str \| None` | `"both"` | Tail clipping mode: `both`, `upper`, or `lower` |
| `num_tones` | `int \| None` | `256` | Number of output tones |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
stretched = wbe.percentage_contrast_stretch(image)
stretched_custom = wbe.percentage_contrast_stretch(image, clip=2.0, tail='upper', num_tones=512)
```

---

### `wbe.sigmoidal_contrast_stretch`

```
wbe.sigmoidal_contrast_stretch(
    input,
    cutoff=None,
    gain=None,
    num_tones=None,
    output_path=None,
    callback=None,
) -> Raster
```

Performs a sigmoidal contrast stretch controlled by midpoint (`cutoff`) and slope (`gain`).

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `cutoff` | `float \| None` | `0.0` | Normalized sigmoid midpoint (clamped to 0.0-0.95) |
| `gain` | `float \| None` | `1.0` | Sigmoid gain/slope parameter |
| `num_tones` | `int \| None` | `256` | Number of output tones |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
sig = wbe.sigmoidal_contrast_stretch(image)
sig_strong = wbe.sigmoidal_contrast_stretch(image, cutoff=0.5, gain=10.0, num_tones=512)
```

---

### `wbe.standard_deviation_contrast_stretch`

```
wbe.standard_deviation_contrast_stretch(
    input,
    clip=None,
    num_tones=None,
    output_path=None,
    callback=None,
) -> Raster
```

Performs a linear contrast stretch using clip bounds derived from the mean and standard deviation of the image.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `clip` | `float \| None` | `2.0` | Standard deviation multiplier used to define lower and upper clip bounds |
| `num_tones` | `int \| None` | `256` | Number of output tones |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
sds = wbe.standard_deviation_contrast_stretch(image)
sds3 = wbe.standard_deviation_contrast_stretch(image, clip=3.0, num_tones=512)
```

---

### `wbe.ihs_to_rgb`

```
wbe.ihs_to_rgb(
    intensity,
    hue,
    saturation,
    red_output=None,
    green_output=None,
    blue_output=None,
    callback=None,
) -> dict[str, Raster]
```

Converts intensity, hue, and saturation band rasters back to red, green, and blue channels (0–255).

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `intensity` | `Raster` | required | Intensity band raster (values in 0–1) |
| `hue` | `Raster` | required | Hue band raster (radians, 0–2π) |
| `saturation` | `Raster` | required | Saturation band raster (values in 0–1) |
| `red_output` | `str \| None` | `None` | Output file path for the red band; omit to keep in memory |
| `green_output` | `str \| None` | `None` | Output file path for the green band; omit to keep in memory |
| `blue_output` | `str \| None` | `None` | Output file path for the blue band; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Returns**

A dict with keys `"red"`, `"green"`, `"blue"` each holding a single-band `Raster` with values in 0–255.

**Examples**

```python
bands = wbe.ihs_to_rgb(intensity, hue, saturation)
red, green, blue = bands["red"], bands["green"], bands["blue"]
```

---

### `wbe.rgb_to_ihs`

```
wbe.rgb_to_ihs(
    red=None,
    green=None,
    blue=None,
    composite=None,
    intensity_output=None,
    hue_output=None,
    saturation_output=None,
    callback=None,
) -> dict[str, Raster]
```

Transforms red, green, blue rasters (or a packed RGB composite) to intensity, hue, and saturation components using the HSI colour model.

Pass either three separate `red`/`green`/`blue` single-band rasters (each normalised to 0–1 per-band before conversion) or a single packed `composite` raster.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `red` | `Raster \| None` | `None` | Red-band raster (mutually exclusive with `composite`) |
| `green` | `Raster \| None` | `None` | Green-band raster (mutually exclusive with `composite`) |
| `blue` | `Raster \| None` | `None` | Blue-band raster (mutually exclusive with `composite`) |
| `composite` | `Raster \| None` | `None` | Packed RGB composite raster (mutually exclusive with `red`/`green`/`blue`) |
| `intensity_output` | `str \| None` | `None` | Output file path for the intensity band |
| `hue_output` | `str \| None` | `None` | Output file path for the hue band |
| `saturation_output` | `str \| None` | `None` | Output file path for the saturation band |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Returns**

A dict with keys `"intensity"`, `"hue"`, `"saturation"` each holding a single-band `Raster`.

**Examples**

```python
# From separate bands
ihs = wbe.rgb_to_ihs(red=red_band, green=green_band, blue=blue_band)

# From a packed composite
ihs = wbe.rgb_to_ihs(composite=rgb_image)
intensity = ihs["intensity"]
```

---

### `wbe.split_colour_composite`

```
wbe.split_colour_composite(
    input,
    red_output=None,
    green_output=None,
    blue_output=None,
    callback=None,
) -> dict[str, Raster]
```

Splits a packed RGB colour composite into separate red, green, and blue single-band rasters.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input packed RGB raster |
| `red_output` | `str \| None` | `None` | Output file path for the red band; omit to keep in memory |
| `green_output` | `str \| None` | `None` | Output file path for the green band; omit to keep in memory |
| `blue_output` | `str \| None` | `None` | Output file path for the blue band; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Returns**

A dict with keys `"red"`, `"green"`, `"blue"` each holding a single-band `Raster` with values in 0–255.

**Examples**

```python
bands = wbe.split_colour_composite(rgb_image)
red = bands["red"]
green = bands["green"]
blue = bands["blue"]
```

---

### `wbe.thicken_raster_line`

```
wbe.thicken_raster_line(
    input,
    output_path=None,
    callback=None,
) -> Raster
```

Thickens diagonal single-cell raster line segments to prevent crossing between diagonal foreground cells.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster; positive values are treated as line foreground |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
thickened = wbe.thicken_raster_line(lines)
```

---

### `wbe.tophat_transform`

```
wbe.tophat_transform(
    input,
    filter_size_x=None,
    filter_size_y=None,
    variant=None,
    output_path=None,
    callback=None,
) -> Raster
```

Performs either a white or black morphological top-hat transform.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster |
| `filter_size_x` | `int \| None` | `11` | Odd neighborhood width |
| `filter_size_y` | `int \| None` | `11` | Odd neighborhood height |
| `variant` | `str \| None` | `"white"` | Transform variant: `white` or `black` |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
white_hat = wbe.tophat_transform(image)
black_hat = wbe.tophat_transform(image, filter_size_x=9, filter_size_y=9, variant='black')
```

---

### `wbe.canny_edge_detection`

```python
wbe.canny_edge_detection(
    input,
    sigma=0.5,
    low_threshold=0.05,
    high_threshold=0.15,
    add_back=False,
    output_path=None,
    callback=None,
) -> Raster
```

Applies [Canny edge detection](https://en.wikipedia.org/wiki/Canny_edge_detector) to a single-band or packed-RGB raster.
The algorithm proceeds through four stages: Gaussian smoothing, Sobel gradient computation,
non-maximum suppression, and double-threshold hysteresis.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input` | `Raster` | required | Input raster (single-band or packed RGB) |
| `sigma` | `float` | `0.5` | Standard deviation (in pixels) of the Gaussian smoothing kernel (clamped to 0.15–20) |
| `low_threshold` | `float` | `0.05` | Low hysteresis threshold as a fraction (0–1) of the high threshold |
| `high_threshold` | `float` | `0.15` | High hysteresis threshold as a fraction (0–1) of the peak gradient magnitude |
| `add_back` | `bool` | `False` | If `True`, edge pixels are zeroed in the original image instead of producing a binary edge map |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
edges = wbe.canny_edge_detection(image)
edges = wbe.canny_edge_detection(image, sigma=1.0, low_threshold=0.05, high_threshold=0.15, output_path='edges.tif')
```

---

### `wbe.min_dist_classification`

```python
wbe.min_dist_classification(
    input_rasters,
    training_data,
    class_field_name,
    dist_threshold=None,
    output_path=None,
    callback=None,
) -> Raster
```

Performs a supervised [minimum-distance classification](http://www.50northspatial.org/supervised-image-classification-using-minimum-distance-algorithm/)
on a stack of single-band rasters using polygon training data.  Each unknown pixel is
assigned to the class whose mean spectral vector is closest in Euclidean distance.
An optional z-score threshold (`dist_threshold`) can be used to leave uncertain pixels unclassified.
Output pixel values are 1-based class integers; unclassified pixels carry the nodata value (−32768).

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input_rasters` | `List[Raster]` | required | One single-band raster per spectral band |
| `training_data` | `Vector` | required | Polygon vector containing labelled training areas |
| `class_field_name` | `str` | required | Attribute field name identifying each polygon's class |
| `dist_threshold` | `float \| None` | `None` | Z-score threshold; pixels whose Euclidean distance exceeds this threshold are left unclassified. Omit to classify all pixels. |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
training = wbe.Vector('training.shp')
classified = wbe.min_dist_classification([band1, band2, band3], training, 'class')
classified = wbe.min_dist_classification([band1, band2, band3], training, 'class', dist_threshold=3.0, output_path='classified.tif')
```

---

### `wbe.parallelepiped_classification`

```python
wbe.parallelepiped_classification(
    input_rasters,
    training_data,
    class_field_name,
    output_path=None,
    callback=None,
) -> Raster
```

Performs a supervised [parallelepiped classification](http://www.50northspatial.org/supervised-image-classification-using-parallelepiped-algorithm/)
on a stack of single-band rasters using polygon training data.  For each class, the minimum and
maximum pixel values from the training areas define a multi-dimensional hyper-rectangular decision
region (parallelepiped).  A pixel is assigned to the first class (sorted by smallest spectral
volume) whose parallelepiped contains the pixel's feature vector.
Pixels that do not fall within any class parallelepiped are left unclassified (nodata = −32768).

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input_rasters` | `List[Raster]` | required | One single-band raster per spectral band |
| `training_data` | `Vector` | required | Polygon vector containing labelled training areas |
| `class_field_name` | `str` | required | Attribute field name identifying each polygon's class |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
training = wbe.Vector('training.shp')
classified = wbe.parallelepiped_classification([band1, band2, band3], training, 'class')
classified = wbe.parallelepiped_classification([band1, band2, band3], training, 'class', output_path='classified.tif')
```

---

### `wbe.evaluate_training_sites`

```python
wbe.evaluate_training_sites(
    input_rasters,
    training_data,
    class_field_name,
    output_path=None,
    callback=None,
) -> str
```

Evaluates class separability for polygon training sites across one or more spectral bands,
and writes an HTML report containing per-class, per-band distribution statistics
(sample count, min, quartiles, median, max, mean, and standard deviation).

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input_rasters` | `List[Raster]` | required | One single-band raster per spectral band |
| `training_data` | `Vector` | required | Polygon vector containing labelled training areas |
| `class_field_name` | `str` | required | Attribute field name identifying each polygon's class |
| `output_path` | `str \| None` | `None` | Output HTML file path; default is `training_sites_report.html` in the working directory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
training = wbe.Vector('training.shp')
report = wbe.evaluate_training_sites([band1, band2, band3], training, 'class')
report = wbe.evaluate_training_sites([band1, band2, band3], training, 'class', output_path='training_eval.html')
```

---

### `wbe.generalize_with_similarity`

```python
wbe.generalize_with_similarity(
    raster,
    similarity_rasters,
    min_size=5,
    output_path=None,
    callback=None,
) -> Raster
```

Generalizes a classified raster by identifying small contiguous patches and merging each into
a neighboring patch with the most similar multi-band feature center in standardized similarity space.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `raster` | `Raster` | required | Input classified raster |
| `similarity_rasters` | `List[Raster]` | required | One or more rasters used to compute inter-feature similarity |
| `min_size` | `int` | `5` | Minimum feature size (pixels); smaller features are merged |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
generalized = wbe.generalize_with_similarity(classes, [band1, band2, band3])
generalized = wbe.generalize_with_similarity(classes, [band1, band2, band3], min_size=8, output_path='generalized_similarity.tif')
```

---

### `wbe.image_segmentation`

```python
wbe.image_segmentation(
    input_rasters,
    threshold=0.5,
    steps=10,
    min_area=4,
    output_path=None,
    callback=None,
) -> Raster
```

Segments a multi-band raster stack into contiguous, relatively homogeneous regions using
seeded region growing in standardized feature space. An optional minimum-area post-process
can merge undersized regions into similar neighbors.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input_rasters` | `List[Raster]` | required | One single-band raster per input band |
| `threshold` | `float` | `0.5` | Region-growing distance threshold in standardized feature space |
| `steps` | `int` | `10` | Number of seed-priority levels; higher values provide finer seed stratification |
| `min_area` | `int` | `4` | Minimum segment area in pixels; smaller segments are merged in cleanup |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
segments = wbe.image_segmentation([band1, band2, band3])
segments = wbe.image_segmentation([band1, band2, band3], threshold=0.45, steps=12, min_area=6, output_path='segments.tif')
```

---

### `wbe.fuzzy_knn_classification`

```python
wbe.fuzzy_knn_classification(
    input_rasters,
    training_data,
    class_field_name,
    scaling_method="none",
    k=5,
    m=2.0,
    output_path=None,
    probability_output_path=None,
    callback=None,
) -> Tuple[Raster, Raster]
```

Performs fuzzy k-nearest-neighbor classification and returns both a crisp class raster
and a membership-probability raster (the winning-class membership per cell).

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input_rasters` | `List[Raster]` | required | One single-band raster per feature band |
| `training_data` | `Vector` | required | Point/polygon training vector with class labels |
| `class_field_name` | `str` | required | Class field name in training attributes |
| `scaling_method` | `str` | `"none"` | Feature scaling mode: `"none"`, `"normalize"`, `"standardize"` |
| `k` | `int` | `5` | Number of neighbors |
| `m` | `float` | `2.0` | Fuzzy exponent parameter (> 1) |
| `output_path` | `str \| None` | `None` | Optional classified raster output path |
| `probability_output_path` | `str \| None` | `None` | Optional probability raster output path |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
classified, probability = wbe.fuzzy_knn_classification([band1, band2, band3], training, 'class')
classified, probability = wbe.fuzzy_knn_classification(
    [band1, band2, band3],
    training,
    'class',
    scaling_method='standardize',
    k=7,
    m=2.0,
    output_path='fuzzy_knn_classified.tif',
    probability_output_path='fuzzy_knn_probability.tif',
)
```

---

### `wbe.knn_classification`

```python
wbe.knn_classification(
    input_rasters,
    training_data,
    class_field_name,
    scaling_method="none",
    k=5,
    use_clipping=False,
    output_path=None,
    callback=None,
) -> Raster
```

Performs supervised k-nearest-neighbor classification on a multi-band raster stack.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input_rasters` | `List[Raster]` | required | One single-band raster per feature band |
| `training_data` | `Vector` | required | Point/polygon training vector with class labels |
| `class_field_name` | `str` | required | Class field name in training attributes |
| `scaling_method` | `str` | `"none"` | Feature scaling mode: `"none"`, `"normalize"`, `"standardize"` |
| `k` | `int` | `5` | Number of neighbors |
| `use_clipping` | `bool` | `False` | If `True`, removes misclassified training samples using leave-one-out pre-clipping |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
classified = wbe.knn_classification([band1, band2, band3], training, 'class')
classified = wbe.knn_classification(
    [band1, band2, band3],
    training,
    'class',
    scaling_method='standardize',
    k=7,
    use_clipping=True,
    output_path='knn_classified.tif',
)
```

---

### `wbe.knn_regression`

```python
wbe.knn_regression(
    input_rasters,
    training_data,
    field_name,
    scaling_method="none",
    k=5,
    distance_weighting=False,
    output_path=None,
    callback=None,
) -> Raster
```

Performs supervised k-nearest-neighbor regression using point-based training targets.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input_rasters` | `List[Raster]` | required | One single-band raster per feature band |
| `training_data` | `Vector` | required | Point training vector with numeric target values |
| `field_name` | `str` | required | Numeric target field name in training attributes |
| `scaling_method` | `str` | `"none"` | Feature scaling mode: `"none"`, `"normalize"`, `"standardize"` |
| `k` | `int` | `5` | Number of neighbors |
| `distance_weighting` | `bool` | `False` | If `True`, predictions use inverse-distance weighted averaging |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
pred = wbe.knn_regression([band1, band2, band3], training_points, 'value')
pred = wbe.knn_regression(
    [band1, band2, band3],
    training_points,
    'value',
    scaling_method='standardize',
    k=8,
    distance_weighting=True,
    output_path='knn_regression.tif',
)
```

---

### `wbe.logistic_regression`

```python
wbe.logistic_regression(
    input_rasters,
    training_data,
    class_field_name,
    scaling_method="none",
    alpha=0.0,
    output_path=None,
    callback=None,
) -> Raster
```

Performs supervised logistic regression classification on multi-band predictors.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input_rasters` | `List[Raster]` | required | One single-band raster per feature band |
| `training_data` | `Vector` | required | Point/polygon training vector with class labels |
| `class_field_name` | `str` | required | Class field name in training attributes |
| `scaling_method` | `str` | `"none"` | Feature scaling mode: `"none"`, `"normalize"`, `"standardize"` |
| `alpha` | `float` | `0.0` | L2 regularization weight |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
classified = wbe.logistic_regression([band1, band2, band3], training, 'class')
classified = wbe.logistic_regression(
    [band1, band2, band3],
    training,
    'class',
    scaling_method='standardize',
    alpha=0.1,
    output_path='logistic_regression.tif',
)
```

---

### `wbe.svm_classification`

```python
wbe.svm_classification(
    input_rasters,
    training_data,
    class_field_name,
    scaling_method="none",
    kernel="linear",
    c=1.0,
    gamma=None,
    epoch=2,
    output_path=None,
    callback=None,
) -> Raster
```

Performs supervised support-vector-machine classification using one-vs-rest voting for multiclass labels.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input_rasters` | `List[Raster]` | required | One single-band raster per feature band |
| `training_data` | `Vector` | required | Point/polygon training vector with class labels |
| `class_field_name` | `str` | required | Class field name in training attributes |
| `scaling_method` | `str` | `"none"` | Feature scaling mode: `"none"`, `"normalize"`, `"standardize"` |
| `kernel` | `str` | `"linear"` | SVM kernel: `"linear"` or `"rbf"` |
| `c` | `float` | `1.0` | Regularization parameter |
| `gamma` | `float \| None` | `None` | RBF gamma; defaults to `1 / n_features` when omitted |
| `epoch` | `int` | `2` | Number of training epochs |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
classified = wbe.svm_classification([band1, band2, band3], training, 'class')
classified = wbe.svm_classification(
    [band1, band2, band3],
    training,
    'class',
    scaling_method='standardize',
    kernel='rbf',
    c=2.0,
    gamma=0.25,
    epoch=3,
    output_path='svm_classified.tif',
)
```

---

### `wbe.svm_regression`

```python
wbe.svm_regression(
    input_rasters,
    training_data,
    field_name,
    scaling_method="none",
    kernel="linear",
    c=1.0,
    gamma=None,
    eps=0.1,
    tol=1e-3,
    output_path=None,
    callback=None,
) -> Raster
```

Performs supervised support-vector-machine regression on multi-band predictors.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input_rasters` | `List[Raster]` | required | One single-band raster per feature band |
| `training_data` | `Vector` | required | Point training vector with numeric target values |
| `field_name` | `str` | required | Numeric target field name in training attributes |
| `scaling_method` | `str` | `"none"` | Feature scaling mode: `"none"`, `"normalize"`, `"standardize"` |
| `kernel` | `str` | `"linear"` | SVM kernel: `"linear"` or `"rbf"` |
| `c` | `float` | `1.0` | Regularization parameter |
| `gamma` | `float \| None` | `None` | RBF gamma; defaults to `1 / n_features` when omitted |
| `eps` | `float` | `0.1` | Epsilon-insensitive loss width |
| `tol` | `float` | `1e-3` | Optimizer convergence tolerance |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
pred = wbe.svm_regression([band1, band2, band3], training_points, 'value')
pred = wbe.svm_regression(
    [band1, band2, band3],
    training_points,
    'value',
    scaling_method='standardize',
    kernel='rbf',
    c=2.0,
    gamma=0.25,
    eps=0.05,
    tol=1e-3,
    output_path='svm_regression.tif',
)
```

---

### `wbe.random_forest_classification`

```python
wbe.random_forest_classification(
    input_rasters,
    training_data,
    class_field_name,
    scaling_method="none",
    n_trees=200,
    min_samples_leaf=1,
    min_samples_split=2,
    output_path=None,
    callback=None,
) -> Raster
```

Performs supervised random forest classification using point/polygon training data.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input_rasters` | `List[Raster]` | required | One single-band raster per feature band |
| `training_data` | `Vector` | required | Point/polygon training vector with class labels |
| `class_field_name` | `str` | required | Class field name in training attributes |
| `scaling_method` | `str` | `"none"` | Feature scaling mode: `"none"`, `"normalize"`, `"standardize"` |
| `n_trees` | `int` | `200` | Number of trees in the forest |
| `min_samples_leaf` | `int` | `1` | Minimum number of samples at each leaf |
| `min_samples_split` | `int` | `2` | Minimum number of samples to split a node |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
classified = wbe.random_forest_classification([band1, band2, band3], training, 'class')
classified = wbe.random_forest_classification(
    [band1, band2, band3],
    training,
    'class',
    scaling_method='standardize',
    n_trees=300,
    min_samples_leaf=1,
    min_samples_split=2,
    output_path='rf_classified.tif',
)
```

---

### `wbe.random_forest_regression`

```python
wbe.random_forest_regression(
    input_rasters,
    training_data,
    field_name,
    scaling_method="none",
    n_trees=200,
    min_samples_leaf=1,
    min_samples_split=2,
    output_path=None,
    callback=None,
) -> Raster
```

Performs supervised random forest regression using point training targets.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input_rasters` | `List[Raster]` | required | One single-band raster per feature band |
| `training_data` | `Vector` | required | Point training vector with numeric target values |
| `field_name` | `str` | required | Numeric target field name in training attributes |
| `scaling_method` | `str` | `"none"` | Feature scaling mode: `"none"`, `"normalize"`, `"standardize"` |
| `n_trees` | `int` | `200` | Number of trees in the forest |
| `min_samples_leaf` | `int` | `1` | Minimum number of samples at each leaf |
| `min_samples_split` | `int` | `2` | Minimum number of samples to split a node |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
pred = wbe.random_forest_regression([band1, band2, band3], training_points, 'value')
pred = wbe.random_forest_regression(
    [band1, band2, band3],
    training_points,
    'value',
    scaling_method='standardize',
    n_trees=300,
    min_samples_leaf=1,
    min_samples_split=2,
    output_path='rf_regression.tif',
)
```

---

### `wbe.random_forest_classification_fit`

```python
wbe.random_forest_classification_fit(
    input_rasters,
    training_data,
    class_field_name,
    scaling_method="none",
    split_criterion="gini",
    n_trees=200,
    min_samples_leaf=1,
    min_samples_split=2,
    test_proportion=0.2,
    callback=None,
) -> List[int]
```

Fits a random forest classifier and returns serialized model bytes for later prediction.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input_rasters` | `List[Raster]` | required | One single-band raster per feature band |
| `training_data` | `Vector` | required | Point/polygon training vector with class labels |
| `class_field_name` | `str` | required | Class field name in training attributes |
| `scaling_method` | `str` | `"none"` | Feature scaling mode: `"none"`, `"normalize"`, `"standardize"` |
| `split_criterion` | `str` | `"gini"` | Legacy split criterion argument for compatibility |
| `n_trees` | `int` | `200` | Number of trees in the forest |
| `min_samples_leaf` | `int` | `1` | Minimum number of samples at each leaf |
| `min_samples_split` | `int` | `2` | Minimum number of samples to split a node |
| `test_proportion` | `float` | `0.2` | Legacy compatibility parameter for train/test split workflows |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
model_bytes = wbe.random_forest_classification_fit([band1, band2, band3], training, 'class')
```

---

### `wbe.random_forest_classification_predict`

```python
wbe.random_forest_classification_predict(
    input_rasters,
    model_bytes,
    output_path=None,
    callback=None,
) -> Raster
```

Applies a fitted random forest classification model (byte-array payload) to predictor rasters.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input_rasters` | `List[Raster]` | required | One single-band raster per feature band |
| `model_bytes` | `List[int]` | required | Model bytes returned by `wbe.random_forest_classification_fit` |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
model_bytes = wbe.random_forest_classification_fit([band1, band2, band3], training, 'class')
classified = wbe.random_forest_classification_predict([band1, band2, band3], model_bytes)
```

---

### `wbe.random_forest_regression_fit`

```python
wbe.random_forest_regression_fit(
    input_rasters,
    training_data,
    field_name,
    scaling_method="none",
    n_trees=200,
    min_samples_leaf=1,
    min_samples_split=2,
    test_proportion=0.2,
    callback=None,
) -> List[int]
```

Fits a random forest regressor and returns serialized model bytes for later prediction.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input_rasters` | `List[Raster]` | required | One single-band raster per feature band |
| `training_data` | `Vector` | required | Point training vector with numeric target values |
| `field_name` | `str` | required | Numeric target field name in training attributes |
| `scaling_method` | `str` | `"none"` | Feature scaling mode: `"none"`, `"normalize"`, `"standardize"` |
| `n_trees` | `int` | `200` | Number of trees in the forest |
| `min_samples_leaf` | `int` | `1` | Minimum number of samples at each leaf |
| `min_samples_split` | `int` | `2` | Minimum number of samples to split a node |
| `test_proportion` | `float` | `0.2` | Legacy compatibility parameter for train/test split workflows |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
model_bytes = wbe.random_forest_regression_fit([band1, band2, band3], training_points, 'value')
```

---

### `wbe.random_forest_regression_predict`

```python
wbe.random_forest_regression_predict(
    input_rasters,
    model_bytes,
    output_path=None,
    callback=None,
) -> Raster
```

Applies a fitted random forest regression model (byte-array payload) to predictor rasters.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input_rasters` | `List[Raster]` | required | One single-band raster per feature band |
| `model_bytes` | `List[int]` | required | Model bytes returned by `wbe.random_forest_regression_fit` |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
model_bytes = wbe.random_forest_regression_fit([band1, band2, band3], training_points, 'value')
pred = wbe.random_forest_regression_predict([band1, band2, band3], model_bytes)
```

---

### `wbe.nnd_classification`

```python
wbe.nnd_classification(
    input_rasters,
    training_data,
    class_field_name,
    scaling_method="none",
    z_threshold=1.96,
    outlier_is_zero=True,
    k=25,
    output_path=None,
    callback=None,
) -> Raster
```

Performs nearest-normalized-distance classification. For each class, distances are
normalized by within-class distance statistics; optional thresholding can flag outliers.

**Parameters**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `input_rasters` | `List[Raster]` | required | One single-band raster per feature band |
| `training_data` | `Vector` | required | Point/polygon training vector with class labels |
| `class_field_name` | `str` | required | Class field name in training attributes |
| `scaling_method` | `str` | `"none"` | Feature scaling mode: `"none"`, `"normalize"`, `"standardize"` |
| `z_threshold` | `float` | `1.96` | Outlier threshold in normalized-distance units |
| `outlier_is_zero` | `bool` | `True` | If `True`, outliers are encoded as class `0`; otherwise as nodata |
| `k` | `int` | `25` | Neighborhood size used in class-distance estimates |
| `output_path` | `str \| None` | `None` | Output file path; omit to keep in memory |
| `callback` | `callable \| None` | `None` | Progress/message event handler |

**Examples**

```python
classified = wbe.nnd_classification([band1, band2, band3], training, 'class')
classified = wbe.nnd_classification(
    [band1, band2, band3],
    training,
    'class',
    scaling_method='standardize',
    z_threshold=2.0,
    outlier_is_zero=True,
    k=25,
    output_path='nnd_classified.tif',
)
```

---
