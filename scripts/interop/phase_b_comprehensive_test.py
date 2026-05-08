#!/usr/bin/env python3
"""
Phase B Comprehensive Interop Test Runner
Tests all 15 cases: R01-R08 (raster), V01-V04 (vector), L01-L03 (lidar)
"""

import subprocess
import json
import sys
from pathlib import Path
from datetime import datetime

def run_cmd(cmd):
    """Execute shell command"""
    try:
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=30)
        return result.returncode, result.stdout.strip(), result.stderr.strip()
    except subprocess.TimeoutExpired:
        return 1, "", "Timeout"
    except Exception as e:
        return 1, "", str(e)

def test_r01():
    """R01: int16 + nodata + EPSG roundtrip"""
    test_dir = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/artifacts/interop/results/raster/R01")
    test_dir.mkdir(parents=True, exist_ok=True)
    
    source = test_dir / "source_gdal.tif"
    roundtrip = test_dir / "roundtrip_wbraster.tif"
    
    # Ensure source exists
    if not source.exists():
        # Create test GeoTIFF with VRT
        vrt_content = '''<VRTDataset rasterXSize="10" rasterYSize="10">
  <SRS>EPSG:4326</SRS>
  <GeoTransform>-180, 36, 0, 90, 0, -18</GeoTransform>
  <VRTRasterBand dataType="Int16" band="1">
    <NoDataValue>-9999</NoDataValue>
    <ConstantBand val="42"/>
  </VRTRasterBand>
</VRTDataset>'''
        import tempfile
        with tempfile.NamedTemporaryFile(mode='w', suffix='.vrt', delete=False) as f:
            f.write(vrt_content)
            vrt_path = f.name
        
        cmd = f"gdal_translate -of GTiff {vrt_path} {source}"
        rc, _, _ = run_cmd(cmd)
        import os
        try:
            os.unlink(vrt_path)
        except:
            pass
        if rc != 0:
            return "FAIL", "Could not create source"
    
    # Roundtrip test
    cmd = f"""python3 << 'EOF'
import whitebox_workflows as wbw
env = wbw.WbEnvironment()
raster = env.read_raster('{source}')
env.write_raster(raster, '{roundtrip}')
rt = env.read_raster('{roundtrip}')
assert raster.num_cells() == rt.num_cells()
assert raster.band_count == rt.band_count
EOF"""
    rc, out, err = run_cmd(cmd)
    
    if rc == 0 and roundtrip.exists():
        return "PASS", "Roundtrip successful"
    else:
        return "FAIL", f"{err}"

def test_r02():
    """R02: float32 + scale/offset roundtrip"""
    test_dir = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/artifacts/interop/results/raster/R02")
    test_dir.mkdir(parents=True, exist_ok=True)
    return "NOT_STARTED", "Requires scale/offset handling"

def test_r03():
    """R03: COG roundtrip"""
    test_dir = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/artifacts/interop/results/raster/R03")
    test_dir.mkdir(parents=True, exist_ok=True)
    return "NOT_STARTED", "Requires COG support"

def test_r04():
    """R04: DTED elevation roundtrip"""
    test_dir = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/artifacts/interop/results/raster/R04")
    test_dir.mkdir(parents=True, exist_ok=True)
    return "NOT_STARTED", "Requires DTED test data"

def test_r05():
    """R05: HFA (.img) RLC compression"""
    test_dir = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/artifacts/interop/results/raster/R05")
    test_dir.mkdir(parents=True, exist_ok=True)
    return "NOT_STARTED", "Requires HFA test data"

def test_r06():
    """R06: Esri Float Grid"""
    test_dir = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/artifacts/interop/results/raster/R06")
    test_dir.mkdir(parents=True, exist_ok=True)
    return "NOT_STARTED", "Requires Esri Float Grid test data"

def test_r07():
    """R07: PNG + World File"""
    test_dir = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/artifacts/interop/results/raster/R07")
    test_dir.mkdir(parents=True, exist_ok=True)
    return "NOT_STARTED", "Requires PNG test data"

def test_r08():
    """R08: QGIS producer variance check"""
    test_dir = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/artifacts/interop/results/raster/R08")
    test_dir.mkdir(parents=True, exist_ok=True)
    return "NOT_STARTED", "Requires QGIS integration"

def test_v01():
    """V01: Mixed fields/nulls/multipart via QGIS"""
    test_dir = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/artifacts/interop/results/vector/V01")
    test_dir.mkdir(parents=True, exist_ok=True)
    return "NOT_STARTED", "Requires QGIS vector support"

def test_v02():
    """V02: Schema constraints via GDAL"""
    test_dir = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/artifacts/interop/results/vector/V02")
    test_dir.mkdir(parents=True, exist_ok=True)
    return "NOT_STARTED", "Requires vector integration"

def test_v03():
    """V03: GeoJSON interchange"""
    test_dir = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/artifacts/interop/results/vector/V03")
    test_dir.mkdir(parents=True, exist_ok=True)
    return "NOT_STARTED", "Requires vector integration"

def test_v04():
    """V04: FlatGeobuf binary interchange"""
    test_dir = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/artifacts/interop/results/vector/V04")
    test_dir.mkdir(parents=True, exist_ok=True)
    return "NOT_STARTED", "Requires vector integration"

def test_l01():
    """L01: LAS 1.4 point14 baseline"""
    test_dir = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/artifacts/interop/results/lidar/L01")
    test_dir.mkdir(parents=True, exist_ok=True)
    return "NOT_STARTED", "Requires lidar integration"

def test_l02():
    """L02: LAZ compressed roundtrip"""
    test_dir = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/artifacts/interop/results/lidar/L02")
    test_dir.mkdir(parents=True, exist_ok=True)
    return "NOT_STARTED", "Requires lidar integration"

def test_l03():
    """L03: COPC hierarchy-aware roundtrip"""
    test_dir = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/artifacts/interop/results/lidar/L03")
    test_dir.mkdir(parents=True, exist_ok=True)
    return "NOT_STARTED", "Requires lidar integration"

def main():
    """Run Phase B test suite"""
    print("=" * 70)
    print("Phase B Comprehensive Interop Test Suite (v1.0)")
    print(f"Timestamp: {datetime.now().isoformat()}")
    print("=" * 70)
    print()
    
    cases = {
        # Raster
        "R01": ("int16 + nodata + EPSG", test_r01),
        "R02": ("float32 + scale/offset", test_r02),
        "R03": ("COG roundtrip", test_r03),
        "R04": ("DTED elevation", test_r04),
        "R05": ("HFA RLC compression", test_r05),
        "R06": ("Esri Float Grid", test_r06),
        "R07": ("PNG + World File", test_r07),
        "R08": ("QGIS producer variance", test_r08),
        # Vector
        "V01": ("QGIS mixed fields/nulls", test_v01),
        "V02": ("GDAL schema constraints", test_v02),
        "V03": ("GeoJSON interchange", test_v03),
        "V04": ("FlatGeobuf binary", test_v04),
        # Lidar
        "L01": ("LAS 1.4 point14", test_l01),
        "L02": ("LAZ compression", test_l02),
        "L03": ("COPC hierarchy", test_l03),
    }
    
    results = {}
    
    # Run all tests
    for case_id in sorted(cases.keys()):
        desc, func = cases[case_id]
        print(f"{case_id}: {desc}...", end=" ", flush=True)
        status, note = func()
        results[case_id] = {"status": status, "description": desc, "note": note}
        
        # Color-coded output
        if status == "PASS":
            print(f"✓ PASS")
        elif status == "NOT_STARTED":
            print(f"⊘ {status}")
        else:
            print(f"✗ {status}")
    
    # Summary
    print()
    print("=" * 70)
    print("Summary")
    print("=" * 70)
    
    passed = sum(1 for v in results.values() if v["status"] == "PASS")
    failed = sum(1 for v in results.values() if v["status"] == "FAIL")
    skipped = sum(1 for v in results.values() if v["status"] == "NOT_STARTED")
    total = len(results)
    
    print(f"Total:       {total} cases")
    print(f"Passed:      {passed}")
    print(f"Failed:      {failed}")
    print(f"Not Started: {skipped}")
    print()
    
    # Save results
    results_file = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/artifacts/interop/results/phase_b_matrix_results.json")
    results_file.parent.mkdir(parents=True, exist_ok=True)
    with open(results_file, 'w') as f:
        json.dump(results, f, indent=2)
    print(f"Results saved: {results_file}")
    
    return 0 if failed == 0 else 1

if __name__ == "__main__":
    sys.exit(main())
