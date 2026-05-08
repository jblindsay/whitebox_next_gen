#!/usr/bin/env python3
"""
Phase B Raster Interop Test Runner
Validates roundtrip read/write for raster formats using GDAL producer + wbraster
"""

import subprocess
import json
import sys
import os
from pathlib import Path
from datetime import datetime

def run_command(cmd, description=""):
    """Execute shell command and return output"""
    try:
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=30)
        return result.returncode, result.stdout, result.stderr
    except subprocess.TimeoutExpired:
        return 1, "", f"Timeout: {description}"
    except Exception as e:
        return 1, "", str(e)

def test_case_r01():
    """R01: int16 + nodata + EPSG roundtrip via wbraster"""
    print("\n=== R01: int16 + nodata + EPSG roundtrip ===")
    
    test_dir = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/artifacts/interop/results/raster/R01")
    test_dir.mkdir(parents=True, exist_ok=True)
    
    source_tif = test_dir / "source_gdal.tif"
    roundtrip_tif = test_dir / "roundtrip_wbraster.tif"
    
    # Step 1: Create source with GDAL
    print("Step 1: Creating source artifact with GDAL...")
    cmd = f"""python3 << 'EOF'
try:
    from osgeo import gdal, osr
    import numpy as np
    
    # Create test data
    data = np.arange(100, dtype=np.int16).reshape(10, 10)
    
    # Create GeoTIFF
    driver = gdal.GetDriverByName('GTiff')
    ds = driver.Create('{source_tif}', 10, 10, 1, gdal.GDT_Int16)
    
    # Set geotransform (EPSG:4326)
    ds.SetGeoTransform([-180.0, 36.0, 0, 90.0, 0, -18.0])
    
    # Set projection
    srs = osr.SpatialReference()
    srs.ImportFromEPSG(4326)
    ds.SetProjection(srs.ExportToWkt())
    
    # Write data and nodata
    band = ds.GetRasterBand(1)
    band.WriteArray(data)
    band.SetNoDataValue(-9999)
    
    ds = None
    print("✓ Source created")
except Exception as e:
    print(f"✗ Failed: {{e}}")
    exit(1)
EOF"""
    rc, out, err = run_command(cmd)
    if rc != 0:
        print(f"✗ GDAL source creation failed: {err}")
        return "FAIL", f"GDAL source creation: {err}"
    print(out.strip())
    
    if not source_tif.exists():
        return "FAIL", "Source file not created"
    print(f"✓ Source size: {source_tif.stat().st_size} bytes")
    
    # Step 2: Roundtrip with wbraster
    print("Step 2: Reading and writing with wbraster...")
    cmd = f"""python3 << 'EOF'
try:
    import whitebox_workflows as wbw
    
    # Read with wbraster
    raster = wbw.read_raster('{source_tif}')
    print(f"✓ Read: {{raster.cols}}x{{raster.rows}}, {{raster.data_type}}")
    print(f"  NoData: {{raster.nodata}}")
    print(f"  CRS: {{raster.crs}}")
    
    # Write roundtrip
    wbw.write_raster(raster, '{roundtrip_tif}')
    print(f"✓ Written: {roundtrip_tif}")
except Exception as e:
    print(f"✗ Failed: {{e}}")
    import traceback
    traceback.print_exc()
    exit(1)
EOF"""
    rc, out, err = run_command(cmd)
    if rc != 0:
        print(f"✗ Roundtrip failed:\n{out}\n{err}")
        return "FAIL", f"Roundtrip: {err}"
    print(out.strip())
    
    if not roundtrip_tif.exists():
        return "FAIL", "Roundtrip file not created"
    print(f"✓ Roundtrip size: {roundtrip_tif.stat().st_size} bytes")
    
    # Step 3: Validate
    print("Step 3: Validating metadata consistency...")
    cmd = f"""python3 << 'EOF'
try:
    from osgeo import gdal
    
    source = gdal.Open('{source_tif}')
    roundtrip = gdal.Open('{roundtrip_tif}')
    
    if not source or not roundtrip:
        print("✗ Failed to open rasters")
        exit(1)
    
    s_band = source.GetRasterBand(1)
    r_band = roundtrip.GetRasterBand(1)
    
    checks = {{
        "Size match": (source.RasterXSize == roundtrip.RasterXSize and 
                       source.RasterYSize == roundtrip.RasterYSize),
        "NoData match": s_band.GetNoDataValue() == r_band.GetNoDataValue(),
        "Data type match": s_band.DataType == r_band.DataType,
        "Geotransform match": source.GetGeoTransform() == roundtrip.GetGeoTransform(),
    }}
    
    for name, result in checks.items():
        status = "✓" if result else "✗"
        print(f"{{status}} {{name}}")
    
    if all(checks.values()):
        print("✓ Validation PASS")
        exit(0)
    else:
        print("✗ Validation FAIL")
        exit(1)
except Exception as e:
    print(f"✗ Error: {{e}}")
    exit(1)
EOF"""
    rc, out, err = run_command(cmd)
    print(out.strip())
    
    return ("PASS" if rc == 0 else "FAIL"), out

def main():
    """Run Phase B raster test suite"""
    print("=" * 70)
    print("Phase B Raster Interop Test Suite (v1)")
    print(f"Started: {datetime.now().isoformat()}")
    print("=" * 70)
    
    results_dir = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/artifacts/interop/results/raster")
    results_dir.mkdir(parents=True, exist_ok=True)
    
    results = {}
    
    # Run test cases
    results['R01'], results['R01_details'] = test_case_r01()
    
    # Summary
    print("\n" + "=" * 70)
    print("Summary")
    print("=" * 70)
    for case_id in ['R01']:
        status = results.get(case_id, 'UNKNOWN')
        print(f"  {case_id}: {status}")
    
    # Write results
    results_file = results_dir / "phase_b_raster_results_v1.json"
    with open(results_file, 'w') as f:
        json.dump(results, f, indent=2)
    print(f"\nResults written to: {results_file}")
    
    return 0 if all(v == "PASS" for k, v in results.items() if k.startswith('R')) else 1

if __name__ == "__main__":
    sys.exit(main())
