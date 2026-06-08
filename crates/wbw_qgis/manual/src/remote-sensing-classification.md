# Image Classification


---

## Evaluate Training Sites

**Function name:** `evaluate_training_sites`


### Description

 

This tool performs an evaluation of the reflectance properties of multi-spectral image dataset for a group of digitized class polygons. This is often viewed as the first step in a supervised classification procedure, such as those performed using the `min_dist_classification` or `parallelepiped_classification` tools. The analysis is based on a series of one or more input images (`inputs`) and an input polygon vector file (`polys`). The user must also specify the attribute name (`field`), within the attribute table, containing the class ID associated with each feature in input the polygon vector. A single class may be designated by multiple polygon features in the test site polygon vector. Note that the input polygon file is generally created by digitizing training areas of exemplar reflectance properties for each class type. The input polygon vector should be in the same coordinate system as the input multi-spectral images. The input images must represent a multi-spectral data set made up of individual bands. Do not input colour composite images. Lastly, the user must specify the name of the output HTML file. This file will contain a series of `box-and-whisker plots`, one for each band in the multi-spectral data set, that visualize the distribution of each class in the associated bands. This can be helpful in determining the overlap between spectral properties for the classes, which may be useful if further class or test site refinement is necessary. For a subsequent supervised classification to be successful, each class should not overlap significantly with the other classes in at least one of the input bands. If this is not the case, the user may need to refine the class system. 

 

### See Also

 

`min_dist_classification`, `parallelepiped_classification` 

### Python API

```python
def evaluate_training_sites(self, input_rasters: List[Raster], training_polygons: Vector, class_field_name: str, output_html_file: str) -> None:
```


---

## Fuzzy kNN Classification

**Function name:** `fuzzy_knn_classification`


Experimental

Fuzzy k-NN classification that yields soft class membership and optional probability surfaces.

remote_sensing classification knn fuzzy legacy-port

### Parameters

NameDescriptionRequiredDefault
`inputs`Array of single-band input rasters.Required`['band1.tif', 'band2.tif', 'band3.tif']`
`training_data`Point/polygon vector training data path.Required`training.shp`
`class_field`Class field in training_data attributes.Required`class`
`scaling`Feature scaling mode: none (default), normalize, standardize.Optional`none`
`k`Number of neighbors (default 5).Optional`5`
`m`Fuzzy exponent parameter (> 1; default 2.0).Optional`2.0`
`output`Optional output classified raster path.Optional—
`probability_output`Optional membership-probability raster path.Optional—

### Examples

*Run fuzzy kNN and output both class and confidence rasters.*
`wbe.fuzzy_knn_classification(class_field='class', inputs=['band1.tif', 'band2.tif', 'band3.tif'], k=7, m=2.0, output='fuzzy_knn_classified.tif', probability_output='fuzzy_knn_probability.tif', training_data='training.shp')`


---

## Generalize Classified Raster

**Function name:** `generalize_classified_raster`


### Description

 

This tool can be used to generalize a raster containing class or object features. Such rasters are usually derived from some classification procedure (e.g. image classification and landform classification), or as the output of a segmentation procedure (`image_segmentation`). Rasters that are created in this way often contain many very small features that make their interpretation, or vectorization, challenging. Therefore, it is common for practitioners to remove the smaller features. Many different approaches have been used for this task in the past. For example, it is common to remove small features using a filtering based approach (`majority_filter`). While this can be an effective strategy, it does have the disadvantage of modifying all of the boundaries in the class raster, including those that define larger features. In many applications, this can be a serious issue of concern. 

The `generalize_classified_raster` tool offers an alternative method for simplifying class rasters. The process begins by identifying each contiguous group of cells in the input (i.e. a clumping operation) and then defines the subset of features that are smaller than the user-specified minimum feature size (`min_size`), in grid cells. This set of small features is then dealt with using one of three methods (`method`). In the first method (*longest*), a small feature may be reassigned the class value of the neighbouring feature with the longest shared border. The sum of the neighbouring feature size and the small feature size must be larger than the specified size threshold, and the tool will iterate through this process of reassigning feature values to neighbouring values until each small feature has been resolved. 

The second method, *largest*, operates in much the same way as the first, except that objects are reassigned the value of the largest neighbour. Again, this process of reassigning small feature values iterates until every small feature has been reassigned to a large neighbouring feature. 

The third and last method (*nearest*) takes a different approach to resolving the reassignment of small features. Using the *nearest* generalization approach, each grid cell contained within a small feature is reassigned the value of the nearest large neighbouring feature. When there are two or more neighbouring features that are equally distanced to a small feature cell, the cell will be reassigned to the largest neighbour. Perhaps the most significant disadvantage of this approach is that it creates a new artificial boundary in the output image that is not contained within the input class raster. That is, with the previous two methods, boundaries associated with smaller features in the input images are 'erased' in the output map, but every boundary in the output raster exactly matches boundaries within the input raster (i.e. the output boundaries are a subset of the input feature boundaries). However, with the *nearest* method, artificial boundaries, determined by the divide between nearest neighbours, are introduced to the output raster and these new feature boundaries do not have any basis in the original classification/segmentation process. Thus caution should be exercised when using this approach, especially when larger minimum size thresholds are used. The *longest* method is the recommended approach to class feature generalization. 

 

 

For a video tutorial on how to use the `generalize_classified_raster` tool, see `this YouTube video`. 

### See Also

 

`generalize_with_similarity`, `majority_filter`, `image_segmentation` 

### Python API

```python
def generalize_classified_raster(self, raster: Raster, area_threshold: int = 5, method: str = "longest") -> Raster:
```


---

## Generalize With Similarity

**Function name:** `generalize_with_similarity`


### Description

 

This tool can be used to generalize a raster containing class features (`input`) by reassigning the identifier values of small features (`min_size`) to those of neighbouring features. Therefore, this tool performs a very similar operation to the `generalize_classified_raster` tool. However, while the `generalize_classified_raster` tool re-labels small features based on the geometric properties of neighbouring features (e.g. neighbour with the longest shared border, largest neighbour, or nearest neighbour), the `generalize_with_similarity` tool reassigns feature labels based on similarity with neighbouring features. Similarity is determined using a series of input similarity criteria rasters (`similarity`), which may be factors used in the creation of the input class raster. For example, the similarlity rasters may be bands of multi-spectral imagery, if the input raster is a classified land-cover map, or DEM-derived land surface parameters, if the input raster is a landform class map. 

The tool works by identifying each contiguous group of pixels (features) in the input class raster (`input`), i.e. a clumping operation. The mean value is then calculated for each feature and each similarity input, which defines a multi-dimensional 'similarity centre point' associated with each feature. It should be noted that the similarity raster data are standardized prior to calculating these centre point values. Lastly, the tool then reassigns the input label values of all features smaller than the user-specified minimum feature size (`min_size`) to that of the neighbouring feature with the shortest distance between similarity centre points. 

For small features that are entirely enclosed by a single larger feature, this process will result in the same generalization solution presented by any of the geometric-based methods of the `generalize_classified_raster` tool. However, for small features that have more than one neighbour, this tool may provide a superior generalization solution than those based solely on geometric information. 

 

For a video tutorial on how to use the `generalize_with_similarity` tool, see `this YouTube video`. 

### See Also

 

`generalize_classified_raster`, `majority_filter`, `image_segmentation` 

### Python API

```python
def generalize_with_similarity(self, raster: Raster, similarity_rasters: List[Raster], area_threshold: int = 5) -> Raster:
```


---

## K Means Clustering

**Function name:** `k_means_clustering`


This tool can be used to perform a k-means clustering operation on two or more input images (`inputs`), typically several bands of multi-spectral satellite imagery. The tool creates two outputs, including the classified image (`output` and a classification HTML report (`out_html`). The user must specify the number of class (`classes`), which should be known *a priori*, and the strategy for initializing class clusters (`initialize`). The initialization strategies include "diagonal" (clusters are initially located randomly along the multi-dimensional diagonal of spectral space) and "random" (clusters are initially located randomly throughout spectral space). The algorithm will continue updating cluster center locations with each iteration of the process until either the user-specified maximum number of iterations (`max_iterations`) is reached, or until a stability criteria (`class_change`) is achieved. The stability criteria is the percent of the total number of pixels in the image that are changed among the class values between consecutive iterations. Lastly, the user must specify the minimum allowable number of pixels in a cluster (`min_class_size`). 

Note, each of the input images must have the same number of rows and columns and the same spatial extent because the analysis is performed on a pixel-by-pixel basis. **NoData** values in any of the input images will result in the removal of the corresponding pixel from the analysis. 

### See Also

 

`modified_k_means_clustering` 

### Python API

```python
def k_means_clustering(self, input_rasters: List[Raster], output_html_file: str = "", num_clusters: int = 5, max_iterations: int = 10, percent_changed_threshold: float = 2.0, initialization_mode: str = "dia", min_class_size: int = 10) -> Raster:
```


---

## kNN Classification

**Function name:** `knn_classification`


### Description

 

This tool performs a supervised `*k*-nearest neighbour (*k*-NN) classification` using multiple predictor rasters (`inputs`), or features, and training data (`training`). It can be used to model the spatial distribution of class data, such as land-cover type, soil class, or vegetation type. The training data take the form of an input vector Shapefile containing a set of points or polygons, for which the known class information is contained within a field (`field`) of the attribute table. Each grid cell defines a stack of feature values (one value for each input raster), which serves as a point within the multi-dimensional feature space. The algorithm works by identifying a user-defined number (*k*, `-k`) of feature-space neighbours from the training set for each grid cell. The class that is then assigned to the grid cell in the output raster (`output`) is then determined as the most common class among the set of neighbours. Note that the `knn_regression` tool can be used to apply the *k*-NN method to the modelling of continuous data. 

The user has the option to clip the training set data (`clip`). When this option is selected, each training pixel for which the estimated class value, based on the *k*-NN procedure, is not equal to the known class value, is removed from the training set before proceeding with labelling all grid cells. This has the effect of removing outlier points within the training set and often improves the overall classification accuracy. 

The tool splits the training data into two sets, one for training the classifier and one for testing the classification. These test data are used to calculate the overall accuracy and Cohen's kappa index of agreement, as well as to estimate the variable importance. The `test_proportion` parameter is used to set the proportion of the input training data used in model testing. For example, if `test_proportion = 0.2`, 20% of the training data will be set aside for testing, and this subset will be selected randomly. As a result of this random selection of test data, the tool behaves stochastically, and will result in a different model each time it is run. 

Note that the output image parameter (`output`) is optional. When unspecified, the tool will simply report the model accuracy statistics and variable importance, allowing the user to experiment with different parameter settings and input predictor raster combinations to optimize the model before applying it to classify the whole image data set. 

Like all supervised classification methods, this technique relies heavily on proper selection of training data. Training sites are exemplar areas/points of known and representative class value (e.g. land cover type). The algorithm determines the feature signatures of the pixels within each training area. In selecting training sites, care should be taken to ensure that they cover the full range of variability within each class. Otherwise the classification accuracy will be impacted. If possible, multiple training sites should be selected for each class. It is also advisable to avoid areas near the edges of class objects (e.g. land-cover patches), where mixed pixels may impact the purity of training site values. 

After selecting training sites, the feature value distributions of each class type can be assessed using the `evaluate_training_sites` tool. In particular, the distribution of class values should ideally be non-overlapping in at least one feature dimension. 

The *k*-NN algorithm is based on the calculation of distances in multi-dimensional space. Feature scaling is essential to the application of *k*-NN modelling, especially when the ranges of the features are different, for example, if they are measured in different units. Without scaling, features with larger ranges will have greater influence in computing the distances between points. The tool offers three options for feature-scaling (`scaling`), including 'None', 'Normalize', and 'Standardize'. Normalization simply rescales each of the features onto a 0-1 range. This is a good option for most applications, but it is highly sensitive to outliers because it is determined by the range of the minimum and maximum values. Standardization rescales predictors using their means and standard deviations, transforming the data into z-scores. This is a better option than normalization when you know that the data contain outlier values; however, it does does assume that the feature data are somewhat normally distributed, or are at least symmetrical in distribution. 

Because the *k*-NN algorithm calculates distances in feature-space, like many other related algorithms, it suffers from the `curse of dimensionality`. Distances become less meaningful in high-dimensional space because the vastness of these spaces means that distances between points are less significant (more similar). As such, if the predictor list includes insignificant or highly correlated variables, it is advisable to exclude these features during the model-building phase, or to use a dimension reduction technique such as `principal_component_analysis` to transform the features into a smaller set of uncorrelated predictors. 

For a video tutorial on how to use the `knn_classification` tool, see `this YouTube video`. 

### Memory Usage

 

The peak memory usage of this tool is approximately 8 bytes per grid cell &times; # predictors. 

### See Also

 

`knn_regression`, `random_forest_classification`, `svm_classification`, `parallelepiped_classification`, `evaluate_training_sites` 

### Python API

```python
def knn_classification(self, input_rasters: List[Raster], training_data: Vector, class_field_name: str, scaling_method: str = "none", k: int = 5, test_proportion: float = 0.2, use_clipping: bool = False, create_output: bool = False) -> Optional[Raster]:
```


---

## kNN Regression

**Function name:** `knn_regression`


### Description

 

This tool performs a supervised `*k*-nearest neighbour (*k*-NN) regression analysis` using multiple predictor rasters (`inputs`), or features, and training data (`training`). It can be used to model the spatial distribution of continuous data, such as soil properties (e.g. percent sand/silt/clay). The training data take the form of an input vector Shapefile containing a set of points, for which the known outcome information is contained within a field (`field`) of the attribute table. Each grid cell defines a stack of feature values (one value for each input raster), which serves as a point within the multi-dimensional feature space. The algorithm works by identifying a user-defined number (*k*, `-k`) of feature-space neighbours from the training set for each grid cell. The value that is then assigned to the grid cell in the output raster (`output`) is then determined as the mean of the outcome variable among the set of neighbours. The user may optionally choose to weight neighbour outcome values in the averaging calculation, with weights determined by the inverse distance function (`weight`). Note that the `knn_classification` tool can be used to apply the *k*-NN method to the modelling of categorical data. 

The tool splits the training data into two sets, one for training the model and one for testing the prediction. These test data are used to calculate the regression accuracy statistics, as well as to estimate the variable importance. The `test_proportion` parameter is used to set the proportion of the input training data used in model testing. For example, if `test_proportion = 0.2`, 20% of the training data will be set aside for testing, and this subset will be selected randomly. As a result of this random selection of test data, the tool behaves stochastically, and will result in a different model each time it is run. 

Note that the output image parameter (`output`) is optional. When unspecified, the tool will simply report the model accuracy statistics and variable importance, allowing the user to experiment with different parameter settings and input predictor raster combinations to optimize the model before applying it to model the outcome variable across the whole region defined by image data set. 

The *k*-NN algorithm is based on the calculation of distances in multi-dimensional space. Feature scaling is essential to the application of *k*-NN modelling, especially when the ranges of the features are different, for example, if they are measured in different units. Without scaling, features with larger ranges will have greater influence in computing the distances between points. The tool offers three options for feature-scaling (`scaling`), including 'None', 'Normalize', and 'Standardize'. Normalization simply rescales each of the features onto a 0-1 range. This is a good option for most applications, but it is highly sensitive to outliers because it is determined by the range of the minimum and maximum values. Standardization rescales predictors using their means and standard deviations, transforming the data into z-scores. This is a better option than normalization when you know that the data contain outlier values; however, it does does assume that the feature data are somewhat normally distributed, or are at least symmetrical in distribution. 

Because the *k*-NN algorithm calculates distances in feature-space, like many other related algorithms, it suffers from the `curse of dimensionality`. Distances become less meaningful in high-dimensional space because the vastness of these spaces means that distances between points are less significant (more similar). As such, if the predictor list includes insignificant or highly correlated variables, it is advisable to exclude these features during the model-building phase, or to use a dimension reduction technique such as `principal_component_analysis` to transform the features into a smaller set of uncorrelated predictors. 

### Memory Usage

 

The peak memory usage of this tool is approximately 8 bytes per grid cell &times; # predictors. 

### See Also

 

`knn_classification`, `random_forest_regression`, `svm_regression`, `principal_component_analysis` 

### Python API

```python
def knn_regression(self, input_rasters: List[Raster], training_data: Vector, field_name: str, scaling_method: str = "none", k: int = 5, distance_weighting: bool = False, test_proportion: float = 0.2, create_output: bool = False) -> Optional[Raster]:
```


---

## Logistic Regression

**Function name:** `logistic_regression`


### Description

 

This tool performs a `logistic regression analysis` using multiple predictor rasters (`inputs`), or features, and training data (`training`). Logistic regression is a type of linear statistical classifier that in its basic form uses a logistic function to model a binary outcome variable, although the implementation used by this tool can handle multi-class dependent variables. This tool can be used to model the spatial distribution of class data, such as land-cover type, soil class, or vegetation type. 

The training data take the form of an input vector Shapefile containing a set of points or polygons, for which the known class information is contained within a field (`field`) of the attribute table. Each grid cell defines a stack of feature values (one value for each input raster), which serves as a point within the multi-dimensional feature space. 

The tool splits the training data into two sets, one for training the model and one for testing the prediction. These test data are used to calculate the classification accuracy stats, as well as to estimate the variable importance. The `test_proportion` parameter is used to set the proportion of the input training data used in model testing. For example, if `test_proportion = 0.2`, 20% of the training data will be set aside for testing, and this subset will be selected randomly. As a result of this random selection of test data, the tool behaves stochastically, and will result in a different model each time it is run. 

Note that the output image parameter (`output`) is optional. When unspecified, the tool will simply report the model accuracy statistics and variable importance, allowing the user to experiment with different parameter settings and input predictor raster combinations to optimize the model before applying it to model the outcome variable across the whole region defined by image data set. 

The user may opt for feature scaling, which can be important when the ranges of the features are different, for example, if they are measured in different units. Without scaling, features with larger ranges will have greater influence in computing the distances between points. The tool offers three options for feature-scaling (`scaling`), including 'None', 'Normalize', and 'Standardize'. Normalization simply rescales each of the features onto a 0-1 range. This is a good option for most applications, but it is highly sensitive to outliers because it is determined by the range of the minimum and maximum values. Standardization rescales predictors using their means and standard deviations, transforming the data into z-scores. This is a better option than normalization when you know that the data contain outlier values; however, it does does assume that the feature data are somewhat normally distributed, or are at least symmetrical in distribution. 

Because the logistic regression calculates distances in feature-space, like many other related algorithms, it suffers from the `curse of dimensionality`. Distances become less meaningful in high-dimensional space because the vastness of these spaces means that distances between points are less significant (more similar). As such, if the predictor list includes insignificant or highly correlated variables, it is advisable to exclude these features during the model-building phase, or to use a dimension reduction technique such as `principal_component_analysis` to transform the features into a smaller set of uncorrelated predictors. 

### See Also

 

`svm_classification`, `random_forest_classification`, `knn_classification`, `principal_component_analysis` 

### Python API

```python
def logistic_regression(self, input_rasters: List[Raster], training_data: Vector, class_field_name: str, scaling_method: str = "none", test_proportion: float = 0.2, create_output: bool = False) -> Optional[Raster]:
```


---

## Min Dist Classification

**Function name:** `min_dist_classification`


### Description

 

This tool performs a supervised `minimum-distance classification` using training site polygons (`polys`) and multi-spectral images (`inputs`). This classification method uses the mean vectors for each class and calculates the Euclidean distance from each unknown pixel to the class mean vector. Unclassed pixels are then assigned to the nearest class mean. A threshold distance (`threshold`), expressed in number of `z-scores`, may optionally be used and pixels whose multi-spectral distance is greater than this threshold will not be assigned a class in the output image (`output`). When a threshold distance is unspecified, all pixels will be assigned to a class. 

Like all supervised classification methods, this technique relies heavily on proper selection of training data. Training sites are exemplar areas of known and representative land cover type. The algorithm determines the spectral signature of the pixels within each training area, and uses this information to define the mean vector of each class. It is preferable that training sites are based on either field-collected data or fine-resolution reference imagery. In selecting training sites, care should be taken to ensure that they cover the full range of variability within each class. Otherwise the classification accuracy will be impacted. If possible, multiple training sites should be selected for each class. It is also advisable to avoid areas near the edges of land-cover patches, where mixed pixels may impact the purity of training site reflectance values. 

After selecting training sites, the reflectance values of each land-cover type can be assessed using the `evaluate_training_sites` tool. In particular, the distribution of reflectance values should ideally be non-overlapping in at least one band of the multi-spectral data set. 

### See Also

 

`evaluate_training_sites`, `parallelepiped_classification` 

### Python API

```python
def min_dist_classification(self, input_rasters: List[Raster], training_data: Vector, class_field_name: str, dist_threshold: float = float('inf')) -> Raster:
```
 

### Python API

```python
def min_dist_classification(self, input_rasters: List[Raster], training_data: Vector, class_field_name: str, dist_threshold: float = float('inf')) -> Raster:
```


---

## Modified K Means Clustering

**Function name:** `modified_k_means_clustering`


This modified k-means algorithm is similar to that described by Mather and Koch (2011). The main difference between the traditional k-means and this technique is that the user does not need to specify the desired number of classes/clusters prior to running the tool. Instead, the algorithm initializes with a very liberal overestimate of the number of classes and then merges classes that have cluster centres that are separated by less than a user-defined threshold. The main difference between this algorithm and the ISODATA technique is that clusters can not be broken apart into two smaller clusters. 

### Reference

 

Mather, P. M., & Koch, M. (2011). Computer processing of remotely-sensed images: an introduction. John Wiley & Sons. 

### See Also

 

`k_means_clustering` 

### Python API

```python
def modified_k_means_clustering(self, input_rasters: List[Raster], output_html_file: str = "", num_start_clusters: int = 1000, merge_distance: float = 1.0, max_iterations: int = 10, percent_changed_threshold: float = 2.0) -> Raster:
```


---

## Nnd Classification

**Function name:** `nnd_classification`


### Description

 

This tool performs a supervised `*k*-nearest neighbour (*k*-NN) classification` using multiple predictor rasters (`inputs`), or features, and training data (`training`). It can be used to model the spatial distribution of class data, such as land-cover type, soil class, or vegetation type. The training data take the form of an input vector Shapefile containing a set of points or polygons, for which the known class information is contained within a field (`field`) of the attribute table. Each grid cell defines a stack of feature values (one value for each input raster), which serves as a point within the multi-dimensional feature space. The algorithm works by identifying a user-defined number (*k*, `-k`) of feature-space neighbours from the training set for each grid cell. The class that is then assigned to the grid cell in the output raster (`output`) is then determined as the most common class among the set of neighbours. Note that the `knn_regression` tool can be used to apply the *k*-NN method to the modelling of continuous data. 

The user has the option to clip the training set data (`clip`). When this option is selected, each training pixel for which the estimated class value, based on the *k*-NN procedure, is not equal to the known class value, is removed from the training set before proceeding with labelling all grid cells. This has the effect of removing outlier points within the training set and often improves the overall classification accuracy. 

The tool splits the training data into two sets, one for training the classifier and one for testing the classification. These test data are used to calculate the overall accuracy and Cohen's kappa index of agreement, as well as to estimate the variable importance. The `test_proportion` parameter is used to set the proportion of the input training data used in model testing. For example, if `test_proportion = 0.2`, 20% of the training data will be set aside for testing, and this subset will be selected randomly. As a result of this random selection of test data, the tool behaves stochastically, and will result in a different model each time it is run. 

Note that the output image parameter (`output`) is optional. When unspecified, the tool will simply report the model accuracy statistics and variable importance, allowing the user to experiment with different parameter settings and input predictor raster combinations to optimize the model before applying it to classify the whole image data set. 

Like all supervised classification methods, this technique relies heavily on proper selection of training data. Training sites are exemplar areas/points of known and representative class value (e.g. land cover type). The algorithm determines the feature signatures of the pixels within each training area. In selecting training sites, care should be taken to ensure that they cover the full range of variability within each class. Otherwise the classification accuracy will be impacted. If possible, multiple training sites should be selected for each class. It is also advisable to avoid areas near the edges of class objects (e.g. land-cover patches), where mixed pixels may impact the purity of training site values. 

After selecting training sites, the feature value distributions of each class type can be assessed using the `evaluate_training_sites` tool. In particular, the distribution of class values should ideally be non-overlapping in at least one feature dimension. 

The *k*-NN algorithm is based on the calculation of distances in multi-dimensional space. Feature scaling is essential to the application of *k*-NN modelling, especially when the ranges of the features are different, for example, if they are measured in different units. Without scaling, features with larger ranges will have greater influence in computing the distances between points. The tool offers three options for feature-scaling (`scaling`), including 'None', 'Normalize', and 'Standardize'. Normalization simply rescales each of the features onto a 0-1 range. This is a good option for most applications, but it is highly sensitive to outliers because it is determined by the range of the minimum and maximum values. Standardization rescales predictors using their means and standard deviations, transforming the data into z-scores. This is a better option than normalization when you know that the data contain outlier values; however, it does does assume that the feature data are somewhat normally distributed, or are at least symmetrical in distribution. 

Because the *k*-NN algorithm calculates distances in feature-space, like many other related algorithms, it suffers from the `curse of dimensionality`. Distances become less meaningful in high-dimensional space because the vastness of these spaces means that distances between points are less significant (more similar). As such, if the predictor list includes insignificant or highly correlated variables, it is advisable to exclude these features during the model-building phase, or to use a dimension reduction technique such as `principal_component_analysis` to transform the features into a smaller set of uncorrelated predictors. 

For a video tutorial on how to use the `knn_classification` tool, see `this YouTube video`. 

### Memory Usage

 

The peak memory usage of this tool is approximately 8 bytes per grid cell &times; # predictors. 

### See Also

 

`knn_regression`, `random_forest_classification`, `svm_classification`, `parallelepiped_classification`, `evaluate_training_sites` 

### Python API

```python
def knn_classification(self, input_rasters: List[Raster], training_data: Vector, class_field_name: str, scaling_method: str = "none", k: int = 5, test_proportion: float = 0.2, use_clipping: bool = False, create_output: bool = False) -> Optional[Raster]:
```


---

## Otsu Thresholding

**Function name:** `otsu_thresholding`


This tool uses `Ostu's method` for optimal automatic binary thresholding, transforming an input image (`--input`) into background and foreground pixels (`--output`). Otsu’s method uses the grayscale  image histogram to detect an optimal threshold value that separates two regions with maximum inter-class variance. The process begins by calculating the image histogram of the input. 

### References

 

Otsu, N., 1979. A threshold selection method from gray-level histograms. IEEE transactions on  systems, man, and cybernetics, 9(1), pp.62-66. 

### See Also

 

`image_segmentation`, `image_segmentation` 

### Python API

```python
def otsu_thresholding(self, raster: Raster) -> Raster:
```


---

## Parallelepiped Classification

**Function name:** `parallelepiped_classification`


### Description

 

This tool performs a supervised `parallelepiped classification` using training site polygons (`polys`) and multi-spectral images (`inputs`). This classification method uses the minimum and maximum reflectance values for each class within the training data to characterize a set of `parallelepipeds`, i.e. multi-dimensional geometric shapes. The algorithm then assigns each unknown pixel in the image data set to the first class for which the pixel's spectral vector is contained within the corresponding class parallelepiped. Pixels with spectral vectors that are not contained within any class parallelepiped will not be assigned a class in the output image. 

Like all supervised classification methods, this technique relies heavily on proper selection of training data. Training sites are exemplar areas of known and representative land cover type. The algorithm determines the spectral signature of the pixels within each training area, and uses this information to define the mean vector of each class. It is preferable that training sites are based on either field-collected data or fine-resolution reference imagery. In selecting training sites, care should be taken to ensure that they cover the full range of variability within each class. Otherwise the classification accuracy will be impacted. If possible, multiple training sites should be selected for each class. It is also advisable to avoid areas near the edges of land-cover patches, where mixed pixels may impact the purity of training site reflectance values. 

After selecting training sites, the reflectance values of each land-cover type can be assessed using the `evaluate_training_sites` tool. In particular, the distribution of reflectance values should ideally be non-overlapping in at least one band of the multi-spectral data set. 

### See Also

 

`evaluate_training_sites`, `min_dist_classification` 

### Python API

```python
def parallelepiped_classification(self, input_rasters: List[Raster], training_data: Vector, class_field_name: str) -> Raster:
```


---

## Random Forest Classification

**Function name:** `random_forest_classification`


Experimental

Supervised Random Forest classification for multisource raster features using point/polygon training data.

remote_sensing classification random_forest legacy-port

### Parameters

NameDescriptionRequiredDefault
`inputs`Array of single-band input rasters.Required`['band1.tif', 'band2.tif', 'band3.tif']`
`training_data`Point/polygon vector training data path.Required`training.shp`
`class_field`Class field in training_data attributes.Required`class`
`scaling`Feature scaling mode: none (default), normalize, standardize.Optional`none`
`n_trees`Number of trees in the forest (default 200).Optional`200`
`min_samples_leaf`Minimum number of samples required at a leaf node (default 1).Optional`1`
`min_samples_split`Minimum number of samples required to split an internal node (default 2).Optional`2`
`output`Optional output raster path.Optional—

### Examples

*Run random forest classification on multiband predictors.*
`wbe.random_forest_classification(class_field='class', inputs=['band1.tif', 'band2.tif', 'band3.tif'], n_trees=300, output='rf_classification.tif', scaling='standardize', training_data='training.shp')`


---

## Random Forest Regression

**Function name:** `random_forest_regression`


Experimental

Random Forest regression for continuous targets (e.g., biomass, moisture, temperature) from raster predictors.

remote_sensing regression random_forest legacy-port

### Parameters

NameDescriptionRequiredDefault
`inputs`Array of single-band input rasters.Required`['band1.tif', 'band2.tif', 'band3.tif']`
`training_data`Point vector training data path.Required`training_points.shp`
`field`Numeric target field in training_data attributes.Required`value`
`scaling`Feature scaling mode: none (default), normalize, standardize.Optional`none`
`n_trees`Number of trees in the forest (default 200).Optional`200`
`min_samples_leaf`Minimum number of samples required at a leaf node (default 1).Optional`1`
`min_samples_split`Minimum number of samples required to split an internal node (default 2).Optional`2`
`output`Optional output raster path.Optional—

### Examples

*Run random forest regression on multiband predictors.*
`wbe.random_forest_regression(field='target', inputs=['band1.tif', 'band2.tif', 'band3.tif'], n_trees=300, output='rf_regression.tif', scaling='standardize', training_data='training_points.shp')`


---

## SVM Classification

**Function name:** `svm_classification`


### Description

 

This tool performs a `support vector machine (SVM) binary classification` using multiple predictor rasters (`inputs`), or features, and training data (`training`). SVMs are a common class of supervised learning algorithms widely applied in many problem domains. This tool can be used to model the spatial distribution of class data, such as land-cover type, soil class, or vegetation type. The training data take the form of an input vector Shapefile containing a set of points or polygons, for which the known class information is contained within a field (`field`) of the attribute table. Each grid cell defines a stack of feature values (one value for each input raster), which serves as a point within the multi-dimensional feature space. Note that the `svm_regression` tool can be used to apply the SVM method to the modelling of continuous data. 

The user must specify the values of three parameters used in the development of the model, the *c* parameters (`-c`), gamma (`gamma`), and the tolerance (`tolerance`). The *c*-value is the regularization parameter used in model optimization. The gamma parameter defines the radial basis function (Gaussian) kernel parameter. The tolerance parameter controls the stopping condition used during model optimization. 

The tool splits the training data into two sets, one for training the classifier and one for testing the classification. These test data are used to calculate the overall accuracy and Matthew correlation coefficient (MCC). The `test_proportion` parameter is used to set the proportion of the input training data used in model testing. For example, if `test_proportion = 0.2`, 20% of the training data will be set aside for testing, and this subset will be selected randomly. As a result of this random selection of test data, the tool behaves stochastically, and will result in a different model each time it is run. 

Note that the output image parameter (`output`) is optional. When unspecified, the tool will simply report the model accuracy statistics, allowing the user to experiment with different parameter settings and input predictor raster combinations to optimize the model before applying it to classify the whole image data set. 

Like all supervised classification methods, this technique relies heavily on proper selection of training data. Training sites are exemplar areas/points of known and representative class value (e.g. land cover type). The algorithm determines the feature signatures of the pixels within each training area. In selecting training sites, care should be taken to ensure that they cover the full range of variability within each class. Otherwise the classification accuracy will be impacted. If possible, multiple training sites should be selected for each class. It is also advisable to avoid areas near the edges of class objects (e.g. land-cover patches), where mixed pixels may impact the purity of training site values. 

After selecting training sites, the feature value distributions of each class type can be assessed using the `evaluate_training_sites` tool. In particular, the distribution of class values should ideally be non-overlapping in at least one feature dimension. 

The SVM algorithm is based on the calculation of distances in multi-dimensional space. Feature scaling is essential to the application of SVM-based modelling, especially when the ranges of the features are different, for example, if they are measured in different units. Without scaling, features with larger ranges will have greater influence in computing the distances between points. The tool offers three options for feature-scaling (`scaling`), including 'None', 'Normalize', and 'Standardize'. Normalization simply rescales each of the features onto a 0-1 range. This is a good option for most applications, but it is highly sensitive to outliers because it is determined by the range of the minimum and maximum values. Standardization rescales predictors using their means and standard deviations, transforming the data into z-scores. This is a better option than normalization when you know that the data contain outlier values; however, it does does assume that the feature data are somewhat normally distributed, or are at least symmetrical in distribution. 

Because the SVM algorithm calculates distances in feature-space, like many other related algorithms, it suffers from the `curse of dimensionality`. Distances become less meaningful in high-dimensional space because the vastness of these spaces means that distances between points are less significant (more similar). As such, if the predictor list includes insignificant or highly correlated variables, it is advisable to exclude these features during the model-building phase, or to use a dimension reduction technique such as `principal_component_analysis` to transform the features into a smaller set of uncorrelated predictors. 

### See Also

 

`random_forest_classification`, `knn_classification`, `parallelepiped_classification`, `evaluate_training_sites`, `principal_component_analysis` 

### Python API

```python
def svm_classification(self, input_rasters: List[Raster], training_data: Vector, class_field_name: str, scaling_method: str = "none", c_value: float = 50.0, kernel_gamma: float = 0.5, tolerance: float = 0.1, test_proportion: float = 0.2, create_output: bool = False) -> Optional[Raster]:
```


---

## SVM Regression

**Function name:** `svm_regression`


### Description

 

This tool performs a supervised `support vector machine (SVM) regression analysis` using multiple predictor rasters (`inputs`), or features, and training data (`training`). SVMs are a common class of supervised learning algorithms widely applied in many problem domains. This tool can be used to model the spatial distribution of continuous data, such as soil properties (e.g. percent sand/silt/clay). The training data take the form of an input vector Shapefile containing a set of points for which the known outcome data is contained within a field (`field`) of the attribute table. Each grid cell defines a stack of feature values (one value for each input raster), which serves as a point within the multi-dimensional feature space. Note that the `svm_classification` tool can be used to apply the SVM method to the modelling of categorical data. 

The user must specify the *c*-value (`-c`), the regularization parameter used in model optimization, the epsilon-value (`eps`), used in the development of the epsilon-SVM regression model, and the gamma-value (`gamma`), which is used in defining the radial basis function (Gaussian) kernel parameter. 

The tool splits the training data into two sets, one for training the model and one for testing the prediction. These test data are used to calculate the regression accuracy statistics, as well as to estimate the variable importance. The `test_proportion` parameter is used to set the proportion of the input training data used in model testing. For example, if `test_proportion = 0.2`, 20% of the training data will be set aside for testing, and this subset will be selected randomly. As a result of this random selection of test data, the tool behaves stochastically, and will result in a different model each time it is run. 

Note that the output image parameter (`output`) is optional. When unspecified, the tool will simply report the model accuracy statistics and variable importance, allowing the user to experiment with different parameter settings and input predictor raster combinations to optimize the model before applying it to model the outcome variable across the whole region defined by image data set. 

The SVM algorithm is based on the calculation of distances in multi-dimensional space. Feature scaling is essential to the application of SVM modelling, especially when the ranges of the features are different, for example, if they are measured in different units. Without scaling, features with larger ranges will have greater influence in computing the distances between points. The tool offers three options for feature-scaling (`scaling`), including 'None', 'Normalize', and 'Standardize'. Normalization simply rescales each of the features onto a 0-1 range. This is a good option for most applications, but it is highly sensitive to outliers because it is determined by the range of the minimum and maximum values. Standardization rescales predictors using their means and standard deviations, transforming the data into z-scores. This is a better option than normalization when you know that the data contain outlier values; however, it does does assume that the feature data are somewhat normally distributed, or are at least symmetrical in distribution. 

Because the SVM algorithm calculates distances in feature-space, like many other related algorithms, it suffers from the `curse of dimensionality`. Distances become less meaningful in high-dimensional space because the vastness of these spaces means that distances between points are less significant (more similar). As such, if the predictor list includes insignificant or highly correlated variables, it is advisable to exclude these features during the model-building phase, or to use a dimension reduction technique such as `principal_component_analysis` to transform the features into a smaller set of uncorrelated predictors. 

### See Also

 

`svm_classification`, `random_forest_regression`, `knn_regression`, `principal_component_analysis` 

### Python API

```python
def svm_regression(self, input_rasters: List[Raster], training_data: Vector, class_field_name: str, scaling_method: str = "none", c_value: float = 50.0, epsilon_value: float = 10.0, kernel_gamma: float = 0.5, test_proportion: float = 0.2, create_output: bool = False) -> Optional[Raster]:
```
