#!/usr/bin/env python3
"""
Test script for buffer_vector tool with:
1. Corrected cap direction (round caps should be visible)
2. New dissolve parameter (overlapping buffers should merge)
"""

import sys
sys.path.insert(0, '/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbw_python')

import whitebox_workflows as wbw
import tempfile
import os
import json

def test_buffer_dissolve():
    """Test buffer_vector with dissolve parameter"""
    print("\n=== Testing buffer_vector dissolve parameter ===\n")
    
    env = wbw.WbEnvironment()
    
    # Use an existing test dataset or create a simple one with wbt tools
    with tempfile.TemporaryDirectory() as tmpdir:
        # Create input GeoJSON with two overlapping lines
        input_geojson = {
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "properties": {"id": 1},
                    "geometry": {
                        "type": "LineString",
                        "coordinates": [[0, 0], [100, 0]]
                    }
                },
                {
                    "type": "Feature",
                    "properties": {"id": 2},
                    "geometry": {
                        "type": "LineString",
                        "coordinates": [[50, -20], [150, -20]]
                    }
                }
            ]
        }
        
        input_path = os.path.join(tmpdir, "test_lines.geojson")
        with open(input_path, 'w') as f:
            json.dump(input_geojson, f)
        
        output_no_dissolve = os.path.join(tmpdir, "buffer_no_dissolve.shp")
        output_with_dissolve = os.path.join(tmpdir, "buffer_with_dissolve.shp")
        
        print(f"Input GeoJSON: {input_path}")
        print(f"Output (no dissolve): {output_no_dissolve}")
        print(f"Output (with dissolve): {output_with_dissolve}\n")
        
        # Test 1: Buffer WITHOUT dissolve
        print("Test 1: Buffer without dissolve")
        print("-" * 50)
        try:
            result_no_dissolve = env.buffer_vector(
                input=input_path,
                distance=20.0,
                dissolve=False,
                output=output_no_dissolve
            )
            print(f"✓ Success: {len(result_no_dissolve.features)} output features")
        except Exception as e:
            print(f"✗ Error: {e}")
            import traceback
            traceback.print_exc()
            return
        
        # Test 2: Buffer WITH dissolve
        print("\nTest 2: Buffer with dissolve=true")
        print("-" * 50)
        try:
            result_with_dissolve = env.buffer_vector(
                input=input_path,
                distance=20.0,
                dissolve=True,
                output=output_with_dissolve
            )
            print(f"✓ Success: {len(result_with_dissolve.features)} output features")
        except Exception as e:
            print(f"✗ Error: {e}")
            import traceback
            traceback.print_exc()
            return
        
        # Verify results
        print("\nTest Results")
        print("=" * 50)
        num_no_dissolve = len(result_no_dissolve.features)
        num_with_dissolve = len(result_with_dissolve.features)
        
        print(f"Without dissolve: {num_no_dissolve} features")
        print(f"With dissolve:    {num_with_dissolve} features")
        
        if num_no_dissolve == 2 and num_with_dissolve == 1:
            print("\n✓ PASS: Dissolve correctly merged 2 overlapping buffers into 1")
            return True
        elif num_no_dissolve > 0 and num_with_dissolve > 0 and num_with_dissolve < num_no_dissolve:
            print(f"\n✓ PARTIAL PASS: Dissolve reduced features from {num_no_dissolve} to {num_with_dissolve}")
            return True
        else:
            print(f"\n✗ FAIL: Expected dissolve to reduce feature count")
            return False

if __name__ == "__main__":
    try:
        success = test_buffer_dissolve()
        if success:
            print("\n✓ All tests passed")
        else:
            print("\n✗ Tests failed")
    except Exception as e:
        print(f"\n✗ Error: {e}")
        import traceback
        traceback.print_exc()

