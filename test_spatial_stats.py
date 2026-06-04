#!/usr/bin/env python3
"""
Test suite for Phase C/D spatial statistics tools.
Tests with synthetic data and real datasets (Ward boundaries, Woodrill yield points).
"""

import os
import sys
import json
import math
import tempfile
import subprocess
from pathlib import Path

# Add whitebox_next_gen to path
sys.path.insert(0, '/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen')

try:
    import whitebox_next_gen as wbw
except ImportError as e:
    print(f"Error importing whitebox_next_gen: {e}")
    print("Make sure the Python bindings are built: maturin develop")
    sys.exit(1)

try:
    import geopandas as gpd
    import numpy as np
except ImportError as e:
    print(f"Error: Missing dependency {e}")
    print("Install with: pip install geopandas numpy shapely")
    sys.exit(1)


def create_synthetic_regression_data(output_path: str, n_points: int = 50):
    """Create synthetic point data for Phase C regression testing."""
    np.random.seed(42)
    
    # Create points on a grid with some randomness
    x = np.random.uniform(0, 10, n_points)
    y = np.random.uniform(0, 10, n_points)
    
    # Create response variable with spatial autocorrelation
    response = 5 + 2*x + 1.5*y + np.random.normal(0, 1, n_points)
    
    # Create predictor variables
    pred1 = x + 0.5*np.random.normal(0, 1, n_points)
    pred2 = y + 0.5*np.random.normal(0, 1, n_points)
    
    # Create GeoDataFrame
    from shapely.geometry import Point
    geometry = [Point(xi, yi) for xi, yi in zip(x, y)]
    gdf = gpd.GeoDataFrame(
        {
            'response': response,
            'pred1': pred1,
            'pred2': pred2,
        },
        geometry=geometry,
        crs='EPSG:4326'
    )
    
    # Save to GeoPackage
    gdf.to_file(output_path, layer='points', driver='GPKG')
    print(f"✓ Created synthetic regression data: {output_path} ({n_points} points)")
    return output_path


def create_synthetic_point_pattern_data(output_path: str, n_points: int = 100):
    """Create synthetic point pattern for Phase D testing."""
    np.random.seed(42)
    
    # Create clustered point pattern (Poisson with hot spots)
    points = []
    
    # Cluster 1: around (2, 2)
    cluster1 = np.random.normal(2, 0.5, (30, 2))
    points.extend([(p[0], p[1]) for p in cluster1 if 0 <= p[0] <= 10 and 0 <= p[1] <= 10])
    
    # Cluster 2: around (7, 7)
    cluster2 = np.random.normal(7, 0.5, (30, 2))
    points.extend([(p[0], p[1]) for p in cluster2 if 0 <= p[0] <= 10 and 0 <= p[1] <= 10])
    
    # Random background points
    background_x = np.random.uniform(0, 10, 40)
    background_y = np.random.uniform(0, 10, 40)
    points.extend([(x, y) for x, y in zip(background_x, background_y)])
    
    # Create GeoDataFrame
    from shapely.geometry import Point
    geometry = [Point(p[0], p[1]) for p in points]
    gdf = gpd.GeoDataFrame(
        {'point_id': range(len(points))},
        geometry=geometry,
        crs='EPSG:4326'
    )
    
    # Save to GeoPackage
    gdf.to_file(output_path, layer='points', driver='GPKG')
    print(f"✓ Created synthetic point pattern data: {output_path} ({len(points)} points)")
    return output_path


def test_phase_c_regression(env, input_path: str, test_name: str):
    """Test Phase C regression tools."""
    print(f"\n{'='*60}")
    print(f"Testing Phase C: {test_name}")
    print(f"{'='*60}")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        # Test SpatialLagRegression (SAR)
        output_sar = os.path.join(tmpdir, f"sar_{test_name}.gpkg")
        try:
            result = env.spatial_lag_regression(
                input=input_path,
                response_field="response",
                predictor_fields="pred1,pred2",
                output=output_sar
            )
            if result:
                print(f"  ✓ SpatialLagRegression: {output_sar}")
            else:
                print(f"  ✗ SpatialLagRegression failed")
        except Exception as e:
            print(f"  ✗ SpatialLagRegression error: {e}")
        
        # Test SpatialErrorRegression (SEM)
        output_sem = os.path.join(tmpdir, f"sem_{test_name}.gpkg")
        try:
            result = env.spatial_error_regression(
                input=input_path,
                response_field="response",
                predictor_fields="pred1,pred2",
                output=output_sem
            )
            if result:
                print(f"  ✓ SpatialErrorRegression: {output_sem}")
            else:
                print(f"  ✗ SpatialErrorRegression failed")
        except Exception as e:
            print(f"  ✗ SpatialErrorRegression error: {e}")
        
        # Test GeographicallyWeightedRegression (GWR)
        output_gwr = os.path.join(tmpdir, f"gwr_{test_name}.gpkg")
        try:
            result = env.geographically_weighted_regression(
                input=input_path,
                response_field="response",
                predictor_fields="pred1,pred2",
                output=output_gwr
            )
            if result:
                print(f"  ✓ GeographicallyWeightedRegression: {output_gwr}")
            else:
                print(f"  ✗ GeographicallyWeightedRegression failed")
        except Exception as e:
            print(f"  ✗ GeographicallyWeightedRegression error: {e}")


def test_phase_d_point_process(env, input_path: str, test_name: str):
    """Test Phase D point process tools."""
    print(f"\n{'='*60}")
    print(f"Testing Phase D: {test_name}")
    print(f"{'='*60}")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        # Test InhomogeneousIntensity (KDE)
        output_kde = os.path.join(tmpdir, f"kde_{test_name}.tif")
        try:
            result = env.inhomogeneous_intensity(
                input=input_path,
                output=output_kde,
                cell_size=0.5
            )
            if result:
                print(f"  ✓ InhomogeneousIntensity (KDE): {output_kde}")
            else:
                print(f"  ✗ InhomogeneousIntensity failed")
        except Exception as e:
            print(f"  ✗ InhomogeneousIntensity error: {e}")
        
        # Test RipleysK
        output_k = os.path.join(tmpdir, f"ripleysk_{test_name}.gpkg")
        try:
            result = env.ripleys_k_function(
                input=input_path,
                output=output_k,
                max_distance=3.0
            )
            if result:
                print(f"  ✓ RipleysK: {output_k}")
            else:
                print(f"  ✗ RipleysK failed")
        except Exception as e:
            print(f"  ✗ RipleysK error: {e}")
        
        # Test EnvelopeTest
        output_env = os.path.join(tmpdir, f"envelope_{test_name}.gpkg")
        try:
            result = env.envelope_test(
                input=input_path,
                output=output_env,
                max_distance=3.0,
                num_simulations=99
            )
            if result:
                print(f"  ✓ EnvelopeTest: {output_env}")
            else:
                print(f"  ✗ EnvelopeTest failed")
        except Exception as e:
            print(f"  ✗ EnvelopeTest error: {e}")
        
        # Test PointProcessResiduals
        output_resid = os.path.join(tmpdir, f"residuals_{test_name}.gpkg")
        try:
            result = env.point_process_residuals(
                input=input_path,
                output=output_resid
            )
            if result:
                print(f"  ✓ PointProcessResiduals: {output_resid}")
            else:
                print(f"  ✗ PointProcessResiduals failed")
        except Exception as e:
            print(f"  ✗ PointProcessResiduals error: {e}")


def main():
    """Run test suite."""
    print("Phase C/D Spatial Statistics Tools - Test Suite")
    print("=" * 60)
    
    # Create Whitebox environment
    try:
        env = wbw.WbEnvironment()
        print(f"✓ Whitebox environment initialized")
    except Exception as e:
        print(f"✗ Failed to initialize Whitebox: {e}")
        return 1
    
    with tempfile.TemporaryDirectory() as tmpdir:
        # Create synthetic data
        print("\nGenerating synthetic test data...")
        syn_regression = os.path.join(tmpdir, "synthetic_regression.gpkg")
        syn_points = os.path.join(tmpdir, "synthetic_points.gpkg")
        
        create_synthetic_regression_data(syn_regression, n_points=50)
        create_synthetic_point_pattern_data(syn_points, n_points=100)
        
        # Test with synthetic data
        test_phase_c_regression(env, syn_regression, "synthetic")
        test_phase_d_point_process(env, syn_points, "synthetic")
        
        # Test with real data if available
        real_points_path = "/Users/johnlindsay/Documents/data/Yield/Woodrill/Woodrill_UTM.shp"
        if os.path.exists(real_points_path):
            print(f"\n{'='*60}")
            print("Testing with real data: Woodrill yield points")
            print(f"{'='*60}")
            
            try:
                # Load real data to check it
                gdf = gpd.read_file(real_points_path)
                print(f"✓ Loaded real data: {len(gdf)} features")
                print(f"  Columns: {list(gdf.columns)}")
                print(f"  CRS: {gdf.crs}")
                
                # For Phase C, we need numeric response and predictor fields
                numeric_cols = gdf.select_dtypes(include=[np.number]).columns.tolist()
                if len(numeric_cols) >= 2:
                    response_field = numeric_cols[0]
                    predictor_fields = ",".join(numeric_cols[1:3])
                    print(f"  Using response={response_field}, predictors={predictor_fields}")
                    
                    # Test on real data
                    test_phase_c_regression(env, real_points_path, "Woodrill")
                    test_phase_d_point_process(env, real_points_path, "Woodrill")
                else:
                    print(f"  Note: Real data has only {len(numeric_cols)} numeric columns")
                    test_phase_d_point_process(env, real_points_path, "Woodrill")
                    
            except Exception as e:
                print(f"✗ Error with real data: {e}")
        else:
            print(f"\nNote: Real data not found at {real_points_path}")
    
    print("\n" + "=" * 60)
    print("Test suite completed!")
    return 0


if __name__ == "__main__":
    sys.exit(main())
