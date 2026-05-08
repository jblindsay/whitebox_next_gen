#!/usr/bin/env python3
"""Phase B comprehensive interop test runner for R01-R08, V01-V04, L01-L03."""

import json
import shutil
import subprocess
import sys
import tempfile
from datetime import datetime
from pathlib import Path

ROOT = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen")
RESULTS_ROOT = ROOT / "artifacts/interop/results"


def run_cmd(cmd, timeout=60):
    """Execute a shell command and return (code, stdout, stderr)."""
    try:
        result = subprocess.run(
            cmd,
            shell=True,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        return result.returncode, result.stdout.strip(), result.stderr.strip()
    except subprocess.TimeoutExpired:
        return 1, "", "Timeout"
    except Exception as exc:
        return 1, "", str(exc)


def ogr_feature_count(path):
    """Return feature count from ogrinfo output when available."""
    rc, out, err = run_cmd(f"ogrinfo -ro -so -al '{path}'")
    if rc != 0:
        return None, err
    for line in out.splitlines():
        line = line.strip()
        if line.lower().startswith("feature count:"):
            try:
                return int(line.split(":", 1)[1].strip()), ""
            except Exception:
                return None, f"Unable to parse feature count from line: {line}"
    return None, "Feature Count line not found in ogrinfo output"


def require_cmd(name):
    """Return True when a command is on PATH."""
    return shutil.which(name) is not None


def get_env():
    """Load wbw environment lazily to keep error reporting per-case."""
    try:
        import whitebox_workflows as wbw

        return wbw.WbEnvironment(), None
    except Exception as exc:
        return None, f"whitebox_workflows import failed: {exc}"


def create_vrt_raster(path, data_type="Int16", value="42", nodata="-9999"):
    """Create a tiny georeferenced raster using gdal_translate from an in-memory VRT."""
    vrt_content = f'''<VRTDataset rasterXSize="10" rasterYSize="10">
  <SRS>EPSG:4326</SRS>
  <GeoTransform>-80, 0.01, 0, 44, 0, -0.01</GeoTransform>
  <VRTRasterBand dataType="{data_type}" band="1">
    <NoDataValue>{nodata}</NoDataValue>
    <ConstantBand val="{value}"/>
  </VRTRasterBand>
</VRTDataset>'''
    with tempfile.NamedTemporaryFile(mode="w", suffix=".vrt", delete=False) as vrt_file:
        vrt_file.write(vrt_content)
        vrt_path = Path(vrt_file.name)
    rc, _, err = run_cmd(f"gdal_translate -of GTiff '{vrt_path}' '{path}'")
    try:
        vrt_path.unlink(missing_ok=True)
    except Exception:
        pass
    if rc != 0:
        return False, f"gdal_translate failed: {err}"
    return True, ""


def raster_roundtrip(source, roundtrip):
    """Read/write/read a raster with wbw and validate core invariants."""
    env, err = get_env()
    if env is None:
        return "FAIL", err
    try:
        src = env.read_raster(str(source))
        env.write_raster(src, str(roundtrip))
        out = env.read_raster(str(roundtrip))
        if src.num_cells() != out.num_cells() or src.band_count != out.band_count:
            return "FAIL", "Mismatch in cell or band count"
        return "PASS", f"cells={src.num_cells()}, bands={src.band_count}"
    except Exception as exc:
        return "FAIL", str(exc)


def vector_roundtrip(source, roundtrip):
    """Read/write/read a vector with wbw and validate feature count."""
    env, err = get_env()
    if env is None:
        return "FAIL", err
    try:
        src = env.read_vector(str(source))
        env.write_vector(src, str(roundtrip))
        out = env.read_vector(str(roundtrip))
        src_count = src.feature_count()
        out_count = out.feature_count()
        if src_count != out_count:
            return "FAIL", f"Feature count mismatch: {src_count} vs {out_count}"
        return "PASS", f"features={src_count}"
    except Exception as exc:
        return "FAIL", str(exc)


def lidar_roundtrip(source, roundtrip):
    """Read/write/read lidar and validate point count."""
    env, err = get_env()
    if env is None:
        return "FAIL", err
    try:
        src = env.read_lidar(str(source))
        env.write_lidar(src, str(roundtrip))
        out = env.read_lidar(str(roundtrip))
        src_count = int(src.point_count)
        out_count = int(out.point_count)
        if src_count != out_count:
            return "FAIL", f"Point count mismatch: {src_count} vs {out_count}"
        return "PASS", f"points={src_count}"
    except Exception as exc:
        return "FAIL", str(exc)


def write_sample_geojson(path, mixed_geometry=True):
    """Create a small feature collection with nullable attributes."""
    if mixed_geometry:
        features = [
            {
                "type": "Feature",
                "properties": {"id": 1, "name": "alpha", "flag": None},
                "geometry": {"type": "Point", "coordinates": [-79.5, 43.5]},
            },
            {
                "type": "Feature",
                "properties": {"id": 2, "name": "beta", "flag": 1},
                "geometry": {
                    "type": "MultiLineString",
                    "coordinates": [[[-79.6, 43.5], [-79.59, 43.52]]],
                },
            },
        ]
    else:
        features = [
            {
                "type": "Feature",
                "properties": {"id": 1, "name": "alpha", "flag": None},
                "geometry": {"type": "Point", "coordinates": [-79.5, 43.5]},
            },
            {
                "type": "Feature",
                "properties": {"id": 2, "name": "beta", "flag": 1},
                "geometry": {"type": "Point", "coordinates": [-79.6, 43.6]},
            },
        ]
    content = {"type": "FeatureCollection", "features": features}
    with open(path, "w", encoding="utf-8") as f:
        json.dump(content, f)


def make_faux_lidar(path):
    """Create synthetic point cloud with PDAL readers.faux."""
    with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as pipeline_file:
        pipeline = {
            "pipeline": [
                {
                    "type": "readers.faux",
                    "bounds": "([-79.7,-79.5],[43.5,43.7],[100,110])",
                    "count": 100,
                    "mode": "constant",
                },
                {"type": "writers.las", "filename": str(path)},
            ]
        }
        json.dump(pipeline, pipeline_file)
        pipeline_path = Path(pipeline_file.name)
    rc, _, err = run_cmd(f"pdal pipeline '{pipeline_path}'", timeout=120)
    try:
        pipeline_path.unlink(missing_ok=True)
    except Exception:
        pass
    if rc != 0:
        return False, f"PDAL faux generation failed: {err}"
    return True, ""


def test_r01():
    """R01: int16 + nodata + EPSG roundtrip."""
    test_dir = RESULTS_ROOT / "raster/R01"
    test_dir.mkdir(parents=True, exist_ok=True)
    source = test_dir / "source_gdal.tif"
    roundtrip = test_dir / "roundtrip_wbraster.tif"
    if not source.exists():
        ok, msg = create_vrt_raster(source, data_type="Int16", value="42", nodata="-9999")
        if not ok:
            return "FAIL", msg
    return raster_roundtrip(source, roundtrip)


def test_r02():
    """R02: float32 + scale/offset roundtrip."""
    test_dir = RESULTS_ROOT / "raster/R02"
    test_dir.mkdir(parents=True, exist_ok=True)
    source = test_dir / "source_gdal_float32.tif"
    roundtrip = test_dir / "roundtrip_wbraster_float32.tif"
    base = test_dir / "_tmp_base.tif"
    ok, msg = create_vrt_raster(base, data_type="Float32", value="3.25", nodata="-9999")
    if not ok:
        return "FAIL", msg
    rc, _, err = run_cmd(
        f"gdal_translate -a_scale 0.01 -a_offset 100.0 '{base}' '{source}'"
    )
    base.unlink(missing_ok=True)
    if rc != 0:
        return "FAIL", f"gdal_translate scale/offset failed: {err}"
    return raster_roundtrip(source, roundtrip)


def test_r03():
    """R03: COG roundtrip."""
    test_dir = RESULTS_ROOT / "raster/R03"
    test_dir.mkdir(parents=True, exist_ok=True)
    base = test_dir / "_tmp_base.tif"
    source = test_dir / "source_gdal_cog.tif"
    roundtrip = test_dir / "roundtrip_wbraster_from_cog.tif"
    ok, msg = create_vrt_raster(base, data_type="Int16", value="15", nodata="-9999")
    if not ok:
        return "FAIL", msg
    rc, _, err = run_cmd(f"gdal_translate -of COG '{base}' '{source}'")
    base.unlink(missing_ok=True)
    if rc != 0:
        return "FAIL", f"COG creation failed: {err}"
    return raster_roundtrip(source, roundtrip)


def test_r04():
    """R04: DTED elevation roundtrip."""
    test_dir = RESULTS_ROOT / "raster/R04"
    test_dir.mkdir(parents=True, exist_ok=True)
    base = test_dir / "_tmp_base.tif"
    source = test_dir / "source_gdal.dt1"
    roundtrip = test_dir / "roundtrip_wbraster.dt1"
    dted_vrt = '''<VRTDataset rasterXSize="1201" rasterYSize="1201">
  <SRS>EPSG:4326</SRS>
  <GeoTransform>-80, 0.0008333333333333334, 0, 44, 0, -0.0008333333333333334</GeoTransform>
  <VRTRasterBand dataType="Int16" band="1">
    <NoDataValue>-32767</NoDataValue>
    <ConstantBand val="123"/>
  </VRTRasterBand>
</VRTDataset>'''
    with tempfile.NamedTemporaryFile(mode="w", suffix=".vrt", delete=False) as vrt_file:
        vrt_file.write(dted_vrt)
        vrt_path = Path(vrt_file.name)
    rc, _, err = run_cmd(f"gdal_translate -of GTiff '{vrt_path}' '{base}'")
    vrt_path.unlink(missing_ok=True)
    if rc != 0:
        return "FAIL", f"DTED base raster creation failed: {err}"
    rc, _, err = run_cmd(f"gdal_translate -of DTED '{base}' '{source}'")
    base.unlink(missing_ok=True)
    if rc != 0:
        return "FAIL", f"DTED creation failed: {err}"
    return raster_roundtrip(source, roundtrip)


def test_r05():
    """R05: HFA (.img) RLC compression roundtrip."""
    test_dir = RESULTS_ROOT / "raster/R05"
    test_dir.mkdir(parents=True, exist_ok=True)
    base = test_dir / "_tmp_base.tif"
    source = test_dir / "source_gdal.img"
    roundtrip = test_dir / "roundtrip_wbraster.img"
    ok, msg = create_vrt_raster(base, data_type="Int16", value="7", nodata="-9999")
    if not ok:
        return "FAIL", msg
    rc, _, err = run_cmd(
        f"gdal_translate -of HFA -co COMPRESSED=YES '{base}' '{source}'"
    )
    base.unlink(missing_ok=True)
    if rc != 0:
        return "FAIL", f"HFA creation failed: {err}"
    return raster_roundtrip(source, roundtrip)


def test_r06():
    """R06: Esri Float Grid roundtrip."""
    test_dir = RESULTS_ROOT / "raster/R06"
    test_dir.mkdir(parents=True, exist_ok=True)
    base = test_dir / "_tmp_base.tif"
    source = test_dir / "source_gdal.flt"
    roundtrip = test_dir / "roundtrip_wbraster.flt"
    ok, msg = create_vrt_raster(base, data_type="Float32", value="1.5", nodata="-9999")
    if not ok:
        return "FAIL", msg
    rc, _, err = run_cmd(f"gdal_translate -of EHdr '{base}' '{source}'")
    base.unlink(missing_ok=True)
    if rc != 0:
        return "FAIL", f"Esri Float Grid creation failed: {err}"
    return raster_roundtrip(source, roundtrip)


def test_r07():
    """R07: PNG + world file roundtrip."""
    test_dir = RESULTS_ROOT / "raster/R07"
    test_dir.mkdir(parents=True, exist_ok=True)
    base = test_dir / "_tmp_base.tif"
    source = test_dir / "source_gdal.png"
    roundtrip = test_dir / "roundtrip_wbraster.png"
    ok, msg = create_vrt_raster(base, data_type="Byte", value="120", nodata="0")
    if not ok:
        return "FAIL", msg
    rc, _, err = run_cmd(
        f"gdal_translate -of PNG -co WORLDFILE=YES '{base}' '{source}'"
    )
    base.unlink(missing_ok=True)
    if rc != 0:
        return "FAIL", f"PNG creation failed: {err}"
    status, note = raster_roundtrip(source, roundtrip)
    if status != "PASS":
        return status, note
    has_world = (
        roundtrip.with_suffix(".pgw").exists()
        or roundtrip.with_suffix(".pngw").exists()
        or roundtrip.with_suffix(".wld").exists()
    )
    if not has_world:
        return "FAIL", "Roundtrip PNG world file was not created"
    return "PASS", f"{note}; world_file=present"


def test_r08():
    """R08: QGIS producer variance check."""
    test_dir = RESULTS_ROOT / "raster/R08"
    test_dir.mkdir(parents=True, exist_ok=True)
    qgis_process = shutil.which("qgis_process")
    if qgis_process is None:
        bundled_qgis = Path("/Applications/QGIS-final-4_0_0.app/Contents/MacOS/qgis_process")
        if bundled_qgis.exists():
            qgis_process = str(bundled_qgis)
    if qgis_process is None:
        return "NOT_STARTED", "qgis_process not available on PATH"

    base = test_dir / "_tmp_base.tif"
    source = test_dir / "source_qgis_translate.tif"
    roundtrip = test_dir / "roundtrip_wbraster_qgis.tif"
    ok, msg = create_vrt_raster(base, data_type="Int16", value="9", nodata="-9999")
    if not ok:
        return "FAIL", msg
    rc, _, err = run_cmd(
        f"'{qgis_process}' run gdal:translate -- INPUT='{base}' TARGET_CRS='EPSG:4326' OUTPUT='{source}'",
        timeout=120,
    )
    base.unlink(missing_ok=True)
    if rc != 0:
        return "FAIL", f"QGIS translate failed: {err}"
    return raster_roundtrip(source, roundtrip)


def test_v01():
    """V01: Mixed fields/nulls/multipart via QGIS."""
    test_dir = RESULTS_ROOT / "vector/V01"
    test_dir.mkdir(parents=True, exist_ok=True)
    qgis_process = shutil.which("qgis_process")
    if qgis_process is None:
        bundled_qgis = Path("/Applications/QGIS-final-4_0_0.app/Contents/MacOS/qgis_process")
        if bundled_qgis.exists():
            qgis_process = str(bundled_qgis)
    if qgis_process is None:
        return "NOT_STARTED", "qgis_process not available on PATH"

    source_geojson = test_dir / "source_input.geojson"
    source_gpkg = test_dir / "source_qgis.gpkg"
    roundtrip = test_dir / "roundtrip_wbraster.gpkg"
    write_sample_geojson(source_geojson, mixed_geometry=True)
    rc, _, err = run_cmd(
        f"'{qgis_process}' run native:savefeatures -- INPUT='{source_geojson}' OUTPUT='{source_gpkg}'",
        timeout=120,
    )
    if rc != 0:
        return "FAIL", f"QGIS savefeatures failed: {err}"
    return vector_roundtrip(source_gpkg, roundtrip)


def test_v02():
    """V02: Schema constraints via GDAL (Shapefile)."""
    test_dir = RESULTS_ROOT / "vector/V02"
    test_dir.mkdir(parents=True, exist_ok=True)
    source_geojson = test_dir / "source_input.geojson"
    source_shp = test_dir / "source_gdal.shp"
    roundtrip = test_dir / "roundtrip_wbraster.shp"
    write_sample_geojson(source_geojson, mixed_geometry=False)
    rc, _, err = run_cmd(f"ogr2ogr -f 'ESRI Shapefile' '{source_shp}' '{source_geojson}'")
    if rc != 0:
        return "FAIL", f"Shapefile creation failed: {err}"
    return vector_roundtrip(source_shp, roundtrip)


def test_v03():
    """V03: GeoJSON interchange."""
    test_dir = RESULTS_ROOT / "vector/V03"
    test_dir.mkdir(parents=True, exist_ok=True)
    source_geojson = test_dir / "source_gdal.geojson"
    roundtrip = test_dir / "roundtrip_wbraster.geojson"
    write_sample_geojson(source_geojson, mixed_geometry=True)
    return vector_roundtrip(source_geojson, roundtrip)


def test_v04():
    """V04: FlatGeobuf binary interchange."""
    test_dir = RESULTS_ROOT / "vector/V04"
    test_dir.mkdir(parents=True, exist_ok=True)
    source_geojson = test_dir / "source_input.geojson"
    source_fgb = test_dir / "source_gdal.fgb"
    roundtrip = test_dir / "roundtrip_wbraster.fgb"
    write_sample_geojson(source_geojson, mixed_geometry=False)
    rc, _, err = run_cmd(
        f"ogr2ogr -f FlatGeobuf -lco SPATIAL_INDEX=NO '{source_fgb}' '{source_geojson}'"
    )
    if rc != 0:
        return "FAIL", f"FlatGeobuf creation failed: {err}"
    producer_count, count_err = ogr_feature_count(source_fgb)
    if producer_count is None:
        return "FAIL", f"FlatGeobuf producer count unavailable: {count_err}"

    env, env_err = get_env()
    if env is None:
        return "FAIL", env_err
    try:
        wbw_source_count = env.read_vector(str(source_fgb)).feature_count()
    except Exception as exc:
        return "FAIL", str(exc)

    if wbw_source_count != producer_count:
        return "FAIL", (
            "FlatGeobuf source parse mismatch: "
            f"producer_count={producer_count}, wbw_count={wbw_source_count}"
        )

    return vector_roundtrip(source_fgb, roundtrip)


def test_l01():
    """L01: LAS 1.4 point14 baseline."""
    if not require_cmd("pdal"):
        return "NOT_STARTED", "PDAL not available"
    test_dir = RESULTS_ROOT / "lidar/L01"
    test_dir.mkdir(parents=True, exist_ok=True)
    source = test_dir / "source_pdal.las"
    roundtrip = test_dir / "roundtrip_wbraster.las"
    ok, msg = make_faux_lidar(source)
    if not ok:
        return "FAIL", msg
    return lidar_roundtrip(source, roundtrip)


def test_l02():
    """L02: LAZ compressed roundtrip."""
    if not require_cmd("pdal"):
        return "NOT_STARTED", "PDAL not available"
    test_dir = RESULTS_ROOT / "lidar/L02"
    test_dir.mkdir(parents=True, exist_ok=True)
    source_las = test_dir / "_tmp_source.las"
    source_laz = test_dir / "source_pdal.laz"
    roundtrip = test_dir / "roundtrip_wbraster.laz"
    ok, msg = make_faux_lidar(source_las)
    if not ok:
        return "FAIL", msg
    rc, _, err = run_cmd(f"pdal translate '{source_las}' '{source_laz}'")
    source_las.unlink(missing_ok=True)
    if rc != 0:
        return "FAIL", f"LAZ creation failed: {err}"
    return lidar_roundtrip(source_laz, roundtrip)


def test_l03():
    """L03: COPC hierarchy-aware roundtrip."""
    if not require_cmd("pdal"):
        return "NOT_STARTED", "PDAL not available"
    test_dir = RESULTS_ROOT / "lidar/L03"
    test_dir.mkdir(parents=True, exist_ok=True)
    source_las = test_dir / "_tmp_source.las"
    source_copc = test_dir / "source_pdal.copc.laz"
    roundtrip = test_dir / "roundtrip_wbraster.copc.laz"
    ok, msg = make_faux_lidar(source_las)
    if not ok:
        return "FAIL", msg
    rc, _, err = run_cmd(f"pdal translate '{source_las}' '{source_copc}'")
    source_las.unlink(missing_ok=True)
    if rc != 0:
        return "FAIL", f"COPC creation failed: {err}"
    return lidar_roundtrip(source_copc, roundtrip)

def main():
    """Run Phase B test suite"""
    print("=" * 70)
    print("Phase B Comprehensive Interop Test Suite (v1.5)")
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
