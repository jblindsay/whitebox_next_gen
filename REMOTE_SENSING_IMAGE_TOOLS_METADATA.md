# Remote Sensing Image Processing Tools - Metadata Summary
**Generated:** 2026-06-04  
**Source:** wbtools_oss remote_sensing module  
**Purpose:** Documentation enhancement - Tool IDs and current summaries

---

## Convolution Filters (Edge Detection & Sharpening)

| Tool ID | Display Name | Current Metadata.Summary | Current Manifest.Summary |
|---------|--------------|------------------------|--------------------------|
| `high_pass_filter` | High-Pass Filter | Performs high-pass filtering using neighborhood mean subtraction. | Performs high-pass filtering using neighborhood mean subtraction. |
| `laplacian_filter` | Laplacian Filter | Performs Laplacian edge/sharpen filtering. | Performs Laplacian edge/sharpen filtering. |
| `sobel_filter` | Sobel Filter | Performs Sobel edge detection. | Performs Sobel edge detection. |
| `prewitt_filter` | Prewitt Filter | Performs Prewitt edge detection. | Performs Prewitt edge detection. |

---

## Convolution Extra Filters (Additional Edge & Emboss)

| Tool ID | Display Name | Current Metadata.Summary | Current Manifest.Summary |
|---------|--------------|------------------------|--------------------------|
| `scharr_filter` | Scharr Filter | Performs Scharr edge detection. | Performs Scharr edge detection. |
| `roberts_cross_filter` | Roberts Cross Filter | Performs Roberts Cross edge detection. | Performs Roberts Cross edge detection. |
| `line_detection_filter` | Line Detection Filter | Performs directional line detection. | Performs directional line detection. |
| `emboss_filter` | Emboss Filter | Performs directional emboss filtering. | Performs directional emboss filtering. |
| `user_defined_weights_filter` | User Defined Weights Filter | Applies a user-defined convolution kernel. | Applies a user-defined convolution kernel. |

---

## Rank-Based Filters (Moving Window Rank Operations)

| Tool ID | Display Name | Current Metadata.Summary | Current Manifest.Summary |
|---------|--------------|------------------------|--------------------------|
| `median_filter` | Median Filter | Computes moving-window median values. | Computes moving-window median values. |
| `percentile_filter` | Percentile Filter | Computes center-cell percentile rank in a moving window. | Computes center-cell percentile rank in a moving window. |
| `majority_filter` | Majority Filter | Computes moving-window mode (majority class/value). | Computes moving-window mode (majority class/value). |
| `diversity_filter` | Diversity Filter | Computes moving-window diversity (count of unique values). | Computes moving-window diversity (count of unique values). |

---

## Window Statistics Filters (Moving Window Statistical Operations)

| Tool ID | Display Name | Current Metadata.Summary | Current Manifest.Summary |
|---------|--------------|------------------------|--------------------------|
| `mean_filter` | Mean Filter | Computes a moving-window mean for each raster cell. | Computes a moving-window mean for each raster cell. |
| `total_filter` | Total Filter | Computes a moving-window total for each raster cell. | Computes a moving-window total for each raster cell. |
| `standard_deviation_filter` | Standard Deviation Filter | Computes a moving-window standard deviation for each raster cell. | Computes a moving-window standard deviation for each raster cell. |
| `minimum_filter` | Minimum Filter | Computes a moving-window minimum for each raster cell. | Computes a moving-window minimum for each raster cell. |
| `maximum_filter` | Maximum Filter | Computes a moving-window maximum for each raster cell. | Computes a moving-window maximum for each raster cell. |
| `range_filter` | Range Filter | Computes a moving-window range (max-min) for each raster cell. | Computes a moving-window range (max-min) for each raster cell. |

---

## Phase 3 Filters (Advanced Smoothing & Enhancement)

| Tool ID | Display Name | Current Metadata.Summary | Current Manifest.Summary |
|---------|--------------|------------------------|--------------------------|
| `fast_almost_gaussian_filter` | Fast Almost Gaussian Filter | Performs a fast approximation to Gaussian smoothing. | Performs a fast approximation to Gaussian smoothing. |
| `edge_preserving_mean_filter` | Edge Preserving Mean Filter | Performs thresholded edge-preserving mean filtering. | Performs thresholded edge-preserving mean filtering. |
| `unsharp_masking` | Unsharp Masking | Performs edge-enhancing unsharp masking. | Performs edge-enhancing unsharp masking. |
| `diff_of_gaussians_filter` | Difference of Gaussians Filter | Performs Difference-of-Gaussians band-pass filtering. | Performs Difference-of-Gaussians band-pass filtering. |
| `adaptive_filter` | Adaptive Filter | Performs adaptive thresholded mean replacement based on local z-scores. | Performs adaptive thresholded mean replacement based on local z-scores. |
| `lee_filter` | Lee Filter | Performs Lee sigma filtering using in-range neighborhood averaging. | Performs Lee sigma filtering using in-range neighborhood averaging. |
| `refined_lee_filter` | Refined Lee Filter | Performs Refined Lee filtering with edge-preserving sub-window homogeneity classification. | Performs Refined Lee filtering with edge-preserving sub-window homogeneity classification. |
| `enhanced_lee_filter` | Enhanced Lee Filter | Performs Enhanced Lee filtering using sigma-ratio weighting and ENL-dependent blending. | Performs Enhanced Lee filtering using sigma-ratio weighting and ENL-dependent blending. |
| `conservative_smoothing_filter` | Conservative Smoothing Filter | Performs conservative smoothing by clipping impulse outliers to neighborhood bounds. | Performs conservative smoothing by clipping impulse outliers to neighborhood bounds. |
| `olympic_filter` | Olympic Filter | Performs Olympic smoothing by averaging local values excluding min and max. | Performs Olympic smoothing by averaging local values excluding min and max. |
| `k_nearest_mean_filter` | K-Nearest Mean Filter | Performs edge-preserving k-nearest neighbor mean smoothing. | Performs edge-preserving k-nearest neighbor mean smoothing. |
| `high_pass_median_filter` | High-Pass Median Filter | Performs high-pass filtering by subtracting local median from center values. | Performs high-pass filtering by subtracting local median from center values. |
| `laplacian_of_gaussians_filter` | Laplacian of Gaussians Filter | Performs Laplacian-of-Gaussians edge enhancement. | Performs Laplacian-of-Gaussians edge enhancement. |

---

## Gaussian Filter

| Tool ID | Display Name | Current Metadata.Summary | Current Manifest.Summary |
|---------|--------------|------------------------|--------------------------|
| `gaussian_filter` | Gaussian Filter | Performs Gaussian smoothing on a raster image. | Performs Gaussian smoothing on a raster image. |

---

## Bilateral Filter (Edge-Preserving Smoothing)

| Tool ID | Display Name | Current Metadata.Summary | Current Manifest.Summary |
|---------|--------------|------------------------|--------------------------|
| `bilateral_filter` | Bilateral Filter | Performs an edge-preserving bilateral smoothing filter on a raster image. | Performs an edge-preserving bilateral smoothing filter on a raster image. |
| `high_pass_bilateral_filter` | High Pass Bilateral Filter | Performs high-pass bilateral filtering. | (Inferred: High-pass variant of bilateral filtering) |

---

## Advanced Filters (Specialized Denoising & Enhancement)

| Tool ID | Display Name | Current Metadata.Summary | Current Manifest.Summary |
|---------|--------------|------------------------|--------------------------|
| `anisotropic_diffusion_filter` | Anisotropic Diffusion Filter | Performs Perona-Malik edge-preserving anisotropic diffusion smoothing. | Performs Perona-Malik edge-preserving anisotropic diffusion smoothing. |
| `gamma_correction` | Gamma Correction | Applies gamma intensity correction to grayscale or RGB imagery. | Applies gamma intensity correction to grayscale or RGB imagery. |
| `guided_filter` | Guided Filter | Performs edge-preserving guided filtering using local linear models. | Performs edge-preserving guided filtering using local linear models. |
| `wiener_filter` | Wiener Filter | Performs adaptive Wiener denoising using local mean and variance. | Performs adaptive Wiener denoising using local mean and variance. |
| `non_local_means_filter` | Non-Local Means Filter | Performs non-local means denoising using patch similarity weighting. | Performs non-local means denoising using patch similarity weighting. |
| `kuwahara_filter` | Kuwahara Filter | Performs edge-preserving Kuwahara filtering using minimum-variance subwindows. | Performs edge-preserving Kuwahara filtering using minimum-variance subwindows. |
| `frost_filter` | Frost Filter | Performs adaptive Frost speckle filtering for radar imagery. | Performs adaptive Frost speckle filtering for radar imagery. |
| `gamma_map_filter` | Gamma-MAP Filter | Performs Gamma-MAP speckle filtering for radar imagery. | Performs Gamma-MAP speckle filtering for radar imagery. |
| `kuan_filter` | Kuan Filter | Performs Kuan speckle filtering for radar imagery. | Performs Kuan speckle filtering for radar imagery. |
| `gabor_filter_bank` | Gabor Filter Bank | Performs multi-orientation Gabor response filtering. | Performs multi-orientation Gabor response filtering. |
| `frangi_filter` | Frangi Filter | Performs multiscale Frangi vesselness enhancement. | Performs multiscale Frangi vesselness enhancement. |
| `savitzky_golay_2d_filter` | Savitzky-Golay 2D Filter | Performs 2D Savitzky-Golay smoothing. | Performs 2D Savitzky-Golay smoothing. |

---

## Non-Filter Image Processing Tools (Contrast, Thresholding, Morphology, Classification)

### Contrast Enhancement & Stretching

| Tool ID | Display Name | Current Metadata.Summary | Current Manifest.Summary |
|---------|--------------|------------------------|--------------------------|
| `balance_contrast_enhancement` | Balance Contrast Enhancement | Reduces colour bias in a packed RGB image using per-channel parabolic stretches. | (Same as metadata) |
| `gaussian_contrast_stretch` | Gaussian Contrast Stretch | Stretches contrast by matching to a Gaussian reference distribution. | (Same as metadata) |
| `min_max_contrast_stretch` | Min-Max Contrast Stretch | Linearly stretches values between user-specified minimum and maximum. | (Same as metadata) |
| `percentage_contrast_stretch` | Percentage Contrast Stretch | Performs linear contrast stretch with percentile clipping. | (Same as metadata) |
| `piecewise_contrast_stretch` | Piecewise Contrast Stretch | (To be extracted from individual tool implementation) | (To be extracted) |
| `sigmoid_contrast_stretch` | Sigmoidal Contrast Stretch | Performs sigmoidal contrast stretching using gain and cutoff. | (Same as metadata) |
| `standard_deviation_contrast_stretch` | Standard Deviation Contrast Stretch | Performs linear contrast stretch using mean plus/minus a standard deviation multiplier. | (Same as metadata) |
| `direct_decorrelation_stretch` | Direct Decorrelation Stretch | Improves packed RGB colour saturation by reducing the achromatic component and linearly stretching channels. | (Same as metadata) |
| `histogram_equalization` | Histogram Equalization | Applies histogram equalization to improve image contrast. | (Same as metadata) |
| `histogram_matching` | Histogram Matching | Matches an image histogram to a supplied reference histogram. | (Same as metadata) |
| `histogram_matching_two_images` | Histogram Matching Two Images | Matches an input image histogram to a reference image histogram. | (Same as metadata) |

### Thresholding & Feature Extraction

| Tool ID | Display Name | Current Metadata.Summary | Current Manifest.Summary |
|---------|--------------|------------------------|--------------------------|
| `otsu_thresholding` | Otsu Thresholding | Applies Otsu's automatic thresholding to create a binary raster. | (Same as metadata) |
| `corner_detection` | Corner Detection | Identifies corner patterns in binary rasters using hit-and-miss templates. | (Same as metadata) |
| `canny_edge_detection` | Canny Edge Detection | (To be extracted from non_filter_tools.rs) | (To be extracted) |

### Morphological Operations

| Tool ID | Display Name | Current Metadata.Summary | Current Manifest.Summary |
|---------|--------------|------------------------|--------------------------|
| `opening` | Opening | Performs a morphological opening operation using a rectangular structuring element. | (Same as metadata) |
| `closing` | Closing | Performs a morphological closing operation using a rectangular structuring element. | (Same as metadata) |
| `tophat_transform` | Top-Hat Transform | Performs a white or black morphological top-hat transform. | (Same as metadata) |
| `line_thinning` | Line Thinning | Reduces connected binary raster features to one-cell-wide skeleton lines. | (Same as metadata) |
| `thicken_raster_line` | Thicken Raster Line | Thickens diagonal raster line segments to prevent diagonal leak-through. | (Same as metadata) |
| `remove_spurs` | Remove Spurs | Removes short spur artifacts from binary raster features by iterative pruning. | (Same as metadata) |

### Spectral Indexing & Normalization

| Tool ID | Display Name | Current Metadata.Summary | Current Manifest.Summary |
|---------|--------------|------------------------|--------------------------|
| `normalized_difference_index` | Normalized Difference Index | Computes (band1 - band2) / (band1 + band2) from a multiband raster. | (Same as metadata) |
| `integral_image_transform` | Integral Image Transform | Computes a summed-area (integral image) transform for each band. | (Same as metadata) |

### Image Manipulation & Color Space

| Tool ID | Display Name | Current Metadata.Summary | Current Manifest.Summary |
|---------|--------------|------------------------|--------------------------|
| `flip_image` | Flip Image | Flips an image vertically, horizontally, or both. | (Same as metadata) |
| `create_colour_composite` | Create Colour Composite | Creates a packed RGB colour composite from red, green, blue, and optional opacity rasters. | (Same as metadata) |
| `split_colour_composite` | Split Colour Composite | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `rgb_to_ihs` | RGB to IHS | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `ihs_to_rgb` | IHS to RGB | (To be extracted from non_filter_tools.rs) | (To be extracted) |

### Geospatial Mosaicing & Resampling

| Tool ID | Display Name | Current Metadata.Summary | Current Manifest.Summary |
|---------|--------------|------------------------|--------------------------|
| `mosaic` | Mosaic | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `mosaic_with_feathering` | Mosaic with Feathering | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `resample` | Resample | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `panchromatic_sharpening` | Panchromatic Sharpening | (To be extracted from non_filter_tools.rs) | (To be extracted) |

### Image Analysis & Visualization

| Tool ID | Display Name | Current Metadata.Summary | Current Manifest.Summary |
|---------|--------------|------------------------|--------------------------|
| `image_slider` | Image Slider | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `image_stack_profile` | Image Stack Profile | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `evaluate_training_sites` | Evaluate Training Sites | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `correct_vignetting` | Correct Vignetting | (To be extracted from non_filter_tools.rs) | (To be extracted) |

### Classification & Clustering

| Tool ID | Display Name | Current Metadata.Summary | Current Manifest.Summary |
|---------|--------------|------------------------|--------------------------|
| `k_means_clustering` | K Means Clustering | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `modified_k_means_clustering` | Modified K Means Clustering | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `knn_classification` | KNN Classification | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `knn_regression` | KNN Regression | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `fuzzy_knn_classification` | Fuzzy KNN Classification | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `min_dist_classification` | Min Dist Classification | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `parallelepiped_classification` | Parallelepiped Classification | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `nnd_classification` | NND Classification | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `logistic_regression` | Logistic Regression | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `random_forest_classification` | Random Forest Classification | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `random_forest_regression` | Random Forest Regression | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `random_forest_classification_fit` | Random Forest Classification Fit | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `random_forest_classification_predict` | Random Forest Classification Predict | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `random_forest_regression_fit` | Random Forest Regression Fit | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `random_forest_regression_predict` | Random Forest Regression Predict | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `svm_classification` | SVM Classification | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `svm_regression` | SVM Regression | (To be extracted from non_filter_tools.rs) | (To be extracted) |

### Generalization & Segmentation

| Tool ID | Display Name | Current Metadata.Summary | Current Manifest.Summary |
|---------|--------------|------------------------|--------------------------|
| `generalize_classified_raster` | Generalize Classified Raster | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `generalize_with_similarity` | Generalize with Similarity | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `image_segmentation` | Image Segmentation | (To be extracted from non_filter_tools.rs) | (To be extracted) |
| `change_vector_analysis` | Change Vector Analysis | (To be extracted from non_filter_tools.rs) | (To be extracted) |

### Memory & Utility

| Tool ID | Display Name | Current Metadata.Summary | Current Manifest.Summary |
|---------|--------------|------------------------|--------------------------|
| `write_function_memory_insertion` | Write Function Memory Insertion | (To be extracted from non_filter_tools.rs) | (To be extracted) |

---

## Radiometric & Atmospheric Tools (Multi-Spectral Calibration)

### Status: To be extracted from radiometric_tools.rs
- `dark_object_subtraction`
- `dn_to_toa_reflectance`
- `image_difference_change_detection`
- `pca_based_change_detection`
- `post_classification_change`
- `linear_spectral_unmixing`
- `continuum_removal`
- `cloude_pottier_decomposition`
- `freeman_durden_decomposition`
- `yamaguchi_4component_decomposition`
- `h_alpha_wisart_classification`
- `wisart_iterative_clustering`
- `spectral_angle_mapper`
- `spectral_library_matching`
- `minimum_noise_fraction`
- `ndvi_based_emissivity`
- `land_surface_temperature_single_channel`
- `land_surface_temperature_split_window`

---

## Texture & GLCM Tools

### Status: To be extracted from texture_glcm_tool.rs
- `glcm_texture`

---

## Object-Based Image Analysis (OBIA) Tools

### Status: To be extracted from obia_tools.rs  
- `segment_slic_superpixels`
- `segment_graph_felzenszwalb`
- `segment_watershed_markers`
- `segment_multiresolution_hierarchical`
- `segment_scale_parameter_optimizer`
- `segments_merge_small_regions`
- `segments_split_low_cohesion`
- `segments_to_polygons`
- `polygons_to_segments`
- `object_features_spectral_basic`
- `object_features_shape_basic`
- `object_features_texture_glcm_basic`
- `object_features_context_neighbors`
- `object_features_topology_relations`
- `object_class_probability_maps`
- `classify_objects_random_forest`
- `classify_objects_svm`
- `classify_objects_ensemble_pro`
- `classify_objects_rules_basic`
- `classify_objects_rules_hierarchical`
- `objects_enforce_min_mapping_unit`
- `objects_boundary_refinement_pro`
- `evaluate_segmentation_quality_pro`
- `build_object_hierarchy_multiscale`
- `propagate_labels_across_hierarchy`
- `evaluate_object_classification_accuracy`
- `obia_pipeline_basic`
- `obia_batch_orchestrator_pro`
- `obia_audit_report_pro`
- `object_uncertainty_diagnostics_pro`

---

## Geospatial Tools (Georeference, Orthorectify, BRDF, TerrainCorrected)

### Status: To be extracted from specialized module files
- `brdf_normalization`
- `terrain_corrected_optical_analytics`
- `orthorectification`
- `georeference_raster_from_control_points`

---

## Notes on Current Status

- ✅ **Extracted:** 
  - Convolution Filters (4 tools)
  - Rank-Based Filters (4 tools)
  - Window Statistics Filters (6 tools)
  - Phase 3 Filters (13 tools)
  - Gaussian Filter (1 tool)
  - Bilateral Filter (2 tools)
  - Advanced Filters (12 tools)
  - Convolution Extra Filters (5 tools)
  - Non-Filter Tools - Basic Operations (22 tools)
  - **Total Extracted: 69 tools with metadata/summaries**

- ⏳ **Partially Extracted:** Non-filter tools requiring individual metadata() implementations
  - Classification tools (17 tools)
  - Mosaicing & Resampling (4 tools)
  - Image Analysis (4 tools)
  - Color space operations (3 tools)
  - Segmentation tools (3 tools)

- ⏳ **Pending:** Complete extraction from:
  - radiometric_tools.rs (18+ tools)
  - obia_tools.rs (30+ tools)
  - Specialized module files (4 tools: brdf_normalization, terrain_corrected_optical, orthorectification, georeference)
  - texture_glcm_tool.rs (1 tool)

- 📝 **Format:** Each extracted tool includes Tool ID, Display Name, metadata.summary, and manifest.summary

---

## Next Steps for Documentation Enhancement

1. **Extract remaining summaries** from non_filter_tools.rs using metadata() and manifest() implementations
2. **Cross-reference** parameter schemas in mod.rs with tool implementations
3. **Organize by category** (edge detection, morphological, statistical, spectral, classification, etc.)
4. **Verify consistency** between metadata.summary and manifest.summary for each tool
5. **Add examples and use cases** for better documentation clarity
6. **Document parameter descriptions** for each tool's required/optional inputs
