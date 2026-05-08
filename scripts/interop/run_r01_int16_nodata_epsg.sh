#!/bin/bash
# Phase B Raster Test: R01 (int16 + nodata + EPSG roundtrip)
# Producer: GDAL | Format: GeoTIFF

set -e

CASE_ID="R01"
TEST_DIR="/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/artifacts/interop/results/raster/R01"
PROJ_DIR="/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen"

mkdir -p "$TEST_DIR"

echo "=== Phase B Raster Test: $CASE_ID ==="
echo "Case: int16 + nodata + EPSG roundtrip"
echo "Test directory: $TEST_DIR"

# Step 1: Create source artifact with GDAL
SOURCE_ARTIFACT="$TEST_DIR/source_gdal.tif"
echo ""
echo "Step 1: Creating source artifact with GDAL..."

# Create a 10x10 int16 raster with EPSG:4326 and nodata=-9999
gdal_translate -of GTiff -ot Int16 -a_srs EPSG:4326 -a_nodata -9999 \
  /vsimem/test.tif "$SOURCE_ARTIFACT" 2>/dev/null || {
    # Fallback: use a more basic approach
    python3 << 'EOF'
import numpy as np
import osgeo.gdal as gdal
from osgeo import osr

# Create a simple 10x10 int16 array
data = np.arange(100, dtype=np.int16).reshape(10, 10)

# Create GeoTIFF
driver = gdal.GetDriverByName('GTiff')
ds = driver.Create('/tmp/source_gdal_r01.tif', 10, 10, 1, gdal.GDT_Int16)

# Set geotransform (4326)
ds.SetGeoTransform([-180.0, 36.0, 0, 90.0, 0, -18.0])

# Set projection
srs = osr.SpatialReference()
srs.ImportFromEPSG(4326)
ds.SetProjection(srs.ExportToWkt())

# Write data and set nodata
band = ds.GetRasterBand(1)
band.WriteArray(data)
band.SetNoDataValue(-9999)

ds = None
print("✓ Source artifact created")
EOF
    SOURCE_ARTIFACT="/tmp/source_gdal_r01.tif"
}

if [ ! -f "$SOURCE_ARTIFACT" ]; then
    echo "✗ Failed to create source artifact"
    exit 1
fi

gdalinfo "$SOURCE_ARTIFACT" | grep -E "(Size is|Type=|NoData|PROJ|GEOGCS)" | head -10
echo "✓ Source artifact created: $(ls -lh "$SOURCE_ARTIFACT" | awk '{print $5}')"

# Step 2: Read and write with wbraster
echo ""
echo "Step 2: Reading and writing with wbraster..."

ROUNDTRIP_ARTIFACT="$TEST_DIR/roundtrip_wbraster.tif"

cd "$PROJ_DIR"
cargo run --bin phase_b_raster_test --release -- "$CASE_ID" "$SOURCE_ARTIFACT" "$ROUNDTRIP_ARTIFACT" 2>&1 | grep -E "(✓|✗|STATUS|Error)"

if [ ! -f "$ROUNDTRIP_ARTIFACT" ]; then
    echo "✗ Failed to create roundtrip artifact"
    exit 1
fi

gdalinfo "$ROUNDTRIP_ARTIFACT" | grep -E "(Size is|Type=|NoData|PROJ|GEOGCS)" | head -10
echo "✓ Roundtrip artifact created: $(ls -lh "$ROUNDTRIP_ARTIFACT" | awk '{print $5}')"

# Step 3: Validate roundtrip
echo ""
echo "Step 3: Validating roundtrip..."

python3 << 'VALIDATE_EOF'
import osgeo.gdal as gdal
import numpy as np

source = gdal.Open('/tmp/source_gdal_r01.tif')
roundtrip = gdal.Open('ROUNDTRIP_PLACEHOLDER')

if not source or not roundtrip:
    print("✗ Failed to open rasters")
    exit(1)

s_band = source.GetRasterBand(1)
r_band = roundtrip.GetRasterBand(1)

# Check metadata
checks = []
checks.append(("Size match", source.RasterXSize == roundtrip.RasterXSize and source.RasterYSize == roundtrip.RasterYSize))
checks.append(("NoData match", s_band.GetNoDataValue() == r_band.GetNoDataValue()))
checks.append(("Data type match", s_band.DataType == r_band.DataType))
checks.append(("Geotransform match", source.GetGeoTransform() == roundtrip.GetGeoTransform()))

all_pass = True
for check_name, result in checks:
    status = "✓" if result else "✗"
    print(f"{status} {check_name}")
    all_pass = all_pass and result

if all_pass:
    print("STATUS: PASS")
    exit(0)
else:
    print("STATUS: FAIL")
    exit(1)
VALIDATE_EOF

echo ""
echo "✓ Case $CASE_ID complete"
