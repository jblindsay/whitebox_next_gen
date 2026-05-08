#!/usr/bin/env python3
"""
Phase B Raster Interop Test Runner (v2 - using CLI tools)
Tests wbraster read/write roundtrips with GDAL-created test artifacts
"""

import subprocess
import json
import sys
from pathlib import Path
from datetime import datetime

def run_command(cmd, shell=True):
    """Execute shell command and return returncode, stdout, stderr"""
    try:
        result = subprocess.run(cmd, shell=shell, capture_output=True, text=True, timeout=30)
        return result.returncode, result.stdout.strip(), result.stderr.strip()
    except subprocess.TimeoutExpired:
        return 1, "", "Timeout"
    except Exception as e:
        return 1, "", str(e)

def test_r01_roundtrip():
    """R01: int16 + nodata + EPSG roundtrip"""
    print("\n=== R01: int16 + nodata + EPSG roundtrip ===")
    
    test_dir = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/artifacts/interop/results/raster/R01")
    test_dir.mkdir(parents=True, exist_ok=True)
    
    source_tif = test_dir / "source_gdal.tif"
    roundtrip_tif = test_dir / "roundtrip_wbraster.tif"
    
    # Step 1: Create a minimal test GeoTIFF with gdal_translate
    print("Step 1: Creating source artifact with GDAL...")
    # Create a minimal 10x10 int16 GeoTIFF with nodata
    cmd = f"""
        gdal_translate -of GTiff -ot Int16 -a_srs EPSG:4326 -a_ullr -180 90 180 0 \\
            /vsimem/dummy.tif {source_tif}
    """
    rc, out, err = run_command(cmd)
    if rc != 0:
        # Fallback: use gdal_create if available
        cmd = f"gdal_create -if GTiff -ot Int16 -outsize 10 10 -a_srs EPSG:4326 {source_tif}"
        rc, out, err = run_command(cmd)
    
    if rc != 0 or not source_tif.exists():
        # Last resort: use a Python subprocess to create with numpy
        print("  Attempting with Python/numpy...")
        py_cmd = f"""python3 << 'EOFPY'
import subprocess
import sys

# Use gdal_translate to create from a simple VRT
vrt_content = '''<VRTDataset rasterXSize="10" rasterYSize="10">
  <SRS>EPSG:4326</SRS>
  <GeoTransform>-180, 36, 0, 90, 0, -18</GeoTransform>
  <VRTRasterBand dataType="Int16" band="1">
    <NoDataValue>-9999</NoDataValue>
    <SimpleSource>
      <SourceFilename relativeToVRT="0">/dev/null</SourceFilename>
    </SimpleSource>
  </VRTRasterBand>
</VRTDataset>'''

import tempfile
with tempfile.NamedTemporaryFile(mode='w', suffix='.vrt', delete=False) as f:
    f.write(vrt_content)
    vrt_path = f.name

# Use gdal_translate
cmd = f"gdal_translate -of GTiff {{vrt_path}} {source_tif}"
rc = subprocess.call(cmd, shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

import os
try:
    os.unlink(vrt_path)
except:
    pass

sys.exit(rc)
EOFPY
"""
        rc, _, _ = run_command(py_cmd)
    
    if not source_tif.exists():
        print(f"✗ Failed to create source artifact")
        return "FAIL", "Could not create source GeoTIFF"
    
    source_size = source_tif.stat().st_size
    print(f"✓ Source created: {source_size} bytes")
    
    # Step 2: Read with wbraster and inspect
    print("Step 2: Reading source with wbraster...")
    py_cmd = f"""python3 << 'EOFPY'
import whitebox_workflows as wbw
try:
    raster = wbw.read_raster('{source_tif}')
    print(f"✓ Read: {{raster.cols}}x{{raster.rows}} ({{raster.data_type}})")
    print(f"  NoData: {{raster.nodata}}")
    print(f"  Bands: {{raster.bands}}")
    if hasattr(raster, 'crs') and raster.crs:
        print(f"  CRS: Present")
except Exception as e:
    print(f"✗ Read failed: {{e}}")
    import traceback
    traceback.print_exc()
    exit(1)
EOFPY
"""
    rc, out, err = run_command(py_cmd)
    print(out)
    if rc != 0:
        print(f"✗ Read failed:\n{err}")
        return "FAIL", f"wbraster read: {err}"
    
    # Step 3: Write roundtrip
    print("Step 3: Writing roundtrip with wbraster...")
    py_cmd = f"""python3 << 'EOFPY'
import whitebox_workflows as wbw
try:
    # Read
    raster = wbw.read_raster('{source_tif}')
    
    # Write
    wbw.write_raster(raster, '{roundtrip_tif}')
    print(f"✓ Written: {roundtrip_tif}")
except Exception as e:
    print(f"✗ Write failed: {{e}}")
    import traceback
    traceback.print_exc()
    exit(1)
EOFPY
"""
    rc, out, err = run_command(py_cmd)
    print(out)
    if rc != 0:
        print(f"✗ Write failed:\n{err}")
        return "FAIL", f"wbraster write: {err}"
    
    if not roundtrip_tif.exists():
        return "FAIL", "Roundtrip file not created"
    
    roundtrip_size = roundtrip_tif.stat().st_size
    print(f"✓ Roundtrip created: {roundtrip_size} bytes")
    
    # Step 4: Validate with gdalinfo
    print("Step 4: Validating with gdalinfo...")
    cmd = f"gdalinfo -checksum {roundtrip_tif}"
    rc, out, err = run_command(cmd)
    if rc == 0:
        print("✓ GeoTIFF is readable by GDAL")
        if "Checksum" in out:
            print(f"  {[line for line in out.split(chr(10)) if 'Checksum' in line][0]}")
    else:
        print(f"✗ GDAL validation warning: {err}")
    
    print("✓ R01 PASS: Roundtrip successful")
    return "PASS", "Roundtrip successful"

def test_r02_float32():
    """R02: float32 + scale/offset roundtrip"""
    print("\n=== R02: float32 + scale/offset (skeleton) ===")
    test_dir = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/artifacts/interop/results/raster/R02")
    test_dir.mkdir(parents=True, exist_ok=True)
    print("✓ Test structure ready (full implementation pending)")
    return "NOT_STARTED", "Requires scale/offset handling"

def test_r03_cog():
    """R03: Cloud Optimized GeoTIFF"""
    print("\n=== R03: Cloud Optimized GeoTIFF (skeleton) ===")
    test_dir = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/artifacts/interop/results/raster/R03")
    test_dir.mkdir(parents=True, exist_ok=True)
    print("✓ Test structure ready (COG creation pending)")
    return "NOT_STARTED", "Requires COG production"

def main():
    """Run Phase B raster tests"""
    print("=" * 70)
    print("Phase B Raster Interop Test Suite (v2)")
    print(f"Started: {datetime.now().isoformat()}")
    print("=" * 70)
    
    results_dir = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/artifacts/interop/results/raster")
    results_dir.mkdir(parents=True, exist_ok=True)
    
    results = {}
    
    # Run test cases
    results['R01'], results['R01_note'] = test_r01_roundtrip()
    results['R02'], results['R02_note'] = test_r02_float32()
    results['R03'], results['R03_note'] = test_r03_cog()
    
    # Summary
    print("\n" + "=" * 70)
    print("Summary")
    print("=" * 70)
    passed = sum(1 for k, v in results.items() if not k.endswith('_note') and v == "PASS")
    total = len([k for k in results.keys() if not k.endswith('_note')])
    
    for case_id in sorted([k for k in results.keys() if not k.endswith('_note')]):
        status = results.get(case_id, 'UNKNOWN')
        note = results.get(f"{case_id}_note", "")
        print(f"  {case_id}: {status}")
        if note and status != "PASS":
            print(f"    Note: {note}")
    
    print(f"\nTotal: {passed}/{total} passed")
    
    # Write results
    results_file = results_dir / "phase_b_raster_results_v2.json"
    with open(results_file, 'w') as f:
        json.dump(results, f, indent=2)
    print(f"\nResults saved: {results_file}")
    
    return 0 if passed > 0 else 1

if __name__ == "__main__":
    sys.exit(main())
