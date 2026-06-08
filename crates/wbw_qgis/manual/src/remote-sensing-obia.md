# Object-Based Image Analysis (OBIA)


---

## Build Object Hierarchy Multiscale

**Function name:** `build_object_hierarchy_multiscale`


*No help documentation available for this tool.*


---

## Classify Objects Ensemble Pro

**Function name:** `classify_objects_ensemble_pro`


*No help documentation available for this tool.*


---

## Classify Objects Random Forest

**Function name:** `classify_objects_random_forest`


*No help documentation available for this tool.*


---

## Classify Objects Rules Basic

**Function name:** `classify_objects_rules_basic`


*No help documentation available for this tool.*


---

## Classify Objects Rules Hierarchical

**Function name:** `classify_objects_rules_hierarchical`


*No help documentation available for this tool.*


---

## Classify Objects SVM

**Function name:** `classify_objects_svm`


*No help documentation available for this tool.*


---

## Evaluate Object Classification Accuracy

**Function name:** `evaluate_object_classification_accuracy`


*No help documentation available for this tool.*


---

## Evaluate Segmentation Quality Pro

**Function name:** `evaluate_segmentation_quality_pro`


*No help documentation available for this tool.*


---

## Image Segmentation

**Function name:** `image_segmentation`


### Description

 

This tool is used to segment a mult-spectral image data set, or multi-dimensional data stack. The algorithm is based on region-growing operations. Each of the input images are transformed into `standard scores` prior to analysis. The total multi-dimensional distance between each pixel and its eight neighbours is measured, which then serves as a priority value for selecting potential seed pixels for the region-growing operations, with pixels exhibited the least difference with their neighbours more likely to serve as seeds. The region-growing operations initiate at seed pixels and grows outwards, connecting neighbouring pixels that have a multi-dimensional distance from the seed cell that is less than a threshold value. Thus, the region-growing operations attempt to identify contiguous, relatively homogeneous objects. The algorithm stratifies potential seed pixels into bands, based on their total difference with their eight neighbours. The user may control the size and number of these bands using the `threshold` and `steps` parameters respectively. Increasing the magnitude of the threshold parameter will result in fewer mapped objects and vice versa. All pixels that are not assigned to an object after the seeding-based region-growing operations are then clumped simply based on contiguity. 

It is commonly the case that there will be a large number of very small-sized objects identified using this approach. The user may optionally specify that objects that are less than a minimum area (expressed in pixels) be eliminated from the final output raster. The `min_area` parameter must be an integer between 1 and 8. In cleaning small objects from the output, the pixels belonging to these smaller features are assigned to the most homogeneous neighbouring object. 

The input rasters (`inputs`) may be bands of satellite imagery, or any other attribute, such as measures of texture, elevation, or other topographic derivatives, such as slope. If satellite imagery is used as inputs, it can be beneficial to pre-process the data with an edge-preserving low-pass filter, such as the `bilateral_filter` and `edge_preserving_mean_filter` tools. 

### See Also

 

`bilateral_filter`, `edge_preserving_mean_filter` 

### Python API

```python
def image_segmentation(self, input_rasters: List[Raster], dist_threshold: float = 0.5, num_steps: int = 10, area_threshold: int = 4) -> Raster:
```


---

## Image Slider

**Function name:** `image_slider`


### Description

 

This tool creates an interactive image slider from two input images (`input1` and `input2`). An image slider is an interactive visualization of two overlapping images, in which the user moves the position of a slider bar to hide or reveal one of the overlapping images. The output (`output`) is an HTML file. Each of the two input images may be rendered in one of several available palettes. If the input image is a colour composite image, no palette is required. Labels may also be optionally associated with each of the images, displayed in the upper left and right corners. The user must also specify the image height (`height`) in the output file. Note that the output is simply HTML, CSS, and javascript code, which can be readily embedded in other documents. 

The following is an example of what the output of this tool looks like. **Click the image for an interactive example.** 

 

### Python API

```python
def image_slider(self, left_raster: Raster, right_raster: Raster, output_html_file: str, left_palette: WbPalette = WbPalette.Grey, left_reverse_palette: bool = False, left_label: str = "",  right_palette: WbPalette = WbPalette.Grey, right_reverse_palette: bool = False, right_label: str = "", image_height: int = 600) -> None:
```


---

## Image Stack Profile

**Function name:** `image_stack_profile`


This tool can be used to plot an image stack profile (i.e. a signature) for a set of points (`points`) and a multispectral image stack (`inputs`). The tool outputs an interactive SVG line graph embedded in an HTML document. If the input points vector contains multiple points, each input point will be associated with a single line in the output plot. The order of vertices in each signature line is determined by the order of images specified in the `inputs` parameter. At least two input images are required to run this operation. Note that this tool does not require multispectral images as inputs; other types of data may also be used as the image stack. Also note that the input images should be single-band, continuous greytone rasters. RGB colour images are not good candidates for this tool. 

If you require the raster values to be saved in the vector points file's attribute table, or if you need the raster values to be output as text, you may use the `extract_raster_values_at_points` tool instead. 

### See Also

 

`extract_raster_values_at_points` 

### Python API

```python
def image_stack_profile(self, images: List[Raster], points: Vector, output_html_file: str) -> None:
```


---

## OBIA Audit Report Pro

**Function name:** `obia_audit_report_pro`


*No help documentation available for this tool.*


---

## OBIA Batch Orchestrator Pro

**Function name:** `obia_batch_orchestrator_pro`


*No help documentation available for this tool.*


---

## OBIA Pipeline Basic

**Function name:** `obia_pipeline_basic`


*No help documentation available for this tool.*


---

## Object Class Probability Maps

**Function name:** `object_class_probability_maps`


*No help documentation available for this tool.*


---

## Object Features Context Neighbors

**Function name:** `object_features_context_neighbors`


*No help documentation available for this tool.*


---

## Object Features Shape Basic

**Function name:** `object_features_shape_basic`


*No help documentation available for this tool.*


---

## Object Features Spectral Basic

**Function name:** `object_features_spectral_basic`


*No help documentation available for this tool.*


---

## Object Features Texture GLCM Basic

**Function name:** `object_features_texture_glcm_basic`


*No help documentation available for this tool.*


---

## Object Features Topology Relations

**Function name:** `object_features_topology_relations`


*No help documentation available for this tool.*


---

## Object Uncertainty Diagnostics Pro

**Function name:** `object_uncertainty_diagnostics_pro`


*No help documentation available for this tool.*


---

## Objects Boundary Refinement Pro

**Function name:** `objects_boundary_refinement_pro`


*No help documentation available for this tool.*


---

## Objects Enforce Min Mapping Unit

**Function name:** `objects_enforce_min_mapping_unit`


*No help documentation available for this tool.*


---

## Polygons To Segments

**Function name:** `polygons_to_segments`


*No help documentation available for this tool.*


---

## Propagate Labels Across Hierarchy

**Function name:** `propagate_labels_across_hierarchy`


*No help documentation available for this tool.*


---

## Segment Graph Felzenszwalb

**Function name:** `segment_graph_felzenszwalb`


*No help documentation available for this tool.*


---

## Segment Multiresolution Hierarchical

**Function name:** `segment_multiresolution_hierarchical`


*No help documentation available for this tool.*


---

## Segment Scale Parameter Optimizer

**Function name:** `segment_scale_parameter_optimizer`


*No help documentation available for this tool.*


---

## Segment SLIC Superpixels

**Function name:** `segment_slic_superpixels`


*No help documentation available for this tool.*


---

## Segment Watershed Markers

**Function name:** `segment_watershed_markers`


*No help documentation available for this tool.*


---

## Segments Merge Small Regions

**Function name:** `segments_merge_small_regions`


*No help documentation available for this tool.*


---

## Segments Split Low Cohesion

**Function name:** `segments_split_low_cohesion`


*No help documentation available for this tool.*


---

## Segments To Polygons

**Function name:** `segments_to_polygons`


*No help documentation available for this tool.*
