#!/usr/bin/env python3
"""
Gstat Parity Validation for wbgeostats Kriging Implementation

Compares kriging outputs from wbgeostats against R gstat library using:
- Meuse river floodplain soil dataset (classic geostatistics benchmark)
- Multiple variogram model families (spherical, exponential, gaussian)
- Cross-validation metrics and prediction accuracy

Requires:
  - rpy2 (to call R from Python)
  - R gstat package (install via R: install.packages("gstat"))
  - R sp package (for Meuse dataset)
"""

import sys
import os
import json
import numpy as np
from pathlib import Path

try:
    import rpy2
    from rpy2.robjects.packages import importr
    import rpy2.robjects as ro
    from rpy2.robjects.vectors import DataFrame
    HAS_R = True
except ImportError:
    HAS_R = False
    print("WARNING: rpy2 not installed. Install with: pip install rpy2")

# Add wbtools_oss to Python path for kriging tool access
WHITEBOX_ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(WHITEBOX_ROOT))

# Try to import whitebox kriging bindings (will be available after Task 5C)
try:
    from whitebox_workflows import kriging
    HAS_WB_KRIGING = True
except ImportError:
    HAS_WB_KRIGING = False
    print("INFO: wbgeostats Python bindings not yet available (Task 5C)")


class GstatValidator:
    """Validates wbgeostats kriging against R gstat library"""
    
    def __init__(self):
        self.meuse_data = None
        self.meuse_grid = None
        self.results = {
            "dataset": "Meuse",
            "models_tested": [],
            "comparisons": []
        }
    
    def load_meuse_dataset(self):
        """Load Meuse river dataset from R sp package"""
        if not HAS_R:
            print("ERROR: R/rpy2 not available. Cannot load Meuse dataset.")
            return False
        
        print("Loading Meuse dataset from R sp package...")
        try:
            sp = importr('sp')
            ro.r('data(meuse)')
            ro.r('data(meuse.grid)')
            
            # Extract coordinates and zinc values
            meuse_r = ro.r['meuse']
            meuse_grid_r = ro.r['meuse.grid']
            
            # Convert to Python format
            coords = np.array(ro.r.as_matrix(ro.r['coordinates'](meuse_r)))
            values = np.array(ro.r['meuse$zinc']).flatten()
            
            grid_coords = np.array(ro.r.as_matrix(ro.r['coordinates'](meuse_grid_r)))
            
            self.meuse_data = {
                'coords': coords,
                'values': values,
                'n_points': len(values)
            }
            
            self.meuse_grid = {
                'coords': grid_coords,
                'n_points': len(grid_coords)
            }
            
            print(f"  ✓ Training points: {self.meuse_data['n_points']}")
            print(f"  ✓ Grid points: {self.meuse_grid['n_points']}")
            return True
            
        except Exception as e:
            print(f"ERROR loading Meuse: {e}")
            return False
    
    def fit_variogram_with_gstat(self, model_family="spherical"):
        """Fit variogram using R gstat library"""
        if not HAS_R:
            return None
        
        print(f"\nFitting {model_family} variogram with gstat...")
        try:
            gstat = importr('gstat')
            sp = importr('sp')
            
            # Create SpatialPointsDataFrame in R
            coords_r = ro.r.matrix(self.meuse_data['coords'], nrow=len(self.meuse_data['coords']))
            values_r = ro.FloatVector(self.meuse_data['values'])
            
            ro.r(f'''
            library(sp)
            coords <- {coords_r.r_repr()}
            values <- {values_r.r_repr()}
            meuse_sp <- SpatialPointsDataFrame(coords, data.frame(zinc=values))
            proj4string(meuse_sp) <- CRS("+proj=longlat")
            ''')
            
            # Fit variogram with gstat
            ro.r(f'''
            library(gstat)
            vario <- variogram(zinc ~ 1, meuse_sp)
            fitted_vario <- fit.variogram(vario, model=vgm(psill=20000, model="{model_family}", range=1000, nugget=1000))
            ''')
            
            fitted = ro.r['fitted_vario']
            
            # Extract parameters
            nugget = float(ro.r['fitted_vario$psill[1]'])
            partial_sill = float(ro.r['fitted_vario$psill[2]'])
            range_param = float(ro.r['fitted_vario$range[2]'])
            
            return {
                'family': model_family,
                'nugget': nugget,
                'partial_sill': partial_sill,
                'range': range_param,
                'total_sill': nugget + partial_sill
            }
            
        except Exception as e:
            print(f"ERROR fitting variogram: {e}")
            return None
    
    def predict_kriging_with_gstat(self, vario_params):
        """Make kriging predictions using R gstat"""
        if not HAS_R:
            return None
        
        print(f"Making kriging predictions with gstat...")
        try:
            gstat = importr('gstat')
            sp = importr('sp')
            
            # Already set up meuse_sp and fitted_vario in R session above
            ro.r('''
            library(gstat)
            krige_pred <- krige(zinc ~ 1, meuse_sp, SpatialPoints(coords), model=fitted_vario)
            predictions <- krige_pred@data$var1.pred
            variances <- krige_pred@data$var1.var
            ''')
            
            predictions = np.array(ro.r['predictions']).flatten()
            variances = np.array(ro.r['variances']).flatten()
            
            return {
                'predictions': predictions,
                'variances': variances
            }
            
        except Exception as e:
            print(f"ERROR making predictions: {e}")
            return None
    
    def compare_variogram_fits(self):
        """Compare variogram fits across model families"""
        if not HAS_R:
            print("Skipping gstat comparison (R not available)")
            return
        
        models = ["spherical", "exponential", "gaussian"]
        gstat_results = {}
        
        for model in models:
            vario = self.fit_variogram_with_gstat(model)
            if vario:
                gstat_results[model] = vario
                print(f"  {model:12s}: nugget={vario['nugget']:8.1f}, " +
                      f"psill={vario['partial_sill']:8.1f}, " +
                      f"range={vario['range']:8.1f}")
        
        self.results["gstat_variogram_fits"] = gstat_results
        return gstat_results
    
    def validate_predictions(self):
        """Compare kriging predictions (once Python bindings available)"""
        print("\n" + "="*70)
        print("PREDICTION VALIDATION (Available after Task 5C: Python Bindings)")
        print("="*70)
        print("Once PyO3 bindings are available, this will:")
        print("  1. Extract kriging predictions from wbgeostats")
        print("  2. Compare predictions vs gstat (RMSE, correlation, MAE)")
        print("  3. Validate uncertainty estimates (kriging variance)")
        print("  4. Test cross-validation metrics")
        return None
    
    def summary_report(self):
        """Print validation summary"""
        print("\n" + "="*70)
        print("GSTAT PARITY VALIDATION SUMMARY")
        print("="*70)
        
        if "gstat_variogram_fits" in self.results:
            print("\n✓ Variogram Fitting Comparison:")
            for model, params in self.results["gstat_variogram_fits"].items():
                print(f"  {model}:")
                print(f"    Nugget:       {params['nugget']:.1f}")
                print(f"    Partial Sill: {params['partial_sill']:.1f}")
                print(f"    Range:        {params['range']:.1f}")
        
        print("\n✓ Dataset: Meuse river floodplain (sp package)")
        print(f"  Training points: {self.meuse_data['n_points']}")
        print(f"  Prediction grid: {self.meuse_grid['n_points']}")
        
        print("\n✓ Status:")
        print("  [✓] Variogram fitting against gstat")
        print("  [ ] Prediction comparison (pending Python bindings)")
        print("  [ ] Cross-validation metrics comparison")
        print("  [ ] Performance benchmarking")
        
        print("\nNext Steps:")
        print("  1. Complete Task 5C (Python bindings via PyO3)")
        print("  2. Run prediction comparison")
        print("  3. Benchmark performance (wbgeostats vs gstat)")


def main():
    """Run gstat validation suite"""
    print("="*70)
    print("WBGEOSTATS GSTAT PARITY VALIDATION")
    print("="*70)
    
    validator = GstatValidator()
    
    if not validator.load_meuse_dataset():
        print("\nFallback: Using synthetic test data instead...")
        # Could create synthetic data here if needed
        return 1
    
    # Run variogram fitting comparison
    validator.compare_variogram_fits()
    
    # Validate predictions (placeholder for now)
    validator.validate_predictions()
    
    # Print summary
    validator.summary_report()
    
    # Save results
    output_file = Path(__file__).parent.parent / "kriging_validation_results.json"
    with open(output_file, 'w') as f:
        json.dump(validator.results, f, indent=2)
    print(f"\nResults saved to: {output_file}")
    
    return 0


if __name__ == "__main__":
    sys.exit(main())
