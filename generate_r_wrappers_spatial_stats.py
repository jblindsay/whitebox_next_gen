#!/usr/bin/env python3
"""Generate R wrappers for spatial statistics tools."""

import json
from pathlib import Path

# Load the R tool taxonomy
R_TAXONOMY_PATH = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbw_r/r-package/whiteboxworkflows/inst/extdata/tool_taxonomy.resolved.json")
R_WRAPPERS_PATH = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbw_r/generated/wbw_tools_generated.R")

# Load current wrappers
with open(R_WRAPPERS_PATH, "r") as f:
    current_content = f.read()

# Load taxonomy
with open(R_TAXONOMY_PATH, "r") as f:
    taxonomy = json.load(f)

# Find all tools with descriptions
all_tools = {}
for category in taxonomy["mapping"]:
    for tool_id in category.get("tools", []):
        if tool_id in taxonomy["tools"]:
            tool_meta = taxonomy["tools"][tool_id]
            all_tools[tool_id] = tool_meta.get("display_name", tool_id)

# Find spatial statistics tools
spatial_stats_tools = []
for cat in taxonomy["mapping"]:
    if cat.get("subcategory") == "spatial_statistics":
        for tool_id in cat.get("tools", []):
            spatial_stats_tools.append((tool_id, all_tools.get(tool_id, tool_id)))

# Generate wrapper code for spatial stats tools
wrapper_lines = []
for tool_id, display_name in sorted(spatial_stats_tools):
    if f"session${tool_id}" not in current_content:
        wrapper_lines.append(f'  session${tool_id} <- function(...) {{')
        wrapper_lines.append(f'    # {display_name}')
        wrapper_lines.append(f'    run_tool("{tool_id}", list(...))')
        wrapper_lines.append(f'  }}')

if wrapper_lines:
    print(f"Generated {len(wrapper_lines) // 4} new spatial statistics tool wrappers:")
    for line in wrapper_lines:
        print(line)
else:
    print("All spatial statistics tools already have wrappers")

# Find insertion point (after spatial_join, before spectral_angle_mapper)
# Or find where to insert alphabetically
insertion_marker = 'session$spatial_join <- function(...) {'
if insertion_marker in current_content:
    # Find the end of spatial_join wrapper
    start_idx = current_content.find(insertion_marker)
    # Find the next tool definition after spatial_join
    next_tool_idx = current_content.find('session$s', start_idx + len(insertion_marker))
    
    # Get the line where next_tool starts
    next_tool_start = current_content.rfind('\n', start_idx, next_tool_idx) + 1
    
    # Find the line where we should insert (after spatial_join block closes)
    lines_up_to_next = current_content[:next_tool_start].split('\n')
    
    # Find where the spatial_join block ends (3 lines after the start)
    for i, line in enumerate(lines_up_to_next[-5:]):
        if '}' in line and 'run_tool' in '\n'.join(lines_up_to_next[-6:-i-1] if i > 0 else lines_up_to_next[-6:]):
            insert_line_idx = len(lines_up_to_next) - 5 + i + 1
            break
    
    # Insert the new wrappers
    if wrapper_lines:
        new_lines = lines_up_to_next + [''] + wrapper_lines + [''] + lines_up_to_next[insert_line_idx:]
        new_content = '\n'.join(new_lines)
        
        # Write back to file
        with open(R_WRAPPERS_PATH, "w") as f:
            f.write(new_content)
        print(f"\nInserted {len(wrapper_lines) // 4} spatial statistics tool wrappers into R wrapper file")
    else:
        print("No new wrappers to insert")
else:
    print(f"Could not find insertion point in R wrapper file")
