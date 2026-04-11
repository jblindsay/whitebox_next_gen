#!/usr/bin/env python3
"""
Example: Lidar write options and format conversion.

This example demonstrates the Lidar.write_to_path() and Lidar.copy_to_path()
methods for writing point clouds with format options.

For Phase 1, these methods validate JSON options but perform basic writes.
Future phases will apply writer configuration once the wblidar frontend API
is extended to support configurable writes.
"""

import json
from pathlib import Path
from whitebox_workflows import WbEnvironment

def example_basic_copy():
    """Copy a lidar file to a new destination with automatic COPC transcoding."""
    print("Example 1: Basic Lidar Copy")
    print("-" * 50)
    
    env = WbEnvironment()
    
    # Create a test lidar object pointing to an input file
    # (in real code, this would be an actual LAS/LAZ file)
    input_lidar = env.lidar("input.las")
    
    # Copy to a new destination - if the destination has no extension,
    # it will be saved as COPC (Cloud Optimized Point Cloud)
    output_lidar = input_lidar.copy_to_path("output")
    print(f"Copied: {input_lidar.file_path}")
    print(f"Output: {output_lidar.file_path}")
    print()


def example_write_with_laz_options():
    """Write a lidar file to LAZ format with custom compression options."""
    print("Example 2: Write with LAZ Options")
    print("-" * 50)
    
    env = WbEnvironment()
    input_lidar = env.lidar("input.las")
    
    # Define LAZ write options
    options = {
        "laz": {
            "chunk_size": 25000,      # Points per compressed chunk
            "compression_level": 7     # 0-9, higher = better compression
        }
    }
    
    # Write to LAZ with custom options
    output_lidar = input_lidar.write_to_path(
        "output.laz",
        options_json=json.dumps(options)
    )
    print(f"Wrote to LAZ with custom options:")
    print(f"  chunk_size: {options['laz']['chunk_size']}")
    print(f"  compression_level: {options['laz']['compression_level']}")
    print(f"Output: {output_lidar.file_path}")
    print()


def example_write_with_copc_options():
    """Write a lidar file to Cloud Optimized Point Cloud (COPC) format."""
    print("Example 3: Write with COPC Options")
    print("-" * 50)
    
    env = WbEnvironment()
    input_lidar = env.lidar("input.las")
    
    # Define COPC write options for spatial indexing
    options = {
        "copc": {
            "max_points_per_node": 50000,  # Balance between lookup speed and I/O
            "max_depth": 10,                # Maximum octree depth
            "node_point_ordering": "hilbert"  # auto, morton, or hilbert
        }
    }
    
    # Write to COPC format with spatial optimization
    output_lidar = input_lidar.write_to_path(
        "output.copc.laz",
        options_json=json.dumps(options)
    )
    print(f"Wrote to COPC with spatial options:")
    print(f"  max_points_per_node: {options['copc']['max_points_per_node']}")
    print(f"  max_depth: {options['copc']['max_depth']}")
    print(f"  node_point_ordering: {options['copc']['node_point_ordering']}")
    print(f"Output: {output_lidar.file_path}")
    print()


def example_combined_options():
    """Write with both LAZ and COPC options (for flexibility in future expansions)."""
    print("Example 4: Combined Options")
    print("-" * 50)
    
    env = WbEnvironment()
    input_lidar = env.lidar("input.las")
    
    # Combined options object - useful when format is inferred from extension
    options = {
        "laz": {
            "chunk_size": 40000,
            "compression_level": 6
        },
        "copc": {
            "max_points_per_node": 75000,
            "max_depth": 8,
            "node_point_ordering": "auto"
        }
    }
    
    # Write to any format - options for the detected format will be used
    output_lidar = input_lidar.write_to_path(
        "output.copc.laz",
        options_json=json.dumps(options)
    )
    print(f"Wrote with combined options (format auto-detected from extension)")
    print(f"Output: {output_lidar.file_path}")
    print()


def example_options_validation():
    """Demonstrate that invalid options are caught at the Rust layer."""
    print("Example 5: Options Validation")
    print("-" * 50)
    
    env = WbEnvironment()
    input_lidar = env.lidar("input.las")
    
    try:
        # This will fail because 'node_point_ordering' expects auto, morton, or hilbert
        bad_options = {
            "copc": {
                "node_point_ordering": "invalid_ordering"
            }
        }
        output_lidar = input_lidar.write_to_path(
            "output.laz",
            options_json=json.dumps(bad_options)
        )
    except Exception as e:
        print(f"Caught validation error (as expected):")
        print(f"  {type(e).__name__}: {e}")
    print()


def example_path_inference():
    """Demonstrate automatic path extension inference."""
    print("Example 6: Path Extension Inference")
    print("-" * 50)
    
    env = WbEnvironment()
    input_lidar = env.lidar("input.las")
    
    # No extension → saved as COPC
    output1 = input_lidar.copy_to_path("directory/output")
    print(f"No extension → {Path(output1.file_path).name}")
    
    # .las extension → saved as LAS
    output2 = input_lidar.copy_to_path("directory/output.las")
    print(f".las extension → {Path(output2.file_path).name}")
    
    # .laz extension → saved as LAZ
    output3 = input_lidar.copy_to_path("directory/output.laz")
    print(f".laz extension → {Path(output3.file_path).name}")
    
    # .copc.laz extension → saved as COPC
    output4 = input_lidar.copy_to_path("directory/output.copc.laz")
    print(f".copc.laz extension → {Path(output4.file_path).name}")
    print()


if __name__ == "__main__":
    print("=" * 50)
    print("Whitebox Workflows: Lidar Write Options Examples")
    print("=" * 50)
    print()
    
    try:
        example_basic_copy()
        example_write_with_laz_options()
        example_write_with_copc_options()
        example_combined_options()
        example_options_validation()
        example_path_inference()
    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()
