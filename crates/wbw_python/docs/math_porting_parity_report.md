# Math Tool Porting Parity Report

Date: 2026-03-24

## Scope Decision
- All legacy math tools are in scope, including stats/ML entries.
- The previous 48-tool denominator is retired.
- Exception: `hdbscan_clustering` is intentionally skipped because it was a failed legacy experiment.

## Current Status
- Legacy math tools: 99
- Intentional legacy exceptions: 1 (`hdbscan_clustering`)
- Ported/registered in new backend: **98** ✅ **COMPLETE**
- Remaining to port (excluding exceptions): **0** ✅

## Most Recent Batch — Statistical & Spatial Tools (2026-03-24)
Successfully ported:
- inverse_pca
- principal_component_analysis
- raster_calculator
- trend_surface
- trend_surface_vector_points
- turning_bands_simulation
- zonal_statistics

## Previously Ported Batches
- random_forest_classification
- random_forest_regression
- random_forest_classification_fit
- random_forest_classification_predict
- random_forest_regression_fit
- random_forest_regression_predict
- logistic_regression
- svm_classification
- svm_regression
- raster_summary_stats
- raster_histogram
- list_unique_values_raster
- z_scores
- rescale_value_range
- list_unique_values
- max
- min
- quantiles
- root_mean_square_error
- random_field
- random_sample
- cumulative_distribution
- crispness_index
- ks_normality_test
- inplace_add
- inplace_subtract
- inplace_multiply
- inplace_divide
- attribute_histogram
- attribute_scattergram
- attribute_correlation
- cross_tabulation
- kappa_index
- paired_sample_t_test
- two_sample_ks_test
- wilcoxon_signed_rank_test
- conditional_evaluation
- anova
- phi_coefficient
- image_correlation
- image_autocorrelation
- image_correlation_neighbourhood_analysis
- image_regression

## Remaining Legacy Math IDs
None — all legacy math tools (except intentional exceptions) are now ported!

## Intentionally Skipped Legacy Math IDs (1)
- hdbscan_clustering

## Complete Ported Legacy Math IDs (98)
- abs
- add
- anova
- arccos
- arcosh
- arcsin
- arctan
- arsinh
- artanh
- attribute_correlation
- attribute_histogram
- attribute_scattergram
- atan2
- bool_and
- bool_not
- bool_or
- bool_xor
- ceil
- conditional_evaluation
- cos
- cosh
- crispness_index
- cross_tabulation
- cumulative_distribution
- decrement
- divide
- equal_to
- exp
- exp2
- floor
- greater_than
- image_autocorrelation
- image_correlation
- image_correlation_neighbourhood_analysis
- image_regression
- inverse_pca
- dbscan
- increment
- inplace_add
- inplace_divide
- inplace_multiply
- inplace_subtract
- integer_division
- is_nodata
- kappa_index
- ks_normality_test
- less_than
- list_unique_values
- list_unique_values_raster
- ln
- log10
- log2
- logistic_regression
- max
- min
- modulo
- multiply
- negate
- not_equal_to
- paired_sample_t_test
- phi_coefficient
- power
- principal_component_analysis
- quantiles
- random_field
- random_forest_classification
- random_forest_classification_fit
- random_forest_classification_predict
- random_forest_regression
- random_forest_regression_fit
- random_forest_regression_predict
- random_sample
- raster_calculator
- raster_histogram
- raster_summary_stats
- reciprocal
- rescale_value_range
- root_mean_square_error
- round
- sin
- sinh
- sqrt
- square
- subtract
- tan
- tanh
- to_degrees
- to_radians
- trend_surface
- trend_surface_vector_points
- truncate
- turning_bands_simulation
- z_scores
- zonal_statistics
- sinh
- sqrt
- square
- subtract
- svm_classification
- svm_regression
- tan
- tanh
- to_degrees
- to_radians
- truncate
- two_sample_ks_test
- wilcoxon_signed_rank_test
- z_scores
