"""Insert PolarStereographic match arms into epsg.rs at the two non-exhaustive matches."""

arm = (
    '        PolarStereographic { north, lat_ts } => {\n'
    '            let method = if lat_ts.is_none() {\n'
    '                "Polar_Stereographic_Variant_A"\n'
    '            } else if *north {\n'
    '                "Stereographic_North_Pole"\n'
    '            } else {\n'
    '                "Stereographic_South_Pole"\n'
    '            };\n'
    '            let mut v = vec![\n'
    '                ("latitude_of_origin", p.lat0),\n'
    '                ("central_meridian", p.lon0),\n'
    '                ("false_easting", p.false_easting),\n'
    '                ("false_northing", p.false_northing),\n'
    '            ];\n'
    '            if let Some(phi) = lat_ts {\n'
    '                v.push(("standard_parallel_1", *phi));\n'
    '            } else {\n'
    '                v.push(("scale_factor", p.scale));\n'
    '            }\n'
    '            (method, v)\n'
    '        },\n'
)

with open('src/epsg.rs', 'r') as f:
    content = f.read()
    lines = content.splitlines(keepends=True)

# Find the two insertion points: after the closing paren of ObliqueStereographic arms
# Both are followed by "        Orthographic =>"
# We insert the PolarStereographic arm between ObliqueStereographic and Orthographic

oblique_arm_close = '        ),\n'
orthographic_arm_start = '        Orthographic => (\n'

count = 0
new_lines = []
i = 0
while i < len(lines):
    new_lines.append(lines[i])
    # Check if this is the closing '),' of ObliqueStereographic
    # by looking at the next non-empty line being "Orthographic =>"
    if lines[i] == oblique_arm_close and i + 1 < len(lines) and lines[i+1] == orthographic_arm_start:
        # Check it's one of the two target locations (after an ObliqueStereographic arm)
        # Look back a few lines to confirm
        lookback = ''.join(lines[max(0, i-10):i])
        if '"Oblique_Stereographic"' in lookback:
            new_lines.append(arm)
            count += 1
    i += 1

assert count == 2, f"Expected to insert at 2 locations, got {count}"

with open('src/epsg.rs', 'w') as f:
    f.writelines(new_lines)

print(f"Successfully inserted {count} PolarStereographic arms into epsg.rs")
